//! 开奖期号生成与平台号码生成服务，实现规则化期号流转

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
use std::collections::{BTreeMap, HashSet};

use crate::{
    domain::{
        draw::{
            CreateDrawIssueRequest, DrawIssue, DrawIssueGenerationPreview,
            GenerateDrawIssueRequest, GenerateDrawIssuesRequest,
        },
        lottery::{DrawMode, DrawSchedule, LotteryKind, DEFAULT_ISSUE_FORMAT_PATTERN},
    },
    error::{ApiError, ApiResult},
    services::{draw::DrawRepository, draw_api::ApiDrawSourceLatestIssue},
};

const MAX_GENERATION_COUNT: u32 = 50;
const MAX_UNIQUE_ATTEMPTS_PER_ISSUE: u32 = 100;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const MAX_ISSUE_FORMAT_PATTERN_LENGTH: usize = 96;
const MAX_RENDERED_ISSUE_LENGTH: usize = 64;
const ISSUE_FORMAT_SAMPLE: (i32, u32, u32, u32, u32, u32) = (2026, 6, 2, 20, 1, 0);

/// 生成当前开奖流程下一期的开奖期号。
pub async fn generate_next_draw_issue(
    draws: &DrawRepository,
    lottery: &LotteryKind,
    payload: GenerateDrawIssueRequest,
) -> ApiResult<DrawIssue> {
    let created = generate_draw_issue_batch(
        draws,
        lottery,
        GenerateDrawIssuesRequest {
            lottery_id: payload.lottery_id,
            now: payload.now,
            count: 1,
            sale_close_lead_seconds: payload.sale_close_lead_seconds,
        },
    )
    .await?;

    created
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::Internal("draw issue was not generated".to_string()))
}

/// 预生成一期或多期计划但不落库，用于展示。
pub async fn preview_draw_issue_generation(
    draws: &DrawRepository,
    lottery: &LotteryKind,
    payload: GenerateDrawIssuesRequest,
) -> ApiResult<Vec<DrawIssueGenerationPreview>> {
    plan_draw_issue_generation(draws, lottery, payload).await
}

/// 按 API 开奖源快照计算当前应校准到的下一期，不读取本地旧期号基线。
pub(crate) fn plan_api_draw_source_target(
    lottery: &LotteryKind,
    latest_api_issue: &ApiDrawSourceLatestIssue,
    now: &str,
    sale_close_lead_seconds: u32,
) -> ApiResult<DrawIssueGenerationPreview> {
    if lottery.draw_mode != DrawMode::Api {
        return Err(ApiError::BadRequest(
            "只有 API 开奖彩种可以同步开奖源".to_string(),
        ));
    }
    if sale_close_lead_seconds == 0 {
        return Err(ApiError::BadRequest("封盘时间必须大于 0 秒".to_string()));
    }

    let now = parse_timestamp(now, "now")?;
    let api_anchor = api_issue_anchor_from_latest(latest_api_issue)?;
    let baseline = generation_baseline(lottery, &[], Some(&api_anchor), now)?;
    let mut issue_labeler = IssueLabeler::for_api_anchor(Some(&api_anchor))?;
    let mut issue_opened_at = baseline;
    let mut scheduled_at = next_scheduled_at(&lottery.schedule, baseline)?;

    for _ in 0..MAX_UNIQUE_ATTEMPTS_PER_ISSUE {
        let label_time = issue_label_time(&lottery.schedule, issue_opened_at, scheduled_at)?;
        let issue = issue_labeler.next_issue(label_time)?;
        let sale_closed_at =
            sale_closed_at_from_draw_time(issue_opened_at, scheduled_at, sale_close_lead_seconds)?;

        if scheduled_at > now {
            return Ok(DrawIssueGenerationPreview {
                lottery_id: lottery.id.clone(),
                lottery_name: lottery.name.clone(),
                issue,
                number_type: lottery.number_type.clone(),
                draw_mode: lottery.draw_mode.clone(),
                scheduled_at: format_timestamp(scheduled_at),
                sale_closed_at: format_timestamp(sale_closed_at),
            });
        }

        issue_opened_at = scheduled_at;
        scheduled_at = next_scheduled_at(&lottery.schedule, scheduled_at)?;
    }

    Err(ApiError::Conflict("无法按开奖源生成可销售期号".to_string()))
}

/// 按批次参数生成开奖期并持久化写入。
pub async fn generate_draw_issue_batch(
    draws: &DrawRepository,
    lottery: &LotteryKind,
    payload: GenerateDrawIssuesRequest,
) -> ApiResult<Vec<DrawIssue>> {
    let plans = plan_draw_issue_generation(draws, lottery, payload).await?;
    let mut created = Vec::with_capacity(plans.len());

    for plan in plans {
        created.push(
            draws
                .create(
                    lottery,
                    CreateDrawIssueRequest {
                        lottery_id: lottery.id.clone(),
                        issue: plan.issue,
                        scheduled_at: plan.scheduled_at,
                        sale_closed_at: plan.sale_closed_at,
                    },
                )
                .await?,
        );
    }

    Ok(created)
}
/// 规划本轮需要生成的新开奖期号。
async fn plan_draw_issue_generation(
    draws: &DrawRepository,
    lottery: &LotteryKind,
    payload: GenerateDrawIssuesRequest,
) -> ApiResult<Vec<DrawIssueGenerationPreview>> {
    validate_request(lottery, &payload)?;
    let now = parse_timestamp(&payload.now, "now")?;
    let existing_issues = draws.list().await?;
    let latest_api_issue = draws.latest_api_issue_for_lottery(&lottery.id).await?;
    let api_anchor = api_issue_anchor(&lottery.id, &existing_issues, latest_api_issue.as_ref())?;
    let baseline = generation_baseline(lottery, &existing_issues, api_anchor.as_ref(), now)?;
    let sale_close_lead_seconds = payload
        .sale_close_lead_seconds
        .unwrap_or(lottery.sale_close_lead_seconds);

    if sale_close_lead_seconds == 0 {
        return Err(ApiError::BadRequest("封盘时间必须大于 0 秒".to_string()));
    }

    if payload.count == 0 || payload.count > MAX_GENERATION_COUNT {
        return Err(ApiError::BadRequest(format!(
            "draw issue generation count must be between 1 and {MAX_GENERATION_COUNT}"
        )));
    }

    let mut known_issues: HashSet<String> = existing_issues
        .iter()
        .filter(|existing| existing.lottery_id == lottery.id)
        .map(|existing| existing.issue.clone())
        .collect();
    let mut issue_labeler =
        IssueLabeler::for_lottery(lottery, &existing_issues, api_anchor.as_ref())?;
    let mut plans = Vec::with_capacity(payload.count as usize);
    let mut issue_opened_at = baseline;
    let mut scheduled_at = next_scheduled_at(&lottery.schedule, baseline)?;
    let attempt_limit = payload.count.saturating_mul(MAX_UNIQUE_ATTEMPTS_PER_ISSUE);

    for _ in 0..attempt_limit {
        let label_time = issue_label_time(&lottery.schedule, issue_opened_at, scheduled_at)?;
        let issue = issue_labeler.next_issue(label_time)?;
        let sale_closed_at =
            sale_closed_at_from_draw_time(issue_opened_at, scheduled_at, sale_close_lead_seconds)?;

        if scheduled_at > now && !known_issues.contains(&issue) {
            known_issues.insert(issue.clone());
            plans.push(DrawIssueGenerationPreview {
                lottery_id: lottery.id.clone(),
                lottery_name: lottery.name.clone(),
                issue,
                number_type: lottery.number_type.clone(),
                draw_mode: lottery.draw_mode.clone(),
                scheduled_at: format_timestamp(scheduled_at),
                sale_closed_at: format_timestamp(sale_closed_at),
            });

            if plans.len() == payload.count as usize {
                return Ok(plans);
            }
        }

        issue_opened_at = scheduled_at;
        scheduled_at = next_scheduled_at(&lottery.schedule, scheduled_at)?;
    }

    Err(ApiError::Conflict(
        "unable to generate requested unique draw issues".to_string(),
    ))
}

