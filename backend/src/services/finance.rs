//! 财务领域模型，定义账户汇总、流水与账户调整参数

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::{
        finance::{
            FinanceOverview, FinancialAccountSummary, LedgerEntry, LedgerEntryKind,
            ManualBalanceAdjustmentRequest,
        },
        order::OrderDetail,
        settlement::SettlementRun,
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

#[derive(Clone)]
pub struct FinanceRepository {
    inner: Arc<RwLock<FinanceStore>>,
    persistence: Option<BusinessDatabase>,
}

impl FinanceRepository {
    /// 返回带内置种子数据的内存仓储实例。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(FinanceStore::seeded())),
            persistence: None,
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_finance_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回财务总览指标。
    pub async fn overview(&self) -> ApiResult<FinanceOverview> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .overview()
    }

    /// 返回全部财务账户列表。
    pub async fn accounts(&self) -> ApiResult<Vec<FinancialAccountSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))
            .map(|store| store.accounts())
    }

    /// 返回财务流水列表。
    pub async fn ledger_entries(&self) -> ApiResult<Vec<LedgerEntry>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))
            .map(|store| store.ledger_entries())
    }

    /// 返回指定用户的财务流水列表。
    pub async fn user_ledger_entries(&self, user_id: &str) -> ApiResult<Vec<LedgerEntry>> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }

        Ok(self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .ledger_entries_for_user(user_id))
    }

    /// 校验用户余额是否可支付指定金额。
    pub async fn ensure_available(&self, user_id: &str, amount_minor: i64) -> ApiResult<()> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .ensure_available(user_id, amount_minor)
    }

    /// 获取用户资金账户，不存在时自动创建默认账户后返回。
    pub async fn account_or_create(&self, user_id: &str) -> ApiResult<FinancialAccountSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.account_or_create(user_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;

        Ok(result)
    }

    /// 校验订单是否满足退款条件。
    pub async fn ensure_order_can_refund(&self, order: &OrderDetail) -> ApiResult<()> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .ensure_order_can_refund(order)
    }

    /// 执行财务手工增减并记录流水。
    pub async fn manual_adjust(
        &self,
        payload: ManualBalanceAdjustmentRequest,
    ) -> ApiResult<LedgerEntry> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.manual_adjust(payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 按订单扣减用户资金。
    pub async fn debit_order(&self, order: &OrderDetail) -> ApiResult<LedgerEntry> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.debit_order(order)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 按订单执行退款并回写流水。
    pub async fn refund_order(&self, order: &OrderDetail) -> ApiResult<LedgerEntry> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.refund_order(order)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 将结算金额回写给用户。
    pub async fn credit_settlement(
        &self,
        settlement: &SettlementRun,
    ) -> ApiResult<Vec<LedgerEntry>> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.credit_settlement(settlement)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 按充值订单给用户入账；同一个充值单重复通知保持幂等。
    pub async fn credit_recharge(
        &self,
        user_id: &str,
        amount_minor: i64,
        recharge_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.credit_recharge(user_id, amount_minor, recharge_order_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 提交提现申请时冻结用户可用余额，并按提现单号保持幂等。
    pub async fn freeze_withdrawal(
        &self,
        user_id: &str,
        amount_minor: i64,
        withdrawal_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.freeze_withdrawal(user_id, amount_minor, withdrawal_order_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 审核通过提现申请时扣减冻结余额，并按提现单号保持幂等。
    pub async fn approve_withdrawal(
        &self,
        user_id: &str,
        amount_minor: i64,
        withdrawal_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.approve_withdrawal(user_id, amount_minor, withdrawal_order_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 驳回提现申请时把冻结余额退回可用余额，并按提现单号保持幂等。
    pub async fn reject_withdrawal(
        &self,
        user_id: &str,
        amount_minor: i64,
        withdrawal_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.reject_withdrawal(user_id, amount_minor, withdrawal_order_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 合买认购时扣减用户可用余额，并按参与记录 ID 保持幂等。
    pub async fn debit_group_buy(
        &self,
        user_id: &str,
        amount_minor: i64,
        participant_id: &str,
        plan_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let result = store.debit_group_buy(user_id, amount_minor, participant_id, plan_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    async fn persist(&self, store: &FinanceStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_finance_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct FinanceStore {
    accounts: BTreeMap<String, FinancialAccountSummary>,
    ledger_entries: Vec<LedgerEntry>,
    next_sequence: u64,
}

async fn load_finance_store(database: &BusinessDatabase) -> ApiResult<FinanceStore> {
    let pool = database.pool();
    let mut accounts = BTreeMap::new();
    for row in sqlx::query(
        "SELECT user_id, available_balance_minor, frozen_balance_minor
         FROM financial_accounts
         ORDER BY user_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?
    {
        let user_id: String = row
            .try_get("user_id")
            .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?;
        accounts.insert(
            user_id.clone(),
            FinancialAccountSummary {
                user_id,
                available_balance_minor: row
                    .try_get("available_balance_minor")
                    .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?,
                frozen_balance_minor: row
                    .try_get("frozen_balance_minor")
                    .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?,
            },
        );
    }

    let mut ledger_entries = Vec::new();
    for row in sqlx::query(
        "SELECT id, user_id, kind, amount_minor, balance_after_minor, reference_id, description, created_at
         FROM ledger_entries
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?
    {
        ledger_entries.push(LedgerEntry {
            id: row
                .try_get("id")
                .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?,
            user_id: row
                .try_get("user_id")
                .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?,
            kind: enum_from_string(
                row.try_get("kind")
                    .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?,
            )?,
            amount_minor: row
                .try_get("amount_minor")
                .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?,
            balance_after_minor: row
                .try_get("balance_after_minor")
                .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?,
            reference_id: row
                .try_get("reference_id")
                .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?,
            description: row
                .try_get("description")
                .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|_| ApiError::Internal("资金流水数据读取失败".to_string()))?,
        });
    }

    let next_sequence = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM finance_runtime WHERE key = 'next_sequence'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("资金运行数据读取失败".to_string()))?
    .unwrap_or_default();

    let mut reconciled_missing_accounts = false;
    for row in sqlx::query("SELECT id FROM users ORDER BY id ASC")
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::Internal("用户资金账户补齐数据读取失败".to_string()))?
    {
        let user_id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("用户资金账户补齐数据读取失败".to_string()))?;
        if accounts.contains_key(&user_id) {
            continue;
        }

        accounts.insert(
            user_id.clone(),
            FinancialAccountSummary {
                user_id,
                available_balance_minor: 0,
                frozen_balance_minor: 0,
            },
        );
        reconciled_missing_accounts = true;
    }

    if accounts.is_empty() && ledger_entries.is_empty() {
        let seeded = FinanceStore::seeded();
        save_finance_store(database, &seeded).await?;
        return Ok(seeded);
    }

    let store = FinanceStore {
        accounts,
        ledger_entries,
        next_sequence: u64::try_from(next_sequence).unwrap_or_default(),
    };

    if reconciled_missing_accounts {
        save_finance_store(database, &store).await?;
    }

    Ok(store)
}

async fn save_finance_store(database: &BusinessDatabase, store: &FinanceStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("资金事务开启失败".to_string()))?;

    for table in ["ledger_entries", "financial_accounts", "finance_runtime"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("资金数据清理失败".to_string()))?;
    }

    for account in store.accounts.values() {
        sqlx::query(
            "INSERT INTO financial_accounts
             (user_id, available_balance_minor, frozen_balance_minor)
             VALUES ($1, $2, $3)",
        )
        .bind(&account.user_id)
        .bind(account.available_balance_minor)
        .bind(account.frozen_balance_minor)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("资金账户数据保存失败".to_string()))?;
    }

    for entry in &store.ledger_entries {
        sqlx::query(
            "INSERT INTO ledger_entries
             (id, user_id, kind, amount_minor, balance_after_minor, reference_id, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&entry.id)
        .bind(&entry.user_id)
        .bind(enum_to_string(&entry.kind)?)
        .bind(entry.amount_minor)
        .bind(entry.balance_after_minor)
        .bind(&entry.reference_id)
        .bind(&entry.description)
        .bind(&entry.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("资金流水数据保存失败".to_string()))?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("资金流水序号过大".to_string()))?;
    sqlx::query("INSERT INTO finance_runtime (key, value) VALUES ('next_sequence', $1)")
        .bind(next_sequence)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("资金运行数据保存失败".to_string()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("资金事务提交失败".to_string()))
}

impl FinanceStore {
    /// 构建并返回种子数据。
    fn seeded() -> Self {
        let mut store = Self::default();
        store.seed_account("U10001", 12_000, 2_000);
        store.seed_account("U10002", 50_000, 0);
        store.seed_account("U10003", 100_000, 0);
        store.seed_account("U10004", 0, 0);
        store.seed_account("U90001", 520_000, 0);
        store
    }

    /// 返回内置种子或测试数据。
    fn seed_account(
        &mut self,
        user_id: &str,
        available_balance_minor: i64,
        frozen_balance_minor: i64,
    ) {
        self.accounts.insert(
            user_id.to_string(),
            FinancialAccountSummary {
                user_id: user_id.to_string(),
                available_balance_minor,
                frozen_balance_minor,
            },
        );
    }

    /// 处理 overview 的具体内部流程。
    fn overview(&self) -> ApiResult<FinanceOverview> {
        let mut total_balance_minor = 0_i64;
        for account in self.accounts.values() {
            total_balance_minor = total_balance_minor
                .checked_add(account.available_balance_minor)
                .and_then(|amount| amount.checked_add(account.frozen_balance_minor))
                .ok_or_else(|| {
                    ApiError::Internal("finance overview amount overflow".to_string())
                })?;
        }

        let today_payout_minor = self
            .ledger_entries
            .iter()
            .filter(|entry| entry.kind == LedgerEntryKind::PayoutCredit)
            .try_fold(0_i64, |total, entry| total.checked_add(entry.amount_minor))
            .ok_or_else(|| ApiError::Internal("finance payout amount overflow".to_string()))?;
        let today_recharge_minor = self
            .ledger_entries
            .iter()
            .filter(|entry| entry.kind == LedgerEntryKind::RechargeCredit)
            .try_fold(0_i64, |total, entry| total.checked_add(entry.amount_minor))
            .ok_or_else(|| ApiError::Internal("finance recharge amount overflow".to_string()))?;

        let pending_withdraw_minor = self
            .accounts
            .values()
            .try_fold(0_i64, |total, account| {
                total.checked_add(account.frozen_balance_minor)
            })
            .ok_or_else(|| ApiError::Internal("finance frozen amount overflow".to_string()))?;

        Ok(FinanceOverview {
            total_balance_minor,
            pending_withdraw_minor,
            today_recharge_minor,
            today_payout_minor,
        })
    }

    /// 处理 accounts 的具体内部流程。
    fn accounts(&self) -> Vec<FinancialAccountSummary> {
        self.accounts.values().cloned().collect()
    }

    /// 处理 ledger_entries 的具体内部流程。
    fn ledger_entries(&self) -> Vec<LedgerEntry> {
        self.ledger_entries.iter().rev().cloned().collect()
    }

    /// 处理 ledger_entries_for_user 的具体内部流程。
    fn ledger_entries_for_user(&self, user_id: &str) -> Vec<LedgerEntry> {
        self.ledger_entries
            .iter()
            .filter(|entry| entry.user_id == user_id)
            .cloned()
            .rev()
            .collect()
    }

    /// 处理 ensure_available 的具体内部流程。
    fn ensure_available(&self, user_id: &str, amount_minor: i64) -> ApiResult<()> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }
        if amount_minor <= 0 {
            return Err(ApiError::BadRequest(
                "amount must be greater than zero".to_string(),
            ));
        }

        let Some(account) = self.accounts.get(user_id) else {
            return Err(ApiError::BadRequest(
                "insufficient available balance".to_string(),
            ));
        };
        if account.available_balance_minor < amount_minor {
            return Err(ApiError::BadRequest(
                "insufficient available balance".to_string(),
            ));
        }

        Ok(())
    }

    /// 处理 account_or_create 的具体内部流程。
    fn account_or_create(&mut self, user_id: &str) -> ApiResult<FinancialAccountSummary> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }

        if let Some(account) = self.accounts.get(user_id) {
            return Ok(account.clone());
        }

        let account = FinancialAccountSummary {
            user_id: user_id.to_string(),
            available_balance_minor: 0,
            frozen_balance_minor: 0,
        };
        self.accounts
            .insert(account.user_id.clone(), account.clone());
        Ok(account)
    }

    /// 处理 ensure_order_can_refund 的具体内部流程。
    fn ensure_order_can_refund(&self, order: &OrderDetail) -> ApiResult<()> {
        if !self.has_reference(&LedgerEntryKind::OrderDebit, &order.id) {
            return Err(ApiError::BadRequest(
                "order debit ledger entry is required before refund".to_string(),
            ));
        }
        if self.has_reference(&LedgerEntryKind::OrderRefund, &order.id) {
            return Err(ApiError::Conflict(format!(
                "order `{}` has already been refunded",
                order.id
            )));
        }

        Ok(())
    }

    /// 处理 manual_adjust 的具体内部流程。
    fn manual_adjust(&mut self, payload: ManualBalanceAdjustmentRequest) -> ApiResult<LedgerEntry> {
        let user_id = payload.user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }
        if payload.amount_minor == 0 {
            return Err(ApiError::BadRequest(
                "adjustment amount must not be zero".to_string(),
            ));
        }

        let description = payload.description.trim();
        if description.is_empty() {
            return Err(ApiError::BadRequest(
                "adjustment description is required".to_string(),
            ));
        }

        self.apply_available_delta(
            user_id,
            LedgerEntryKind::ManualAdjustment,
            payload.amount_minor,
            None,
            description.to_string(),
        )
    }

    /// 处理 debit_order 的具体内部流程。
    fn debit_order(&mut self, order: &OrderDetail) -> ApiResult<LedgerEntry> {
        if self.has_reference(&LedgerEntryKind::OrderDebit, &order.id) {
            return Err(ApiError::Conflict(format!(
                "order `{}` has already been debited",
                order.id
            )));
        }
        self.ensure_available(&order.user_id, order.amount_minor)?;

        self.apply_available_delta(
            &order.user_id,
            LedgerEntryKind::OrderDebit,
            order
                .amount_minor
                .checked_neg()
                .ok_or_else(|| ApiError::BadRequest("order amount is too large".to_string()))?,
            Some(order.id.clone()),
            format!("投注扣款：{} {}", order.lottery_name, order.issue),
        )
    }

    /// 处理 refund_order 的具体内部流程。
    fn refund_order(&mut self, order: &OrderDetail) -> ApiResult<LedgerEntry> {
        if let Some(entry) = self.reference_entry(&LedgerEntryKind::OrderRefund, &order.id) {
            return Ok(entry);
        }
        self.ensure_order_can_refund(order)?;

        self.apply_available_delta(
            &order.user_id,
            LedgerEntryKind::OrderRefund,
            order.amount_minor,
            Some(order.id.clone()),
            format!("取消订单退款：{} {}", order.lottery_name, order.issue),
        )
    }

    /// 处理 credit_settlement 的具体内部流程。
    fn credit_settlement(&mut self, settlement: &SettlementRun) -> ApiResult<Vec<LedgerEntry>> {
        let mut entries = Vec::new();

        for order in &settlement.orders {
            if !order.is_winning || order.payout_minor <= 0 {
                continue;
            }

            let reference_id = format!("{}:{}", settlement.id, order.order_id);
            if let Some(entry) = self.reference_entry(&LedgerEntryKind::PayoutCredit, &reference_id)
            {
                entries.push(entry);
                continue;
            }

            let entry = self.apply_available_delta(
                &order.user_id,
                LedgerEntryKind::PayoutCredit,
                order.payout_minor,
                Some(reference_id),
                format!("中奖派奖：{} {}", settlement.lottery_name, settlement.issue),
            )?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// 处理充值入账，避免同一个充值订单重复生成流水。
    fn credit_recharge(
        &mut self,
        user_id: &str,
        amount_minor: i64,
        recharge_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let user_id = user_id.trim();
        let recharge_order_id = recharge_order_id.trim();
        if amount_minor <= 0 {
            return Err(ApiError::BadRequest(
                "recharge amount must be greater than zero".to_string(),
            ));
        }
        if recharge_order_id.is_empty() {
            return Err(ApiError::BadRequest(
                "recharge order id is required".to_string(),
            ));
        }
        if let Some(entry) =
            self.reference_entry(&LedgerEntryKind::RechargeCredit, recharge_order_id)
        {
            return Ok(entry);
        }

        self.account_or_create(user_id)?;
        self.apply_available_delta(
            user_id,
            LedgerEntryKind::RechargeCredit,
            amount_minor,
            Some(recharge_order_id.to_string()),
            format!("用户充值入账：{recharge_order_id}"),
        )
    }

    /// 提交提现申请时把可用余额转入冻结余额，并生成提现冻结流水。
    fn freeze_withdrawal(
        &mut self,
        user_id: &str,
        amount_minor: i64,
        withdrawal_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let user_id = user_id.trim();
        let withdrawal_order_id = withdrawal_order_id.trim();
        if amount_minor <= 0 {
            return Err(ApiError::BadRequest(
                "withdrawal amount must be greater than zero".to_string(),
            ));
        }
        if withdrawal_order_id.is_empty() {
            return Err(ApiError::BadRequest(
                "withdrawal order id is required".to_string(),
            ));
        }
        if let Some(entry) =
            self.reference_entry(&LedgerEntryKind::WithdrawalFreeze, withdrawal_order_id)
        {
            return Ok(entry);
        }
        self.ensure_available(user_id, amount_minor)?;

        let account = self
            .accounts
            .get_mut(user_id)
            .ok_or_else(|| ApiError::BadRequest("insufficient available balance".to_string()))?;
        account.available_balance_minor = account
            .available_balance_minor
            .checked_sub(amount_minor)
            .ok_or_else(|| ApiError::BadRequest("balance amount is too large".to_string()))?;
        account.frozen_balance_minor = account
            .frozen_balance_minor
            .checked_add(amount_minor)
            .ok_or_else(|| ApiError::BadRequest("balance amount is too large".to_string()))?;
        let balance_after_minor = account
            .available_balance_minor
            .checked_add(account.frozen_balance_minor)
            .ok_or_else(|| ApiError::BadRequest("balance amount is too large".to_string()))?;

        self.next_sequence += 1;
        let entry = LedgerEntry {
            id: format!("L{:012}", self.next_sequence),
            user_id: user_id.to_string(),
            kind: LedgerEntryKind::WithdrawalFreeze,
            amount_minor: amount_minor.checked_neg().ok_or_else(|| {
                ApiError::BadRequest("withdrawal amount is too large".to_string())
            })?,
            balance_after_minor,
            reference_id: Some(withdrawal_order_id.to_string()),
            description: format!("提现申请冻结：{withdrawal_order_id}"),
            created_at: current_timestamp_label(),
        };
        self.ledger_entries.push(entry.clone());

        Ok(entry)
    }

    /// 提现审核通过时扣减冻结余额，表示平台已经完成打款。
    fn approve_withdrawal(
        &mut self,
        user_id: &str,
        amount_minor: i64,
        withdrawal_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        self.apply_frozen_delta(
            user_id,
            amount_minor,
            withdrawal_order_id,
            LedgerEntryKind::WithdrawalPayout,
            amount_minor.checked_neg().ok_or_else(|| {
                ApiError::BadRequest("withdrawal amount is too large".to_string())
            })?,
            format!("提现打款完成：{withdrawal_order_id}"),
            false,
        )
    }

    /// 提现审核驳回时解冻余额，恢复到用户可用余额。
    fn reject_withdrawal(
        &mut self,
        user_id: &str,
        amount_minor: i64,
        withdrawal_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        self.apply_frozen_delta(
            user_id,
            amount_minor,
            withdrawal_order_id,
            LedgerEntryKind::WithdrawalReject,
            amount_minor,
            format!("提现驳回解冻：{withdrawal_order_id}"),
            true,
        )
    }

    /// 合买认购扣款，重复参与记录不会重复扣款。
    fn debit_group_buy(
        &mut self,
        user_id: &str,
        amount_minor: i64,
        participant_id: &str,
        plan_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let user_id = user_id.trim();
        let participant_id = participant_id.trim();
        let plan_id = plan_id.trim();
        if amount_minor <= 0 {
            return Err(ApiError::BadRequest(
                "group buy amount must be greater than zero".to_string(),
            ));
        }
        if participant_id.is_empty() {
            return Err(ApiError::BadRequest(
                "group buy participant id is required".to_string(),
            ));
        }
        if let Some(entry) = self.reference_entry(&LedgerEntryKind::GroupBuyDebit, participant_id) {
            return Ok(entry);
        }
        self.ensure_available(user_id, amount_minor)?;

        self.apply_available_delta(
            user_id,
            LedgerEntryKind::GroupBuyDebit,
            amount_minor
                .checked_neg()
                .ok_or_else(|| ApiError::BadRequest("group buy amount is too large".to_string()))?,
            Some(participant_id.to_string()),
            format!("合买认购扣款：{plan_id}"),
        )
    }

    /// 处理 account 的具体内部流程。
    #[cfg(test)]
    fn account(&self, user_id: &str) -> ApiResult<&FinancialAccountSummary> {
        let user_id = user_id.trim();
        self.accounts
            .get(user_id)
            .ok_or_else(|| ApiError::NotFound(format!("financial account `{user_id}` not found")))
    }

    /// 处理 apply_available_delta 的具体内部流程。
    fn apply_available_delta(
        &mut self,
        user_id: &str,
        kind: LedgerEntryKind,
        amount_minor: i64,
        reference_id: Option<String>,
        description: String,
    ) -> ApiResult<LedgerEntry> {
        let user_id = user_id.trim();
        let account = self.accounts.get_mut(user_id).ok_or_else(|| {
            ApiError::NotFound(format!("financial account `{user_id}` not found"))
        })?;
        let available_balance_minor = account
            .available_balance_minor
            .checked_add(amount_minor)
            .ok_or_else(|| ApiError::BadRequest("balance amount is too large".to_string()))?;
        if available_balance_minor < 0 {
            return Err(ApiError::BadRequest(
                "available balance cannot be negative".to_string(),
            ));
        }

        account.available_balance_minor = available_balance_minor;
        let balance_after_minor = account
            .available_balance_minor
            .checked_add(account.frozen_balance_minor)
            .ok_or_else(|| ApiError::BadRequest("balance amount is too large".to_string()))?;

        self.next_sequence += 1;
        let entry = LedgerEntry {
            id: format!("L{:012}", self.next_sequence),
            user_id: user_id.to_string(),
            kind,
            amount_minor,
            balance_after_minor,
            reference_id,
            description,
            created_at: current_timestamp_label(),
        };
        self.ledger_entries.push(entry.clone());

        Ok(entry)
    }

    /// 检查是否存在目标条件。
    fn has_reference(&self, kind: &LedgerEntryKind, reference_id: &str) -> bool {
        self.reference_entry(kind, reference_id).is_some()
    }

    /// 处理 reference_entry 的具体内部流程。
    fn reference_entry(&self, kind: &LedgerEntryKind, reference_id: &str) -> Option<LedgerEntry> {
        self.ledger_entries
            .iter()
            .find(|entry| {
                &entry.kind == kind && entry.reference_id.as_deref() == Some(reference_id)
            })
            .cloned()
    }

    /// 按提现审核结果调整冻结余额，驳回时同步退回用户可用余额。
    fn apply_frozen_delta(
        &mut self,
        user_id: &str,
        amount_minor: i64,
        withdrawal_order_id: &str,
        kind: LedgerEntryKind,
        ledger_amount_minor: i64,
        description: String,
        restore_available: bool,
    ) -> ApiResult<LedgerEntry> {
        let user_id = user_id.trim();
        let withdrawal_order_id = withdrawal_order_id.trim();
        if amount_minor <= 0 {
            return Err(ApiError::BadRequest(
                "withdrawal amount must be greater than zero".to_string(),
            ));
        }
        if withdrawal_order_id.is_empty() {
            return Err(ApiError::BadRequest(
                "withdrawal order id is required".to_string(),
            ));
        }
        if let Some(entry) = self.reference_entry(&kind, withdrawal_order_id) {
            return Ok(entry);
        }

        let account = self.accounts.get_mut(user_id).ok_or_else(|| {
            ApiError::NotFound(format!("financial account `{user_id}` not found"))
        })?;
        if account.frozen_balance_minor < amount_minor {
            return Err(ApiError::BadRequest(
                "frozen balance is insufficient".to_string(),
            ));
        }

        account.frozen_balance_minor = account
            .frozen_balance_minor
            .checked_sub(amount_minor)
            .ok_or_else(|| ApiError::BadRequest("balance amount is too large".to_string()))?;
        if restore_available {
            account.available_balance_minor = account
                .available_balance_minor
                .checked_add(amount_minor)
                .ok_or_else(|| ApiError::BadRequest("balance amount is too large".to_string()))?;
        }
        let balance_after_minor = account
            .available_balance_minor
            .checked_add(account.frozen_balance_minor)
            .ok_or_else(|| ApiError::BadRequest("balance amount is too large".to_string()))?;

        self.next_sequence += 1;
        let entry = LedgerEntry {
            id: format!("L{:012}", self.next_sequence),
            user_id: user_id.to_string(),
            kind,
            amount_minor: ledger_amount_minor,
            balance_after_minor,
            reference_id: Some(withdrawal_order_id.to_string()),
            description,
            created_at: current_timestamp_label(),
        };
        self.ledger_entries.push(entry.clone());

        Ok(entry)
    }
}

/// 处理 current_timestamp_label 的具体内部流程。
fn current_timestamp_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            finance::{LedgerEntryKind, ManualBalanceAdjustmentRequest},
            lottery::LotteryNumberType,
            order::{OrderDetail, OrderStatus},
            play::{PlayRuleCode, PlaySelection},
            settlement::{OrderSettlement, SettlementRun},
        },
        services::finance::FinanceStore,
    };

    #[test]
    /// 处理 store_debits_order_and_records_ledger 的具体内部流程。
    fn store_debits_order_and_records_ledger() {
        let mut store = FinanceStore::seeded();
        let order = order_detail("O000000000001", "U10001", 200, 0);

        let entry = store.debit_order(&order).expect("order can be debited");
        let account = store.account("U10001").expect("account exists");

        assert_eq!(account.available_balance_minor, 11_800);
        assert_eq!(entry.kind, LedgerEntryKind::OrderDebit);
        assert_eq!(entry.amount_minor, -200);
        assert_eq!(entry.balance_after_minor, 13_800);
        assert_eq!(entry.reference_id.as_deref(), Some("O000000000001"));
    }

    #[test]
    /// 处理 store_rejects_insufficient_order_balance 的具体内部流程。
    fn store_rejects_insufficient_order_balance() {
        let mut store = FinanceStore::seeded();
        let order = order_detail("O000000000001", "U10004", 200, 0);

        assert!(store
            .debit_order(&order)
            .expect_err("zero balance user cannot bet")
            .to_string()
            .contains("insufficient available balance"));
    }

    #[test]
    /// 合买认购扣款会写入专用流水，并按参与记录保持幂等。
    fn store_debits_group_buy_once() {
        let mut store = FinanceStore::seeded();

        let entry = store
            .debit_group_buy("U10001", 1_000, "G202606050001-P001", "G202606050001")
            .expect("group buy debit can be applied");
        let repeated = store
            .debit_group_buy("U10001", 1_000, "G202606050001-P001", "G202606050001")
            .expect("group buy debit is idempotent");
        let account = store.account("U10001").expect("account exists");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(entry.kind, LedgerEntryKind::GroupBuyDebit);
        assert_eq!(entry.amount_minor, -1_000);
        assert_eq!(entry.reference_id.as_deref(), Some("G202606050001-P001"));
        assert_eq!(account.available_balance_minor, 11_000);
    }

    #[test]
    /// 缺少资金账户的历史用户下注时按 0 余额处理，不向用户暴露账户缺失错误。
    fn store_rejects_missing_account_as_insufficient_balance() {
        let mut store = FinanceStore::default();
        let order = order_detail("O000000000001", "U-MISSING", 200, 0);

        assert!(store
            .debit_order(&order)
            .expect_err("missing account user cannot bet")
            .to_string()
            .contains("insufficient available balance"));
    }

    #[test]
    /// 查询或注册后的账户初始化会创建 0 余额资金账户。
    fn store_account_or_create_creates_zero_balance_account() {
        let mut store = FinanceStore::default();

        let account = store
            .account_or_create("U-NEW")
            .expect("missing account should be created");

        assert_eq!(account.user_id, "U-NEW");
        assert_eq!(account.available_balance_minor, 0);
        assert_eq!(account.frozen_balance_minor, 0);
    }

    #[test]
    /// 处理 store_refunds_order_once 的具体内部流程。
    fn store_refunds_order_once() {
        let mut store = FinanceStore::seeded();
        let order = order_detail("O000000000001", "U10001", 200, 0);
        store.debit_order(&order).expect("order can be debited");

        let refunded = store.refund_order(&order).expect("order can be refunded");
        let repeated = store.refund_order(&order).expect("refund is idempotent");
        let account = store.account("U10001").expect("account exists");

        assert_eq!(account.available_balance_minor, 12_000);
        assert_eq!(refunded.id, repeated.id);
        assert_eq!(refunded.kind, LedgerEntryKind::OrderRefund);
        assert_eq!(refunded.amount_minor, 200);
    }

    #[test]
    /// 处理 store_credits_winning_settlement 的具体内部流程。
    fn store_credits_winning_settlement() {
        let mut store = FinanceStore::seeded();
        let settlement = settlement_run("S000000000001", "U10001", 2_000);

        let entries = store
            .credit_settlement(&settlement)
            .expect("settlement can be credited");
        let account = store.account("U10001").expect("account exists");
        let overview = store.overview().expect("overview can be calculated");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].kind, LedgerEntryKind::PayoutCredit);
        assert_eq!(entries[0].amount_minor, 2_000);
        assert_eq!(account.available_balance_minor, 14_000);
        assert_eq!(overview.today_payout_minor, 2_000);
    }

    #[test]
    /// 处理 store_applies_manual_adjustment 的具体内部流程。
    fn store_applies_manual_adjustment() {
        let mut store = FinanceStore::seeded();

        let entry = store
            .manual_adjust(ManualBalanceAdjustmentRequest {
                user_id: "U10001".to_string(),
                amount_minor: 1_000,
                description: "后台补款".to_string(),
            })
            .expect("manual adjustment can be applied");
        let account = store.account("U10001").expect("account exists");

        assert_eq!(entry.kind, LedgerEntryKind::ManualAdjustment);
        assert_eq!(entry.amount_minor, 1_000);
        assert_eq!(account.available_balance_minor, 13_000);
    }

    #[test]
    /// 充值入账会增加余额，并按充值单号保持幂等。
    fn store_credits_recharge_once() {
        let mut store = FinanceStore::seeded();

        let entry = store
            .credit_recharge("U10001", 1_500, "R000000000001")
            .expect("recharge can be credited");
        let repeated = store
            .credit_recharge("U10001", 1_500, "R000000000001")
            .expect("recharge credit is idempotent");
        let account = store.account("U10001").expect("account exists");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(entry.kind, LedgerEntryKind::RechargeCredit);
        assert_eq!(entry.amount_minor, 1_500);
        assert_eq!(account.available_balance_minor, 13_500);
    }

    #[test]
    /// 提现申请会冻结可用余额并记录一条提现冻结流水，重复冻结同一提现单保持幂等。
    fn store_freezes_withdrawal_once() {
        let mut store = FinanceStore::seeded();

        let entry = store
            .freeze_withdrawal("U10001", 1_500, "W000000000001")
            .expect("withdrawal can freeze available balance");
        let repeated = store
            .freeze_withdrawal("U10001", 1_500, "W000000000001")
            .expect("withdrawal freeze is idempotent");
        let account = store.account("U10001").expect("account exists");
        let overview = store.overview().expect("overview can be calculated");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(entry.kind, LedgerEntryKind::WithdrawalFreeze);
        assert_eq!(entry.amount_minor, -1_500);
        assert_eq!(account.available_balance_minor, 10_500);
        assert_eq!(account.frozen_balance_minor, 3_500);
        assert_eq!(overview.pending_withdraw_minor, 3_500);
    }

    #[test]
    /// 提现审核通过会扣减冻结余额，并生成提现打款流水。
    fn store_approves_withdrawal_from_frozen_balance_once() {
        let mut store = FinanceStore::seeded();
        store
            .freeze_withdrawal("U10001", 1_500, "W000000000001")
            .expect("withdrawal can freeze available balance");

        let entry = store
            .approve_withdrawal("U10001", 1_500, "W000000000001")
            .expect("withdrawal can be approved");
        let repeated = store
            .approve_withdrawal("U10001", 1_500, "W000000000001")
            .expect("withdrawal approval is idempotent");
        let account = store.account("U10001").expect("account exists");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(entry.kind, LedgerEntryKind::WithdrawalPayout);
        assert_eq!(entry.amount_minor, -1_500);
        assert_eq!(account.available_balance_minor, 10_500);
        assert_eq!(account.frozen_balance_minor, 2_000);
    }

    #[test]
    /// 提现审核驳回会把冻结余额退回可用余额，并生成解冻流水。
    fn store_rejects_withdrawal_and_restores_available_balance_once() {
        let mut store = FinanceStore::seeded();
        store
            .freeze_withdrawal("U10001", 1_500, "W000000000001")
            .expect("withdrawal can freeze available balance");

        let entry = store
            .reject_withdrawal("U10001", 1_500, "W000000000001")
            .expect("withdrawal can be rejected");
        let repeated = store
            .reject_withdrawal("U10001", 1_500, "W000000000001")
            .expect("withdrawal rejection is idempotent");
        let account = store.account("U10001").expect("account exists");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(entry.kind, LedgerEntryKind::WithdrawalReject);
        assert_eq!(entry.amount_minor, 1_500);
        assert_eq!(account.available_balance_minor, 12_000);
        assert_eq!(account.frozen_balance_minor, 2_000);
    }

    #[test]
    /// 处理 store_filters_ledger_entries_by_user 的具体内部流程。
    fn store_filters_ledger_entries_by_user() {
        let mut store = FinanceStore::seeded();
        let order = order_detail("O000000000001", "U10001", 200, 0);
        let _ = store.debit_order(&order).expect("debit for user 1");

        let _ = store
            .manual_adjust(ManualBalanceAdjustmentRequest {
                user_id: "U10002".to_string(),
                amount_minor: 500,
                description: "other user adjustment".to_string(),
            })
            .expect("adjustment for user 2");

        let entries = store.ledger_entries_for_user("U10001");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].user_id, "U10001");
        assert_eq!(entries[0].kind, LedgerEntryKind::OrderDebit);
    }

    /// 处理 order_detail 的具体内部流程。
    fn order_detail(id: &str, user_id: &str, amount_minor: i64, payout_minor: i64) -> OrderDetail {
        OrderDetail {
            id: id.to_string(),
            user_id: user_id.to_string(),
            lottery_id: "fc3d".to_string(),
            lottery_name: "福彩 3D".to_string(),
            issue: "2026155".to_string(),
            rule_code: PlayRuleCode::ThreeDirect,
            number_type: LotteryNumberType::ThreeDigit,
            selection: PlaySelection::default(),
            stake_count: 1,
            unit_amount_minor: amount_minor,
            amount_minor,
            odds_basis_points: 100_000,
            expanded_bets: vec!["247".to_string()],
            draw_number: None,
            matched_bets: Vec::new(),
            payout_minor,
            status: OrderStatus::PendingDraw,
            settled_at: None,
            created_at: "unix:1780388800".to_string(),
        }
    }

    /// 处理 settlement_run 的具体内部流程。
    fn settlement_run(id: &str, user_id: &str, payout_minor: i64) -> SettlementRun {
        SettlementRun {
            id: id.to_string(),
            draw_issue_id: "D000000000001".to_string(),
            lottery_id: "fc3d".to_string(),
            lottery_name: "福彩 3D".to_string(),
            issue: "2026155".to_string(),
            draw_number: "2,4,7".to_string(),
            settled_order_count: 1,
            winning_order_count: 1,
            total_stake_amount_minor: 200,
            total_payout_minor: payout_minor,
            created_at: "unix:1780389000".to_string(),
            orders: vec![OrderSettlement {
                order_id: "O000000000001".to_string(),
                user_id: user_id.to_string(),
                rule_code: PlayRuleCode::ThreeDirect,
                stake_count: 1,
                amount_minor: 200,
                is_winning: payout_minor > 0,
                matched_bets: vec!["247".to_string()],
                odds_basis_points: 100_000,
                payout_minor,
                status: OrderStatus::Won,
            }],
        }
    }
}
