//! 合买领域模型，定义合买计划、参与人和份额约束

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, PgConnection, Row};
use tokio::sync::Mutex;

use crate::{
    domain::{
        group_buy::{
            AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyParticipant,
            GroupBuyPlan, GroupBuyPlanStatus, GroupBuyPlanSummary, UpdateGroupBuyPlanRequest,
        },
        lottery::LotteryKind,
        user::UserSummary,
    },
    error::{ApiError, ApiResult},
};

use super::{
    business_database::{enum_from_string, enum_to_string, BusinessDatabase},
    pagination::{ListPage, PageRequest},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台合买列表按是否已经生成真实投注订单筛选的状态。
pub enum GroupBuyFormationFilter {
    /// 已经关联真实投注订单的合买计划。
    Formed,
    /// 尚未关联真实投注订单的合买计划。
    Unformed,
}

impl GroupBuyFormationFilter {
    /// 判断合买计划摘要是否符合当前成单筛选口径。
    fn matches_summary(self, plan: &GroupBuyPlanSummary) -> bool {
        match self {
            Self::Formed => plan.order_id.is_some(),
            Self::Unformed => plan.order_id.is_none(),
        }
    }
}

#[derive(Clone)]
/// 合买计划和参与记录仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct GroupBuyRepository {
    /// 合买模块内存快照锁，保存计划、参与人和序号状态。
    pub(crate) inner: Arc<RwLock<GroupBuyStore>>,
    /// 可选数据库持久化句柄；内存模式下为空。
    pub(crate) persistence: Option<BusinessDatabase>,
    /// 串行化合买写操作，避免多个快照异步落库时旧快照覆盖新快照。
    pub(crate) mutation_lock: Arc<Mutex<()>>,
}

/// 合买计划和参与记录仓储，负责该模块数据读取、业务变更和持久化协调。
impl GroupBuyRepository {
    /// 返回带内置种子数据的内存仓储实例。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(GroupBuyStore::seeded())),
            persistence: None,
            mutation_lock: Arc::new(Mutex::new(())),
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_group_buy_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
            mutation_lock: Arc::new(Mutex::new(())),
        })
    }

    /// 按当前仓储快照返回全部合买计划列表。
    pub async fn list(&self) -> ApiResult<Vec<GroupBuyPlanSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 分页返回合买摘要；数据库模式下把机器人发起人、成单状态和分页下推到 SQL。
    pub async fn list_page(
        &self,
        excluded_initiator_user_ids: &[&str],
        formation_filter: Option<GroupBuyFormationFilter>,
        plan_id_filter: Option<&str>,
        page: PageRequest,
    ) -> ApiResult<ListPage<GroupBuyPlanSummary>> {
        let excluded_initiator_user_ids = normalized_filter_values(excluded_initiator_user_ids);
        let plan_id_filter = plan_id_filter
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        if let Some(persistence) = &self.persistence {
            let excluded_initiator_user_ids = excluded_initiator_user_ids
                .iter()
                .cloned()
                .collect::<Vec<_>>();
            return query_group_buy_summary_page(
                persistence,
                &excluded_initiator_user_ids,
                formation_filter,
                plan_id_filter.as_deref(),
                page,
            )
            .await;
        }

        let plans = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .list()
            .into_iter()
            .filter(|plan| !excluded_initiator_user_ids.contains(&plan.initiator_user_id))
            .filter(|plan| formation_filter.map_or(true, |filter| filter.matches_summary(plan)))
            .filter(|plan| {
                plan_id_filter
                    .as_deref()
                    .map_or(true, |plan_id| plan.id == plan_id)
            })
            .collect::<Vec<_>>();
        let plans = sorted_group_buy_summaries_by_created_at(plans);
        Ok(ListPage::from_all(plans, page))
    }

    /// 返回完整计划列表，供用户端大厅按参与记录计算我的合买。
    pub async fn list_details(&self) -> ApiResult<Vec<GroupBuyPlan>> {
        if let Some(persistence) = &self.persistence {
            return query_group_buy_details(persistence).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| store.list_details())
    }

    /// 返回指定用户参与过的合买计划详情，避免用户端“我的合买/我的注单”扫描全部计划。
    pub async fn list_details_for_user(&self, user_id: &str) -> ApiResult<Vec<GroupBuyPlan>> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }
        if let Some(persistence) = &self.persistence {
            return query_group_buy_details_for_user(persistence, user_id).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| store.list_details_for_user(user_id))
    }

    /// 批量读取指定参与人集合的有效合买计划，供代理投注画像避免扫描全量合买。
    pub async fn list_active_details_for_participant_user_ids(
        &self,
        user_ids: &[String],
    ) -> ApiResult<Vec<GroupBuyPlan>> {
        let user_ids = normalized_user_ids(user_ids);
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(persistence) = &self.persistence {
            return query_active_group_buy_details_for_participant_user_ids(persistence, &user_ids)
                .await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| store.list_active_details_for_participant_user_ids(&user_ids))
    }

    /// 分页返回指定用户参与但尚未形成真实订单的合买计划，供“我的合买”避免扫描全量计划。
    pub async fn list_unformed_details_for_user_page(
        &self,
        user_id: &str,
        page: PageRequest,
    ) -> ApiResult<ListPage<GroupBuyPlan>> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }
        if let Some(persistence) = &self.persistence {
            return query_unformed_group_buy_details_for_user_page(persistence, user_id, page)
                .await;
        }

        let plans = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .list_details_for_user(user_id)
            .into_iter()
            .filter(|plan| plan.order_id.is_none())
            .collect::<Vec<_>>();
        let plans = sorted_group_buy_details_by_created_at(plans);
        Ok(ListPage::from_all(plans, page))
    }

    /// 返回指定用户参与过并已经成单的合买订单 ID，供注单列表只读取相关真实订单。
    pub async fn order_ids_for_user(&self, user_id: &str) -> ApiResult<Vec<String>> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }
        if let Some(persistence) = &self.persistence {
            return query_group_buy_order_ids_for_user(persistence, user_id).await;
        }

        Ok(self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .list_details_for_user(user_id)
            .into_iter()
            .filter_map(|plan| plan.order_id)
            .collect())
    }

    /// 返回指定彩种和期号下仍在流转中的合买计划详情，供控奖页面查看发起人自购记录。
    pub async fn list_control_details_for_issue(
        &self,
        lottery_id: &str,
        issue: &str,
    ) -> ApiResult<Vec<GroupBuyPlan>> {
        let lottery_id = lottery_id.trim();
        let issue = issue.trim();
        if lottery_id.is_empty() {
            return Err(ApiError::BadRequest("彩种 ID 不能为空".to_string()));
        }
        if issue.is_empty() {
            return Err(ApiError::BadRequest("期号不能为空".to_string()));
        }
        let keep_initiator_participants = |plans: Vec<GroupBuyPlan>| {
            plans
                .into_iter()
                .filter_map(control_group_buy_plan_with_initiator_participants)
                .collect()
        };
        if let Some(persistence) = &self.persistence {
            return query_control_group_buy_details_for_issue(persistence, lottery_id, issue)
                .await
                .map(keep_initiator_participants);
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| {
                keep_initiator_participants(store.list_control_details_for_issue(lottery_id, issue))
            })
    }

    /// 分页返回用户端合买大厅活跃计划，并只加载当前页计划的参与人。
    pub async fn list_active_details_page(
        &self,
        lottery_ids: &[String],
        status_filter: Option<GroupBuyPlanStatus>,
        page: PageRequest,
    ) -> ApiResult<ListPage<GroupBuyPlan>> {
        if let Some(persistence) = &self.persistence {
            return query_active_group_buy_details_page(
                persistence,
                lottery_ids,
                status_filter,
                page,
            )
            .await;
        }

        let lottery_ids = lottery_ids.iter().collect::<BTreeSet<_>>();
        let plans = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .list_details()
            .into_iter()
            .filter(|plan| {
                (lottery_ids.is_empty() || lottery_ids.contains(&plan.lottery_id))
                    && match &status_filter {
                        Some(status) => plan.status == *status,
                        None => matches!(
                            plan.status,
                            GroupBuyPlanStatus::Draft
                                | GroupBuyPlanStatus::Open
                                | GroupBuyPlanStatus::Filled
                        ),
                    }
            })
            .collect::<Vec<_>>();
        let plans = sorted_group_buy_details_by_created_at(plans);
        Ok(ListPage::from_all(plans, page))
    }

    /// 一键清除已结束合买计划历史；未结算计划会自动保留，避免资金和结算失去追溯。
    pub async fn clear_records(&self) -> ApiResult<usize> {
        self.mutate_and_persist_if(|store| {
            let deleted_count = store.clear_records()?;
            Ok((deleted_count, deleted_count > 0))
        })
        .await
    }

    /// 按业务标识读取单条记录，未命中时返回未找到错误。
    pub async fn get(&self, id: &str) -> ApiResult<GroupBuyPlan> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .get(id)
    }

    /// 按真实投注订单 ID 批量查找合买计划。
    pub async fn plans_for_order_ids(&self, order_ids: &[String]) -> ApiResult<Vec<GroupBuyPlan>> {
        if let Some(persistence) = &self.persistence {
            return query_group_buy_plans_for_order_ids(persistence, order_ids).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| store.plans_for_order_ids(order_ids))
    }

    /// 校验入参并创建一条新记录。
    pub async fn create(
        &self,
        request: CreateGroupBuyPlanRequest,
        lotteries: &[LotteryKind],
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        self.mutate_and_persist(|store| store.create(request, lotteries, users))
            .await
    }

    /// 封盘时取消未满单的合买计划，返回需要退款的计划。
    pub async fn cancel_unfilled_for_issue(
        &self,
        lottery_id: &str,
        issue: &str,
    ) -> ApiResult<Vec<GroupBuyPlan>> {
        self.mutate_and_persist_if(|store| {
            let result = store.cancel_unfilled_for_issue(lottery_id, issue);
            let should_persist = !result.is_empty();
            Ok((result, should_persist))
        })
        .await
    }

    /// 封盘时取消未满单的合买计划，但跳过调度器本轮保护的计划。
    pub async fn cancel_unfilled_for_issue_except(
        &self,
        lottery_id: &str,
        issue: &str,
        protected_plan_ids: &HashSet<String>,
    ) -> ApiResult<Vec<GroupBuyPlan>> {
        self.mutate_and_persist_if(|store| {
            let result =
                store.cancel_unfilled_for_issue_except(lottery_id, issue, protected_plan_ids);
            let should_persist = !result.is_empty();
            Ok((result, should_persist))
        })
        .await
    }

    /// 更新现有记录并持久化变更。
    pub async fn update(
        &self,
        id: &str,
        request: UpdateGroupBuyPlanRequest,
    ) -> ApiResult<GroupBuyPlan> {
        self.mutate_and_persist(|store| store.update(id, request))
            .await
    }

    /// 为团购方案添加参与者。
    pub async fn add_participant(
        &self,
        id: &str,
        request: AddGroupBuyParticipantRequest,
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        self.mutate_and_persist(|store| store.add_participant(id, request, users))
            .await
    }

    /// 移除刚创建但尚未成功扣款的合买计划，用于业务回滚。
    pub async fn remove_unfunded_plan(&self, id: &str) -> ApiResult<()> {
        self.mutate_and_persist(|store| store.remove_plan(id)).await
    }

    /// 移除刚追加但尚未成功扣款的参与记录，用于业务回滚。
    pub async fn remove_unfunded_participant(
        &self,
        id: &str,
        participant_id: &str,
    ) -> ApiResult<GroupBuyPlan> {
        self.mutate_and_persist(|store| store.remove_participant(id, participant_id))
            .await
    }

    /// 执行一次合买写操作并强制落库，落库失败时恢复内存快照。
    async fn mutate_and_persist<T>(
        &self,
        mutate: impl FnOnce(&mut GroupBuyStore) -> ApiResult<T>,
    ) -> ApiResult<T> {
        self.mutate_and_persist_if(|store| mutate(store).map(|result| (result, true)))
            .await
    }

    /// 执行一次合买写操作，并按需把变更快照保存到数据库。
    async fn mutate_and_persist_if<T>(
        &self,
        mutate: impl FnOnce(&mut GroupBuyStore) -> ApiResult<(T, bool)>,
    ) -> ApiResult<T> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let (result, should_persist, previous, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            let previous = store.clone();
            let (result, should_persist) = match mutate(&mut store) {
                Ok(result) => result,
                Err(error) => {
                    *store = previous;
                    return Err(error);
                }
            };
            let snapshot = store.clone();
            (result, should_persist, previous, snapshot)
        };

        if should_persist {
            if let Err(error) = self.persist_incremental(&previous, &snapshot).await {
                self.replace_store(previous)?;
                return Err(error);
            }
        }

        Ok(result)
    }
    /// 把合买快照差异增量保存到持久化存储，避免普通写操作重写整张合买表。
    async fn persist_incremental(
        &self,
        previous: &GroupBuyStore,
        store: &GroupBuyStore,
    ) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            let mut tx = persistence
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::Internal("合买事务开启失败".to_string()))?;
            save_group_buy_store_incremental_in_transaction(&mut *tx, previous, store).await?;
            tx.commit()
                .await
                .map_err(|_| ApiError::Internal("合买事务提交失败".to_string()))?;
        }

        Ok(())
    }

    /// 从数据库重新加载合买计划和参与记录快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_group_buy_store(persistence).await?;
        self.replace_store(store)?;
        Ok(true)
    }

    /// 用事务提交后的快照替换当前合买计划和参与记录内存状态。
    pub(crate) fn replace_store(&self, store: GroupBuyStore) -> ApiResult<()> {
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))? = store;
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// 合买计划和参与记录运行时数据快照，用于内存模式和数据库持久化前的业务校验。
pub(crate) struct GroupBuyStore {
    plans: BTreeMap<String, GroupBuyPlan>,
}

