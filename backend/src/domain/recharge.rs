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
    pub channel: RechargeChannel,
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub pay_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户端充值配置响应，包含可用渠道和金额限制。
pub struct RechargeConfigResponse {
    pub channels: Vec<RechargeChannelConfig>,
    pub min_amount_minor: i64,
    pub max_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户创建充值订单时提交的渠道、金额和支付方式。
pub struct CreateRechargeOrderRequest {
    pub channel: RechargeChannel,
    pub amount_minor: i64,
    #[serde(default)]
    pub pay_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台或回调确认充值订单时提交的第三方交易号。
pub struct ConfirmRechargeOrderRequest {
    #[serde(default)]
    pub provider_trade_no: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 充值订单摘要，供后台和用户端订单列表展示。
pub struct RechargeOrderSummary {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub channel: RechargeChannel,
    pub amount_minor: i64,
    pub status: RechargeOrderStatus,
    pub pay_type: Option<String>,
    pub provider_trade_no: Option<String>,
    pub payment_url: Option<String>,
    pub support_conversation_id: Option<String>,
    pub created_at: String,
    pub paid_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 创建充值订单后的响应，包含跳转支付地址或客服会话编号。
pub struct CreateRechargeOrderResponse {
    pub order: RechargeOrderSummary,
    pub payment_url: Option<String>,
    pub support_conversation_id: Option<String>,
    pub message: String,
}
