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
    /// 提现方式 ID。
    pub method_id: String,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 提现申请摘要，保存收款方式快照、金额和审核状态。
pub struct WithdrawalOrderSummary {
    /// 提现订单 ID。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: String,
    /// 提现方式 ID。
    pub method_id: String,
    /// 提现方式类型。
    pub method_type: WithdrawalMethodType,
    /// 收款人姓名。
    pub account_holder: String,
    /// 收款账号或银行卡号。
    pub account_number: String,
    /// 银行卡开户行名称；非银行卡方式可为空。
    pub bank_name: Option<String>,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: WithdrawalOrderStatus,
    /// 创建时间。
    pub created_at: String,
    /// 审核完成时间。
    pub reviewed_at: Option<String>,
}
