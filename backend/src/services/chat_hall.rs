//! 手机端公共聊天大厅仓储，负责消息校验、历史列表和数据库持久化。

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use chrono::Local;
use sqlx::Row;

use crate::{
    domain::{
        chat_hall::{ChatHallMessage, CreateChatHallMessageRequest},
        user::UserSummary,
    },
    error::{ApiError, ApiResult},
};

use super::business_database::BusinessDatabase;

const CHAT_HALL_HISTORY_LIMIT: usize = 200;
const CHAT_HALL_LIST_LIMIT: usize = 100;
const CHAT_HALL_MESSAGE_MAX_LENGTH: usize = 500;

#[derive(Clone)]
pub struct ChatHallRepository {
    inner: Arc<RwLock<ChatHallStore>>,
    persistence: Option<BusinessDatabase>,
}

impl ChatHallRepository {
    /// 创建空的内存聊天大厅仓储，供未配置数据库的本地开发使用。
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ChatHallStore::default())),
            persistence: None,
        }
    }

    /// 从数据库加载聊天大厅历史消息，并启用持久化保存。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_chat_hall_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回最近的聊天大厅消息，按发送时间正序展示。
    pub async fn list(&self) -> ApiResult<Vec<ChatHallMessage>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("聊天大厅数据锁读取失败".to_string()))
            .map(|store| store.list())
    }

    /// 发送一条聊天大厅消息，保存后由路由层负责广播实时事件。
    pub async fn send(
        &self,
        user: &UserSummary,
        request: CreateChatHallMessageRequest,
    ) -> ApiResult<ChatHallMessage> {
        let (message, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("聊天大厅数据锁写入失败".to_string()))?;
            let message = store.send(user, request)?;
            (message, store.clone())
        };
        self.persist(&snapshot).await?;

        Ok(message)
    }

    /// PostgreSQL 模式下把当前最近消息快照写入数据库。
    async fn persist(&self, store: &ChatHallStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_chat_hall_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
struct ChatHallStore {
    messages: VecDeque<ChatHallMessage>,
    next_sequence: u64,
}

impl ChatHallStore {
    /// 返回最近消息，超过展示上限时仅返回末尾一段历史。
    fn list(&self) -> Vec<ChatHallMessage> {
        let mut messages = self
            .messages
            .iter()
            .rev()
            .take(CHAT_HALL_LIST_LIMIT)
            .cloned()
            .collect::<Vec<_>>();
        messages.reverse();
        messages
    }

    /// 校验用户输入并追加一条新的聊天消息。
    fn send(
        &mut self,
        user: &UserSummary,
        request: CreateChatHallMessageRequest,
    ) -> ApiResult<ChatHallMessage> {
        let content = normalize_chat_hall_content(request.content)?;
        self.next_sequence = self.next_sequence.saturating_add(1);
        let message = ChatHallMessage {
            id: chat_hall_message_id(self.next_sequence),
            user_id: user.id.clone(),
            username: user.username.clone(),
            content,
            created_at: current_time_label(),
        };
        self.messages.push_back(message.clone());
        self.trim_history();

        Ok(message)
    }

    /// 限制内存和数据库中保留的大厅历史消息数量，避免无限增长。
    fn trim_history(&mut self) {
        while self.messages.len() > CHAT_HALL_HISTORY_LIMIT {
            self.messages.pop_front();
        }
    }
}

/// 从数据库读取聊天大厅消息，并恢复下一条消息的序号。
async fn load_chat_hall_store(database: &BusinessDatabase) -> ApiResult<ChatHallStore> {
    let rows = sqlx::query(
        "SELECT id, user_id, username, content, created_at
         FROM chat_hall_messages
         ORDER BY created_at ASC, id ASC",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?;

    let mut store = ChatHallStore::default();
    for row in rows {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?;
        if let Some(sequence) = sequence_from_chat_hall_message_id(&id) {
            store.next_sequence = store.next_sequence.max(sequence);
        }
        store.messages.push_back(ChatHallMessage {
            id,
            user_id: row
                .try_get("user_id")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            username: row
                .try_get("username")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            content: row
                .try_get("content")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
            created_at: row
                .try_get("created_at")
                .map_err(|_| ApiError::Internal("聊天大厅消息数据读取失败".to_string()))?,
        });
    }
    store.trim_history();

    Ok(store)
}

/// 保存聊天大厅最近消息快照；大厅消息只保留近 200 条。
async fn save_chat_hall_store(database: &BusinessDatabase, store: &ChatHallStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("聊天大厅事务开启失败".to_string()))?;

    sqlx::query("DELETE FROM chat_hall_messages")
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("聊天大厅历史消息清理失败".to_string()))?;

    for message in &store.messages {
        sqlx::query(
            "INSERT INTO chat_hall_messages
             (id, user_id, username, content, created_at)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&message.id)
        .bind(&message.user_id)
        .bind(&message.username)
        .bind(&message.content)
        .bind(&message.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("聊天大厅消息保存失败".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("聊天大厅事务提交失败".to_string()))
}

/// 去除首尾空白并限制大厅消息长度。
fn normalize_chat_hall_content(content: String) -> ApiResult<String> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(ApiError::BadRequest("聊天内容不能为空".to_string()));
    }
    if content.chars().count() > CHAT_HALL_MESSAGE_MAX_LENGTH {
        return Err(ApiError::BadRequest(format!(
            "聊天内容不能超过 {CHAT_HALL_MESSAGE_MAX_LENGTH} 个字符"
        )));
    }

    Ok(content)
}

