//! 订单领域模型，定义订单生命周期、金额和结算相关结构

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, Row};

use crate::{
    domain::{
        draw::{DrawIssue, DrawIssueStatus},
        finance::LedgerEntry,
        lottery::LotteryKind,
        order::{
            CreateOrderRequest, OrderDetail, OrderQuote, OrderSource, OrderStatus, OrderSummary,
        },
        play::{BigSmallOddEvenPosition, PlayRuleCode, PlayRuleEvaluateRequest, PlaySelection},
        settlement::{OrderSettlement, SettlementRun},
    },
    error::{ApiError, ApiResult},
    services::{
        finance::{save_finance_store_in_transaction, FinanceRepository},
        group_buy::{save_group_buy_store_in_transaction, GroupBuyRepository},
        play_rules::{
            evaluate_play_rule, expanded_bets_for_rule, number_type_for_rule, play_position_label,
            play_position_select_limit_targets,
        },
    },
};

use super::business_database::{
    enum_from_string, enum_to_string, from_json, to_json, BusinessDatabase,
};

const ODDS_SCALE_BASIS_POINTS: i64 = 10_000;

/// 校验期号与订单关联关系是否允许下单。
pub fn validate_draw_issue_accepts_order(
    draw_issue: &DrawIssue,
    lottery: &LotteryKind,
    issue: &str,
) -> ApiResult<()> {
    if draw_issue.lottery_id != lottery.id {
        return Err(ApiError::BadRequest(
            "draw issue lottery does not match order lottery".to_string(),
        ));
    }
    if draw_issue.issue != issue.trim() {
        return Err(ApiError::BadRequest(
            "draw issue number does not match order issue".to_string(),
        ));
    }
    if draw_issue.number_type != lottery.number_type {
        return Err(ApiError::BadRequest(
            "draw issue number type does not match lottery".to_string(),
        ));
    }
    if draw_issue.status != DrawIssueStatus::Open {
        return Err(ApiError::BadRequest(
            "draw issue is not open for order creation".to_string(),
        ));
    }

    Ok(())
}

#[derive(Clone)]
/// 订单仓储，负责投注订单、取消和结算派奖事务。
pub struct OrderRepository {
    pub(crate) inner: Arc<RwLock<OrderStore>>,
    pub(crate) persistence: Option<BusinessDatabase>,
}

/// 订单仓储的创建、取消和结算方法实现。
impl OrderRepository {
    /// 创建内存仓储实例。
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(OrderStore::default())),
            persistence: None,
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_order_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回完整列表。
    pub async fn list(&self) -> ApiResult<Vec<OrderDetail>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 按 ID 查询单条记录。
    pub async fn get(&self, id: &str) -> ApiResult<OrderDetail> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .get(id)
    }

    #[cfg(test)]
    /// 测试辅助：仅在测试中允许创建未扣款订单，运行时订单创建必须走事务扣款入口。
    pub async fn create(
        &self,
        lottery: &LotteryKind,
        payload: CreateOrderRequest,
    ) -> ApiResult<OrderDetail> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?;
            let result = store.create(lottery, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 校验入参并按指定来源创建一条新记录。
    pub async fn create_with_source(
        &self,
        lottery: &LotteryKind,
        payload: CreateOrderRequest,
        order_source: OrderSource,
    ) -> ApiResult<OrderDetail> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?;
            let result = store.create_with_source(lottery, payload, order_source)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 在同一个业务事务中创建订单并扣减用户余额。
    pub async fn create_with_debit(
        &self,
        finance: &FinanceRepository,
        lottery: &LotteryKind,
        payload: CreateOrderRequest,
        order_source: OrderSource,
    ) -> ApiResult<OrderDetail> {
        let orders = self
            .create_many_with_debit(finance, vec![(lottery.clone(), payload)], order_source)
            .await?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| ApiError::Internal("订单事务没有返回创建结果".to_string()))
    }

    /// 在同一个业务事务中批量创建订单并逐笔扣款，任一失败都不会提交部分订单。
    pub async fn create_many_with_debit(
        &self,
        finance: &FinanceRepository,
        requests: Vec<(LotteryKind, CreateOrderRequest)>,
        order_source: OrderSource,
    ) -> ApiResult<Vec<OrderDetail>> {
        if requests.is_empty() {
            return Ok(Vec::new());
        }

        let mut order_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();

        let mut orders = Vec::with_capacity(requests.len());
        for (lottery, payload) in requests {
            let order = order_store.create_with_source(&lottery, payload, order_source.clone())?;
            finance_store.debit_order(&order)?;
            orders.push(order);
        }

        persist_order_finance_stores(self, finance, &order_store, &finance_store).await?;
        self.replace_store(order_store)?;
        finance.replace_store(finance_store)?;

        Ok(orders)
    }

    /// 按当前彩种规则计算订单注数和应付金额。
    pub async fn quote(
        &self,
        lottery: &LotteryKind,
        payload: &CreateOrderRequest,
    ) -> ApiResult<OrderQuote> {
        calculated_order(lottery, payload).map(|calculation| OrderQuote {
            stake_count: calculation.stake_count,
            amount_minor: calculation.amount_minor,
            odds_basis_points: calculation.odds_basis_points,
        })
    }

    /// 取消开奖期并回退相关状态。
    pub async fn cancel(&self, id: &str) -> ApiResult<OrderDetail> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?;
            let result = store.cancel(id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 在同一个业务事务中取消订单并退回订单扣款。
    pub async fn cancel_with_refund(
        &self,
        finance: &FinanceRepository,
        id: &str,
    ) -> ApiResult<OrderDetail> {
        let mut order_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();

        let order = order_store.cancel(id)?;
        finance_store.refund_order(&order)?;

        persist_order_finance_stores(self, finance, &order_store, &finance_store).await?;
        self.replace_store(order_store)?;
        finance.replace_store(finance_store)?;

        Ok(order)
    }

    /// 清理未支付订单。
    pub async fn remove_unfunded(&self, id: &str) -> ApiResult<OrderDetail> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?;
            let result = store.remove_unfunded(id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 一键清除投注订单和计奖派奖历史；存在待开奖订单时拒绝清理，避免扣款订单失去结算机会。
    pub async fn clear_bet_records(&self) -> ApiResult<usize> {
        let (deleted_count, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?;
            let deleted_count = store.clear_bet_records()?;
            (deleted_count, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(deleted_count)
    }

    /// 返回最近订单汇总列表。
    pub async fn recent_summaries(&self, limit: usize) -> ApiResult<Vec<OrderSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))
            .map(|store| store.recent_summaries(limit))
    }

    /// 查询历史结算任务列表。
    pub async fn settlement_runs(&self) -> ApiResult<Vec<SettlementRun>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))
            .map(|store| store.settlement_runs())
    }

    /// 根据 ID 查询结算明细。
    pub async fn get_settlement(&self, id: &str) -> ApiResult<SettlementRun> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .get_settlement(id)
    }

    /// 对某期进行结算并返回执行结果。
    pub async fn settle_draw_issue(&self, draw_issue: &DrawIssue) -> ApiResult<SettlementRun> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?;
            let result = store.settle_draw_issue(draw_issue)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 在同一个业务事务中完成订单结算、派奖入账和合买计划结算状态回写。
    pub async fn settle_with_payouts(
        &self,
        finance: &FinanceRepository,
        group_buys: &GroupBuyRepository,
        draw_issue: &DrawIssue,
    ) -> ApiResult<(SettlementRun, Vec<LedgerEntry>)> {
        let _group_buy_mutation_guard = group_buys.mutation_lock.lock().await;
        let mut order_store = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .clone();
        let mut finance_store = finance
            .inner
            .read()
            .map_err(|_| ApiError::Internal("finance store lock poisoned".to_string()))?
            .clone();
        let mut group_buy_store = group_buys
            .inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .clone();

        let settlement = order_store.settle_draw_issue(draw_issue)?;
        let order_ids = settlement
            .orders
            .iter()
            .map(|order| order.order_id.clone())
            .collect::<Vec<_>>();
        let group_buy_plans = group_buy_store.plans_for_order_ids(&order_ids);
        let ledger_entries =
            finance_store.credit_settlement_with_group_buys(&settlement, &group_buy_plans)?;
        group_buy_store.mark_settled_by_order_ids(&order_ids);

        persist_order_finance_group_buy_stores(
            self,
            finance,
            group_buys,
            &order_store,
            &finance_store,
            &group_buy_store,
        )
        .await?;
        self.replace_store(order_store)?;
        finance.replace_store(finance_store)?;
        group_buys.replace_store(group_buy_store)?;

        Ok((settlement, ledger_entries))
    }

    async fn persist(&self, store: &OrderStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_order_store(persistence, store).await?;
        }

        Ok(())
    }

    pub(crate) fn replace_store(&self, store: OrderStore) -> ApiResult<()> {
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))? = store;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct OrderStore {
    next_sequence: u64,
    next_settlement_sequence: u64,
    orders: BTreeMap<String, OrderDetail>,
    settlement_runs: BTreeMap<String, SettlementRun>,
}

