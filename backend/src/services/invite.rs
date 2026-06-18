//! 邀请领域模型，定义邀请码、邀请关系与状态变更参数

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::Row;

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

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

#[derive(Clone)]
/// 邀请关系仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct InviteRepository {
    inner: Arc<RwLock<InviteStore>>,
    persistence: Option<BusinessDatabase>,
}

/// 邀请关系仓储，负责该模块数据读取、业务变更和持久化协调。
impl InviteRepository {
    /// 创建一个仅内存的邀请仓储，适用于测试和未接入数据库的场景。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(InviteStore::seeded())),
            persistence: None,
        }
    }

    /// 从数据库恢复邀请仓储状态，确保重启后邀请关系可继续使用。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_invite_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 获取全部邀请关系，返回可直接渲染到管理页的列表数据。
    pub async fn list(&self) -> ApiResult<Vec<InviteRecord>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("invite store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 按关系 ID 获取单条邀请关系记录。
    pub async fn get(&self, id: &str) -> ApiResult<InviteRecord> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("invite store lock poisoned".to_string()))?
            .get(id)
    }

    /// 创建邀请关系：校验邀请人与被邀用户、邀请码归属及策略后保存，并写回数据库。
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

    /// 更新邀请关系状态/返利开关/备注，保持 updated_at 与关系记录同步更新。
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

    /// 将邀请仓储快照落库，支持持久化与服务重启恢复。
    async fn persist(&self, store: &InviteStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_invite_store(persistence, store).await?;
        }

        Ok(())
    }

    /// 从数据库重新加载邀请关系快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_invite_store(persistence).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("邀请关系缓存刷新失败".to_string()))? = store;
        Ok(true)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// 邀请关系运行时数据快照，用于内存模式和数据库持久化前的业务校验。
struct InviteStore {
    records: BTreeMap<String, InviteRecord>,
}

/// 从数据库加载邀请关系运行时快照，空库时按模块规则初始化。
async fn load_invite_store(database: &BusinessDatabase) -> ApiResult<InviteStore> {
    // 从数据库读取全部邀请关系；空库时写入种子关系后返回默认状态。
    let mut records = BTreeMap::new();

    for row in sqlx::query(
        "SELECT id, inviter_user_id, inviter_username, invitee_user_id, invitee_username,
                invite_code, status, rebate_enabled, note, created_at, updated_at
         FROM invite_records
         ORDER BY id ASC",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?;
        records.insert(
            id.clone(),
            InviteRecord {
                id,
                inviter_user_id: row
                    .try_get("inviter_user_id")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                inviter_username: row
                    .try_get("inviter_username")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                invitee_user_id: row
                    .try_get("invitee_user_id")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                invitee_username: row
                    .try_get("invitee_username")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                invite_code: row
                    .try_get("invite_code")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                )?,
                rebate_enabled: row
                    .try_get("rebate_enabled")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                note: row
                    .try_get("note")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|_| ApiError::Internal("邀请关系数据读取失败".to_string()))?,
            },
        );
    }

    if records.is_empty() {
        let seeded = InviteStore::seeded();
        save_invite_store(database, &seeded).await?;
        return Ok(seeded);
    }

    Ok(InviteStore { records })
}

/// 把邀请关系运行时快照保存到数据库。
async fn save_invite_store(database: &BusinessDatabase, store: &InviteStore) -> ApiResult<()> {
    // 全量重建邀请关系表：先清表再插入快照中的全部记录，最后提交事务。
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("邀请关系事务开启失败".to_string()))?;

    sqlx::query("DELETE FROM invite_records")
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("邀请关系数据清理失败".to_string()))?;

    for record in store.records.values() {
        sqlx::query(
            "INSERT INTO invite_records
             (id, inviter_user_id, inviter_username, invitee_user_id, invitee_username,
              invite_code, status, rebate_enabled, note, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&record.id)
        .bind(&record.inviter_user_id)
        .bind(&record.inviter_username)
        .bind(&record.invitee_user_id)
        .bind(&record.invitee_username)
        .bind(&record.invite_code)
        .bind(enum_to_string(&record.status)?)
        .bind(record.rebate_enabled)
        .bind(&record.note)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("邀请关系数据保存失败".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("邀请关系事务提交失败".to_string()))
}

/// 邀请关系运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl InviteStore {
    /// 使用固定种子邀请关系初始化仓储，支持开发环境与首次启动快速可见关系数据。
    fn seeded() -> Self {
        let records = seed_invites()
            .into_iter()
            .map(|record| (record.id.clone(), record))
            .collect();

        Self { records }
    }

    /// 按当前仓储快照返回全部邀请关系列表。
    fn list(&self) -> Vec<InviteRecord> {
        self.records.values().cloned().collect()
    }

    /// 按 ID 查询邀请关系，不存在时返回友好错误。
    fn get(&self, id: &str) -> ApiResult<InviteRecord> {
        self.records
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("invite record `{id}` not found")))
    }

    /// 创建邀请关系：校验 ID、用户存在、邀请码归属与策略，重复关系直接拒绝。
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

    /// 更新关系状态和返利选项，始终刷新更新时间。
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

