use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SupportConversationStatus {
    Open,
    Pending,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SupportPriority {
    Normal,
    Urgent,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SupportMessageAuthor {
    User,
    Admin,
    System,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub struct CreateSupportConversationRequest {
    pub id: String,
    pub user_id: String,
    pub subject: String,
    pub priority: SupportPriority,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSupportConversationRequest {
    pub status: SupportConversationStatus,
    pub priority: SupportPriority,
    pub assigned_admin_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SupportReplyRequest {
    pub admin_id: String,
    pub content: String,
}