async fn load_order_store(database: &BusinessDatabase) -> ApiResult<OrderStore> {
    let pool = database.pool();
    let mut orders = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, order_source, user_id, lottery_id, lottery_name, issue, rule_code, number_type, selection,
                stake_count, unit_amount_minor, amount_minor, odds_basis_points, expanded_bets,
                draw_number, matched_bets, payout_minor, status, settled_at, created_at
         FROM orders
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?;
        let stake_count: i32 = row
            .try_get("stake_count")
            .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?;
        orders.insert(
            id.clone(),
            OrderDetail {
                id,
                order_source: enum_from_string(
                    row.try_get("order_source")
                        .map_err(|_| ApiError::Internal("订单来源数据读取失败".to_string()))?,
                )?,
                user_id: row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                lottery_id: row
                    .try_get("lottery_id")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                lottery_name: row
                    .try_get("lottery_name")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                issue: row
                    .try_get("issue")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                rule_code: enum_from_string(
                    row.try_get("rule_code")
                        .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                )?,
                number_type: enum_from_string(
                    row.try_get("number_type")
                        .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                )?,
                selection: from_json(
                    row.try_get("selection")
                        .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                )?,
                stake_count: u32::try_from(stake_count)
                    .map_err(|_| ApiError::Internal("订单注数数据无效".to_string()))?,
                unit_amount_minor: row
                    .try_get("unit_amount_minor")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                amount_minor: row
                    .try_get("amount_minor")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                odds_basis_points: row
                    .try_get("odds_basis_points")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                expanded_bets: from_json(
                    row.try_get("expanded_bets")
                        .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                )?,
                draw_number: row
                    .try_get("draw_number")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                matched_bets: from_json(
                    row.try_get("matched_bets")
                        .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                )?,
                payout_minor: row
                    .try_get("payout_minor")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                )?,
                settled_at: row
                    .try_get("settled_at")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("订单数据读取失败".to_string()))?,
            },
        );
    }

    let mut settlement_orders = BTreeMap::<String, Vec<OrderSettlement>>::new();
    for row in sqlx::query(
        "SELECT settlement_id, order_id, user_id, rule_code, stake_count, amount_minor, is_winning,
                matched_bets, odds_basis_points, payout_minor, status
         FROM order_settlements
         ORDER BY settlement_id ASC, order_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?
    {
        let settlement_id: String = row
            .try_get("settlement_id")
            .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?;
        let stake_count: i32 = row
            .try_get("stake_count")
            .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?;
        settlement_orders
            .entry(settlement_id)
            .or_default()
            .push(OrderSettlement {
                order_id: row
                    .try_get("order_id")
                    .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                user_id: row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                rule_code: enum_from_string(
                    row.try_get("rule_code")
                        .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                )?,
                stake_count: u32::try_from(stake_count)
                    .map_err(|_| ApiError::Internal("结算订单注数数据无效".to_string()))?,
                amount_minor: row
                    .try_get("amount_minor")
                    .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                is_winning: row
                    .try_get("is_winning")
                    .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                matched_bets: from_json(
                    row.try_get("matched_bets")
                        .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                )?,
                odds_basis_points: row
                    .try_get("odds_basis_points")
                    .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                payout_minor: row
                    .try_get("payout_minor")
                    .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("结算订单数据读取失败".to_string()))?,
                )?,
            });
    }

    let mut settlement_runs = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, draw_issue_id, lottery_id, lottery_name, issue, draw_number,
                settled_order_count, winning_order_count, total_stake_amount_minor,
                total_payout_minor, created_at
         FROM order_settlement_runs
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?;
        let settled_order_count: i32 = row
            .try_get("settled_order_count")
            .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?;
        let winning_order_count: i32 = row
            .try_get("winning_order_count")
            .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?;
        settlement_runs.insert(
            id.clone(),
            SettlementRun {
                id: id.clone(),
                draw_issue_id: row
                    .try_get("draw_issue_id")
                    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?,
                lottery_id: row
                    .try_get("lottery_id")
                    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?,
                lottery_name: row
                    .try_get("lottery_name")
                    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?,
                issue: row
                    .try_get("issue")
                    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?,
                draw_number: row
                    .try_get("draw_number")
                    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?,
                settled_order_count: u32::try_from(settled_order_count)
                    .map_err(|_| ApiError::Internal("结算订单数量无效".to_string()))?,
                winning_order_count: u32::try_from(winning_order_count)
                    .map_err(|_| ApiError::Internal("中奖订单数量无效".to_string()))?,
                total_stake_amount_minor: row
                    .try_get("total_stake_amount_minor")
                    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?,
                total_payout_minor: row
                    .try_get("total_payout_minor")
                    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("结算批次数据读取失败".to_string()))?,
                orders: settlement_orders.remove(&id).unwrap_or_default(),
            },
        );
    }

    let next_sequence =
        sqlx::query_scalar::<_, i64>("SELECT value FROM order_runtime WHERE key = 'next_sequence'")
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::Internal("订单运行数据读取失败".to_string()))?
            .unwrap_or_default();
    let next_settlement_sequence = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM order_runtime WHERE key = 'next_settlement_sequence'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("订单运行数据读取失败".to_string()))?
    .unwrap_or_default();

    Ok(OrderStore {
        next_sequence: u64::try_from(next_sequence)
            .unwrap_or_default()
            .max(max_sequence(orders.keys(), 'O')),
        next_settlement_sequence: u64::try_from(next_settlement_sequence)
            .unwrap_or_default()
            .max(max_sequence(settlement_runs.keys(), 'S')),
        orders,
        settlement_runs,
    })
}

