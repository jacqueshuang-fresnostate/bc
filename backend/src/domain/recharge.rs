//! 充值领域模型，定义充值渠道、订单状态和用户端返回结构

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RechargeChannel {
    RainbowEpay,
    CustomerService,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RechargeOrderStatus {
    Pending,
    WaitingCustomerService,
    Paid,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RechargeChannelConfig {
    pub channel: RechargeChannel,
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub pay_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RechargeConfigResponse {
    pub channels: Vec<RechargeChannelConfig>,
    pub min_amount_minor: i64,
    pub max_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateRechargeOrderRequest {
    pub channel: RechargeChannel,
    pub amount_minor: i64,
    #[serde(default)]
    pub pay_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmRechargeOrderRequest {
    #[serde(default)]
    pub provider_trade_no: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub struct CreateRechargeOrderResponse {
    pub order: RechargeOrderSummary,
    pub payment_url: Option<String>,
    pub support_conversation_id: Option<String>,
    pub message: String,
}
