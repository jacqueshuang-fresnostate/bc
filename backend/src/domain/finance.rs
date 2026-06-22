//! 财务领域模型，定义账户汇总、流水与账户调整参数

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台财务首页的余额、提现、充值和派奖统计。
pub struct FinanceOverview {
    /// 平台全部账户余额合计，单位为分。
    pub total_balance_minor: i64,
    /// 待处理提现金额，单位为分。
    pub pending_withdraw_minor: i64,
    /// 今日充值金额合计，单位为分。
    pub today_recharge_minor: i64,
    /// 今日派奖金额合计，单位为分。
    pub today_payout_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户资金账户摘要，区分可用余额和冻结余额。
pub struct FinancialAccountSummary {
    /// 资金账户所属用户 ID。
    pub user_id: String,
    /// 可用余额，单位为分。
    pub available_balance_minor: i64,
    /// 冻结余额，单位为分。
    pub frozen_balance_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台资金账户列表展示项，在账户摘要外附带用户名。
pub struct AdminFinancialAccountSummary {
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: Option<String>,
    /// 上级代理用户 ID；没有上级代理时为空。
    pub agent_id: Option<String>,
    /// 上级代理用户名；代理账号缺失时为空，前端可结合代理 ID 显示未知代理。
    pub agent_username: Option<String>,
    /// 可用余额，单位为分。
    pub available_balance_minor: i64,
    /// 冻结余额，单位为分。
    pub frozen_balance_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户提现流水门槛累计摘要，用于判断充值后是否完成等额有效投注。
pub struct WithdrawalTurnoverSummary {
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户累计真实充值本金，单位为分。
    pub cumulative_recharge_minor: i64,
    /// 当前需要完成的有效投注金额，单位为分。
    pub required_effective_bet_minor: i64,
    /// 已完成有效投注金额，单位为分。
    pub completed_effective_bet_minor: i64,
    /// 距离可提现还差的有效投注金额，单位为分。
    pub remaining_effective_bet_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台财务列表通用分页结构。
pub struct FinancePage<T> {
    /// 分页数据列表。
    pub items: Vec<T>,
    /// 符合条件的总记录数。
    pub total_count: usize,
    /// 当前页码，从 1 开始。
    pub page: usize,
    /// 每页记录数量。
    pub page_size: usize,
    /// 总页数。
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
    RechargeBonusCredit,
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
    /// 业务唯一标识。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 业务类型。
    pub kind: LedgerEntryKind,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 余额afterminor字段。
    pub balance_after_minor: i64,
    /// referenceid字段。
    pub reference_id: Option<String>,
    /// 配置或记录的中文说明。
    pub description: String,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台手动调账请求，只能通过财务功能修改用户余额。
pub struct ManualBalanceAdjustmentRequest {
    /// 关联用户 ID。
    pub user_id: String,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 配置或记录的中文说明。
    pub description: String,
}
