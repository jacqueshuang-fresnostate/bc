//! 财务领域模型，定义账户汇总、流水与账户调整参数

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台财务首页的余额、提现、充值和派奖统计。
pub struct FinanceOverview {
    pub total_balance_minor: i64,
    pub pending_withdraw_minor: i64,
    pub today_recharge_minor: i64,
    pub today_payout_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户资金账户摘要，区分可用余额和冻结余额。
pub struct FinancialAccountSummary {
    pub user_id: String,
    pub available_balance_minor: i64,
    pub frozen_balance_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台资金账户列表展示项，在账户摘要外附带用户名。
pub struct AdminFinancialAccountSummary {
    pub user_id: String,
    pub username: Option<String>,
    pub available_balance_minor: i64,
    pub frozen_balance_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台财务列表通用分页结构。
pub struct FinancePage<T> {
    pub items: Vec<T>,
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 资金流水类型，标识每一笔余额变动对应的业务来源。
pub enum LedgerEntryKind {
    AgentRebateWithdrawal,
    ManualAdjustment,
    OrderDebit,
    OrderRefund,
    PayoutCredit,
    RechargeCredit,
    RechargeRebateCredit,
    WithdrawalFreeze,
    WithdrawalPayout,
    WithdrawalReject,
    GroupBuyDebit,
    GroupBuyRefund,
    RedPacketDebit,
    RedPacketCredit,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单笔资金流水，记录金额变动、变动后余额和业务引用。
pub struct LedgerEntry {
    pub id: String,
    pub user_id: String,
    pub kind: LedgerEntryKind,
    pub amount_minor: i64,
    pub balance_after_minor: i64,
    pub reference_id: Option<String>,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台手动调账请求，只能通过财务功能修改用户余额。
pub struct ManualBalanceAdjustmentRequest {
    pub user_id: String,
    pub amount_minor: i64,
    pub description: String,
}
