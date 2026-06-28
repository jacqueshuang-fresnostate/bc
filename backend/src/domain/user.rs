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
/// 用户注册地信息，记录请求 IP、粗粒度地区和地区来源，便于后台审计注册来源。
pub struct UserRegistrationLocation {
    /// 注册请求来源 IP。
    #[serde(default)]
    pub registered_ip: String,
    /// 注册地国家或地区。
    #[serde(default)]
    pub country: String,
    /// 注册地省份或区域。
    #[serde(default)]
    pub region: String,
    /// 注册地城市。
    #[serde(default)]
    pub city: String,
    /// 注册地来源标识。
    #[serde(default)]
    pub source: String,
}

impl Default for UserRegistrationLocation {
    /// 返回默认值。
    fn default() -> Self {
        Self {
            registered_ip: String::new(),
            country: String::new(),
            region: String::new(),
            city: String::new(),
            source: "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户摘要，后台和用户端都通过它展示账号基础信息。
pub struct UserSummary {
    /// 用户 ID。
    pub id: String,
    /// 用户展示名。
    pub username: String,
    /// 邮箱地址；为空表示尚未绑定。
    pub email: Option<String>,
    /// 用户头像图片地址。
    #[serde(default)]
    pub avatar_url: String,
    /// 用户选填 QQ 联系方式。
    #[serde(default)]
    pub contact_qq: String,
    /// 业务类型。
    pub kind: UserKind,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: UserStatus,
    /// 用户当前可用余额，单位为分。
    pub balance_minor: i64,
    /// 上级代理用户 ID；为空表示没有代理关系。
    pub agent_id: Option<String>,
    /// 用户邀请码；只有代理邀请码具备邀请能力。
    #[serde(default)]
    pub invite_code: String,
    /// 注册location字段。
    #[serde(default)]
    pub registration_location: UserRegistrationLocation,
    /// 创建时间。
    #[serde(default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户注册请求，支持用户名注册、邮箱注册和可选邀请码。
pub struct UserRegisterRequest {
    /// 用户展示名。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 邮箱地址；为空表示尚未绑定。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// 用户注册 QQ 联系方式，注册时必填。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_qq: Option<String>,
    /// 用户输入的登录密码明文，仅用于请求校验和哈希生成。
    pub password: String,
    /// 用户邀请码；只有代理邀请码具备邀请能力。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invite_code: Option<String>,
    /// 注册location字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registration_location: Option<UserRegistrationLocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户登录请求，login_key 可以是用户名或邮箱。
pub struct UserLoginRequest {
    /// 登录标识，支持用户名或邮箱。
    pub login_key: String,
    /// 用户输入的登录密码明文，仅用于请求校验和哈希生成。
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户绑定邮箱时提交的新邮箱地址。
pub struct UserBindEmailRequest {
    /// 邮箱地址；为空表示尚未绑定。
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户设置头像时提交的图片链接，空字符串表示清空头像。
pub struct UserAvatarRequest {
    /// 用户头像图片地址。
    pub avatar_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户修改登录密码时提交的旧密码和新密码。
pub struct UserChangePasswordRequest {
    /// 修改密码时提交的旧密码。
    pub old_password: String,
    /// 准备写入的新登录密码。
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户发起忘记密码流程时提交的登录标识。
pub struct UserForgotPasswordRequest {
    /// 登录标识，支持用户名或邮箱。
    pub login_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户通过重置 token 设置新密码时提交的数据。
pub struct UserResetPasswordRequest {
    /// 忘记密码流程生成的重置令牌。
    pub reset_token: String,
    /// 准备写入的新登录密码。
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户登录成功后的会话信息，包含 Bearer token 和用户摘要。
pub struct UserAuthSession {
    /// 登录会话 token，接口返回后由客户端作为 Bearer 凭证使用。
    pub token: String,
    /// 用户摘要。
    pub user: UserSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户新增或编辑提现方式时提交的收款账户信息。
pub struct WithdrawalMethodRequest {
    /// 提现方式类型。
    pub method_type: WithdrawalMethodType,
    /// 收款人姓名。
    pub account_holder: String,
    /// 收款账号或银行卡号。
    pub account_number: String,
    /// 银行卡开户行名称；非银行卡方式可为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bank_name: Option<String>,
    /// 是否为默认提现方式。
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户提现方式实体，保存支付宝、微信或银行卡收款信息。
pub struct WithdrawalMethod {
    /// 业务唯一标识。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 提现方式类型。
    pub method_type: WithdrawalMethodType,
    /// 收款人姓名。
    pub account_holder: String,
    /// 收款账号或银行卡号。
    pub account_number: String,
    /// 银行卡开户行名称；非银行卡方式可为空。
    pub bank_name: Option<String>,
    /// 是否为默认提现方式。
    pub is_default: bool,
    /// 创建时间。
    pub created_at: String,
    /// 最后更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户登出接口响应，标记当前登录态是否已失效。
pub struct UserLogoutResponse {
    /// 当前会话是否已经登出。
    pub logged_out: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户重置密码接口响应。
pub struct UserResetPasswordResponse {
    /// 密码是否已经完成重置。
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
    /// 用户摘要。
    pub user: UserSummary,
    /// 用户资金账户摘要。
    pub account: FinancialAccountSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 忘记密码流程返回的临时重置 token 和过期时间。
pub struct UserForgotPasswordResponse {
    /// 忘记密码流程生成的重置令牌。
    pub reset_token: String,
    /// 重置令牌或临时凭证过期时间。
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户个人资料接口响应。
pub struct UserProfileResponse {
    /// 用户摘要。
    pub user: UserSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 邀请中心直属用户按彩种汇总的投注金额。
pub struct UserInvitationBetLotterySummary {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 订单count字段。
    pub order_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 邀请中心直属用户按彩种和玩法汇总的投注金额。
pub struct UserInvitationBetPlaySummary {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 玩法规则编码。
    pub rule_code: String,
    /// 玩法中文名称。
    pub play_name: String,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 订单count字段。
    pub order_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 邀请中心直属用户最近投注的来源类型。
pub enum UserInvitationBetSource {
    /// 用户直接在下注页独立下单。
    Direct,
    /// 用户认购或跟单合买计划。
    GroupBuy,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 邀请中心直属用户最近一笔投注摘要，用于手机端快速查看下级购买内容。
pub struct UserInvitationLatestBet {
    /// 投注订单 ID。
    pub order_id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: String,
    /// 玩法中文名称。
    pub play_name: String,
    /// 投注号码中文摘要。
    pub number_summary: String,
    /// 投注来源，区分独立下注和合买跟单。
    pub bet_source: UserInvitationBetSource,
    /// 关联合买计划 ID，仅合买跟单时存在。
    pub group_buy_plan_id: Option<String>,
    /// 合买发起人脱敏展示名，仅合买跟单时存在。
    pub group_buy_initiator_display: Option<String>,
    /// 普通下注为注数，合买认购为参与份数。
    pub stake_count: u32,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 邀请中心直属用户展示项，包含邀请状态、返利开关、资金汇总和投注画像。
pub struct UserInvitationDirectUser {
    /// 业务唯一标识。
    pub id: String,
    /// 用户展示名。
    pub username: String,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: UserStatus,
    /// 邀请状态字段。
    pub invite_status: InviteStatus,
    /// 是否允许该邀请关系产生返利。
    pub rebate_enabled: bool,
    /// 当前可用余额，单位为分。
    pub available_balance_minor: i64,
    /// totaldepositminor字段。
    pub total_deposit_minor: i64,
    /// total提现minor字段。
    pub total_withdrawal_minor: i64,
    /// totalbetamountminor字段。
    pub total_bet_amount_minor: i64,
    /// bet彩种summaries字段。
    pub bet_lottery_summaries: Vec<UserInvitationBetLotterySummary>,
    /// bet玩法summaries字段。
    pub bet_play_summaries: Vec<UserInvitationBetPlaySummary>,
    /// 最新bet字段。
    pub latest_bet: Option<UserInvitationLatestBet>,
    /// 投注明细列表，按时间倒序返回直属下级每一笔独立下注或合买认购。
    pub bet_records: Vec<UserInvitationLatestBet>,
    /// registeredat字段。
    pub registered_at: String,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户邀请中心汇总响应，展示邀请码、直属用户和返利统计。
pub struct UserInvitationSummaryResponse {
    /// can邀请字段。
    pub can_invite: bool,
    /// invitationcode字段。
    pub invitation_code: String,
    /// 直选count字段。
    pub direct_count: usize,
    /// active直选count字段。
    pub active_direct_count: usize,
    /// total直选depositminor字段。
    pub total_direct_deposit_minor: i64,
    /// totalpaidcommissionminor字段。
    pub total_paid_commission_minor: i64,
    /// 当前返利模式。
    pub rebate_mode: RebateMode,
    /// 默认充值返利比例，单位为万分比。
    pub default_recharge_rebate_basis_points: u16,
    /// 直选用户字段。
    pub direct_users: Vec<UserInvitationDirectUser>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 管理员账号摘要，用于后台账号维护和认证资料返回。
pub struct AdminSummary {
    /// 业务唯一标识。
    pub id: String,
    /// 用户展示名。
    pub username: String,
    /// 角色id字段。
    pub role_id: String,
    /// 角色name字段。
    pub role_name: String,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台创建或编辑管理员账号时提交的资料。
pub struct AdminSaveRequest {
    /// 业务唯一标识。
    pub id: String,
    /// 用户展示名。
    pub username: String,
    /// 角色id字段。
    pub role_id: String,
    /// 角色name字段。
    pub role_name: String,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: UserStatus,
    /// 用户输入的登录密码明文，仅用于请求校验和哈希生成。
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
    /// username启用字段。
    pub username_enabled: bool,
    /// email启用字段。
    pub email_enabled: bool,
    /// agent邀请required字段。
    pub agent_invite_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台切换用户状态时提交的请求。
pub struct UserStatusRequest {
    /// 业务状态，用于筛选、禁用或流转。
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台切换管理员状态时提交的请求。
pub struct AdminStatusRequest {
    /// 业务状态，用于筛选、禁用或流转。
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台重置管理员密码时提交的新密码。
pub struct AdminPasswordResetRequest {
    /// 用户输入的登录密码明文，仅用于请求校验和哈希生成。
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台重置普通用户登录密码时提交的新密码。
pub struct UserPasswordResetRequest {
    /// 用户输入的登录密码明文，仅用于请求校验和哈希生成。
    pub password: String,
}
