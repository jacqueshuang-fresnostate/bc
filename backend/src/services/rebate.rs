//! 返利领域模型，定义邀请返利模式与配置更新参数

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::rebate::{InvitePolicySummary, InvitePolicyUpdateRequest, RebateMode},
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

#[derive(Clone)]
pub struct RebateRepository {
    inner: Arc<RwLock<RebateStore>>,
    persistence: Option<BusinessDatabase>,
}

impl RebateRepository {
    /// 返回带内置种子数据的内存仓储实例。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RebateStore::seeded())),
            persistence: None,
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_rebate_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 按 ID 查询单条记录。
    pub async fn get(&self) -> ApiResult<InvitePolicySummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("rebate store lock poisoned".to_string()))
            .map(|store| store.policy())
    }

    /// 更新现有记录并持久化变更。
    pub async fn update(
        &self,
        request: InvitePolicyUpdateRequest,
    ) -> ApiResult<InvitePolicySummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("rebate store lock poisoned".to_string()))?;
            let result = store.update(request)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    async fn persist(&self, store: &RebateStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_rebate_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct RebateStore {
    agents_can_invite: bool,
    regular_users_can_invite: bool,
    rebate_mode: RebateMode,
    default_recharge_rebate_basis_points: u16,
}

async fn load_rebate_store(database: &BusinessDatabase) -> ApiResult<RebateStore> {
    let store = sqlx::query(
        "SELECT agents_can_invite, regular_users_can_invite, rebate_mode,
                default_recharge_rebate_basis_points
         FROM rebate_policy
         WHERE id = 'default'",
    )
    .fetch_optional(database.pool())
    .await
    .map_err(|_| ApiError::Internal("返利策略数据读取失败".to_string()))?
    .map(|row| {
        let basis_points: i32 = row
            .try_get("default_recharge_rebate_basis_points")
            .map_err(|_| ApiError::Internal("返利策略数据读取失败".to_string()))?;
        Ok(RebateStore {
            agents_can_invite: row
                .try_get("agents_can_invite")
                .map_err(|_| ApiError::Internal("返利策略数据读取失败".to_string()))?,
            regular_users_can_invite: row
                .try_get("regular_users_can_invite")
                .map_err(|_| ApiError::Internal("返利策略数据读取失败".to_string()))?,
            rebate_mode: enum_from_string(
                row.try_get("rebate_mode")
                    .map_err(|_| ApiError::Internal("返利策略数据读取失败".to_string()))?,
            )?,
            default_recharge_rebate_basis_points: u16::try_from(basis_points)
                .map_err(|_| ApiError::Internal("返利比例数据无效".to_string()))?,
        })
    })
    .transpose()?;

    let Some(store) = store else {
        let seeded = RebateStore::seeded();
        save_rebate_store(database, &seeded).await?;
        return Ok(seeded);
    };

    Ok(store)
}

async fn save_rebate_store(database: &BusinessDatabase, store: &RebateStore) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO rebate_policy
         (id, agents_can_invite, regular_users_can_invite, rebate_mode,
          default_recharge_rebate_basis_points)
         VALUES ('default', $1, $2, $3, $4)
         ON CONFLICT (id) DO UPDATE SET
            agents_can_invite = EXCLUDED.agents_can_invite,
            regular_users_can_invite = EXCLUDED.regular_users_can_invite,
            rebate_mode = EXCLUDED.rebate_mode,
            default_recharge_rebate_basis_points = EXCLUDED.default_recharge_rebate_basis_points,
            updated_at = now()",
    )
    .bind(store.agents_can_invite)
    .bind(store.regular_users_can_invite)
    .bind(enum_to_string(&store.rebate_mode)?)
    .bind(i32::from(store.default_recharge_rebate_basis_points))
    .execute(database.pool())
    .await
    .map_err(|_| ApiError::Internal("返利策略数据保存失败".to_string()))?;

    Ok(())
}

impl RebateStore {
    /// 构建并返回种子数据。
    fn seeded() -> Self {
        Self {
            agents_can_invite: true,
            regular_users_can_invite: false,
            rebate_mode: RebateMode::Immediate,
            default_recharge_rebate_basis_points: 350,
        }
    }

