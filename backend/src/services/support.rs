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
            SupportMessage, SupportMessageAuthor, SupportMessageType, SupportPriority,
            SupportReplyRequest, UpdateSupportConversationRequest, UserSupportReplyRequest,
        },
        user::{AdminSummary, UserSummary},
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

#[derive(Clone)]
/// 在线客服会话仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct SupportRepository {
    inner: Arc<RwLock<SupportStore>>,
    persistence: Option<BusinessDatabase>,
}

/// 在线客服会话仓储，负责该模块数据读取、业务变更和持久化协调。
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

    /// 按当前仓储快照返回全部客服会话列表。
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

    /// 按业务标识读取单条记录，未命中时返回未找到错误。
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

    /// 将指定用户自己的客服会话标记为已读，只清理用户侧未读数。
    pub async fn mark_user_read(&self, id: &str, user_id: &str) -> ApiResult<SupportConversation> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?;
            let result = store.mark_user_read(id, user_id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
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

    /// 删除已解决的客服会话，处理中和等待处理的会话不能被直接移除。
    pub async fn delete_resolved(&self, id: &str) -> ApiResult<SupportConversation> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?;
            let result = store.delete_resolved(id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }
    /// 把当前仓储快照同步保存到持久化存储。
    async fn persist(&self, store: &SupportStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_support_store(persistence, store).await?;
        }

        Ok(())
    }

    /// 从数据库重新加载客服会话和消息快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_support_store(persistence).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("客服会话缓存刷新失败".to_string()))? = store;
        Ok(true)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// 在线客服会话运行时数据快照，用于内存模式和数据库持久化前的业务校验。
struct SupportStore {
    conversations: BTreeMap<String, SupportConversation>,
}

