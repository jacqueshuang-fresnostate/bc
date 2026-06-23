//! 提现服务，管理用户提现申请并联动财务冻结

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, PgConnection, Row};

use crate::{
    domain::{
        finance::WithdrawalTurnoverSummary,
        permission::SystemSetting,
        user::{UserSummary, WithdrawalMethod},
        withdrawal::{CreateWithdrawalOrderRequest, WithdrawalOrderStatus, WithdrawalOrderSummary},
    },
    error::{ApiError, ApiResult},
    services::{
        business_database::BusinessDatabase,
        finance::{
            save_finance_store_incremental_in_transaction, FinanceRepository, LedgerEntryIdRemap,
        },
        pagination::{ListPage, PageRequest},
    },
};

use super::business_database::{enum_from_string, enum_to_string};

const WITHDRAWAL_TURNOVER_ENABLED_SETTING_KEY: &str = "withdrawal_turnover_enabled";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 提现流水校验策略，控制用户提现前是否必须完成充值等额有效投注。
pub struct WithdrawalTurnoverPolicy {
    /// 是否启用提现流水校验。
    pub enabled: bool,
}

/// 从系统设置解析提现流水校验策略，旧库缺失配置时默认关闭。
pub fn withdrawal_turnover_policy_from_system_settings(
    settings: &[SystemSetting],
) -> WithdrawalTurnoverPolicy {
    WithdrawalTurnoverPolicy {
        enabled: bool_setting(settings, WITHDRAWAL_TURNOVER_ENABLED_SETTING_KEY),
    }
}

impl WithdrawalTurnoverPolicy {
    /// 返回关闭状态的提现流水策略，供测试或无需校验的调用方显式使用。
    #[cfg(test)]
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// 根据用户累计充值和有效投注判断当前是否允许提现。
    fn ensure_withdrawable(&self, turnover: &WithdrawalTurnoverSummary) -> ApiResult<()> {
        if !self.enabled || turnover.remaining_effective_bet_minor <= 0 {
            return Ok(());
        }

        Err(ApiError::BadRequest(format!(
            "提现流水不足：累计充值 {}，需要有效投注 {}，当前有效投注 {}，还差 {}",
            format_minor_money(turnover.cumulative_recharge_minor),
            format_minor_money(turnover.required_effective_bet_minor),
            format_minor_money(turnover.completed_effective_bet_minor),
            format_minor_money(turnover.remaining_effective_bet_minor)
        )))
    }
}

/// 读取布尔系统设置，只有明确为 true 才视为开启。
fn bool_setting(settings: &[SystemSetting], key: &str) -> bool {
    settings
        .iter()
        .find(|setting| setting.key == key)
        .map(|setting| setting.value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or_default()
}

/// 把分金额格式化为面向用户的元金额文本。
fn format_minor_money(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let amount = i128::from(amount_minor).abs();
    format!("{sign}¥{}.{:02}", amount / 100, amount % 100)
}

#[derive(Clone)]
/// 提现申请仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct WithdrawalRepository {
    /// 提现模块内存快照锁，保存提现申请和运行序号。
    pub(crate) inner: Arc<RwLock<WithdrawalStore>>,
    /// 可选数据库持久化句柄；内存模式下为空。
    pub(crate) persistence: Option<BusinessDatabase>,
}

