//! 开奖期号与开奖控制领域模型，定义状态与开奖请求参数

use serde::{Deserialize, Serialize};

use crate::domain::{
    finance::LedgerEntry,
    lottery::{DrawMode, LotteryNumberType},
    settlement::SettlementRun,
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 开奖期号状态，控制下注、封盘、开奖和撤销流程。
pub enum DrawIssueStatus {
    Open,
    Closed,
    Drawn,
    Cancelled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台手动创建开奖期号时提交的基础排期信息。
pub struct CreateDrawIssueRequest {
    pub lottery_id: String,
    pub issue: String,
    pub scheduled_at: String,
    pub sale_closed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台生成单个下一期开奖期号时使用的参数。
pub struct GenerateDrawIssueRequest {
    pub lottery_id: String,
    pub now: String,
    #[serde(default)]
    pub sale_close_lead_seconds: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台批量生成未来期开奖期号时使用的参数。
pub struct GenerateDrawIssuesRequest {
    pub lottery_id: String,
    pub now: String,
    pub count: u32,
    #[serde(default)]
    pub sale_close_lead_seconds: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 期号生成预览结果，用于前端确认即将生成的期号和封盘时间。
pub struct DrawIssueGenerationPreview {
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub number_type: LotteryNumberType,
    pub draw_mode: DrawMode,
    pub scheduled_at: String,
    pub sale_closed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
/// 手动开奖或控奖提交的开奖号码请求。
pub struct DrawIssueResultRequest {
    #[serde(default)]
    pub draw_number: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 开奖控制范围，支持按彩种、指定期号或指定订单所在期号生效。
pub enum DrawControlTargetScope {
    Lottery,
    Issue,
    Order,
}

/// 开奖控制范围的默认值实现。
impl Default for DrawControlTargetScope {
    /// 默认沿用旧版本按彩种整体生效的控制范围。
    fn default() -> Self {
        Self::Lottery
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台保存彩种开奖控制号码时提交的配置。
pub struct SaveLotteryDrawControlRequest {
    pub enabled: bool,
    #[serde(default)]
    pub draw_number: Option<String>,
    #[serde(default)]
    pub target_scope: DrawControlTargetScope,
    #[serde(default)]
    pub target_issue: Option<String>,
    #[serde(default)]
    pub target_order_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台彩种控制台展示的开奖控制配置。
pub struct LotteryDrawControl {
    pub lottery_id: String,
    pub lottery_name: String,
    pub number_type: LotteryNumberType,
    pub enabled: bool,
    pub draw_number: Option<String>,
    pub target_scope: DrawControlTargetScope,
    pub target_issue: Option<String>,
    pub target_order_id: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个开奖期号，包含彩种、计划开奖时间、封盘时间和开奖结果。
pub struct DrawIssue {
    pub id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub number_type: LotteryNumberType,
    pub draw_mode: DrawMode,
    pub scheduled_at: String,
    pub sale_closed_at: String,
    pub status: DrawIssueStatus,
    pub draw_number: Option<String>,
    pub drawn_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 开奖期号分页响应，供后台期号管理列表使用。
pub struct DrawIssuePage {
    pub items: Vec<DrawIssue>,
    pub total_count: usize,
    pub page: usize,
    pub page_size: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台手动触发开奖自动化时传入的当前时间。
pub struct DrawAutomationRunRequest {
    pub now: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 开奖自动化跳过的期号及原因，便于后台排查调度结果。
pub struct DrawAutomationSkippedIssue {
    pub draw_issue_id: String,
    pub lottery_id: String,
    pub issue: String,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单轮开奖自动化执行结果，汇总封盘、开奖、结算和资金流水。
pub struct DrawAutomationRun {
    pub now: String,
    pub closed_issues: Vec<DrawIssue>,
    pub drawn_issues: Vec<DrawIssue>,
    pub settlement_runs: Vec<SettlementRun>,
    pub ledger_entries: Vec<LedgerEntry>,
    pub skipped_issues: Vec<DrawAutomationSkippedIssue>,
}