/// 校验请求参数并返回错误信息。
fn validate_request(lottery: &LotteryKind, payload: &GenerateDrawIssuesRequest) -> ApiResult<()> {
    if payload.lottery_id.trim().is_empty() {
        return Err(ApiError::BadRequest("lottery id is required".to_string()));
    }
    if payload.lottery_id.trim() != lottery.id {
        return Err(ApiError::BadRequest(
            "request lottery id does not match lottery".to_string(),
        ));
    }
    if payload.now.trim().is_empty() {
        return Err(ApiError::BadRequest("now time is required".to_string()));
    }

    Ok(())
}

/// 根据开奖时间和封盘提前秒数计算封盘时间；超过本期周期时按开盘时间封盘。
fn sale_closed_at_from_draw_time(
    issue_opened_at: NaiveDateTime,
    scheduled_at: NaiveDateTime,
    sale_close_lead_seconds: u32,
) -> ApiResult<NaiveDateTime> {
    if sale_close_lead_seconds == 0 {
        return Err(ApiError::BadRequest("封盘时间必须大于 0 秒".to_string()));
    }
    let configured_sale_close = scheduled_at
        .checked_sub_signed(Duration::seconds(i64::from(sale_close_lead_seconds)))
        .ok_or_else(|| ApiError::BadRequest("sale close time is out of range".to_string()))?;
    Ok(configured_sale_close.max(issue_opened_at))
}

/// 计算期号模板使用的时间；时间节点周期按开盘节点归属期号，避免 00:00 开奖被算成新一天第一期。
fn issue_label_time(
    schedule: &DrawSchedule,
    issue_opened_at: NaiveDateTime,
    scheduled_at: NaiveDateTime,
) -> ApiResult<NaiveDateTime> {
    match schedule {
        DrawSchedule::TimeNode {
            interval_seconds, ..
        } => {
            if *interval_seconds == 0 {
                return Err(ApiError::BadRequest(
                    "time node interval must be greater than zero".to_string(),
                ));
            }
            scheduled_at
                .checked_sub_signed(Duration::seconds(i64::from(*interval_seconds)))
                .ok_or_else(|| ApiError::BadRequest("issue open time is out of range".to_string()))
        }
        DrawSchedule::Periodic { .. }
        | DrawSchedule::Daily { .. }
        | DrawSchedule::Weekly { .. } => Ok(issue_opened_at.max(scheduled_at)),
    }
}

/// 查找指定彩种已有期号中的最新开奖时间。
fn latest_scheduled_at(issues: &[DrawIssue], lottery_id: &str) -> ApiResult<Option<NaiveDateTime>> {
    let mut latest = None;

    for issue in issues
        .iter()
        .filter(|issue| issue.lottery_id == lottery_id.trim())
    {
        let scheduled_at = parse_timestamp(&issue.scheduled_at, "existing scheduled time")?;
        if latest.is_none_or(|current| scheduled_at > current) {
            latest = Some(scheduled_at);
        }
    }

    Ok(latest)
}

/// 仅用外部开奖源快照建立期号锚点，供手动同步时忽略本地旧待开奖期。
fn api_issue_anchor_from_latest(
    latest_api_issue: &ApiDrawSourceLatestIssue,
) -> ApiResult<ApiIssueAnchor> {
    let latest_external_issue =
        parse_api_sequence_issue(&latest_api_issue.issue).ok_or_else(|| {
            ApiError::Internal(format!(
                "API 开奖源最新期号 `{}` 不是数字期号",
                latest_api_issue.issue
            ))
        })?;
    let latest_draw_time = latest_api_issue
        .draw_time
        .as_deref()
        .map(|value| parse_timestamp(value, "api latest draw time"))
        .transpose()?;
    let next_external_issue = latest_api_issue
        .next_issue
        .as_deref()
        .map(|value| {
            parse_api_sequence_issue(value).ok_or_else(|| {
                ApiError::Internal(format!("API 开奖源下一期 `{value}` 不是数字期号"))
            })
        })
        .transpose()?;
    let next_draw_time = latest_api_issue
        .next_draw_time
        .as_deref()
        .map(|value| parse_timestamp(value, "api next draw time"))
        .transpose()?;

    Ok(ApiIssueAnchor {
        latest_external_issue,
        latest_issue: latest_external_issue,
        latest_draw_time,
        next_external_issue,
        next_draw_time,
    })
}

#[derive(Debug, Clone, Copy)]
struct ApiIssueAnchor {
    latest_external_issue: u64,
    latest_issue: u64,
    latest_draw_time: Option<NaiveDateTime>,
    next_external_issue: Option<u64>,
    next_draw_time: Option<NaiveDateTime>,
}

/// 根据外部开奖源快照计算下一期生成锚点。
fn api_issue_anchor(
    lottery_id: &str,
    existing_issues: &[DrawIssue],
    latest_api_issue: Option<&ApiDrawSourceLatestIssue>,
) -> ApiResult<Option<ApiIssueAnchor>> {
    let Some(latest_api_issue) = latest_api_issue else {
        return Ok(None);
    };
    let latest_external_issue =
        parse_api_sequence_issue(&latest_api_issue.issue).ok_or_else(|| {
            ApiError::Internal(format!(
                "API 开奖源最新期号 `{}` 不是数字期号",
                latest_api_issue.issue
            ))
        })?;
    let latest_local_issue = existing_issues
        .iter()
        .filter(|issue| issue.lottery_id == lottery_id)
        .filter_map(|issue| parse_api_sequence_issue(&issue.issue))
        .max();
    let latest_issue = latest_local_issue
        .map(|local_issue| local_issue.max(latest_external_issue))
        .unwrap_or(latest_external_issue);
    let latest_draw_time = latest_api_issue
        .draw_time
        .as_deref()
        .map(|value| parse_timestamp(value, "api latest draw time"))
        .transpose()?;
    let next_external_issue = latest_api_issue
        .next_issue
        .as_deref()
        .map(|value| {
            parse_api_sequence_issue(value).ok_or_else(|| {
                ApiError::Internal(format!("API 开奖源下一期 `{value}` 不是数字期号"))
            })
        })
        .transpose()?;
    let next_draw_time = latest_api_issue
        .next_draw_time
        .as_deref()
        .map(|value| parse_timestamp(value, "api next draw time"))
        .transpose()?;

    Ok(Some(ApiIssueAnchor {
        latest_external_issue,
        latest_issue,
        latest_draw_time,
        next_external_issue,
        next_draw_time,
    }))
}

/// 计算生成下一期时使用的时间基线。
fn generation_baseline(
    lottery: &LotteryKind,
    existing_issues: &[DrawIssue],
    api_anchor: Option<&ApiIssueAnchor>,
    now: NaiveDateTime,
) -> ApiResult<NaiveDateTime> {
    if let (DrawSchedule::Periodic { interval_seconds }, Some(api_anchor)) =
        (&lottery.schedule, api_anchor)
    {
        let ApiIssueAnchor {
            latest_external_issue,
            latest_issue,
            latest_draw_time,
            next_external_issue,
            next_draw_time,
        } = *api_anchor;

        if let (Some(next_external_issue), Some(next_draw_time)) =
            (next_external_issue, next_draw_time)
        {
            let offset_seconds = if latest_issue >= next_external_issue {
                i64::from(*interval_seconds)
                    * issue_offset_count(latest_issue - next_external_issue)?
            } else {
                -i64::from(*interval_seconds)
                    * issue_offset_count(next_external_issue - latest_issue)?
            };
            return next_draw_time
                .checked_add_signed(Duration::seconds(offset_seconds))
                .ok_or_else(|| ApiError::BadRequest("scheduled time is out of range".to_string()));
        }

        let Some(latest_draw_time) = latest_draw_time else {
            return Ok(now);
        };
        let issue_offset = latest_issue
            .checked_sub(latest_external_issue)
            .ok_or_else(|| ApiError::Internal("API 开奖源期号序列无效".to_string()))?;
        let offset_seconds = i64::from(*interval_seconds) * issue_offset_count(issue_offset)?;
        return latest_draw_time
            .checked_add_signed(Duration::seconds(offset_seconds))
            .ok_or_else(|| ApiError::BadRequest("scheduled time is out of range".to_string()));
    }

    if let DrawSchedule::Periodic { interval_seconds } = &lottery.schedule {
        if let Some(latest_scheduled_at) = latest_scheduled_at(existing_issues, &lottery.id)? {
            if *interval_seconds == 0 {
                return Err(ApiError::BadRequest(
                    "periodic interval must be greater than zero".to_string(),
                ));
            }
            if latest_scheduled_at > now {
                return Ok(latest_scheduled_at);
            }

            let interval_seconds = i64::from(*interval_seconds);
            let elapsed_seconds = now
                .signed_duration_since(latest_scheduled_at)
                .num_seconds()
                .max(0);
            let completed_intervals = elapsed_seconds / interval_seconds;
            return latest_scheduled_at
                .checked_add_signed(Duration::seconds(completed_intervals * interval_seconds))
                .ok_or_else(|| ApiError::BadRequest("scheduled time is out of range".to_string()));
        }
    }

    let baseline = latest_scheduled_at(existing_issues, &lottery.id)?.unwrap_or(now);
    Ok(if baseline > now { baseline } else { now })
}

