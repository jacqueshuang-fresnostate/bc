//! 财务领域模型，定义账户汇总、流水与账户调整参数

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, PgConnection, Row};
use tokio::sync::Mutex;

use crate::{
    domain::{
        finance::{
            FinanceOverview, FinancialAccountSummary, LedgerEntry, LedgerEntryKind,
            ManualBalanceAdjustmentRequest, WithdrawalTurnoverSummary,
        },
        group_buy::{GroupBuyParticipant, GroupBuyPlan},
        order::OrderDetail,
        settlement::{OrderSettlement, SettlementRun},
    },
    error::{ApiError, ApiResult},
};

use super::{
    business_database::{enum_from_string, enum_to_string, BusinessDatabase},
    pagination::{ListPage, PageRequest},
};

#[derive(Clone)]
/// 资金账户和资金流水仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct FinanceRepository {
    /// 资金模块内存快照锁，保证余额和流水读写的一致性。
    pub(crate) inner: Arc<RwLock<FinanceStore>>,
    /// 可选数据库持久化句柄；内存模式下为空。
    pub(crate) persistence: Option<BusinessDatabase>,
    /// 串行化兼容层资金写操作，避免异步增量落库时旧快照覆盖新快照。
    pub(crate) mutation_lock: Arc<Mutex<()>>,
}

/// 资金账户和资金流水仓储，负责该模块数据读取、业务变更和持久化协调。
impl FinanceRepository {
    /// 返回带内置种子数据的内存仓储实例。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(FinanceStore::seeded())),
            persistence: None,
            mutation_lock: Arc::new(Mutex::new(())),
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_finance_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
            mutation_lock: Arc::new(Mutex::new(())),
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

    /// 按用户 ID 批量读取资金账户，供后台分页列表只补当前页余额。
    pub async fn accounts_for_user_ids(
        &self,
        user_ids: &[String],
    ) -> ApiResult<Vec<FinancialAccountSummary>> {
        let user_ids = normalized_user_ids(user_ids);
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(persistence) = &self.persistence {
            return query_financial_accounts_for_user_ids(persistence, &user_ids).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))
            .map(|store| {
                store
                    .accounts()
                    .into_iter()
                    .filter(|account| user_ids.contains(&account.user_id))
                    .collect()
            })
    }

    /// 返回财务流水列表。
    pub async fn ledger_entries(&self) -> ApiResult<Vec<LedgerEntry>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))
            .map(|store| store.ledger_entries())
    }

    /// 返回指定用户真实充值本金累计金额。
    ///
    /// 数据库模式只读取增量累计表，避免后台清理资金流水审计列表后把聊天大厅发言门槛误判为未充值。
    pub async fn total_recharge_credit_minor(&self, user_id: &str) -> ApiResult<i64> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }
        if let Some(persistence) = &self.persistence {
            return query_recharge_credit_total_minor(persistence, user_id).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .recharge_credit_total_minor_for_user(user_id)
    }

    /// 返回指定用户提现前有效投注累计摘要，数据库模式直接读取增量累计表。
    pub async fn withdrawal_turnover_for_user(
        &self,
        user_id: &str,
    ) -> ApiResult<WithdrawalTurnoverSummary> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("用户 ID 不能为空".to_string()));
        }
        if let Some(persistence) = &self.persistence {
            return query_withdrawal_turnover_for_user(persistence, user_id).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .withdrawal_turnover_for_user(user_id)
    }

    /// 按用户 ID 集合批量汇总真实充值本金，避免邀请中心读取全量资金流水后再过滤。
    pub async fn recharge_credit_totals_for_user_ids(
        &self,
        user_ids: &[String],
    ) -> ApiResult<BTreeMap<String, i64>> {
        let user_ids = normalized_user_ids(user_ids);
        if user_ids.is_empty() {
            return Ok(BTreeMap::new());
        }
        if let Some(persistence) = &self.persistence {
            return query_recharge_credit_totals_for_user_ids(persistence, &user_ids).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .recharge_credit_totals_for_user_ids(&user_ids)
    }

    /// 一键清除资金流水历史；只清除审计列表，不回滚账户余额也不重置流水序号。
    pub async fn clear_ledger_entries(&self) -> ApiResult<usize> {
        let _mutation_guard = self.mutation_lock.lock().await;
        if let Some(persistence) = &self.persistence {
            let deleted_count = clear_ledger_entries_in_database(persistence).await?;
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            store.clear_ledger_entries();
            return Ok(deleted_count);
        }

        let (deleted_count, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
            let deleted_count = store.clear_ledger_entries();
            (deleted_count, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(deleted_count)
    }

    /// 分页返回资金账户；数据库模式下将用户过滤和分页下推到 SQL。
    pub async fn account_page(
        &self,
        user_id: Option<&str>,
        username: Option<&str>,
        usernames_by_user_id: &BTreeMap<String, String>,
        excluded_user_id: Option<&str>,
        page: PageRequest,
    ) -> ApiResult<ListPage<FinancialAccountSummary>> {
        let user_id = normalized_optional_filter(user_id);
        let username = normalized_optional_filter(username);
        let excluded_user_id = normalized_optional_filter(excluded_user_id);
        if let Some(persistence) = &self.persistence {
            return query_financial_account_page(
                persistence,
                user_id.as_deref(),
                username.as_deref(),
                excluded_user_id.as_deref(),
                page,
            )
            .await;
        }

        let accounts = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .accounts()
            .into_iter()
            .filter(|account| {
                user_id
                    .as_deref()
                    .map_or(true, |target| account.user_id == target)
                    && username.as_deref().map_or(true, |target| {
                        username_matches_account(usernames_by_user_id, &account.user_id, target)
                    })
                    && excluded_user_id
                        .as_deref()
                        .map_or(true, |excluded| account.user_id != excluded)
            })
            .collect::<Vec<_>>();
        let mut accounts = accounts;
        sort_accounts_by_latest_user_desc(&mut accounts);
        Ok(ListPage::from_all(accounts, page))
    }

    /// 分页返回资金流水；数据库模式下将用户过滤、机器人过滤和分页下推到 SQL。
    pub async fn ledger_entry_page(
        &self,
        user_id: Option<&str>,
        excluded_user_id: Option<&str>,
        page: PageRequest,
    ) -> ApiResult<ListPage<LedgerEntry>> {
        let user_id = normalized_optional_filter(user_id);
        let excluded_user_id = normalized_optional_filter(excluded_user_id);
        if let Some(persistence) = &self.persistence {
            return query_ledger_entry_page(
                persistence,
                user_id.as_deref(),
                excluded_user_id.as_deref(),
                page,
            )
            .await;
        }

        let mut entries = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .ledger_entries()
            .into_iter()
            .filter(|entry| {
                user_id
                    .as_deref()
                    .map_or(true, |target| entry.user_id == target)
                    && excluded_user_id
                        .as_deref()
                        .map_or(true, |excluded| entry.user_id != excluded)
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| right.id.cmp(&left.id));
        Ok(ListPage::from_all(entries, page))
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

    /// 分页返回指定用户资金流水，供用户端列表避免全量拉取。
    pub async fn user_ledger_entry_page(
        &self,
        user_id: &str,
        page: PageRequest,
    ) -> ApiResult<ListPage<LedgerEntry>> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest("user id is required".to_string()));
        }

        self.ledger_entry_page(Some(user_id), None, page).await
    }

    /// 返回指定流水类型集合的数据，供返利统计等聚合场景避免读取全部流水。
    pub async fn ledger_entries_by_kinds(
        &self,
        kinds: &[LedgerEntryKind],
    ) -> ApiResult<Vec<LedgerEntry>> {
        Ok(self
            .ledger_entry_kind_page(None, kinds, PageRequest::default())
            .await?
            .items)
    }

    /// 分页返回指定流水类型集合的数据，供代理返利明细按流水源分页。
    pub async fn ledger_entry_kind_page(
        &self,
        user_id: Option<&str>,
        kinds: &[LedgerEntryKind],
        page: PageRequest,
    ) -> ApiResult<ListPage<LedgerEntry>> {
        if kinds.is_empty() {
            return Ok(ListPage::from_all(Vec::new(), page));
        }
        let user_id = normalized_optional_filter(user_id);
        if let Some(persistence) = &self.persistence {
            return query_ledger_entry_kind_page(persistence, user_id.as_deref(), kinds, page)
                .await;
        }

        let mut entries = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .ledger_entries()
            .into_iter()
            .filter(|entry| {
                user_id
                    .as_deref()
                    .map_or(true, |target| entry.user_id == target)
                    && kinds.iter().any(|kind| *kind == entry.kind)
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        Ok(ListPage::from_all(entries, page))
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
        let _mutation_guard = self.mutation_lock.lock().await;
        let (result, previous, mut snapshot) = {
            let previous = self
                .inner
                .read()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
                .clone();
            let mut snapshot = previous.clone();
            let result = snapshot.account_or_create(user_id)?;
            (result, previous, snapshot)
        };
        self.persist_incremental(&previous, &mut snapshot).await?;
        self.replace_store(snapshot)?;

        Ok(result)
    }

    /// 执行财务手工增减并记录流水。
    pub async fn manual_adjust(
        &self,
        payload: ManualBalanceAdjustmentRequest,
    ) -> ApiResult<LedgerEntry> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let (mut result, previous, mut snapshot) = {
            let previous = self
                .inner
                .read()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
                .clone();
            let mut snapshot = previous.clone();
            let result = snapshot.manual_adjust(payload)?;
            (result, previous, snapshot)
        };
        let id_remap = self.persist_incremental(&previous, &mut snapshot).await?;
        id_remap.apply_to_entry(&mut result);
        self.replace_store(snapshot)?;
        Ok(result)
    }

    #[cfg(test)]
    /// 按充值订单给上级代理发放返利；仅供返利策略测试使用，运行路径必须走充值确认事务。
    pub async fn credit_recharge_rebate(
        &self,
        agent_user_id: &str,
        invitee_user_id: &str,
        rebate_amount_minor: i64,
        recharge_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let (mut result, previous, mut snapshot) = {
            let previous = self
                .inner
                .read()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
                .clone();
            let mut snapshot = previous.clone();
            let result = snapshot.credit_recharge_rebate(
                agent_user_id,
                invitee_user_id,
                rebate_amount_minor,
                recharge_order_id,
            )?;
            (result, previous, snapshot)
        };
        let id_remap = self.persist_incremental(&previous, &mut snapshot).await?;
        id_remap.apply_to_entry(&mut result);
        self.replace_store(snapshot)?;
        Ok(result)
    }

    /// 后台处理代理返利提现，从代理可用余额扣减并记录独立返利提现流水。
    pub async fn withdraw_agent_rebate(
        &self,
        agent_user_id: &str,
        amount_minor: i64,
        description: &str,
    ) -> ApiResult<LedgerEntry> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let (mut result, previous, mut snapshot) = {
            let previous = self
                .inner
                .read()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
                .clone();
            let mut snapshot = previous.clone();
            let result =
                snapshot.withdraw_agent_rebate(agent_user_id, amount_minor, description)?;
            (result, previous, snapshot)
        };
        let id_remap = self.persist_incremental(&previous, &mut snapshot).await?;
        id_remap.apply_to_entry(&mut result);
        self.replace_store(snapshot)?;
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
        let _mutation_guard = self.mutation_lock.lock().await;
        let (mut result, previous, mut snapshot) = {
            let previous = self
                .inner
                .read()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
                .clone();
            let mut snapshot = previous.clone();
            let result =
                snapshot.debit_group_buy(user_id, amount_minor, participant_id, plan_id)?;
            (result, previous, snapshot)
        };
        let id_remap = self.persist_incremental(&previous, &mut snapshot).await?;
        id_remap.apply_to_entry(&mut result);
        self.replace_store(snapshot)?;
        Ok(result)
    }

    /// 合买取消或流单时按参与记录退还认购金额。
    pub async fn refund_group_buy_plan(
        &self,
        plan: &GroupBuyPlan,
        reason: &str,
    ) -> ApiResult<Vec<LedgerEntry>> {
        let _mutation_guard = self.mutation_lock.lock().await;
        let (mut result, previous, mut snapshot) = {
            let previous = self
                .inner
                .read()
                .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
                .clone();
            let mut snapshot = previous.clone();
            let result = snapshot.refund_group_buy_plan(plan, reason)?;
            (result, previous, snapshot)
        };
        let id_remap = self.persist_incremental(&previous, &mut snapshot).await?;
        id_remap.apply_to_entries(&mut result);
        self.replace_store(snapshot)?;
        Ok(result)
    }
    /// 把当前仓储快照同步保存到持久化存储。
    async fn persist(&self, store: &FinanceStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_finance_store(persistence, store).await?;
        }

        Ok(())
    }

    /// 把资金快照差异增量保存到持久化存储，避免手工调账、合买扣款等路径重写全量流水。
    async fn persist_incremental(
        &self,
        previous: &FinanceStore,
        store: &mut FinanceStore,
    ) -> ApiResult<LedgerEntryIdRemap> {
        if let Some(persistence) = &self.persistence {
            let mut tx = persistence
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::Internal("资金事务开启失败".to_string()))?;
            let id_remap =
                save_finance_store_incremental_in_transaction(&mut *tx, previous, store).await?;
            tx.commit()
                .await
                .map_err(|_| ApiError::Internal("资金事务提交失败".to_string()))?;
            return Ok(id_remap);
        }

        Ok(LedgerEntryIdRemap::default())
    }

    /// 从数据库重新加载资金账户和资金流水快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_finance_store(persistence).await?;
        self.replace_store(store)?;
        Ok(true)
    }

    /// 用事务提交后的快照替换当前资金账户和资金流水内存状态。
    pub(crate) fn replace_store(&self, store: FinanceStore) -> ApiResult<()> {
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))? = store;
        Ok(())
    }

    /// 数据库原子事务提交后，以增量方式把本次变更合并进内存快照，避免用绝对值覆盖其他并发操作的余额变更。
    pub(crate) fn apply_persisted_order_debits(
        &self,
        previous_accounts: Vec<FinancialAccountSummary>,
        new_accounts: Vec<FinancialAccountSummary>,
        ledger_entries: Vec<LedgerEntry>,
        next_sequence: u64,
    ) -> ApiResult<()> {
        let mut store = self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?;
        for (prev, new) in previous_accounts.iter().zip(new_accounts.iter()) {
            let delta_available = new.available_balance_minor - prev.available_balance_minor;
            let delta_frozen = new.frozen_balance_minor - prev.frozen_balance_minor;
            if let Some(existing) = store.accounts.get_mut(&new.user_id) {
                existing.available_balance_minor += delta_available;
                existing.frozen_balance_minor += delta_frozen;
            } else {
                store.accounts.insert(new.user_id.clone(), new.clone());
            }
        }
        store.ledger_entries.extend(ledger_entries);
        store.next_sequence = store.next_sequence.max(next_sequence);
        Ok(())
    }
}