/// 从数据库加载合买计划和参与记录运行时快照，空库时按模块规则初始化。
async fn load_group_buy_store(database: &BusinessDatabase) -> ApiResult<GroupBuyStore> {
    let pool = database.pool();
    let mut plans = BTreeMap::new();

    for row in sqlx::query(
        "SELECT id, lottery_id, lottery_name, initiator_user_id, initiator_username,
                order_id, issue, rule_code, title, numbers,
                total_amount_minor, filled_amount_minor, min_share_amount_minor,
                participant_min_amount_minor, share_count, status, note, created_at, updated_at
         FROM group_buy_plans
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?
    {
        let plan = group_buy_plan_from_row(row)?;
        plans.insert(plan.id.clone(), plan);
    }

    for row in sqlx::query(
        "SELECT id, plan_id, user_id, username, amount_minor, share_count, note, created_at
         FROM group_buy_participants
         ORDER BY plan_id ASC, id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?
    {
        let (plan_id, participant) = group_buy_participant_from_row(row)?;
        if let Some(plan) = plans.get_mut(&plan_id) {
            plan.participants.push(participant);
        }
    }

    if plans.is_empty() {
        let seeded = GroupBuyStore::seeded();
        save_group_buy_store(database, &seeded).await?;
        return Ok(seeded);
    }

    Ok(GroupBuyStore { plans })
}

/// 从数据库行恢复合买计划，参与人由调用方按需补入。
fn group_buy_plan_from_row(row: PgRow) -> ApiResult<GroupBuyPlan> {
    let share_count: i32 = row
        .try_get("share_count")
        .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?;
    Ok(GroupBuyPlan {
        id: row
            .try_get("id")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        lottery_id: row
            .try_get("lottery_id")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        lottery_name: row
            .try_get("lottery_name")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        order_id: row
            .try_get("order_id")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        issue: row
            .try_get("issue")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        rule_code: row
            .try_get("rule_code")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        title: row
            .try_get("title")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        numbers: row
            .try_get("numbers")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        initiator_user_id: row
            .try_get("initiator_user_id")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        initiator_username: row
            .try_get("initiator_username")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        total_amount_minor: row
            .try_get("total_amount_minor")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        filled_amount_minor: row
            .try_get("filled_amount_minor")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        min_share_amount_minor: row
            .try_get("min_share_amount_minor")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        participant_min_amount_minor: row
            .try_get("participant_min_amount_minor")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        share_count: u32::try_from(share_count)
            .map_err(|_| ApiError::Internal("合买计划份数数据无效".to_string()))?,
        status: enum_from_string(
            row.try_get("status")
                .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        )?,
        participants: Vec::new(),
        note: row
            .try_get("note")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        created_at: row
            .try_get("created_at")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
        updated_at: row
            .try_get("updated_at")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?,
    })
}

/// 从数据库行恢复合买参与人，并返回所属计划 ID。
fn group_buy_participant_from_row(row: PgRow) -> ApiResult<(String, GroupBuyParticipant)> {
    let plan_id: String = row
        .try_get("plan_id")
        .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?;
    let share_count: i32 = row
        .try_get("share_count")
        .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?;
    Ok((
        plan_id,
        GroupBuyParticipant {
            id: row
                .try_get("id")
                .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?,
            user_id: row
                .try_get("user_id")
                .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?,
            username: row
                .try_get("username")
                .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?,
            amount_minor: row
                .try_get("amount_minor")
                .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?,
            share_count: u32::try_from(share_count)
                .map_err(|_| ApiError::Internal("合买参与人份数数据无效".to_string()))?,
            note: row
                .try_get("note")
                .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?,
        },
    ))
}

/// 判断指定用户是否和合买计划有关联，发起人与跟单参与人都应进入“我的合买”。
fn group_buy_plan_related_to_user(plan: &GroupBuyPlan, user_id: &str) -> bool {
    plan.initiator_user_id == user_id
        || plan
            .participants
            .iter()
            .any(|participant| participant.user_id == user_id)
}

/// 数据库模式下分页读取合买摘要，避免后台合买列表先拉全量再裁剪。
async fn query_group_buy_summary_page(
    database: &BusinessDatabase,
    excluded_initiator_user_ids: &[String],
    formation_filter: Option<GroupBuyFormationFilter>,
    plan_id_filter: Option<&str>,
    page: PageRequest,
) -> ApiResult<ListPage<GroupBuyPlanSummary>> {
    let formation_filter = formation_filter.map(|filter| match filter {
        GroupBuyFormationFilter::Formed => "formed",
        GroupBuyFormationFilter::Unformed => "unformed",
    });
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM group_buy_plans
         WHERE (cardinality($1::text[]) = 0 OR NOT (initiator_user_id = ANY($1)))
           AND (
               $2::text IS NULL
               OR ($2 = 'formed' AND order_id IS NOT NULL)
               OR ($2 = 'unformed' AND order_id IS NULL)
           )
           AND ($3::text IS NULL OR id = $3)",
    )
    .bind(excluded_initiator_user_ids)
    .bind(formation_filter)
    .bind(plan_id_filter)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("合买计划分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("合买计划分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, initiator_user_id, initiator_username,
                order_id, issue, rule_code, title, numbers,
                total_amount_minor, filled_amount_minor, min_share_amount_minor,
                participant_min_amount_minor, share_count, status, note, created_at, updated_at
         FROM group_buy_plans
         WHERE (cardinality($1::text[]) = 0 OR NOT (initiator_user_id = ANY($1)))
           AND (
               $2::text IS NULL
               OR ($2 = 'formed' AND order_id IS NOT NULL)
               OR ($2 = 'unformed' AND order_id IS NULL)
           )
           AND ($3::text IS NULL OR id = $3)
         ORDER BY created_at DESC, id DESC
         LIMIT $4 OFFSET $5",
    )
    .bind(excluded_initiator_user_ids)
    .bind(formation_filter)
    .bind(plan_id_filter)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("合买计划分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(group_buy_plan_from_row)
        .map(|result| result.map(|plan| plan.summary()))
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下读取指定用户发起或参与过的合买计划，并只加载这些计划的参与人。
/// 数据库模式下读取全部合买计划详情及参与人，供调度器强制满单扫描使用。
async fn query_group_buy_details(database: &BusinessDatabase) -> ApiResult<Vec<GroupBuyPlan>> {
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, initiator_user_id, initiator_username,
                order_id, issue, rule_code, title, numbers,
                total_amount_minor, filled_amount_minor, min_share_amount_minor,
                participant_min_amount_minor, share_count, status, note, created_at, updated_at
         FROM group_buy_plans
         ORDER BY issue DESC, created_at DESC, id DESC",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("合买计划全量数据读取失败".to_string()))?;
    let mut plans = rows
        .into_iter()
        .map(group_buy_plan_from_row)
        .map(|result| result.map(|plan| (plan.id.clone(), plan)))
        .collect::<ApiResult<BTreeMap<_, _>>>()?;
    if plans.is_empty() {
        return Ok(Vec::new());
    }

    let plan_ids = plans.keys().cloned().collect::<Vec<_>>();
    let participant_rows = sqlx::query(
        "SELECT id, plan_id, user_id, username, amount_minor, share_count, note, created_at
         FROM group_buy_participants
         WHERE plan_id = ANY($1)
         ORDER BY plan_id ASC, id ASC",
    )
    .bind(&plan_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("合买参与人全量数据读取失败".to_string()))?;
    for row in participant_rows {
        let participant = group_buy_participant_from_row(row)?;
        if let Some(plan) = plans.get_mut(&participant.0) {
            plan.participants.push(participant.1);
        }
    }

    Ok(sorted_group_buy_plans(plans.values())
        .into_iter()
        .cloned()
        .collect())
}

async fn query_group_buy_details_for_user(
    database: &BusinessDatabase,
    user_id: &str,
) -> ApiResult<Vec<GroupBuyPlan>> {
    let rows = sqlx::query(
        "SELECT p.id, p.lottery_id, p.lottery_name, p.initiator_user_id, p.initiator_username,
                p.order_id, p.issue, p.rule_code, p.title, p.numbers,
                p.total_amount_minor, p.filled_amount_minor, p.min_share_amount_minor,
                p.participant_min_amount_minor, p.share_count, p.status, p.note, p.created_at, p.updated_at
         FROM group_buy_plans p
         WHERE p.initiator_user_id = $1
            OR EXISTS (
                SELECT 1 FROM group_buy_participants gp
                WHERE gp.plan_id = p.id AND gp.user_id = $1
            )
         ORDER BY p.issue DESC, p.created_at DESC, p.id DESC",
    )
    .bind(user_id)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户合买计划数据读取失败".to_string()))?;
    let mut plans = rows
        .into_iter()
        .map(group_buy_plan_from_row)
        .map(|result| result.map(|plan| (plan.id.clone(), plan)))
        .collect::<ApiResult<BTreeMap<_, _>>>()?;
    if plans.is_empty() {
        return Ok(Vec::new());
    }

    let plan_ids = plans.keys().cloned().collect::<Vec<_>>();
    let participant_rows = sqlx::query(
        "SELECT id, plan_id, user_id, username, amount_minor, share_count, note, created_at
         FROM group_buy_participants
         WHERE plan_id = ANY($1)
         ORDER BY plan_id ASC, id ASC",
    )
    .bind(&plan_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户合买参与人数据读取失败".to_string()))?;
    for row in participant_rows {
        let participant = group_buy_participant_from_row(row)?;
        if let Some(plan) = plans.get_mut(&participant.0) {
            plan.participants.push(participant.1);
        }
    }

    Ok(sorted_group_buy_plans(plans.values())
        .into_iter()
        .cloned()
        .collect())
}

/// 数据库模式下按参与人集合读取有效合买计划，并批量加载参与记录。
async fn query_active_group_buy_details_for_participant_user_ids(
    database: &BusinessDatabase,
    user_ids: &BTreeSet<String>,
) -> ApiResult<Vec<GroupBuyPlan>> {
    let user_ids = user_ids.iter().cloned().collect::<Vec<_>>();
    let rows = sqlx::query(
        "SELECT DISTINCT p.id, p.lottery_id, p.lottery_name, p.initiator_user_id, p.initiator_username,
                p.order_id, p.issue, p.rule_code, p.title, p.numbers,
                p.total_amount_minor, p.filled_amount_minor, p.min_share_amount_minor,
                p.participant_min_amount_minor, p.share_count, p.status, p.note, p.created_at, p.updated_at
         FROM group_buy_plans p
         INNER JOIN group_buy_participants gp ON gp.plan_id = p.id
         WHERE gp.user_id = ANY($1::text[])
           AND gp.amount_minor > 0
           AND p.status <> $2
         ORDER BY p.issue DESC, p.created_at DESC, p.id DESC",
    )
    .bind(&user_ids)
    .bind(enum_to_string(&GroupBuyPlanStatus::Cancelled)?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("直属用户合买计划数据读取失败".to_string()))?;
    let mut plans = rows
        .into_iter()
        .map(group_buy_plan_from_row)
        .map(|result| result.map(|plan| (plan.id.clone(), plan)))
        .collect::<ApiResult<BTreeMap<_, _>>>()?;
    attach_participants_to_plans(database, &mut plans, "直属用户合买参与人数据读取失败").await?;

    Ok(sorted_group_buy_plans(plans.values())
        .into_iter()
        .cloned()
        .collect())
}

/// 数据库模式下分页读取用户发起或参与但未成单的合买计划。
async fn query_unformed_group_buy_details_for_user_page(
    database: &BusinessDatabase,
    user_id: &str,
    page: PageRequest,
) -> ApiResult<ListPage<GroupBuyPlan>> {
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM group_buy_plans p
         WHERE p.order_id IS NULL
           AND (
               p.initiator_user_id = $1
               OR EXISTS (
                   SELECT 1 FROM group_buy_participants gp
                   WHERE gp.plan_id = p.id AND gp.user_id = $1
               )
           )",
    )
    .bind(user_id)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户未成单合买分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("用户未成单合买分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT p.id, p.lottery_id, p.lottery_name, p.initiator_user_id, p.initiator_username,
                p.order_id, p.issue, p.rule_code, p.title, p.numbers,
                p.total_amount_minor, p.filled_amount_minor, p.min_share_amount_minor,
                p.participant_min_amount_minor, p.share_count, p.status, p.note, p.created_at, p.updated_at
         FROM group_buy_plans p
         WHERE p.order_id IS NULL
           AND (
               p.initiator_user_id = $1
               OR EXISTS (
                   SELECT 1 FROM group_buy_participants gp
                   WHERE gp.plan_id = p.id AND gp.user_id = $1
               )
           )
         ORDER BY p.created_at DESC, p.id DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户未成单合买分页数据读取失败".to_string()))?;
    let mut plans = rows
        .into_iter()
        .map(group_buy_plan_from_row)
        .map(|result| result.map(|plan| (plan.id.clone(), plan)))
        .collect::<ApiResult<BTreeMap<_, _>>>()?;
    attach_participants_to_plans(database, &mut plans, "用户未成单合买参与人数据读取失败").await?;

    let items = sorted_group_buy_plans(plans.values())
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下读取用户发起或参与过且已经形成真实订单的合买订单 ID。
async fn query_group_buy_order_ids_for_user(
    database: &BusinessDatabase,
    user_id: &str,
) -> ApiResult<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT p.order_id
         FROM group_buy_plans p
         WHERE p.order_id IS NOT NULL
           AND (
               p.initiator_user_id = $1
               OR EXISTS (
                   SELECT 1 FROM group_buy_participants gp
                   WHERE gp.plan_id = p.id AND gp.user_id = $1
               )
           )
         ORDER BY p.order_id DESC",
    )
    .bind(user_id)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户合买订单编号读取失败".to_string()))?;

    rows.into_iter()
        .map(|row| {
            row.try_get("order_id")
                .map_err(|_| ApiError::Internal("用户合买订单编号读取失败".to_string()))
        })
        .collect()
}

/// 数据库模式下按真实订单 ID 批量读取合买计划，并加载参与记录。
async fn query_group_buy_plans_for_order_ids(
    database: &BusinessDatabase,
    order_ids: &[String],
) -> ApiResult<Vec<GroupBuyPlan>> {
    if order_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, initiator_user_id, initiator_username,
                order_id, issue, rule_code, title, numbers,
                total_amount_minor, filled_amount_minor, min_share_amount_minor,
                participant_min_amount_minor, share_count, status, note, created_at, updated_at
         FROM group_buy_plans
         WHERE order_id = ANY($1::text[])
         ORDER BY issue DESC, created_at DESC, id DESC",
    )
    .bind(order_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("订单合买计划数据读取失败".to_string()))?;
    let mut plans = rows
        .into_iter()
        .map(group_buy_plan_from_row)
        .map(|result| result.map(|plan| (plan.id.clone(), plan)))
        .collect::<ApiResult<BTreeMap<_, _>>>()?;
    attach_participants_to_plans(database, &mut plans, "订单合买参与人数据读取失败").await?;

    Ok(sorted_group_buy_plans(plans.values())
        .into_iter()
        .cloned()
        .collect())
}

/// 批量为已加载的合买计划补充参与人，避免多个分页查询重复写装配逻辑。
async fn attach_participants_to_plans(
    database: &BusinessDatabase,
    plans: &mut BTreeMap<String, GroupBuyPlan>,
    error_message: &'static str,
) -> ApiResult<()> {
    if plans.is_empty() {
        return Ok(());
    }
    let plan_ids = plans.keys().cloned().collect::<Vec<_>>();
    let participant_rows = sqlx::query(
        "SELECT id, plan_id, user_id, username, amount_minor, share_count, note, created_at
         FROM group_buy_participants
         WHERE plan_id = ANY($1::text[])
         ORDER BY plan_id ASC, created_at DESC, id DESC",
    )
    .bind(&plan_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal(error_message.to_string()))?;
    for row in participant_rows {
        let (plan_id, participant) = group_buy_participant_from_row(row)?;
        if let Some(plan) = plans.get_mut(&plan_id) {
            plan.participants.push(participant);
        }
    }
    Ok(())
}

/// 数据库模式下读取控奖页面所需的当前期合买计划，并批量加载参与记录。
async fn query_control_group_buy_details_for_issue(
    database: &BusinessDatabase,
    lottery_id: &str,
    issue: &str,
) -> ApiResult<Vec<GroupBuyPlan>> {
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, initiator_user_id, initiator_username,
                order_id, issue, rule_code, title, numbers,
                total_amount_minor, filled_amount_minor, min_share_amount_minor,
                participant_min_amount_minor, share_count, status, note, created_at, updated_at
         FROM group_buy_plans
         WHERE lottery_id = $1
           AND issue = $2
           AND status IN ('draft', 'open', 'filled')
         ORDER BY created_at DESC, id DESC",
    )
    .bind(lottery_id)
    .bind(issue)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("控奖合买计划数据读取失败".to_string()))?;
    let mut plans = rows
        .into_iter()
        .map(group_buy_plan_from_row)
        .map(|result| result.map(|plan| (plan.id.clone(), plan)))
        .collect::<ApiResult<BTreeMap<_, _>>>()?;
    if plans.is_empty() {
        return Ok(Vec::new());
    }

    let plan_ids = plans.keys().cloned().collect::<Vec<_>>();
    let participant_rows = sqlx::query(
        "SELECT id, plan_id, user_id, username, amount_minor, share_count, note, created_at
         FROM group_buy_participants
         WHERE plan_id = ANY($1)
         ORDER BY plan_id ASC, created_at DESC, id DESC",
    )
    .bind(&plan_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("控奖合买认购记录读取失败".to_string()))?;
    for row in participant_rows {
        let (plan_id, participant) = group_buy_participant_from_row(row)?;
        if let Some(plan) = plans.get_mut(&plan_id) {
            plan.participants.push(participant);
        }
    }

    Ok(sorted_group_buy_plans(plans.values())
        .into_iter()
        .cloned()
        .collect())
}