/// 提现申请仓储，负责该模块数据读取、业务变更和持久化协调。
impl WithdrawalRepository {
    /// 返回空的内存提现仓储，适配无数据库开发模式。
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(WithdrawalStore::default())),
            persistence: None,
        }
    }

    /// 从数据库加载提现订单，并创建持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_withdrawal_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回指定用户的提现申请列表。
    pub async fn list_for_user(&self, user_id: &str) -> ApiResult<Vec<WithdrawalOrderSummary>> {
        let user_id = required_trimmed(user_id, "user id")?;
        Ok(self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?
            .list_for_user(&user_id))
    }

    /// 分页返回指定用户提现申请，供手机端避免全量拉取历史申请。
    pub async fn list_for_user_page(
        &self,
        user_id: &str,
        page: PageRequest,
    ) -> ApiResult<ListPage<WithdrawalOrderSummary>> {
        let user_id = required_trimmed(user_id, "user id")?;
        if let Some(persistence) = &self.persistence {
            return query_withdrawal_order_page(persistence, Some(&user_id), page).await;
        }

        let mut orders = self.list_for_user(&user_id).await?;
        orders.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(ListPage::from_all(orders, page))
    }

    /// 返回全部提现申请列表，供后台财务管理审核。
    pub async fn list(&self) -> ApiResult<Vec<WithdrawalOrderSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 分页返回全部提现申请；数据库模式下直接按时间倒序分页。
    pub async fn list_page(
        &self,
        page: PageRequest,
    ) -> ApiResult<ListPage<WithdrawalOrderSummary>> {
        if let Some(persistence) = &self.persistence {
            return query_withdrawal_order_page(persistence, None, page).await;
        }

        let mut orders = self.list().await?;
        orders.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(ListPage::from_all(orders, page))
    }

    /// 返回已通过提现申请，供代理返利统计避免读取待审、驳回和取消记录。
    pub async fn approved_orders(&self) -> ApiResult<Vec<WithdrawalOrderSummary>> {
        if let Some(persistence) = &self.persistence {
            return query_withdrawal_orders_by_status(persistence, WithdrawalOrderStatus::Approved)
                .await;
        }

        Ok(self
            .list()
            .await?
            .into_iter()
            .filter(|order| order.status == WithdrawalOrderStatus::Approved)
            .collect())
    }

    /// 一键清除提现历史；存在待审核申请时拒绝清理，避免冻结余额失去对应申请。
    pub async fn clear_records(&self) -> ApiResult<usize> {
        let (deleted_count, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?;
            let deleted_count = store.clear_records()?;
            (deleted_count, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(deleted_count)
    }

    /// 创建提现申请，并在创建成功前冻结对应用户可用余额。
    pub async fn create_order(
        &self,
        user: &UserSummary,
        method: &WithdrawalMethod,
        request: CreateWithdrawalOrderRequest,
        finance: &FinanceRepository,
        turnover_policy: &WithdrawalTurnoverPolicy,
    ) -> ApiResult<WithdrawalOrderSummary> {
        let previous_withdrawal_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?
            .clone();
        let mut withdrawal_store = previous_withdrawal_store.clone();
        let previous_finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = previous_finance_store.clone();

        let order = withdrawal_store.draft_order(user, method, request)?;
        let turnover = finance.withdrawal_turnover_for_user(&order.user_id).await?;
        turnover_policy.ensure_withdrawable(&turnover)?;
        finance_store.freeze_withdrawal(&order.user_id, order.amount_minor, &order.id)?;
        let result = withdrawal_store.insert_order(order)?;

        persist_withdrawal_finance_stores(
            self,
            finance,
            &previous_withdrawal_store,
            &withdrawal_store,
            &previous_finance_store,
            &mut finance_store,
        )
        .await?;
        self.replace_store(withdrawal_store)?;
        finance.replace_store(finance_store)?;
        Ok(result)
    }

    /// 审核通过提现申请，扣减冻结余额并把申请标记为已通过。
    pub async fn approve_order(
        &self,
        id: &str,
        finance: &FinanceRepository,
    ) -> ApiResult<WithdrawalOrderSummary> {
        let previous_withdrawal_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?
            .clone();
        let mut withdrawal_store = previous_withdrawal_store.clone();
        let previous_finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = previous_finance_store.clone();

        let order = withdrawal_store.reviewable_order(id, WithdrawalOrderStatus::Approved)?;
        finance_store.approve_withdrawal(&order.user_id, order.amount_minor, &order.id)?;
        let result = withdrawal_store.mark_reviewed(id, WithdrawalOrderStatus::Approved)?;

        persist_withdrawal_finance_stores(
            self,
            finance,
            &previous_withdrawal_store,
            &withdrawal_store,
            &previous_finance_store,
            &mut finance_store,
        )
        .await?;
        self.replace_store(withdrawal_store)?;
        finance.replace_store(finance_store)?;
        Ok(result)
    }

    /// 审核驳回提现申请，解冻余额并把申请标记为已驳回。
    pub async fn reject_order(
        &self,
        id: &str,
        finance: &FinanceRepository,
    ) -> ApiResult<WithdrawalOrderSummary> {
        let previous_withdrawal_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?
            .clone();
        let mut withdrawal_store = previous_withdrawal_store.clone();
        let previous_finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = previous_finance_store.clone();

        let order = withdrawal_store.reviewable_order(id, WithdrawalOrderStatus::Rejected)?;
        finance_store.reject_withdrawal(&order.user_id, order.amount_minor, &order.id)?;
        let result = withdrawal_store.mark_reviewed(id, WithdrawalOrderStatus::Rejected)?;

        persist_withdrawal_finance_stores(
            self,
            finance,
            &previous_withdrawal_store,
            &withdrawal_store,
            &previous_finance_store,
            &mut finance_store,
        )
        .await?;
        self.replace_store(withdrawal_store)?;
        finance.replace_store(finance_store)?;
        Ok(result)
    }

    /// 用事务提交后的快照替换当前提现申请内存状态。
    pub(crate) fn replace_store(&self, store: WithdrawalStore) -> ApiResult<()> {
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))? = store;
        Ok(())
    }

    /// 从数据库重新加载提现申请快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_withdrawal_store(persistence).await?;
        self.replace_store(store)?;
        Ok(true)
    }
    /// 把当前仓储快照同步保存到持久化存储。
    async fn persist(&self, store: &WithdrawalStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            let mut tx = persistence
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::Internal("提现事务开启失败".to_string()))?;
            save_withdrawal_store_in_transaction(&mut *tx, store).await?;
            tx.commit()
                .await
                .map_err(|_| ApiError::Internal("提现事务提交失败".to_string()))?;
        }
        Ok(())
    }
}

