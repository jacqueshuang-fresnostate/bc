//! 用户与管理员领域模型，定义状态、查询和编辑参数

use serde::{Deserialize, Serialize};

use crate::domain::{finance::FinancialAccountSummary, invite::InviteStatus, rebate::RebateMode};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户类型，区分普通会员和具备邀请能力的代理。
pub enum UserKind {
    Regular,
    Agent,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户或管理员账号状态，控制是否允许登录和使用系统。
pub enum UserStatus {
    Active,
    Suspended,
    Locked,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户摘要，后台和用户端都通过它展示账号基础信息。
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
/// 用户注册请求，支持用户名注册、邮箱注册和可选邀请码。
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
/// 用户登录请求，login_key 可以是用户名或邮箱。
pub struct UserLoginRequest {
    pub login_key: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户绑定邮箱时提交的新邮箱地址。
pub struct UserBindEmailRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户修改登录密码时提交的旧密码和新密码。
pub struct UserChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户发起忘记密码流程时提交的登录标识。
pub struct UserForgotPasswordRequest {
    pub login_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户通过重置 token 设置新密码时提交的数据。
pub struct UserResetPasswordRequest {
    pub reset_token: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户登录成功后的会话信息，包含 Bearer token 和用户摘要。
pub struct UserAuthSession {
    pub token: String,
    pub user: UserSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户新增或编辑提现方式时提交的收款账户信息。
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
/// 用户提现方式实体，保存支付宝、微信或银行卡收款信息。
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
/// 用户登出接口响应，标记当前登录态是否已失效。
pub struct UserLogoutResponse {
    pub logged_out: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户重置密码接口响应。
pub struct UserResetPasswordResponse {
    pub reset: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 提现方式类型，决定收款字段展示和校验规则。
pub enum WithdrawalMethodType {
    Alipay,
    Wechat,
    BankCard,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户余额接口响应，同时返回用户摘要和资金账户。
pub struct UserBalanceResponse {
    pub user: UserSummary,
    pub account: FinancialAccountSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 忘记密码流程返回的临时重置 token 和过期时间。
pub struct UserForgotPasswordResponse {
    pub reset_token: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户个人资料接口响应。
pub struct UserProfileResponse {
    pub user: UserSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 邀请中心直属用户展示项，包含邀请状态、返利开关和充值汇总。
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
/// 用户邀请中心汇总响应，展示邀请码、直属用户和返利统计。
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
/// 管理员账号摘要，用于后台账号维护和认证资料返回。
pub struct AdminSummary {
    pub id: String,
    pub username: String,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台创建或编辑管理员账号时提交的资料。
pub struct AdminSaveRequest {
    pub id: String,
    pub username: String,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// 管理员保存请求的派生展示方法。
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
/// 注册开关配置，控制用户名、邮箱和代理邀请码规则。
pub struct RegistrationConfig {
    pub username_enabled: bool,
    pub email_enabled: bool,
    pub agent_invite_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台切换用户状态时提交的请求。
pub struct UserStatusRequest {
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台切换管理员状态时提交的请求。
pub struct AdminStatusRequest {
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台重置管理员密码时提交的新密码。
pub struct AdminPasswordResetRequest {
    pub password: String,
}