async fn save_order_store(database: &BusinessDatabase, store: &OrderStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("订单事务开启失败".to_string()))?;

    save_order_store_in_transaction(&mut *tx, store).await?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("订单事务提交失败".to_string()))
}

pub(crate) async fn save_order_store_in_transaction(
    connection: &mut PgConnection,
    store: &OrderStore,
) -> ApiResult<()> {
    for table in [
        "order_settlements",
        "order_settlement_runs",
        "orders",
        "order_runtime",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("订单数据清理失败".to_string()))?;
    }

    for order in store.orders.values() {
        sqlx::query(
            "INSERT INTO orders
             (id, order_source, user_id, lottery_id, lottery_name, issue, rule_code, number_type, selection,
              stake_count, unit_amount_minor, amount_minor, odds_basis_points, expanded_bets,
              draw_number, matched_bets, payout_minor, status, settled_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20)",
        )
        .bind(&order.id)
        .bind(enum_to_string(&order.order_source)?)
        .bind(&order.user_id)
        .bind(&order.lottery_id)
        .bind(&order.lottery_name)
        .bind(&order.issue)
        .bind(enum_to_string(&order.rule_code)?)
        .bind(enum_to_string(&order.number_type)?)
        .bind(to_json(&order.selection)?)
        .bind(i32::try_from(order.stake_count).map_err(|_| {
            ApiError::Internal("订单注数过大".to_string())
        })?)
        .bind(order.unit_amount_minor)
        .bind(order.amount_minor)
        .bind(order.odds_basis_points)
        .bind(to_json(&order.expanded_bets)?)
        .bind(&order.draw_number)
        .bind(to_json(&order.matched_bets)?)
        .bind(order.payout_minor)
        .bind(enum_to_string(&order.status)?)
        .bind(&order.settled_at)
        .bind(&order.created_at)
        .execute(&mut *connection)
        .await
        .map_err(|_| ApiError::Internal("订单数据保存失败".to_string()))?;
    }

    for run in store.settlement_runs.values() {
        sqlx::query(
            "INSERT INTO order_settlement_runs
             (id, draw_issue_id, lottery_id, lottery_name, issue, draw_number, settled_order_count,
              winning_order_count, total_stake_amount_minor, total_payout_minor, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&run.id)
        .bind(&run.draw_issue_id)
        .bind(&run.lottery_id)
        .bind(&run.lottery_name)
        .bind(&run.issue)
        .bind(&run.draw_number)
        .bind(
            i32::try_from(run.settled_order_count)
                .map_err(|_| ApiError::Internal("结算订单数量过大".to_string()))?,
        )
        .bind(
            i32::try_from(run.winning_order_count)
                .map_err(|_| ApiError::Internal("中奖订单数量过大".to_string()))?,
        )
        .bind(run.total_stake_amount_minor)
        .bind(run.total_payout_minor)
        .bind(&run.created_at)
        .execute(&mut *connection)
        .await
        .map_err(|_| ApiError::Internal("结算批次数据保存失败".to_string()))?;

        for order in &run.orders {
            sqlx::query(
                "INSERT INTO order_settlements
                 (settlement_id, order_id, user_id, rule_code, stake_count, amount_minor,
                  is_winning, matched_bets, odds_basis_points, payout_minor, status)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(&run.id)
            .bind(&order.order_id)
            .bind(&order.user_id)
            .bind(enum_to_string(&order.rule_code)?)
            .bind(
                i32::try_from(order.stake_count)
                    .map_err(|_| ApiError::Internal("结算订单注数过大".to_string()))?,
            )
            .bind(order.amount_minor)
            .bind(order.is_winning)
            .bind(to_json(&order.matched_bets)?)
            .bind(order.odds_basis_points)
            .bind(order.payout_minor)
            .bind(enum_to_string(&order.status)?)
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("结算订单数据保存失败".to_string()))?;
        }
    }

    for (key, value) in [
        ("next_sequence", store.next_sequence),
        ("next_settlement_sequence", store.next_settlement_sequence),
    ] {
        sqlx::query("INSERT INTO order_runtime (key, value) VALUES ($1, $2)")
            .bind(key)
            .bind(
                i64::try_from(value)
                    .map_err(|_| ApiError::Internal("订单运行序号过大".to_string()))?,
            )
            .execute(&mut *connection)
            .await
            .map_err(|_| ApiError::Internal("订单运行数据保存失败".to_string()))?;
    }

    Ok(())
}

