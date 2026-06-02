use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

use chrono::Local;

use crate::{
    domain::{
        group_buy::{
            AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyParticipant,
            GroupBuyPlan, GroupBuyPlanStatus, GroupBuyPlanSummary, UpdateGroupBuyPlanRequest,
        },
        lottery::LotteryKind,
        user::UserSummary,
    },
    error::{ApiError, ApiResult},
};

#[derive(Clone)]
pub struct GroupBuyRepository {
    inner: Arc<RwLock<GroupBuyStore>>,
}

impl GroupBuyRepository {
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(GroupBuyStore::seeded())),
        }
    }

    pub async fn list(&self) -> ApiResult<Vec<GroupBuyPlanSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    pub async fn get(&self, id: &str) -> ApiResult<GroupBuyPlan> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .get(id)
    }

    pub async fn create(
        &self,
        request: CreateGroupBuyPlanRequest,
        lotteries: &[LotteryKind],
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .create(request, lotteries, users)
    }

    pub async fn update(
        &self,
        id: &str,
        request: UpdateGroupBuyPlanRequest,
    ) -> ApiResult<GroupBuyPlan> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .update(id, request)
    }

    pub async fn add_participant(
        &self,
        id: &str,
        request: AddGroupBuyParticipantRequest,
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("group buy store lock poisoned".to_string()))?
            .add_participant(id, request, users)
    }
}

#[derive(Debug)]
struct GroupBuyStore {
    plans: BTreeMap<String, GroupBuyPlan>,
}

impl GroupBuyStore {
    fn seeded() -> Self {
        let plans = seed_group_buy_plans()
            .into_iter()
            .map(|plan| (plan.id.clone(), plan))
            .collect();

        Self { plans }
    }

    fn list(&self) -> Vec<GroupBuyPlanSummary> {
        self.plans.values().map(GroupBuyPlan::summary).collect()
    }

