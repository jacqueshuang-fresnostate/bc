//! 合买领域模型，定义合买计划、参与人和份额约束

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Row};

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

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

#[derive(Clone)]
/// 合买计划和参与记录仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct GroupBuyRepository {
    pub(crate) inner: Arc<RwLock<GroupBuyStore>>,
    pub(crate) persistence: Option<BusinessDatabase>,
}

/// 合买计划和参与记录仓储，负责该模块数据读取、业务变更和持久化协调。
impl GroupBuyRepository {
    /// 返回带内置种子数据的内存仓储实例。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(GroupBuyStore::seeded())),
            persistence: None,
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_group_buy_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回完整列表。
    pub async fn list(&self) -> ApiResult<Vec<GroupBuyPlanSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 返回完整计划列表，供用户端大厅按参与记录计算我的合买。
    pub async fn list_details(&self) -> ApiResult<Vec<GroupBuyPlan>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| store.list_details())
    }

    /// 按 ID 查询单条记录。
    pub async fn get(&self, id: &str) -> ApiResult<GroupBuyPlan> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .get(id)
    }

    /// 按真实投注订单 ID 批量查找合买计划。
    pub async fn plans_for_order_ids(&self, order_ids: &[String]) -> ApiResult<Vec<GroupBuyPlan>> {
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
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            let result = store.create(request, lotteries, users)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 满单后把合买计划关联到真实投注订单。
    pub async fn attach_order(&self, id: &str, order_id: &str) -> ApiResult<GroupBuyPlan> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            let result = store.attach_order(id, order_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 根据结算订单 ID 把对应合买计划标记为已结算。
    pub async fn mark_settled_by_order_ids(
        &self,
        order_ids: &[String],
    ) -> ApiResult<Vec<GroupBuyPlan>> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            let result = store.mark_settled_by_order_ids(order_ids);
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 封盘时取消未满单的合买计划，返回需要退款的计划。
    pub async fn cancel_unfilled_for_issue(
        &self,
        lottery_id: &str,
        issue: &str,
    ) -> ApiResult<Vec<GroupBuyPlan>> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            let result = store.cancel_unfilled_for_issue(lottery_id, issue);
            (result, store.clone())
        };
        if !result.is_empty() {
            self.persist(&snapshot).await?;
        }
        Ok(result)
    }

    /// 更新现有记录并持久化变更。
    pub async fn update(
        &self,
        id: &str,
        request: UpdateGroupBuyPlanRequest,
    ) -> ApiResult<GroupBuyPlan> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            let result = store.update(id, request)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 为团购方案添加参与者。
    pub async fn add_participant(
        &self,
        id: &str,
        request: AddGroupBuyParticipantRequest,
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            let result = store.add_participant(id, request, users)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 移除刚创建但尚未成功扣款的合买计划，用于业务回滚。
    pub async fn remove_unfunded_plan(&self, id: &str) -> ApiResult<()> {
        let snapshot = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            store.remove_plan(id)?;
            store.clone()
        };
        self.persist(&snapshot).await
    }

    /// 移除刚追加但尚未成功扣款的参与记录，用于业务回滚。
    pub async fn remove_unfunded_participant(
        &self,
        id: &str,
        participant_id: &str,
    ) -> ApiResult<GroupBuyPlan> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?;
            let result = store.remove_participant(id, participant_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    async fn persist(&self, store: &GroupBuyStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_group_buy_store(persistence, store).await?;
        }

        Ok(())
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
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?;
        let share_count: i32 = row
            .try_get("share_count")
            .map_err(|_| ApiError::Internal("合买计划数据读取失败".to_string()))?;
        plans.insert(
            id.clone(),
            GroupBuyPlan {
                id,
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
            },
        );
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
        let plan_id: String = row
            .try_get("plan_id")
            .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?;
        let share_count: i32 = row
            .try_get("share_count")
            .map_err(|_| ApiError::Internal("合买参与人数据读取失败".to_string()))?;
        if let Some(plan) = plans.get_mut(&plan_id) {
            plan.participants.push(GroupBuyParticipant {
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
            });
        }
    }

    if plans.is_empty() {
        let seeded = GroupBuyStore::seeded();
        save_group_buy_store(database, &seeded).await?;
        return Ok(seeded);
    }

    Ok(GroupBuyStore { plans })
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
    for table in ["group_buy_participants", "group_buy_plans"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("合买数据清理失败".to_string()))?;
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
        .map_err(|_| ApiError::Internal("合买计划数据保存失败".to_string()))?;

        for participant in &plan.participants {
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
            .bind(
                i32::try_from(participant.share_count)
                    .map_err(|_| ApiError::Internal("合买参与人份数过大".to_string()))?,
            )
            .bind(&participant.note)
            .bind(&participant.created_at)
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("合买参与人数据保存失败".to_string()))?;
        }
    }

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

    /// 返回完整数据列表。
    fn list(&self) -> Vec<GroupBuyPlanSummary> {
        self.plans.values().map(GroupBuyPlan::summary).collect()
    }

    /// 返回完整计划列表。
    fn list_details(&self) -> Vec<GroupBuyPlan> {
        self.plans.values().cloned().collect()
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
        let lottery_id = lottery_id.trim();
        let issue = issue.trim();
        let mut cancelled = Vec::new();
        for plan in self.plans.values_mut() {
            if plan.lottery_id == lottery_id
                && plan.issue == issue
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

    /// 处理 add_participant 的具体内部流程。
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

        if request.amount_minor < participant_min_amount(plan) {
            return Err(ApiError::BadRequest(format!(
                "participant amount must be at least {}",
                participant_min_amount(plan)
            )));
        }

        let next_filled = plan
            .filled_amount_minor
            .checked_add(request.amount_minor)
            .ok_or_else(|| ApiError::Internal("group buy filled amount overflow".to_string()))?;
        if next_filled > plan.total_amount_minor {
            return Err(ApiError::BadRequest(
                "participant amount exceeds remaining group buy amount".to_string(),
            ));
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

/// 处理 participant_min_amount 的具体内部流程。
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

/// 校验输入参数并返回校验结果。
fn validate_positive_amount(amount: i64, label: &str) -> ApiResult<()> {
    if amount <= 0 {
        return Err(ApiError::BadRequest(format!(
            "{label} must be greater than zero"
        )));
    }

    Ok(())
}

/// 校验输入参数并返回校验结果。
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

/// 处理 minimum_amount_by_percent 的具体内部流程。
fn minimum_amount_by_percent(total_amount_minor: i64, percent: u8) -> ApiResult<i64> {
    let raw = total_amount_minor
        .checked_mul(i64::from(percent))
        .ok_or_else(|| ApiError::Internal("group buy minimum amount overflow".to_string()))?;

    Ok((raw + 99) / 100)
}

/// 处理 share_count 的具体内部流程。
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

/// 返回内置种子或测试数据。
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

        assert!(error.to_string().contains("exceeds remaining"));
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