/// 把期号偏移量转换为安全的有符号计数。
fn issue_offset_count(issue_offset: u64) -> ApiResult<i64> {
    i64::try_from(issue_offset)
        .map_err(|_| ApiError::Internal("API 开奖源期号偏移超出范围".to_string()))
}

/// 根据彩种排期计算下一期开奖时间。
fn next_scheduled_at(schedule: &DrawSchedule, baseline: NaiveDateTime) -> ApiResult<NaiveDateTime> {
    match schedule {
        DrawSchedule::Periodic { interval_seconds } => {
            if *interval_seconds == 0 {
                return Err(ApiError::BadRequest(
                    "periodic interval must be greater than zero".to_string(),
                ));
            }
            baseline
                .checked_add_signed(Duration::seconds(i64::from(*interval_seconds)))
                .ok_or_else(|| ApiError::BadRequest("scheduled time is out of range".to_string()))
        }
        DrawSchedule::TimeNode {
            interval_seconds,
            start_time,
        } => next_time_node_at(*interval_seconds, start_time, baseline),
        DrawSchedule::Daily { time } => {
            let draw_time = parse_time(time, "daily draw time")?;
            let today = combine_date_time(baseline.date(), draw_time)?;
            if today > baseline {
                Ok(today)
            } else {
                let next_day = baseline
                    .date()
                    .checked_add_signed(Duration::days(1))
                    .ok_or_else(|| {
                        ApiError::BadRequest("scheduled date is out of range".to_string())
                    })?;
                combine_date_time(next_day, draw_time)
            }
        }
        DrawSchedule::Weekly { weekdays, time } => {
            let draw_time = parse_time(time, "weekly draw time")?;
            let weekdays = parse_weekdays(weekdays)?;
            for day_offset in 0..14 {
                let date = baseline
                    .date()
                    .checked_add_signed(Duration::days(day_offset))
                    .ok_or_else(|| {
                        ApiError::BadRequest("scheduled date is out of range".to_string())
                    })?;
                if !weekdays.contains(&date.weekday()) {
                    continue;
                }

                let candidate = combine_date_time(date, draw_time)?;
                if candidate > baseline {
                    return Ok(candidate);
                }
            }

            Err(ApiError::BadRequest(
                "weekly schedule cannot produce next draw time".to_string(),
            ))
        }
    }
}

/// 按自然时间节点计算下一期开奖时间，例如 00:00 起点、300 秒周期会生成 00:05/00:10。
fn next_time_node_at(
    interval_seconds: u32,
    start_time: &str,
    baseline: NaiveDateTime,
) -> ApiResult<NaiveDateTime> {
    if interval_seconds == 0 {
        return Err(ApiError::BadRequest(
            "time node interval must be greater than zero".to_string(),
        ));
    }
    if 86_400 % interval_seconds != 0 {
        return Err(ApiError::BadRequest(
            "time node interval must divide one day exactly".to_string(),
        ));
    }

    let start_time = parse_time(start_time, "time node start time")?;
    let start_at = combine_date_time(baseline.date(), start_time)?;
    if baseline < start_at {
        return Ok(start_at);
    }

    let interval_seconds = i64::from(interval_seconds);
    let elapsed_seconds = baseline
        .signed_duration_since(start_at)
        .num_seconds()
        .max(0);
    let next_step = elapsed_seconds
        .checked_div(interval_seconds)
        .and_then(|step| step.checked_add(1))
        .ok_or_else(|| ApiError::BadRequest("time node interval is invalid".to_string()))?;
    start_at
        .checked_add_signed(Duration::seconds(next_step * interval_seconds))
        .ok_or_else(|| ApiError::BadRequest("scheduled time is out of range".to_string()))
}

/// 按给定格式解析时间戳。
fn parse_timestamp(value: &str, label: &str) -> ApiResult<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value.trim(), TIMESTAMP_FORMAT)
        .map_err(|_| ApiError::BadRequest(format!("{label} must use YYYY-MM-DD HH:mm:ss format")))
}

/// 解析时分秒格式字符串。
fn parse_time(value: &str, label: &str) -> ApiResult<NaiveTime> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M:%S")
        .map_err(|_| ApiError::BadRequest(format!("{label} must use HH:mm:ss format")))
}

/// 解析并标准化周几配置。
fn parse_weekdays(values: &[String]) -> ApiResult<Vec<Weekday>> {
    if values.is_empty() {
        return Err(ApiError::BadRequest(
            "weekly weekdays are required".to_string(),
        ));
    }

    values
        .iter()
        .map(|value| match value.trim() {
            "Monday" => Ok(Weekday::Mon),
            "Tuesday" => Ok(Weekday::Tue),
            "Wednesday" => Ok(Weekday::Wed),
            "Thursday" => Ok(Weekday::Thu),
            "Friday" => Ok(Weekday::Fri),
            "Saturday" => Ok(Weekday::Sat),
            "Sunday" => Ok(Weekday::Sun),
            weekday => Err(ApiError::BadRequest(format!(
                "unsupported weekly weekday `{weekday}`"
            ))),
        })
        .collect()
}

/// 合并日期和时间并转换为本地业务时间。
fn combine_date_time(date: NaiveDate, time: NaiveTime) -> ApiResult<NaiveDateTime> {
    Ok(date.and_time(time))
}

/// 按固定格式转换输出。
fn format_timestamp(value: NaiveDateTime) -> String {
    value.format(TIMESTAMP_FORMAT).to_string()
}

/// 标准化平台期开奖期号格式，空值回退到默认日期序号格式。
pub(crate) fn normalize_issue_format_pattern(value: &str) -> ApiResult<String> {
    let pattern = value.trim();
    let pattern = if pattern.is_empty() {
        DEFAULT_ISSUE_FORMAT_PATTERN
    } else {
        pattern
    };
    validate_issue_format_pattern(pattern)?;
    Ok(pattern.to_string())
}

