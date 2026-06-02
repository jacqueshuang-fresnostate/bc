use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinanceOverview {
    pub total_balance_minor: i64,
    pub pending_withdraw_minor: i64,
    pub today_recharge_minor: i64,
    pub today_payout_minor: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialAccountSummary {
    pub user_id: String,
    pub available_balance_minor: i64,
    pub frozen_balance_minor: i64,
}