/// 在同一个数据库事务中保存提现和资金快照，确保冻结、打款和解冻不会只落一边。
async fn persist_withdrawal_finance_stores(
    withdrawals: &WithdrawalRepository,
    finance: &FinanceRepository,
    previous_withdrawal_store: &WithdrawalStore,
    withdrawal_store: &WithdrawalStore,
    previous_finance_store: &super::finance::FinanceStore,
    finance_store: &mut super::finance::FinanceStore,
) -> ApiResult<LedgerEntryIdRemap> {
    match (&withdrawals.persistence, &finance.persistence) {
        (Some(database), Some(_)) => {
            let mut tx = database
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::Internal("提现资金事务开启失败".to_string()))?;
            save_withdrawal_store_incremental_in_transaction(
                &mut *tx,
                previous_withdrawal_store,
                withdrawal_store,
            )
            .await?;
            let id_remap = save_finance_store_incremental_in_transaction(
                &mut *tx,
                previous_finance_store,
                finance_store,
            )
            .await?;
            tx.commit()
                .await
                .map_err(|_| ApiError::Internal("提现资金事务提交失败".to_string()))?;
            Ok(id_remap)
        }
        (None, None) => Ok(LedgerEntryIdRemap::default()),
        _ => Err(ApiError::Internal("提现和资金持久化配置不一致".to_string())),
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
/// 提现申请运行时数据快照，用于内存模式和数据库持久化前的业务校验。
pub(crate) struct WithdrawalStore {
    orders: BTreeMap<String, WithdrawalOrderSummary>,
    next_sequence: u64,
}

/// 提现申请运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl WithdrawalStore {
    /// 返回全部提现申请列表，最新申请排在最前面。
    fn list(&self) -> Vec<WithdrawalOrderSummary> {
        self.orders.values().cloned().rev().collect()
    }

    /// 返回某个用户自己的提现申请列表。
    fn list_for_user(&self, user_id: &str) -> Vec<WithdrawalOrderSummary> {
        self.orders
            .values()
            .filter(|order| order.user_id == user_id)
            .cloned()
            .rev()
            .collect()
    }

    /// 清除已结束的提现申请记录；待审核申请会保留，要求管理员先审核或驳回。
    fn clear_records(&mut self) -> ApiResult<usize> {
        let pending_count = self
            .orders
            .values()
            .filter(|order| order.status == WithdrawalOrderStatus::Pending)
            .count();
        if pending_count > 0 {
            return Err(ApiError::BadRequest(format!(
                "存在 {pending_count} 笔待审核提现申请，请先审核或驳回后再清除记录"
            )));
        }

        let deleted_count = self.orders.len();
        self.orders.clear();
        Ok(deleted_count)
    }

    /// 校验提现参数并生成尚未入库的提现申请。
    pub(crate) fn draft_order(
        &mut self,
        user: &UserSummary,
        method: &WithdrawalMethod,
        request: CreateWithdrawalOrderRequest,
    ) -> ApiResult<WithdrawalOrderSummary> {
        let method_id = required_trimmed(request.method_id, "withdrawal method id")?;
        if method_id != method.id || method.user_id != user.id {
            return Err(ApiError::BadRequest("提现方式不属于当前用户".to_string()));
        }
        if request.amount_minor <= 0 {
            return Err(ApiError::BadRequest("提现金额必须大于 0".to_string()));
        }

        self.next_sequence += 1;
        let id = format!("W{:012}", self.next_sequence);
        Ok(WithdrawalOrderSummary {
            id,
            user_id: user.id.clone(),
            username: user.username.clone(),
            method_id: method.id.clone(),
            method_type: method.method_type.clone(),
            account_holder: method.account_holder.clone(),
            account_number: method.account_number.clone(),
            bank_name: method.bank_name.clone(),
            amount_minor: request.amount_minor,
            status: WithdrawalOrderStatus::Pending,
            created_at: current_time_label(),
            reviewed_at: None,
        })
    }

    /// 保存提现申请到内存集合，重复订单 ID 直接拒绝。
    pub(crate) fn insert_order(
        &mut self,
        order: WithdrawalOrderSummary,
    ) -> ApiResult<WithdrawalOrderSummary> {
        if self.orders.contains_key(&order.id) {
            return Err(ApiError::Conflict(format!(
                "withdrawal order `{}` already exists",
                order.id
            )));
        }
        self.orders.insert(order.id.clone(), order.clone());
        Ok(order)
    }

    /// 返回可审核的提现申请；重复点击同一审核结果视为幂等。
    pub(crate) fn reviewable_order(
        &self,
        id: &str,
        target_status: WithdrawalOrderStatus,
    ) -> ApiResult<WithdrawalOrderSummary> {
        let id = required_trimmed(id, "withdrawal order id")?;
        let order = self
            .orders
            .get(&id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("withdrawal order `{id}` not found")))?;
        match (&order.status, &target_status) {
            (WithdrawalOrderStatus::Pending, _)
            | (WithdrawalOrderStatus::Approved, WithdrawalOrderStatus::Approved)
            | (WithdrawalOrderStatus::Rejected, WithdrawalOrderStatus::Rejected) => Ok(order),
            (WithdrawalOrderStatus::Approved, WithdrawalOrderStatus::Rejected) => {
                Err(ApiError::BadRequest("已通过的提现申请不能驳回".to_string()))
            }
            (WithdrawalOrderStatus::Rejected, WithdrawalOrderStatus::Approved) => {
                Err(ApiError::BadRequest("已驳回的提现申请不能通过".to_string()))
            }
            (WithdrawalOrderStatus::Cancelled, _) => {
                Err(ApiError::BadRequest("已取消的提现申请不能审核".to_string()))
            }
            (_, _) => Err(ApiError::BadRequest("提现审核状态不支持".to_string())),
        }
    }

    /// 把提现申请标记为目标审核状态，并记录审核时间。
    pub(crate) fn mark_reviewed(
        &mut self,
        id: &str,
        target_status: WithdrawalOrderStatus,
    ) -> ApiResult<WithdrawalOrderSummary> {
        let id = required_trimmed(id, "withdrawal order id")?;
        let order = self
            .orders
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("withdrawal order `{id}` not found")))?;
        if order.status != target_status {
            if order.status != WithdrawalOrderStatus::Pending {
                return Err(ApiError::BadRequest("提现申请当前状态不能审核".to_string()));
            }
            order.status = target_status;
        }
        if order.reviewed_at.is_none() {
            order.reviewed_at = Some(current_time_label());
        }
        Ok(order.clone())
    }
}