/// 控奖页面只需要发起人自购单，把跟单参与人从专用接口响应中剔除。
fn control_group_buy_plan_with_initiator_participants(
    mut plan: GroupBuyPlan,
) -> Option<GroupBuyPlan> {
    let initiator_user_id = plan.initiator_user_id.trim().to_string();
    plan.participants
        .retain(|participant| participant.user_id.trim() == initiator_user_id);
    if plan.participants.is_empty() {
        return None;
    }
    Some(plan)
}

/// 数据库模式下分页读取用户端合买大厅活跃计划，并加载当前页参与人。
async fn query_active_group_buy_details_page(
    database: &BusinessDatabase,
    lottery_ids: &[String],
    status_filter: Option<GroupBuyPlanStatus>,
    page: PageRequest,
) -> ApiResult<ListPage<GroupBuyPlan>> {
    let filter_by_lottery = !lottery_ids.is_empty();
    let lottery_ids = lottery_ids.to_vec();
    let status_clause = match &status_filter {
        Some(status) => format!("status = '{}'", enum_to_string(status)?),
        None => "status IN ('draft', 'open', 'filled')".to_string(),
    };
    let total_count = sqlx::query_scalar::<_, i64>(&format!(
        "SELECT COUNT(*) FROM group_buy_plans WHERE {status_clause} AND ($1::bool = false OR lottery_id = ANY($2))"
    ))
    .bind(filter_by_lottery)
    .bind(&lottery_ids)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("合买大厅分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("合买大厅分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(&format!(
        "SELECT id, lottery_id, lottery_name, initiator_user_id, initiator_username,
                order_id, issue, rule_code, title, numbers,
                total_amount_minor, filled_amount_minor, min_share_amount_minor,
                participant_min_amount_minor, share_count, status, note, created_at, updated_at
         FROM group_buy_plans
         WHERE {status_clause}
           AND ($1::bool = false OR lottery_id = ANY($2))
         ORDER BY created_at DESC, id DESC
         LIMIT $3 OFFSET $4"
    ))
    .bind(filter_by_lottery)
    .bind(&lottery_ids)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("合买大厅分页数据读取失败".to_string()))?;
    let mut plans = rows
        .into_iter()
        .map(group_buy_plan_from_row)
        .map(|result| result.map(|plan| (plan.id.clone(), plan)))
        .collect::<ApiResult<BTreeMap<_, _>>>()?;
    let plan_ids = plans.keys().cloned().collect::<Vec<_>>();
    if !plan_ids.is_empty() {
        let participant_rows = sqlx::query(
            "SELECT id, plan_id, user_id, username, amount_minor, share_count, note, created_at
             FROM group_buy_participants
             WHERE plan_id = ANY($1)
             ORDER BY plan_id ASC, id ASC",
        )
        .bind(&plan_ids)
        .fetch_all(database.pool())
        .await
        .map_err(|_| ApiError::Internal("合买大厅参与人数据读取失败".to_string()))?;
        for row in participant_rows {
            let (plan_id, participant) = group_buy_participant_from_row(row)?;
            if let Some(plan) = plans.get_mut(&plan_id) {
                plan.participants.push(participant);
            }
        }
    }
    let items = sorted_group_buy_details_by_created_at(plans.into_values().collect());

    Ok(ListPage::new(items, resolved))
}