/// 校验期号格式模板，提前发现不支持的变量或会生成非法期号的配置。
fn validate_issue_format_pattern(pattern: &str) -> ApiResult<()> {
    if pattern.len() > MAX_ISSUE_FORMAT_PATTERN_LENGTH {
        return Err(ApiError::BadRequest(
            "期号生成格式不能超过 96 个字符".to_string(),
        ));
    }
    if !contains_supported_issue_token(pattern) {
        return Err(ApiError::BadRequest(
            "期号生成格式至少需要包含一个日期或时间变量".to_string(),
        ));
    }

    let sample = NaiveDate::from_ymd_opt(
        ISSUE_FORMAT_SAMPLE.0,
        ISSUE_FORMAT_SAMPLE.1,
        ISSUE_FORMAT_SAMPLE.2,
    )
    .and_then(|date| {
        date.and_hms_opt(
            ISSUE_FORMAT_SAMPLE.3,
            ISSUE_FORMAT_SAMPLE.4,
            ISSUE_FORMAT_SAMPLE.5,
        )
    })
    .ok_or_else(|| ApiError::Internal("期号格式示例时间无效".to_string()))?;
    issue_sequence_width(pattern)?;
    let rendered = render_issue_format(sample, pattern, Some(1))?;
    if rendered.is_empty() {
        return Err(ApiError::BadRequest(
            "期号生成格式不能生成空期号".to_string(),
        ));
    }
    if rendered.len() > MAX_RENDERED_ISSUE_LENGTH {
        return Err(ApiError::BadRequest(
            "期号生成结果不能超过 64 个字符".to_string(),
        ));
    }
    if !rendered
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ApiError::BadRequest(
            "期号生成结果只能包含字母、数字、短横线或下划线".to_string(),
        ));
    }

    Ok(())
}

/// 判断模板是否包含至少一个受支持的日期或时间变量，避免常量期号造成重复冲突。
fn contains_supported_issue_token(pattern: &str) -> bool {
    [
        "{yyyy}",
        "{yy}",
        "{MM}",
        "{dd}",
        "{HH}",
        "{mm}",
        "{ss}",
        "{date}",
        "{time}",
        "{timestamp}",
    ]
    .iter()
    .any(|token| pattern.contains(token))
}

/// 按平台配置模板生成期号。
fn format_issue(
    value: NaiveDateTime,
    pattern: &str,
    sequence_state: &mut IssueSequenceState,
    fixed_sequence: Option<u32>,
) -> ApiResult<String> {
    let sequence_width = issue_sequence_width(pattern)?;
    let sequence = if let Some(width) = sequence_width {
        Some(if let Some(sequence) = fixed_sequence {
            let max_sequence = max_sequence_for_width(width);
            if sequence == 0 || sequence > max_sequence {
                return Err(ApiError::Conflict(format!(
                    "当天平台期号序号已超过 {max_sequence}"
                )));
            }
            sequence
        } else {
            sequence_state.next_sequence(value.date(), width)?
        })
    } else {
        None
    };

    render_issue_format(value, pattern, sequence)
}

/// 渲染期号模板变量，模板外的普通字母数字会原样保留。
fn render_issue_format(
    value: NaiveDateTime,
    pattern: &str,
    sequence: Option<u32>,
) -> ApiResult<String> {
    let mut output = String::new();
    let mut cursor = 0;

    while cursor < pattern.len() {
        let remaining = &pattern[cursor..];
        if remaining.starts_with('{') {
            let Some(close_offset) = remaining.find('}') else {
                return Err(ApiError::BadRequest(
                    "期号生成格式存在未闭合变量".to_string(),
                ));
            };
            let token = &remaining[..=close_offset];
            output.push_str(&issue_token_value(value, token, sequence)?);
            cursor += close_offset + 1;
            continue;
        }
        if remaining.starts_with('}') {
            return Err(ApiError::BadRequest(
                "期号生成格式存在多余右括号".to_string(),
            ));
        }

        let Some(ch) = remaining.chars().next() else {
            break;
        };
        output.push(ch);
        cursor += ch.len_utf8();
    }

    Ok(output)
}

/// 返回单个期号模板变量的实际值。
fn issue_token_value(
    value: NaiveDateTime,
    token: &str,
    sequence: Option<u32>,
) -> ApiResult<String> {
    let formatted = match token {
        "{yyyy}" => format!("{:04}", value.year()),
        "{yy}" => format!("{:02}", value.year().rem_euclid(100)),
        "{MM}" => format!("{:02}", value.month()),
        "{dd}" => format!("{:02}", value.day()),
        "{HH}" => format!("{:02}", value.hour()),
        "{mm}" => format!("{:02}", value.minute()),
        "{ss}" => format!("{:02}", value.second()),
        "{date}" => value.format("%Y%m%d").to_string(),
        "{time}" => value.format("%H%M%S").to_string(),
        "{timestamp}" => value.format("%Y%m%d%H%M%S").to_string(),
        "{seq1}" | "{seq2}" | "{seq3}" | "{seq4}" => {
            let width = sequence_token_width(token)
                .ok_or_else(|| ApiError::Internal("期号序号变量宽度解析失败".to_string()))?;
            let sequence = sequence
                .ok_or_else(|| ApiError::Internal("期号序号变量缺少递增状态".to_string()))?;
            if sequence > max_sequence_for_width(width) {
                return Err(ApiError::Conflict(format!(
                    "当天平台期号序号已超过 {}",
                    max_sequence_for_width(width)
                )));
            }
            format!("{sequence:0width$}")
        }
        _ => {
            return Err(ApiError::BadRequest(format!(
                "期号生成格式包含不支持的变量 {token}"
            )))
        }
    };

    Ok(formatted)
}

/// 返回模板使用的序号变量宽度，同一模板不能混用不同宽度的序号变量。
fn issue_sequence_width(pattern: &str) -> ApiResult<Option<usize>> {
    let mut found = None;
    for token in ["{seq1}", "{seq2}", "{seq3}", "{seq4}"] {
        if !pattern.contains(token) {
            continue;
        }
        let width = sequence_token_width(token)
            .ok_or_else(|| ApiError::Internal("期号序号变量宽度解析失败".to_string()))?;
        if let Some(existing) = found {
            if existing != width {
                return Err(ApiError::BadRequest(
                    "期号生成格式不能混用多个序号变量".to_string(),
                ));
            }
        }
        found = Some(width);
    }
    Ok(found)
}

/// 解析 `{seqN}` 变量的位数，供校验、渲染和恢复序号复用。
fn sequence_token_width(token: &str) -> Option<usize> {
    match token {
        "{seq1}" => Some(1),
        "{seq2}" => Some(2),
        "{seq3}" => Some(3),
        "{seq4}" => Some(4),
        _ => None,
    }
}

/// 返回指定序号位数在当天允许的最大序号。
fn max_sequence_for_width(width: usize) -> u32 {
    match width {
        1 => 9,
        2 => 99,
        3 => 999,
        4 => 9_999,
        _ => 0,
    }
}

/// 时间节点排期的每日序号配置，用自然时钟节点位置生成 `{seqN}`。
#[derive(Debug, Clone, Copy)]
struct TimeNodeSequenceConfig {
    interval_seconds: u32,
    start_time: NaiveTime,
}

impl TimeNodeSequenceConfig {
    /// 根据彩种排期和期号模板判断是否需要固定节点序号。
    fn from_schedule(schedule: &DrawSchedule, pattern: &str) -> ApiResult<Option<Self>> {
        if issue_sequence_width(pattern)?.is_none() {
            return Ok(None);
        }

        let DrawSchedule::TimeNode {
            interval_seconds,
            start_time,
        } = schedule
        else {
            return Ok(None);
        };
        if *interval_seconds == 0 {
            return Err(ApiError::BadRequest(
                "time node interval must be greater than zero".to_string(),
            ));
        }
        if 86_400 % *interval_seconds != 0 {
            return Err(ApiError::BadRequest(
                "time node interval must divide one day exactly".to_string(),
            ));
        }

        Ok(Some(Self {
            interval_seconds: *interval_seconds,
            start_time: parse_time(start_time, "time node start time")?,
        }))
    }

    /// 计算开盘节点在所属业务日内的序号，00:00 开盘为 1，23:55 开盘为 288。
    fn sequence_for(&self, label_time: NaiveDateTime) -> ApiResult<u32> {
        let start_at = combine_date_time(label_time.date(), self.start_time)?;
        let business_start = if label_time < start_at {
            start_at
                .checked_sub_signed(Duration::days(1))
                .ok_or_else(|| ApiError::BadRequest("time node date is out of range".to_string()))?
        } else {
            start_at
        };
        let elapsed_seconds = label_time
            .signed_duration_since(business_start)
            .num_seconds()
            .max(0);
        let step = elapsed_seconds
            .checked_div(i64::from(self.interval_seconds))
            .ok_or_else(|| ApiError::BadRequest("time node interval is invalid".to_string()))?;

        u32::try_from(
            step.checked_add(1)
                .ok_or_else(|| ApiError::Internal("时间节点期号序号超出范围".to_string()))?,
        )
        .map_err(|_| ApiError::Internal("时间节点期号序号超出范围".to_string()))
    }
}