/// 从数据库加载提现申请运行时快照，空库时按模块规则初始化。
async fn load_withdrawal_store(database: &BusinessDatabase) -> ApiResult<WithdrawalStore> {
    let pool = database.pool();
    let mut orders = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, user_id, username, method_id, method_type, account_holder,
                account_number, bank_name, amount_minor, status, created_at, reviewed_at
         FROM withdrawal_orders
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?
    {
        let order = withdrawal_order_from_row(row)?;
        orders.insert(order.id.clone(), order);
    }

    let next_sequence = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM withdrawal_runtime WHERE key = 'next_sequence'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("提现运行数据读取失败".to_string()))?
    .unwrap_or_default();

    Ok(WithdrawalStore {
        orders,
        next_sequence: u64::try_from(next_sequence).unwrap_or_default(),
    })
}

/// 从数据库行恢复提现申请结构，供启动加载和分页查询复用。
fn withdrawal_order_from_row(row: PgRow) -> ApiResult<WithdrawalOrderSummary> {
    Ok(WithdrawalOrderSummary {
        id: row
            .try_get("id")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        user_id: row
            .try_get("user_id")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        username: row
            .try_get("username")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        method_id: row
            .try_get("method_id")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        method_type: enum_from_string(
            row.try_get("method_type")
                .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        )?,
        account_holder: row
            .try_get("account_holder")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        account_number: row
            .try_get("account_number")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        bank_name: row
            .try_get("bank_name")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        amount_minor: row
            .try_get("amount_minor")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        status: enum_from_string(
            row.try_get("status")
                .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        )?,
        created_at: row
            .try_get("created_at")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
        reviewed_at: row
            .try_get("reviewed_at")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?,
    })
}

