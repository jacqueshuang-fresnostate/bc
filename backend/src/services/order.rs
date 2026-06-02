use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    domain::{
        lottery::{LotteryKind, PlayCategory},
        order::{CreateOrderRequest, OrderDetail, OrderStatus, OrderSummary},
        play::PlayRuleCode,
    },
    error::{ApiError, ApiResult},
    services::play_rules::{expanded_bets_for_rule, number_type_for_rule},
};

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

    pub async fn cancel(&self, id: &str) -> ApiResult<OrderDetail> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))?
            .cancel(id)
    }

    pub async fn recent_summaries(&self, limit: usize) -> ApiResult<Vec<OrderSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("order store lock poisoned".to_string()))
            .map(|store| store.recent_summaries(limit))
    }
}

#[derive(Debug, Default)]
struct OrderStore {
    next_sequence: u64,
    orders: BTreeMap<String, OrderDetail>,
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
        validate_order_request(lottery, &payload)?;

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
            stake_count,
            unit_amount_minor: payload.unit_amount_minor,
            amount_minor,
            expanded_bets,
            status: OrderStatus::PendingDraw,
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

    fn recent_summaries(&self, limit: usize) -> Vec<OrderSummary> {
        self.orders
            .values()
            .rev()
            .take(limit)
            .map(OrderDetail::summary)
            .collect()
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

    let required_category = play_category_for_rule(&payload.rule_code);
    if !lottery.play_categories.contains(&required_category) {
        return Err(ApiError::BadRequest(
            "lottery does not enable this play category".to_string(),
        ));
    }

    Ok(())
}

pub fn play_category_for_rule(rule_code: &PlayRuleCode) -> PlayCategory {
    match rule_code {
        PlayRuleCode::ThreeDirect
        | PlayRuleCode::FiveFrontDirect
        | PlayRuleCode::FiveMiddleDirect
        | PlayRuleCode::FiveBackDirect => PlayCategory::Direct,
        PlayRuleCode::FiveFrontDirectCombination
        | PlayRuleCode::FiveMiddleDirectCombination
        | PlayRuleCode::FiveBackDirectCombination => PlayCategory::DirectCombination,
        PlayRuleCode::ThreeGroupThree
        | PlayRuleCode::ThreeGroupThreeBanker
        | PlayRuleCode::FiveFrontGroupThree
        | PlayRuleCode::FiveMiddleGroupThree
        | PlayRuleCode::FiveBackGroupThree
        | PlayRuleCode::FiveFrontGroupThreeBanker
        | PlayRuleCode::FiveMiddleGroupThreeBanker
        | PlayRuleCode::FiveBackGroupThreeBanker => PlayCategory::GroupThree,
        PlayRuleCode::ThreeGroupSix
        | PlayRuleCode::ThreeGroupSixBanker
        | PlayRuleCode::FiveFrontGroupSix
        | PlayRuleCode::FiveMiddleGroupSix
        | PlayRuleCode::FiveBackGroupSix
        | PlayRuleCode::FiveFrontGroupSixBanker
        | PlayRuleCode::FiveMiddleGroupSixBanker
        | PlayRuleCode::FiveBackGroupSixBanker => PlayCategory::GroupSix,
        PlayRuleCode::FiveBigSmallOddEven => PlayCategory::BigSmallOddEven,
    }
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
            lottery::{DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType},
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
        assert_eq!(order.expanded_bets, vec!["247"]);
        assert_eq!(order.status, OrderStatus::PendingDraw);
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
            .contains("lottery does not enable this play category"));
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

    fn lottery_with_categories(
        play_categories: Vec<crate::domain::lottery::PlayCategory>,
    ) -> LotteryKind {
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
        }
    }
}
