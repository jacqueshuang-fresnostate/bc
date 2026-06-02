use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RebateMode {
    Immediate,
    RechargeTiered,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitePolicySummary {
    pub agents_can_invite: bool,
    pub regular_users_can_invite: bool,
    pub rebate_mode: RebateMode,
    pub supported_rebate_modes: Vec<RebateMode>,
    pub default_recharge_rebate_basis_points: u16,
}