/// 平台开奖每日序号状态，用于把 `{seqN}` 渲染成当天递增的固定位数数字。
#[derive(Debug, Clone, Default)]
struct IssueSequenceState {
    next_by_date: BTreeMap<NaiveDate, u32>,
}

impl IssueSequenceState {
    /// 根据已存在期号恢复当天下一个序号，避免调度重启后重复生成。
    fn from_existing_issues(
        pattern: &str,
        schedule: &DrawSchedule,
        issues: &[DrawIssue],
        lottery_id: &str,
    ) -> ApiResult<Self> {
        let mut state = Self::default();
        if issue_sequence_width(pattern)?.is_none() {
            return Ok(state);
        }

        for issue in issues.iter().filter(|issue| issue.lottery_id == lottery_id) {
            let scheduled_at = parse_timestamp(&issue.scheduled_at, "existing scheduled time")?;
            let label_time = issue_label_time(schedule, scheduled_at, scheduled_at)?;
            let Some(seq) = parse_pattern_sequence(pattern, label_time, &issue.issue) else {
                continue;
            };
            let next_seq = seq.saturating_add(1);
            let entry = state.next_by_date.entry(label_time.date()).or_insert(1);
            *entry = (*entry).max(next_seq);
        }

        Ok(state)
    }

    /// 获取指定日期的下一个固定位数序号并推进状态。
    fn next_sequence(&mut self, date: NaiveDate, width: usize) -> ApiResult<u32> {
        let next_seq = self.next_by_date.entry(date).or_insert(1);
        let max_sequence = max_sequence_for_width(width);
        if *next_seq > max_sequence {
            return Err(ApiError::Conflict(format!(
                "当天平台期号序号已超过 {max_sequence}"
            )));
        }

        let current = *next_seq;
        *next_seq = current
            .checked_add(1)
            .ok_or_else(|| ApiError::Internal("平台期号序号超出范围".to_string()))?;
        Ok(current)
    }
}

/// 按当前期号模板从已存在期号中提取每日序号。
fn parse_pattern_sequence(pattern: &str, value: NaiveDateTime, issue: &str) -> Option<u32> {
    let issue = issue.trim();
    let mut issue_cursor = 0;
    let mut pattern_cursor = 0;
    let mut parsed_sequence = None;

    while pattern_cursor < pattern.len() {
        let remaining = &pattern[pattern_cursor..];
        if remaining.starts_with('{') {
            let close_offset = remaining.find('}')?;
            let token = &remaining[..=close_offset];
            if let Some(width) = sequence_token_width(token) {
                let sequence_text = issue.get(issue_cursor..issue_cursor + width)?;
                if !sequence_text.bytes().all(|byte| byte.is_ascii_digit()) {
                    return None;
                }
                let sequence = sequence_text.parse::<u32>().ok()?;
                if sequence == 0 || sequence > max_sequence_for_width(width) {
                    return None;
                }
                if parsed_sequence.is_some_and(|existing| existing != sequence) {
                    return None;
                }
                parsed_sequence = Some(sequence);
                issue_cursor += width;
            } else {
                let token_value = issue_static_token_value(value, token)?;
                if !issue[issue_cursor..].starts_with(&token_value) {
                    return None;
                }
                issue_cursor += token_value.len();
            }
            pattern_cursor += close_offset + 1;
            continue;
        }

        let ch = remaining.chars().next()?;
        let issue_remaining = issue.get(issue_cursor..)?;
        if !issue_remaining.starts_with(ch) {
            return None;
        }
        pattern_cursor += ch.len_utf8();
        issue_cursor += ch.len_utf8();
    }

    if issue_cursor == issue.len() {
        parsed_sequence
    } else {
        None
    }
}

/// 返回无需序号状态即可渲染的期号变量值。
fn issue_static_token_value(value: NaiveDateTime, token: &str) -> Option<String> {
    match token {
        "{yyyy}" => Some(format!("{:04}", value.year())),
        "{yy}" => Some(format!("{:02}", value.year().rem_euclid(100))),
        "{MM}" => Some(format!("{:02}", value.month())),
        "{dd}" => Some(format!("{:02}", value.day())),
        "{HH}" => Some(format!("{:02}", value.hour())),
        "{mm}" => Some(format!("{:02}", value.minute())),
        "{ss}" => Some(format!("{:02}", value.second())),
        "{date}" => Some(value.format("%Y%m%d").to_string()),
        "{time}" => Some(value.format("%H%M%S").to_string()),
        "{timestamp}" => Some(value.format("%Y%m%d%H%M%S").to_string()),
        _ => None,
    }
}

/// 期号标签生成器，负责按不同排期推进下一期号。
enum IssueLabeler {
    Pattern {
        pattern: String,
        sequence_state: IssueSequenceState,
        time_node_sequence: Option<TimeNodeSequenceConfig>,
    },
    Sequential {
        next_issue: u64,
    },
}

/// 期号标签生成器，负责按不同排期推进下一期号。
impl IssueLabeler {
    /// 根据彩种模式和 API 锚点选择期号生成方式。
    fn for_lottery(
        lottery: &LotteryKind,
        existing_issues: &[DrawIssue],
        api_anchor: Option<&ApiIssueAnchor>,
    ) -> ApiResult<Self> {
        if let Some(api_anchor) = api_anchor {
            return Self::for_api_anchor(Some(api_anchor));
        }

        let pattern = if lottery.draw_mode == DrawMode::Platform {
            normalize_issue_format_pattern(&lottery.issue_format)?
        } else {
            DEFAULT_ISSUE_FORMAT_PATTERN.to_string()
        };

        let sequence_state = IssueSequenceState::from_existing_issues(
            &pattern,
            &lottery.schedule,
            existing_issues,
            &lottery.id,
        )?;
        let time_node_sequence =
            TimeNodeSequenceConfig::from_schedule(&lottery.schedule, &pattern)?;

        Ok(Self::Pattern {
            pattern,
            sequence_state,
            time_node_sequence,
        })
    }

    /// 按 API 锚点初始化期号序号生成器。
    fn for_api_anchor(api_anchor: Option<&ApiIssueAnchor>) -> ApiResult<Self> {
        let Some(api_anchor) = api_anchor else {
            return Ok(Self::Pattern {
                pattern: DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
                sequence_state: IssueSequenceState::default(),
                time_node_sequence: None,
            });
        };
        let next_issue = api_anchor
            .latest_issue
            .checked_add(1)
            .ok_or_else(|| ApiError::Internal("API 开奖源最新期号超出范围".to_string()))?;

        Ok(Self::Sequential { next_issue })
    }

    /// 按当前格式生成下一个期号。
    fn next_issue(&mut self, scheduled_at: NaiveDateTime) -> ApiResult<String> {
        match self {
            Self::Pattern {
                pattern,
                sequence_state,
                time_node_sequence,
            } => {
                let fixed_sequence = time_node_sequence
                    .as_ref()
                    .map(|config| config.sequence_for(scheduled_at))
                    .transpose()?;
                format_issue(scheduled_at, pattern, sequence_state, fixed_sequence)
            }
            Self::Sequential { next_issue } => {
                let issue = *next_issue;
                *next_issue = (*next_issue)
                    .checked_add(1)
                    .ok_or_else(|| ApiError::Internal("API 开奖源期号序列超出范围".to_string()))?;
                Ok(issue.to_string())
            }
        }
    }
}

