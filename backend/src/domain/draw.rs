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
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩票期号。
    pub issue: String,
    /// 计划开奖时间。
    pub scheduled_at: String,
    /// 封盘时间。
    pub sale_closed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台生成单个下一期开奖期号时使用的参数。
pub struct GenerateDrawIssueRequest {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 当前业务时间字符串。
    pub now: String,
    /// 开奖前封盘提前秒数。
    #[serde(default)]
    pub sale_close_lead_seconds: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台批量生成未来期开奖期号时使用的参数。
pub struct GenerateDrawIssuesRequest {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 当前业务时间字符串。
    pub now: String,
    /// 本次需要生成或处理的数量。
    pub count: u32,
    /// 开奖前封盘提前秒数。
    #[serde(default)]
    pub sale_close_lead_seconds: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 期号生成预览结果，用于前端确认即将生成的期号和封盘时间。
pub struct DrawIssueGenerationPreview {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 彩票期号。
    pub issue: String,
    /// 号码类型，决定开奖号码长度和玩法目录。
    pub number_type: LotteryNumberType,
    /// 开奖模式。
    pub draw_mode: DrawMode,
    /// 计划开奖时间。
    pub scheduled_at: String,
    /// 封盘时间。
    pub sale_closed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 外部 API 开奖源同步时返回的开奖源期号快照。
pub struct ApiDrawSourceIssueSnapshot {
    /// 外部开奖源最近已开奖期号。
    pub latest_issue: String,
    /// 外部开奖源最近期开奖时间。
    pub latest_draw_time: Option<String>,
    /// 外部开奖源提示的下一期期号。
    pub next_issue: Option<String>,
    /// 外部开奖源提示的下一期开奖时间。
    pub next_draw_time: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// API 开奖源单次采集快照摘要，供后台审计第三方期号、开奖号码和原始响应。
pub struct ApiDrawSourceCrawlSnapshotSummary {
    /// 采集快照编号。
    pub id: String,
    /// 开奖源配置编号。
    pub source_id: String,
    /// 开奖源采集时的名称快照。
    pub source_name: String,
    /// 开奖源供应商类型。
    pub provider: String,
    /// 本系统彩种 ID。
    pub lottery_id: String,
    /// 采集用途，`latestIssue` 表示最新期号，`drawNumber` 表示按期号取开奖号码。
    pub request_kind: String,
    /// 按期号取开奖号码时请求的期号。
    pub requested_issue: Option<String>,
    /// 第三方接口解析出的最新已开奖期号。
    pub latest_issue: Option<String>,
    /// 第三方接口解析出的最新已开奖时间。
    pub latest_draw_time: Option<String>,
    /// 第三方接口解析出的下一期期号。
    pub next_issue: Option<String>,
    /// 第三方接口解析出的下一期开奖时间。
    pub next_draw_time: Option<String>,
    /// 第三方接口解析出的开奖号码，统一用英文逗号分隔。
    pub draw_number: Option<String>,
    /// 本次实际请求的接口地址。
    pub endpoint: String,
    /// 本次使用的开奖源编码。
    pub lot_code: String,
    /// 第三方接口 HTTP 状态码。
    pub http_status: Option<i32>,
    /// 本次请求和解析是否成功。
    pub success: bool,
    /// 失败时保存的错误信息。
    pub error_message: Option<String>,
    /// 可解析为 JSON 的原始响应。
    pub raw_response: Option<serde_json::Value>,
    /// 原始响应文本，非 JSON 响应也会保存。
    pub raw_response_text: String,
    /// 快照写入数据库的时间。
    pub crawled_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
/// API 开奖源采集快照分页响应，供后台开奖源比对页面使用。
pub struct ApiDrawSourceCrawlSnapshotPage {
    /// 分页数据列表。
    pub items: Vec<ApiDrawSourceCrawlSnapshotSummary>,
    /// 符合筛选条件的总记录数。
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
/// 后台手动同步 API 开奖源后的本地校准结果。
pub struct DrawSourceSyncResult {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 第三方开奖源快照。
    pub api_snapshot: ApiDrawSourceIssueSnapshot,
    /// 本次校准匹配到的本地期号。
    pub target_issue: DrawIssue,
    /// 本次生成的新期号列表。
    pub generated_issues: Vec<DrawIssue>,
    /// 本次更新的期号列表。
    pub updated_issues: Vec<DrawIssue>,
    /// 本次取消的期号列表。
    pub cancelled_issues: Vec<DrawIssue>,
    /// 本次保持不变的期号列表。
    pub kept_issues: Vec<DrawIssue>,
    /// 返回给前端或日志展示的中文消息。
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
/// 手动开奖或控奖提交的开奖号码请求。
pub struct DrawIssueResultRequest {
    /// 开奖号码，使用英文逗号分隔。
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
    /// 功能开关。
    pub enabled: bool,
    /// 开奖号码，使用英文逗号分隔。
    #[serde(default)]
    pub draw_number: Option<String>,
    /// 控奖生效范围。
    #[serde(default)]
    pub target_scope: DrawControlTargetScope,
    /// 本次校准匹配到的本地期号。
    #[serde(default)]
    pub target_issue: Option<String>,
    /// 指定控单时关联的订单 ID。
    #[serde(default)]
    pub target_order_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台彩种控制台展示的开奖控制配置。
pub struct LotteryDrawControl {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 号码类型，决定开奖号码长度和玩法目录。
    pub number_type: LotteryNumberType,
    /// 功能开关。
    pub enabled: bool,
    /// 开奖号码，使用英文逗号分隔。
    pub draw_number: Option<String>,
    /// 控奖生效范围。
    pub target_scope: DrawControlTargetScope,
    /// 本次校准匹配到的本地期号。
    pub target_issue: Option<String>,
    /// 指定控单时关联的订单 ID。
    pub target_order_id: Option<String>,
    /// 最后更新时间。
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个开奖期号，包含彩种、计划开奖时间、封盘时间和开奖结果。
pub struct DrawIssue {
    /// 开奖期号记录 ID。
    pub id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 彩票期号。
    pub issue: String,
    /// 号码类型，决定开奖号码长度和玩法目录。
    pub number_type: LotteryNumberType,
    /// 开奖模式。
    pub draw_mode: DrawMode,
    /// 计划开奖时间。
    pub scheduled_at: String,
    /// 封盘时间。
    pub sale_closed_at: String,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: DrawIssueStatus,
    /// 开奖号码，使用英文逗号分隔。
    pub draw_number: Option<String>,
    /// 实际开奖完成时间。
    pub drawn_at: Option<String>,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 开奖期号分页响应，供后台期号管理列表使用。
pub struct DrawIssuePage {
    /// 分页数据列表。
    pub items: Vec<DrawIssue>,
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
/// 后台手动触发开奖自动化时传入的当前时间。
pub struct DrawAutomationRunRequest {
    /// 当前业务时间字符串。
    pub now: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 开奖自动化跳过的期号及原因，便于后台排查调度结果。
pub struct DrawAutomationSkippedIssue {
    /// 开奖期号记录 ID。
    pub draw_issue_id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩票期号。
    pub issue: String,
    /// 申请、审核或跳过原因。
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单轮开奖自动化执行结果，汇总封盘、开奖、结算和资金流水。
pub struct DrawAutomationRun {
    /// 当前业务时间字符串。
    pub now: String,
    /// 本轮自动封盘的期号列表。
    pub closed_issues: Vec<DrawIssue>,
    /// 本轮完成开奖的期号列表。
    pub drawn_issues: Vec<DrawIssue>,
    /// 本轮生成的结算记录。
    pub settlement_runs: Vec<SettlementRun>,
    /// 本轮生成的资金流水。
    pub ledger_entries: Vec<LedgerEntry>,
    /// 本轮跳过处理的期号及原因。
    pub skipped_issues: Vec<DrawAutomationSkippedIssue>,
}