/// 在同一个数据库事务中保存订单和资金快照，确保投注扣款、取消退款不会只落一边。
async fn persist_order_finance_stores(
    orders: &OrderRepository,
    finance: &FinanceRepository,
    order_store: &OrderStore,
    finance_store: &super::finance::FinanceStore,
) -> ApiResult<()> {
    match (&orders.persistence, &finance.persistence) {
        (Some(database), Some(_)) => {
            let mut tx = database
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::Internal("订单资金事务开启失败".to_string()))?;
            save_order_store_in_transaction(&mut *tx, order_store).await?;
            save_finance_store_in_transaction(&mut *tx, finance_store).await?;
            tx.commit()
                .await
                .map_err(|_| ApiError::Internal("订单资金事务提交失败".to_string()))
        }
        (None, None) => Ok(()),
        _ => Err(ApiError::Internal("订单和资金持久化配置不一致".to_string())),
    }
}

/// 在同一个数据库事务中保存订单、资金和合买快照，确保结算派奖状态一致。
async fn persist_order_finance_group_buy_stores(
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    order_store: &OrderStore,
    finance_store: &super::finance::FinanceStore,
    group_buy_store: &super::group_buy::GroupBuyStore,
) -> ApiResult<()> {
    match (
        &orders.persistence,
        &finance.persistence,
        &group_buys.persistence,
    ) {
        (Some(database), Some(_), Some(_)) => {
            let mut tx = database
                .pool()
                .begin()
                .await
                .map_err(|_| ApiError::Internal("订单资金合买事务开启失败".to_string()))?;
            save_order_store_in_transaction(&mut *tx, order_store).await?;
            save_finance_store_in_transaction(&mut *tx, finance_store).await?;
            save_group_buy_store_in_transaction(&mut *tx, group_buy_store).await?;
            tx.commit()
                .await
                .map_err(|_| ApiError::Internal("订单资金合买事务提交失败".to_string()))
        }
        (None, None, None) => Ok(()),
        _ => Err(ApiError::Internal(
            "订单、资金和合买持久化配置不一致".to_string(),
        )),
    }
}

/// 计算并返回序列号最大值。
fn max_sequence<'a>(ids: impl Iterator<Item = &'a String>, prefix: char) -> u64 {
    ids.filter_map(|id| id.strip_prefix(prefix))
        .filter_map(|value| value.parse::<u64>().ok())
        .max()
        .unwrap_or_default()
}

impl OrderStore {
    /// 返回完整数据列表。
    fn list(&self) -> Vec<OrderDetail> {
        self.orders.values().rev().cloned().collect()
    }

    #[cfg(test)]
    /// 测试中使用的默认来源订单创建辅助方法。
    fn create(
        &mut self,
        lottery: &LotteryKind,
        payload: CreateOrderRequest,
    ) -> ApiResult<OrderDetail> {
        self.create_with_source(lottery, payload, OrderSource::Direct)
    }

