//! 提现服务，管理用户提现申请并联动财务冻结

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Row};

use crate::{
    domain::{
        user::{UserSummary, WithdrawalMethod},
        withdrawal::{CreateWithdrawalOrderRequest, WithdrawalOrderStatus, WithdrawalOrderSummary},
    },
    error::{ApiError, ApiResult},
    services::{
        business_database::BusinessDatabase,
        finance::{save_finance_store_in_transaction, FinanceRepository},
    },
};

use super::business_database::{enum_from_string, enum_to_string};

#[derive(Clone)]
/// 提现申请仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct WithdrawalRepository {
    pub(crate) inner: Arc<RwLock<WithdrawalStore>>,
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

    /// 返回全部提现申请列表，供后台财务管理审核。
    pub async fn list(&self) -> ApiResult<Vec<WithdrawalOrderSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 创建提现申请，并在创建成功前冻结对应用户可用余额。
    pub async fn create_order(
        &self,
        user: &UserSummary,
        method: &WithdrawalMethod,
        request: CreateWithdrawalOrderRequest,
        finance: &FinanceRepository,
    ) -> ApiResult<WithdrawalOrderSummary> {
        let mut withdrawal_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();

        let order = withdrawal_store.draft_order(user, method, request)?;
        finance_store.freeze_withdrawal(&order.user_id, order.amount_minor, &order.id)?;
        let result = withdrawal_store.insert_order(order)?;

        persist_withdrawal_finance_stores(self, finance, &withdrawal_store, &finance_store).await?;
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
        let mut withdrawal_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();

        let order = withdrawal_store.reviewable_order(id, WithdrawalOrderStatus::Approved)?;
        finance_store.approve_withdrawal(&order.user_id, order.amount_minor, &order.id)?;
        let result = withdrawal_store.mark_reviewed(id, WithdrawalOrderStatus::Approved)?;

        persist_withdrawal_finance_stores(self, finance, &withdrawal_store, &finance_store).await?;
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
        let mut withdrawal_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();

        let order = withdrawal_store.reviewable_order(id, WithdrawalOrderStatus::Rejected)?;
        finance_store.reject_withdrawal(&order.user_id, order.amount_minor, &order.id)?;
        let result = withdrawal_store.mark_reviewed(id, WithdrawalOrderStatus::Rejected)?;

        persist_withdrawal_finance_stores(self, finance, &withdrawal_store, &finance_store).await?;
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
}

/// 在同一个数据库事务中保存提现和资金快照，确保冻结、打款和解冻不会只落一边。
async fn persist_withdrawal_finance_stores(
    withdrawals: &WithdrawalRepository,
    finance: &FinanceRepository,
    withdrawal_store: &WithdrawalStore,
    finance_store: &super::finance::FinanceStore,
) -> ApiResult<()> {
    match (&withdrawals.persistence, &finance.persistence) {
        (Some(database), Some(_)) => {
            let mut tx = database
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::Internal("提现资金事务开启失败".to_string()))?;
            save_withdrawal_store_in_transaction(&mut *tx, withdrawal_store).await?;
            save_finance_store_in_transaction(&mut *tx, finance_store).await?;
            tx.commit()
                .await
                .map_err(|_| ApiError::Internal("提现资金事务提交失败".to_string()))
        }
        (None, None) => Ok(()),
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
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("提现申请数据读取失败".to_string()))?;
        orders.insert(
            id.clone(),
            WithdrawalOrderSummary {
                id,
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
            },
        );
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

fn required_trimmed(value: impl Into<String>, label: &str) -> ApiResult<String> {
    let value = value.into().trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            user::{UserKind, UserStatus, UserSummary, WithdrawalMethod, WithdrawalMethodType},
            withdrawal::{CreateWithdrawalOrderRequest, WithdrawalOrderStatus},
        },
        services::withdrawal::WithdrawalStore,
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

    fn sample_user() -> UserSummary {
        UserSummary {
            id: "U10001".to_string(),
            username: "demo".to_string(),
            email: None,
            kind: UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id: None,
            invite_code: "ABCD1234".to_string(),
        }
    }

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
