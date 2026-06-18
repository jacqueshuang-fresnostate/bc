//! 计奖派奖领域模型，定义结算批次与订单结算明细

use serde::{Deserialize, Serialize};

use crate::domain::{order::OrderStatus, play::PlayRuleCode};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单次计奖派奖批次，记录某个期号的结算汇总和订单明细。
pub struct SettlementRun {
    /// 业务唯一标识。
    pub id: String,
    /// 开奖期号记录 ID。
    pub draw_issue_id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 彩票期号。
    pub issue: String,
    /// 开奖号码，使用英文逗号分隔。
    pub draw_number: String,
    /// 已结算订单数量。
    pub settled_order_count: u32,
    /// 中奖订单数量。
    pub winning_order_count: u32,
    /// 参与结算的投注本金合计，单位为分。
    pub total_stake_amount_minor: i64,
    /// 派奖金额合计，单位为分。
    pub total_payout_minor: i64,
    /// 创建时间。
    pub created_at: String,
    /// 结算涉及的订单列表。
    pub orders: Vec<OrderSettlement>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个订单的计奖结果，保存命中项、派奖金额和最终状态。
pub struct OrderSettlement {
    /// 投注订单 ID。
    pub order_id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 玩法规则编码。
    pub rule_code: PlayRuleCode,
    /// 投注注数。
    pub stake_count: u32,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 是否中奖。
    pub is_winning: bool,
    /// 中奖匹配项。
    pub matched_bets: Vec<String>,
    /// 赔率基点，10000 表示 1 倍。
    pub odds_basis_points: i64,
    /// 派奖金额，单位为分。
    pub payout_minor: i64,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: OrderStatus,
}