/// 数据库模式下分页读取提现申请，支持后台审核列表和用户端本人列表。
async fn query_withdrawal_order_page(
    database: &BusinessDatabase,
    user_id: Option<&str>,
    page: PageRequest,
) -> ApiResult<ListPage<WithdrawalOrderSummary>> {
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM withdrawal_orders
         WHERE ($1::text IS NULL OR user_id = $1)",
    )
    .bind(user_id)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("提现申请分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("提现申请分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT id, user_id, username, method_id, method_type, account_holder,
                account_number, bank_name, amount_minor, status, created_at, reviewed_at
         FROM withdrawal_orders
         WHERE ($1::text IS NULL OR user_id = $1)
         ORDER BY created_at DESC, id DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("提现申请分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(withdrawal_order_from_row)
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下按状态读取提现申请，供聚合统计只读取必要业务状态。
async fn query_withdrawal_orders_by_status(
    database: &BusinessDatabase,
    status: WithdrawalOrderStatus,
) -> ApiResult<Vec<WithdrawalOrderSummary>> {
    let status = enum_to_string(&status)?;
    let rows = sqlx::query(
        "SELECT id, user_id, username, method_id, method_type, account_holder,
                account_number, bank_name, amount_minor, status, created_at, reviewed_at
         FROM withdrawal_orders
         WHERE status = $1
         ORDER BY created_at DESC, id DESC",
    )
    .bind(status)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("提现申请状态数据读取失败".to_string()))?;

    rows.into_iter().map(withdrawal_order_from_row).collect()
}

/// 在外层事务中保存提现申请运行时快照，供跨仓储事务复用。
pub(crate) async fn save_withdrawal_store_in_transaction(
    connection: &mut PgConnection,
    store: &WithdrawalStore,
) -> ApiResult<()> {
    for table in ["withdrawal_orders", "withdrawal_runtime"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("提现数据清理失败".to_string()))?;
    }

    for order in store.orders.values() {
        sqlx::query(
            "INSERT INTO withdrawal_orders
             (id, user_id, username, method_id, method_type, account_holder,
              account_number, bank_name, amount_minor, status, created_at, reviewed_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&order.id)
        .bind(&order.user_id)
        .bind(&order.username)
        .bind(&order.method_id)
        .bind(enum_to_string(&order.method_type)?)
        .bind(&order.account_holder)
        .bind(&order.account_number)
        .bind(&order.bank_name)
        .bind(order.amount_minor)
        .bind(enum_to_string(&order.status)?)
        .bind(&order.created_at)
        .bind(&order.reviewed_at)
        .execute(&mut *connection)
        .await
        .map_err(|_| ApiError::Internal("提现申请数据保存失败".to_string()))?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("提现序号过大".to_string()))?;
    sqlx::query("INSERT INTO withdrawal_runtime (key, value) VALUES ('next_sequence', $1)")
        .bind(next_sequence)
        .execute(&mut *connection)
        .await
        .map_err(|_| ApiError::Internal("提现运行数据保存失败".to_string()))?;

    Ok(())
}

/// 在外层事务中按前后快照差异保存提现申请，避免审核时重写全部提现历史。
pub(crate) async fn save_withdrawal_store_incremental_in_transaction(
    connection: &mut PgConnection,
    previous: &WithdrawalStore,
    store: &WithdrawalStore,
) -> ApiResult<()> {
    for order_id in previous
        .orders
        .keys()
        .filter(|order_id| !store.orders.contains_key(*order_id))
    {
        sqlx::query("DELETE FROM withdrawal_orders WHERE id = $1")
            .bind(order_id)
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("提现申请数据删除失败".to_string()))?;
    }

    for (order_id, order) in &store.orders {
        if previous.orders.get(order_id) == Some(order) {
            continue;
        }
        upsert_withdrawal_order_in_transaction(connection, order).await?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("提现序号过大".to_string()))?;
    sqlx::query(
        "INSERT INTO withdrawal_runtime (key, value) VALUES ('next_sequence', $1)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
    )
    .bind(next_sequence)
    .execute(&mut *connection)
    .await
    .map_err(|_| ApiError::Internal("提现运行数据保存失败".to_string()))?;

    Ok(())
}