    /// 按标识查询并返回单条记录。
    fn get(&self, id: &str) -> ApiResult<OrderDetail> {
        self.orders
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("order `{id}` not found")))
    }

    /// 校验入参并创建新记录。
    /// 校验入参并按来源创建新记录。
    pub(crate) fn create_with_source(
        &mut self,
        lottery: &LotteryKind,
        payload: CreateOrderRequest,
        order_source: OrderSource,
    ) -> ApiResult<OrderDetail> {
        let calculation = calculated_order(lottery, &payload)?;

        self.next_sequence += 1;
        let order = OrderDetail {
            id: format!("O{:012}", self.next_sequence),
            order_source,
            user_id: payload.user_id.trim().to_string(),
            lottery_id: lottery.id.clone(),
            lottery_name: lottery.name.clone(),
            issue: payload.issue.trim().to_string(),
            rule_code: payload.rule_code,
            number_type: lottery.number_type.clone(),
            selection: payload.selection,
            stake_count: calculation.stake_count,
            unit_amount_minor: payload.unit_amount_minor,
            amount_minor: calculation.amount_minor,
            odds_basis_points: calculation.odds_basis_points,
            expanded_bets: calculation.expanded_bets,
            draw_number: None,
            matched_bets: Vec::new(),
            payout_minor: 0,
            status: OrderStatus::PendingDraw,
            settled_at: None,
            created_at: current_timestamp_label(),
        };

        self.orders.insert(order.id.clone(), order.clone());
        Ok(order)
    }

    /// 处理 cancel 的具体内部流程。
    pub(crate) fn cancel(&mut self, id: &str) -> ApiResult<OrderDetail> {
        let order = self
            .orders
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("order `{id}` not found")))?;

        if order.status != OrderStatus::PendingDraw {
            return Err(ApiError::BadRequest(
                "only pending draw orders can be cancelled".to_string(),
            ));
        }

        order.status = OrderStatus::Cancelled;
        Ok(order.clone())
    }

    /// 处理 remove_unfunded 的具体内部流程。
    pub(crate) fn remove_unfunded(&mut self, id: &str) -> ApiResult<OrderDetail> {
        let order = self
            .orders
            .get(id)
            .ok_or_else(|| ApiError::NotFound(format!("order `{id}` not found")))?;

        if order.status != OrderStatus::PendingDraw {
            return Err(ApiError::BadRequest(
                "only pending draw orders can be removed after failed debit".to_string(),
            ));
        }

        self.orders
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("order `{id}` not found")))
    }

    /// 清除投注订单与对应结算批次，保留订单和结算流水号防止后续 ID 重复。
    fn clear_bet_records(&mut self) -> ApiResult<usize> {
        let pending_count = self
            .orders
            .values()
            .filter(|order| order.status == OrderStatus::PendingDraw)
            .count();
        if pending_count > 0 {
            return Err(ApiError::BadRequest(format!(
                "存在 {pending_count} 笔待开奖投注订单，请先开奖结算或取消后再清除记录"
            )));
        }

        let deleted_count = self.orders.len();
        self.orders.clear();
        self.settlement_runs.clear();
        Ok(deleted_count)
    }

    /// 处理 recent_summaries 的具体内部流程。
    fn recent_summaries(&self, limit: usize) -> Vec<OrderSummary> {
        self.orders
            .values()
            .rev()
            .take(limit)
            .map(OrderDetail::summary)
            .collect()
    }

    /// 处理 settlement_runs 的具体内部流程。
    fn settlement_runs(&self) -> Vec<SettlementRun> {
        self.settlement_runs.values().rev().cloned().collect()
    }

    /// 处理 get_settlement 的具体内部流程。
    fn get_settlement(&self, id: &str) -> ApiResult<SettlementRun> {
        self.settlement_runs
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("settlement `{id}` not found")))
    }

    /// 处理 settle_draw_issue 的具体内部流程。
    pub(crate) fn settle_draw_issue(&mut self, draw_issue: &DrawIssue) -> ApiResult<SettlementRun> {
        if draw_issue.status != DrawIssueStatus::Drawn {
            return Err(ApiError::BadRequest(
                "only drawn issues can be settled".to_string(),
            ));
        }

        let Some(draw_number) = draw_issue.draw_number.as_deref() else {
            return Err(ApiError::BadRequest(
                "draw issue does not have draw number".to_string(),
            ));
        };
        let draw_number = draw_number.to_string();

        if self
            .settlement_runs
            .values()
            .any(|run| run.draw_issue_id == draw_issue.id)
        {
            return Err(ApiError::Conflict(format!(
                "draw issue `{}` is already settled",
                draw_issue.id
            )));
        }

        self.next_settlement_sequence += 1;
        let settlement_id = format!("S{:012}", self.next_settlement_sequence);
        let settled_at = current_timestamp_label();
        let mut order_settlements = Vec::new();

        for order in self.orders.values_mut() {
            if order.status != OrderStatus::PendingDraw
                || order.lottery_id != draw_issue.lottery_id
                || order.issue != draw_issue.issue
            {
                continue;
            }

            let evaluation = evaluate_play_rule(PlayRuleEvaluateRequest {
                number_type: order.number_type.clone(),
                rule_code: order.rule_code.clone(),
                selection: order.selection.clone(),
                draw_number: draw_number.clone(),
            })?;
            let matched_bets = evaluation.matched_bets;
            let is_winning = !matched_bets.is_empty();
            let odds_basis_points = if is_winning {
                order.odds_basis_points
            } else {
                0
            };
            let payout_minor = payout_amount_minor(
                matched_bets.len(),
                order.unit_amount_minor,
                odds_basis_points,
            )?;
            let status = if is_winning {
                OrderStatus::Won
            } else {
                OrderStatus::Lost
            };

            order.draw_number = Some(draw_number.clone());
            order.matched_bets = matched_bets.clone();
            order.payout_minor = payout_minor;
            order.status = status.clone();
            order.settled_at = Some(settled_at.clone());

            order_settlements.push(OrderSettlement {
                order_id: order.id.clone(),
                user_id: order.user_id.clone(),
                rule_code: order.rule_code.clone(),
                stake_count: order.stake_count,
                amount_minor: order.amount_minor,
                is_winning,
                matched_bets,
                odds_basis_points,
                payout_minor,
                status,
            });
        }

        let winning_order_count = order_settlements
            .iter()
            .filter(|settlement| settlement.is_winning)
            .count() as u32;
        let total_stake_amount_minor = order_settlements
            .iter()
            .map(|settlement| settlement.amount_minor)
            .sum();
        let total_payout_minor = order_settlements
            .iter()
            .map(|settlement| settlement.payout_minor)
            .sum();

        let run = SettlementRun {
            id: settlement_id,
            draw_issue_id: draw_issue.id.clone(),
            lottery_id: draw_issue.lottery_id.clone(),
            lottery_name: draw_issue.lottery_name.clone(),
            issue: draw_issue.issue.clone(),
            draw_number,
            settled_order_count: order_settlements.len() as u32,
            winning_order_count,
            total_stake_amount_minor,
            total_payout_minor,
            created_at: settled_at,
            orders: order_settlements,
        };

        self.settlement_runs.insert(run.id.clone(), run.clone());
        Ok(run)
    }
}

/// 校验输入参数并返回校验结果。
fn validate_order_request(lottery: &LotteryKind, payload: &CreateOrderRequest) -> ApiResult<()> {
    if payload.user_id.trim().is_empty() {
        return Err(ApiError::BadRequest("user id is required".to_string()));
    }
    if payload.lottery_id.trim().is_empty() {
        return Err(ApiError::BadRequest("lottery id is required".to_string()));
    }
    if payload.lottery_id.trim() != lottery.id {
        return Err(ApiError::BadRequest(
            "request lottery id does not match lottery".to_string(),
        ));
    }
    if payload.issue.trim().is_empty() {
        return Err(ApiError::BadRequest("issue is required".to_string()));
    }
    if payload.unit_amount_minor <= 0 {
        return Err(ApiError::BadRequest(
            "unit amount must be greater than zero".to_string(),
        ));
    }
    if !lottery.sale_enabled {
        return Err(ApiError::BadRequest("lottery is not on sale".to_string()));
    }
    if number_type_for_rule(&payload.rule_code) != lottery.number_type {
        return Err(ApiError::BadRequest(
            "rule code does not match lottery number type".to_string(),
        ));
    }

    Ok(())
}

struct CalculatedOrder {
    stake_count: u32,
    amount_minor: i64,
    odds_basis_points: i64,
    expanded_bets: Vec<String>,
}

/// 处理 calculated_order 的具体内部流程。
fn calculated_order(
    lottery: &LotteryKind,
    payload: &CreateOrderRequest,
) -> ApiResult<CalculatedOrder> {
    validate_order_request(lottery, payload)?;
    let play_config = lottery
        .play_configs
        .iter()
        .find(|config| config.rule_code == payload.rule_code)
        .ok_or_else(|| {
            ApiError::BadRequest("lottery does not configure this play rule".to_string())
        })?;
    if !play_config.enabled {
        return Err(ApiError::BadRequest(
            "lottery does not enable this play rule".to_string(),
        ));
    }
    if play_config.odds_basis_points <= 0 {
        return Err(ApiError::BadRequest(
            "play odds basis points must be greater than zero".to_string(),
        ));
    }
    validate_position_select_limits(
        &play_config.rule_code,
        &play_config.position_select_limits,
        &payload.selection,
    )?;

    let expanded_bets = expanded_bets_for_rule(&payload.rule_code, &payload.selection)?;
    if expanded_bets.is_empty() {
        return Err(ApiError::BadRequest(
            "order must contain at least one stake".to_string(),
        ));
    }

    let stake_count = expanded_bets.len() as u32;
    let amount_minor = i64::from(stake_count)
        .checked_mul(payload.unit_amount_minor)
        .ok_or_else(|| ApiError::BadRequest("order amount is too large".to_string()))?;

    Ok(CalculatedOrder {
        stake_count,
        amount_minor,
        odds_basis_points: play_config.odds_basis_points,
        expanded_bets,
    })
}