/// 校验邀请码是否存在且归属邀请人本人，且必须是代理身份，返回邀请人用户信息。
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

/// 按当前平台策略校验邀请人身份是否允许创建邀请关系。
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

/// 去除字符串空白并校验不能为空，用于所有包含 id/用户名等必填字段。
fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

/// 当前本地时间戳，作为 invite_records 的创建与更新时间字段展示用。
fn current_time_label() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 固定邀请关系种子：提供演示数据和重启恢复的一致性起点。
fn seed_invites() -> Vec<InviteRecord> {
    const AGENT_INVITE_CODE: &str = "KJHG8DSA";
    vec![
        InviteRecord {
            id: "INV-10001".to_string(),
            inviter_user_id: "U90001".to_string(),
            inviter_username: "agent_alpha".to_string(),
            invitee_user_id: "U10001".to_string(),
            invitee_username: "demo_user".to_string(),
            invite_code: AGENT_INVITE_CODE.to_string(),
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
            invite_code: AGENT_INVITE_CODE.to_string(),
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
    /// 验证邀请仓储可以创建并更新代理邀请关系。
    #[tokio::test]
    async fn invite_repository_creates_and_updates_agent_invite() {
        let invites = InviteRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        access
            .create_user(UserSummary {
                id: "U20001".to_string(),
                username: "fresh_invitee".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: String::new(),
                registration_location: crate::domain::user::UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            })
            .await
            .expect("test invitee can be created");
        let access = access.snapshot().await.expect("access snapshot can load");
        let policy = RebateRepository::memory_seeded()
            .get()
            .await
            .expect("policy can load");
        let inviter_code = access
            .users
            .iter()
            .find(|user| user.id == "U90001")
            .map(|user| user.invite_code.clone())
            .expect("inviter invite code exists");

        let created = invites
            .create(
                CreateInviteRecordRequest {
                    id: " INV-NEW ".to_string(),
                    inviter_user_id: "U90001".to_string(),
                    invitee_user_id: "U20001".to_string(),
                    invite_code: format!(" {inviter_code} "),
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
        assert_eq!(created.invite_code, inviter_code);
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
    /// 验证普通用户邀请码不能产生邀请关系。
    #[tokio::test]
    async fn invite_repository_rejects_regular_user_invite_code() {
        let invites = InviteRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let _inviter_code = access
            .users
            .iter()
            .find(|user| user.id == "U90001")
            .map(|user| user.invite_code.clone())
            .expect("inviter invite code exists");
        let policy = RebateRepository::memory_seeded()
            .get()
            .await
            .expect("policy can load");
        let regular_user_code = access
            .users
            .iter()
            .find(|user| user.id == "U10001")
            .map(|user| user.invite_code.clone())
            .expect("regular invite code exists");

        let error = invites
            .create(
                CreateInviteRecordRequest {
                    id: "INV-REGULAR".to_string(),
                    inviter_user_id: "U10001".to_string(),
                    invitee_user_id: "U10004".to_string(),
                    invite_code: regular_user_code,
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
    /// 验证策略关闭时代理也不能新增邀请关系。
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
                    invite_code: access
                        .users
                        .iter()
                        .find(|user| user.id == "U90001")
                        .map(|user| user.invite_code.clone())
                        .expect("inviter invite code exists"),
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
    /// 验证未知被邀请用户会被拒绝。
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
    /// 验证同一代理邀请码可以邀请不同用户。
    #[tokio::test]
    async fn invite_repository_allows_agent_code_reuse_for_different_invitees() {
        let invites = InviteRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        access
            .create_user(UserSummary {
                id: "U20002".to_string(),
                username: "agent_code_reuse_invitee".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: String::new(),
                registration_location: crate::domain::user::UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            })
            .await
            .expect("test invitee can be created");
        let access = access.snapshot().await.expect("access snapshot can load");
        let policy = RebateRepository::memory_seeded()
            .get()
            .await
            .expect("policy can load");
        let inviter_code = access
            .users
            .iter()
            .find(|user| user.id == "U90001")
            .map(|user| user.invite_code.clone())
            .expect("inviter invite code exists");

        let created = invites
            .create(
                CreateInviteRecordRequest {
                    id: "INV-REUSE-CODE".to_string(),
                    inviter_user_id: "U90001".to_string(),
                    invitee_user_id: "U20002".to_string(),
                    invite_code: inviter_code.clone(),
                    rebate_enabled: true,
                    note: String::new(),
                },
                &access.users,
                &policy,
            )
            .await
            .expect("agent code can be reused for another invitee");

        assert_eq!(created.invite_code, inviter_code);
    }
}
