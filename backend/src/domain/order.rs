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
    /// 投注订单 ID。
    pub id: String,
    /// 订单来源，区分独立投注和合买订单。
    pub order_source: OrderSource,
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: Option<String>,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: PlayRuleCode,
    /// 投注注数。
    pub stake_count: u32,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 赔率基点，10000 表示 1 倍。
    pub odds_basis_points: i64,
    /// 开奖号码，使用英文逗号分隔。
    pub draw_number: Option<String>,
    /// 中奖匹配项。
    pub matched_bets: Vec<String>,
    /// 派奖金额，单位为分。
    pub payout_minor: i64,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: OrderStatus,
    /// 结算完成时间。
    pub settled_at: Option<String>,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后端创建注单时使用的投注请求。
pub struct CreateOrderRequest {
    /// 关联用户 ID。
    pub user_id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: PlayRuleCode,
    /// 用户选择的投注号码结构。
    pub selection: PlaySelection,
    /// 单注金额，单位为分。
    pub unit_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 玩法报价结果，返回注数、总金额和赔率。
pub struct OrderQuote {
    /// 投注注数。
    pub stake_count: u32,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 赔率基点，10000 表示 1 倍。
    pub odds_basis_points: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 注单完整详情，保存原始选号、展开注码、匹配项和派奖信息。
pub struct OrderDetail {
    /// 投注订单 ID。
    pub id: String,
    /// 订单来源，区分独立投注和合买订单。
    pub order_source: OrderSource,
    /// 关联用户 ID。
    pub user_id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: PlayRuleCode,
    /// 号码类型，决定开奖号码长度和玩法目录。
    pub number_type: LotteryNumberType,
    /// 用户选择的投注号码结构。
    pub selection: PlaySelection,
    /// 投注注数。
    pub stake_count: u32,
    /// 单注金额，单位为分。
    pub unit_amount_minor: i64,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 赔率基点，10000 表示 1 倍。
    pub odds_basis_points: i64,
    /// 按玩法展开后的投注明细。
    pub expanded_bets: Vec<String>,
    /// 开奖号码，使用英文逗号分隔。
    pub draw_number: Option<String>,
    /// 中奖匹配项。
    pub matched_bets: Vec<String>,
    /// 派奖金额，单位为分。
    pub payout_minor: i64,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: OrderStatus,
    /// 结算完成时间。
    pub settled_at: Option<String>,
    /// 创建时间。
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
            username: None,
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
