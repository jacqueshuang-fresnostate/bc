//! 计奖派奖领域模型，定义结算批次与订单结算明细

use serde::{Deserialize, Serialize};

use crate::domain::{order::OrderStatus, play::PlayRuleCode};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单次计奖派奖批次，记录某个期号的结算汇总和订单明细。
pub struct SettlementRun {
    pub id: String,
    pub draw_issue_id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub draw_number: String,
    pub settled_order_count: u32,
    pub winning_order_count: u32,
    pub total_stake_amount_minor: i64,
    pub total_payout_minor: i64,
    pub created_at: String,
    pub orders: Vec<OrderSettlement>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个订单的计奖结果，保存命中项、派奖金额和最终状态。
pub struct OrderSettlement {
    pub order_id: String,
    pub user_id: String,
    pub rule_code: PlayRuleCode,
    pub stake_count: u32,
    pub amount_minor: i64,
    pub is_winning: bool,
    pub matched_bets: Vec<String>,
    pub odds_basis_points: i64,
    pub payout_minor: i64,
    pub status: OrderStatus,
}