/// 校验本玩法各位置是否超过后台配置的最大可选数量。
fn validate_position_select_limits(
    rule_code: &PlayRuleCode,
    limits: &[crate::domain::lottery::LotteryPlayPositionSelectLimit],
    selection: &PlaySelection,
) -> ApiResult<()> {
    for limit in limits {
        let selected_count = selected_count_for_position(rule_code, selection, &limit.position_key);
        if selected_count > limit.max_select_count as usize {
            let label = play_position_label(rule_code, &limit.position_key);
            return Err(ApiError::BadRequest(format!(
                "{label}最多选择 {} 个号码",
                limit.max_select_count
            )));
        }
    }

    Ok(())
}

/// 按玩法和位置 key 读取当前请求实际选择数量。
fn selected_count_for_position(
    rule_code: &PlayRuleCode,
    selection: &PlaySelection,
    position_key: &str,
) -> usize {
    let targets = play_position_select_limit_targets(rule_code);
    let target_index = targets
        .iter()
        .position(|(key, _)| *key == position_key)
        .unwrap_or(usize::MAX);

    match rule_code {
        PlayRuleCode::ThreeDirect
        | PlayRuleCode::FiveFrontDirect
        | PlayRuleCode::FiveMiddleDirect
        | PlayRuleCode::FiveBackDirect => selection
            .positions
            .get(target_index)
            .map(|digits| unique_digit_count(digits))
            .unwrap_or_default(),
        PlayRuleCode::ThreeGroupThree
        | PlayRuleCode::ThreeGroupSix
        | PlayRuleCode::FiveFrontDirectCombination
        | PlayRuleCode::FiveMiddleDirectCombination
        | PlayRuleCode::FiveBackDirectCombination
        | PlayRuleCode::FiveFrontGroupThree
        | PlayRuleCode::FiveMiddleGroupThree
        | PlayRuleCode::FiveBackGroupThree
        | PlayRuleCode::FiveFrontGroupSix
        | PlayRuleCode::FiveMiddleGroupSix
        | PlayRuleCode::FiveBackGroupSix => {
            if position_key == "numbers" {
                unique_digit_count(&selection.numbers)
            } else {
                0
            }
        }
        PlayRuleCode::ThreeGroupThreeBanker
        | PlayRuleCode::ThreeGroupSixBanker
        | PlayRuleCode::FiveFrontGroupThreeBanker
        | PlayRuleCode::FiveMiddleGroupThreeBanker
        | PlayRuleCode::FiveBackGroupThreeBanker
        | PlayRuleCode::FiveFrontGroupSixBanker
        | PlayRuleCode::FiveMiddleGroupSixBanker
        | PlayRuleCode::FiveBackGroupSixBanker => match position_key {
            "banker" => unique_digit_count(&selection.banker_numbers),
            "drag" => unique_digit_count(&selection.drag_numbers),
            _ => 0,
        },
        PlayRuleCode::FiveBigSmallOddEven => selection
            .big_small_odd_even
            .iter()
            .find(|pick| big_small_odd_even_position_key(&pick.position) == position_key)
            .map(|pick| pick.attributes.len())
            .unwrap_or_default(),
    }
}

/// 去重后统计数字选择数量，避免重复提交同一号码绕过上限。
fn unique_digit_count(digits: &[u8]) -> usize {
    digits.iter().copied().collect::<BTreeSet<_>>().len()
}

/// 大小单双位置枚举转为配置 key。
fn big_small_odd_even_position_key(position: &BigSmallOddEvenPosition) -> &'static str {
    match position {
        BigSmallOddEvenPosition::Tens => "tens",
        BigSmallOddEvenPosition::Ones => "ones",
    }
}