    fn get(&self, id: &str) -> ApiResult<GroupBuyPlan> {
        self.plans
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))
    }

    fn create(
        &mut self,
        request: CreateGroupBuyPlanRequest,
        lotteries: &[LotteryKind],
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        let id = required_trimmed(request.id, "group buy plan id")?;
        if self.plans.contains_key(&id) {
            return Err(ApiError::Conflict(format!(
                "group buy plan `{id}` already exists"
            )));
        }

        let lottery_id = required_trimmed(request.lottery_id, "lottery id")?;
        let lottery = find_lottery(lotteries, &lottery_id)?;
        if !lottery.group_buy.enabled {
            return Err(ApiError::BadRequest(format!(
                "lottery `{lottery_id}` group buy is disabled"
            )));
        }

        let initiator_user_id = required_trimmed(request.initiator_user_id, "initiator user id")?;
        let initiator = find_user(users, &initiator_user_id)?;
        validate_positive_amount(request.total_amount_minor, "total amount")?;
        validate_positive_amount(request.initiator_amount_minor, "initiator amount")?;
        validate_share_amount(
            request.total_amount_minor,
            lottery.group_buy.min_share_amount_minor,
            "total amount",
        )?;
        validate_share_amount(
            request.initiator_amount_minor,
            lottery.group_buy.min_share_amount_minor,
            "initiator amount",
        )?;

        if request.initiator_amount_minor > request.total_amount_minor {
            return Err(ApiError::BadRequest(
                "initiator amount cannot exceed total amount".to_string(),
            ));
        }

        let minimum_initiator_amount = minimum_amount_by_percent(
            request.total_amount_minor,
            lottery.group_buy.initiator_min_percent,
        )?;
        if request.initiator_amount_minor < minimum_initiator_amount {
            return Err(ApiError::BadRequest(format!(
                "initiator amount must be at least {minimum_initiator_amount}"
            )));
        }

        let now = current_time_label();
        let participant = GroupBuyParticipant {
            id: format!("{id}-P001"),
            user_id: initiator.id.clone(),
            username: initiator.username.clone(),
            amount_minor: request.initiator_amount_minor,
            share_count: share_count(
                request.initiator_amount_minor,
                lottery.group_buy.min_share_amount_minor,
            )?,
            note: "发起人认购".to_string(),
            created_at: now.clone(),
        };
        let status = if request.initiator_amount_minor == request.total_amount_minor {
            GroupBuyPlanStatus::Filled
        } else {
            GroupBuyPlanStatus::Open
        };
        let plan = GroupBuyPlan {
            id: id.clone(),
            lottery_id: lottery.id.clone(),
            lottery_name: lottery.name.clone(),
            initiator_user_id: initiator.id.clone(),
            initiator_username: initiator.username.clone(),
            total_amount_minor: request.total_amount_minor,
            filled_amount_minor: request.initiator_amount_minor,
            min_share_amount_minor: lottery.group_buy.min_share_amount_minor,
            participant_min_amount_minor: lottery.group_buy.participant_min_amount_minor,
            share_count: share_count(
                request.total_amount_minor,
                lottery.group_buy.min_share_amount_minor,
            )?,
            status,
            participants: vec![participant],
            note: request.note.trim().to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        self.plans.insert(id, plan.clone());
        Ok(plan)
    }

    fn update(&mut self, id: &str, request: UpdateGroupBuyPlanRequest) -> ApiResult<GroupBuyPlan> {
        let plan = self
            .plans
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))?;

        if matches!(
            request.status,
            GroupBuyPlanStatus::Filled | GroupBuyPlanStatus::Settled
        ) && plan.filled_amount_minor < plan.total_amount_minor
        {
            return Err(ApiError::BadRequest(
                "group buy plan must be fully filled before filled or settled status".to_string(),
            ));
        }

        plan.status = request.status;
        plan.note = request.note.trim().to_string();
        plan.updated_at = current_time_label();

        Ok(plan.clone())
    }

    fn add_participant(
        &mut self,
        id: &str,
        request: AddGroupBuyParticipantRequest,
        users: &[UserSummary],
    ) -> ApiResult<GroupBuyPlan> {
        let plan = self
            .plans
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("group buy plan `{id}` not found")))?;

        if !matches!(
            plan.status,
            GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open
        ) {
            return Err(ApiError::BadRequest(
                "group buy plan is not open for participation".to_string(),
            ));
        }

        let participant_id = required_trimmed(request.id, "group buy participant id")?;
        if plan
            .participants
            .iter()
            .any(|participant| participant.id == participant_id)
        {
            return Err(ApiError::Conflict(format!(
                "group buy participant `{participant_id}` already exists"
            )));
        }

        let user_id = required_trimmed(request.user_id, "participant user id")?;
        let user = find_user(users, &user_id)?;
        validate_positive_amount(request.amount_minor, "participant amount")?;
        validate_share_amount(
            request.amount_minor,
            plan.min_share_amount_minor,
            "participant amount",
        )?;

        if request.amount_minor < participant_min_amount(plan) {
            return Err(ApiError::BadRequest(format!(
                "participant amount must be at least {}",
                participant_min_amount(plan)
            )));
        }

        let next_filled = plan
            .filled_amount_minor
            .checked_add(request.amount_minor)
            .ok_or_else(|| ApiError::Internal("group buy filled amount overflow".to_string()))?;
        if next_filled > plan.total_amount_minor {
            return Err(ApiError::BadRequest(
                "participant amount exceeds remaining group buy amount".to_string(),
            ));
        }

        plan.participants.push(GroupBuyParticipant {
            id: participant_id,
            user_id: user.id.clone(),
            username: user.username.clone(),
            amount_minor: request.amount_minor,
            share_count: share_count(request.amount_minor, plan.min_share_amount_minor)?,
            note: request.note.trim().to_string(),
            created_at: current_time_label(),
        });
        plan.filled_amount_minor = next_filled;
        if plan.filled_amount_minor == plan.total_amount_minor {
            plan.status = GroupBuyPlanStatus::Filled;
        }
        plan.updated_at = current_time_label();

        Ok(plan.clone())
    }
}

fn participant_min_amount(plan: &GroupBuyPlan) -> i64 {
    plan.participant_min_amount_minor
        .max(plan.min_share_amount_minor)
        .max(1)
}

fn find_lottery<'a>(lotteries: &'a [LotteryKind], id: &str) -> ApiResult<&'a LotteryKind> {
    lotteries
        .iter()
        .find(|lottery| lottery.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
}

fn find_user<'a>(users: &'a [UserSummary], id: &str) -> ApiResult<&'a UserSummary> {
    users
        .iter()
        .find(|user| user.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))
}

fn validate_positive_amount(amount: i64, label: &str) -> ApiResult<()> {
    if amount <= 0 {
        return Err(ApiError::BadRequest(format!(
            "{label} must be greater than zero"
        )));
    }

    Ok(())
}

fn validate_share_amount(amount: i64, min_share_amount_minor: i64, label: &str) -> ApiResult<()> {
    if min_share_amount_minor <= 0 {
        return Err(ApiError::BadRequest(
            "group buy min share amount must be greater than zero".to_string(),
        ));
    }

    if amount % min_share_amount_minor != 0 {
        return Err(ApiError::BadRequest(format!(
            "{label} must be divisible by min share amount"
        )));
    }

    Ok(())
}

fn minimum_amount_by_percent(total_amount_minor: i64, percent: u8) -> ApiResult<i64> {
    let raw = total_amount_minor
        .checked_mul(i64::from(percent))
        .ok_or_else(|| ApiError::Internal("group buy minimum amount overflow".to_string()))?;

    Ok((raw + 99) / 100)
}

