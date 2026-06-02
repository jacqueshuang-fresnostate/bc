use std::sync::{Arc, RwLock};

use crate::{
    domain::rebate::{InvitePolicySummary, InvitePolicyUpdateRequest, RebateMode},
    error::{ApiError, ApiResult},
};

#[derive(Clone)]
pub struct RebateRepository {
    inner: Arc<RwLock<RebateStore>>,
}

impl RebateRepository {
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RebateStore::seeded())),
        }
    }

    pub async fn get(&self) -> ApiResult<InvitePolicySummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("rebate store lock poisoned".to_string()))
            .map(|store| store.policy())
    }

    pub async fn update(
        &self,
        request: InvitePolicyUpdateRequest,
    ) -> ApiResult<InvitePolicySummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("rebate store lock poisoned".to_string()))?
            .update(request)
    }
}

#[derive(Debug)]
struct RebateStore {
    agents_can_invite: bool,
    regular_users_can_invite: bool,
    rebate_mode: RebateMode,
    default_recharge_rebate_basis_points: u16,
}

impl RebateStore {
    fn seeded() -> Self {
        Self {
            agents_can_invite: true,
            regular_users_can_invite: false,
            rebate_mode: RebateMode::Immediate,
            default_recharge_rebate_basis_points: 350,
        }
    }

    fn policy(&self) -> InvitePolicySummary {
        InvitePolicySummary {
            agents_can_invite: self.agents_can_invite,
            regular_users_can_invite: self.regular_users_can_invite,
            rebate_mode: self.rebate_mode.clone(),
            supported_rebate_modes: supported_rebate_modes(),
            default_recharge_rebate_basis_points: self.default_recharge_rebate_basis_points,
        }
    }

    fn update(&mut self, request: InvitePolicyUpdateRequest) -> ApiResult<InvitePolicySummary> {
        validate_policy(&request)?;

        self.agents_can_invite = request.agents_can_invite;
        self.regular_users_can_invite = request.regular_users_can_invite;
        self.rebate_mode = request.rebate_mode;
        self.default_recharge_rebate_basis_points = request.default_recharge_rebate_basis_points;

        Ok(self.policy())
    }
}

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

fn supported_rebate_modes() -> Vec<RebateMode> {
    vec![RebateMode::Immediate, RebateMode::RechargeTiered]
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
