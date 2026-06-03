//! 返利领域模型，定义邀请返利模式与配置更新参数

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RebateMode {
    Immediate,
    RechargeTiered,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvitePolicySummary {
    pub agents_can_invite: bool,
    pub regular_users_can_invite: bool,
    pub rebate_mode: RebateMode,
    pub supported_rebate_modes: Vec<RebateMode>,
    pub default_recharge_rebate_basis_points: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvitePolicyUpdateRequest {
    pub agents_can_invite: bool,
    pub regular_users_can_invite: bool,
    pub rebate_mode: RebateMode,
    pub default_recharge_rebate_basis_points: u16,
}
