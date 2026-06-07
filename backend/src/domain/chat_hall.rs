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
    pub id: String,
    pub user_id: String,
    pub username: String,
    #[serde(default)]
    pub avatar_url: String,
    pub content: String,
    pub message_type: ChatHallMessageType,
    pub payload: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户发送普通聊天大厅文本消息的请求。
pub struct CreateChatHallMessageRequest {
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户在聊天大厅发送红包时提交的金额、份数和祝福语。
pub struct CreateChatHallRedPacketRequest {
    pub amount_minor: i64,
    pub claim_count: u32,
    pub greeting: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个红包领取记录，用于展示已领取用户和入账金额。
pub struct ChatHallRedPacketClaim {
    pub id: String,
    pub red_packet_id: String,
    pub user_id: String,
    pub username: String,
    pub amount_minor: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 聊天大厅红包主记录，保存总额、剩余金额和领取进度。
pub struct ChatHallRedPacket {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub total_amount_minor: i64,
    pub remaining_amount_minor: i64,
    pub claim_count: u32,
    pub claimed_count: u32,
    pub greeting: String,
    pub claims: Vec<ChatHallRedPacketClaim>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 红包消息写入聊天 payload 的公开数据，供手机端刷新卡片状态。
pub struct ChatHallRedPacketPayload {
    pub red_packet_id: String,
    pub greeting: String,
    pub total_amount_minor: i64,
    pub remaining_amount_minor: i64,
    pub claim_count: u32,
    pub claimed_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买计划分享消息的 payload，保存卡片展示所需的计划摘要。
pub struct ChatHallGroupBuyPlanPayload {
    pub plan_id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub play_name: String,
    pub title: String,
    pub total_amount_minor: i64,
    pub share_amount_minor: i64,
    pub sold_shares: u32,
    pub available_shares: u32,
    pub progress_percent: u32,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户把自己的合买计划分享到聊天大厅时提交的计划编号。
pub struct ShareChatHallGroupBuyPlanRequest {
    pub plan_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户领取红包后的响应，同时返回更新后的红包消息和最新余额。
pub struct ClaimChatHallRedPacketResponse {
    pub message: ChatHallMessage,
    pub claim: ChatHallRedPacketClaim,
    pub available_balance_minor: i64,
}
