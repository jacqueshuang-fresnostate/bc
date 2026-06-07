//! 提现领域模型，定义用户提现申请、状态和接口载荷

use serde::{Deserialize, Serialize};

use crate::domain::user::WithdrawalMethodType;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 提现申请状态，描述待审核、已通过、已驳回和已取消。
pub enum WithdrawalOrderStatus {
    Pending,
    Approved,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户提交提现申请时选择的提现方式和金额。
pub struct CreateWithdrawalOrderRequest {
    pub method_id: String,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 提现申请摘要，保存收款方式快照、金额和审核状态。
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
