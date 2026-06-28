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
    /// 代理用户是否允许邀请下级。
    pub agents_can_invite: bool,
    /// 普通用户邀请码是否允许邀请。
    pub regular_users_can_invite: bool,
    /// 当前返利模式。
    pub rebate_mode: RebateMode,
    /// 系统支持的返利模式列表。
    pub supported_rebate_modes: Vec<RebateMode>,
    /// 默认充值返利比例，单位为万分比。
    pub default_recharge_rebate_basis_points: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台更新邀请权限和默认充值返利比例时提交的请求。
pub struct InvitePolicyUpdateRequest {
    /// 代理用户是否允许邀请下级。
    pub agents_can_invite: bool,
    /// 普通用户邀请码是否允许邀请。
    pub regular_users_can_invite: bool,
    /// 当前返利模式。
    pub rebate_mode: RebateMode,
    /// 默认充值返利比例，单位为万分比。
    pub default_recharge_rebate_basis_points: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台代理返利统计项，汇总代理下级充值返利和已处理提现金额。
pub struct AgentRebateSummary {
    /// 代理用户 ID。
    pub agent_user_id: String,
    /// 代理用户名。
    pub agent_username: String,
    /// 用户邀请码；只有代理邀请码具备邀请能力。
    pub invite_code: String,
    /// 直属下级数量。
    pub direct_invitee_count: usize,
    /// 直属下级累计充值金额，单位为分。
    pub direct_invitee_recharge_minor: i64,
    /// 直属下级累计提现金额，单位为分。
    pub direct_invitee_withdrawal_minor: i64,
    /// 返利流水记录数量。
    pub rebate_record_count: usize,
    /// 累计返利金额，单位为分。
    pub total_rebate_minor: i64,
    /// 已提现返利金额，单位为分。
    pub withdrawn_rebate_minor: i64,
    /// 待处理返利金额，单位为分。
    pub pending_rebate_minor: i64,
    /// 可提现返利金额，单位为分。
    pub withdrawable_rebate_minor: i64,
    /// 代理资金账户可用余额，单位为分。
    pub account_available_balance_minor: i64,
    /// 最近一笔返利时间。
    pub last_rebate_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台代理返利详情中的直属下级汇总项，用于先定位下级再查看该下级返利流水。
pub struct AgentRebateInviteeSummary {
    /// 被邀请用户 ID。
    pub invitee_user_id: String,
    /// 被邀请用户名。
    pub invitee_username: String,
    /// 该下级累计已入账充值金额，单位为分。
    pub total_recharge_minor: i64,
    /// 该下级累计已通过提现金额，单位为分。
    pub total_withdrawal_minor: i64,
    /// 该下级带来的累计返利金额，单位为分。
    pub total_rebate_minor: i64,
    /// 该下级带来的返利流水笔数。
    pub rebate_record_count: usize,
    /// 该下级最近一笔返利时间。
    pub last_rebate_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台代理返利明细，按充值返利流水还原下级用户、充值单和返利金额。
pub struct AgentRebateRecord {
    /// 关联资金流水 ID。
    pub ledger_entry_id: String,
    /// 代理用户 ID。
    pub agent_user_id: String,
    /// 代理用户名。
    pub agent_username: String,
    /// 被邀请用户 ID。
    pub invitee_user_id: Option<String>,
    /// 被邀请用户名。
    pub invitee_username: Option<String>,
    /// 该下级累计充值金额，单位为分。
    pub invitee_total_recharge_minor: i64,
    /// 该下级累计提现金额，单位为分。
    pub invitee_total_withdrawal_minor: i64,
    /// 关联充值订单 ID。
    pub recharge_order_id: Option<String>,
    /// 关联充值金额，单位为分。
    pub recharge_amount_minor: Option<i64>,
    /// 本笔返利金额，单位为分。
    pub rebate_amount_minor: i64,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台处理代理返利提现时提交的金额和备注。
pub struct AgentRebateWithdrawalRequest {
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 配置或记录的中文说明。
    pub description: String,
}
