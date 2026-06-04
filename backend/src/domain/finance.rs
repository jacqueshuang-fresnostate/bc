//! 财务领域模型，定义账户汇总、流水与账户调整参数

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinanceOverview {
    pub total_balance_minor: i64,
    pub pending_withdraw_minor: i64,
    pub today_recharge_minor: i64,
    pub today_payout_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinancialAccountSummary {
    pub user_id: String,
    pub available_balance_minor: i64,
    pub frozen_balance_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminFinancialAccountSummary {
    pub user_id: String,
    pub username: Option<String>,
    pub available_balance_minor: i64,
    pub frozen_balance_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FinancePage<T> {
    pub items: Vec<T>,
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LedgerEntryKind {
    ManualAdjustment,
    OrderDebit,
    OrderRefund,
    PayoutCredit,
    RechargeCredit,
    WithdrawalFreeze,
    WithdrawalPayout,
    WithdrawalReject,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub struct ManualBalanceAdjustmentRequest {
    pub user_id: String,
    pub amount_minor: i64,
    pub description: String,
}
