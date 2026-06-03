use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        invite::{
            CreateInviteRecordRequest, InviteRecord, InviteStatus, UpdateInviteRecordRequest,
        },
        rebate::InvitePolicySummary,
        user::{UserKind, UserSummary},
    },
    error::{ApiError, ApiResult},
};

use super::state_document::StateDocumentRepository;

const INVITE_STATE_NAMESPACE: &str = "invites";

#[derive(Clone)]
pub struct InviteRepository {
    inner: Arc<RwLock<InviteStore>>,
    persistence: Option<StateDocumentRepository>,
}

impl InviteRepository {
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(InviteStore::seeded())),
            persistence: None,
        }
    }

    pub async fn persistent(persistence: StateDocumentRepository) -> ApiResult<Self> {
        let store = persistence
            .load_or_seed(INVITE_STATE_NAMESPACE, InviteStore::seeded())
            .await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    pub async fn list(&self) -> ApiResult<Vec<InviteRecord>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("invite store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    pub async fn get(&self, id: &str) -> ApiResult<InviteRecord> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("invite store lock poisoned".to_string()))?
            .get(id)
    }

    pub async fn create(
        &self,
        request: CreateInviteRecordRequest,
        users: &[UserSummary],
        policy: &InvitePolicySummary,
    ) -> ApiResult<InviteRecord> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("invite store lock poisoned".to_string()))?;
            let result = store.create(request, users, policy)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    pub async fn update(
        &self,
        id: &str,
        request: UpdateInviteRecordRequest,
    ) -> ApiResult<InviteRecord> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("invite store lock poisoned".to_string()))?;
            let result = store.update(id, request)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    async fn persist(&self, store: &InviteStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            persistence.save(INVITE_STATE_NAMESPACE, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct InviteStore {
    records: BTreeMap<String, InviteRecord>,
}

impl InviteStore {
    fn seeded() -> Self {
        let records = seed_invites()
            .into_iter()
            .map(|record| (record.id.clone(), record))
            .collect();

        Self { records }
    }

    fn list(&self) -> Vec<InviteRecord> {
        self.records.values().cloned().collect()
    }

    fn get(&self, id: &str) -> ApiResult<InviteRecord> {
        self.records
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("invite record `{id}` not found")))
    }

    fn create(
        &mut self,
        request: CreateInviteRecordRequest,
        users: &[UserSummary],
        policy: &InvitePolicySummary,
    ) -> ApiResult<InviteRecord> {
        let id = required_trimmed(request.id, "invite record id")?;
        if self.records.contains_key(&id) {
            return Err(ApiError::Conflict(format!(
                "invite record `{id}` already exists"
            )));
        }

        let inviter_user_id = required_trimmed(request.inviter_user_id, "inviter user id")?;
        let invitee_user_id = required_trimmed(request.invitee_user_id, "invitee user id")?;
        if inviter_user_id == invitee_user_id {
            return Err(ApiError::BadRequest(
                "inviter and invitee must be different users".to_string(),
            ));
        }

        let inviter = users
            .iter()
            .find(|user| user.id == inviter_user_id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{inviter_user_id}` not found")))?;
        let invitee = users
            .iter()
            .find(|user| user.id == invitee_user_id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{invitee_user_id}` not found")))?;
        let invite_code = required_trimmed(request.invite_code, "invite code")?;
        validate_invite_code(&invite_code, &inviter_user_id, users)?;
        validate_inviter(inviter, policy)?;

        if self.records.values().any(|record| {
            record.inviter_user_id == inviter_user_id && record.invitee_user_id == invitee_user_id
        }) {
            return Err(ApiError::Conflict(format!(
                "invite relation `{inviter_user_id}` -> `{invitee_user_id}` already exists"
            )));
        }

        let now = current_time_label();
        let record = InviteRecord {
            id: id.clone(),
            inviter_user_id: inviter.id.clone(),
            inviter_username: inviter.username.clone(),
            invitee_user_id: invitee.id.clone(),
            invitee_username: invitee.username.clone(),
            invite_code,
            status: InviteStatus::Active,
            rebate_enabled: request.rebate_enabled,
            note: request.note.trim().to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.records.insert(id, record.clone());
        Ok(record)
    }

    fn update(&mut self, id: &str, request: UpdateInviteRecordRequest) -> ApiResult<InviteRecord> {
        let record = self
            .records
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("invite record `{id}` not found")))?;
        record.status = request.status;
        record.rebate_enabled = request.rebate_enabled;
        record.note = request.note.trim().to_string();
        record.updated_at = current_time_label();

        Ok(record.clone())
    }
}