/// 把合买计划和参与记录运行时快照保存到数据库。
async fn save_group_buy_store(database: &BusinessDatabase, store: &GroupBuyStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("合买事务开启失败".to_string()))?;

    save_group_buy_store_in_transaction(&mut *tx, store).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("合买事务提交失败".to_string()))
}

/// 在外层事务中保存合买计划和参与记录运行时快照，供跨仓储事务复用。
pub(crate) async fn save_group_buy_store_in_transaction(
    connection: &mut PgConnection,
    store: &GroupBuyStore,
) -> ApiResult<()> {
    sqlx::query("LOCK TABLE group_buy_participants, group_buy_plans IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *connection)
        .await
        .map_err(|error| {
            tracing::error!(%error, "合买数据表锁定失败");
            ApiError::Internal("合买数据表锁定失败".to_string())
        })?;

    for table in ["group_buy_participants", "group_buy_plans"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *connection)
            .await
            .map_err(|error| {
                tracing::error!(%error, table, "合买数据清理失败");
                ApiError::Internal("合买数据清理失败".to_string())
            })?;
    }

    for plan in store.plans.values() {
        sqlx::query(
            "INSERT INTO group_buy_plans
             (id, lottery_id, lottery_name, initiator_user_id, initiator_username,
              order_id, issue, rule_code, title, numbers,
              total_amount_minor, filled_amount_minor, min_share_amount_minor,
              participant_min_amount_minor, share_count, status, note, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)",
        )
        .bind(&plan.id)
        .bind(&plan.lottery_id)
        .bind(&plan.lottery_name)
        .bind(&plan.initiator_user_id)
        .bind(&plan.initiator_username)
        .bind(&plan.order_id)
        .bind(&plan.issue)
        .bind(&plan.rule_code)
        .bind(&plan.title)
        .bind(&plan.numbers)
        .bind(plan.total_amount_minor)
        .bind(plan.filled_amount_minor)
        .bind(plan.min_share_amount_minor)
        .bind(plan.participant_min_amount_minor)
        .bind(
            i32::try_from(plan.share_count)
                .map_err(|_| ApiError::Internal("合买计划份数过大".to_string()))?,
        )
        .bind(enum_to_string(&plan.status)?)
        .bind(&plan.note)
        .bind(&plan.created_at)
        .bind(&plan.updated_at)
        .execute(&mut *connection)
        .await
        .map_err(|error| {
            tracing::error!(
                %error,
                group_buy_plan_id = plan.id.as_str(),
                lottery_id = plan.lottery_id.as_str(),
                issue = plan.issue.as_str(),
                "合买计划数据保存失败"
            );
            ApiError::Internal("合买计划数据保存失败".to_string())
        })?;

        for participant in &plan.participants {
            let share_count = i32::try_from(participant.share_count)
                .map_err(|_| ApiError::Internal("合买参与人份数过大".to_string()))?;
            sqlx::query(
                "INSERT INTO group_buy_participants
                 (id, plan_id, user_id, username, amount_minor, share_count, note, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&participant.id)
            .bind(&plan.id)
            .bind(&participant.user_id)
            .bind(&participant.username)
            .bind(participant.amount_minor)
            .bind(share_count)
            .bind(&participant.note)
            .bind(&participant.created_at)
            .execute(&mut *connection)
            .await
            .map_err(|error| {
                tracing::error!(
                    %error,
                    group_buy_plan_id = plan.id.as_str(),
                    participant_id = participant.id.as_str(),
                    user_id = participant.user_id.as_str(),
                    amount_minor = participant.amount_minor,
                    share_count,
                    "合买参与人数据保存失败"
                );
                ApiError::Internal("合买参与人数据保存失败".to_string())
            })?;
        }
    }

    Ok(())
}

/// 在外层事务中按前后快照差异保存合买数据，避免认购或结算时重写全量合买记录。
pub(crate) async fn save_group_buy_store_incremental_in_transaction(
    connection: &mut PgConnection,
    previous: &GroupBuyStore,
    store: &GroupBuyStore,
) -> ApiResult<()> {
    for plan_id in previous
        .plans
        .keys()
        .filter(|plan_id| !store.plans.contains_key(*plan_id))
    {
        sqlx::query("DELETE FROM group_buy_participants WHERE plan_id = $1")
            .bind(plan_id)
            .execute(&mut *connection)
            .await
            .map_err(|error| {
                tracing::error!(%error, group_buy_plan_id = plan_id.as_str(), "合买参与人数据删除失败");
                ApiError::Internal("合买参与人数据删除失败".to_string())
            })?;
        sqlx::query("DELETE FROM group_buy_plans WHERE id = $1")
            .bind(plan_id)
            .execute(&mut *connection)
            .await
            .map_err(|error| {
                tracing::error!(%error, group_buy_plan_id = plan_id.as_str(), "合买计划数据删除失败");
                ApiError::Internal("合买计划数据删除失败".to_string())
            })?;
    }

    for (plan_id, plan) in &store.plans {
        if previous.plans.get(plan_id) == Some(plan) {
            continue;
        }
        upsert_group_buy_plan_in_transaction(connection, plan).await?;
    }

    let previous_participants = group_buy_participants_by_id(previous);
    let current_participants = group_buy_participants_by_id(store);
    for participant_id in previous_participants
        .keys()
        .filter(|participant_id| !current_participants.contains_key(*participant_id))
    {
        sqlx::query("DELETE FROM group_buy_participants WHERE id = $1")
            .bind(participant_id)
            .execute(&mut *connection)
            .await
            .map_err(|error| {
                tracing::error!(%error, participant_id = *participant_id, "合买参与人数据删除失败");
                ApiError::Internal("合买参与人数据删除失败".to_string())
            })?;
    }

    for (participant_id, (plan_id, participant)) in &current_participants {
        let unchanged = previous_participants
            .get(participant_id)
            .map(|(previous_plan_id, previous_participant)| {
                previous_plan_id == plan_id && previous_participant == participant
            })
            .unwrap_or_default();
        if unchanged {
            continue;
        }
        upsert_group_buy_participant_in_transaction(connection, plan_id, participant).await?;
    }

    Ok(())
}

/// 把合买参与人按参与记录 ID 建立索引，便于增量保存判断新增、删除和修改。
fn group_buy_participants_by_id(
    store: &GroupBuyStore,
) -> BTreeMap<&str, (&str, &GroupBuyParticipant)> {
    store
        .plans
        .iter()
        .flat_map(|(plan_id, plan)| {
            plan.participants
                .iter()
                .map(move |participant| (participant.id.as_str(), (plan_id.as_str(), participant)))
        })
        .collect()
}

/// 在事务中插入或更新单个合买计划。
async fn upsert_group_buy_plan_in_transaction(
    connection: &mut PgConnection,
    plan: &GroupBuyPlan,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO group_buy_plans
         (id, lottery_id, lottery_name, initiator_user_id, initiator_username,
          order_id, issue, rule_code, title, numbers,
          total_amount_minor, filled_amount_minor, min_share_amount_minor,
          participant_min_amount_minor, share_count, status, note, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
         ON CONFLICT (id) DO UPDATE SET
            lottery_id = EXCLUDED.lottery_id,
            lottery_name = EXCLUDED.lottery_name,
            initiator_user_id = EXCLUDED.initiator_user_id,
            initiator_username = EXCLUDED.initiator_username,
            order_id = EXCLUDED.order_id,
            issue = EXCLUDED.issue,
            rule_code = EXCLUDED.rule_code,
            title = EXCLUDED.title,
            numbers = EXCLUDED.numbers,
            total_amount_minor = EXCLUDED.total_amount_minor,
            filled_amount_minor = EXCLUDED.filled_amount_minor,
            min_share_amount_minor = EXCLUDED.min_share_amount_minor,
            participant_min_amount_minor = EXCLUDED.participant_min_amount_minor,
            share_count = EXCLUDED.share_count,
            status = EXCLUDED.status,
            note = EXCLUDED.note,
            created_at = EXCLUDED.created_at,
            updated_at = EXCLUDED.updated_at",
    )
    .bind(&plan.id)
    .bind(&plan.lottery_id)
    .bind(&plan.lottery_name)
    .bind(&plan.initiator_user_id)
    .bind(&plan.initiator_username)
    .bind(&plan.order_id)
    .bind(&plan.issue)
    .bind(&plan.rule_code)
    .bind(&plan.title)
    .bind(&plan.numbers)
    .bind(plan.total_amount_minor)
    .bind(plan.filled_amount_minor)
    .bind(plan.min_share_amount_minor)
    .bind(plan.participant_min_amount_minor)
    .bind(
        i32::try_from(plan.share_count)
            .map_err(|_| ApiError::Internal("合买计划份数过大".to_string()))?,
    )
    .bind(enum_to_string(&plan.status)?)
    .bind(&plan.note)
    .bind(&plan.created_at)
    .bind(&plan.updated_at)
    .execute(&mut *connection)
    .await
    .map_err(|error| {
        tracing::error!(
            %error,
            group_buy_plan_id = plan.id.as_str(),
            lottery_id = plan.lottery_id.as_str(),
            issue = plan.issue.as_str(),
            "合买计划数据保存失败"
        );
        ApiError::Internal("合买计划数据保存失败".to_string())
    })?;
    Ok(())
}

/// 在事务中插入或更新单个合买参与人。
async fn upsert_group_buy_participant_in_transaction(
    connection: &mut PgConnection,
    plan_id: &str,
    participant: &GroupBuyParticipant,
) -> ApiResult<()> {
    let share_count = i32::try_from(participant.share_count)
        .map_err(|_| ApiError::Internal("合买参与人份数过大".to_string()))?;
    sqlx::query(
        "INSERT INTO group_buy_participants
         (id, plan_id, user_id, username, amount_minor, share_count, note, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (id) DO UPDATE SET
            plan_id = EXCLUDED.plan_id,
            user_id = EXCLUDED.user_id,
            username = EXCLUDED.username,
            amount_minor = EXCLUDED.amount_minor,
            share_count = EXCLUDED.share_count,
            note = EXCLUDED.note,
            created_at = EXCLUDED.created_at",
    )
    .bind(&participant.id)
    .bind(plan_id)
    .bind(&participant.user_id)
    .bind(&participant.username)
    .bind(participant.amount_minor)
    .bind(share_count)
    .bind(&participant.note)
    .bind(&participant.created_at)
    .execute(&mut *connection)
    .await
    .map_err(|error| {
        tracing::error!(
            %error,
            group_buy_plan_id = plan_id,
            participant_id = participant.id.as_str(),
            user_id = participant.user_id.as_str(),
            amount_minor = participant.amount_minor,
            share_count,
            "合买参与人数据保存失败"
        );
        ApiError::Internal("合买参与人数据保存失败".to_string())
    })?;
    Ok(())
}

/// 合买计划和参与记录运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl GroupBuyStore {
    /// 构建并返回种子数据。
    fn seeded() -> Self {
        let plans = seed_group_buy_plans()
            .into_iter()
            .map(|plan| (plan.id.clone(), plan))
            .collect();

        Self { plans }
    }

    /// 按当前仓储快照返回全部合买计划列表。
    fn list(&self) -> Vec<GroupBuyPlanSummary> {
        sorted_group_buy_plans(self.plans.values())
            .into_iter()
            .map(GroupBuyPlan::summary)
            .collect()
    }

    /// 返回完整计划列表。
    fn list_details(&self) -> Vec<GroupBuyPlan> {
        sorted_group_buy_plans(self.plans.values())
            .into_iter()
            .cloned()
            .collect()
    }

    /// 返回指定用户发起或参与过的合买计划列表。
    fn list_details_for_user(&self, user_id: &str) -> Vec<GroupBuyPlan> {
        sorted_group_buy_plans(self.plans.values())
            .into_iter()
            .filter(|plan| group_buy_plan_related_to_user(plan, user_id))
            .cloned()
            .collect()
    }

    /// 返回指定参与人集合关联的有效合买计划。
    fn list_active_details_for_participant_user_ids(
        &self,
        user_ids: &BTreeSet<String>,
    ) -> Vec<GroupBuyPlan> {
        sorted_group_buy_plans(self.plans.values())
            .into_iter()
            .filter(|plan| {
                !matches!(plan.status, GroupBuyPlanStatus::Cancelled)
                    && plan.participants.iter().any(|participant| {
                        user_ids.contains(&participant.user_id) && participant.amount_minor > 0
                    })
            })
            .cloned()
            .collect()
    }

    /// 返回控奖页面当前期仍在流转中的合买计划，服务层会再裁剪为发起人自购记录。
    fn list_control_details_for_issue(&self, lottery_id: &str, issue: &str) -> Vec<GroupBuyPlan> {
        sorted_group_buy_plans(self.plans.values())
            .into_iter()
            .filter(|plan| {
                plan.lottery_id == lottery_id
                    && plan.issue == issue
                    && matches!(
                        plan.status,
                        GroupBuyPlanStatus::Draft
                            | GroupBuyPlanStatus::Open
                            | GroupBuyPlanStatus::Filled
                    )
            })
            .cloned()
            .collect()
    }

    /// 清除已取消或已结算的合买历史；未结算计划直接保留，不阻断本次清理。
    fn clear_records(&mut self) -> ApiResult<usize> {
        let before_count = self.plans.len();
        self.plans.retain(|_, plan| {
            !matches!(
                plan.status,
                GroupBuyPlanStatus::Cancelled | GroupBuyPlanStatus::Settled
            )
        });
        let deleted_count = before_count.saturating_sub(self.plans.len());
        Ok(deleted_count)
    }

    /// 返回可被机器人批量清理入口删除的合买计划，发起人和所有参与人都必须属于机器人账号集合。
    pub(crate) fn robot_cleanup_candidates(
        &self,
        robot_user_ids: &BTreeSet<String>,
    ) -> Vec<GroupBuyPlan> {
        sorted_group_buy_plans(self.plans.values())
            .into_iter()
            .filter(|plan| {
                robot_user_ids.contains(plan.initiator_user_id.as_str())
                    && plan
                        .participants
                        .iter()
                        .all(|participant| robot_user_ids.contains(participant.user_id.as_str()))
            })
            .cloned()
            .collect()
    }

    /// 批量删除指定合买计划，供机器人清理接口与订单仓储同事务保存使用。
    pub(crate) fn delete_plans_by_ids(&mut self, plan_ids: &BTreeSet<String>) -> Vec<GroupBuyPlan> {
        let mut deleted = Vec::new();
        for plan_id in plan_ids {
            if let Some(plan) = self.plans.remove(plan_id) {
                deleted.push(plan);
            }
        }
        deleted
    }

    /// 按订单 ID 查找已经成单的合买计划。
    pub(crate) fn plans_for_order_ids(&self, order_ids: &[String]) -> Vec<GroupBuyPlan> {
        self.plans
            .values()
            .filter(|plan| {
                plan.order_id
                    .as_ref()
                    .is_some_and(|order_id| order_ids.iter().any(|id| id == order_id))
            })
            .cloned()
            .collect()
    }

    /// 按标识查询并返回单条记录。
    fn get(&self, id: &str) -> ApiResult<GroupBuyPlan> {
        self.plans
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))
    }

    /// 校验入参并创建新记录。
    pub(crate) fn create(
        &mut self,
        request: CreateGroupBuyPlanRequest,
        lotteries: &[LotteryKind],
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        let id = required_trimmed(request.id, "group buy plan id")?;
        if self.plans.contains_key(&id) {
            return Err(ApiError::Conflict(format!(
                "group buy plan `{id}` already exists"
            )));
        }

        let lottery_id = required_trimmed(request.lottery_id, "lottery id")?;
        let lottery = find_lottery(lotteries, &lottery_id)?;
        if !lottery.group_buy.enabled {
            return Err(ApiError::BadRequest(format!(
                "lottery `{lottery_id}` group buy is disabled"
            )));
        }

        let initiator_user_id = required_trimmed(request.initiator_user_id, "initiator user id")?;
        let initiator = find_user(users, &initiator_user_id)?;
        validate_positive_amount(request.total_amount_minor, "total amount")?;
        validate_positive_amount(request.initiator_amount_minor, "initiator amount")?;
        validate_share_amount(
            request.total_amount_minor,
            lottery.group_buy.min_share_amount_minor,
            "total amount",
        )?;
        validate_share_amount(
            request.initiator_amount_minor,
            lottery.group_buy.min_share_amount_minor,
            "initiator amount",
        )?;

        if request.initiator_amount_minor > request.total_amount_minor {
            return Err(ApiError::BadRequest(
                "initiator amount cannot exceed total amount".to_string(),
            ));
        }

        let minimum_initiator_amount = minimum_amount_by_percent(
            request.total_amount_minor,
            lottery.group_buy.initiator_min_percent,
        )?;
        if request.initiator_amount_minor < minimum_initiator_amount {
            return Err(ApiError::BadRequest(format!(
                "initiator amount must be at least {minimum_initiator_amount}"
            )));
        }

        let now = current_time_label();
        let issue = required_trimmed(request.issue, "group buy issue")?;
        let rule_code = required_trimmed(request.rule_code, "group buy rule code")?;
        let title = optional_title(&request.title, &lottery.name, &issue);
        let numbers = required_trimmed(request.numbers, "group buy numbers")?;
        let participant = GroupBuyParticipant {
            id: format!("{id}-P001"),
            user_id: initiator.id.clone(),
            username: initiator.username.clone(),
            amount_minor: request.initiator_amount_minor,
            share_count: share_count(
                request.initiator_amount_minor,
                lottery.group_buy.min_share_amount_minor,
            )?,
            note: "发起人认购".to_string(),
            created_at: now.clone(),
        };
        let status = if request.initiator_amount_minor == request.total_amount_minor {
            GroupBuyPlanStatus::Filled
        } else {
            GroupBuyPlanStatus::Open
        };
        let plan = GroupBuyPlan {
            id: id.clone(),
            lottery_id: lottery.id.clone(),
            lottery_name: lottery.name.clone(),
            order_id: None,
            issue,
            rule_code,
            title,
            numbers,
            initiator_user_id: initiator.id.clone(),
            initiator_username: initiator.username.clone(),
            total_amount_minor: request.total_amount_minor,
            filled_amount_minor: request.initiator_amount_minor,
            min_share_amount_minor: lottery.group_buy.min_share_amount_minor,
            participant_min_amount_minor: lottery.group_buy.participant_min_amount_minor,
            share_count: share_count(
                request.total_amount_minor,
                lottery.group_buy.min_share_amount_minor,
            )?,
            status,
            participants: vec![participant],
            note: request.note.trim().to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.plans.insert(id, plan.clone());
        Ok(plan)
    }

    /// 校验入参并更新指定记录。
    fn update(&mut self, id: &str, request: UpdateGroupBuyPlanRequest) -> ApiResult<GroupBuyPlan> {
        let plan = self
            .plans
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))?;

        if matches!(plan.status, GroupBuyPlanStatus::Settled)
            && !matches!(request.status, GroupBuyPlanStatus::Settled)
        {
            return Err(ApiError::BadRequest(
                "settled group buy plan cannot change status".to_string(),
            ));
        }
        if matches!(
            request.status,
            GroupBuyPlanStatus::Filled | GroupBuyPlanStatus::Settled
        ) && plan.filled_amount_minor < plan.total_amount_minor
        {
            return Err(ApiError::BadRequest(
                "group buy plan must be fully filled before filled or settled status".to_string(),
            ));
        }
        if matches!(request.status, GroupBuyPlanStatus::Settled) && plan.order_id.is_none() {
            return Err(ApiError::BadRequest(
                "group buy plan must create order before settled status".to_string(),
            ));
        }

        plan.status = request.status;
        plan.note = request.note.trim().to_string();
        plan.updated_at = current_time_label();

        Ok(plan.clone())
    }

    /// 关联真实投注订单号，避免同一合买重复成单。
    pub(crate) fn attach_order(&mut self, id: &str, order_id: &str) -> ApiResult<GroupBuyPlan> {
        let plan = self
            .plans
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))?;
        let order_id = required_trimmed(order_id.to_string(), "group buy order id")?;
        if !matches!(plan.status, GroupBuyPlanStatus::Filled) {
            return Err(ApiError::BadRequest(
                "group buy plan must be filled before creating order".to_string(),
            ));
        }
        if let Some(existing_order_id) = &plan.order_id {
            if existing_order_id == &order_id {
                return Ok(plan.clone());
            }
            return Err(ApiError::Conflict(format!(
                "group buy plan `{id}` already has order `{existing_order_id}`"
            )));
        }

        plan.order_id = Some(order_id);
        plan.updated_at = current_time_label();
        Ok(plan.clone())
    }

    /// 按订单 ID 将已结算合买计划标记为已结算。
    pub(crate) fn mark_settled_by_order_ids(&mut self, order_ids: &[String]) -> Vec<GroupBuyPlan> {
        let mut settled = Vec::new();
        for plan in self.plans.values_mut() {
            let matches_order = plan
                .order_id
                .as_ref()
                .is_some_and(|order_id| order_ids.iter().any(|id| id == order_id));
            if matches_order
                && matches!(
                    plan.status,
                    GroupBuyPlanStatus::Filled | GroupBuyPlanStatus::Open
                )
            {
                plan.status = GroupBuyPlanStatus::Settled;
                plan.updated_at = current_time_label();
                settled.push(plan.clone());
            }
        }
        settled
    }

    /// 封盘时取消仍未满单的合买计划。
    pub(crate) fn cancel_unfilled_for_issue(
        &mut self,
        lottery_id: &str,
        issue: &str,
    ) -> Vec<GroupBuyPlan> {
        self.cancel_unfilled_for_issue_except(lottery_id, issue, &HashSet::new())
    }

    /// 封盘时取消仍未满单的合买计划，跳过保护名单中的计划。
    pub(crate) fn cancel_unfilled_for_issue_except(
        &mut self,
        lottery_id: &str,
        issue: &str,
        protected_plan_ids: &HashSet<String>,
    ) -> Vec<GroupBuyPlan> {
        let lottery_id = lottery_id.trim();
        let issue = issue.trim();
        let mut cancelled = Vec::new();
        for plan in self.plans.values_mut() {
            if plan.lottery_id == lottery_id
                && plan.issue == issue
                && !protected_plan_ids.contains(&plan.id)
                && plan.filled_amount_minor < plan.total_amount_minor
                && matches!(
                    plan.status,
                    GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open
                )
            {
                plan.status = GroupBuyPlanStatus::Cancelled;
                plan.updated_at = current_time_label();
                cancelled.push(plan.clone());
            }
        }
        cancelled
    }

    /// 为合买计划追加参与人并更新认购进度。
    pub(crate) fn add_participant(
        &mut self,
        id: &str,
        request: AddGroupBuyParticipantRequest,
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        let plan = self
            .plans
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))?;

        if !matches!(
            plan.status,
            GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open
        ) {
            return Err(ApiError::BadRequest(
                "group buy plan is not open for participation".to_string(),
            ));
        }

        let participant_id = required_trimmed(request.id, "group buy participant id")?;
        if plan
            .participants
            .iter()
            .any(|participant| participant.id == participant_id)
        {
            return Err(ApiError::Conflict(format!(
                "group buy participant `{participant_id}` already exists"
            )));
        }

        let user_id = required_trimmed(request.user_id, "participant user id")?;
        let user = find_user(users, &user_id)?;
        validate_positive_amount(request.amount_minor, "participant amount")?;
        validate_share_amount(
            request.amount_minor,
            plan.min_share_amount_minor,
            "participant amount",
        )?;

        let remaining_before = plan
            .total_amount_minor
            .checked_sub(plan.filled_amount_minor)
            .ok_or_else(|| ApiError::Internal("合买计划认购进度数据异常".to_string()))?;
        let minimum_participant_amount = participant_min_amount(plan);
        if request.amount_minor < minimum_participant_amount
            && request.amount_minor != remaining_before
        {
            return Err(ApiError::BadRequest(format!(
                "认购金额不能低于最低认购金额 {} 分",
                minimum_participant_amount
            )));
        }

        let next_filled = plan
            .filled_amount_minor
            .checked_add(request.amount_minor)
            .ok_or_else(|| ApiError::Internal("group buy filled amount overflow".to_string()))?;
        if next_filled > plan.total_amount_minor {
            return Err(ApiError::BadRequest(
                "认购金额超过剩余可认购金额".to_string(),
            ));
        }
        let remaining_after = plan
            .total_amount_minor
            .checked_sub(next_filled)
            .ok_or_else(|| ApiError::Internal("合买计划剩余金额计算异常".to_string()))?;
        if remaining_after > 0 && remaining_after < minimum_participant_amount {
            let minimum_required = remaining_before
                .checked_sub(remaining_after)
                .and_then(|amount| amount.checked_add(minimum_participant_amount))
                .ok_or_else(|| ApiError::Internal("合买最低认购金额计算异常".to_string()))?;
            return Err(ApiError::BadRequest(format!(
                "认购后剩余金额低于最低认购金额，请至少认购 {} 分或选择全包",
                minimum_required.min(remaining_before)
            )));
        }

        plan.participants.push(GroupBuyParticipant {
            id: participant_id,
            user_id: user.id.clone(),
            username: user.username.clone(),
            amount_minor: request.amount_minor,
            share_count: share_count(request.amount_minor, plan.min_share_amount_minor)?,
            note: request.note.trim().to_string(),
            created_at: current_time_label(),
        });
        plan.filled_amount_minor = next_filled;
        if plan.filled_amount_minor == plan.total_amount_minor {
            plan.status = GroupBuyPlanStatus::Filled;
        }
        plan.updated_at = current_time_label();

        Ok(plan.clone())
    }

    /// 移除指定合买计划。
    pub(crate) fn remove_plan(&mut self, id: &str) -> ApiResult<()> {
        self.plans
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))
    }

    /// 删除指定合买计划并返回被删除的数据，供后台删除前后展示和审计。
    pub(crate) fn delete_plan(&mut self, id: &str) -> ApiResult<GroupBuyPlan> {
        self.plans
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))
    }

    /// 移除指定参与记录，并回退进度和状态。
    pub(crate) fn remove_participant(
        &mut self,
        id: &str,
        participant_id: &str,
    ) -> ApiResult<GroupBuyPlan> {
        let plan = self
            .plans
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))?;
        let index = plan
            .participants
            .iter()
            .position(|participant| participant.id == participant_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "group buy participant `{participant_id}` not found"
                ))
            })?;
        let participant = plan.participants.remove(index);
        plan.filled_amount_minor = plan
            .filled_amount_minor
            .checked_sub(participant.amount_minor)
            .ok_or_else(|| ApiError::Internal("group buy filled amount underflow".to_string()))?;
        if matches!(plan.status, GroupBuyPlanStatus::Filled)
            && plan.filled_amount_minor < plan.total_amount_minor
        {
            plan.status = GroupBuyPlanStatus::Open;
            plan.order_id = None;
        }
        plan.updated_at = current_time_label();
        Ok(plan.clone())
    }
}

