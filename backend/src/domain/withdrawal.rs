//! 提现领域模型，定义用户提现申请、状态和接口载荷

use serde::{Deserialize, Serialize};

use crate::domain::user::WithdrawalMethodType;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WithdrawalOrderStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateWithdrawalOrderRequest {
    pub method_id: String,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalOrderSummary {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub method_id: String,
    pub method_type: WithdrawalMethodType,
    pub account_holder: String,
    pub account_number: String,
    pub bank_name: Option<String>,
    pub amount_minor: i64,
    pub status: WithdrawalOrderStatus,
    pub created_at: String,
    pub reviewed_at: Option<String>,
}