    /// 处理 policy 的具体内部流程。
    fn policy(&self) -> InvitePolicySummary {
        InvitePolicySummary {
            agents_can_invite: self.agents_can_invite,
            regular_users_can_invite: self.regular_users_can_invite,
            rebate_mode: self.rebate_mode.clone(),
            supported_rebate_modes: supported_rebate_modes(),
            default_recharge_rebate_basis_points: self.default_recharge_rebate_basis_points,
        }
    }

    /// 校验入参并更新指定记录。
    fn update(&mut self, request: InvitePolicyUpdateRequest) -> ApiResult<InvitePolicySummary> {
        validate_policy(&request)?;

        self.agents_can_invite = request.agents_can_invite;
        self.regular_users_can_invite = request.regular_users_can_invite;
        self.rebate_mode = request.rebate_mode;
        self.default_recharge_rebate_basis_points = request.default_recharge_rebate_basis_points;

        Ok(self.policy())
    }
}

/// 校验返利策略的字段与范围。
fn validate_policy(request: &InvitePolicyUpdateRequest) -> ApiResult<()> {
    if !request.agents_can_invite && !request.regular_users_can_invite {
        return Err(ApiError::BadRequest(
            "agents or regular users must be able to invite".to_string(),
        ));
    }

    if request.default_recharge_rebate_basis_points > 10_000 {
        return Err(ApiError::BadRequest(
            "default recharge rebate basis points must not exceed 10000".to_string(),
        ));
    }

    Ok(())
}

/// 处理 supported_rebate_modes 的具体内部流程。
fn supported_rebate_modes() -> Vec<RebateMode> {
    vec![RebateMode::Immediate, RebateMode::RechargeTiered]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::business_database::BusinessDatabase;

    #[tokio::test]
    async fn rebate_repository_updates_invite_policy() {
        let rebates = RebateRepository::memory_seeded();

        let policy = rebates
            .update(InvitePolicyUpdateRequest {
                agents_can_invite: true,
                regular_users_can_invite: true,
                rebate_mode: RebateMode::RechargeTiered,
                default_recharge_rebate_basis_points: 520,
            })
            .await
            .expect("policy can be updated");

        assert!(policy.agents_can_invite);
        assert!(policy.regular_users_can_invite);
        assert_eq!(policy.rebate_mode, RebateMode::RechargeTiered);
        assert_eq!(policy.default_recharge_rebate_basis_points, 520);
        assert_eq!(policy.supported_rebate_modes.len(), 2);
    }

    #[tokio::test]
    async fn rebate_repository_rejects_closed_invite_entries() {
        let rebates = RebateRepository::memory_seeded();

        let error = rebates
            .update(InvitePolicyUpdateRequest {
                agents_can_invite: false,
                regular_users_can_invite: false,
                rebate_mode: RebateMode::Immediate,
                default_recharge_rebate_basis_points: 350,
            })
            .await
            .expect_err("all invite entries cannot be closed");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn rebate_repository_rejects_rebate_above_full_amount() {
        let rebates = RebateRepository::memory_seeded();

        let error = rebates
            .update(InvitePolicyUpdateRequest {
                agents_can_invite: true,
                regular_users_can_invite: false,
                rebate_mode: RebateMode::Immediate,
                default_recharge_rebate_basis_points: 10_001,
            })
            .await
            .expect_err("rebate rate above 100 percent cannot be saved");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn rebate_repository_persists_policy_when_test_database_configured() {
        let Ok(database_url) = std::env::var("BC_TEST_DATABASE_URL") else {
            return;
        };

        let database = BusinessDatabase::postgres(&database_url)
            .await
            .expect("测试数据库可以连接并运行迁移");
        let rebates = RebateRepository::persistent(database.clone())
            .await
            .expect("返利仓储可以从数据库启动");

        rebates
            .update(InvitePolicyUpdateRequest {
                agents_can_invite: true,
                regular_users_can_invite: true,
                rebate_mode: RebateMode::RechargeTiered,
                default_recharge_rebate_basis_points: 618,
            })
            .await
            .expect("返利策略可以写入数据库");

        let restored = RebateRepository::persistent(database)
            .await
            .expect("返利仓储可以重新从数据库启动")
            .get()
            .await
            .expect("返利策略可以从数据库读取");

        assert!(restored.agents_can_invite);
        assert!(restored.regular_users_can_invite);
        assert_eq!(restored.rebate_mode, RebateMode::RechargeTiered);
        assert_eq!(restored.default_recharge_rebate_basis_points, 618);
    }
}
