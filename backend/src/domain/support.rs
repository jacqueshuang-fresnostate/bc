//! 在线客服领域模型，定义会话、消息、优先级与状态

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 客服会话状态，描述会话是否处理中、待用户回复、已解决或关闭。
pub enum SupportConversationStatus {
    Open,
    Pending,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 客服会话优先级，用于后台客服筛选和提醒。
pub enum SupportPriority {
    Normal,
    Urgent,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 客服消息作者类型，区分用户、后台客服和系统消息。
pub enum SupportMessageAuthor {
    User,
    Admin,
    System,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单条客服消息，保存作者快照和消息内容。
pub struct SupportMessage {
    pub id: String,
    pub author: SupportMessageAuthor,
    pub author_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 客服会话完整实体，包含分配客服、未读数和消息列表。
pub struct SupportConversation {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub subject: String,
    pub status: SupportConversationStatus,
    pub priority: SupportPriority,
    pub assigned_admin_id: Option<String>,
    pub assigned_admin_name: Option<String>,
    pub unread_count: u16,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<SupportMessage>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台为用户创建客服会话时提交的首条消息。
pub struct CreateSupportConversationRequest {
    pub id: String,
    pub user_id: String,
    pub subject: String,
    pub priority: SupportPriority,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台更新客服会话状态、优先级和分配客服时提交的请求。
pub struct UpdateSupportConversationRequest {
    pub status: SupportConversationStatus,
    pub priority: SupportPriority,
    pub assigned_admin_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台客服回复会话时提交的客服编号和内容。
pub struct SupportReplyRequest {
    pub admin_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户端继续回复自己客服会话时提交的内容。
pub struct UserSupportReplyRequest {
    pub content: String,
}
