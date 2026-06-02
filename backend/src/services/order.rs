use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    domain::{
        draw::{DrawIssue, DrawIssueStatus},
        lottery::LotteryKind,
        order::{CreateOrderRequest, OrderDetail, OrderQuote, OrderStatus, OrderSummary},
        play::PlayRuleEvaluateRequest,
        settlement::{OrderSettlement, SettlementRun},
    },
    error::{ApiError, ApiResult},
    services::play_rules::{evaluate_play_rule, expanded_bets_for_rule, number_type_for_rule},
};

const ODDS_SCALE_BASIS_POINTS: i64 = 10_000;

#[derive(Clone)]
pub struct OrderRepository {
    inner: Arc<RwLock<OrderStore>>,
}

impl OrderRepository {
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(OrderStore::default())),
        }
    }

    pub async fn list(&self) -> ApiResult<Vec<OrderDetail>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    pub async fn get(&self, id: &str) -> ApiResult<OrderDetail> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .get(id)
    }

    pub async fn create(
        &self,
        lottery: &LotteryKind,
        payload: CreateOrderRequest,
    ) -> ApiResult<OrderDetail> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .create(lottery, payload)
    }

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

    pub async fn cancel(&self, id: &str) -> ApiResult<OrderDetail> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .cancel(id)
    }

    pub async fn remove_unfunded(&self, id: &str) -> ApiResult<OrderDetail> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .remove_unfunded(id)
    }

    pub async fn recent_summaries(&self, limit: usize) -> ApiResult<Vec<OrderSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))
            .map(|store| store.recent_summaries(limit))
    }

    pub async fn settlement_runs(&self) -> ApiResult<Vec<SettlementRun>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))
            .map(|store| store.settlement_runs())
    }

    pub async fn get_settlement(&self, id: &str) -> ApiResult<SettlementRun> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .get_settlement(id)
    }

    pub async fn settle_draw_issue(&self, draw_issue: &DrawIssue) -> ApiResult<SettlementRun> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .settle_draw_issue(draw_issue)
    }
}

#[derive(Debug, Default)]
struct OrderStore {
    next_sequence: u64,
    next_settlement_sequence: u64,
    orders: BTreeMap<String, OrderDetail>,
    settlement_runs: BTreeMap<String, SettlementRun>,
}

impl OrderStore {
    fn list(&self) -> Vec<OrderDetail> {
        self.orders.values().rev().cloned().collect()
    }

    fn get(&self, id: &str) -> ApiResult<OrderDetail> {
        self.orders
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("order `{id}` not found")))
    }

    fn create(
        &mut self,
        lottery: &LotteryKind,
        payload: CreateOrderRequest,
    ) -> ApiResult<OrderDetail> {
        let calculation = calculated_order(lottery, &payload)?;

        self.next_sequence += 1;
        let order = OrderDetail {
            id: format!("O{:012}", self.next_sequence),
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

    fn cancel(&mut self, id: &str) -> ApiResult<OrderDetail> {
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

    fn remove_unfunded(&mut self, id: &str) -> ApiResult<OrderDetail> {
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

    fn recent_summaries(&self, limit: usize) -> Vec<OrderSummary> {
        self.orders
            .values()
            .rev()
            .take(limit)
            .map(OrderDetail::summary)
            .collect()
    }

    fn settlement_runs(&self) -> Vec<SettlementRun> {
        self.settlement_runs.values().rev().cloned().collect()
    }

    fn get_settlement(&self, id: &str) -> ApiResult<SettlementRun> {
        self.settlement_runs
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("settlement `{id}` not found")))
    }

    fn settle_draw_issue(&mut self, draw_issue: &DrawIssue) -> ApiResult<SettlementRun> {
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
            lottery::{
                DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType,
                LotteryPlayConfig, PlayCategory,
            },
            order::{CreateOrderRequest, OrderStatus},
            play::{PlayRuleCode, PlaySelection},
        },
        services::order::OrderStore,
    };

    #[test]
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
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("247")))
            .expect("drawn issue can be settled");

        assert_eq!(run.settled_order_count, 2);
        assert_eq!(run.winning_order_count, 1);
        assert_eq!(run.total_stake_amount_minor, 400);
        assert_eq!(run.total_payout_minor, 2000);
        assert_eq!(run.orders.len(), 2);
        assert_eq!(run.orders[0].odds_basis_points, 100_000);

        let winning = store.get(&winning.id).expect("winning order exists");
        assert_eq!(winning.status, OrderStatus::Won);
        assert_eq!(winning.draw_number.as_deref(), Some("247"));
        assert_eq!(winning.matched_bets, vec!["247"]);
        assert_eq!(winning.payout_minor, 2000);
        assert!(winning.settled_at.is_some());

        let losing = store.get(&losing.id).expect("losing order exists");
        assert_eq!(losing.status, OrderStatus::Lost);
        assert_eq!(losing.draw_number.as_deref(), Some("247"));
        assert!(losing.matched_bets.is_empty());
        assert_eq!(losing.payout_minor, 0);
        assert!(losing.settled_at.is_some());
    }

    #[test]
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
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("247")))
            .expect("drawn issue can be settled");

        assert_eq!(run.settled_order_count, 0);
        let order = store.get(&order.id).expect("order exists");
        assert_eq!(order.status, OrderStatus::Cancelled);
        assert_eq!(order.draw_number, None);
        assert_eq!(order.payout_minor, 0);
    }

    #[test]
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
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("247")))
            .expect("drawn issue can be settled");
        assert!(store
            .settle_draw_issue(&draw_issue(DrawIssueStatus::Drawn, Some("247")))
            .expect_err("same issue cannot be settled twice")
            .to_string()
            .contains("already settled"));
    }

    fn lottery_with_categories(play_categories: Vec<PlayCategory>) -> LotteryKind {
        let play_configs = vec![
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeDirect,
                enabled: play_categories.contains(&PlayCategory::Direct),
                odds_basis_points: 100_000,
            },
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeGroupThree,
                enabled: play_categories.contains(&PlayCategory::GroupThree),
                odds_basis_points: 50_000,
            },
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeGroupThreeBanker,
                enabled: play_categories.contains(&PlayCategory::GroupThree),
                odds_basis_points: 50_000,
            },
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeGroupSix,
                enabled: play_categories.contains(&PlayCategory::GroupSix),
                odds_basis_points: 50_000,
            },
            LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeGroupSixBanker,
                enabled: play_categories.contains(&PlayCategory::GroupSix),
                odds_basis_points: 50_000,
            },
        ];

        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
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
