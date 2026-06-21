//! 充值领域模型，定义充值渠道、订单状态和用户端返回结构

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 充值渠道，区分彩虹易支付和客服直充。
pub enum RechargeChannel {
    RainbowEpay,
    CustomerService,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 充值订单状态，描述待支付、等待客服、已入账和已取消。
pub enum RechargeOrderStatus {
    Pending,
    WaitingCustomerService,
    Paid,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个充值渠道的前台展示配置。
pub struct RechargeChannelConfig {
    /// 充值渠道类型。
    pub channel: RechargeChannel,
    /// 展示名称。
    pub name: String,
    /// 功能开关。
    pub enabled: bool,
    /// 配置或记录的中文说明。
    pub description: String,
    /// 该渠道支持的支付类型列表。
    pub pay_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 充值赠送活动档位，按单笔充值金额匹配固定赠送彩金。
pub struct RechargeBonusRule {
    /// 触发赠送的单笔充值门槛，单位为分。
    pub threshold_amount_minor: i64,
    /// 达到门槛后赠送的金额，单位为分。
    pub bonus_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户端充值配置响应，包含可用渠道和金额限制。
pub struct RechargeConfigResponse {
    /// 可用充值渠道配置列表。
    pub channels: Vec<RechargeChannelConfig>,
    /// minamountminor字段。
    pub min_amount_minor: i64,
    /// maxamountminor字段。
    pub max_amount_minor: i64,
    /// 充值赠送活动是否开启。
    pub bonus_enabled: bool,
    /// 充值赠送活动档位列表，按充值门槛从低到高返回。
    pub bonus_rules: Vec<RechargeBonusRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户创建充值订单时提交的渠道、金额和支付方式。
pub struct CreateRechargeOrderRequest {
    /// 充值渠道类型。
    pub channel: RechargeChannel,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 彩虹易支付等渠道的具体支付类型。
    #[serde(default)]
    pub pay_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台或回调确认充值订单时提交的第三方交易号。
pub struct ConfirmRechargeOrderRequest {
    /// 第三方支付平台交易号。
    #[serde(default)]
    pub provider_trade_no: Option<String>,
    /// 后台确认入账备注，便于财务核对付款凭证或线下沟通结果。
    #[serde(default)]
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 充值订单摘要，供后台和用户端订单列表展示。
pub struct RechargeOrderSummary {
    /// 充值订单 ID。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: String,
    /// 充值渠道类型。
    pub channel: RechargeChannel,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: RechargeOrderStatus,
    /// 彩虹易支付等渠道的具体支付类型。
    pub pay_type: Option<String>,
    /// 第三方支付平台交易号。
    pub provider_trade_no: Option<String>,
    /// 支付跳转或收银台链接。
    pub payment_url: Option<String>,
    /// 关联客服直充会话 ID。
    pub support_conversation_id: Option<String>,
    /// 后台确认入账备注。
    pub remark: String,
    /// 创建时间。
    pub created_at: String,
    /// 支付完成时间。
    pub paid_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 创建充值订单后的响应，包含跳转支付地址或客服会话编号。
pub struct CreateRechargeOrderResponse {
    /// 订单字段。
    pub order: RechargeOrderSummary,
    /// 支付跳转或收银台链接。
    pub payment_url: Option<String>,
    /// 关联客服直充会话 ID。
    pub support_conversation_id: Option<String>,
    /// 返回给前端或日志展示的中文消息。
    pub message: String,
}
