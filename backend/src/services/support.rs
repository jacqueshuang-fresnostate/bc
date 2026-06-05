//! 在线客服领域模型，定义会话、消息、优先级与状态

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::{
        support::{
            CreateSupportConversationRequest, SupportConversation, SupportConversationStatus,
            SupportMessage, SupportMessageAuthor, SupportPriority, SupportReplyRequest,
            UpdateSupportConversationRequest, UserSupportReplyRequest,
        },
        user::{AdminSummary, UserSummary},
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

#[derive(Clone)]
pub struct SupportRepository {
    inner: Arc<RwLock<SupportStore>>,
    persistence: Option<BusinessDatabase>,
}

impl SupportRepository {
    /// 返回带内置种子数据的内存仓储实例。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SupportStore::seeded())),
            persistence: None,
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_support_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回完整列表。
    pub async fn list(&self) -> ApiResult<Vec<SupportConversation>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 返回指定用户自己的客服会话列表。
    pub async fn list_for_user(&self, user_id: &str) -> ApiResult<Vec<SupportConversation>> {
        let user_id = user_id.trim();
        if user_id.is_empty() {
            return Err(ApiError::BadRequest(
                "support user id is required".to_string(),
            ));
        }

        Ok(self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?
            .list_for_user(user_id))
    }

    /// 按 ID 查询单条记录。
    pub async fn get(&self, id: &str) -> ApiResult<SupportConversation> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?
            .get(id)
    }

    /// 按 ID 查询指定用户自己的客服会话。
    pub async fn get_for_user(&self, id: &str, user_id: &str) -> ApiResult<SupportConversation> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?
            .get_for_user(id, user_id)
    }

    /// 校验入参并创建一条新记录。
    pub async fn create(
        &self,
        request: CreateSupportConversationRequest,
        users: &[UserSummary],
    ) -> ApiResult<SupportConversation> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?;
            let result = store.create(request, users)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 更新现有记录并持久化变更。
    pub async fn update(
        &self,
        id: &str,
        request: UpdateSupportConversationRequest,
        admins: &[AdminSummary],
    ) -> ApiResult<SupportConversation> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?;
            let result = store.update(id, request, admins)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 追加客服回复并更新会话。
    pub async fn reply(
        &self,
        id: &str,
        request: SupportReplyRequest,
        admins: &[AdminSummary],
    ) -> ApiResult<SupportConversation> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?;
            let result = store.reply(id, request, admins)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 追加用户消息，用于用户端继续客服会话和客服直充沟通。
    pub async fn user_reply(
        &self,
        id: &str,
        user: &UserSummary,
        request: UserSupportReplyRequest,
    ) -> ApiResult<SupportConversation> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?;
            let result = store.user_reply(id, user, request)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    async fn persist(&self, store: &SupportStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_support_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SupportStore {
    conversations: BTreeMap<String, SupportConversation>,
}

async fn load_support_store(database: &BusinessDatabase) -> ApiResult<SupportStore> {
    let pool = database.pool();
    let mut conversations = BTreeMap::new();

    for row in sqlx::query(
        "SELECT id, user_id, username, subject, status, priority, assigned_admin_id,
                assigned_admin_name, unread_count, created_at, updated_at
         FROM support_conversations
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?;
        let unread_count: i32 = row
            .try_get("unread_count")
            .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?;
        conversations.insert(
            id.clone(),
            SupportConversation {
                id,
                user_id: row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                username: row
                    .try_get("username")
                    .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                subject: row
                    .try_get("subject")
                    .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                )?,
                priority: enum_from_string(
                    row.try_get("priority")
                        .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                )?,
                assigned_admin_id: row
                    .try_get("assigned_admin_id")
                    .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                assigned_admin_name: row
                    .try_get("assigned_admin_name")
                    .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                unread_count: u16::try_from(unread_count)
                    .map_err(|_| ApiError::Internal("客服未读数量数据无效".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|_| ApiError::Internal("客服会话数据读取失败".to_string()))?,
                messages: Vec::new(),
            },
        );
    }

    for row in sqlx::query(
        "SELECT id, conversation_id, author, author_id, author_name, content, created_at
         FROM support_messages
         ORDER BY conversation_id ASC, id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?
    {
        let conversation_id: String = row
            .try_get("conversation_id")
            .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?;
        if let Some(conversation) = conversations.get_mut(&conversation_id) {
            conversation.messages.push(SupportMessage {
                id: row
                    .try_get("id")
                    .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?,
                author: enum_from_string(
                    row.try_get("author")
                        .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?,
                )?,
                author_id: row
                    .try_get("author_id")
                    .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?,
                author_name: row
                    .try_get("author_name")
                    .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?,
                content: row
                    .try_get("content")
                    .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?,
            });
        }
    }

    if conversations.is_empty() {
        let seeded = SupportStore::seeded();
        save_support_store(database, &seeded).await?;
        return Ok(seeded);
    }

    Ok(SupportStore { conversations })
}

async fn save_support_store(database: &BusinessDatabase, store: &SupportStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("客服事务开启失败".to_string()))?;

    for table in ["support_messages", "support_conversations"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("客服数据清理失败".to_string()))?;
    }

    for conversation in store.conversations.values() {
        sqlx::query(
            "INSERT INTO support_conversations
             (id, user_id, username, subject, status, priority, assigned_admin_id,
              assigned_admin_name, unread_count, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&conversation.id)
        .bind(&conversation.user_id)
        .bind(&conversation.username)
        .bind(&conversation.subject)
        .bind(enum_to_string(&conversation.status)?)
        .bind(enum_to_string(&conversation.priority)?)
        .bind(&conversation.assigned_admin_id)
        .bind(&conversation.assigned_admin_name)
        .bind(i32::from(conversation.unread_count))
        .bind(&conversation.created_at)
        .bind(&conversation.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("客服会话数据保存失败".to_string()))?;

        for message in &conversation.messages {
            sqlx::query(
                "INSERT INTO support_messages
                 (id, conversation_id, author, author_id, author_name, content, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&message.id)
            .bind(&conversation.id)
            .bind(enum_to_string(&message.author)?)
            .bind(&message.author_id)
            .bind(&message.author_name)
            .bind(&message.content)
            .bind(&message.created_at)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("客服消息数据保存失败".to_string()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("客服事务提交失败".to_string()))
}

impl SupportStore {
    /// 构建并返回种子数据。
    fn seeded() -> Self {
        let conversations = seed_conversations()
            .into_iter()
            .map(|conversation| (conversation.id.clone(), conversation))
            .collect();

        Self { conversations }
    }

    /// 返回完整数据列表。
    fn list(&self) -> Vec<SupportConversation> {
        self.conversations.values().cloned().collect()
    }

    /// 返回指定用户自己的会话，避免用户端读取他人客服记录。
    fn list_for_user(&self, user_id: &str) -> Vec<SupportConversation> {
        self.conversations
            .values()
            .filter(|conversation| conversation.user_id == user_id)
            .cloned()
            .collect()
    }

    /// 按标识查询并返回单条记录。
    fn get(&self, id: &str) -> ApiResult<SupportConversation> {
        self.conversations
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("support conversation `{id}` not found")))
    }

    /// 查询指定用户自己的单条会话，归属不匹配时按不存在处理。
    fn get_for_user(&self, id: &str, user_id: &str) -> ApiResult<SupportConversation> {
        let conversation = self.get(id)?;
        if conversation.user_id != user_id.trim() {
            return Err(ApiError::NotFound(format!(
                "support conversation `{id}` not found"
            )));
        }

        Ok(conversation)
    }

    /// 校验入参并创建新记录。
    fn create(
        &mut self,
        request: CreateSupportConversationRequest,
        users: &[UserSummary],
    ) -> ApiResult<SupportConversation> {
        let id = required_trimmed(request.id, "support conversation id")?;
        if self.conversations.contains_key(&id) {
            return Err(ApiError::Conflict(format!(
                "support conversation `{id}` already exists"
            )));
        }

        let user_id = required_trimmed(request.user_id, "support user id")?;
        let user = users
            .iter()
            .find(|user| user.id == user_id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{user_id}` not found")))?;
        let subject = required_trimmed(request.subject, "support subject")?;
        let content = required_trimmed(request.content, "support message content")?;
        let now = current_time_label();

        let conversation = SupportConversation {
            id: id.clone(),
            user_id: user.id.clone(),
            username: user.username.clone(),
            subject,
            status: SupportConversationStatus::Open,
            priority: request.priority,
            assigned_admin_id: None,
            assigned_admin_name: None,
            unread_count: 1,
            created_at: now.clone(),
            updated_at: now.clone(),
            messages: vec![SupportMessage {
                id: message_id(&id, 1),
                author: SupportMessageAuthor::User,
                author_id: user.id.clone(),
                author_name: user.username.clone(),
                content,
                created_at: now,
            }],
        };

        self.conversations.insert(id, conversation.clone());
        Ok(conversation)
    }

    /// 校验入参并更新指定记录。
    fn update(
        &mut self,
        id: &str,
        request: UpdateSupportConversationRequest,
        admins: &[AdminSummary],
    ) -> ApiResult<SupportConversation> {
        let conversation = self
            .conversations
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("support conversation `{id}` not found")))?;

        let assigned = normalize_assigned_admin(request.assigned_admin_id, admins)?;
        conversation.status = request.status;
        conversation.priority = request.priority;
        conversation.assigned_admin_id = assigned.as_ref().map(|admin| admin.id.clone());
        conversation.assigned_admin_name = assigned.map(|admin| admin.username.clone());
        if matches!(
            conversation.status,
            SupportConversationStatus::Resolved | SupportConversationStatus::Closed
        ) {
            conversation.unread_count = 0;
        }
        conversation.updated_at = current_time_label();

        Ok(conversation.clone())
    }

    /// 记录回复并持久化会话更新。
    fn reply(
        &mut self,
        id: &str,
        request: SupportReplyRequest,
        admins: &[AdminSummary],
    ) -> ApiResult<SupportConversation> {
        let conversation = self
            .conversations
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("support conversation `{id}` not found")))?;
        let admin_id = required_trimmed(request.admin_id, "support reply admin id")?;
        let admin = admins
            .iter()
            .find(|admin| admin.id == admin_id)
            .ok_or_else(|| ApiError::NotFound(format!("admin `{admin_id}` not found")))?;
        let content = required_trimmed(request.content, "support reply content")?;
        let next_index = conversation.messages.len() + 1;
        let now = current_time_label();

        conversation.messages.push(SupportMessage {
            id: message_id(id, next_index),
            author: SupportMessageAuthor::Admin,
            author_id: admin.id.clone(),
            author_name: admin.username.clone(),
            content,
            created_at: now.clone(),
        });
        if conversation.assigned_admin_id.is_none() {
            conversation.assigned_admin_id = Some(admin.id.clone());
            conversation.assigned_admin_name = Some(admin.username.clone());
        }
        conversation.unread_count = 0;
        conversation.updated_at = now;

        Ok(conversation.clone())
    }

    /// 记录用户消息并恢复待处理状态，供客服后台继续处理。
    fn user_reply(
        &mut self,
        id: &str,
        user: &UserSummary,
        request: UserSupportReplyRequest,
    ) -> ApiResult<SupportConversation> {
        let conversation = self
            .conversations
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("support conversation `{id}` not found")))?;
        if conversation.user_id != user.id {
            return Err(ApiError::NotFound(format!(
                "support conversation `{id}` not found"
            )));
        }

        let content = required_trimmed(request.content, "support user message content")?;
        let next_index = conversation.messages.len() + 1;
        let now = current_time_label();

        conversation.messages.push(SupportMessage {
            id: message_id(id, next_index),
            author: SupportMessageAuthor::User,
            author_id: user.id.clone(),
            author_name: user.username.clone(),
            content,
            created_at: now.clone(),
        });
        if matches!(
            conversation.status,
            SupportConversationStatus::Pending
                | SupportConversationStatus::Resolved
                | SupportConversationStatus::Closed
        ) {
            conversation.status = SupportConversationStatus::Open;
        }
        conversation.unread_count = conversation.unread_count.saturating_add(1);
        conversation.updated_at = now;

        Ok(conversation.clone())
    }
}

/// 标准化输入并返回规范值。
fn normalize_assigned_admin(
    admin_id: Option<String>,
    admins: &[AdminSummary],
) -> ApiResult<Option<AdminSummary>> {
    let Some(admin_id) = admin_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    admins
        .iter()
        .find(|admin| admin.id == admin_id)
        .cloned()
        .map(Some)
        .ok_or_else(|| ApiError::NotFound(format!("admin `{admin_id}` not found")))
}

/// 去除空白并校验必填字段。
fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

/// 返回当前时间的展示文本。
fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 处理 message_id 的具体内部流程。
fn message_id(conversation_id: &str, index: usize) -> String {
    format!("{conversation_id}-M{index:03}")
}

/// 返回内置种子或测试数据。
fn seed_conversations() -> Vec<SupportConversation> {
    vec![
        SupportConversation {
            id: "CS-10001".to_string(),
            user_id: "U10001".to_string(),
            username: "demo_user".to_string(),
            subject: "订单派奖咨询".to_string(),
            status: SupportConversationStatus::Open,
            priority: SupportPriority::Normal,
            assigned_admin_id: Some("A10002".to_string()),
            assigned_admin_name: Some("locked_admin".to_string()),
            unread_count: 1,
            created_at: "2026-06-02 09:20:00".to_string(),
            updated_at: "2026-06-02 09:22:00".to_string(),
            messages: vec![
                SupportMessage {
                    id: "CS-10001-M001".to_string(),
                    author: SupportMessageAuthor::User,
                    author_id: "U10001".to_string(),
                    author_name: "demo_user".to_string(),
                    content: "我的中奖订单还没有看到派奖。".to_string(),
                    created_at: "2026-06-02 09:20:00".to_string(),
                },
                SupportMessage {
                    id: "CS-10001-M002".to_string(),
                    author: SupportMessageAuthor::Admin,
                    author_id: "A10002".to_string(),
                    author_name: "locked_admin".to_string(),
                    content: "已经为您核对订单，请稍后查看资金流水。".to_string(),
                    created_at: "2026-06-02 09:22:00".to_string(),
                },
            ],
        },
        SupportConversation {
            id: "CS-10002".to_string(),
            user_id: "U10004".to_string(),
            username: "risk_watch".to_string(),
            subject: "账号状态咨询".to_string(),
            status: SupportConversationStatus::Pending,
            priority: SupportPriority::Urgent,
            assigned_admin_id: None,
            assigned_admin_name: None,
            unread_count: 2,
            created_at: "2026-06-02 10:05:00".to_string(),
            updated_at: "2026-06-02 10:05:00".to_string(),
            messages: vec![SupportMessage {
                id: "CS-10002-M001".to_string(),
                author: SupportMessageAuthor::User,
                author_id: "U10004".to_string(),
                author_name: "risk_watch".to_string(),
                content: "账号为什么不能继续投注？".to_string(),
                created_at: "2026-06-02 10:05:00".to_string(),
            }],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::user::{UserKind, UserStatus},
        services::access::AccessRepository,
    };

    #[tokio::test]
    async fn support_repository_creates_updates_and_replies() {
        let support = SupportRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");

        let created = support
            .create(
                CreateSupportConversationRequest {
                    id: " CS-NEW ".to_string(),
                    user_id: "U10001".to_string(),
                    subject: "充值咨询".to_string(),
                    priority: SupportPriority::Normal,
                    content: "充值多久到账？".to_string(),
                },
                &access.users,
            )
            .await
            .expect("conversation can be created");

        assert_eq!(created.id, "CS-NEW");
        assert_eq!(created.messages.len(), 1);

        let updated = support
            .update(
                "CS-NEW",
                UpdateSupportConversationRequest {
                    status: SupportConversationStatus::Pending,
                    priority: SupportPriority::Urgent,
                    assigned_admin_id: Some("A10001".to_string()),
                },
                &access.admins,
            )
            .await
            .expect("conversation can be updated");
        assert_eq!(updated.assigned_admin_name.as_deref(), Some("admin"));

        let replied = support
            .reply(
                "CS-NEW",
                SupportReplyRequest {
                    admin_id: "A10001".to_string(),
                    content: "请查看最新流水。".to_string(),
                },
                &access.admins,
            )
            .await
            .expect("reply can be added");
        assert_eq!(replied.messages.len(), 2);
        assert_eq!(replied.unread_count, 0);
    }

    #[tokio::test]
    async fn support_repository_rejects_unknown_user() {
        let support = SupportRepository::memory_seeded();
        let error = support
            .create(
                CreateSupportConversationRequest {
                    id: "CS-BAD".to_string(),
                    user_id: "missing".to_string(),
                    subject: "测试".to_string(),
                    priority: SupportPriority::Normal,
                    content: "测试".to_string(),
                },
                &[UserSummary {
                    id: "U1".to_string(),
                    username: "user".to_string(),
                    email: None,
                    kind: UserKind::Regular,
                    status: UserStatus::Active,
                    balance_minor: 0,
                    agent_id: None,
                    invite_code: "USER1".to_string(),
                }],
            )
            .await
            .expect_err("unknown user must be rejected");

        assert!(matches!(error, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn support_repository_rejects_unknown_admin_assignment() {
        let support = SupportRepository::memory_seeded();
        let error = support
            .update(
                "CS-10001",
                UpdateSupportConversationRequest {
                    status: SupportConversationStatus::Open,
                    priority: SupportPriority::Normal,
                    assigned_admin_id: Some("missing".to_string()),
                },
                &[],
            )
            .await
            .expect_err("unknown admin must be rejected");

        assert!(matches!(error, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn support_repository_rejects_empty_reply_content() {
        let support = SupportRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let error = support
            .reply(
                "CS-10001",
                SupportReplyRequest {
                    admin_id: "A10001".to_string(),
                    content: " ".to_string(),
                },
                &access.admins,
            )
            .await
            .expect_err("empty reply must be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn support_repository_allows_user_to_continue_owned_conversation() {
        let support = SupportRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let user = access
            .users
            .iter()
            .find(|user| user.id == "U10001")
            .cloned()
            .expect("seed user exists");

        let updated = support
            .user_reply(
                "CS-10001",
                &user,
                UserSupportReplyRequest {
                    content: "我再补充一条充值凭证。".to_string(),
                },
            )
            .await
            .expect("user can reply owned conversation");

        assert_eq!(updated.messages.len(), 3);
        assert_eq!(updated.unread_count, 2);
        assert_eq!(support.list_for_user("U10001").await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn support_repository_reopens_pending_conversation_when_user_replies() {
        let support = SupportRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let user = access
            .users
            .iter()
            .find(|user| user.id == "U10001")
            .cloned()
            .expect("seed user exists");
        let unread_before = support
            .get("CS-10001")
            .await
            .expect("conversation can load")
            .unread_count;
        support
            .update(
                "CS-10001",
                UpdateSupportConversationRequest {
                    status: SupportConversationStatus::Pending,
                    priority: SupportPriority::Normal,
                    assigned_admin_id: None,
                },
                &access.admins,
            )
            .await
            .expect("conversation can be moved to pending");

        let updated = support
            .user_reply(
                "CS-10001",
                &user,
                UserSupportReplyRequest {
                    content: "我已经补充付款凭证，请继续处理。".to_string(),
                },
            )
            .await
            .expect("user can reopen pending conversation");

        assert_eq!(updated.status, SupportConversationStatus::Open);
        assert_eq!(updated.unread_count, unread_before + 1);
    }

    #[tokio::test]
    async fn support_repository_rejects_user_reply_to_other_conversation() {
        let support = SupportRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let user = access
            .users
            .iter()
            .find(|user| user.id == "U10001")
            .cloned()
            .expect("seed user exists");

        let error = support
            .user_reply(
                "CS-10002",
                &user,
                UserSupportReplyRequest {
                    content: "这不是我的会话。".to_string(),
                },
            )
            .await
            .expect_err("user cannot reply other conversation");

        assert!(matches!(error, ApiError::NotFound(_)));
    }
}
