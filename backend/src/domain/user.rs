//! 用户与管理员领域模型，定义状态、查询和编辑参数

use serde::{Deserialize, Serialize};

use crate::domain::{finance::FinancialAccountSummary, invite::InviteStatus, rebate::RebateMode};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UserKind {
    Regular,
    Agent,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UserStatus {
    Active,
    Suspended,
    Locked,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserSummary {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub kind: UserKind,
    pub status: UserStatus,
    pub balance_minor: i64,
    pub agent_id: Option<String>,
    #[serde(default)]
    pub invite_code: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserRegisterRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserLoginRequest {
    pub login_key: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserBindEmailRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserForgotPasswordRequest {
    pub login_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserResetPasswordRequest {
    pub reset_token: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserAuthSession {
    pub token: String,
    pub user: UserSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalMethodRequest {
    pub method_type: WithdrawalMethodType,
    pub account_holder: String,
    pub account_number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalMethod {
    pub id: String,
    pub user_id: String,
    pub method_type: WithdrawalMethodType,
    pub account_holder: String,
    pub account_number: String,
    pub bank_name: Option<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserLogoutResponse {
    pub logged_out: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserResetPasswordResponse {
    pub reset: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WithdrawalMethodType {
    Alipay,
    Wechat,
    BankCard,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserBalanceResponse {
    pub user: UserSummary,
    pub account: FinancialAccountSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserForgotPasswordResponse {
    pub reset_token: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileResponse {
    pub user: UserSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserInvitationDirectUser {
    pub id: String,
    pub username: String,
    pub status: UserStatus,
    pub invite_status: InviteStatus,
    pub rebate_enabled: bool,
    pub total_deposit_minor: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserInvitationSummaryResponse {
    pub can_invite: bool,
    pub invitation_code: String,
    pub direct_count: usize,
    pub active_direct_count: usize,
    pub total_direct_deposit_minor: i64,
    pub total_paid_commission_minor: i64,
    pub rebate_mode: RebateMode,
    pub default_recharge_rebate_basis_points: u16,
    pub direct_users: Vec<UserInvitationDirectUser>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminSummary {
    pub id: String,
    pub username: String,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminSaveRequest {
    pub id: String,
    pub username: String,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl AdminSaveRequest {
    /// 生成当前实体对象的汇总信息。
    pub fn summary(&self) -> AdminSummary {
        AdminSummary {
            id: self.id.clone(),
            username: self.username.clone(),
            role_id: self.role_id.clone(),
            role_name: self.role_name.clone(),
            status: self.status.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationConfig {
    pub username_enabled: bool,
    pub email_enabled: bool,
    pub agent_invite_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserStatusRequest {
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminStatusRequest {
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminPasswordResetRequest {
    pub password: String,
}
