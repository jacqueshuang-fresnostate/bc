//! 返利领域模型，定义邀请返利模式与配置更新参数

use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::{
        finance::LedgerEntry,
        invite::{InviteRecord, InviteStatus},
        rebate::{AgentRebateSummary, InvitePolicySummary, InvitePolicyUpdateRequest, RebateMode},
        recharge::{RechargeOrderStatus, RechargeOrderSummary},
        user::{UserKind, UserStatus, UserSummary},
        withdrawal::WithdrawalOrderStatus,
    },
    error::{ApiError, ApiResult},
    services::{
        access::AccessRepository,
        finance::FinanceRepository,
        invite::InviteRepository,
        pagination::{ListPage, PageRequest},
    },
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

#[derive(Clone)]
/// 邀请返利策略仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct RebateRepository {
    inner: Arc<RwLock<RebateStore>>,
    persistence: Option<BusinessDatabase>,
}

/// 邀请返利策略仓储，负责该模块数据读取、业务变更和持久化协调。
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

    /// 按业务标识读取单条记录，未命中时返回未找到错误。
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
    /// 把当前仓储快照同步保存到持久化存储。
    async fn persist(&self, store: &RebateStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_rebate_store(persistence, store).await?;
        }

        Ok(())
    }

    /// 从数据库重新加载邀请返利策略快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_rebate_store(persistence).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("返利策略缓存刷新失败".to_string()))? = store;
        Ok(true)
    }

    /// 数据库模式下分页聚合代理返利统计；内存模式返回 None 交给路由复用旧聚合路径。
    pub async fn agent_rebate_summary_page(
        &self,
        page: PageRequest,
    ) -> ApiResult<Option<ListPage<AgentRebateSummary>>> {
        let Some(persistence) = &self.persistence else {
            return Ok(None);
        };

        query_agent_rebate_summary_page(persistence, page)
            .await
            .map(Some)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// 邀请返利策略运行时数据快照，用于内存模式和数据库持久化前的业务校验。
struct RebateStore {
    agents_can_invite: bool,
    regular_users_can_invite: bool,
    rebate_mode: RebateMode,
    default_recharge_rebate_basis_points: u16,
}

/// 从数据库加载邀请返利策略运行时快照，空库时按模块规则初始化。
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

/// 把邀请返利策略运行时快照保存到数据库。
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

/// 数据库模式下直接聚合代理返利统计，避免后台统计页读取全量用户、流水、充值和提现记录。
async fn query_agent_rebate_summary_page(
    database: &BusinessDatabase,
    page: PageRequest,
) -> ApiResult<ListPage<AgentRebateSummary>> {
    let agent_kind = enum_to_string(&UserKind::Agent)?;
    let active_invite_status = enum_to_string(&InviteStatus::Active)?;
    let rebate_kind =
        enum_to_string(&crate::domain::finance::LedgerEntryKind::RechargeRebateCredit)?;
    let withdrawal_kind =
        enum_to_string(&crate::domain::finance::LedgerEntryKind::AgentRebateWithdrawal)?;
    let paid_recharge_status = enum_to_string(&RechargeOrderStatus::Paid)?;
    let approved_withdrawal_status = enum_to_string(&WithdrawalOrderStatus::Approved)?;

    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM users
         WHERE kind = $1",
    )
    .bind(&agent_kind)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("代理返利统计总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("代理返利统计总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "WITH direct_links AS (
            SELECT inviter_user_id AS agent_user_id, invitee_user_id
            FROM invite_records
            WHERE status = $2
            UNION
            SELECT agent_id AS agent_user_id, id AS invitee_user_id
            FROM users
            WHERE agent_id IS NOT NULL AND agent_id <> ''
         ),
         ledger_totals AS (
            SELECT user_id AS agent_user_id,
                   COUNT(*) FILTER (WHERE kind = $3) AS rebate_record_count,
                   COALESCE(SUM(CASE WHEN kind = $3 AND amount_minor > 0 THEN amount_minor ELSE 0 END), 0)::bigint AS total_rebate_minor,
                   COALESCE(SUM(CASE WHEN kind = $4 AND amount_minor < 0 THEN -amount_minor ELSE 0 END), 0)::bigint AS withdrawn_rebate_minor,
                   MAX(created_at) FILTER (WHERE kind = $3) AS last_rebate_at
            FROM ledger_entries
            WHERE kind IN ($3, $4)
            GROUP BY user_id
         ),
         direct_counts AS (
            SELECT agent_user_id, COUNT(DISTINCT invitee_user_id) AS direct_invitee_count
            FROM direct_links
            GROUP BY agent_user_id
         ),
         direct_recharges AS (
            SELECT links.agent_user_id, COALESCE(SUM(recharge.amount_minor), 0)::bigint AS direct_invitee_recharge_minor
            FROM (SELECT DISTINCT agent_user_id, invitee_user_id FROM direct_links) AS links
            JOIN recharge_orders AS recharge
              ON recharge.user_id = links.invitee_user_id
             AND recharge.status = $5
            GROUP BY links.agent_user_id
         ),
         direct_withdrawals AS (
            SELECT links.agent_user_id, COALESCE(SUM(withdrawal.amount_minor), 0)::bigint AS direct_invitee_withdrawal_minor
            FROM (SELECT DISTINCT agent_user_id, invitee_user_id FROM direct_links) AS links
            JOIN withdrawal_orders AS withdrawal
              ON withdrawal.user_id = links.invitee_user_id
             AND withdrawal.status = $6
            GROUP BY links.agent_user_id
         )
         SELECT agent.id AS agent_user_id,
                agent.username AS agent_username,
                agent.invite_code,
                COALESCE(direct_counts.direct_invitee_count, 0) AS direct_invitee_count,
                COALESCE(direct_recharges.direct_invitee_recharge_minor, 0) AS direct_invitee_recharge_minor,
                COALESCE(direct_withdrawals.direct_invitee_withdrawal_minor, 0) AS direct_invitee_withdrawal_minor,
                COALESCE(ledger_totals.rebate_record_count, 0) AS rebate_record_count,
                COALESCE(ledger_totals.total_rebate_minor, 0) AS total_rebate_minor,
                COALESCE(ledger_totals.withdrawn_rebate_minor, 0) AS withdrawn_rebate_minor,
                GREATEST(COALESCE(ledger_totals.total_rebate_minor, 0) - COALESCE(ledger_totals.withdrawn_rebate_minor, 0), 0) AS pending_rebate_minor,
                LEAST(
                    GREATEST(COALESCE(ledger_totals.total_rebate_minor, 0) - COALESCE(ledger_totals.withdrawn_rebate_minor, 0), 0),
                    COALESCE(account.available_balance_minor, 0)
                ) AS withdrawable_rebate_minor,
                COALESCE(account.available_balance_minor, 0) AS account_available_balance_minor,
                ledger_totals.last_rebate_at AS last_rebate_at
         FROM users AS agent
         LEFT JOIN financial_accounts AS account ON account.user_id = agent.id
         LEFT JOIN ledger_totals ON ledger_totals.agent_user_id = agent.id
         LEFT JOIN direct_counts ON direct_counts.agent_user_id = agent.id
         LEFT JOIN direct_recharges ON direct_recharges.agent_user_id = agent.id
         LEFT JOIN direct_withdrawals ON direct_withdrawals.agent_user_id = agent.id
         WHERE agent.kind = $1
         ORDER BY
           CASE WHEN ledger_totals.last_rebate_at IS NULL OR ledger_totals.last_rebate_at = '' THEN 1 ELSE 0 END ASC,
           ledger_totals.last_rebate_at DESC,
           GREATEST(COALESCE(ledger_totals.total_rebate_minor, 0) - COALESCE(ledger_totals.withdrawn_rebate_minor, 0), 0) DESC,
           agent.id DESC
         LIMIT $7 OFFSET $8",
    )
    .bind(&agent_kind)
    .bind(&active_invite_status)
    .bind(&rebate_kind)
    .bind(&withdrawal_kind)
    .bind(&paid_recharge_status)
    .bind(&approved_withdrawal_status)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("代理返利统计分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(|row| {
            let direct_invitee_count: i64 = row
                .try_get("direct_invitee_count")
                .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?;
            let rebate_record_count: i64 = row
                .try_get("rebate_record_count")
                .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?;
            Ok(AgentRebateSummary {
                account_available_balance_minor: row
                    .try_get("account_available_balance_minor")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                agent_user_id: row
                    .try_get("agent_user_id")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                agent_username: row
                    .try_get("agent_username")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                direct_invitee_count: usize::try_from(direct_invitee_count)
                    .map_err(|_| ApiError::Internal("代理直属下级数量数据无效".to_string()))?,
                direct_invitee_recharge_minor: row
                    .try_get("direct_invitee_recharge_minor")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                direct_invitee_withdrawal_minor: row
                    .try_get("direct_invitee_withdrawal_minor")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                invite_code: row
                    .try_get("invite_code")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                last_rebate_at: row
                    .try_get("last_rebate_at")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                pending_rebate_minor: row
                    .try_get("pending_rebate_minor")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                rebate_record_count: usize::try_from(rebate_record_count)
                    .map_err(|_| ApiError::Internal("代理返利记录数量数据无效".to_string()))?,
                total_rebate_minor: row
                    .try_get("total_rebate_minor")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                withdrawable_rebate_minor: row
                    .try_get("withdrawable_rebate_minor")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
                withdrawn_rebate_minor: row
                    .try_get("withdrawn_rebate_minor")
                    .map_err(|_| ApiError::Internal("代理返利统计数据读取失败".to_string()))?,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 邀请返利策略运行时数据快照，用于内存模式和数据库持久化前的业务校验。
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

    /// 返回当前邀请返利策略摘要。
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

/// 返回系统支持的返利模式。
fn supported_rebate_modes() -> Vec<RebateMode> {
    vec![RebateMode::Immediate, RebateMode::RechargeTiered]
}

/// 充值成功后尝试给上级代理发放返利；没有符合条件的代理时静默跳过。
pub async fn credit_recharge_rebate_for_order(
    access: &AccessRepository,
    invites: &InviteRepository,
    rebates: &RebateRepository,
    finance: &FinanceRepository,
    order: &RechargeOrderSummary,
) -> ApiResult<Option<LedgerEntry>> {
    let policy = rebates.get().await?;
    let rebate_amount_minor = recharge_rebate_amount_minor(order.amount_minor, &policy)?;
    if rebate_amount_minor <= 0 {
        return Ok(None);
    }

    let users = access.users().await?;
    let invite_records = invites.list().await?;
    let Some(recipient) =
        recharge_rebate_recipient(&order.user_id, &users, &invite_records, &policy)
    else {
        return Ok(None);
    };

    let entry = finance
        .credit_recharge_rebate(
            &recipient.id,
            &order.user_id,
            rebate_amount_minor,
            &order.id,
        )
        .await?;

    Ok(Some(entry))
}

/// 计算充值返利金额；阶梯模式未配置独立阶梯时沿用默认比例，避免策略开启后不发放。
fn recharge_rebate_amount_minor(
    recharge_amount_minor: i64,
    policy: &InvitePolicySummary,
) -> ApiResult<i64> {
    if recharge_amount_minor <= 0 || policy.default_recharge_rebate_basis_points == 0 {
        return Ok(0);
    }

    let basis_points = i64::from(policy.default_recharge_rebate_basis_points);
    recharge_amount_minor
        .checked_mul(basis_points)
        .map(|amount| amount / 10_000)
        .ok_or_else(|| ApiError::BadRequest("充值返利金额过大".to_string()))
}

/// 根据后台邀请记录或注册代理关系解析返利接收代理。
fn recharge_rebate_recipient(
    invitee_user_id: &str,
    users: &[UserSummary],
    invite_records: &[InviteRecord],
    policy: &InvitePolicySummary,
) -> Option<UserSummary> {
    if !policy.agents_can_invite {
        return None;
    }

    let users_by_id = users
        .iter()
        .map(|user| (user.id.as_str(), user))
        .collect::<std::collections::BTreeMap<_, _>>();
    let records_for_invitee = invite_records
        .iter()
        .filter(|record| record.invitee_user_id == invitee_user_id)
        .collect::<Vec<_>>();

    if !records_for_invitee.is_empty() {
        return records_for_invitee.into_iter().find_map(|record| {
            if !matches!(record.status, InviteStatus::Active) || !record.rebate_enabled {
                return None;
            }
            let inviter = users_by_id.get(record.inviter_user_id.as_str())?;
            eligible_rebate_agent(inviter, invitee_user_id).then(|| (*inviter).clone())
        });
    }

    let invitee = users_by_id.get(invitee_user_id)?;
    let agent_id = invitee.agent_id.as_deref()?;
    let agent = users_by_id.get(agent_id)?;
    eligible_rebate_agent(agent, invitee_user_id).then(|| (*agent).clone())
}

/// 校验返利接收方必须是有效代理，普通用户的邀请码不会产生充值返利。
fn eligible_rebate_agent(user: &UserSummary, invitee_user_id: &str) -> bool {
    user.id != invitee_user_id
        && matches!(user.kind, UserKind::Agent)
        && matches!(user.status, UserStatus::Active)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            recharge::{RechargeChannel, RechargeOrderStatus, RechargeOrderSummary},
            user::{UserKind, UserStatus, UserSummary},
        },
        services::{
            access::AccessRepository, business_database::BusinessDatabase,
            finance::FinanceRepository, invite::InviteRepository,
        },
    };
    /// 验证返利仓储可以更新邀请返利策略。
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
    /// 验证关闭状态邀请记录不能产生返利。
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
    /// 验证返利比例不能超过完整充值金额。
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
    /// 验证充值返利只会给启用代理发放一次。
    #[tokio::test]
    async fn recharge_rebate_is_paid_to_active_agent_once() {
        let access = AccessRepository::memory_seeded();
        let invites = InviteRepository::memory_seeded();
        let rebates = RebateRepository::memory_seeded();
        let finance = FinanceRepository::memory_seeded();
        let order = recharge_order("U10001", 10_000);

        let entry = credit_recharge_rebate_for_order(&access, &invites, &rebates, &finance, &order)
            .await
            .expect("recharge rebate should be processed")
            .expect("active agent invite should receive rebate");
        let repeated =
            credit_recharge_rebate_for_order(&access, &invites, &rebates, &finance, &order)
                .await
                .expect("recharge rebate should stay idempotent")
                .expect("existing rebate entry should be returned");
        let account = finance
            .account_or_create("U90001")
            .await
            .expect("agent account should exist");

        assert_eq!(entry.id, repeated.id);
        assert_eq!(entry.amount_minor, 350);
        assert_eq!(account.available_balance_minor, 520_350);
    }
    /// 验证禁用邀请记录不会回退到注册代理关系发放返利。
    #[tokio::test]
    async fn recharge_rebate_skips_disabled_invite_record_without_agent_link_fallback() {
        let access = AccessRepository::memory_seeded();
        let invites = InviteRepository::memory_seeded();
        let rebates = RebateRepository::memory_seeded();
        let finance = FinanceRepository::memory_seeded();
        let order = recharge_order("U10004", 10_000);

        let entry = credit_recharge_rebate_for_order(&access, &invites, &rebates, &finance, &order)
            .await
            .expect("recharge rebate skip should not fail");
        let account = finance
            .account_or_create("U90001")
            .await
            .expect("agent account should exist");

        assert!(entry.is_none());
        assert_eq!(account.available_balance_minor, 520_000);
    }
    /// 验证没有人工邀请记录时使用注册代理关系返利。
    #[tokio::test]
    async fn recharge_rebate_uses_registered_agent_link_when_no_manual_record_exists() {
        let access = AccessRepository::memory_seeded();
        access
            .create_user(UserSummary {
                id: "U20010".to_string(),
                username: "linked_invitee".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: Some("U90001".to_string()),
                invite_code: "ZXCV1234".to_string(),
                registration_location: crate::domain::user::UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            })
            .await
            .expect("linked user can be created");
        let invites = InviteRepository::memory_seeded();
        let rebates = RebateRepository::memory_seeded();
        let finance = FinanceRepository::memory_seeded();
        let order = recharge_order("U20010", 20_000);

        let entry = credit_recharge_rebate_for_order(&access, &invites, &rebates, &finance, &order)
            .await
            .expect("recharge rebate should be processed")
            .expect("agent link should receive rebate");

        assert_eq!(entry.user_id, "U90001");
        assert_eq!(entry.amount_minor, 700);
    }
    /// 验证配置测试数据库时返利策略可持久化。
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
    /// 构造返利测试充值订单。
    fn recharge_order(user_id: &str, amount_minor: i64) -> RechargeOrderSummary {
        RechargeOrderSummary {
            id: "R000000000001".to_string(),
            user_id: user_id.to_string(),
            username: "demo_user".to_string(),
            channel: RechargeChannel::CustomerService,
            amount_minor,
            status: RechargeOrderStatus::Paid,
            pay_type: None,
            provider_trade_no: Some("测试收款".to_string()),
            payment_url: None,
            support_conversation_id: None,
            remark: String::new(),
            created_at: "2026-06-06 03:00:00".to_string(),
            paid_at: Some("2026-06-06 03:00:00".to_string()),
        }
    }
}
