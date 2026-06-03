use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        support::{
            CreateSupportConversationRequest, SupportConversation, SupportConversationStatus,
            SupportMessage, SupportMessageAuthor, SupportPriority, SupportReplyRequest,
            UpdateSupportConversationRequest,
        },
        user::{AdminSummary, UserSummary},
    },
    error::{ApiError, ApiResult},
};

use super::state_document::StateDocumentRepository;

const SUPPORT_STATE_NAMESPACE: &str = "support";

#[derive(Clone)]
pub struct SupportRepository {
    inner: Arc<RwLock<SupportStore>>,
    persistence: Option<StateDocumentRepository>,
}

impl SupportRepository {
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SupportStore::seeded())),
            persistence: None,
        }
    }

    pub async fn persistent(persistence: StateDocumentRepository) -> ApiResult<Self> {
        let store = persistence
            .load_or_seed(SUPPORT_STATE_NAMESPACE, SupportStore::seeded())
            .await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    pub async fn list(&self) -> ApiResult<Vec<SupportConversation>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    pub async fn get(&self, id: &str) -> ApiResult<SupportConversation> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("support store lock poisoned".to_string()))?
            .get(id)
    }

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

    async fn persist(&self, store: &SupportStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            persistence.save(SUPPORT_STATE_NAMESPACE, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SupportStore {
    conversations: BTreeMap<String, SupportConversation>,
}

impl SupportStore {
    fn seeded() -> Self {
        let conversations = seed_conversations()
            .into_iter()
            .map(|conversation| (conversation.id.clone(), conversation))
            .collect();

        Self { conversations }
    }

    fn list(&self) -> Vec<SupportConversation> {
        self.conversations.values().cloned().collect()
    }

    fn get(&self, id: &str) -> ApiResult<SupportConversation> {
        self.conversations
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("support conversation `{id}` not found")))
    }

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
}

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

fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn message_id(conversation_id: &str, index: usize) -> String {
    format!("{conversation_id}-M{index:03}")
}

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
}