/// 从数据库加载在线客服会话运行时快照，空库时按模块规则初始化。
async fn load_support_store(database: &BusinessDatabase) -> ApiResult<SupportStore> {
    let pool = database.pool();
    let mut conversations = BTreeMap::new();

    for row in sqlx::query(
        "SELECT id, user_id, username, subject, status, priority, assigned_admin_id,
                assigned_admin_name, unread_count, user_unread_count, created_at, updated_at
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
        let user_unread_count: i32 = row
            .try_get("user_unread_count")
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
                user_unread_count: u16::try_from(user_unread_count)
                    .map_err(|_| ApiError::Internal("用户客服未读数量数据无效".to_string()))?,
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
        "SELECT id, conversation_id, author, author_id, author_name, message_type, content,
                image_url, created_at
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
                message_type: enum_from_string(
                    row.try_get("message_type")
                        .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?,
                )?,
                content: row
                    .try_get("content")
                    .map_err(|_| ApiError::Internal("客服消息数据读取失败".to_string()))?,
                image_url: row
                    .try_get("image_url")
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

/// 把在线客服会话运行时快照保存到数据库。
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
              assigned_admin_name, unread_count, user_unread_count, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
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
        .bind(i32::from(conversation.user_unread_count))
        .bind(&conversation.created_at)
        .bind(&conversation.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("客服会话数据保存失败".to_string()))?;

        for message in &conversation.messages {
            sqlx::query(
                "INSERT INTO support_messages
                 (id, conversation_id, author, author_id, author_name, message_type, content,
                  image_url, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(&message.id)
            .bind(&conversation.id)
            .bind(enum_to_string(&message.author)?)
            .bind(&message.author_id)
            .bind(&message.author_name)
            .bind(enum_to_string(&message.message_type)?)
            .bind(&message.content)
            .bind(&message.image_url)
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

/// 在线客服会话运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl SupportStore {
    /// 构建并返回种子数据。
    fn seeded() -> Self {
        let conversations = seed_conversations()
            .into_iter()
            .map(|conversation| (conversation.id.clone(), conversation))
            .collect();

        Self { conversations }
    }

    /// 按当前仓储快照返回全部客服会话列表。
    fn list(&self) -> Vec<SupportConversation> {
        sort_support_conversations(self.conversations.values().cloned().collect())
    }

    /// 返回指定用户自己的会话，避免用户端读取他人客服记录。
    fn list_for_user(&self, user_id: &str) -> Vec<SupportConversation> {
        sort_support_conversations(
            self.conversations
                .values()
                .filter(|conversation| conversation.user_id == user_id)
                .cloned()
                .collect(),
        )
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

    /// 用户打开会话后清零自己的未读数，不影响后台客服待处理未读数。
    fn mark_user_read(&mut self, id: &str, user_id: &str) -> ApiResult<SupportConversation> {
        let conversation = self
            .conversations
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("support conversation `{id}` not found")))?;
        if conversation.user_id != user_id.trim() {
            return Err(ApiError::NotFound(format!(
                "support conversation `{id}` not found"
            )));
        }

        conversation.user_unread_count = 0;
        Ok(conversation.clone())
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
            user_unread_count: 0,
            created_at: now.clone(),
            updated_at: now.clone(),
            messages: vec![SupportMessage {
                id: message_id(&id, 1),
                author: SupportMessageAuthor::User,
                author_id: user.id.clone(),
                author_name: user.username.clone(),
                message_type: SupportMessageType::Text,
                content,
                image_url: None,
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
        let admin_id = required_trimmed(request.admin_id.clone(), "support reply admin id")?;
        let admin = admins
            .iter()
            .find(|admin| admin.id == admin_id)
            .ok_or_else(|| ApiError::NotFound(format!("admin `{admin_id}` not found")))?;
        let (message_type, content, image_url) = normalize_reply_content(request)?;
        let next_index = conversation.messages.len() + 1;
        let now = current_time_label();

        conversation.messages.push(SupportMessage {
            id: message_id(id, next_index),
            author: SupportMessageAuthor::Admin,
            author_id: admin.id.clone(),
            author_name: admin.username.clone(),
            message_type,
            content,
            image_url,
            created_at: now.clone(),
        });
        if conversation.assigned_admin_id.is_none() {
            conversation.assigned_admin_id = Some(admin.id.clone());
            conversation.assigned_admin_name = Some(admin.username.clone());
        }
        conversation.unread_count = 0;
        conversation.user_unread_count = conversation.user_unread_count.saturating_add(1);
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

        let (message_type, content, image_url) = normalize_user_reply_content(request)?;
        let next_index = conversation.messages.len() + 1;
        let now = current_time_label();

        conversation.messages.push(SupportMessage {
            id: message_id(id, next_index),
            author: SupportMessageAuthor::User,
            author_id: user.id.clone(),
            author_name: user.username.clone(),
            message_type,
            content,
            image_url,
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
        conversation.user_unread_count = 0;
        conversation.updated_at = now;

        Ok(conversation.clone())
    }

    /// 删除已解决会话并返回被删除的快照，便于路由层发布删除实时事件。
    fn delete_resolved(&mut self, id: &str) -> ApiResult<SupportConversation> {
        let conversation =
            self.conversations.get(id).cloned().ok_or_else(|| {
                ApiError::NotFound(format!("support conversation `{id}` not found"))
            })?;
        if conversation.status != SupportConversationStatus::Resolved {
            return Err(ApiError::BadRequest(
                "只有已解决的客服会话可以删除".to_string(),
            ));
        }

        self.conversations.remove(id);
        Ok(conversation)
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

/// 标准化用户端客服回复，文本要求内容非空，图片要求提供 http/https 链接。
fn normalize_user_reply_content(
    request: UserSupportReplyRequest,
) -> ApiResult<(SupportMessageType, String, Option<String>)> {
    normalize_support_message_content(
        request.content,
        request.message_type,
        request.image_url,
        "客服消息内容不能为空",
        "客服图片链接不能为空",
        "客服图片链接必须是 http 或 https 地址",
    )
}

/// 标准化后台客服回复，文本消息要求内容非空，图片消息要求提供 http/https 图片链接。
fn normalize_reply_content(
    request: SupportReplyRequest,
) -> ApiResult<(SupportMessageType, String, Option<String>)> {
    normalize_support_message_content(
        request.content,
        request.message_type,
        request.image_url,
        "客服回复内容不能为空",
        "客服图片链接不能为空",
        "客服图片链接必须是 http 或 https 地址",
    )
}

/// 标准化客服消息内容，统一约束用户端和后台端图片消息契约。
fn normalize_support_message_content(
    content: Option<String>,
    message_type: Option<SupportMessageType>,
    image_url: Option<String>,
    text_required_message: &str,
    image_required_message: &str,
    image_format_message: &str,
) -> ApiResult<(SupportMessageType, String, Option<String>)> {
    let message_type = message_type.unwrap_or(SupportMessageType::Text);
    let content = content
        .map(|value| value.trim().to_string())
        .unwrap_or_default();

    match message_type {
        SupportMessageType::Text => {
            if content.is_empty() {
                return Err(ApiError::BadRequest(text_required_message.to_string()));
            }
            Ok((SupportMessageType::Text, content, None))
        }
        SupportMessageType::Image => {
            let image_url = image_url
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ApiError::BadRequest(image_required_message.to_string()))?;
            if !is_http_image_url(&image_url) {
                return Err(ApiError::BadRequest(image_format_message.to_string()));
            }
            Ok((SupportMessageType::Image, content, Some(image_url)))
        }
    }
}

/// 校验图片链接是否适合作为客服图片消息保存。
fn is_http_image_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

/// 返回当前时间的展示文本。
fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 按会话 ID 和序号生成客服消息 ID。
fn message_id(conversation_id: &str, index: usize) -> String {
    format!("{conversation_id}-M{index:03}")
}

/// 返回内置客服会话种子数据。
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
            user_unread_count: 1,
            created_at: "2026-06-02 09:20:00".to_string(),
            updated_at: "2026-06-02 09:22:00".to_string(),
            messages: vec![
                SupportMessage {
                    id: "CS-10001-M001".to_string(),
                    author: SupportMessageAuthor::User,
                    author_id: "U10001".to_string(),
                    author_name: "demo_user".to_string(),
                    message_type: SupportMessageType::Text,
                    content: "我的中奖订单还没有看到派奖。".to_string(),
                    image_url: None,
                    created_at: "2026-06-02 09:20:00".to_string(),
                },
                SupportMessage {
                    id: "CS-10001-M002".to_string(),
                    author: SupportMessageAuthor::Admin,
                    author_id: "A10002".to_string(),
                    author_name: "locked_admin".to_string(),
                    message_type: SupportMessageType::Text,
                    content: "已经为您核对订单，请稍后查看资金流水。".to_string(),
                    image_url: None,
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
            user_unread_count: 0,
            created_at: "2026-06-02 10:05:00".to_string(),
            updated_at: "2026-06-02 10:05:00".to_string(),
            messages: vec![SupportMessage {
                id: "CS-10002-M001".to_string(),
                author: SupportMessageAuthor::User,
                author_id: "U10004".to_string(),
                author_name: "risk_watch".to_string(),
                message_type: SupportMessageType::Text,
                content: "账号为什么不能继续投注？".to_string(),
                image_url: None,
                created_at: "2026-06-02 10:05:00".to_string(),
            }],
        },
    ]
}

/// 按客服处理优先级排序会话：后台未读优先，其次最近消息或更新时间最新优先。
fn sort_support_conversations(
    mut conversations: Vec<SupportConversation>,
) -> Vec<SupportConversation> {
    conversations.sort_by(|left, right| {
        let left_unread = left.unread_count > 0;
        let right_unread = right.unread_count > 0;

        right_unread
            .cmp(&left_unread)
            .then_with(|| support_activity_time(right).cmp(&support_activity_time(left)))
            .then_with(|| right.id.cmp(&left.id))
    });
    conversations
}

/// 取会话最后活跃时间，优先使用最后一条消息时间，没有消息时退回更新时间和创建时间。
fn support_activity_time(conversation: &SupportConversation) -> &str {
    conversation
        .messages
        .last()
        .map(|message| message.created_at.as_str())
        .unwrap_or_else(|| {
            if conversation.updated_at.trim().is_empty() {
                conversation.created_at.as_str()
            } else {
                conversation.updated_at.as_str()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::user::{UserKind, UserStatus},
        services::access::AccessRepository,
    };
    /// 验证客服会话创建、状态更新和回复流程。
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
        assert_eq!(created.user_unread_count, 0);

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
                    content: Some("请查看最新流水。".to_string()),
                    image_url: None,
                    message_type: None,
                },
                &access.admins,
            )
            .await
            .expect("reply can be added");
        assert_eq!(replied.messages.len(), 2);
        assert_eq!(replied.unread_count, 0);
        assert_eq!(replied.user_unread_count, 1);
    }
    /// 验证未读且最近更新的客服会话优先展示。
    #[tokio::test]
    async fn support_repository_lists_unread_recent_conversations_first() {
        let support = SupportRepository::memory_seeded();
        let conversations = support
            .list()
            .await
            .expect("support conversations can be listed");

        assert_eq!(
            conversations
                .iter()
                .map(|conversation| conversation.id.as_str())
                .collect::<Vec<_>>(),
            vec!["CS-10002", "CS-10001"]
        );
    }
    /// 验证未知用户不能创建客服会话。
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
                    avatar_url: String::new(),
                    contact_qq: String::new(),
                    kind: UserKind::Regular,
                    status: UserStatus::Active,
                    balance_minor: 0,
                    agent_id: None,
                    invite_code: "USER1".to_string(),
                    registration_location: crate::domain::user::UserRegistrationLocation::default(),
                    created_at: "2026-06-05 10:00:00".to_string(),
                }],
            )
            .await
            .expect_err("unknown user must be rejected");

        assert!(matches!(error, ApiError::NotFound(_)));
    }
    /// 验证不能把会话分配给未知管理员。
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
    /// 验证空文本客服回复会被拒绝。
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
                    content: Some(" ".to_string()),
                    image_url: None,
                    message_type: None,
                },
                &access.admins,
            )
            .await
            .expect_err("empty reply must be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }
    /// 验证客服管理员可以发送图片回复。
    #[tokio::test]
    async fn support_repository_allows_admin_image_reply() {
        let support = SupportRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");

        let replied = support
            .reply(
                "CS-10001",
                SupportReplyRequest {
                    admin_id: "A10001".to_string(),
                    content: Some("这是充值凭证截图。".to_string()),
                    image_url: Some("https://oss.example.test/support-proof.png".to_string()),
                    message_type: Some(SupportMessageType::Image),
                },
                &access.admins,
            )
            .await
            .expect("image reply can be added");
        let message = replied.messages.last().expect("reply message exists");

        assert_eq!(message.message_type, SupportMessageType::Image);
        assert_eq!(
            message.image_url.as_deref(),
            Some("https://oss.example.test/support-proof.png")
        );
        assert_eq!(message.content, "这是充值凭证截图。");
        assert_eq!(replied.unread_count, 0);
        assert_eq!(replied.user_unread_count, 2);
    }
    /// 验证用户可以继续自己的客服会话。
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
                    content: Some("我再补充一条充值凭证。".to_string()),
                    image_url: None,
                    message_type: None,
                },
            )
            .await
            .expect("user can reply owned conversation");

        assert_eq!(updated.messages.len(), 3);
        assert_eq!(updated.unread_count, 2);
        assert_eq!(updated.user_unread_count, 0);
        assert_eq!(support.list_for_user("U10001").await.unwrap().len(), 1);
    }
    /// 验证用户可以发送图片客服消息。
    #[tokio::test]
    async fn support_repository_allows_user_image_reply() {
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
                    content: Some("用户上传充值截图。".to_string()),
                    image_url: Some("https://oss.example.test/user-proof.png".to_string()),
                    message_type: Some(SupportMessageType::Image),
                },
            )
            .await
            .expect("user image reply can be added");
        let message = updated.messages.last().expect("user image message exists");

        assert_eq!(message.message_type, SupportMessageType::Image);
        assert_eq!(
            message.image_url.as_deref(),
            Some("https://oss.example.test/user-proof.png")
        );
        assert_eq!(message.content, "用户上传充值截图。");
        assert_eq!(updated.unread_count, 2);
        assert_eq!(updated.user_unread_count, 0);
    }
    /// 验证客服端读取后会标记用户消息已读。
    #[tokio::test]
    async fn support_repository_marks_user_messages_read() {
        let support = SupportRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");

        support
            .reply(
                "CS-10001",
                SupportReplyRequest {
                    admin_id: "A10001".to_string(),
                    content: Some("请查看客服回复。".to_string()),
                    image_url: None,
                    message_type: None,
                },
                &access.admins,
            )
            .await
            .expect("reply can be added");

        let before_read = support
            .get_for_user("CS-10001", "U10001")
            .await
            .expect("conversation can load");
        assert!(before_read.user_unread_count > 0);

        let after_read = support
            .mark_user_read("CS-10001", "U10001")
            .await
            .expect("user can mark own support conversation read");

        assert_eq!(after_read.user_unread_count, 0);
        assert_eq!(after_read.unread_count, before_read.unread_count);
    }
    /// 验证用户回复会重新打开待处理客服会话。
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
                    content: Some("我已经补充付款凭证，请继续处理。".to_string()),
                    image_url: None,
                    message_type: None,
                },
            )
            .await
            .expect("user can reopen pending conversation");

        assert_eq!(updated.status, SupportConversationStatus::Open);
        assert_eq!(updated.unread_count, unread_before + 1);
    }
    /// 验证只有已解决客服会话允许删除。
    #[tokio::test]
    async fn support_repository_deletes_only_resolved_conversations() {
        let support = SupportRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");

        let open_error = support
            .delete_resolved("CS-10001")
            .await
            .expect_err("open conversation cannot be deleted");
        assert!(matches!(open_error, ApiError::BadRequest(_)));

        support
            .update(
                "CS-10001",
                UpdateSupportConversationRequest {
                    status: SupportConversationStatus::Resolved,
                    priority: SupportPriority::Normal,
                    assigned_admin_id: Some("A10001".to_string()),
                },
                &access.admins,
            )
            .await
            .expect("conversation can be resolved");

        let deleted = support
            .delete_resolved("CS-10001")
            .await
            .expect("resolved conversation can be deleted");
        assert_eq!(deleted.id, "CS-10001");
        assert!(matches!(
            support.get("CS-10001").await,
            Err(ApiError::NotFound(_))
        ));
    }
    /// 验证用户不能回复他人的客服会话。
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
                    content: Some("这不是我的会话。".to_string()),
                    image_url: None,
                    message_type: None,
                },
            )
            .await
            .expect_err("user cannot reply other conversation");

        assert!(matches!(error, ApiError::NotFound(_)));
    }
}
