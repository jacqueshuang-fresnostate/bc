//! 返利领域模型，定义邀请返利模式与配置更新参数

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 返利模式，决定充值返利是即时发放还是按后续规则扩展。
pub enum RebateMode {
    Immediate,
    RechargeTiered,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 邀请与返利策略摘要，供后台设置页和手机端邀请中心读取。
pub struct InvitePolicySummary {
    pub agents_can_invite: bool,
    pub regular_users_can_invite: bool,
    pub rebate_mode: RebateMode,
    pub supported_rebate_modes: Vec<RebateMode>,
    pub default_recharge_rebate_basis_points: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台更新邀请权限和默认充值返利比例时提交的请求。
pub struct InvitePolicyUpdateRequest {
    pub agents_can_invite: bool,
    pub regular_users_can_invite: bool,
    pub rebate_mode: RebateMode,
    pub default_recharge_rebate_basis_points: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台代理返利统计项，汇总代理下级充值返利和已处理提现金额。
pub struct AgentRebateSummary {
    pub agent_user_id: String,
    pub agent_username: String,
    pub invite_code: String,
    pub direct_invitee_count: usize,
    pub direct_invitee_withdrawal_minor: i64,
    pub rebate_record_count: usize,
    pub total_rebate_minor: i64,
    pub withdrawn_rebate_minor: i64,
    pub pending_rebate_minor: i64,
    pub withdrawable_rebate_minor: i64,
    pub account_available_balance_minor: i64,
    pub last_rebate_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台代理返利明细，按充值返利流水还原下级用户、充值单和返利金额。
pub struct AgentRebateRecord {
    pub ledger_entry_id: String,
    pub agent_user_id: String,
    pub agent_username: String,
    pub invitee_user_id: Option<String>,
    pub invitee_username: Option<String>,
    pub invitee_total_withdrawal_minor: i64,
    pub recharge_order_id: Option<String>,
    pub recharge_amount_minor: Option<i64>,
    pub rebate_amount_minor: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台处理代理返利提现时提交的金额和备注。
pub struct AgentRebateWithdrawalRequest {
    pub amount_minor: i64,
    pub description: String,
}