/// 按期号倒序返回合买计划，期号相同时再按创建时间和计划 ID 倒序稳定展示。
fn sorted_group_buy_plans<'a>(
    plans: impl Iterator<Item = &'a GroupBuyPlan>,
) -> Vec<&'a GroupBuyPlan> {
    let mut sorted_plans: Vec<_> = plans.collect();
    sorted_plans.sort_by(|left, right| {
        right
            .issue
            .cmp(&left.issue)
            .then_with(|| right.created_at.cmp(&left.created_at))
            .then_with(|| right.id.cmp(&left.id))
    });
    sorted_plans
}

/// 后台列表按创建时间倒序展示，方便运营优先看到最新生成的合买计划。
fn sorted_group_buy_summaries_by_created_at(
    plans: Vec<GroupBuyPlanSummary>,
) -> Vec<GroupBuyPlanSummary> {
    let mut sorted_plans = plans;
    sorted_plans.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    sorted_plans
}

/// 用户端“我的合买”按创建时间倒序展示，避免内存模式沿用期号排序。
fn sorted_group_buy_details_by_created_at(plans: Vec<GroupBuyPlan>) -> Vec<GroupBuyPlan> {
    let mut sorted_plans = plans;
    sorted_plans.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    sorted_plans
}

/// 归一化用户 ID 集合，去重并移除空值。
fn normalized_user_ids(user_ids: &[String]) -> BTreeSet<String> {
    user_ids
        .iter()
        .map(|user_id| user_id.trim())
        .filter(|user_id| !user_id.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// 归一化多个筛选值，空字符串不参与过滤并自动去重。
fn normalized_filter_values(values: &[&str]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// 计算合买计划当前最小可认购金额。
fn participant_min_amount(plan: &GroupBuyPlan) -> i64 {
    plan.participant_min_amount_minor
        .max(plan.min_share_amount_minor)
        .max(1)
}

/// 在列表中查找指定彩种。
fn find_lottery<'a>(lotteries: &'a [LotteryKind], id: &str) -> ApiResult<&'a LotteryKind> {
    lotteries
        .iter()
        .find(|lottery| lottery.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
}

/// 在列表中查找指定用户。
fn find_user<'a>(users: &'a [UserSummary], id: &str) -> ApiResult<&'a UserSummary> {
    users
        .iter()
        .find(|user| user.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))
}

