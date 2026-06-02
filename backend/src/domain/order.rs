use serde::{Deserialize, Serialize};

use crate::domain::{
    lottery::LotteryNumberType,
    play::{PlayRuleCode, PlaySelection},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OrderStatus {
    PendingDraw,
    Won,
    Lost,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummary {
    pub id: String,
    pub user_id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub rule_code: PlayRuleCode,
    pub stake_count: u32,
    pub amount_minor: i64,
    pub draw_number: Option<String>,
    pub matched_bets: Vec<String>,
    pub payout_minor: i64,
    pub status: OrderStatus,
    pub settled_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateOrderRequest {
    pub user_id: String,
    pub lottery_id: String,
    pub issue: String,
    pub rule_code: PlayRuleCode,
    pub selection: PlaySelection,
    pub unit_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrderQuote {
    pub stake_count: u32,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetail {
    pub id: String,
    pub user_id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub rule_code: PlayRuleCode,
    pub number_type: LotteryNumberType,
    pub selection: PlaySelection,
    pub stake_count: u32,
    pub unit_amount_minor: i64,
    pub amount_minor: i64,
    pub expanded_bets: Vec<String>,
    pub draw_number: Option<String>,
    pub matched_bets: Vec<String>,
    pub payout_minor: i64,
    pub status: OrderStatus,
    pub settled_at: Option<String>,
    pub created_at: String,
}

impl OrderDetail {
    pub fn summary(&self) -> OrderSummary {
        OrderSummary {
            id: self.id.clone(),
            user_id: self.user_id.clone(),
            lottery_id: self.lottery_id.clone(),
            lottery_name: self.lottery_name.clone(),
            issue: self.issue.clone(),
            rule_code: self.rule_code.clone(),
            stake_count: self.stake_count,
            amount_minor: self.amount_minor,
            draw_number: self.draw_number.clone(),
            matched_bets: self.matched_bets.clone(),
            payout_minor: self.payout_minor,
            status: self.status.clone(),
            settled_at: self.settled_at.clone(),
            created_at: self.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyPlanSummary {
    pub id: String,
    pub lottery_id: String,
    pub initiator_user_id: String,
    pub total_amount_minor: i64,
    pub filled_amount_minor: i64,
    pub share_count: u32,
    pub status: String,
}
