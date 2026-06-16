//! 代理申请服务层，负责手机端代理申请记录和后台审核记录的持久化。

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;
use sqlx::Row;

use crate::{
    domain::{
        agent_application::{
            AgentApplication, AgentApplicationStatus, ReviewAgentApplicationRequest,
            SubmitAgentApplicationRequest,
        },
        user::AdminSummary,
        user::{UserKind, UserStatus, UserSummary},
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const AGENT_APPLICATION_ID_PREFIX: &str = "AGAPP";
const MAX_REASON_LEN: usize = 500;
const MAX_REVIEW_NOTE_LEN: usize = 500;

#[derive(Clone)]
/// 代理申请仓储，负责申请记录查询、提交、审核和数据库快照同步。
pub struct AgentApplicationRepository {
    inner: Arc<RwLock<AgentApplicationStore>>,
    persistence: Option<BusinessDatabase>,
}

/// 代理申请仓储，负责申请记录查询、提交、审核和数据库快照同步。
impl AgentApplicationRepository {
    /// 创建空的内存代理申请仓储，适合本地无数据库调试。
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AgentApplicationStore::default())),
            persistence: None,
        }
    }

    /// 从数据库加载代理申请记录并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_agent_application_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回代理申请列表，可按审核状态筛选。
    pub async fn list(
        &self,
        status: Option<AgentApplicationStatus>,
    ) -> ApiResult<Vec<AgentApplication>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("代理申请仓储锁读取失败".to_string()))
            .map(|store| store.list(status.as_ref()))
    }

    /// 按申请 ID 查询代理申请详情。
    pub async fn get(&self, id: &str) -> ApiResult<AgentApplication> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("代理申请仓储锁读取失败".to_string()))?
            .get(id)
    }

    /// 查询某个用户最近一次代理申请，供手机端代理中心显示当前审核状态。
    pub async fn latest_for_user(&self, user_id: &str) -> ApiResult<Option<AgentApplication>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("代理申请仓储锁读取失败".to_string()))
            .map(|store| store.latest_for_user(user_id))
    }

    /// 手机端提交代理申请；已有待审核申请时直接返回原申请，避免重复生成。
    pub async fn submit(
        &self,
        user: &UserSummary,
        payload: SubmitAgentApplicationRequest,
    ) -> ApiResult<AgentApplication> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("代理申请仓储锁写入失败".to_string()))?;
            let result = store.submit(user, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 后台写入代理申请审核结果，身份升级由路由层先完成，仓储只负责记录审核事实。
    pub async fn review(
        &self,
        id: &str,
        payload: ReviewAgentApplicationRequest,
        admin: &AdminSummary,
    ) -> ApiResult<AgentApplication> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("代理申请仓储锁写入失败".to_string()))?;
            let result = store.review(id, payload, admin)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 从数据库重新加载代理申请快照，供后台缓存维护按钮使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_agent_application_store(persistence).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("代理申请缓存刷新失败".to_string()))? = store;
        Ok(true)
    }

    /// 数据库模式下保存代理申请仓储快照。
    async fn persist(&self, store: &AgentApplicationStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_agent_application_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
/// 代理申请运行时快照，用于内存模式和数据库持久化前的业务校验。
struct AgentApplicationStore {
    applications: BTreeMap<String, AgentApplication>,
}

/// 代理申请运行时快照，用于内存模式和数据库持久化前的业务校验。
impl AgentApplicationStore {
    /// 返回代理申请列表，默认待审核优先、同状态按创建时间倒序。
    fn list(&self, status: Option<&AgentApplicationStatus>) -> Vec<AgentApplication> {
        let mut applications = self
            .applications
            .values()
            .filter(|application| status.map_or(true, |value| &application.status == value))
            .cloned()
            .collect::<Vec<_>>();
        applications.sort_by(|left, right| compare_application_desc(left, right));
        applications
    }

    /// 按申请 ID 查询详情。
    fn get(&self, id: &str) -> ApiResult<AgentApplication> {
        self.applications
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("agent application `{id}` not found")))
    }

    /// 返回用户最近一次申请，手机端据此显示待审核或驳回说明。
    fn latest_for_user(&self, user_id: &str) -> Option<AgentApplication> {
        self.applications
            .values()
            .filter(|application| application.user_id == user_id)
            .max_by(|left, right| {
                left.created_at
                    .cmp(&right.created_at)
                    .then_with(|| left.id.cmp(&right.id))
            })
            .cloned()
    }

    /// 创建新的待审核申请，或者返回用户已有的待审核申请。
    fn submit(
        &mut self,
        user: &UserSummary,
        payload: SubmitAgentApplicationRequest,
    ) -> ApiResult<AgentApplication> {
        if matches!(&user.kind, UserKind::Agent) {
            return Err(ApiError::BadRequest(
                "当前账号已经是代理，无需重复申请".to_string(),
            ));
        }
        if !matches!(&user.status, UserStatus::Active) {
            return Err(ApiError::BadRequest(
                "账号状态异常，不能申请成为代理".to_string(),
            ));
        }
        if let Some(pending) = self.applications.values().find(|application| {
            application.user_id == user.id
                && matches!(&application.status, AgentApplicationStatus::Pending)
        }) {
            return Ok(pending.clone());
        }

        let now = now_string();
        let application = AgentApplication {
            id: self.next_application_id(),
            user_id: user.id.clone(),
            username: user.username.clone(),
            invite_code: user.invite_code.clone(),
            status: AgentApplicationStatus::Pending,
            reason: normalize_required_text(payload.reason, "申请说明", MAX_REASON_LEN)?,
            review_note: None,
            reviewed_by_admin_id: None,
            reviewed_by_admin_username: None,
            reviewed_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        self.applications
            .insert(application.id.clone(), application.clone());
        Ok(application)
    }

    /// 写入审核结果，只有待审核申请允许被审核。
    fn review(
        &mut self,
        id: &str,
        payload: ReviewAgentApplicationRequest,
        admin: &AdminSummary,
    ) -> ApiResult<AgentApplication> {
        let mut application = self.get(id)?;
        if !matches!(&application.status, AgentApplicationStatus::Pending) {
            return Err(ApiError::BadRequest(
                "代理申请已经审核，不能重复处理".to_string(),
            ));
        }

        let now = now_string();
        application.status = if payload.approved {
            AgentApplicationStatus::Approved
        } else {
            AgentApplicationStatus::Rejected
        };
        application.review_note = normalize_optional_text(payload.note, MAX_REVIEW_NOTE_LEN)?;
        application.reviewed_by_admin_id = Some(admin.id.clone());
        application.reviewed_by_admin_username = Some(admin.username.clone());
        application.reviewed_at = Some(now.clone());
        application.updated_at = now;

        self.applications
            .insert(application.id.clone(), application.clone());
        Ok(application)
    }

    /// 基于现有申请 ID 生成下一条代理申请 ID。
    fn next_application_id(&self) -> String {
        let max_number = self
            .applications
            .keys()
            .filter_map(|id| id.strip_prefix(AGENT_APPLICATION_ID_PREFIX))
            .filter_map(|number| number.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!(
            "{AGENT_APPLICATION_ID_PREFIX}{:06}",
            max_number.saturating_add(1)
        )
    }
}

/// 从数据库加载代理申请运行时快照。
async fn load_agent_application_store(
    database: &BusinessDatabase,
) -> ApiResult<AgentApplicationStore> {
    let mut applications = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, user_id, username, invite_code, status, reason, review_note,
                reviewed_by_admin_id, reviewed_by_admin_username, reviewed_at, created_at, updated_at
         FROM agent_applications
         ORDER BY created_at DESC, id DESC",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?;
        applications.insert(
            id.clone(),
            AgentApplication {
                id,
                user_id: row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                username: row
                    .try_get("username")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                invite_code: row
                    .try_get("invite_code")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                )?,
                reason: row
                    .try_get("reason")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                review_note: row
                    .try_get("review_note")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                reviewed_by_admin_id: row
                    .try_get("reviewed_by_admin_id")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                reviewed_by_admin_username: row
                    .try_get("reviewed_by_admin_username")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                reviewed_at: row
                    .try_get("reviewed_at")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|_| ApiError::Internal("代理申请数据读取失败".to_string()))?,
            },
        );
    }

    Ok(AgentApplicationStore { applications })
}