fn validate_invite_code<'a>(
    invite_code: &str,
    inviter_user_id: &str,
    users: &'a [UserSummary],
) -> ApiResult<&'a UserSummary> {
    let Some(owner) = users.iter().find(|user| user.invite_code == invite_code) else {
        return Err(ApiError::BadRequest("邀请码无效".to_string()));
    };

    if owner.kind != UserKind::Agent {
        return Err(ApiError::BadRequest("邀请码无效".to_string()));
    }
    if owner.id != inviter_user_id {
        return Err(ApiError::BadRequest("邀请码与邀请人不匹配".to_string()));
    }

    Ok(owner)
}

fn validate_inviter(inviter: &UserSummary, policy: &InvitePolicySummary) -> ApiResult<()> {
    match inviter.kind {
        UserKind::Agent if policy.agents_can_invite => Ok(()),
        UserKind::Regular if policy.regular_users_can_invite => Ok(()),
        UserKind::Agent => Err(ApiError::Forbidden(
            "agent invite entry is disabled".to_string(),
        )),
        UserKind::Regular => Err(ApiError::Forbidden(
            "regular user invite entry is disabled".to_string(),
        )),
    }
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

fn seed_invites() -> Vec<InviteRecord> {
    vec![
        InviteRecord {
            id: "INV-10001".to_string(),
            inviter_user_id: "U90001".to_string(),
            inviter_username: "agent_alpha".to_string(),
            invitee_user_id: "U10001".to_string(),
            invitee_username: "demo_user".to_string(),
            invite_code: "AGENT10001".to_string(),
            status: InviteStatus::Active,
            rebate_enabled: true,
            note: "默认代理邀请关系".to_string(),
            created_at: "2026-06-02 08:30:00".to_string(),
            updated_at: "2026-06-02 08:30:00".to_string(),
        },
        InviteRecord {
            id: "INV-10002".to_string(),
            inviter_user_id: "U90001".to_string(),
            inviter_username: "agent_alpha".to_string(),
            invitee_user_id: "U10004".to_string(),
            invitee_username: "risk_watch".to_string(),
            invite_code: "AGENT10001".to_string(),
            status: InviteStatus::Pending,
            rebate_enabled: false,
            note: "风险观察用户暂不返利".to_string(),
            created_at: "2026-06-02 10:15:00".to_string(),
            updated_at: "2026-06-02 10:15:00".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::rebate::RebateMode,
        domain::user::{UserKind, UserStatus, UserSummary},
        services::{access::AccessRepository, rebate::RebateRepository},
    };

    #[tokio::test]
    async fn invite_repository_creates_and_updates_agent_invite() {
        let invites = InviteRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        access
            .create_user(UserSummary {
                id: "U20001".to_string(),
                username: "fresh_invitee".to_string(),
                email: None,
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: String::new(),
            })
            .await
            .expect("test invitee can be created");
        let access = access.snapshot().await.expect("access snapshot can load");
        let policy = RebateRepository::memory_seeded()
            .get()
            .await
            .expect("policy can load");

        let created = invites
            .create(
                CreateInviteRecordRequest {
                    id: " INV-NEW ".to_string(),
                    inviter_user_id: "U90001".to_string(),
                    invitee_user_id: "U20001".to_string(),
                    invite_code: " AGENT10001 ".to_string(),
                    rebate_enabled: true,
                    note: " 新邀请 ".to_string(),
                },
                &access.users,
                &policy,
            )
            .await
            .expect("agent invite can be created");

        assert_eq!(created.id, "INV-NEW");
        assert_eq!(created.inviter_username, "agent_alpha");
        assert_eq!(created.invitee_username, "fresh_invitee");
        assert_eq!(created.invite_code, "AGENT10001");
        assert_eq!(created.note, "新邀请");

        let updated = invites
            .update(
                "INV-NEW",
                UpdateInviteRecordRequest {
                    status: InviteStatus::Disabled,
                    rebate_enabled: false,
                    note: "暂停返利".to_string(),
                },
            )
            .await
            .expect("invite can be updated");
        assert_eq!(updated.status, InviteStatus::Disabled);
        assert!(!updated.rebate_enabled);
    }

    #[tokio::test]
    async fn invite_repository_rejects_regular_user_invite_code() {
        let invites = InviteRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let policy = RebateRepository::memory_seeded()
            .get()
            .await
            .expect("policy can load");

        let error = invites
            .create(
                CreateInviteRecordRequest {
                    id: "INV-REGULAR".to_string(),
                    inviter_user_id: "U10001".to_string(),
                    invitee_user_id: "U10004".to_string(),
                    invite_code: "USER10001".to_string(),
                    rebate_enabled: true,
                    note: String::new(),
                },
                &access.users,
                &policy,
            )
            .await
            .expect_err("regular invite code must be rejected");

        assert!(matches!(error, ApiError::BadRequest(message) if message == "邀请码无效"));
    }

    #[tokio::test]
    async fn invite_repository_rejects_agent_inviter_when_policy_disabled() {
        let invites = InviteRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let policy = InvitePolicySummary {
            agents_can_invite: false,
            regular_users_can_invite: false,
            rebate_mode: RebateMode::Immediate,
            supported_rebate_modes: vec![RebateMode::Immediate, RebateMode::RechargeTiered],
            default_recharge_rebate_basis_points: 350,
        };

        let error = invites
            .create(
                CreateInviteRecordRequest {
                    id: "INV-AGENT-DISABLED".to_string(),
                    inviter_user_id: "U90001".to_string(),
                    invitee_user_id: "U10004".to_string(),
                    invite_code: "AGENT10001".to_string(),
                    rebate_enabled: true,
                    note: String::new(),
                },
                &access.users,
                &policy,
            )
            .await
            .expect_err("disabled agent invite entry must be rejected");

        assert!(matches!(error, ApiError::Forbidden(_)));
    }

    #[tokio::test]
    async fn invite_repository_rejects_unknown_invitee() {
        let invites = InviteRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let policy = RebateRepository::memory_seeded()
            .get()
            .await
            .expect("policy can load");

        let error = invites
            .create(
                CreateInviteRecordRequest {
                    id: "INV-BAD".to_string(),
                    inviter_user_id: "U90001".to_string(),
                    invitee_user_id: "missing".to_string(),
                    invite_code: "BAD10001".to_string(),
                    rebate_enabled: true,
                    note: String::new(),
                },
                &access.users,
                &policy,
            )
            .await
            .expect_err("unknown invitee must be rejected");

        assert!(matches!(error, ApiError::NotFound(_)));
    }

    #[tokio::test]
    async fn invite_repository_allows_agent_code_reuse_for_different_invitees() {
        let invites = InviteRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        access
            .create_user(UserSummary {
                id: "U20002".to_string(),
                username: "agent_code_reuse_invitee".to_string(),
                email: None,
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: String::new(),
            })
            .await
            .expect("test invitee can be created");
        let access = access.snapshot().await.expect("access snapshot can load");
        let policy = RebateRepository::memory_seeded()
            .get()
            .await
            .expect("policy can load");

        let created = invites
            .create(
                CreateInviteRecordRequest {
                    id: "INV-REUSE-CODE".to_string(),
                    inviter_user_id: "U90001".to_string(),
                    invitee_user_id: "U20002".to_string(),
                    invite_code: "AGENT10001".to_string(),
                    rebate_enabled: true,
                    note: String::new(),
                },
                &access.users,
                &policy,
            )
            .await
            .expect("agent code can be reused for another invitee");

        assert_eq!(created.invite_code, "AGENT10001");
    }
}
