//! 手机端公共聊天大厅领域模型，定义大厅消息、红包和合买分享请求。

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ChatHallMessageType {
    Text,
    RedPacket,
    GroupBuyPlan,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatHallMessage {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub message_type: ChatHallMessageType,
    pub payload: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateChatHallMessageRequest {
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateChatHallRedPacketRequest {
    pub amount_minor: i64,
    pub claim_count: u32,
    pub greeting: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub struct ShareChatHallGroupBuyPlanRequest {
    pub plan_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimChatHallRedPacketResponse {
    pub message: ChatHallMessage,
    pub claim: ChatHallRedPacketClaim,
    pub available_balance_minor: i64,
}