/// 把代理申请运行时快照保存到数据库。
async fn save_agent_application_store(
    database: &BusinessDatabase,
    store: &AgentApplicationStore,
) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("代理申请事务开启失败".to_string()))?;

    sqlx::query("DELETE FROM agent_applications")
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("代理申请数据清理失败".to_string()))?;

    for application in store.applications.values() {
        sqlx::query(
            "INSERT INTO agent_applications (
                id, user_id, username, invite_code, status, reason, review_note,
                reviewed_by_admin_id, reviewed_by_admin_username, reviewed_at, created_at, updated_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&application.id)
        .bind(&application.user_id)
        .bind(&application.username)
        .bind(&application.invite_code)
        .bind(enum_to_string(&application.status)?)
        .bind(&application.reason)
        .bind(&application.review_note)
        .bind(&application.reviewed_by_admin_id)
        .bind(&application.reviewed_by_admin_username)
        .bind(&application.reviewed_at)
        .bind(&application.created_at)
        .bind(&application.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("代理申请数据保存失败".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("代理申请事务提交失败".to_string()))
}

/// 代理申请列表排序：待审核优先，同状态按创建时间和 ID 倒序。
fn compare_application_desc(
    left: &AgentApplication,
    right: &AgentApplication,
) -> std::cmp::Ordering {
    status_priority(&left.status)
        .cmp(&status_priority(&right.status))
        .then_with(|| right.created_at.cmp(&left.created_at))
        .then_with(|| right.id.cmp(&left.id))
}

/// 返回状态排序权重，待审核越需要优先处理。
fn status_priority(status: &AgentApplicationStatus) -> u8 {
    match status {
        AgentApplicationStatus::Pending => 0,
        AgentApplicationStatus::Rejected => 1,
        AgentApplicationStatus::Approved => 2,
    }
}

/// 校验并规范化必填文本。
fn normalize_required_text(value: String, label: &str, max_len: usize) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label}不能为空")));
    }
    if value.chars().count() > max_len {
        return Err(ApiError::BadRequest(format!(
            "{label}不能超过 {max_len} 个字符"
        )));
    }
    Ok(value)
}