/// 校验金额必须大于 0。
fn validate_positive_amount(amount: i64, label: &str) -> ApiResult<()> {
    if amount <= 0 {
        return Err(ApiError::BadRequest(format!(
            "{label} must be greater than zero"
        )));
    }

    Ok(())
}

/// 校验金额必须符合单份金额倍数。
fn validate_share_amount(amount: i64, min_share_amount_minor: i64, label: &str) -> ApiResult<()> {
    if min_share_amount_minor <= 0 {
        return Err(ApiError::BadRequest(
            "group buy min share amount must be greater than zero".to_string(),
        ));
    }

    if amount % min_share_amount_minor != 0 {
        return Err(ApiError::BadRequest(format!(
            "{label} must be divisible by min share amount"
        )));
    }

    Ok(())
}

/// 按总金额和比例计算最低认购金额。
fn minimum_amount_by_percent(total_amount_minor: i64, percent: u8) -> ApiResult<i64> {
    let raw = total_amount_minor
        .checked_mul(i64::from(percent))
        .ok_or_else(|| ApiError::Internal("group buy minimum amount overflow".to_string()))?;

    Ok((raw + 99) / 100)
}

/// 按金额和单份金额计算份数。
fn share_count(amount_minor: i64, min_share_amount_minor: i64) -> ApiResult<u32> {
    validate_share_amount(amount_minor, min_share_amount_minor, "amount")?;
    let shares = amount_minor / min_share_amount_minor;
    u32::try_from(shares)
        .map_err(|_| ApiError::BadRequest("group buy share count is too large".to_string()))
}

/// 生成合买计划标题，用户未填写时使用彩种和期号兜底。
fn optional_title(title: &str, lottery_name: &str, issue: &str) -> String {
    let title = title.trim();
    if !title.is_empty() {
        return title.to_string();
    }

    if issue.trim().is_empty() {
        format!("{lottery_name} 合买计划")
    } else {
        format!("{lottery_name} 第{issue}期合买")
    }
}

/// 去除空白并校验必填字段。
fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