/// 归一化可选筛选值，空字符串不参与过滤。
fn normalized_optional_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

/// 判断资金账户关联用户名是否命中后台关键字搜索；未知用户不命中过滤。
fn username_matches_account(
    usernames_by_user_id: &BTreeMap<String, String>,
    user_id: &str,
    keyword: &str,
) -> bool {
    let keyword = keyword.trim().to_lowercase();
    if keyword.is_empty() {
        return true;
    }
    usernames_by_user_id
        .get(user_id)
        .map(|username| username.to_lowercase().contains(&keyword))
        .unwrap_or(false)
}

/// 从用户编号中提取数字序号，用于“最新用户优先”排序。
fn user_id_sequence_for_sort(user_id: &str) -> Option<u64> {
    user_id.trim().strip_prefix('U')?.parse().ok()
}

/// 资金账户按用户编号倒序，保证内存模式与数据库分页顺序一致。
fn sort_accounts_by_latest_user_desc(accounts: &mut [FinancialAccountSummary]) {
    accounts.sort_by(|left, right| {
        user_id_sequence_for_sort(&right.user_id)
            .cmp(&user_id_sequence_for_sort(&left.user_id))
            .then_with(|| right.user_id.cmp(&left.user_id))
    });
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
/// 资金账户和资金流水运行时数据快照，用于内存模式和数据库持久化前的业务校验。
pub(crate) struct FinanceStore {
    accounts: BTreeMap<String, FinancialAccountSummary>,
    ledger_entries: Vec<LedgerEntry>,
    next_sequence: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// 数据库持久化时的资金流水 ID 映射，用于把快照临时 ID 回写为 PostgreSQL 序列 ID。
pub(crate) struct LedgerEntryIdRemap {
    ids: BTreeMap<String, String>,
}

impl LedgerEntryIdRemap {
    /// 记录一次流水 ID 替换；相同 ID 不需要写入映射。
    fn insert(&mut self, old_id: String, new_id: String) {
        if old_id != new_id {
            self.ids.insert(old_id, new_id);
        }
    }

    /// 把映射应用到单条流水，确保接口返回和内存快照里的 ID 一致。
    pub(crate) fn apply_to_entry(&self, entry: &mut LedgerEntry) {
        if let Some(new_id) = self.ids.get(&entry.id) {
            entry.id = new_id.clone();
        }
    }

    /// 把映射应用到多条流水，供批量退款或派奖结果回写。
    pub(crate) fn apply_to_entries(&self, entries: &mut [LedgerEntry]) {
        for entry in entries {
            self.apply_to_entry(entry);
        }
    }

    /// 把映射应用到可选流水，供充值返利这类可选入账结果回写。
    pub(crate) fn apply_to_optional_entry(&self, entry: &mut Option<LedgerEntry>) {
        if let Some(entry) = entry {
            self.apply_to_entry(entry);
        }
    }
}

/// 从数据库加载资金账户和资金流水运行时快照，空库时按模块规则初始化。
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
        ledger_entries.push(ledger_entry_from_row(row)?);
    }

    let runtime_next_sequence = sqlx::query_scalar::<_, i64>(
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

    let runtime_next_sequence = u64::try_from(runtime_next_sequence).unwrap_or_default();
    let next_sequence =
        runtime_next_sequence.max(next_sequence_from_ledger_entries(&ledger_entries));
    let reconciled_next_sequence = next_sequence != runtime_next_sequence;

    let store = FinanceStore {
        accounts,
        ledger_entries,
        next_sequence,
    };

    sync_ledger_entry_database_sequence(database, next_sequence).await?;

    if reconciled_missing_accounts || reconciled_next_sequence {
        save_finance_store(database, &store).await?;
    }

    Ok(store)
}

/// 数据库模式下一键清除资金流水审计记录，不重写资金账户余额。
async fn clear_ledger_entries_in_database(database: &BusinessDatabase) -> ApiResult<usize> {
    let result = sqlx::query("DELETE FROM ledger_entries")
        .execute(database.pool())
        .await
        .map_err(|error| {
            tracing::error!(%error, "资金流水数据清除失败");
            ApiError::Internal("资金流水数据清除失败".to_string())
        })?;
    usize::try_from(result.rows_affected())
        .map_err(|_| ApiError::Internal("资金流水清除数量无效".to_string()))
}

/// 从数据库行恢复资金流水结构，供全量加载和分页查询共用。
fn ledger_entry_from_row(row: PgRow) -> ApiResult<LedgerEntry> {
    Ok(LedgerEntry {
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
    })
}

/// 数据库模式下分页读取资金账户，避免后台资金账户先查全量再裁剪。
async fn query_financial_account_page(
    database: &BusinessDatabase,
    user_id: Option<&str>,
    username: Option<&str>,
    excluded_user_id: Option<&str>,
    page: PageRequest,
) -> ApiResult<ListPage<FinancialAccountSummary>> {
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM financial_accounts AS account
         LEFT JOIN users AS app_user ON app_user.id = account.user_id
         WHERE ($1::text IS NULL OR account.user_id = $1)
           AND ($2::text IS NULL OR app_user.username ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR account.user_id <> $3)",
    )
    .bind(user_id)
    .bind(username)
    .bind(excluded_user_id)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("资金账户分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("资金账户分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT user_id, available_balance_minor, frozen_balance_minor
         FROM financial_accounts AS account
         LEFT JOIN users AS app_user ON app_user.id = account.user_id
         WHERE ($1::text IS NULL OR account.user_id = $1)
           AND ($2::text IS NULL OR app_user.username ILIKE '%' || $2 || '%')
           AND ($3::text IS NULL OR account.user_id <> $3)
         ORDER BY
           CASE WHEN account.user_id ~ '^U[0-9]+$' THEN substring(account.user_id from 2)::bigint ELSE 0 END DESC,
           account.user_id DESC
         LIMIT $4 OFFSET $5",
    )
    .bind(user_id)
    .bind(username)
    .bind(excluded_user_id)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("资金账户分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(|row| {
            let user_id: String = row
                .try_get("user_id")
                .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?;
            Ok(FinancialAccountSummary {
                user_id,
                available_balance_minor: row
                    .try_get("available_balance_minor")
                    .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?,
                frozen_balance_minor: row
                    .try_get("frozen_balance_minor")
                    .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下按用户 ID 批量读取资金账户。
async fn query_financial_accounts_for_user_ids(
    database: &BusinessDatabase,
    user_ids: &BTreeSet<String>,
) -> ApiResult<Vec<FinancialAccountSummary>> {
    let user_ids = user_ids.iter().cloned().collect::<Vec<_>>();
    let rows = sqlx::query(
        "SELECT user_id, available_balance_minor, frozen_balance_minor
         FROM financial_accounts
         WHERE user_id = ANY($1::text[])",
    )
    .bind(&user_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("资金账户批量数据读取失败".to_string()))?;

    rows.into_iter()
        .map(|row| {
            Ok(FinancialAccountSummary {
                user_id: row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?,
                available_balance_minor: row
                    .try_get("available_balance_minor")
                    .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?,
                frozen_balance_minor: row
                    .try_get("frozen_balance_minor")
                    .map_err(|_| ApiError::Internal("资金账户数据读取失败".to_string()))?,
            })
        })
        .collect()
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

/// 数据库模式下分页读取资金流水，避免财务流水列表先查全量再裁剪。
async fn query_ledger_entry_page(
    database: &BusinessDatabase,
    user_id: Option<&str>,
    excluded_user_id: Option<&str>,
    page: PageRequest,
) -> ApiResult<ListPage<LedgerEntry>> {
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM ledger_entries
         WHERE ($1::text IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR user_id <> $2)",
    )
    .bind(user_id)
    .bind(excluded_user_id)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("资金流水分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("资金流水分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT id, user_id, kind, amount_minor, balance_after_minor, reference_id, description, created_at
         FROM ledger_entries
         WHERE ($1::text IS NULL OR user_id = $1)
           AND ($2::text IS NULL OR user_id <> $2)
         ORDER BY created_at DESC, id DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(user_id)
    .bind(excluded_user_id)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("资金流水分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(ledger_entry_from_row)
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下按流水类型分页读取资金流水，避免返利明细扫描无关业务流水。
async fn query_ledger_entry_kind_page(
    database: &BusinessDatabase,
    user_id: Option<&str>,
    kinds: &[LedgerEntryKind],
    page: PageRequest,
) -> ApiResult<ListPage<LedgerEntry>> {
    let kind_names = ledger_entry_kind_names(kinds)?;
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM ledger_entries
         WHERE ($1::text IS NULL OR user_id = $1)
           AND kind = ANY($2)",
    )
    .bind(user_id)
    .bind(&kind_names)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("资金流水类型分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("资金流水类型分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT id, user_id, kind, amount_minor, balance_after_minor, reference_id, description, created_at
         FROM ledger_entries
         WHERE ($1::text IS NULL OR user_id = $1)
           AND kind = ANY($2)
         ORDER BY created_at DESC, id DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(user_id)
    .bind(&kind_names)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("资金流水类型分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(ledger_entry_from_row)
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下汇总指定用户真实充值本金，供聊天大厅发言门槛等资格判断使用。
///
/// 只读取 `user_withdrawal_turnovers` 中的累计充值字段；该表由资金流水触发器增量维护，
/// 即使运营一键清理 `ledger_entries` 审计列表，历史累计充值也不会丢失。没有累计行时表示当前累计充值为 0。
async fn query_recharge_credit_total_minor(
    database: &BusinessDatabase,
    user_id: &str,
) -> ApiResult<i64> {
    sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(
            (
                SELECT cumulative_recharge_minor
                FROM user_withdrawal_turnovers
                WHERE user_id = $1
            ),
            0
         )::BIGINT",
    )
    .bind(user_id)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户累计充值金额读取失败".to_string()))
}

/// 数据库模式下读取用户提现流水累计摘要，避免提现时扫描全量资金流水。
async fn query_withdrawal_turnover_for_user(
    database: &BusinessDatabase,
    user_id: &str,
) -> ApiResult<WithdrawalTurnoverSummary> {
    let row = sqlx::query(
        "SELECT user_id, cumulative_recharge_minor, required_effective_bet_minor, completed_effective_bet_minor
         FROM user_withdrawal_turnovers
         WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(database.pool())
    .await
    .map_err(|error| {
        tracing::error!(%error, user_id, "用户提现流水累计读取失败");
        ApiError::Internal("用户提现流水累计读取失败".to_string())
    })?;

    let Some(row) = row else {
        return withdrawal_turnover_summary(user_id, 0, 0, 0);
    };

    let user_id: String = row
        .try_get("user_id")
        .map_err(|_| ApiError::Internal("用户提现流水累计读取失败".to_string()))?;
    let cumulative_recharge_minor = row
        .try_get("cumulative_recharge_minor")
        .map_err(|_| ApiError::Internal("用户提现流水累计读取失败".to_string()))?;
    let required_effective_bet_minor = row
        .try_get("required_effective_bet_minor")
        .map_err(|_| ApiError::Internal("用户提现流水累计读取失败".to_string()))?;
    let completed_effective_bet_minor = row
        .try_get("completed_effective_bet_minor")
        .map_err(|_| ApiError::Internal("用户提现流水累计读取失败".to_string()))?;

    withdrawal_turnover_summary(
        &user_id,
        cumulative_recharge_minor,
        required_effective_bet_minor,
        completed_effective_bet_minor,
    )
}

/// 组装用户提现流水累计摘要，并计算仍需完成的有效投注金额。
fn withdrawal_turnover_summary(
    user_id: &str,
    cumulative_recharge_minor: i64,
    required_effective_bet_minor: i64,
    completed_effective_bet_minor: i64,
) -> ApiResult<WithdrawalTurnoverSummary> {
    if cumulative_recharge_minor < 0
        || required_effective_bet_minor < 0
        || completed_effective_bet_minor < 0
    {
        return Err(ApiError::Internal("用户提现流水累计金额无效".to_string()));
    }

    Ok(WithdrawalTurnoverSummary {
        user_id: user_id.to_string(),
        cumulative_recharge_minor,
        required_effective_bet_minor,
        completed_effective_bet_minor,
        remaining_effective_bet_minor: required_effective_bet_minor
            .saturating_sub(completed_effective_bet_minor),
    })
}

/// 根据资金流水类型计算提现流水累计表需要追加的充值和有效投注变化量。
fn withdrawal_turnover_deltas_for_ledger_entry(
    kind: &LedgerEntryKind,
    amount_minor: i64,
) -> Option<(i64, i64)> {
    match kind {
        LedgerEntryKind::RechargeCredit if amount_minor > 0 => Some((amount_minor, 0)),
        LedgerEntryKind::OrderDebit | LedgerEntryKind::GroupBuyDebit if amount_minor < 0 => {
            Some((0, amount_minor.checked_neg()?))
        }
        LedgerEntryKind::OrderRefund | LedgerEntryKind::GroupBuyRefund if amount_minor > 0 => {
            Some((0, amount_minor.checked_neg()?))
        }
        _ => None,
    }
}

/// 数据库模式下按用户集合聚合充值本金，避免代理中心扫描无关用户资金流水。
///
/// 只读取累计表，缺失累计行的用户按 0 处理。
async fn query_recharge_credit_totals_for_user_ids(
    database: &BusinessDatabase,
    user_ids: &BTreeSet<String>,
) -> ApiResult<BTreeMap<String, i64>> {
    let user_ids = user_ids.iter().cloned().collect::<Vec<_>>();
    let rows = sqlx::query(
        "SELECT user_id, cumulative_recharge_minor AS total_minor
         FROM user_withdrawal_turnovers
         WHERE user_id = ANY($1::text[])
           AND cumulative_recharge_minor > 0",
    )
    .bind(&user_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("直属用户累计充值金额读取失败".to_string()))?;

    let mut totals = BTreeMap::new();
    for row in rows {
        let user_id: String = row
            .try_get("user_id")
            .map_err(|_| ApiError::Internal("直属用户累计充值金额读取失败".to_string()))?;
        let total_minor: i64 = row
            .try_get("total_minor")
            .map_err(|_| ApiError::Internal("直属用户累计充值金额读取失败".to_string()))?;
        totals.insert(user_id, total_minor);
    }
    Ok(totals)
}

/// 把流水类型转换为数据库枚举字符串。
fn ledger_entry_kind_names(kinds: &[LedgerEntryKind]) -> ApiResult<Vec<String>> {
    kinds.iter().map(enum_to_string).collect()
}

/// 把资金账户和资金流水运行时快照保存到数据库。
async fn save_finance_store(database: &BusinessDatabase, store: &FinanceStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("资金事务开启失败".to_string()))?;

    save_finance_store_in_transaction(&mut *tx, store).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("资金事务提交失败".to_string()))
}

/// 在外层事务中保存资金账户和资金流水运行时快照，供跨仓储事务复用。
pub(crate) async fn save_finance_store_in_transaction(
    connection: &mut PgConnection,
    store: &FinanceStore,
) -> ApiResult<()> {
    sqlx::query(
        "LOCK TABLE ledger_entries, financial_accounts, finance_runtime IN ACCESS EXCLUSIVE MODE",
    )
    .execute(&mut *connection)
    .await
    .map_err(|error| {
        tracing::error!(%error, "资金表锁定失败");
        ApiError::Internal("资金表锁定失败".to_string())
    })?;

    for table in ["ledger_entries", "financial_accounts", "finance_runtime"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *connection)
            .await
            .map_err(|error| {
                tracing::error!(%error, table, "资金数据清理失败");
                ApiError::Internal("资金数据清理失败".to_string())
            })?;
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
        .execute(&mut *connection)
        .await
        .map_err(|error| {
            tracing::error!(
                %error,
                user_id = account.user_id.as_str(),
                "资金账户数据保存失败"
            );
            ApiError::Internal("资金账户数据保存失败".to_string())
        })?;
    }

    for entry in &store.ledger_entries {
        let kind = enum_to_string(&entry.kind)?;
        sqlx::query(
            "INSERT INTO ledger_entries
             (id, user_id, kind, amount_minor, balance_after_minor, reference_id, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&entry.id)
        .bind(&entry.user_id)
        .bind(&kind)
        .bind(entry.amount_minor)
        .bind(entry.balance_after_minor)
        .bind(&entry.reference_id)
        .bind(&entry.description)
        .bind(&entry.created_at)
        .execute(&mut *connection)
        .await
        .map_err(|error| {
            tracing::error!(
                %error,
                entry_id = entry.id.as_str(),
                user_id = entry.user_id.as_str(),
                kind = kind.as_str(),
                reference_id = entry.reference_id.as_deref().unwrap_or("无"),
                "资金流水数据保存失败"
            );
            ApiError::Internal("资金流水数据保存失败".to_string())
        })?;

        ensure_withdrawal_turnover_event_in_transaction(&mut *connection, entry).await?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("资金流水序号过大".to_string()))?;
    sqlx::query("INSERT INTO finance_runtime (key, value) VALUES ('next_sequence', $1)")
        .bind(next_sequence)
        .execute(&mut *connection)
        .await
        .map_err(|error| {
            tracing::error!(%error, "资金运行数据保存失败");
            ApiError::Internal("资金运行数据保存失败".to_string())
        })?;

    sync_ledger_entry_database_sequence_in_transaction(connection, store.next_sequence).await?;

    Ok(())
}

/// 在资金流水写入后补偿维护用户累计充值和有效投注表。
///
/// 正常情况下数据库触发器会先处理新增流水；这里再次按事件表幂等写入，
/// 可以兼容旧库触发器缺失、迁移前数据异常或 `ON CONFLICT DO UPDATE` 没有触发新增事件的场景。
async fn ensure_withdrawal_turnover_event_in_transaction(
    connection: &mut PgConnection,
    entry: &LedgerEntry,
) -> ApiResult<()> {
    let Some((recharge_delta, effective_delta)) =
        withdrawal_turnover_deltas_for_ledger_entry(&entry.kind, entry.amount_minor)
    else {
        return Ok(());
    };
    let kind = enum_to_string(&entry.kind)?;
    let inserted = sqlx::query(
        "INSERT INTO user_withdrawal_turnover_events
         (ledger_entry_id, user_id, kind, amount_minor, created_at)
         VALUES ($1, $2, $3, $4, now())
         ON CONFLICT (ledger_entry_id) DO NOTHING
         RETURNING ledger_entry_id",
    )
    .bind(&entry.id)
    .bind(&entry.user_id)
    .bind(&kind)
    .bind(entry.amount_minor)
    .fetch_optional(&mut *connection)
    .await
    .map_err(|error| {
        tracing::error!(
            %error,
            entry_id = entry.id.as_str(),
            user_id = entry.user_id.as_str(),
            "用户累计充值事件补偿写入失败"
        );
        ApiError::Internal("用户累计充值事件补偿写入失败".to_string())
    })?;

    if inserted.is_none() {
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO user_withdrawal_turnovers (
            user_id,
            cumulative_recharge_minor,
            required_effective_bet_minor,
            completed_effective_bet_minor,
            created_at,
            updated_at
         )
         VALUES ($1, $2, $2, GREATEST(0::BIGINT, $3), now(), now())
         ON CONFLICT (user_id) DO UPDATE SET
            cumulative_recharge_minor = user_withdrawal_turnovers.cumulative_recharge_minor + EXCLUDED.cumulative_recharge_minor,
            required_effective_bet_minor = user_withdrawal_turnovers.required_effective_bet_minor + EXCLUDED.required_effective_bet_minor,
            completed_effective_bet_minor = GREATEST(
                0,
                user_withdrawal_turnovers.completed_effective_bet_minor + $3
            ),
            updated_at = now()",
    )
    .bind(&entry.user_id)
    .bind(recharge_delta)
    .bind(effective_delta)
    .execute(&mut *connection)
    .await
    .map_err(|error| {
        tracing::error!(
            %error,
            entry_id = entry.id.as_str(),
            user_id = entry.user_id.as_str(),
            "用户累计充值金额补偿更新失败"
        );
        ApiError::Internal("用户累计充值金额补偿更新失败".to_string())
    })?;

    Ok(())
}

/// 在外层事务中按前后快照差异保存资金数据，避免派奖或扣款时重写全量资金流水。
pub(crate) async fn save_finance_store_incremental_in_transaction(
    connection: &mut PgConnection,
    previous: &FinanceStore,
    store: &mut FinanceStore,
) -> ApiResult<LedgerEntryIdRemap> {
    let id_remap = assign_database_ledger_entry_ids(connection, previous, store).await?;

    for user_id in previous
        .accounts
        .keys()
        .filter(|user_id| !store.accounts.contains_key(*user_id))
    {
        sqlx::query("DELETE FROM financial_accounts WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *connection)
            .await
            .map_err(|error| {
                tracing::error!(%error, user_id = user_id.as_str(), "资金账户数据删除失败");
                ApiError::Internal("资金账户数据删除失败".to_string())
            })?;
    }

    for (user_id, account) in &store.accounts {
        let previous_account = previous.accounts.get(user_id);
        let delta_available = if let Some(prev) = previous_account {
            account.available_balance_minor - prev.available_balance_minor
        } else {
            account.available_balance_minor
        };
        let delta_frozen = if let Some(prev) = previous_account {
            account.frozen_balance_minor - prev.frozen_balance_minor
        } else {
            account.frozen_balance_minor
        };
        if delta_available == 0 && delta_frozen == 0 {
            continue;
        }
        sqlx::query(
            "INSERT INTO financial_accounts
             (user_id, available_balance_minor, frozen_balance_minor)
             VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET
                available_balance_minor = financial_accounts.available_balance_minor + EXCLUDED.available_balance_minor,
                frozen_balance_minor = financial_accounts.frozen_balance_minor + EXCLUDED.frozen_balance_minor,
                updated_at = now()",
        )
        .bind(&account.user_id)
        .bind(delta_available)
        .bind(delta_frozen)
        .execute(&mut *connection)
        .await
        .map_err(|error| {
            tracing::error!(
                %error,
                user_id = account.user_id.as_str(),
                delta_available,
                delta_frozen,
                "资金账户增量保存失败"
            );
            ApiError::Internal("资金账户增量保存失败".to_string())
        })?;
    }

    let previous_entries = previous
        .ledger_entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();

    for entry in &store.ledger_entries {
        if previous_entries
            .get(entry.id.as_str())
            .map(|previous| *previous == entry)
            .unwrap_or_default()
        {
            continue;
        }
        let kind = enum_to_string(&entry.kind)?;
        sqlx::query(
            "INSERT INTO ledger_entries
             (id, user_id, kind, amount_minor, balance_after_minor, reference_id, description, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&entry.id)
        .bind(&entry.user_id)
        .bind(&kind)
        .bind(entry.amount_minor)
        .bind(entry.balance_after_minor)
        .bind(&entry.reference_id)
        .bind(&entry.description)
        .bind(&entry.created_at)
        .execute(&mut *connection)
        .await
        .map_err(|error| {
            tracing::error!(
                %error,
                entry_id = entry.id.as_str(),
                user_id = entry.user_id.as_str(),
                kind = kind.as_str(),
                reference_id = entry.reference_id.as_deref().unwrap_or("无"),
                "资金流水数据保存失败"
            );
            ApiError::Internal("资金流水数据保存失败".to_string())
        })?;

        ensure_withdrawal_turnover_event_in_transaction(&mut *connection, entry).await?;
    }

    let next_sequence = i64::try_from(store.next_sequence)
        .map_err(|_| ApiError::Internal("资金流水序号过大".to_string()))?;
    sqlx::query(
        "INSERT INTO finance_runtime (key, value) VALUES ('next_sequence', $1)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(next_sequence)
    .execute(&mut *connection)
    .await
    .map_err(|error| {
        tracing::error!(%error, "资金运行数据保存失败");
        ApiError::Internal("资金运行数据保存失败".to_string())
    })?;

    Ok(id_remap)
}

/// 在启动加载时把 PostgreSQL 流水序列校准到当前快照最大值，修复历史库序列落后问题。
async fn sync_ledger_entry_database_sequence(
    database: &BusinessDatabase,
    next_sequence: u64,
) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("资金流水数据库序列同步事务开启失败".to_string()))?;
    sync_ledger_entry_database_sequence_in_transaction(&mut *tx, next_sequence).await?;
    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("资金流水数据库序列同步事务提交失败".to_string()))
}

/// 把 PostgreSQL 流水序列至少推进到指定值，绝不向后回退。
async fn sync_ledger_entry_database_sequence_in_transaction(
    connection: &mut PgConnection,
    next_sequence: u64,
) -> ApiResult<()> {
    let sequence = i64::try_from(next_sequence)
        .map_err(|_| ApiError::Internal("资金流水序号过大".to_string()))?;
    sqlx::query(
        "SELECT setval(
            'ledger_entry_id_sequence',
            GREATEST($1::BIGINT, COALESCE((SELECT last_value FROM ledger_entry_id_sequence), 0)),
            true
        )",
    )
    .bind(sequence)
    .execute(&mut *connection)
    .await
    .map_err(|error| {
        tracing::error!(%error, "资金流水数据库序列同步失败");
        ApiError::Internal("资金流水数据库序列同步失败".to_string())
    })?;
    Ok(())
}

/// 为本次新增的资金流水分配 PostgreSQL 序列 ID，避免快照临时序号和普通下注序列撞号。
async fn assign_database_ledger_entry_ids(
    connection: &mut PgConnection,
    previous: &FinanceStore,
    store: &mut FinanceStore,
) -> ApiResult<LedgerEntryIdRemap> {
    let previous_entry_ids = previous
        .ledger_entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut id_remap = LedgerEntryIdRemap::default();
    let mut max_sequence = store.next_sequence;

    for entry in &mut store.ledger_entries {
        if previous_entry_ids.contains(entry.id.as_str()) {
            max_sequence = max_sequence.max(sequence_from_ledger_entry_id(&entry.id).unwrap_or(0));
            continue;
        }

        let old_id = entry.id.clone();
        let sequence = next_ledger_entry_sequence_in_transaction(connection).await?;
        let new_id = ledger_entry_id_from_sequence(sequence);
        entry.id = new_id.clone();
        max_sequence = max_sequence.max(sequence);
        id_remap.insert(old_id, new_id);
    }

    store.next_sequence = store.next_sequence.max(max_sequence);
    Ok(id_remap)
}

/// 从 PostgreSQL sequence 取资金流水序号，所有数据库模式新增流水都必须走这里。
async fn next_ledger_entry_sequence_in_transaction(
    connection: &mut PgConnection,
) -> ApiResult<u64> {
    let sequence = sqlx::query_scalar::<_, i64>("SELECT nextval('ledger_entry_id_sequence')")
        .fetch_one(&mut *connection)
        .await
        .map_err(|error| {
            tracing::error!(%error, "资金流水数据库序列取号失败");
            ApiError::Internal("资金流水数据库序列取号失败".to_string())
        })?;
    u64::try_from(sequence).map_err(|_| ApiError::Internal("资金流水序号过大".to_string()))
}

/// 把数字序号格式化为统一的资金流水 ID。
fn ledger_entry_id_from_sequence(sequence: u64) -> String {
    format!("L{sequence:012}")
}

/// 资金账户和资金流水运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl FinanceStore {
    /// 构建并返回种子数据。
    fn seeded() -> Self {
        let mut store = Self::default();
        store.seed_account("U10001", 12_000, 2_000);
        store.seed_account("U10002", 50_000, 0);
        store.seed_account("U10003", 100_000, 0);
        store.seed_account("U10004", 0, 0);
        store.seed_account("U90001", 520_000, 0);
        store.seed_account("X90002", 520_000, 0);
        store.seed_account("X90003", 520_000, 0);
        store.seed_account("X90004", 520_000, 0);
        store.seed_account("X90005", 520_000, 0);
        store.seed_account("X90006", 520_000, 0);
        store.seed_account("X90007", 520_000, 0);
        store.seed_account("X90008", 520_000, 0);
        store.seed_account("X90009", 520_000, 0);
        store.seed_account("X90010", 520_000, 0);
        store
    }

    /// 构造内置用户的初始化资金账户。
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

    /// 聚合资金账户和流水生成财务概览。
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

    /// 返回资金账户列表。
    fn accounts(&self) -> Vec<FinancialAccountSummary> {
        self.accounts.values().cloned().collect()
    }

    /// 返回资金流水列表。
    fn ledger_entries(&self) -> Vec<LedgerEntry> {
        self.ledger_entries.iter().rev().cloned().collect()
    }

    /// 清除资金流水审计列表，保留余额和下一流水序号，避免后续流水 ID 重复。
    fn clear_ledger_entries(&mut self) -> usize {
        let deleted_count = self.ledger_entries.len();
        self.ledger_entries.clear();
        deleted_count
    }

    /// 按用户筛选资金流水。
    fn ledger_entries_for_user(&self, user_id: &str) -> Vec<LedgerEntry> {
        self.ledger_entries
            .iter()
            .filter(|entry| entry.user_id == user_id)
            .cloned()
            .rev()
            .collect()
    }

    /// 统计指定用户正向充值本金流水，赠送彩金和代理返利都不计入充值门槛。
    fn recharge_credit_total_minor_for_user(&self, user_id: &str) -> ApiResult<i64> {
        self.ledger_entries
            .iter()
            .filter(|entry| {
                entry.user_id == user_id
                    && matches!(entry.kind, LedgerEntryKind::RechargeCredit)
                    && entry.amount_minor > 0
            })
            .try_fold(0_i64, |total, entry| {
                total
                    .checked_add(entry.amount_minor)
                    .ok_or_else(|| ApiError::Internal("用户累计充值金额溢出".to_string()))
            })
    }

    /// 内存模式下从当前资金流水临时计算提现流水要求摘要。
    fn withdrawal_turnover_for_user(&self, user_id: &str) -> ApiResult<WithdrawalTurnoverSummary> {
        let mut cumulative_recharge_minor = 0_i64;
        let mut completed_effective_bet_minor = 0_i64;
        for entry in self
            .ledger_entries
            .iter()
            .filter(|entry| entry.user_id == user_id)
        {
            match entry.kind {
                LedgerEntryKind::RechargeCredit if entry.amount_minor > 0 => {
                    cumulative_recharge_minor = cumulative_recharge_minor
                        .checked_add(entry.amount_minor)
                        .ok_or_else(|| ApiError::Internal("用户累计充值金额溢出".to_string()))?;
                }
                LedgerEntryKind::OrderDebit | LedgerEntryKind::GroupBuyDebit
                    if entry.amount_minor < 0 =>
                {
                    let amount_minor = entry
                        .amount_minor
                        .checked_neg()
                        .ok_or_else(|| ApiError::Internal("用户有效投注金额溢出".to_string()))?;
                    completed_effective_bet_minor = completed_effective_bet_minor
                        .checked_add(amount_minor)
                        .ok_or_else(|| ApiError::Internal("用户有效投注金额溢出".to_string()))?;
                }
                LedgerEntryKind::OrderRefund | LedgerEntryKind::GroupBuyRefund
                    if entry.amount_minor > 0 =>
                {
                    completed_effective_bet_minor =
                        completed_effective_bet_minor.saturating_sub(entry.amount_minor);
                }
                _ => {}
            }
        }

        withdrawal_turnover_summary(
            user_id,
            cumulative_recharge_minor,
            cumulative_recharge_minor,
            completed_effective_bet_minor,
        )
    }

    /// 按用户集合聚合充值本金，内存模式下只扫描一次资金流水。
    fn recharge_credit_totals_for_user_ids(
        &self,
        user_ids: &BTreeSet<String>,
    ) -> ApiResult<BTreeMap<String, i64>> {
        let mut totals = BTreeMap::new();
        for entry in self.ledger_entries.iter().filter(|entry| {
            user_ids.contains(&entry.user_id)
                && matches!(entry.kind, LedgerEntryKind::RechargeCredit)
                && entry.amount_minor > 0
        }) {
            let current = totals.entry(entry.user_id.clone()).or_insert(0_i64);
            *current = current
                .checked_add(entry.amount_minor)
                .ok_or_else(|| ApiError::Internal("直属用户累计充值金额溢出".to_string()))?;
        }
        Ok(totals)
    }

    /// 校验用户可用余额是否足够扣款。
    pub(crate) fn ensure_available(&self, user_id: &str, amount_minor: i64) -> ApiResult<()> {
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

    /// 读取资金账户；不存在时按用户信息创建初始账户。
    pub(crate) fn account_or_create(
        &mut self,
        user_id: &str,
    ) -> ApiResult<FinancialAccountSummary> {
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

    /// 校验订单退款前账户和流水状态是否允许退款。
    pub(crate) fn ensure_order_can_refund(&self, order: &OrderDetail) -> ApiResult<()> {
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

    /// 执行后台手动调账并记录资金流水。
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

    /// 投注下单时扣减用户可用余额并写入流水。
    pub(crate) fn debit_order(&mut self, order: &OrderDetail) -> ApiResult<LedgerEntry> {
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

    /// 投注取消或流单时退回用户投注本金。
    pub(crate) fn refund_order(&mut self, order: &OrderDetail) -> ApiResult<LedgerEntry> {
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

    #[cfg(test)]
    /// 结算中奖订单并按派奖金额入账。
    fn credit_settlement(&mut self, settlement: &SettlementRun) -> ApiResult<Vec<LedgerEntry>> {
        let mut entries = Vec::new();

        for order in &settlement.orders {
            if !order.is_winning || order.payout_minor <= 0 {
                continue;
            }

            entries.push(self.credit_order_payout(settlement, order)?);
        }

        Ok(entries)
    }

    /// 结算派奖时识别合买总单，并把奖金拆给参与人。
    pub(crate) fn credit_settlement_with_group_buys(
        &mut self,
        settlement: &SettlementRun,
        group_buy_plans: &[GroupBuyPlan],
        skip_group_buy_order_ids: &BTreeSet<String>,
    ) -> ApiResult<Vec<LedgerEntry>> {
        let mut entries = Vec::new();

        for order in &settlement.orders {
            if !order.is_winning || order.payout_minor <= 0 {
                continue;
            }

            if skip_group_buy_order_ids.contains(&order.order_id) {
                tracing::warn!(
                    order_id = %order.order_id,
                    settlement_id = %settlement.id,
                    "合买订单缺少对应合买计划，已跳过派奖入账"
                );
                continue;
            }

            if let Some(plan) = group_buy_plans
                .iter()
                .find(|plan| plan.order_id.as_deref() == Some(order.order_id.as_str()))
            {
                entries.extend(self.credit_group_buy_payout(settlement, order, plan)?);
            } else {
                entries.push(self.credit_order_payout(settlement, order)?);
            }
        }

        Ok(entries)
    }

    /// 给普通投注订单派奖。
    fn credit_order_payout(
        &mut self,
        settlement: &SettlementRun,
        order: &OrderSettlement,
    ) -> ApiResult<LedgerEntry> {
        let reference_id = format!("{}:{}", settlement.id, order.order_id);
        if let Some(entry) = self.reference_entry(&LedgerEntryKind::PayoutCredit, &reference_id) {
            return Ok(entry);
        }

        self.apply_available_delta(
            &order.user_id,
            LedgerEntryKind::PayoutCredit,
            order.payout_minor,
            Some(reference_id),
            format!("中奖派奖：{} {}", settlement.lottery_name, settlement.issue),
        )
    }

    /// 给合买参与人按出资比例分配派奖金额。
    fn credit_group_buy_payout(
        &mut self,
        settlement: &SettlementRun,
        order: &OrderSettlement,
        plan: &GroupBuyPlan,
    ) -> ApiResult<Vec<LedgerEntry>> {
        if plan.total_amount_minor <= 0 {
            return Err(ApiError::BadRequest("合买总金额无效".to_string()));
        }

        let mut entries = Vec::new();
        let mut remaining_payout = order.payout_minor;
        let participants = plan
            .participants
            .iter()
            .filter(|participant| participant.amount_minor > 0)
            .collect::<Vec<_>>();
        let participant_count = participants.len();
        if participant_count == 0 {
            return Err(ApiError::BadRequest("合买计划没有可派奖参与人".to_string()));
        }

        for (index, participant) in participants.into_iter().enumerate() {
            let payout_minor = if index + 1 == participant_count {
                remaining_payout
            } else {
                proportional_amount(
                    order.payout_minor,
                    participant.amount_minor,
                    plan.total_amount_minor,
                )?
            };
            remaining_payout = remaining_payout
                .checked_sub(payout_minor)
                .ok_or_else(|| ApiError::BadRequest("合买派奖金额过大".to_string()))?;
            if payout_minor <= 0 {
                continue;
            }

            let reference_id = format!("{}:{}:{}", settlement.id, order.order_id, participant.id);
            if let Some(entry) = self.reference_entry(&LedgerEntryKind::PayoutCredit, &reference_id)
            {
                entries.push(entry);
                continue;
            }

            let entry = self.apply_available_delta(
                &participant.user_id,
                LedgerEntryKind::PayoutCredit,
                payout_minor,
                Some(reference_id),
                format!(
                    "合买中奖分账：{} {} {}",
                    settlement.lottery_name, settlement.issue, plan.id
                ),
            )?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// 处理充值入账，避免同一个充值订单重复生成流水。
    pub(crate) fn credit_recharge(
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

    /// 处理充值赠送活动入账，引用 ID 绑定充值单，避免重复回调导致重复赠送。
    pub(crate) fn credit_recharge_bonus(
        &mut self,
        user_id: &str,
        bonus_amount_minor: i64,
        recharge_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let user_id = user_id.trim();
        let recharge_order_id = recharge_order_id.trim();
        if bonus_amount_minor <= 0 {
            return Err(ApiError::BadRequest("充值赠送金额必须大于 0".to_string()));
        }
        if recharge_order_id.is_empty() {
            return Err(ApiError::BadRequest("充值订单 ID 不能为空".to_string()));
        }

        let reference_id = recharge_bonus_reference_id(recharge_order_id);
        if let Some(entry) =
            self.reference_entry(&LedgerEntryKind::RechargeBonusCredit, &reference_id)
        {
            return Ok(entry);
        }

        self.account_or_create(user_id)?;
        self.apply_available_delta(
            user_id,
            LedgerEntryKind::RechargeBonusCredit,
            bonus_amount_minor,
            Some(reference_id),
            format!("充值活动赠送彩金：订单 {recharge_order_id}"),
        )
    }

    /// 处理上级代理充值返利，引用 ID 只绑定充值单，避免代理关系变更后重复发放。
    pub(crate) fn credit_recharge_rebate(
        &mut self,
        agent_user_id: &str,
        invitee_user_id: &str,
        rebate_amount_minor: i64,
        recharge_order_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let agent_user_id = agent_user_id.trim();
        let invitee_user_id = invitee_user_id.trim();
        let recharge_order_id = recharge_order_id.trim();
        if agent_user_id.is_empty() {
            return Err(ApiError::BadRequest("代理用户 ID 不能为空".to_string()));
        }
        if invitee_user_id.is_empty() {
            return Err(ApiError::BadRequest("下级用户 ID 不能为空".to_string()));
        }
        if agent_user_id == invitee_user_id {
            return Err(ApiError::BadRequest(
                "不能给同一用户发放邀请返利".to_string(),
            ));
        }
        if rebate_amount_minor <= 0 {
            return Err(ApiError::BadRequest("返利金额必须大于 0".to_string()));
        }
        if recharge_order_id.is_empty() {
            return Err(ApiError::BadRequest("充值订单 ID 不能为空".to_string()));
        }

        let reference_id = recharge_rebate_reference_id(recharge_order_id);
        if let Some(entry) =
            self.reference_entry(&LedgerEntryKind::RechargeRebateCredit, &reference_id)
        {
            return Ok(entry);
        }

        self.account_or_create(agent_user_id)?;
        self.apply_available_delta(
            agent_user_id,
            LedgerEntryKind::RechargeRebateCredit,
            rebate_amount_minor,
            Some(reference_id),
            format!("下级用户充值返利：订单 {recharge_order_id}，下级 {invitee_user_id}"),
        )
    }

    /// 处理代理返利提现，直接扣减可用余额并保留独立流水便于统计已提现返利。
    pub(crate) fn withdraw_agent_rebate(
        &mut self,
        agent_user_id: &str,
        amount_minor: i64,
        description: &str,
    ) -> ApiResult<LedgerEntry> {
        let agent_user_id = agent_user_id.trim();
        let description = description.trim();
        if agent_user_id.is_empty() {
            return Err(ApiError::BadRequest("代理用户 ID 不能为空".to_string()));
        }
        if amount_minor <= 0 {
            return Err(ApiError::BadRequest("返利提现金额必须大于 0".to_string()));
        }
        if description.is_empty() {
            return Err(ApiError::BadRequest("返利提现说明不能为空".to_string()));
        }
        self.ensure_available(agent_user_id, amount_minor)?;

        self.apply_available_delta(
            agent_user_id,
            LedgerEntryKind::AgentRebateWithdrawal,
            amount_minor
                .checked_neg()
                .ok_or_else(|| ApiError::BadRequest("返利提现金额过大".to_string()))?,
            None,
            description.to_string(),
        )
    }

    /// 提交提现申请时把可用余额转入冻结余额，并生成提现冻结流水。
    pub(crate) fn freeze_withdrawal(
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
    pub(crate) fn approve_withdrawal(
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
    pub(crate) fn reject_withdrawal(
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
    pub(crate) fn debit_group_buy(
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

    /// 合买取消或流单时按参与记录退还认购金额。
    pub(crate) fn refund_group_buy_plan(
        &mut self,
        plan: &GroupBuyPlan,
        reason: &str,
    ) -> ApiResult<Vec<LedgerEntry>> {
        let mut entries = Vec::new();
        let reason = reason.trim();
        for participant in &plan.participants {
            if participant.amount_minor <= 0 {
                continue;
            }
            entries.push(self.refund_group_buy_participant(
                plan,
                participant,
                if reason.is_empty() {
                    "合买退款"
                } else {
                    reason
                },
            )?);
        }
        Ok(entries)
    }

    /// 退还单条合买参与记录，按参与记录 ID 保持幂等。
    fn refund_group_buy_participant(
        &mut self,
        plan: &GroupBuyPlan,
        participant: &GroupBuyParticipant,
        reason: &str,
    ) -> ApiResult<LedgerEntry> {
        if let Some(entry) = self.reference_entry(&LedgerEntryKind::GroupBuyRefund, &participant.id)
        {
            return Ok(entry);
        }
        self.apply_available_delta(
            &participant.user_id,
            LedgerEntryKind::GroupBuyRefund,
            participant.amount_minor,
            Some(participant.id.clone()),
            format!("合买退款：{} {reason}", plan.id),
        )
    }

    /// 聊天大厅发送红包时扣减发送人的可用余额，并按红包 ID 保持幂等。
    pub(crate) fn debit_chat_red_packet(
        &mut self,
        user_id: &str,
        amount_minor: i64,
        red_packet_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let user_id = user_id.trim();
        let red_packet_id = red_packet_id.trim();
        if red_packet_id.is_empty() {
            return Err(ApiError::BadRequest("红包编号不能为空".to_string()));
        }
        if amount_minor <= 0 {
            return Err(ApiError::BadRequest("红包金额必须大于 0".to_string()));
        }
        if let Some(entry) = self.reference_entry(&LedgerEntryKind::RedPacketDebit, red_packet_id) {
            return Ok(entry);
        }
        self.ensure_available(user_id, amount_minor)?;

        self.apply_available_delta(
            user_id,
            LedgerEntryKind::RedPacketDebit,
            amount_minor
                .checked_neg()
                .ok_or_else(|| ApiError::BadRequest("红包金额过大".to_string()))?,
            Some(red_packet_id.to_string()),
            format!("聊天大厅发送红包扣款：{red_packet_id}"),
        )
    }

    /// 聊天大厅领取红包时给用户入账，并按领取记录 ID 保持幂等。
    pub(crate) fn credit_chat_red_packet(
        &mut self,
        user_id: &str,
        amount_minor: i64,
        claim_id: &str,
        red_packet_id: &str,
    ) -> ApiResult<LedgerEntry> {
        let user_id = user_id.trim();
        let claim_id = claim_id.trim();
        let red_packet_id = red_packet_id.trim();
        if claim_id.is_empty() {
            return Err(ApiError::BadRequest("红包领取记录不能为空".to_string()));
        }
        if amount_minor <= 0 {
            return Err(ApiError::BadRequest("红包领取金额必须大于 0".to_string()));
        }
        if let Some(entry) = self.reference_entry(&LedgerEntryKind::RedPacketCredit, claim_id) {
            return Ok(entry);
        }
        self.account_or_create(user_id)?;

        self.apply_available_delta(
            user_id,
            LedgerEntryKind::RedPacketCredit,
            amount_minor,
            Some(claim_id.to_string()),
            format!("聊天大厅领取红包入账：{red_packet_id}"),
        )
    }

    /// 按用户 ID 读取资金账户。
    #[cfg(test)]
    fn account(&self, user_id: &str) -> ApiResult<&FinancialAccountSummary> {
        let user_id = user_id.trim();
        self.accounts
            .get(user_id)
            .ok_or_else(|| ApiError::NotFound(format!("financial account `{user_id}` not found")))
    }

    /// 调整可用余额并保证余额不为负。
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

    /// 按流水类型和关联单号查找幂等流水。
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

/// 按比例计算金额，向下取整，最后一名参与人由调用方承接余数。
fn proportional_amount(total_minor: i64, part_minor: i64, base_minor: i64) -> ApiResult<i64> {
    if total_minor < 0 || part_minor < 0 || base_minor <= 0 {
        return Err(ApiError::BadRequest("合买派奖比例金额无效".to_string()));
    }
    total_minor
        .checked_mul(part_minor)
        .map(|amount| amount / base_minor)
        .ok_or_else(|| ApiError::BadRequest("合买派奖金额过大".to_string()))
}

/// 生成充值返利流水的业务引用 ID，用于支付回调、后台确认重复触发时识别同一笔返利。
fn recharge_rebate_reference_id(recharge_order_id: &str) -> String {
    format!("recharge-rebate:{recharge_order_id}")
}

/// 生成充值赠送流水的业务引用 ID，用于重复回调时识别同一笔活动赠送彩金。
fn recharge_bonus_reference_id(recharge_order_id: &str) -> String {
    format!("recharge-bonus:{recharge_order_id}")
}

/// 从已有资金流水编号恢复最大序号，避免运行时序号落后导致新流水主键重复。
fn next_sequence_from_ledger_entries(entries: &[LedgerEntry]) -> u64 {
    entries
        .iter()
        .filter_map(|entry| sequence_from_ledger_entry_id(&entry.id))
        .max()
        .unwrap_or_default()
}

/// 解析 `L000000000001` 这类资金流水编号中的数字部分。
fn sequence_from_ledger_entry_id(id: &str) -> Option<u64> {
    id.strip_prefix('L')?.parse().ok()
}

/// 生成当前本地时间字符串。
fn current_timestamp_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::{
        domain::{
            finance::{LedgerEntry, LedgerEntryKind, ManualBalanceAdjustmentRequest},
            group_buy::{GroupBuyParticipant, GroupBuyPlan, GroupBuyPlanStatus},
            lottery::LotteryNumberType,
            order::OrderSource,
            order::{OrderDetail, OrderStatus},
            play::{PlayRuleCode, PlaySelection},
            settlement::{OrderSettlement, SettlementRun},
        },
        services::{
            finance::{FinanceRepository, FinanceStore},
            pagination::PageRequest,
        },
    };

    #[tokio::test]
    /// 资金账户分页在内存模式下也会按用户名先过滤再分页。
    async fn repository_account_page_filters_by_username_before_pagination() {
        let repository = FinanceRepository::memory_seeded();
        let usernames = BTreeMap::from([
            ("U10001".to_string(), "alice".to_string()),
            ("U10002".to_string(), "bob".to_string()),
            ("U10003".to_string(), "carol".to_string()),
            ("U10004".to_string(), "alice_vip".to_string()),
            ("U90001".to_string(), "agent_alpha".to_string()),
        ]);

        let page = repository
            .account_page(
                None,
                Some("alice"),
                &usernames,
                None,
                PageRequest::new(Some(1), Some(10)),
            )
            .await
            .expect("account page can filter by username");

        assert_eq!(page.total_count, 2);
        assert_eq!(
            page.items
                .iter()
                .map(|account| account.user_id.as_str())
                .collect::<Vec<_>>(),
            vec!["U10004", "U10001"]
        );
    }

    #[test]
    /// 验证下单扣款会生成对应资金流水。
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
    /// 验证余额不足时拒绝投注扣款。
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
    /// 历史数据库如果运行序号落后，启动加载必须按已有流水编号恢复最大序号。
    fn finance_sequence_recovers_from_existing_ledger_ids() {
        let entries = vec![
            ledger_entry("L000000000009"),
            ledger_entry("legacy-entry"),
            ledger_entry("L000000000012"),
        ];

        assert_eq!(super::next_sequence_from_ledger_entries(&entries), 12);
        assert_eq!(
            super::sequence_from_ledger_entry_id("L000000000013"),
            Some(13)
        );
        assert_eq!(super::sequence_from_ledger_entry_id("BAD0001"), None);
    }

    #[test]
    /// 验证订单退款具备幂等保护。
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
    /// 验证派奖结算会增加用户余额。
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
    /// 合买总单派奖会按参与金额拆给每个参与用户。
    fn store_credits_group_buy_settlement_by_participant_share() {
        let mut store = FinanceStore::seeded();
        let settlement = settlement_run("S000000000001", "U90001", 3_000);
        let plan = group_buy_plan_with_order("G202606050001", "O000000000001");

        let entries = store
            .credit_settlement_with_group_buys(&settlement, &[plan], &BTreeSet::new())
            .expect("group buy payout can be credited");
        let agent = store.account("U90001").expect("agent account exists");
        let user = store.account("U10001").expect("user account exists");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, LedgerEntryKind::PayoutCredit);
        assert_eq!(entries[0].amount_minor, 1_000);
        assert_eq!(entries[1].amount_minor, 2_000);
        assert!(entries
            .iter()
            .all(|entry| entry.reference_id.as_deref() != Some("S000000000001:O000000000001")));
        assert_eq!(agent.available_balance_minor, 521_000);
        assert_eq!(user.available_balance_minor, 14_000);
    }

    #[test]
    /// 合买退款按参与记录退还认购金额，并按参与记录保持幂等。
    fn store_refunds_group_buy_plan_once() {
        let mut store = FinanceStore::seeded();
        let plan = group_buy_plan_with_order("G202606050001", "O000000000001");
        store
            .debit_group_buy("U90001", 1_000, "G202606050001-P001", "G202606050001")
            .expect("initiator debit can be applied");
        store
            .debit_group_buy("U10001", 2_000, "G202606050001-P002", "G202606050001")
            .expect("participant debit can be applied");

        let entries = store
            .refund_group_buy_plan(&plan, "流单退款")
            .expect("group buy plan can be refunded");
        let repeated = store
            .refund_group_buy_plan(&plan, "流单退款")
            .expect("group buy refund is idempotent");
        let agent = store.account("U90001").expect("agent account exists");
        let user = store.account("U10001").expect("user account exists");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].kind, LedgerEntryKind::GroupBuyRefund);
        assert_eq!(entries[0].id, repeated[0].id);
        assert_eq!(agent.available_balance_minor, 520_000);
        assert_eq!(user.available_balance_minor, 12_000);
    }

    #[test]
    /// 验证后台手动调账会更新余额和流水。
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
    /// 清除资金流水只清理审计列表，不回滚余额，也不重置下一流水序号。
    fn store_clears_ledger_entries_without_changing_balance_or_sequence() {
        let mut store = FinanceStore::seeded();

        let first_entry = store
            .manual_adjust(ManualBalanceAdjustmentRequest {
                user_id: "U10001".to_string(),
                amount_minor: 1_000,
                description: "后台补款".to_string(),
            })
            .expect("manual adjustment can be applied");
        let balance_before_clear = store
            .account("U10001")
            .expect("account exists")
            .available_balance_minor;
        let sequence_before_clear = store.next_sequence;

        let deleted_count = store.clear_ledger_entries();
        let account_after_clear = store.account("U10001").expect("account exists");

        assert_eq!(first_entry.id, "L000000000001");
        assert_eq!(deleted_count, 1);
        assert!(store.ledger_entries().is_empty());
        assert_eq!(
            account_after_clear.available_balance_minor,
            balance_before_clear
        );
        assert_eq!(store.next_sequence, sequence_before_clear);

        let second_entry = store
            .manual_adjust(ManualBalanceAdjustmentRequest {
                user_id: "U10001".to_string(),
                amount_minor: -500,
                description: "后台扣款".to_string(),
            })
            .expect("manual adjustment can be applied after clearing ledger entries");

        assert_eq!(second_entry.id, "L000000000002");
        assert_eq!(store.ledger_entries().len(), 1);
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
    /// 批量汇总充值本金时只统计目标用户的正向充值入账。
    fn store_sums_recharge_credits_for_user_set() {
        let mut store = FinanceStore::seeded();
        store
            .credit_recharge("U10001", 1_500, "R000000000001")
            .expect("first recharge can be credited");
        store
            .credit_recharge("U10001", 2_000, "R000000000002")
            .expect("second recharge can be credited");
        store
            .credit_recharge("U10002", 800, "R000000000003")
            .expect("other user recharge can be credited");
        store
            .credit_recharge_bonus("U10001", 500, "R000000000001")
            .expect("bonus is not recharge principal");
        store
            .credit_recharge_rebate("U90001", "U10001", 350, "R000000000001")
            .expect("rebate is not recharge principal");

        let user_ids = ["U10001".to_string(), "U10003".to_string()]
            .into_iter()
            .collect();
        let totals = store
            .recharge_credit_totals_for_user_ids(&user_ids)
            .expect("recharge totals can be calculated");

        assert_eq!(totals.get("U10001").copied(), Some(3_500));
        assert!(!totals.contains_key("U10002"));
        assert!(!totals.contains_key("U10003"));
    }

    #[test]
    /// 验证数据库累计表补偿逻辑只把真实充值本金和有效投注写入资格累计。
    fn withdrawal_turnover_deltas_ignore_bonus_rebate_and_adjustment() {
        assert_eq!(
            super::withdrawal_turnover_deltas_for_ledger_entry(
                &LedgerEntryKind::RechargeCredit,
                50_000
            ),
            Some((50_000, 0))
        );
        assert_eq!(
            super::withdrawal_turnover_deltas_for_ledger_entry(
                &LedgerEntryKind::OrderDebit,
                -2_000
            ),
            Some((0, 2_000))
        );
        assert_eq!(
            super::withdrawal_turnover_deltas_for_ledger_entry(
                &LedgerEntryKind::GroupBuyDebit,
                -3_000
            ),
            Some((0, 3_000))
        );
        assert_eq!(
            super::withdrawal_turnover_deltas_for_ledger_entry(
                &LedgerEntryKind::OrderRefund,
                1_000
            ),
            Some((0, -1_000))
        );
        assert_eq!(
            super::withdrawal_turnover_deltas_for_ledger_entry(
                &LedgerEntryKind::RechargeBonusCredit,
                500
            ),
            None
        );
        assert_eq!(
            super::withdrawal_turnover_deltas_for_ledger_entry(
                &LedgerEntryKind::RechargeRebateCredit,
                500
            ),
            None
        );
        assert_eq!(
            super::withdrawal_turnover_deltas_for_ledger_entry(
                &LedgerEntryKind::ManualAdjustment,
                50_000
            ),
            None
        );
    }

    #[test]
    /// 提现流水累计只统计真实充值本金和有效投注，充值赠送不计入门槛，退款会扣回有效投注。
    fn store_calculates_withdrawal_turnover_from_ledger_entries() {
        let mut store = FinanceStore::seeded();
        store
            .credit_recharge("U10001", 10_000, "R000000000001")
            .expect("recharge can be credited");
        store
            .credit_recharge_bonus("U10001", 500, "R000000000001")
            .expect("recharge bonus can be credited");
        let order = order_detail("O000000000001", "U10001", 3_000, 0);
        store.debit_order(&order).expect("order can be debited");
        store.refund_order(&order).expect("order can be refunded");
        store
            .debit_group_buy("U10001", 2_000, "G202606050001-P001", "G202606050001")
            .expect("group buy debit can be applied");

        let turnover = store
            .withdrawal_turnover_for_user("U10001")
            .expect("withdrawal turnover can be calculated");

        assert_eq!(turnover.cumulative_recharge_minor, 10_000);
        assert_eq!(turnover.required_effective_bet_minor, 10_000);
        assert_eq!(turnover.completed_effective_bet_minor, 2_000);
        assert_eq!(turnover.remaining_effective_bet_minor, 8_000);
    }

    #[test]
    /// 充值赠送活动会给充值用户入账，并按充值单保持幂等。
    fn store_credits_recharge_bonus_once() {
        let mut store = FinanceStore::seeded();

        let entry = store
            .credit_recharge_bonus("U10001", 500, "R000000000001")
            .expect("recharge bonus can be credited");
        let repeated = store
            .credit_recharge_bonus("U10001", 500, "R000000000001")
            .expect("recharge bonus is idempotent");
        let account = store.account("U10001").expect("account exists");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(entry.kind, LedgerEntryKind::RechargeBonusCredit);
        assert_eq!(entry.amount_minor, 500);
        assert_eq!(
            entry.reference_id.as_deref(),
            Some("recharge-bonus:R000000000001")
        );
        assert_eq!(account.available_balance_minor, 12_500);
    }

    #[test]
    /// 充值返利会入账给上级代理，并按充值单保持幂等。
    fn store_credits_recharge_rebate_once() {
        let mut store = FinanceStore::seeded();

        let entry = store
            .credit_recharge_rebate("U90001", "U10001", 350, "R000000000001")
            .expect("recharge rebate can be credited");
        let repeated = store
            .credit_recharge_rebate("U90001", "U10001", 350, "R000000000001")
            .expect("recharge rebate is idempotent");
        let account = store.account("U90001").expect("agent account exists");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(entry.kind, LedgerEntryKind::RechargeRebateCredit);
        assert_eq!(entry.amount_minor, 350);
        assert_eq!(
            entry.reference_id.as_deref(),
            Some("recharge-rebate:R000000000001")
        );
        assert_eq!(account.available_balance_minor, 520_350);
    }

    #[test]
    /// 代理返利提现会扣减可用余额，并生成独立流水供后台统计已处理金额。
    fn store_withdraws_agent_rebate() {
        let mut store = FinanceStore::seeded();
        store
            .credit_recharge_rebate("U90001", "U10001", 350, "R000000000001")
            .expect("recharge rebate can be credited");

        let entry = store
            .withdraw_agent_rebate("U90001", 200, "代理返利提现处理")
            .expect("agent rebate can be withdrawn");
        let account = store.account("U90001").expect("agent account exists");

        assert_eq!(entry.kind, LedgerEntryKind::AgentRebateWithdrawal);
        assert_eq!(entry.amount_minor, -200);
        assert_eq!(account.available_balance_minor, 520_150);
    }

    #[test]
    /// 同一充值单的返利如果再次触发，即使传入了不同代理，也不会产生第二笔返利。
    fn store_recharge_rebate_idempotency_ignores_changed_agent() {
        let mut store = FinanceStore::seeded();

        let entry = store
            .credit_recharge_rebate("U90001", "U10001", 350, "R000000000001")
            .expect("recharge rebate can be credited");
        let repeated = store
            .credit_recharge_rebate("U10002", "U10001", 350, "R000000000001")
            .expect("changed agent repeat keeps idempotency");
        let original_agent = store.account("U90001").expect("original agent exists");
        let changed_agent = store.account("U10002").expect("changed agent exists");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(repeated.user_id, "U90001");
        assert_eq!(original_agent.available_balance_minor, 520_350);
        assert_eq!(changed_agent.available_balance_minor, 50_000);
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
    /// 验证资金流水可按用户筛选。
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

    /// 构造资金测试所需的订单详情。
    fn order_detail(id: &str, user_id: &str, amount_minor: i64, payout_minor: i64) -> OrderDetail {
        OrderDetail {
            id: id.to_string(),
            order_source: OrderSource::Direct,
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

    /// 构造资金流水，用于校验历史流水序号恢复。
    fn ledger_entry(id: &str) -> LedgerEntry {
        LedgerEntry {
            id: id.to_string(),
            user_id: "U10001".to_string(),
            kind: LedgerEntryKind::ManualAdjustment,
            amount_minor: 0,
            balance_after_minor: 0,
            reference_id: None,
            description: "测试流水".to_string(),
            created_at: "unix:1780388800".to_string(),
        }
    }

    /// 构造资金测试所需的结算记录。
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

    /// 构造带两名参与人的合买计划，用于财务分账和退款测试。
    fn group_buy_plan_with_order(id: &str, order_id: &str) -> GroupBuyPlan {
        GroupBuyPlan {
            id: id.to_string(),
            lottery_id: "fc3d".to_string(),
            lottery_name: "福彩 3D".to_string(),
            order_id: Some(order_id.to_string()),
            issue: "20260605001".to_string(),
            rule_code: "threeDirect".to_string(),
            title: "测试合买".to_string(),
            numbers: "1,2,3".to_string(),
            initiator_user_id: "U90001".to_string(),
            initiator_username: "agent_alpha".to_string(),
            total_amount_minor: 3_000,
            filled_amount_minor: 3_000,
            min_share_amount_minor: 1_000,
            participant_min_amount_minor: 1_000,
            share_count: 3,
            status: GroupBuyPlanStatus::Filled,
            participants: vec![
                GroupBuyParticipant {
                    id: format!("{id}-P001"),
                    user_id: "U90001".to_string(),
                    username: "agent_alpha".to_string(),
                    amount_minor: 1_000,
                    share_count: 1,
                    note: "发起人认购".to_string(),
                    created_at: "2026-06-05 16:00:00".to_string(),
                },
                GroupBuyParticipant {
                    id: format!("{id}-P002"),
                    user_id: "U10001".to_string(),
                    username: "demo_user".to_string(),
                    amount_minor: 2_000,
                    share_count: 2,
                    note: "参与合买".to_string(),
                    created_at: "2026-06-05 16:01:00".to_string(),
                },
            ],
            note: "测试计划".to_string(),
            created_at: "2026-06-05 16:00:00".to_string(),
            updated_at: "2026-06-05 16:01:00".to_string(),
        }
    }
}