/// 校验并规范化可选文本，空字符串视为未填写。
fn normalize_optional_text(value: Option<String>, max_len: usize) -> ApiResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_len {
        return Err(ApiError::BadRequest(format!(
            "审核备注不能超过 {max_len} 个字符"
        )));
    }
    Ok(Some(value))
}

/// 返回当前本地时间字符串。
fn now_string() -> String {
    Local::now().format(TIMESTAMP_FORMAT).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::AdminSummary;

    fn test_user(kind: UserKind) -> UserSummary {
        UserSummary {
            id: "U10001".to_string(),
            username: "demo_user".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id: None,
            invite_code: "ABCD1234".to_string(),
            registration_location: crate::domain::user::UserRegistrationLocation::default(),
            created_at: "2026-06-05 10:00:00".to_string(),
        }
    }

    fn test_admin() -> AdminSummary {
        AdminSummary {
            id: "A001".to_string(),
            username: "admin".to_string(),
            role_id: "super-admin".to_string(),
            role_name: "超级管理员".to_string(),
            status: UserStatus::Active,
        }
    }

    #[tokio::test]
    async fn agent_application_submit_is_idempotent_for_pending_user() {
        let repository = AgentApplicationRepository::memory();
        let user = test_user(UserKind::Regular);
        let first = repository
            .submit(
                &user,
                SubmitAgentApplicationRequest {
                    reason: "我想推广平台".to_string(),
                },
            )
            .await
            .expect("代理申请可提交");
        let second = repository
            .submit(
                &user,
                SubmitAgentApplicationRequest {
                    reason: "再次申请".to_string(),
                },
            )
            .await
            .expect("待审核申请重复提交时返回原申请");

        assert_eq!(first.id, second.id);
        assert_eq!(repository.list(None).await.expect("列表可读取").len(), 1);
    }

    #[tokio::test]
    async fn agent_application_rejects_existing_agent() {
        let repository = AgentApplicationRepository::memory();
        let error = repository
            .submit(
                &test_user(UserKind::Agent),
                SubmitAgentApplicationRequest {
                    reason: "重复申请".to_string(),
                },
            )
            .await
            .expect_err("代理不能重复申请");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn agent_application_review_records_admin_snapshot() {
        let repository = AgentApplicationRepository::memory();
        let created = repository
            .submit(
                &test_user(UserKind::Regular),
                SubmitAgentApplicationRequest {
                    reason: "具备推广资源".to_string(),
                },
            )
            .await
            .expect("代理申请可提交");

        let reviewed = repository
            .review(
                &created.id,
                ReviewAgentApplicationRequest {
                    approved: true,
                    note: Some("资料通过".to_string()),
                },
                &test_admin(),
            )
            .await
            .expect("代理申请可审核");

        assert_eq!(reviewed.status, AgentApplicationStatus::Approved);
        assert_eq!(
            reviewed.reviewed_by_admin_username.as_deref(),
            Some("admin")
        );
        assert_eq!(reviewed.review_note.as_deref(), Some("资料通过"));
    }
}