fn share_count(amount_minor: i64, min_share_amount_minor: i64) -> ApiResult<u32> {
    validate_share_amount(amount_minor, min_share_amount_minor, "amount")?;
    let shares = amount_minor / min_share_amount_minor;
    u32::try_from(shares)
        .map_err(|_| ApiError::BadRequest("group buy share count is too large".to_string()))
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

fn seed_group_buy_plans() -> Vec<GroupBuyPlan> {
    vec![GroupBuyPlan {
        id: "G202606020001".to_string(),
        lottery_id: "fc3d".to_string(),
        lottery_name: "福彩 3D".to_string(),
        initiator_user_id: "U90001".to_string(),
        initiator_username: "agent_alpha".to_string(),
        total_amount_minor: 100_000,
        filled_amount_minor: 72_000,
        min_share_amount_minor: 100,
        participant_min_amount_minor: 1_000,
        share_count: 1_000,
        status: GroupBuyPlanStatus::Open,
        participants: vec![
            GroupBuyParticipant {
                id: "G202606020001-P001".to_string(),
                user_id: "U90001".to_string(),
                username: "agent_alpha".to_string(),
                amount_minor: 10_000,
                share_count: 100,
                note: "发起人认购".to_string(),
                created_at: "2026-06-02 09:00:00".to_string(),
            },
            GroupBuyParticipant {
                id: "G202606020001-P002".to_string(),
                user_id: "U10001".to_string(),
                username: "demo_user".to_string(),
                amount_minor: 62_000,
                share_count: 620,
                note: "普通用户参与".to_string(),
                created_at: "2026-06-02 09:30:00".to_string(),
            },
        ],
        note: "默认合买计划示例".to_string(),
        created_at: "2026-06-02 09:00:00".to_string(),
        updated_at: "2026-06-02 09:30:00".to_string(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::{access::AccessRepository, lottery::seed_lotteries};

    #[tokio::test]
    async fn group_buy_repository_creates_plan_with_initiator_participant() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let plan = repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-001".to_string(),
                    lottery_id: "fc3d".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 100_000,
                    initiator_amount_minor: 10_000,
                    note: "测试计划".to_string(),
                },
                &seed_lotteries(),
                &access.users,
            )
            .await
            .expect("valid group buy plan can be created");

        assert_eq!(plan.share_count, 1_000);
        assert_eq!(plan.filled_amount_minor, 10_000);
        assert_eq!(plan.status, GroupBuyPlanStatus::Open);
        assert_eq!(plan.participants.len(), 1);
        assert_eq!(plan.participants[0].share_count, 100);
    }

    #[tokio::test]
    async fn group_buy_repository_rejects_disabled_lottery() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let error = repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-002".to_string(),
                    lottery_id: "manual-test".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 100_000,
                    initiator_amount_minor: 10_000,
                    note: "关闭合买彩种".to_string(),
                },
                &seed_lotteries(),
                &access.users,
            )
            .await
            .expect_err("disabled group buy lottery must be rejected");

        assert!(error.to_string().contains("group buy is disabled"));
    }

    #[tokio::test]
    async fn group_buy_repository_rejects_low_initiator_amount() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let error = repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-003".to_string(),
                    lottery_id: "fc3d".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 100_000,
                    initiator_amount_minor: 9_900,
                    note: "低于发起人比例".to_string(),
                },
                &seed_lotteries(),
                &access.users,
            )
            .await
            .expect_err("low initiator amount must be rejected");

        assert!(error
            .to_string()
            .contains("initiator amount must be at least"));
    }

    #[tokio::test]
    async fn group_buy_repository_adds_participant_and_fills_plan() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let plan = repository
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-TEST-004".to_string(),
                    lottery_id: "fc3d".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 20_000,
                    initiator_amount_minor: 10_000,
                    note: "可满单计划".to_string(),
                },
                &seed_lotteries(),
                &access.users,
            )
            .await
            .expect("plan can be created");

        assert_eq!(plan.status, GroupBuyPlanStatus::Open);

        let filled = repository
            .add_participant(
                "G-TEST-004",
                AddGroupBuyParticipantRequest {
                    id: "G-TEST-004-P002".to_string(),
                    user_id: "U10001".to_string(),
                    amount_minor: 10_000,
                    note: "参与满单".to_string(),
                },
                &access.users,
            )
            .await
            .expect("participant can fill plan");

        assert_eq!(filled.filled_amount_minor, 20_000);
        assert_eq!(filled.status, GroupBuyPlanStatus::Filled);
        assert_eq!(filled.participants.len(), 2);
    }

    #[tokio::test]
    async fn group_buy_repository_rejects_participant_overfill() {
        let repository = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let error = repository
            .add_participant(
                "G202606020001",
                AddGroupBuyParticipantRequest {
                    id: "G202606020001-P999".to_string(),
                    user_id: "U10001".to_string(),
                    amount_minor: 40_000,
                    note: "超额参与".to_string(),
                },
                &access.users,
            )
            .await
            .expect_err("participant amount over remaining must be rejected");

        assert!(error.to_string().contains("exceeds remaining"));
    }
}