/// 处理 payout_amount_minor 的具体内部流程。
fn payout_amount_minor(
    matched_bet_count: usize,
    unit_amount_minor: i64,
    odds_basis_points: i64,
) -> ApiResult<i64> {
    let matched_bet_count = i64::try_from(matched_bet_count)
        .map_err(|_| ApiError::BadRequest("payout amount is too large".to_string()))?;
    matched_bet_count
        .checked_mul(unit_amount_minor)
        .and_then(|amount| amount.checked_mul(odds_basis_points))
        .map(|amount| amount / ODDS_SCALE_BASIS_POINTS)
        .ok_or_else(|| ApiError::BadRequest("payout amount is too large".to_string()))
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
            draw::{DrawIssue, DrawIssueStatus},
            finance::LedgerEntryKind,
            lottery::{
                DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType,
                LotteryPlayConfig, LotteryPlayPositionSelectLimit, PlayCategory,
            },
            order::{CreateOrderRequest, OrderSource, OrderStatus},
            play::{PlayRuleCode, PlaySelection},
        },
        services::{
            finance::FinanceRepository,
            group_buy::GroupBuyRepository,
            order::{OrderRepository, OrderStore},
        },
    };

    #[test]
    /// 处理 store_creates_order_from_play_rule_stakes 的具体内部流程。
    fn store_creates_order_from_play_rule_stakes() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let mut store = OrderStore::default();

        let order = store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026155".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect("order can be created");

        assert_eq!(order.stake_count, 1);
        assert_eq!(order.order_source, OrderSource::Direct);
        assert_eq!(order.amount_minor, 200);
        assert_eq!(order.odds_basis_points, 100_000);
        assert_eq!(order.expanded_bets, vec!["247"]);
        assert_eq!(order.status, OrderStatus::PendingDraw);
        assert_eq!(order.draw_number, None);
        assert!(order.matched_bets.is_empty());
        assert_eq!(order.payout_minor, 0);
        assert_eq!(order.settled_at, None);
    }

    #[test]
    /// 后台配置单位置选号上限后，订单报价会拒绝超出限制的选择。
    fn store_rejects_position_select_limit_overflow() {
        let mut lottery =
            lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        if let Some(config) = lottery
            .play_configs
            .iter_mut()
            .find(|config| config.rule_code == PlayRuleCode::ThreeDirect)
        {
            config.position_select_limits = vec![LotteryPlayPositionSelectLimit {
                position_key: "hundreds".to_string(),
                max_select_count: 1,
            }];
        }
        let mut store = OrderStore::default();

        let error = store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026155".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2, 3], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect_err("超过位置上限的订单需要被拒绝");

        assert!(error.to_string().contains("百位最多选择 1 个号码"));
    }

    #[tokio::test]
    /// 批量下注任一扣款失败时不会提交前面已经在快照中生成的订单和扣款。
    async fn repository_batch_create_with_debit_is_atomic_on_later_failure() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let first = direct_order_request("U10002", "2026155", 30_000);
        let second = direct_order_request("U10002", "2026155", 30_000);

        let error = orders
            .create_many_with_debit(
                &finance,
                vec![(lottery.clone(), first), (lottery, second)],
                OrderSource::Direct,
            )
            .await
            .expect_err("second debit should fail");

        assert!(error.to_string().contains("insufficient available balance"));
        assert!(orders.list().await.expect("orders can list").is_empty());
        let account = finance
            .accounts()
            .await
            .expect("accounts can list")
            .into_iter()
            .find(|account| account.user_id == "U10002")
            .expect("seed account exists");
        assert_eq!(account.available_balance_minor, 50_000);
        assert!(finance
            .ledger_entries()
            .await
            .expect("ledger can list")
            .into_iter()
            .all(|entry| entry.kind != LedgerEntryKind::OrderDebit));
    }

    #[tokio::test]
    /// 订单取消事务会同时回写订单状态和退款流水。
    async fn repository_cancel_with_refund_updates_order_and_balance() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let order = orders
            .create_with_debit(
                &finance,
                &lottery,
                direct_order_request("U10001", "2026155", 200),
                OrderSource::Direct,
            )
            .await
            .expect("order can be created with debit");

        let cancelled = orders
            .cancel_with_refund(&finance, &order.id)
            .await
            .expect("order can be cancelled with refund");
        let account = finance
            .accounts()
            .await
            .expect("accounts can list")
            .into_iter()
            .find(|account| account.user_id == "U10001")
            .expect("seed account exists");
        let refund_count = finance
            .ledger_entries()
            .await
            .expect("ledger can list")
            .into_iter()
            .filter(|entry| entry.kind == LedgerEntryKind::OrderRefund)
            .count();

        assert_eq!(cancelled.status, OrderStatus::Cancelled);
        assert_eq!(account.available_balance_minor, 12_000);
        assert_eq!(refund_count, 1);
    }

    #[tokio::test]
    /// 开奖结算事务会同时回写订单状态和中奖派奖流水。
    async fn repository_settle_with_payouts_updates_order_and_balance() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let order = orders
            .create_with_debit(
                &finance,
                &lottery,
                direct_order_request("U10001", "2026156", 200),
                OrderSource::Direct,
            )
            .await
            .expect("order can be created with debit");

        let (settlement, entries) = orders
            .settle_with_payouts(
                &finance,
                &group_buys,
                &draw_issue(DrawIssueStatus::Drawn, Some("2,4,7")),
            )
            .await
            .expect("drawn issue can settle with payout");
        let settled = orders.get(&order.id).await.expect("order exists");
        let account = finance
            .accounts()
            .await
            .expect("accounts can list")
            .into_iter()
            .find(|account| account.user_id == "U10001")
            .expect("seed account exists");

        assert_eq!(settlement.winning_order_count, 1);
        assert_eq!(entries.len(), 1);
        assert_eq!(settled.status, OrderStatus::Won);
        assert_eq!(account.available_balance_minor, 13_800);
    }

    #[test]
    /// 处理 store_creates_group_buy_source_order 的具体内部流程。
    fn store_creates_group_buy_source_order() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let mut store = OrderStore::default();

        let order = store
            .create_with_source(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026155".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
                OrderSource::GroupBuy,
            )
            .expect("group buy order can be created");

        assert_eq!(order.order_source, OrderSource::GroupBuy);
        assert_eq!(order.summary().order_source, OrderSource::GroupBuy);
    }

    #[test]
    /// 处理 store_rejects_disabled_play_category 的具体内部流程。
    fn store_rejects_disabled_play_category() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let mut store = OrderStore::default();

        let error = store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026155".to_string(),
                    rule_code: PlayRuleCode::ThreeGroupSix,
                    selection: PlaySelection {
                        numbers: vec![1, 2, 4],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect_err("category should be rejected");

        assert!(error
            .to_string()
            .contains("lottery does not enable this play rule"));
    }

    #[test]
    /// 处理 store_cancels_pending_order_once 的具体内部流程。
    fn store_cancels_pending_order_once() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let mut store = OrderStore::default();

        let order = store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026155".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect("order can be created");

        let cancelled = store.cancel(&order.id).expect("order can be cancelled");

        assert_eq!(cancelled.status, OrderStatus::Cancelled);
        assert!(store
            .cancel(&order.id)
            .expect_err("cancelled order cannot be cancelled again")
            .to_string()
            .contains("only pending draw orders can be cancelled"));
    }

    #[test]
    /// 处理 store_settles_drawn_issue_and_updates_order_statuses 的具体内部流程。
    fn store_settles_drawn_issue_and_updates_order_statuses() {
        let lottery = lottery_with_categories(vec![
            crate::domain::lottery::PlayCategory::Direct,
            crate::domain::lottery::PlayCategory::GroupSix,
        ]);
        let mut store = OrderStore::default();
        let winning = store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026156".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect("winning order can be created");
        let losing = store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10002".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026156".to_string(),
                    rule_code: PlayRuleCode::ThreeGroupSix,
                    selection: PlaySelection {
                        numbers: vec![1, 2, 3],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect("losing order can be created");

        let run = store
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("2,4,7")))
            .expect("drawn issue can be settled");

        assert_eq!(run.settled_order_count, 2);
        assert_eq!(run.winning_order_count, 1);
        assert_eq!(run.total_stake_amount_minor, 400);
        assert_eq!(run.total_payout_minor, 2000);
        assert_eq!(run.orders.len(), 2);
        assert_eq!(run.orders[0].odds_basis_points, 100_000);

        let winning = store.get(&winning.id).expect("winning order exists");
        assert_eq!(winning.status, OrderStatus::Won);
        assert_eq!(winning.draw_number.as_deref(), Some("2,4,7"));
        assert_eq!(winning.matched_bets, vec!["247"]);
        assert_eq!(winning.payout_minor, 2000);
        assert!(winning.settled_at.is_some());

        let losing = store.get(&losing.id).expect("losing order exists");
        assert_eq!(losing.status, OrderStatus::Lost);
        assert_eq!(losing.draw_number.as_deref(), Some("2,4,7"));
        assert!(losing.matched_bets.is_empty());
        assert_eq!(losing.payout_minor, 0);
        assert!(losing.settled_at.is_some());
    }

    #[test]
    /// 清理投注记录会拒绝待开奖订单，避免已经扣款的订单失去结算机会。
    fn store_clear_bet_records_rejects_pending_draw_orders() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let mut store = OrderStore::default();
        store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026155".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect("order can be created");

        assert!(store
            .clear_bet_records()
            .expect_err("pending draw order cannot be cleared")
            .to_string()
            .contains("待开奖投注订单"));
    }

    #[test]
    /// 已结算订单允许一键清理，并同步清除计奖派奖批次。
    fn store_clear_bet_records_removes_settlement_runs() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let mut store = OrderStore::default();
        store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026156".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect("order can be created");
        store
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("2,4,7")))
            .expect("drawn issue can be settled");

        assert_eq!(
            store
                .clear_bet_records()
                .expect("settled records can clear"),
            1
        );
        assert!(store.list().is_empty());
        assert!(store.settlement_runs().is_empty());
    }

    #[test]
    /// 处理 store_skips_cancelled_orders_when_settling 的具体内部流程。
    fn store_skips_cancelled_orders_when_settling() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let mut store = OrderStore::default();
        let order = store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026156".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect("order can be created");

        store.cancel(&order.id).expect("order can be cancelled");
        let run = store
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("2,4,7")))
            .expect("drawn issue can be settled");

        assert_eq!(run.settled_order_count, 0);
        let order = store.get(&order.id).expect("order exists");
        assert_eq!(order.status, OrderStatus::Cancelled);
        assert_eq!(order.draw_number, None);
        assert_eq!(order.payout_minor, 0);
    }

    #[test]
    /// 处理 store_rejects_unfinished_or_duplicate_settlement 的具体内部流程。
    fn store_rejects_unfinished_or_duplicate_settlement() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let mut store = OrderStore::default();
        store
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    issue: "2026156".to_string(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![2], vec![4], vec![7]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .expect("order can be created");

        assert!(store
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Open, None))
            .expect_err("open issue cannot be settled")
            .to_string()
            .contains("only drawn issues can be settled"));

        store
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("2,4,7")))
            .expect("drawn issue can be settled");
        assert!(store
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("2,4,7")))
            .expect_err("same issue cannot be settled twice")
            .to_string()
            .contains("already settled"));
    }

    #[test]
    /// 处理 draw_issue_must_be_open_before_order_creation 的具体内部流程。
    fn draw_issue_must_be_open_before_order_creation() {
        let lottery = lottery_with_categories(vec![crate::domain::lottery::PlayCategory::Direct]);
        let open_issue = draw_issue(DrawIssueStatus::Open, None);

        super::validate_draw_issue_accepts_order(&open_issue, &lottery, "2026156")
            .expect("open issue can accept orders");

        for status in [
            DrawIssueStatus::Closed,
            DrawIssueStatus::Drawn,
            DrawIssueStatus::Cancelled,
        ] {
            let error = super::validate_draw_issue_accepts_order(
                &draw_issue(status, None),
                &lottery,
                "2026156",
            )
            .expect_err("non-open issue cannot accept orders");
            assert!(error
                .to_string()
                .contains("draw issue is not open for order creation"));
        }

        let error = super::validate_draw_issue_accepts_order(&open_issue, &lottery, "2026157")
            .expect_err("mismatched issue cannot accept orders");
        assert!(error
            .to_string()
            .contains("draw issue number does not match order issue"));
    }

    /// 构造三位直选订单请求，供仓储事务测试复用。
    fn direct_order_request(
        user_id: &str,
        issue: &str,
        unit_amount_minor: i64,
    ) -> CreateOrderRequest {
        CreateOrderRequest {
            user_id: user_id.to_string(),
            lottery_id: "fc3d".to_string(),
            issue: issue.to_string(),
            rule_code: PlayRuleCode::ThreeDirect,
            selection: PlaySelection {
                positions: vec![vec![2], vec![4], vec![7]],
                ..PlaySelection::default()
            },
            unit_amount_minor,
        }
    }

    /// 处理 lottery_with_categories 的具体内部流程。
    fn lottery_with_categories(play_categories: Vec<PlayCategory>) -> LotteryKind {
        let play_configs = vec![
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeDirect,
                enabled: play_categories.contains(&PlayCategory::Direct),
                odds_basis_points: 100_000,
                position_select_limits: Vec::new(),
            },
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeGroupThree,
                enabled: play_categories.contains(&PlayCategory::GroupThree),
                odds_basis_points: 50_000,
                position_select_limits: Vec::new(),
            },
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeGroupThreeBanker,
                enabled: play_categories.contains(&PlayCategory::GroupThree),
                odds_basis_points: 50_000,
                position_select_limits: Vec::new(),
            },
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeGroupSix,
                enabled: play_categories.contains(&PlayCategory::GroupSix),
                odds_basis_points: 50_000,
                position_select_limits: Vec::new(),
            },
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeGroupSixBanker,
                enabled: play_categories.contains(&PlayCategory::GroupSix),
                odds_basis_points: 50_000,
                position_select_limits: Vec::new(),
            },
        ];

        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            category: "regional".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: true,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1000,
            },
            play_categories,
            play_configs,
        }
    }

    /// 处理 draw_issue 的具体内部流程。
    fn draw_issue(status: DrawIssueStatus, draw_number: Option<&str>) -> DrawIssue {
        DrawIssue {
            id: "D000000000001".to_string(),
            lottery_id: "fc3d".to_string(),
            lottery_name: "福彩 3D".to_string(),
            issue: "2026156".to_string(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            scheduled_at: "2026-06-02 21:00:15".to_string(),
            sale_closed_at: "2026-06-02 20:59:45".to_string(),
            status,
            draw_number: draw_number.map(str::to_string),
            drawn_at: draw_number.map(|_| "unix:1780389000".to_string()),
            created_at: "unix:1780388800".to_string(),
        }
    }
}