/// 在事务中插入或更新单个提现申请。
async fn upsert_withdrawal_order_in_transaction(
    connection: &mut PgConnection,
    order: &WithdrawalOrderSummary,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO withdrawal_orders
         (id, user_id, username, method_id, method_type, account_holder,
          account_number, bank_name, amount_minor, status, created_at, reviewed_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         ON CONFLICT (id) DO UPDATE SET
            user_id = EXCLUDED.user_id,
            username = EXCLUDED.username,
            method_id = EXCLUDED.method_id,
            method_type = EXCLUDED.method_type,
            account_holder = EXCLUDED.account_holder,
            account_number = EXCLUDED.account_number,
            bank_name = EXCLUDED.bank_name,
            amount_minor = EXCLUDED.amount_minor,
            status = EXCLUDED.status,
            created_at = EXCLUDED.created_at,
            reviewed_at = EXCLUDED.reviewed_at",
    )
    .bind(&order.id)
    .bind(&order.user_id)
    .bind(&order.username)
    .bind(&order.method_id)
    .bind(enum_to_string(&order.method_type)?)
    .bind(&order.account_holder)
    .bind(&order.account_number)
    .bind(&order.bank_name)
    .bind(order.amount_minor)
    .bind(enum_to_string(&order.status)?)
    .bind(&order.created_at)
    .bind(&order.reviewed_at)
    .execute(&mut *connection)
    .await
    .map_err(|_| ApiError::Internal("提现申请数据保存失败".to_string()))?;
    Ok(())
}
/// 清洗必填字符串，空值时返回接口错误。
fn required_trimmed(value: impl Into<String>, label: &str) -> ApiResult<String> {
    let value = value.into().trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}
