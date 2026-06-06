//! 手机端公共聊天大厅领域模型，定义大厅消息和发送请求。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatHallMessage {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateChatHallMessageRequest {
    pub content: String,
}
