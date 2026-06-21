//! 手机端公共聊天大厅领域模型，定义大厅消息、红包和合买分享请求。

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 聊天大厅消息类型，决定手机端按文本、红包或合买计划卡片渲染。
pub enum ChatHallMessageType {
    Text,
    RedPacket,
    GroupBuyPlan,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 聊天大厅消息实体，既承载普通文本，也承载结构化扩展 payload。
pub struct ChatHallMessage {
    /// 聊天大厅消息 ID。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: String,
    /// 用户头像图片地址。
    #[serde(default)]
    pub avatar_url: String,
    /// 消息正文内容。
    pub content: String,
    /// 消息类型，区分文本、图片、红包等。
    pub message_type: ChatHallMessageType,
    /// payload字段。
    pub payload: Option<Value>,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户发送普通聊天大厅文本消息的请求。
pub struct CreateChatHallMessageRequest {
    /// 消息正文内容。
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户在聊天大厅发送红包时提交的金额、份数和祝福语。
pub struct CreateChatHallRedPacketRequest {
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 红包可领取总份数。
    pub claim_count: u32,
    /// 红包祝福语。
    pub greeting: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个红包领取记录，用于展示已领取用户和入账金额。
pub struct ChatHallRedPacketClaim {
    /// 业务唯一标识。
    pub id: String,
    /// 聊天大厅红包 ID。
    pub red_packet_id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: String,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 聊天大厅红包主记录，保存总额、剩余金额和领取进度。
pub struct ChatHallRedPacket {
    /// 聊天大厅红包 ID。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 剩余未领取或未认购金额，单位为分。
    pub remaining_amount_minor: i64,
    /// 红包可领取总份数。
    pub claim_count: u32,
    /// 已领取份数。
    pub claimed_count: u32,
    /// 红包祝福语。
    pub greeting: String,
    /// 红包领取记录列表。
    pub claims: Vec<ChatHallRedPacketClaim>,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 红包消息写入聊天 payload 的公开数据，供手机端刷新卡片状态。
pub struct ChatHallRedPacketPayload {
    /// 聊天大厅红包 ID。
    pub red_packet_id: String,
    /// 红包祝福语。
    pub greeting: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 剩余未领取或未认购金额，单位为分。
    pub remaining_amount_minor: i64,
    /// 红包可领取总份数。
    pub claim_count: u32,
    /// 已领取份数。
    pub claimed_count: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 红包领取记录查询响应，供聊天大厅红包卡片查看已领取用户。
pub struct ChatHallRedPacketClaimsResponse {
    /// 聊天大厅红包 ID。
    pub red_packet_id: String,
    /// 红包祝福语。
    pub greeting: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 剩余未领取或未认购金额，单位为分。
    pub remaining_amount_minor: i64,
    /// 红包可领取总份数。
    pub claim_count: u32,
    /// 已领取份数。
    pub claimed_count: u32,
    /// 红包领取记录列表。
    pub claims: Vec<ChatHallRedPacketClaim>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 当前用户在聊天大厅的发言资格状态，由后端按系统设置和累计充值统一判断。
pub struct ChatHallSpeakingStatusResponse {
    /// 是否允许在聊天大厅发言、发红包或分享合买计划。
    pub can_speak: bool,
    /// 后台配置的最低累计充值金额，单位为分，0 表示不限制。
    pub required_recharge_minor: i64,
    /// 当前用户已真实入账的充值本金合计，单位为分。
    pub current_recharge_minor: i64,
    /// 距离发言门槛还差的金额，单位为分。
    pub missing_recharge_minor: i64,
    /// 不满足门槛时给手机端直接展示的中文提示。
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买计划分享消息的 payload，保存卡片展示所需的计划摘要。
pub struct ChatHallGroupBuyPlanPayload {
    /// 合买计划 ID。
    pub plan_id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 彩票期号。
    pub issue: String,
    /// 玩法中文名称。
    pub play_name: String,
    /// 展示标题。
    pub title: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 单份金额，单位为分。
    pub share_amount_minor: i64,
    /// 已售份数。
    pub sold_shares: u32,
    /// 剩余可认购份数。
    pub available_shares: u32,
    /// 合买进度百分比。
    pub progress_percent: u32,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户把自己的合买计划分享到聊天大厅时提交的计划编号。
pub struct ShareChatHallGroupBuyPlanRequest {
    /// 合买计划 ID。
    pub plan_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户领取红包后的响应，同时返回更新后的红包消息和最新余额。
pub struct ClaimChatHallRedPacketResponse {
    /// 返回给前端或日志展示的中文消息。
    pub message: ChatHallMessage,
    /// claim字段。
    pub claim: ChatHallRedPacketClaim,
    /// 可用余额，单位为分。
    pub available_balance_minor: i64,
}