/// 生成当前本地时间字符串。
fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            finance::WithdrawalTurnoverSummary,
            permission::SystemSetting,
            user::{UserKind, UserStatus, UserSummary, WithdrawalMethod, WithdrawalMethodType},
            withdrawal::{CreateWithdrawalOrderRequest, WithdrawalOrderStatus},
        },
        services::withdrawal::{
            withdrawal_turnover_policy_from_system_settings, WithdrawalStore,
            WithdrawalTurnoverPolicy,
        },
    };

    #[test]
    /// 提现仓储会生成待审核申请，并把提现方式信息做快照保存。
    fn withdrawal_store_drafts_pending_order() {
        let mut store = WithdrawalStore::default();
        let user = sample_user();
        let method = sample_method(&user.id);

        let order = store
            .draft_order(
                &user,
                &method,
                CreateWithdrawalOrderRequest {
                    method_id: method.id.clone(),
                    amount_minor: 2_000,
                },
            )
            .expect("withdrawal order can be drafted");
        let inserted = store
            .insert_order(order)
            .expect("withdrawal order can be inserted");

        assert_eq!(inserted.id, "W000000000001");
        assert_eq!(inserted.status, WithdrawalOrderStatus::Pending);
        assert_eq!(inserted.method_type, WithdrawalMethodType::BankCard);
        assert_eq!(inserted.account_holder, "张三");
        assert_eq!(store.list_for_user(&user.id).len(), 1);
    }

    #[test]
    /// 提现方式必须属于当前用户，不能用其他人的收款方式提交申请。
    fn withdrawal_store_rejects_other_user_method() {
        let mut store = WithdrawalStore::default();
        let user = sample_user();
        let method = sample_method("U-OTHER");

        let error = store
            .draft_order(
                &user,
                &method,
                CreateWithdrawalOrderRequest {
                    method_id: method.id.clone(),
                    amount_minor: 2_000,
                },
            )
            .expect_err("foreign withdrawal method must be rejected");

        assert!(error.to_string().contains("提现方式不属于当前用户"));
    }

    #[test]
    /// 后台审核提现申请会更新状态和审核时间，并拒绝反向审核。
    fn withdrawal_store_marks_reviewed_status() {
        let mut store = WithdrawalStore::default();
        let user = sample_user();
        let method = sample_method(&user.id);
        let order = store
            .draft_order(
                &user,
                &method,
                CreateWithdrawalOrderRequest {
                    method_id: method.id.clone(),
                    amount_minor: 2_000,
                },
            )
            .expect("withdrawal order can be drafted");
        let inserted = store
            .insert_order(order)
            .expect("withdrawal order can be inserted");

        let reviewable = store
            .reviewable_order(&inserted.id, WithdrawalOrderStatus::Approved)
            .expect("pending order can be approved");
        let approved = store
            .mark_reviewed(&reviewable.id, WithdrawalOrderStatus::Approved)
            .expect("order can be marked approved");

        assert_eq!(approved.status, WithdrawalOrderStatus::Approved);
        assert!(approved.reviewed_at.is_some());
        assert!(store
            .reviewable_order(&approved.id, WithdrawalOrderStatus::Rejected)
            .expect_err("approved order cannot be rejected")
            .to_string()
            .contains("不能驳回"));
    }

    #[test]
    /// 提现流水策略从系统设置读取，开启后会拒绝有效投注不足的用户提现。
    fn withdrawal_turnover_policy_rejects_missing_effective_bet() {
        let settings = vec![SystemSetting {
            key: "withdrawal_turnover_enabled".to_string(),
            value: "true".to_string(),
            description: "是否开启提现前充值等额有效投注要求".to_string(),
        }];
        let policy = withdrawal_turnover_policy_from_system_settings(&settings);
        let turnover = WithdrawalTurnoverSummary {
            user_id: "U10001".to_string(),
            cumulative_recharge_minor: 100_000,
            required_effective_bet_minor: 100_000,
            completed_effective_bet_minor: 25_000,
            remaining_effective_bet_minor: 75_000,
        };

        let error = policy
            .ensure_withdrawable(&turnover)
            .expect_err("insufficient turnover must be rejected");

        assert!(error.to_string().contains("提现流水不足"));
        assert!(error.to_string().contains("¥750.00"));
    }

    #[test]
    /// 提现流水策略关闭时不会拦截提现申请。
    fn disabled_withdrawal_turnover_policy_allows_any_turnover() {
        let policy = WithdrawalTurnoverPolicy::disabled();
        let turnover = WithdrawalTurnoverSummary {
            user_id: "U10001".to_string(),
            cumulative_recharge_minor: 100_000,
            required_effective_bet_minor: 100_000,
            completed_effective_bet_minor: 0,
            remaining_effective_bet_minor: 100_000,
        };

        policy
            .ensure_withdrawable(&turnover)
            .expect("disabled policy should not block withdrawal");
    }

    #[test]
    /// 清理提现记录会拒绝待审核申请，已审核结束后才允许清理。
    fn withdrawal_store_clear_records_rejects_pending_orders() {
        let mut store = WithdrawalStore::default();
        let user = sample_user();
        let method = sample_method(&user.id);
        let order = store
            .draft_order(
                &user,
                &method,
                CreateWithdrawalOrderRequest {
                    method_id: method.id.clone(),
                    amount_minor: 2_000,
                },
            )
            .expect("withdrawal order can be drafted");
        let inserted = store
            .insert_order(order)
            .expect("withdrawal order can be inserted");

        assert!(store
            .clear_records()
            .expect_err("pending order cannot be cleared")
            .to_string()
            .contains("待审核提现申请"));

        store
            .mark_reviewed(&inserted.id, WithdrawalOrderStatus::Rejected)
            .expect("order can be rejected");
        assert_eq!(
            store.clear_records().expect("finished records can clear"),
            1
        );
        assert!(store.list().is_empty());
    }
    /// 构造提现测试用户。
    fn sample_user() -> UserSummary {
        UserSummary {
            id: "U10001".to_string(),
            username: "demo".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id: None,
            invite_code: "ABCD1234".to_string(),
            registration_location: crate::domain::user::UserRegistrationLocation::default(),
            created_at: "2026-06-05 10:00:00".to_string(),
        }
    }
    /// 构造提现测试方式。
    fn sample_method(user_id: &str) -> WithdrawalMethod {
        WithdrawalMethod {
            id: "WM0001".to_string(),
            user_id: user_id.to_string(),
            method_type: WithdrawalMethodType::BankCard,
            account_holder: "张三".to_string(),
            account_number: "6222000000000000".to_string(),
            bank_name: Some("招商银行".to_string()),
            is_default: true,
            created_at: "2026-06-04 22:00:00".to_string(),
            updated_at: "2026-06-04 22:00:00".to_string(),
        }
    }
}