/// 生成聊天大厅消息编号，便于数据库恢复时解析最大序号。
fn chat_hall_message_id(sequence: u64) -> String {
    format!("CHM-{sequence:012}")
}

/// 从聊天大厅消息编号中解析序号。
fn sequence_from_chat_hall_message_id(id: &str) -> Option<u64> {
    id.strip_prefix("CHM-")?.parse().ok()
}

/// 返回与系统其他业务字段一致的本地时间文本。
fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{UserKind, UserStatus};

    #[test]
    /// 验证聊天大厅发送消息会修剪内容并按最近消息正序返回。
    fn chat_hall_store_sends_and_lists_messages() {
        let mut store = ChatHallStore::default();
        let user = test_user("U90001", "alice");

        let message = store
            .send(
                &user,
                CreateChatHallMessageRequest {
                    content: "  大家好  ".to_string(),
                },
            )
            .unwrap();

        assert_eq!(message.id, "CHM-000000000001");
        assert_eq!(message.user_id, "U90001");
        assert_eq!(message.username, "alice");
        assert_eq!(message.content, "大家好");
        assert_eq!(store.list(), vec![message]);
    }

    #[test]
    /// 验证聊天大厅拒绝空消息和超长消息。
    fn chat_hall_store_rejects_invalid_content() {
        let mut store = ChatHallStore::default();
        let user = test_user("U90001", "alice");

        assert!(matches!(
            store.send(
                &user,
                CreateChatHallMessageRequest {
                    content: "  ".to_string(),
                },
            ),
            Err(ApiError::BadRequest(_))
        ));
        assert!(matches!(
            store.send(
                &user,
                CreateChatHallMessageRequest {
                    content: "字".repeat(CHAT_HALL_MESSAGE_MAX_LENGTH + 1),
                },
            ),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    /// 验证聊天大厅只保留最近 200 条历史消息。
    fn chat_hall_store_keeps_recent_history() {
        let mut store = ChatHallStore::default();
        let user = test_user("U90001", "alice");

        for index in 0..=CHAT_HALL_HISTORY_LIMIT {
            store
                .send(
                    &user,
                    CreateChatHallMessageRequest {
                        content: format!("消息 {index}"),
                    },
                )
                .unwrap();
        }

        assert_eq!(store.messages.len(), CHAT_HALL_HISTORY_LIMIT);
        assert_eq!(store.messages.front().unwrap().content, "消息 1");
        assert_eq!(
            store.messages.back().unwrap().id,
            chat_hall_message_id((CHAT_HALL_HISTORY_LIMIT + 1) as u64)
        );
    }

    fn test_user(id: &str, username: &str) -> UserSummary {
        UserSummary {
            id: id.to_string(),
            username: username.to_string(),
            email: None,
            kind: UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id: None,
            invite_code: "INVITE1".to_string(),
        }
    }
}