/// 返回当前时间的展示文本。
fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 返回内置合买计划种子数据。
fn seed_group_buy_plans() -> Vec<GroupBuyPlan> {
    vec![GroupBuyPlan {
        id: "G202606020001".to_string(),
        lottery_id: "fc3d".to_string(),
        lottery_name: "福彩 3D".to_string(),
        order_id: None,
        issue: "20260602001".to_string(),
        rule_code: "threeDirect".to_string(),
        title: "福彩 3D 第20260602001期合买".to_string(),
        numbers: "1,2,3".to_string(),
        initiator_user_id: "U90001".to_string(),
        initiator_username: "agent_alpha".to_string(),
        total_amount_minor: 100_000,
        filled_amount_minor: 72_000,
        min_share_amount_minor: 100,
        participant_min_amount_minor: 1_000,
        share_count: 1_000,
        status: GroupBuyPlanStatus::Open,
        participants: vec![
            GroupBuyParticipant {
                id: "G202606020001-P001".to_string(),
                user_id: "U90001".to_string(),
                username: "agent_alpha".to_string(),
                amount_minor: 10_000,
                share_count: 100,
                note: "发起人认购".to_string(),
                created_at: "2026-06-02 09:00:00".to_string(),
            },
            GroupBuyParticipant {
                id: "G202606020001-P002".to_string(),
                user_id: "U10001".to_string(),
                username: "demo_user".to_string(),
                amount_minor: 62_000,
                share_count: 620,
                note: "普通用户参与".to_string(),
                created_at: "2026-06-02 09:30:00".to_string(),
            },
        ],
        note: "默认合买计划示例".to_string(),
        created_at: "2026-06-02 09:00:00".to_string(),
        updated_at: "2026-06-02 09:30:00".to_string(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::lottery::LotteryKind,
        services::{access::AccessRepository, lottery::seed_lotteries},
    };
    /// 验证合买合买仓储创建计划带发起人参与人。
    #[tokio::test]
    async fn group_buy_repository_creates_plan_with_initiator_participant() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let lotteries = lotteries_with_group_buy_enabled("fc3d");
        let plan = repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "20260602001".to_string(),
                    rule_code: "threeDirect".to_string(),
                    title: "测试合买".to_string(),
                    numbers: "1,2,3".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 100_000,
                    initiator_amount_minor: 10_000,
                    note: "测试计划".to_string(),
                },
                &lotteries,
                &access.users,
            )
            .await
            .expect("valid group buy plan can be created");

        assert_eq!(plan.share_count, 1_000);
        assert_eq!(plan.filled_amount_minor, 10_000);
        assert_eq!(plan.status, GroupBuyPlanStatus::Open);
        assert_eq!(plan.participants.len(), 1);
        assert_eq!(plan.participants[0].share_count, 100);
    }
    /// 验证合买合买仓储列表最新期号优先。
    #[tokio::test]
    async fn group_buy_repository_lists_latest_issue_first() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let lotteries = lotteries_with_group_buy_enabled("fc3d");

        for (id, issue, title) in [
            ("G-ORDER-OLDER", "20260602002", "较旧期号"),
            ("G-ORDER-NEWER", "20260602003", "较新期号"),
        ] {
            repository
                .create(
                    CreateGroupBuyPlanRequest {
                        id: id.to_string(),
                        lottery_id: "fc3d".to_string(),
                        issue: issue.to_string(),
                        rule_code: "threeDirect".to_string(),
                        title: title.to_string(),
                        numbers: "1,2,3".to_string(),
                        initiator_user_id: "U90001".to_string(),
                        total_amount_minor: 100_000,
                        initiator_amount_minor: 10_000,
                        note: title.to_string(),
                    },
                    &lotteries,
                    &access.users,
                )
                .await
                .expect("plan can be created");
        }

        let summaries = repository
            .list()
            .await
            .expect("summary list can load")
            .into_iter()
            .collect::<Vec<_>>();
        let details = repository
            .list_details()
            .await
            .expect("detail list can load")
            .into_iter()
            .collect::<Vec<_>>();
        let summary_issues = summaries
            .iter()
            .map(|plan| plan.issue.as_str())
            .collect::<Vec<_>>();
        let detail_issues = details
            .iter()
            .map(|plan| plan.issue.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            &summary_issues[..3],
            ["20260602003", "20260602002", "20260602001"]
        );
        assert_eq!(
            &detail_issues[..3],
            ["20260602003", "20260602002", "20260602001"]
        );
    }

    /// 验证后台合买分页列表可按成单状态筛选，并且优先展示最新创建的计划。
    #[tokio::test]
    async fn group_buy_repository_list_page_filters_formation_and_sorts_by_created_at() {
        let mut first = seed_group_buy_plans()
            .into_iter()
            .next()
            .expect("seed plan exists");
        first.id = "G-FORMED-OLDER".to_string();
        first.order_id = Some("O-GROUP-001".to_string());
        first.created_at = "2026-06-02 09:00:00".to_string();

        let mut second = first.clone();
        second.id = "G-FORMED-NEWER".to_string();
        second.order_id = Some("O-GROUP-002".to_string());
        second.created_at = "2026-06-02 10:00:00".to_string();

        let mut unformed = first.clone();
        unformed.id = "G-UNFORMED".to_string();
        unformed.order_id = None;
        unformed.created_at = "2026-06-02 11:00:00".to_string();

        let repository = GroupBuyRepository {
            inner: Arc::new(RwLock::new(GroupBuyStore {
                plans: BTreeMap::from([
                    (first.id.clone(), first),
                    (second.id.clone(), second),
                    (unformed.id.clone(), unformed),
                ]),
            })),
            persistence: None,
            mutation_lock: Arc::new(Mutex::new(())),
        };

        let formed_page = repository
            .list_page(
                &[],
                Some(GroupBuyFormationFilter::Formed),
                None,
                PageRequest::new(Some(1), Some(10)),
            )
            .await
            .expect("formed page can load");
        let unformed_page = repository
            .list_page(
                &[],
                Some(GroupBuyFormationFilter::Unformed),
                None,
                PageRequest::new(Some(1), Some(10)),
            )
            .await
            .expect("unformed page can load");
        let id_filtered_page = repository
            .list_page(
                &[],
                None,
                Some("G-FORMED-OLDER"),
                PageRequest::new(Some(1), Some(10)),
            )
            .await
            .expect("plan id filtered page can load");

        assert_eq!(
            formed_page
                .items
                .iter()
                .map(|plan| plan.id.as_str())
                .collect::<Vec<_>>(),
            ["G-FORMED-NEWER", "G-FORMED-OLDER"]
        );
        assert_eq!(formed_page.total_count, 2);
        assert_eq!(unformed_page.items[0].id, "G-UNFORMED");
        assert_eq!(unformed_page.total_count, 1);
        assert_eq!(id_filtered_page.total_count, 1);
        assert_eq!(id_filtered_page.items[0].id, "G-FORMED-OLDER");
    }

    /// 验证后台合买分页按完整机器人账号集合过滤机器人发起计划。
    #[tokio::test]
    async fn group_buy_repository_list_page_excludes_robot_initiator_set() {
        let mut user_plan = seed_group_buy_plans()
            .into_iter()
            .next()
            .expect("seed plan exists");
        user_plan.id = "G-USER".to_string();
        user_plan.initiator_user_id = "U10001".to_string();

        let mut group_robot_plan = user_plan.clone();
        group_robot_plan.id = "G-ROBOT-U".to_string();
        group_robot_plan.initiator_user_id = "U90001".to_string();

        let mut fill_robot_plan = user_plan.clone();
        fill_robot_plan.id = "G-ROBOT-X".to_string();
        fill_robot_plan.initiator_user_id = "X90002".to_string();

        let repository = GroupBuyRepository {
            inner: Arc::new(RwLock::new(GroupBuyStore {
                plans: BTreeMap::from([
                    (user_plan.id.clone(), user_plan),
                    (group_robot_plan.id.clone(), group_robot_plan),
                    (fill_robot_plan.id.clone(), fill_robot_plan),
                ]),
            })),
            persistence: None,
            mutation_lock: Arc::new(Mutex::new(())),
        };

        let page = repository
            .list_page(
                &["U90001", "X90002"],
                None,
                None,
                PageRequest::new(Some(1), Some(20)),
            )
            .await
            .expect("group buy page can exclude robot initiators");

        assert_eq!(page.total_count, 1);
        assert_eq!(page.items[0].id, "G-USER");
    }

    /// 验证用户端未成单合买分页按创建时间倒序，而不是沿用期号排序。
    #[tokio::test]
    async fn group_buy_repository_unformed_user_page_sorts_by_created_at() {
        let mut older = seed_group_buy_plans()
            .into_iter()
            .next()
            .expect("seed plan exists");
        older.id = "G-UNFORMED-OLDER".to_string();
        older.issue = "20260602099".to_string();
        older.order_id = None;
        older.created_at = "2026-06-02 09:00:00".to_string();

        let mut newer = older.clone();
        newer.id = "G-UNFORMED-NEWER".to_string();
        newer.issue = "20260602001".to_string();
        newer.created_at = "2026-06-02 10:00:00".to_string();

        let repository = GroupBuyRepository {
            inner: Arc::new(RwLock::new(GroupBuyStore {
                plans: BTreeMap::from([(older.id.clone(), older), (newer.id.clone(), newer)]),
            })),
            persistence: None,
            mutation_lock: Arc::new(Mutex::new(())),
        };

        let page = repository
            .list_unformed_details_for_user_page("U10001", PageRequest::new(Some(1), Some(10)))
            .await
            .expect("unformed user page can load");

        assert_eq!(
            page.items
                .iter()
                .map(|plan| plan.id.as_str())
                .collect::<Vec<_>>(),
            ["G-UNFORMED-NEWER", "G-UNFORMED-OLDER"]
        );
    }

    /// 验证用户端合买大厅分页按创建时间倒序，而不是沿用期号排序。
    #[tokio::test]
    async fn group_buy_repository_active_hall_page_sorts_by_created_at() {
        let mut older = seed_group_buy_plans()
            .into_iter()
            .next()
            .expect("seed plan exists");
        older.id = "G-HALL-OLDER".to_string();
        older.issue = "20260602099".to_string();
        older.created_at = "2026-06-02 09:00:00".to_string();

        let mut newer = older.clone();
        newer.id = "G-HALL-NEWER".to_string();
        newer.issue = "20260602001".to_string();
        newer.created_at = "2026-06-02 10:00:00".to_string();

        let repository = GroupBuyRepository {
            inner: Arc::new(RwLock::new(GroupBuyStore {
                plans: BTreeMap::from([(older.id.clone(), older), (newer.id.clone(), newer)]),
            })),
            persistence: None,
            mutation_lock: Arc::new(Mutex::new(())),
        };

        let page = repository
            .list_active_details_page(&[], None, PageRequest::new(Some(1), Some(10)))
            .await
            .expect("hall page can load");

        assert_eq!(
            page.items
                .iter()
                .map(|plan| plan.id.as_str())
                .collect::<Vec<_>>(),
            ["G-HALL-NEWER", "G-HALL-OLDER"]
        );
    }

    /// 验证发起人即使没有参与人行，也能在“我的合买”仓储查询中看到自己的未成单计划。
    #[tokio::test]
    async fn group_buy_repository_unformed_user_page_includes_initiator_owned_plan() {
        let mut plan = seed_group_buy_plans()
            .into_iter()
            .next()
            .expect("seed plan exists");
        plan.id = "G-UNFORMED-INITIATOR".to_string();
        plan.initiator_user_id = "U20002".to_string();
        plan.order_id = None;
        plan.participants.clear();
        plan.filled_amount_minor = 10_000;

        let repository = GroupBuyRepository {
            inner: Arc::new(RwLock::new(GroupBuyStore {
                plans: BTreeMap::from([(plan.id.clone(), plan)]),
            })),
            persistence: None,
            mutation_lock: Arc::new(Mutex::new(())),
        };

        let page = repository
            .list_unformed_details_for_user_page("U20002", PageRequest::new(Some(1), Some(10)))
            .await
            .expect("initiator unformed user page can load");

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, "G-UNFORMED-INITIATOR");
    }

    /// 验证合买合买仓储skips未结算计划whenclearing。
    #[test]
    fn group_buy_store_skips_unsettled_plans_when_clearing() {
        let mut store = GroupBuyStore::seeded();
        let deleted_count = store
            .clear_records()
            .expect("unsettled group buy plan should be skipped");

        assert_eq!(deleted_count, 0);
        assert_eq!(store.list().len(), 1);
    }
    /// 验证合买合买仓储清理已完成记录和keeps未结算。
    #[test]
    fn group_buy_store_clears_finished_records_and_keeps_unsettled() {
        let mut store = GroupBuyStore::seeded();
        let base_plan = store
            .plans
            .values()
            .next()
            .cloned()
            .expect("seeded plan exists");
        let mut settled_plan = base_plan.clone();
        settled_plan.id = "G-SETTLED".to_string();
        settled_plan.status = GroupBuyPlanStatus::Settled;
        let mut cancelled_plan = base_plan;
        cancelled_plan.id = "G-CANCELLED".to_string();
        cancelled_plan.status = GroupBuyPlanStatus::Cancelled;
        store.plans.insert(settled_plan.id.clone(), settled_plan);
        store
            .plans
            .insert(cancelled_plan.id.clone(), cancelled_plan);

        assert_eq!(
            store
                .clear_records()
                .expect("finished group buy plans can be cleared"),
            2
        );
        assert_eq!(store.list().len(), 1);
        assert!(store.plans.values().all(|plan| !matches!(
            plan.status,
            GroupBuyPlanStatus::Cancelled | GroupBuyPlanStatus::Settled
        )));
    }
    /// 验证合买合买仓储清理已完成记录。
    #[test]
    fn group_buy_store_clears_finished_records() {
        let mut store = GroupBuyStore::seeded();
        for plan in store.plans.values_mut() {
            plan.status = GroupBuyPlanStatus::Settled;
        }

        assert_eq!(
            store
                .clear_records()
                .expect("finished group buy plans can be cleared"),
            1
        );
        assert!(store.list().is_empty());
    }

    /// 验证机器人批量清理候选只包含发起人和参与人都属于机器人的合买计划。
    #[test]
    fn group_buy_store_robot_cleanup_candidates_skip_real_participants() {
        let mut store = GroupBuyStore::seeded();
        let base_plan = store
            .plans
            .values()
            .next()
            .cloned()
            .expect("seeded plan exists");
        let mut robot_only_plan = base_plan.clone();
        robot_only_plan.id = "G-ROBOT-ONLY".to_string();
        robot_only_plan.participants = robot_only_plan
            .participants
            .into_iter()
            .filter(|participant| participant.user_id == "U90001")
            .collect();
        store
            .plans
            .insert(robot_only_plan.id.clone(), robot_only_plan);

        let robot_user_ids = ["U90001", "X90002"]
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<_>>();
        let candidates = store.robot_cleanup_candidates(&robot_user_ids);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "G-ROBOT-ONLY");
    }

    /// 验证机器人批量清理允许一个计划中混合多个系统机器人参与人。
    #[test]
    fn group_buy_store_robot_cleanup_candidates_include_mixed_robot_participants() {
        let mut store = GroupBuyStore::seeded();
        let base_plan = store
            .plans
            .values()
            .next()
            .cloned()
            .expect("seeded plan exists");
        let mut mixed_robot_plan = base_plan.clone();
        mixed_robot_plan.id = "G-MIXED-ROBOT-ONLY".to_string();
        mixed_robot_plan.initiator_user_id = "U90001".to_string();
        mixed_robot_plan.participants = vec![
            GroupBuyParticipant {
                id: "G-MIXED-ROBOT-ONLY-P001".to_string(),
                user_id: "U90001".to_string(),
                username: "发单机器人".to_string(),
                amount_minor: 10_000,
                share_count: 100,
                note: "机器人发起".to_string(),
                created_at: "2026-07-01 06:50:00".to_string(),
            },
            GroupBuyParticipant {
                id: "G-MIXED-ROBOT-ONLY-P002".to_string(),
                user_id: "X90002".to_string(),
                username: "补单机器人02".to_string(),
                amount_minor: 20_000,
                share_count: 200,
                note: "机器人补单".to_string(),
                created_at: "2026-07-01 06:51:00".to_string(),
            },
            GroupBuyParticipant {
                id: "G-MIXED-ROBOT-ONLY-P003".to_string(),
                user_id: "X90003".to_string(),
                username: "补单机器人03".to_string(),
                amount_minor: 20_000,
                share_count: 200,
                note: "机器人补单".to_string(),
                created_at: "2026-07-01 06:52:00".to_string(),
            },
        ];
        store
            .plans
            .insert(mixed_robot_plan.id.clone(), mixed_robot_plan);

        let robot_user_ids = ["U90001", "X90002", "X90003"]
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<_>>();
        let candidates = store.robot_cleanup_candidates(&robot_user_ids);

        assert!(candidates
            .iter()
            .any(|plan| plan.id == "G-MIXED-ROBOT-ONLY"));
        assert!(candidates.iter().all(|plan| {
            robot_user_ids.contains(plan.initiator_user_id.as_str())
                && plan
                    .participants
                    .iter()
                    .all(|participant| robot_user_ids.contains(participant.user_id.as_str()))
        }));
    }

    /// 验证机器人批量清理候选覆盖全部合买生命周期状态。
    #[test]
    fn group_buy_store_robot_cleanup_candidates_include_all_statuses() {
        let base_plan = GroupBuyStore::seeded()
            .plans
            .values()
            .next()
            .cloned()
            .expect("seeded plan exists");
        let robot_participants = base_plan
            .participants
            .iter()
            .filter(|participant| participant.user_id == "U90001")
            .cloned()
            .collect::<Vec<_>>();
        let mut store = GroupBuyStore {
            plans: BTreeMap::new(),
        };

        for (index, status) in [
            GroupBuyPlanStatus::Draft,
            GroupBuyPlanStatus::Open,
            GroupBuyPlanStatus::Filled,
            GroupBuyPlanStatus::Cancelled,
            GroupBuyPlanStatus::Settled,
        ]
        .into_iter()
        .enumerate()
        {
            let mut plan = base_plan.clone();
            plan.id = format!("G-ROBOT-STATUS-{index}");
            plan.status = status;
            plan.participants = robot_participants.clone();
            store.plans.insert(plan.id.clone(), plan);
        }

        let robot_user_ids = ["U90001"]
            .into_iter()
            .map(String::from)
            .collect::<BTreeSet<_>>();
        let candidates = store.robot_cleanup_candidates(&robot_user_ids);
        let plan_ids = candidates
            .iter()
            .map(|plan| plan.id.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(candidates.len(), 5);
        assert_eq!(store.delete_plans_by_ids(&plan_ids).len(), 5);
        assert!(store.plans.is_empty());
    }

    /// 验证后台删除机器人合买计划时可以拿到被删除的记录，便于页面即时移除和审计。
    #[test]
    fn group_buy_store_deletes_plan_and_returns_record() {
        let mut store = GroupBuyStore::seeded();
        let plan_id = store
            .plans
            .keys()
            .next()
            .cloned()
            .expect("seeded plan exists");

        let deleted = store
            .delete_plan(&plan_id)
            .expect("robot plan can be deleted");

        assert_eq!(deleted.id, plan_id);
        assert!(store.get(&plan_id).is_err());
    }
    /// 验证合买合买仓储拒绝停用彩种。
    #[tokio::test]
    async fn group_buy_repository_rejects_disabled_lottery() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let lotteries = seed_lotteries();
        let error = repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-002".to_string(),
                    lottery_id: "manual-test".to_string(),
                    issue: "20260602001".to_string(),
                    rule_code: "threeDirect".to_string(),
                    title: "关闭合买彩种".to_string(),
                    numbers: "1,2,3".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 100_000,
                    initiator_amount_minor: 10_000,
                    note: "关闭合买彩种".to_string(),
                },
                &lotteries,
                &access.users,
            )
            .await
            .expect_err("disabled group buy lottery must be rejected");

        assert!(error.to_string().contains("group buy is disabled"));
    }
    /// 验证合买合买仓储拒绝低发起人金额。
    #[tokio::test]
    async fn group_buy_repository_rejects_low_initiator_amount() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let lotteries = lotteries_with_group_buy_enabled("fc3d");
        let error = repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-003".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "20260602001".to_string(),
                    rule_code: "threeDirect".to_string(),
                    title: "低于发起人比例".to_string(),
                    numbers: "1,2,3".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 100_000,
                    initiator_amount_minor: 9_900,
                    note: "低于发起人比例".to_string(),
                },
                &lotteries,
                &access.users,
            )
            .await
            .expect_err("low initiator amount must be rejected");

        assert!(error
            .to_string()
            .contains("initiator amount must be at least"));
    }
    /// 验证合买合买仓储adds参与人和补满计划。
    #[tokio::test]
    async fn group_buy_repository_adds_participant_and_fills_plan() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let lotteries = lotteries_with_group_buy_enabled("fc3d");
        let plan = repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-004".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "20260602001".to_string(),
                    rule_code: "threeDirect".to_string(),
                    title: "可满单计划".to_string(),
                    numbers: "1,2,3".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 20_000,
                    initiator_amount_minor: 10_000,
                    note: "可满单计划".to_string(),
                },
                &lotteries,
                &access.users,
            )
            .await
            .expect("plan can be created");

        assert_eq!(plan.status, GroupBuyPlanStatus::Open);

        let filled = repository
            .add_participant(
                "G-TEST-004",
                AddGroupBuyParticipantRequest {
                    id: "G-TEST-004-P002".to_string(),
                    user_id: "U10001".to_string(),
                    amount_minor: 10_000,
                    note: "参与满单".to_string(),
                },
                &access.users,
            )
            .await
            .expect("participant can fill plan");

        assert_eq!(filled.filled_amount_minor, 20_000);
        assert_eq!(filled.status, GroupBuyPlanStatus::Filled);
        assert_eq!(filled.participants.len(), 2);
    }

    /// 验证控奖专用合买查询只返回发起人自购记录，不把跟单用户暴露给控奖列表。
    #[tokio::test]
    async fn control_group_buy_details_keep_only_initiator_participants() {
        let repository = GroupBuyRepository::memory_seeded();

        let plans = repository
            .list_control_details_for_issue("fc3d", "20260602001")
            .await
            .expect("control group buy details can load");

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].participants.len(), 1);
        assert_eq!(plans[0].participants[0].user_id, plans[0].initiator_user_id);
        assert_eq!(plans[0].participants[0].id, "G202606020001-P001");
    }

    /// 验证合买合买仓储allowsfinal尾差below参与人minimum。
    #[tokio::test]
    async fn group_buy_repository_allows_final_remainder_below_participant_minimum() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let lotteries = lotteries_with_group_buy_enabled("fc3d");
        repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-FINAL-REMAINDER".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "20260602001".to_string(),
                    rule_code: "threeDirect".to_string(),
                    title: "允许尾单全包".to_string(),
                    numbers: "1,2,3".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 1_500,
                    initiator_amount_minor: 1_000,
                    note: "剩余低于最低认购金额".to_string(),
                },
                &lotteries,
                &access.users,
            )
            .await
            .expect("plan with small final remainder can be created");

        let filled = repository
            .add_participant(
                "G-TEST-FINAL-REMAINDER",
                AddGroupBuyParticipantRequest {
                    id: "G-TEST-FINAL-REMAINDER-P002".to_string(),
                    user_id: "U10001".to_string(),
                    amount_minor: 500,
                    note: "全包剩余尾单".to_string(),
                },
                &access.users,
            )
            .await
            .expect("final remainder below participant minimum can be fully subscribed");

        assert_eq!(filled.filled_amount_minor, 1_500);
        assert_eq!(filled.status, GroupBuyPlanStatus::Filled);
        assert_eq!(filled.participants[1].share_count, 5);
    }
    /// 验证合买合买仓储拒绝参与人thatleavesunjoinable尾差。
    #[tokio::test]
    async fn group_buy_repository_rejects_participant_that_leaves_unjoinable_remainder() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let lotteries = lotteries_with_group_buy_enabled("fc3d");
        repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-UNJOINABLE-REMAINDER".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "20260602001".to_string(),
                    rule_code: "threeDirect".to_string(),
                    title: "拒绝留下尾单".to_string(),
                    numbers: "1,2,3".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 2_500,
                    initiator_amount_minor: 1_000,
                    note: "防止剩余不足最低认购".to_string(),
                },
                &lotteries,
                &access.users,
            )
            .await
            .expect("plan can be created");

        let error = repository
            .add_participant(
                "G-TEST-UNJOINABLE-REMAINDER",
                AddGroupBuyParticipantRequest {
                    id: "G-TEST-UNJOINABLE-REMAINDER-P002".to_string(),
                    user_id: "U10001".to_string(),
                    amount_minor: 1_000,
                    note: "留下小尾巴".to_string(),
                },
                &access.users,
            )
            .await
            .expect_err("participant cannot leave a remainder below minimum");

        assert!(error.to_string().contains("剩余金额低于最低认购金额"));
    }
    /// 验证合买合买仓储拒绝参与人超额认购。
    #[tokio::test]
    async fn group_buy_repository_rejects_participant_overfill() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let error = repository
            .add_participant(
                "G202606020001",
                AddGroupBuyParticipantRequest {
                    id: "G202606020001-P999".to_string(),
                    user_id: "U10001".to_string(),
                    amount_minor: 40_000,
                    note: "超额参与".to_string(),
                },
                &access.users,
            )
            .await
            .expect_err("participant amount over remaining must be rejected");

        assert!(error.to_string().contains("超过剩余可认购金额"));
    }

    /// 返回显式开启指定彩种合买的测试彩种列表。
    fn lotteries_with_group_buy_enabled(lottery_id: &str) -> Vec<LotteryKind> {
        let mut lotteries = seed_lotteries();
        if let Some(lottery) = lotteries
            .iter_mut()
            .find(|lottery| lottery.id == lottery_id)
        {
            lottery.group_buy.enabled = true;
        }
        lotteries
    }
}
