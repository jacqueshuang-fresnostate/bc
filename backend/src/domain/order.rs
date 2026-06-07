//! 订单领域模型，定义订单生命周期、金额和结算相关结构

use serde::{Deserialize, Serialize};

use crate::domain::{
    lottery::LotteryNumberType,
    play::{PlayRuleCode, PlaySelection},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 注单状态，描述待开奖、中奖、未中奖和已取消的生命周期。
pub enum OrderStatus {
    PendingDraw,
    Won,
    Lost,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 注单来源，用于区分普通独立投注和合买生成的投注。
pub enum OrderSource {
    Direct,
    GroupBuy,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 注单列表摘要，保留运营和用户列表页需要的关键字段。
pub struct OrderSummary {
    pub id: String,
    pub order_source: OrderSource,
    pub user_id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub rule_code: PlayRuleCode,
    pub stake_count: u32,
    pub amount_minor: i64,
    pub odds_basis_points: i64,
    pub draw_number: Option<String>,
    pub matched_bets: Vec<String>,
    pub payout_minor: i64,
    pub status: OrderStatus,
    pub settled_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后端创建注单时使用的投注请求。
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
/// 玩法报价结果，返回注数、总金额和赔率。
pub struct OrderQuote {
    pub stake_count: u32,
    pub amount_minor: i64,
    pub odds_basis_points: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 注单完整详情，保存原始选号、展开注码、匹配项和派奖信息。
pub struct OrderDetail {
    pub id: String,
    pub order_source: OrderSource,
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
    pub odds_basis_points: i64,
    pub expanded_bets: Vec<String>,
    pub draw_number: Option<String>,
    pub matched_bets: Vec<String>,
    pub payout_minor: i64,
    pub status: OrderStatus,
    pub settled_at: Option<String>,
    pub created_at: String,
}

/// 注单详情的派生展示方法。
impl OrderDetail {
    /// 生成当前实体对象的汇总信息。
    pub fn summary(&self) -> OrderSummary {
        OrderSummary {
            id: self.id.clone(),
            order_source: self.order_source.clone(),
            user_id: self.user_id.clone(),
            lottery_id: self.lottery_id.clone(),
            lottery_name: self.lottery_name.clone(),
            issue: self.issue.clone(),
            rule_code: self.rule_code.clone(),
            stake_count: self.stake_count,
            amount_minor: self.amount_minor,
            odds_basis_points: self.odds_basis_points,
            draw_number: self.draw_number.clone(),
            matched_bets: self.matched_bets.clone(),
            payout_minor: self.payout_minor,
            status: self.status.clone(),
            settled_at: self.settled_at.clone(),
            created_at: self.created_at.clone(),
        }
    }
}
