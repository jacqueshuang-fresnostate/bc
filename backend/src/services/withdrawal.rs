//! 提现服务，管理用户提现申请并联动财务冻结

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::{
        user::{UserSummary, WithdrawalMethod},
        withdrawal::{CreateWithdrawalOrderRequest, WithdrawalOrderStatus, WithdrawalOrderSummary},
    },
    error::{ApiError, ApiResult},
    services::{business_database::BusinessDatabase, finance::FinanceRepository},
};

use super::business_database::{enum_from_string, enum_to_string};

#[derive(Clone)]
pub struct WithdrawalRepository {
    inner: Arc<RwLock<WithdrawalStore>>,
    persistence: Option<BusinessDatabase>,
}

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

    /// 创建提现申请，并在创建成功前冻结对应用户可用余额。
    pub async fn create_order(
        &self,
        user: &UserSummary,
        method: &WithdrawalMethod,
        request: CreateWithdrawalOrderRequest,
        finance: &FinanceRepository,
    ) -> ApiResult<WithdrawalOrderSummary> {
        let order = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?;
            store.draft_order(user, method, request)?
        };

        finance
            .freeze_withdrawal(&order.user_id, order.amount_minor, &order.id)
            .await?;

        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("withdrawal store lock poisoned".to_string()))?;
            let result = store.insert_order(order)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    async fn persist(&self, store: &WithdrawalStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_withdrawal_store(persistence, store).await?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct WithdrawalStore {
    orders: BTreeMap<String, WithdrawalOrderSummary>,
    next_sequence: u64,
}

impl WithdrawalStore {
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
    fn draft_order(
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
    fn insert_order(&mut self, order: WithdrawalOrderSummary) -> ApiResult<WithdrawalOrderSummary> {
        if self.orders.contains_key(&order.id) {
            return Err(ApiError::Conflict(format!(
                "withdrawal order `{}` already exists",
                order.id
            )));
        }
        self.orders.insert(order.id.clone(), order.clone());
        Ok(order)
    }
}

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

async fn save_withdrawal_store(
    database: &BusinessDatabase,
    store: &WithdrawalStore,
) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("提现事务开启失败".to_string()))?;

    for table in ["withdrawal_orders", "withdrawal_runtime"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
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
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("提现申请数据保存失败".to_string()))?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("提现序号过大".to_string()))?;
    sqlx::query("INSERT INTO withdrawal_runtime (key, value) VALUES ('next_sequence', $1)")
        .bind(next_sequence)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("提现运行数据保存失败".to_string()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("提现事务提交失败".to_string()))
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
