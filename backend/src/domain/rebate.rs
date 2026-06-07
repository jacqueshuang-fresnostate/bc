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