/// 解析 API 返回的纯数字期号文本并提取序号，供补期和自动开奖过期判断复用。
pub(crate) fn parse_api_sequence_issue(value: &str) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    value.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            draw::{CreateDrawIssueRequest, GenerateDrawIssueRequest, GenerateDrawIssuesRequest},
            lottery::{
                DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType,
                PlayCategory,
            },
        },
        services::{
            draw::DrawRepository,
            draw_api::ApiDrawSourceRepository,
            draw_generation::{
                generate_draw_issue_batch, generate_next_draw_issue, preview_draw_issue_generation,
            },
        },
    };

    const API68_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "操作成功",
        "result": {
            "businessCode": 0,
            "message": "操作成功",
            "data": [
                { "preDrawIssue": 2026143, "preDrawCode": "3,7,6", "preDrawTime": "2026-06-02 21:15:00" },
                { "preDrawIssue": "2026142", "preDrawCode": "8,9,4", "preDrawTime": "2026-06-01 21:15:00" }
            ]
        }
    }"#;
    const API68_AU5_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "操作成功",
        "result": {
            "businessCode": 0,
            "message": "操作成功",
            "data": [
                { "preDrawIssue": 51320849, "preDrawCode": "4,5,4,3,0", "preDrawTime": "2026-06-03 11:18:40" }
            ]
        }
    }"#;
    const KJ_TXFFC_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "",
        "result": {
            "businessCode": "202606031178",
            "message": "",
            "data": {
                "lotKey": "txffc",
                "lotName": "腾讯分分彩",
                "preDrawIssue": "202606031178",
                "preDrawCode": "9,9,8,7,2",
                "preDrawTime": "2026-06-03 19:38:01",
                "drawIssue": 202606031179,
                "drawTime": "2026-06-03 19:39:00"
            }
        }
    }"#;
    /// 验证周期排期生成nextinterval之后当前时间。
    #[tokio::test]
    async fn periodic_schedule_generates_next_interval_after_now() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("issue can be generated");

        assert_eq!(issue.issue, "202606020001");
        assert_eq!(issue.scheduled_at, "2026-06-02 20:01:00");
        assert_eq!(issue.sale_closed_at, "2026-06-02 20:00:59");
    }
    /// 验证生成期号时默认使用彩种自身封盘提前秒数。
    #[tokio::test]
    async fn generation_uses_lottery_sale_close_lead_seconds_by_default() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });
        lottery.sale_close_lead_seconds = 15;

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("issue can be generated with lottery sale close lead seconds");

        assert_eq!(issue.scheduled_at, "2026-06-02 20:01:00");
        assert_eq!(issue.sale_closed_at, "2026-06-02 20:00:45");
    }
    /// 构造请求sale封盘leadsecondsoverrides彩种默认测试请求。
    #[tokio::test]
    async fn request_sale_close_lead_seconds_overrides_lottery_default() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });
        lottery.sale_close_lead_seconds = 15;
        let mut payload = request("2026-06-02 20:00:00");
        payload.sale_close_lead_seconds = Some(5);

        let issue = generate_next_draw_issue(&draws, &lottery, payload)
            .await
            .expect("request sale close lead seconds can override lottery config");

        assert_eq!(issue.sale_closed_at, "2026-06-02 20:00:55");
    }
    /// 验证 300 秒周期配置 60 秒封盘时会在剩余 60 秒封盘。
    #[tokio::test]
    async fn sale_close_lead_closes_300_second_lottery_with_60_seconds_remaining() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 300,
        });
        lottery.sale_close_lead_seconds = 60;

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("issue can be generated with 60 second sale close lead");

        assert_eq!(issue.scheduled_at, "2026-06-02 20:05:00");
        assert_eq!(issue.sale_closed_at, "2026-06-02 20:04:00");
    }
    /// 验证时间节点排期对齐toconfigured整点nodes。
    #[tokio::test]
    async fn time_node_schedule_aligns_to_configured_clock_nodes() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::TimeNode {
            interval_seconds: 300,
            start_time: "00:00:00".to_string(),
        });

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 00:00:00"))
            .await
            .expect("time node issue can be generated");

        assert_eq!(issue.issue, "202606020001");
        assert_eq!(issue.scheduled_at, "2026-06-02 00:05:00");
        assert_eq!(issue.sale_closed_at, "2026-06-02 00:04:59");
    }
    /// 验证时间节点零点开奖使用previous开盘节点用于期号文案。
    #[tokio::test]
    async fn time_node_midnight_draw_uses_previous_open_node_for_issue_label() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::TimeNode {
            interval_seconds: 300,
            start_time: "00:00:00".to_string(),
        });
        lottery.draw_mode = DrawMode::Platform;
        lottery.issue_format = "{date}{time}".to_string();

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-01 23:55:00"))
            .await
            .expect("midnight draw issue can be generated");

        assert_eq!(issue.issue, "20260601235500");
        assert_eq!(issue.scheduled_at, "2026-06-02 00:00:00");
        assert_eq!(issue.sale_closed_at, "2026-06-01 23:59:59");
    }
    /// 验证时间节点zero整点开盘优先当前日期期号。
    #[tokio::test]
    async fn time_node_zero_clock_opens_first_current_day_issue() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::TimeNode {
            interval_seconds: 300,
            start_time: "00:00:00".to_string(),
        });
        lottery.draw_mode = DrawMode::Platform;
        lottery.issue_format = "{date}{time}".to_string();

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 00:00:00"))
            .await
            .expect("zero clock open issue can be generated");

        assert_eq!(issue.issue, "20260602000000");
        assert_eq!(issue.scheduled_at, "2026-06-02 00:05:00");
        assert_eq!(issue.sale_closed_at, "2026-06-02 00:04:59");
    }
    /// 验证时间节点默认序号按自然节点计算，00:05 开奖是当天第一期。
    #[tokio::test]
    async fn time_node_sequence_first_draw_after_midnight_is_first_issue() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::TimeNode {
            interval_seconds: 300,
            start_time: "00:00:00".to_string(),
        });
        lottery.draw_mode = DrawMode::Platform;
        lottery.issue_format = "{date}{seq4}".to_string();

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-20 00:00:00"))
            .await
            .expect("first current day time node issue can be generated");

        assert_eq!(issue.issue, "202606200001");
        assert_eq!(issue.scheduled_at, "2026-06-20 00:05:00");
        assert_eq!(issue.sale_closed_at, "2026-06-20 00:04:59");
    }
    /// 验证时间节点跨到次日 00:00 开奖时，仍归属前一天最后一期。
    #[tokio::test]
    async fn time_node_sequence_midnight_draw_is_previous_day_last_issue() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::TimeNode {
            interval_seconds: 300,
            start_time: "00:00:00".to_string(),
        });
        lottery.draw_mode = DrawMode::Platform;
        lottery.issue_format = "{date}{seq4}".to_string();

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-19 23:55:00"))
            .await
            .expect("previous day last time node issue can be generated");

        assert_eq!(issue.issue, "202606190288");
        assert_eq!(issue.scheduled_at, "2026-06-20 00:00:00");
        assert_eq!(issue.sale_closed_at, "2026-06-19 23:59:59");
    }
    /// 验证历史期号不完整时，时间节点序号仍按时钟位置而不是已有数量递增。
    #[tokio::test]
    async fn time_node_sequence_ignores_incomplete_existing_issue_count() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::TimeNode {
            interval_seconds: 300,
            start_time: "00:00:00".to_string(),
        });
        lottery.draw_mode = DrawMode::Platform;
        lottery.issue_format = "{date}{seq4}".to_string();
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "202606190263".to_string(),
                    scheduled_at: "2026-06-19 23:55:00".to_string(),
                    sale_closed_at: "2026-06-19 23:54:59".to_string(),
                },
            )
            .await
            .expect("incomplete existing issue can be created");

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-19 23:55:00"))
            .await
            .expect("time node issue can ignore incomplete existing count");

        assert_eq!(issue.issue, "202606190288");
        assert_eq!(issue.scheduled_at, "2026-06-20 00:00:00");
        assert_eq!(issue.sale_closed_at, "2026-06-19 23:59:59");
    }
    /// 验证时间节点排期重新对齐后，序号仍按当天时钟节点位置计算。
    #[tokio::test]
    async fn time_node_schedule_realigns_after_offset_existing_issue() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::TimeNode {
            interval_seconds: 300,
            start_time: "00:00:00".to_string(),
        });
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260610201827".to_string(),
                    scheduled_at: "2026-06-10 20:18:27".to_string(),
                    sale_closed_at: "2026-06-10 20:18:17".to_string(),
                },
            )
            .await
            .expect("existing offset issue can be created");

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-10 20:18:52"))
            .await
            .expect("time node schedule can realign");

        assert_eq!(issue.issue, "202606100244");
        assert_eq!(issue.scheduled_at, "2026-06-10 20:20:00");
        assert_eq!(issue.sale_closed_at, "2026-06-10 20:19:59");
    }
    /// 验证平台开奖支持自定义期号格式。
    #[tokio::test]
    async fn platform_schedule_uses_custom_issue_format() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });
        lottery.draw_mode = DrawMode::Platform;
        lottery.issue_format = "{yy}{MM}{dd}{HH}{mm}".to_string();

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("issue can be generated with custom format");

        assert_eq!(issue.issue, "2606022001");
        assert_eq!(issue.scheduled_at, "2026-06-02 20:01:00");
    }
    /// 验证平台开奖期号格式支持短序号占位符。
    #[tokio::test]
    async fn platform_schedule_supports_short_sequence_tokens() {
        for (token, expected) in [
            ("{seq1}", "202606021"),
            ("{seq2}", "2026060201"),
            ("{seq3}", "20260602001"),
        ] {
            let draws = DrawRepository::memory();
            let mut lottery = lottery(DrawSchedule::Periodic {
                interval_seconds: 60,
            });
            lottery.draw_mode = DrawMode::Platform;
            lottery.issue_format = format!("{{date}}{token}");

            let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
                .await
                .expect("short sequence issue can be generated");

            assert_eq!(issue.issue, expected);
        }
    }
    /// 验证平台开奖能从已有期号恢复短序号。
    #[tokio::test]
    async fn platform_schedule_recovers_short_sequence_from_existing_issue() {
        let draws = DrawRepository::memory();
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });
        lottery.draw_mode = DrawMode::Platform;
        lottery.issue_format = "{date}{seq2}".to_string();
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "2026060209".to_string(),
                    scheduled_at: "2026-06-02 20:01:00".to_string(),
                    sale_closed_at: "2026-06-02 20:00:59".to_string(),
                },
            )
            .await
            .expect("existing seq2 issue can be created");

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:01:05"))
            .await
            .expect("next issue can continue seq2 sequence");

        assert_eq!(issue.issue, "2026060210");
        assert_eq!(issue.scheduled_at, "2026-06-02 20:02:00");
    }
    /// 验证生成下一期时以最新已有期号作为基线。
    #[tokio::test]
    async fn generation_uses_latest_existing_issue_as_baseline() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });

        let first = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("first issue can be generated");
        let second = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("second issue can be generated");

        assert_eq!(first.issue, "202606020001");
        assert_eq!(second.issue, "202606020002");
    }
    /// 验证每日排期过开奖时间后滚动到下一天。
    #[tokio::test]
    async fn daily_schedule_rolls_to_next_day_after_draw_time() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Daily {
            time: "21:00:15".to_string(),
        });

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 22:00:00"))
            .await
            .expect("issue can be generated");

        assert_eq!(issue.issue, "202606030001");
        assert_eq!(issue.sale_closed_at, "2026-06-03 21:00:14");
    }
    /// 验证api68每日排期使用最新external期号。
    #[tokio::test]
    async fn api68_daily_schedule_uses_latest_external_issue() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let lottery = lottery(DrawSchedule::Daily {
            time: "21:00:15".to_string(),
        });

        let plans = preview_draw_issue_generation(
            &draws,
            &lottery,
            batch_request("2026-06-02 22:00:00", 2),
        )
        .await
        .expect("plans can be previewed");

        assert_eq!(
            plans
                .iter()
                .map(|plan| plan.issue.as_str())
                .collect::<Vec<_>>(),
            vec!["2026144", "2026145"]
        );
        assert_eq!(plans[0].scheduled_at, "2026-06-03 21:00:15");
    }
    /// 验证api68每日排期continues之后已有真实期号。
    #[tokio::test]
    async fn api68_daily_schedule_continues_after_existing_real_issue() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let lottery = lottery(DrawSchedule::Daily {
            time: "21:00:15".to_string(),
        });
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "2026144".to_string(),
                    scheduled_at: "2026-06-03 21:00:15".to_string(),
                    sale_closed_at: "2026-06-03 20:59:45".to_string(),
                },
            )
            .await
            .expect("existing issue can be created");

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 22:00:00"))
            .await
            .expect("issue can be generated");

        assert_eq!(issue.issue, "2026145");
        assert_eq!(issue.scheduled_at, "2026-06-04 21:00:15");
    }
    /// 验证api68reused来源生成真实期号用于pl3。
    #[tokio::test]
    async fn api68_reused_source_generates_real_issue_for_pl3() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let mut lottery = lottery(DrawSchedule::Daily {
            time: "21:00:15".to_string(),
        });
        lottery.id = "pl3".to_string();
        lottery.name = "排列 3".to_string();

        let issue =
            generate_next_draw_issue(&draws, &lottery, request_for("pl3", "2026-06-02 22:00:00"))
                .await
                .expect("issue can be generated");

        assert_eq!(issue.issue, "2026144");
    }
    /// 验证api68au5来源生成eight数字期号。
    #[tokio::test]
    async fn api68_au5_source_generates_eight_digit_issue() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_AU5_SAMPLE),
        );
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 300,
        });
        lottery.id = "au5".to_string();
        lottery.name = "澳洲 5 分彩".to_string();
        lottery.number_type = LotteryNumberType::FiveDigit;

        let issue =
            generate_next_draw_issue(&draws, &lottery, request_for("au5", "2026-06-03 11:20:00"))
                .await
                .expect("issue can be generated");

        assert_eq!(issue.issue, "51320850");
        assert_eq!(issue.scheduled_at, "2026-06-03 11:23:40");
        assert_eq!(issue.sale_closed_at, "2026-06-03 11:23:39");
    }
    /// 验证api68周期排期重新对齐之后已有local期号。
    #[tokio::test]
    async fn api68_periodic_schedule_realigns_after_existing_local_issue() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_AU5_SAMPLE),
        );
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 300,
        });
        lottery.id = "au5".to_string();
        lottery.name = "澳洲 5 分彩".to_string();
        lottery.number_type = LotteryNumberType::FiveDigit;
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "51320850".to_string(),
                    scheduled_at: "2026-06-03 11:30:00".to_string(),
                    sale_closed_at: "2026-06-03 11:29:30".to_string(),
                },
            )
            .await
            .expect("existing issue can be created");

        let issue =
            generate_next_draw_issue(&draws, &lottery, request_for("au5", "2026-06-03 11:20:00"))
                .await
                .expect("issue can be generated");

        assert_eq!(issue.issue, "51320851");
        assert_eq!(issue.scheduled_at, "2026-06-03 11:28:40");
    }
    /// 验证api68周期排期keeps等待期号之后sale封盘时间。
    #[tokio::test]
    async fn api68_periodic_schedule_keeps_waiting_issue_after_sale_close_time() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_AU5_SAMPLE),
        );
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 300,
        });
        lottery.id = "au5".to_string();
        lottery.name = "澳洲 5 分彩".to_string();
        lottery.number_type = LotteryNumberType::FiveDigit;

        let issue =
            generate_next_draw_issue(&draws, &lottery, request_for("au5", "2026-06-03 11:23:39"))
                .await
                .expect("issue can be generated");

        assert_eq!(issue.issue, "51320850");
        assert_eq!(issue.scheduled_at, "2026-06-03 11:23:40");
    }
    /// 验证api68周期排期skips期号之后开奖时间。
    #[tokio::test]
    async fn api68_periodic_schedule_skips_issue_after_draw_time() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_AU5_SAMPLE),
        );
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 300,
        });
        lottery.id = "au5".to_string();
        lottery.name = "澳洲 5 分彩".to_string();
        lottery.number_type = LotteryNumberType::FiveDigit;

        let issue =
            generate_next_draw_issue(&draws, &lottery, request_for("au5", "2026-06-03 11:23:40"))
                .await
                .expect("issue can be generated");

        assert_eq!(issue.issue, "51320851");
        assert_eq!(issue.scheduled_at, "2026-06-03 11:28:40");
    }
    /// 验证kjtxffc来源生成providernext期号。
    #[tokio::test]
    async fn kj_txffc_source_generates_provider_next_issue() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::kj_seeded_with_static_response(KJ_TXFFC_SAMPLE),
        );
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });
        lottery.id = "txffc".to_string();
        lottery.name = "腾讯分分彩".to_string();
        lottery.number_type = LotteryNumberType::FiveDigit;

        let issue = generate_next_draw_issue(
            &draws,
            &lottery,
            request_for("txffc", "2026-06-03 19:38:20"),
        )
        .await
        .expect("issue can be generated");

        assert_eq!(issue.issue, "202606031179");
        assert_eq!(issue.scheduled_at, "2026-06-03 19:39:00");
    }
    /// 验证kjtxffc来源keeps等待providernext期号。
    #[tokio::test]
    async fn kj_txffc_source_keeps_waiting_provider_next_issue() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::kj_seeded_with_static_response(KJ_TXFFC_SAMPLE),
        );
        let mut lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });
        lottery.id = "txffc".to_string();
        lottery.name = "腾讯分分彩".to_string();
        lottery.number_type = LotteryNumberType::FiveDigit;

        let issue = generate_next_draw_issue(
            &draws,
            &lottery,
            request_for("txffc", "2026-06-03 19:38:59"),
        )
        .await
        .expect("issue can be generated");

        assert_eq!(issue.issue, "202606031179");
        assert_eq!(issue.scheduled_at, "2026-06-03 19:39:00");
    }
    /// 验证每周排期会选择下一个匹配星期。
    #[tokio::test]
    async fn weekly_schedule_picks_next_matching_weekday() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Weekly {
            weekdays: vec!["Tuesday".to_string(), "Thursday".to_string()],
            time: "21:00:00".to_string(),
        });

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 22:00:00"))
            .await
            .expect("issue can be generated");

        assert_eq!(issue.issue, "202606040001");
        assert_eq!(issue.scheduled_at, "2026-06-04 21:00:00");
    }
    /// 验证预览生成不会写入真实开奖期号。
    #[tokio::test]
    async fn preview_generation_does_not_create_draw_issues() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });

        let plans = preview_draw_issue_generation(
            &draws,
            &lottery,
            batch_request("2026-06-02 20:00:00", 3),
        )
        .await
        .expect("plans can be previewed");

        assert_eq!(
            plans
                .iter()
                .map(|plan| plan.issue.as_str())
                .collect::<Vec<_>>(),
            vec!["202606020001", "202606020002", "202606020003"]
        );
        assert!(draws
            .list()
            .await
            .expect("draw issues can be listed")
            .is_empty());
    }
    /// 验证批量生成可以创建多个周期期号。
    #[tokio::test]
    async fn batch_generation_creates_multiple_periodic_draw_issues() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });

        let issues =
            generate_draw_issue_batch(&draws, &lottery, batch_request("2026-06-02 20:00:00", 3))
                .await
                .expect("issues can be generated");

        assert_eq!(
            issues
                .iter()
                .map(|issue| issue.issue.as_str())
                .collect::<Vec<_>>(),
            vec!["202606020001", "202606020002", "202606020003"]
        );
        assert_eq!(
            draws.list().await.expect("draw issues can be listed").len(),
            3
        );
    }
    /// 验证周期生成keepscadencewhen调度器runslate。
    #[tokio::test]
    async fn periodic_generation_keeps_cadence_when_scheduler_runs_late() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260610201827".to_string(),
                    scheduled_at: "2026-06-10 20:18:27".to_string(),
                    sale_closed_at: "2026-06-10 20:18:17".to_string(),
                },
            )
            .await
            .expect("existing issue can be created");

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-10 20:18:52"))
            .await
            .expect("late scheduler can generate next cadence issue");

        assert_eq!(issue.issue, "202606100001");
        assert_eq!(issue.scheduled_at, "2026-06-10 20:19:27");
        assert_eq!(issue.sale_closed_at, "2026-06-10 20:19:26");
    }
    /// 验证批量生成以最新已有期号作为基线。
    #[tokio::test]
    async fn batch_generation_uses_latest_existing_issue_as_baseline() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });

        generate_draw_issue_batch(&draws, &lottery, batch_request("2026-06-02 20:00:00", 2))
            .await
            .expect("first batch can be generated");
        let issues =
            generate_draw_issue_batch(&draws, &lottery, batch_request("2026-06-02 20:00:00", 2))
                .await
                .expect("second batch can be generated");

        assert_eq!(
            issues
                .iter()
                .map(|issue| issue.issue.as_str())
                .collect::<Vec<_>>(),
            vec!["202606020003", "202606020004"]
        );
    }
    /// 验证每日批量生成可以跨天滚动。
    #[tokio::test]
    async fn daily_batch_generation_rolls_across_days() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Daily {
            time: "21:00:15".to_string(),
        });

        let plans = preview_draw_issue_generation(
            &draws,
            &lottery,
            batch_request("2026-06-02 22:00:00", 2),
        )
        .await
        .expect("plans can be previewed");

        assert_eq!(
            plans
                .iter()
                .map(|plan| plan.issue.as_str())
                .collect::<Vec<_>>(),
            vec!["202606030001", "202606040001"]
        );
    }
    /// 验证每周批量生成只选择配置的星期。
    #[tokio::test]
    async fn weekly_batch_generation_picks_configured_weekdays() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Weekly {
            weekdays: vec!["Tuesday".to_string(), "Thursday".to_string()],
            time: "21:00:00".to_string(),
        });

        let plans = preview_draw_issue_generation(
            &draws,
            &lottery,
            batch_request("2026-06-02 22:00:00", 3),
        )
        .await
        .expect("plans can be previewed");

        assert_eq!(
            plans
                .iter()
                .map(|plan| plan.issue.as_str())
                .collect::<Vec<_>>(),
            vec!["202606040001", "202606090001", "202606110001"]
        );
    }
    /// 验证批量生成数量超出范围时会被拒绝。
    #[tokio::test]
    async fn batch_generation_rejects_count_out_of_range() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });

        let error = preview_draw_issue_generation(
            &draws,
            &lottery,
            batch_request("2026-06-02 20:00:00", 0),
        )
        .await
        .expect_err("zero count is rejected");

        assert!(error
            .to_string()
            .contains("draw issue generation count must be between 1 and 50"));
    }

    /// 构造测试用期号生成请求。
    fn request(now: &str) -> GenerateDrawIssueRequest {
        request_for("fc3d", now)
    }

    /// 按彩种构造测试用期号生成请求。
    fn request_for(lottery_id: &str, now: &str) -> GenerateDrawIssueRequest {
        GenerateDrawIssueRequest {
            lottery_id: lottery_id.to_string(),
            now: now.to_string(),
            sale_close_lead_seconds: None,
        }
    }

    /// 构造测试用批量生成期号请求。
    fn batch_request(now: &str, count: u32) -> GenerateDrawIssuesRequest {
        GenerateDrawIssuesRequest {
            lottery_id: "fc3d".to_string(),
            now: now.to_string(),
            count,
            sale_close_lead_seconds: None,
        }
    }

    /// 构造测试或种子使用的彩种配置。
    fn lottery(schedule: DrawSchedule) -> LotteryKind {
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            category: "regional".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            api_draw_delay_seconds: 0,
            draw_control_enabled: true,
            issue_format: crate::domain::lottery::DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            sale_close_lead_seconds: crate::domain::lottery::DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            schedule,
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: true,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1_000,
            },
            play_categories: vec![PlayCategory::Direct],
            play_configs: Vec::new(),
        }
    }
}
