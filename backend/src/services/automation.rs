//! 开奖自动化服务，统一编排手动/自动开奖执行链路

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use chrono::{Duration, NaiveDateTime};

use crate::{
    domain::{
        draw::{
            DrawAutomationRun, DrawAutomationRunRequest, DrawAutomationSkippedIssue, DrawIssue,
            DrawIssueResultRequest, DrawIssueStatus,
        },
        finance::LedgerEntry,
        lottery::{DrawMode, LotteryKind},
    },
    error::{ApiError, ApiResult},
    services::{
        draw::DrawRepository,
        draw_avoidance::{
            draw_prefetched_api_with_avoid_winning_policy, draw_with_avoid_winning_policy,
        },
        draw_generation::parse_api_sequence_issue,
        draw_settlement_queue,
        finance::FinanceRepository,
        group_buy::GroupBuyRepository,
        order::OrderRepository,
        redis_runtime::RedisRuntime,
    },
};
use tokio::sync::Semaphore;

const API_DRAW_RETRY_MAX_LATEST_ISSUE_DISTANCE: u64 = 5;
const DRAW_ISSUE_CONCURRENCY_LIMIT: usize = 8;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Clone)]
enum ApiLatestIssueLookup {
    Found(String),
    Missing,
}

enum ApiDrawNumberLookup {
    Ready(Option<String>),
    Failed(String),
}

#[derive(Clone)]
struct LotteryAutomationConfig {
    lottery: LotteryKind,
    api_draw_delay_seconds: u32,
    draw_control_enabled: bool,
}

#[derive(Clone)]
/// 单个期号开奖、结算和派奖完成后的进度通知，供调度器实时推送开奖结果。
pub struct DrawIssueSettlementProgress {
    /// 已写入开奖结果的开奖期。
    pub drawn_issue: DrawIssue,
    /// 本期结算产生的资金流水。
    pub ledger_entries: Vec<LedgerEntry>,
}

/// 自动开奖慢阶段的进度回调；必须保持轻量，耗时操作应由调用方自行异步派发。
pub type DrawIssueSettlementProgressCallback =
    Arc<dyn Fn(DrawIssueSettlementProgress) + Send + Sync>;

struct DrawExecutionJob {
    order: usize,
    issue: DrawIssue,
    lottery: LotteryKind,
    api_draw_number: Option<String>,
    allow_control_number: bool,
}

struct DrawExecutionOutcome {
    order: usize,
    source_issue: DrawIssue,
    result: ApiResult<DrawIssue>,
}

/// 触发自动化开奖一轮任务并返回执行结果。
pub async fn run_draw_automation(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
) -> ApiResult<DrawAutomationRun> {
    let close_run = close_due_draw_issues(draws, lotteries, payload.clone()).await?;
    let refund_run =
        refund_closed_unfilled_group_buys(draws, lotteries, finance, group_buys, payload.clone())
            .await?;
    let draw_run = draw_due_issues(draws, lotteries, orders, finance, group_buys, payload).await?;

    Ok(merge_draw_automation_runs(
        merge_draw_automation_runs(close_run, refund_run),
        draw_run,
    ))
}

/// 只处理到期封盘，供调度器优先释放下一期开盘。
pub async fn close_due_draw_issues(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    payload: DrawAutomationRunRequest,
) -> ApiResult<DrawAutomationRun> {
    let now = normalize_automation_now(&payload)?;

    let mut run = empty_draw_automation_run(&now);

    let lottery_configs = lottery_automation_configs(lotteries).await?;
    for issue in draws.list_scheduler_active().await? {
        if let Some(reason) = skip_issue_if_lottery_config_missing(&issue, &lottery_configs) {
            run.skipped_issues.push(skipped_issue(&issue, &reason));
            continue;
        }

        if should_close(&issue, &now) {
            let closed = draws.close(&issue.id).await?;
            run.closed_issues.push(closed);
        }
    }

    Ok(run)
}

/// 后台慢阶段处理已经封盘的未满员合买流单退款，避免资金写入阻塞下一期开盘。
pub async fn refund_closed_unfilled_group_buys(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
) -> ApiResult<DrawAutomationRun> {
    refund_closed_unfilled_group_buys_with_protected_plans(
        draws,
        lotteries,
        finance,
        group_buys,
        payload,
        &HashSet::new(),
    )
    .await
}

/// 后台慢阶段处理封盘未满员合买退款，但跳过本轮机器人兜底失败的保护计划。
pub async fn refund_closed_unfilled_group_buys_with_protected_plans(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
    protected_plan_ids: &HashSet<String>,
) -> ApiResult<DrawAutomationRun> {
    let now = normalize_automation_now(&payload)?;
    let mut run = empty_draw_automation_run(&now);
    let lottery_configs = lottery_automation_configs(lotteries).await?;

    for issue in draws.list_refundable_draw_issues().await? {
        if skip_issue_if_lottery_config_missing(&issue, &lottery_configs).is_some() {
            continue;
        }

        if should_refund_unfilled_group_buy(&issue, &now) {
            let cancelled_plans = if protected_plan_ids.is_empty() {
                group_buys
                    .cancel_unfilled_for_issue(&issue.lottery_id, &issue.issue)
                    .await?
            } else {
                group_buys
                    .cancel_unfilled_for_issue_except(
                        &issue.lottery_id,
                        &issue.issue,
                        protected_plan_ids,
                    )
                    .await?
            };
            for plan in cancelled_plans {
                let entries = finance
                    .refund_group_buy_plan(&plan, "封盘未满员流单退款")
                    .await?;
                run.ledger_entries.extend(entries);
            }
        }
    }

    Ok(run)
}

/// 处理到期开奖、订单结算、派奖入账和合买结算，允许在开盘推送之后继续执行。
pub async fn draw_due_issues(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
) -> ApiResult<DrawAutomationRun> {
    draw_due_issues_with_progress(draws, lotteries, orders, finance, group_buys, payload, None)
        .await
}

/// 处理到期开奖并在每个期号完成结算后回调进度，避免整批开奖完成前客户端一直等待。
pub async fn draw_due_issues_with_progress(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
    progress_callback: Option<DrawIssueSettlementProgressCallback>,
) -> ApiResult<DrawAutomationRun> {
    draw_due_issues_with_progress_excluding_group_buy_issues(
        draws,
        lotteries,
        orders,
        finance,
        group_buys,
        payload,
        progress_callback,
        &HashSet::new(),
    )
    .await
}

/// 处理到期开奖并跳过本轮合买兜底失败的期号，避免未满单计划被同轮开奖固化。
pub async fn draw_due_issues_with_progress_excluding_group_buy_issues(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
    progress_callback: Option<DrawIssueSettlementProgressCallback>,
    protected_issue_keys: &HashSet<String>,
) -> ApiResult<DrawAutomationRun> {
    draw_due_issues_with_progress_excluding_group_buy_issues_internal(
        draws,
        lotteries,
        orders,
        finance,
        group_buys,
        payload,
        progress_callback,
        protected_issue_keys,
        None,
    )
    .await
}

/// 处理到期开奖，把结算派奖投递到 Redis 队列，避免调度器等待慢派奖链路。
pub async fn draw_due_issues_with_progress_excluding_group_buy_issues_and_settlement_queue(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
    progress_callback: Option<DrawIssueSettlementProgressCallback>,
    protected_issue_keys: &HashSet<String>,
    settlement_queue: &RedisRuntime,
) -> ApiResult<DrawAutomationRun> {
    draw_due_issues_with_progress_excluding_group_buy_issues_internal(
        draws,
        lotteries,
        orders,
        finance,
        group_buys,
        payload,
        progress_callback,
        protected_issue_keys,
        Some(settlement_queue),
    )
    .await
}

async fn draw_due_issues_with_progress_excluding_group_buy_issues_internal(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
    progress_callback: Option<DrawIssueSettlementProgressCallback>,
    protected_issue_keys: &HashSet<String>,
    settlement_queue: Option<&RedisRuntime>,
) -> ApiResult<DrawAutomationRun> {
    let now = normalize_automation_now(&payload)?;
    let mut run = empty_draw_automation_run(&now);
    let lottery_configs = lottery_automation_configs(lotteries).await?;
    retry_unsettled_drawn_issue_settlements(
        &mut run,
        draws,
        orders,
        finance,
        group_buys,
        &progress_callback,
        settlement_queue,
    )
    .await?;
    let mut local_draw_candidates = Vec::new();
    let mut api_draw_candidates = Vec::new();
    for issue in draws.list_scheduler_active().await? {
        if let Some(reason) = skip_issue_if_lottery_config_missing(&issue, &lottery_configs) {
            push_skipped_issue_once(&mut run, &issue, &reason);
            continue;
        }

        if !should_draw(&issue, &now, &lottery_configs) {
            continue;
        }
        if protected_issue_keys.contains(&group_buy_issue_key(&issue.lottery_id, &issue.issue)) {
            run.skipped_issues.push(skipped_issue(
                &issue,
                "本期存在合买兜底失败计划，暂缓开奖等待下一轮补满",
            ));
            continue;
        }
        if issue.draw_mode == DrawMode::Manual
            && !lottery_draw_control_enabled(&issue, &lottery_configs)
        {
            run.skipped_issues
                .push(skipped_issue(&issue, "彩种未开启开奖号码控制"));
            continue;
        }
        if issue.draw_mode == DrawMode::Manual && !draws.has_active_draw_control(&issue).await? {
            run.skipped_issues
                .push(skipped_issue(&issue, "手动开奖需要管理员录入开奖号码"));
            continue;
        }

        if issue.draw_mode == DrawMode::Api {
            api_draw_candidates.push(issue);
        } else {
            local_draw_candidates.push(issue);
        }
    }

    let local_jobs =
        draw_execution_jobs_from_candidates(&mut run, &lottery_configs, local_draw_candidates);
    let local_outcomes = execute_draw_jobs_concurrently(draws, orders, local_jobs).await?;
    settle_draw_outcomes(
        &mut run,
        orders,
        finance,
        group_buys,
        local_outcomes,
        &progress_callback,
        settlement_queue,
    )
    .await?;

    let api_latest_issue_cache =
        prefetch_api_latest_issue_lookups(draws, &api_draw_candidates).await;
    let mut executable_api_issues = Vec::new();
    let mut api_draw_prefetch_issues = Vec::new();
    for issue in api_draw_candidates {
        if let Some(reason) = stale_api_issue_retry_reason(&api_latest_issue_cache, &issue) {
            if cancel_api_issue_without_pending_business(
                &mut run,
                draws,
                orders,
                group_buys,
                &issue,
                &reason,
                "API旧期号无待处理业务，已自动取消",
            )
            .await?
            {
                continue;
            }
            run.skipped_issues.push(skipped_issue(&issue, &reason));
            continue;
        }
        let allow_control = lottery_draw_control_enabled(&issue, &lottery_configs);
        if !allow_control || !draws.has_active_draw_control(&issue).await? {
            api_draw_prefetch_issues.push(issue.clone());
        }

        executable_api_issues.push(issue);
    }
    let api_draw_number_cache = prefetch_api_draw_numbers(draws, api_draw_prefetch_issues).await;

    let mut api_draw_jobs = Vec::new();
    for (order, issue) in executable_api_issues.into_iter().enumerate() {
        let api_draw_number = match api_draw_number_cache.get(&issue.id) {
            Some(ApiDrawNumberLookup::Ready(draw_number)) => draw_number.clone(),
            Some(ApiDrawNumberLookup::Failed(reason)) => {
                if api_draw_number_failure_is_missing_old_issue(&issue, reason)
                    && cancel_api_issue_without_pending_business(
                        &mut run,
                        draws,
                        orders,
                        group_buys,
                        &issue,
                        reason,
                        "API开奖源找不到旧期号且无待处理业务，已自动取消",
                    )
                    .await?
                {
                    continue;
                }
                run.skipped_issues.push(skipped_issue(&issue, reason));
                continue;
            }
            None => None,
        };

        let Some(lottery_config) = lottery_configs.get(&issue.lottery_id) else {
            run.skipped_issues
                .push(skipped_issue(&issue, "未找到彩种配置，跳过自动开奖"));
            continue;
        };

        api_draw_jobs.push(DrawExecutionJob {
            order,
            allow_control_number: lottery_draw_control_enabled(&issue, &lottery_configs),
            lottery: lottery_config.lottery.clone(),
            issue,
            api_draw_number,
        });
    }

    let api_outcomes = execute_draw_jobs_concurrently(draws, orders, api_draw_jobs).await?;
    settle_draw_outcomes(
        &mut run,
        orders,
        finance,
        group_buys,
        api_outcomes,
        &progress_callback,
        settlement_queue,
    )
    .await?;

    Ok(run)
}

/// 没有业务单据的 API 旧期号自动取消，避免无订单历史期每秒拖慢调度。
async fn cancel_api_issue_without_pending_business(
    run: &mut DrawAutomationRun,
    draws: &DrawRepository,
    orders: &OrderRepository,
    group_buys: &GroupBuyRepository,
    issue: &DrawIssue,
    source_reason: &str,
    log_message: &'static str,
) -> ApiResult<bool> {
    if issue_has_pending_business(orders, group_buys, issue).await? {
        return Ok(false);
    }

    let cancelled = draws.cancel(&issue.id).await?;
    let reason = format!("{source_reason}；本期没有待开奖订单或活跃合买，已自动取消");
    tracing::warn!(
        draw_issue_id = %cancelled.id,
        lottery_id = %cancelled.lottery_id,
        issue = %cancelled.issue,
        "{log_message}"
    );
    run.skipped_issues.push(skipped_issue(&cancelled, &reason));

    Ok(true)
}

/// 判断 API 开奖失败是否属于开奖源已经只返回更新期号的确定性旧期缺失。
fn api_draw_number_failure_is_missing_old_issue(issue: &DrawIssue, reason: &str) -> bool {
    if !reason.contains("API 开奖源未找到期号") {
        return false;
    }
    let Some(returned_issue) = returned_issue_from_missing_reason(reason) else {
        return false;
    };
    api_returned_issue_is_beyond_retry_window(&issue.issue, returned_issue)
}

/// 从开奖源缺失错误文案中提取当前返回期号，辅助判断缺失期是否超出重试窗口。
fn returned_issue_from_missing_reason(reason: &str) -> Option<&str> {
    reason
        .split("当前返回期号 `")
        .nth(1)?
        .split('`')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// 判断开奖源当前返回期号是否已经明显晚于本地期号且超过重试窗口。
fn api_returned_issue_is_beyond_retry_window(expected_issue: &str, returned_issue: &str) -> bool {
    let expected_issue = expected_issue.trim();
    let returned_issue = returned_issue.trim();
    let numeric_expected = parse_api_sequence_issue(expected_issue);
    let numeric_returned = parse_api_sequence_issue(returned_issue);
    if let (Some(expected_sequence), Some(returned_sequence)) = (numeric_expected, numeric_returned)
    {
        if returned_sequence > expected_sequence {
            return returned_sequence - expected_sequence
                > API_DRAW_RETRY_MAX_LATEST_ISSUE_DISTANCE;
        }
    }

    expected_issue.bytes().all(|byte| byte.is_ascii_digit())
        && returned_issue.bytes().all(|byte| byte.is_ascii_digit())
        && expected_issue.len() != returned_issue.len()
        && returned_issue > expected_issue
}

/// 判断期号是否仍有关联待开奖订单或活跃合买，避免自动取消影响资金审计。
async fn issue_has_pending_business(
    orders: &OrderRepository,
    group_buys: &GroupBuyRepository,
    issue: &DrawIssue,
) -> ApiResult<bool> {
    if !orders
        .list_pending_for_issue(&issue.lottery_id, &issue.issue)
        .await?
        .is_empty()
    {
        return Ok(true);
    }

    Ok(!group_buys
        .list_control_details_for_issue(&issue.lottery_id, &issue.issue)
        .await?
        .is_empty())
}

/// 把不依赖 API 预取的本地开奖候选转换为开奖执行任务，确保平台开奖可以优先落库。
fn draw_execution_jobs_from_candidates(
    run: &mut DrawAutomationRun,
    lottery_configs: &HashMap<String, LotteryAutomationConfig>,
    candidates: Vec<DrawIssue>,
) -> Vec<DrawExecutionJob> {
    let mut draw_jobs = Vec::new();
    for (order, issue) in candidates.into_iter().enumerate() {
        let Some(lottery_config) = lottery_configs.get(&issue.lottery_id) else {
            run.skipped_issues
                .push(skipped_issue(&issue, "未找到彩种配置，跳过自动开奖"));
            continue;
        };

        draw_jobs.push(DrawExecutionJob {
            order,
            allow_control_number: lottery_draw_control_enabled(&issue, lottery_configs),
            lottery: lottery_config.lottery.clone(),
            issue,
            api_draw_number: None,
        });
    }

    draw_jobs
}

/// 按执行结果逐期结算并推送进度；平台开奖和 API 开奖共用同一套派奖链路。
async fn settle_draw_outcomes(
    run: &mut DrawAutomationRun,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    draw_outcomes: Vec<DrawExecutionOutcome>,
    progress_callback: &Option<DrawIssueSettlementProgressCallback>,
    settlement_queue: Option<&RedisRuntime>,
) -> ApiResult<()> {
    for outcome in draw_outcomes {
        let issue = outcome.source_issue;
        let draw_result = outcome.result;
        let drawn = match draw_result {
            Ok(drawn) => drawn,
            Err(error) => {
                let reason = automation_error_reason(&error);
                tracing::warn!(
                    draw_issue_id = %issue.id,
                    lottery_id = %issue.lottery_id,
                    issue = %issue.issue,
                    error = %error.log_message(),
                    "自动开奖因开奖失败跳过期号"
                );
                run.skipped_issues.push(skipped_issue(&issue, &reason));
                continue;
            }
        };
        if enqueue_drawn_issue_for_async_settlement(&drawn, settlement_queue).await? {
            if let Some(callback) = progress_callback {
                callback(DrawIssueSettlementProgress {
                    drawn_issue: drawn.clone(),
                    ledger_entries: Vec::new(),
                });
            }
            run.drawn_issues.push(drawn);
            continue;
        }
        settle_drawn_issue_with_retry_marker(
            run,
            orders,
            finance,
            group_buys,
            drawn,
            progress_callback,
            true,
        )
        .await?;
    }

    Ok(())
}

/// 重试已经写入开奖号码但尚未完成计奖派奖的期号，避免历史 drawn 期号脱离调度扫描。
async fn retry_unsettled_drawn_issue_settlements(
    run: &mut DrawAutomationRun,
    draws: &DrawRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    progress_callback: &Option<DrawIssueSettlementProgressCallback>,
    settlement_queue: Option<&RedisRuntime>,
) -> ApiResult<()> {
    let settled_draw_issue_ids = orders.settled_draw_issue_ids().await?;
    let unsettled_drawn_issues = draws
        .list_unsettled_drawn_issues(&settled_draw_issue_ids)
        .await?;

    for issue in unsettled_drawn_issues {
        if enqueue_drawn_issue_for_async_settlement(&issue, settlement_queue).await? {
            continue;
        }
        settle_drawn_issue_with_retry_marker(
            run,
            orders,
            finance,
            group_buys,
            issue,
            progress_callback,
            false,
        )
        .await?;
    }

    Ok(())
}

async fn enqueue_drawn_issue_for_async_settlement(
    drawn: &DrawIssue,
    settlement_queue: Option<&RedisRuntime>,
) -> ApiResult<bool> {
    let Some(settlement_queue) = settlement_queue else {
        return Ok(false);
    };
    if !settlement_queue.is_enabled() {
        return Ok(false);
    }
    match draw_settlement_queue::enqueue(settlement_queue, drawn).await {
        Ok(enqueued) => {
            if enqueued {
                tracing::info!(
                    draw_issue_id = %drawn.id,
                    lottery_id = %drawn.lottery_id,
                    issue = %drawn.issue,
                    "已开奖期号已加入异步结算队列"
                );
            }
            Ok(enqueued)
        }
        Err(error) => {
            tracing::warn!(
                draw_issue_id = %drawn.id,
                lottery_id = %drawn.lottery_id,
                issue = %drawn.issue,
                error = %error.log_message(),
                "已开奖期号加入异步结算队列失败，回退同步结算"
            );
            Ok(false)
        }
    }
}

/// 串行提交单期计奖派奖；失败时记录可重试原因，不阻断同轮其它期号开奖。
async fn settle_drawn_issue_with_retry_marker(
    run: &mut DrawAutomationRun,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    drawn: DrawIssue,
    progress_callback: &Option<DrawIssueSettlementProgressCallback>,
    record_drawn_issue: bool,
) -> ApiResult<()> {
    let settlement_result = orders
        .settle_with_payouts(finance, group_buys, &drawn)
        .await;

    let (settlement, entries) = match settlement_result {
        Ok(result) => result,
        Err(error) if matches!(&error, ApiError::Conflict(_)) => {
            let reason = automation_error_reason(&error);
            tracing::warn!(
                draw_issue_id = %drawn.id,
                lottery_id = %drawn.lottery_id,
                issue = %drawn.issue,
                error = %error.log_message(),
                "自动开奖计奖派奖遇到已结算期号，跳过重复处理"
            );
            push_skipped_issue_once(run, &drawn, &reason);
            return Ok(());
        }
        Err(error) => {
            let reason = automation_error_reason(&error);
            tracing::warn!(
                draw_issue_id = %drawn.id,
                lottery_id = %drawn.lottery_id,
                issue = %drawn.issue,
                error = %error.log_message(),
                "自动开奖计奖派奖失败，保留期号等待后续调度重试"
            );
            push_skipped_issue_once(run, &drawn, &reason);
            return Ok(());
        }
    };

    if let Some(callback) = progress_callback {
        callback(DrawIssueSettlementProgress {
            drawn_issue: drawn.clone(),
            ledger_entries: entries.clone(),
        });
    }

    if record_drawn_issue {
        run.drawn_issues.push(drawn);
    }
    run.settlement_runs.push(settlement);
    run.ledger_entries.extend(entries);

    Ok(())
}

/// 并发写入本轮已满足开奖条件的期号结果；结算阶段仍由调用方按顺序提交，避免资金快照竞争。
async fn execute_draw_jobs_concurrently(
    draws: &DrawRepository,
    orders: &OrderRepository,
    jobs: Vec<DrawExecutionJob>,
) -> ApiResult<Vec<DrawExecutionOutcome>> {
    let semaphore = Arc::new(Semaphore::new(DRAW_ISSUE_CONCURRENCY_LIMIT));
    let mut handles = Vec::new();

    for job in jobs {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| ApiError::Internal("自动开奖并发执行许可获取失败".to_string()))?;
        let draws = draws.clone();
        let orders = orders.clone();
        let fallback_issue = job.issue.clone();
        handles.push((
            fallback_issue,
            tokio::spawn(async move {
                let _permit = permit;
                let issue = job.issue;
                let result = if issue.draw_mode == DrawMode::Api {
                    draw_prefetched_api_with_avoid_winning_policy(
                        &draws,
                        &orders,
                        &job.lottery,
                        &issue.id,
                        job.api_draw_number,
                        job.allow_control_number,
                    )
                    .await
                } else {
                    draw_with_avoid_winning_policy(
                        &draws,
                        &orders,
                        &job.lottery,
                        &issue.id,
                        DrawIssueResultRequest::default(),
                    )
                    .await
                };
                DrawExecutionOutcome {
                    order: job.order,
                    source_issue: issue,
                    result,
                }
            }),
        ));
    }

    let mut outcomes = Vec::new();
    for (issue, handle) in handles {
        match handle.await {
            Ok(outcome) => outcomes.push(outcome),
            Err(error) => {
                tracing::warn!(
                    draw_issue_id = %issue.id,
                    lottery_id = %issue.lottery_id,
                    issue = %issue.issue,
                    error = %error,
                    "自动开奖并发写入期号任务执行失败"
                );
                outcomes.push(DrawExecutionOutcome {
                    order: usize::MAX,
                    source_issue: issue,
                    result: Err(ApiError::Internal("自动开奖并发任务执行失败".to_string())),
                });
            }
        }
    }
    outcomes.sort_by_key(|outcome| outcome.order);

    Ok(outcomes)
}
/// 规范化自动开奖请求中的当前时间参数。
fn normalize_automation_now(payload: &DrawAutomationRunRequest) -> ApiResult<String> {
    let now = payload.now.trim().to_string();
    if now.is_empty() {
        return Err(ApiError::BadRequest(
            "automation time is required".to_string(),
        ));
    }

    Ok(now)
}
/// 创建没有任何处理结果的自动开奖运行记录。
fn empty_draw_automation_run(now: &str) -> DrawAutomationRun {
    DrawAutomationRun {
        now: now.to_string(),
        closed_issues: Vec::new(),
        drawn_issues: Vec::new(),
        settlement_runs: Vec::new(),
        ledger_entries: Vec::new(),
        skipped_issues: Vec::new(),
    }
}

/// 合并两个自动化阶段的结果，跳过原因按期号去重，避免停售彩种重复告警。
pub fn merge_draw_automation_runs(
    mut first: DrawAutomationRun,
    second: DrawAutomationRun,
) -> DrawAutomationRun {
    first.closed_issues.extend(second.closed_issues);
    first.drawn_issues.extend(second.drawn_issues);
    first.settlement_runs.extend(second.settlement_runs);
    first.ledger_entries.extend(second.ledger_entries);
    for skipped in second.skipped_issues {
        if !first
            .skipped_issues
            .iter()
            .any(|existing| existing.draw_issue_id == skipped.draw_issue_id)
        {
            first.skipped_issues.push(skipped);
        }
    }
    first
}

/// 并发预取本轮涉及的 API 彩种最新期号，避免多个彩种串行等待第三方接口。
async fn prefetch_api_latest_issue_lookups(
    draws: &DrawRepository,
    issues: &[DrawIssue],
) -> HashMap<String, ApiLatestIssueLookup> {
    let mut seen_lotteries = HashSet::new();
    let mut handles = Vec::new();

    for issue in issues {
        if issue.draw_mode != DrawMode::Api || !seen_lotteries.insert(issue.lottery_id.clone()) {
            continue;
        }

        let draws = draws.clone();
        let lottery_id = issue.lottery_id.clone();
        let issue_label = issue.issue.clone();
        handles.push(tokio::spawn(async move {
            let lookup = match draws.latest_api_issue_for_lottery(&lottery_id).await {
                Ok(Some(latest_issue)) => ApiLatestIssueLookup::Found(latest_issue.issue),
                Ok(None) => ApiLatestIssueLookup::Missing,
                Err(error) => {
                    tracing::warn!(
                        lottery_id = %lottery_id,
                        issue = %issue_label,
                        error = %error.log_message(),
                        "API开奖旧期号重试上限判断读取最新期号失败"
                    );
                    ApiLatestIssueLookup::Missing
                }
            };
            (lottery_id, lookup)
        }));
    }

    let mut lookups = HashMap::new();
    for handle in handles {
        match handle.await {
            Ok((lottery_id, lookup)) => {
                lookups.insert(lottery_id, lookup);
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "API开奖最新期号并发任务执行失败"
                );
            }
        }
    }

    lookups
}

/// 并发预取 API 开奖号码，后续写库和结算仍按期号顺序串行执行。
async fn prefetch_api_draw_numbers(
    draws: &DrawRepository,
    issues: Vec<DrawIssue>,
) -> HashMap<String, ApiDrawNumberLookup> {
    let mut handles = Vec::new();
    for issue in issues {
        let draws = draws.clone();
        handles.push(tokio::spawn(async move {
            let lookup = match draws.api_draw_number_for_issue(&issue).await {
                Ok(draw_number) => ApiDrawNumberLookup::Ready(draw_number),
                Err(error) => {
                    let reason = automation_error_reason(&error);
                    tracing::warn!(
                        draw_issue_id = %issue.id,
                        lottery_id = %issue.lottery_id,
                        issue = %issue.issue,
                        error = %error.log_message(),
                        "自动开奖并发预取 API 开奖号码失败"
                    );
                    ApiDrawNumberLookup::Failed(reason)
                }
            };
            (issue.id, lookup)
        }));
    }

    let mut lookups = HashMap::new();
    for handle in handles {
        match handle.await {
            Ok((draw_issue_id, lookup)) => {
                lookups.insert(draw_issue_id, lookup);
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "API开奖号码并发任务执行失败"
                );
            }
        }
    }

    lookups
}

/// 判断期号是否已经到达封盘时间。
fn should_close(issue: &DrawIssue, now: &str) -> bool {
    issue.status == DrawIssueStatus::Open && is_due_at(&issue.sale_closed_at, now)
}

/// 判断是否需要处理封盘后未满员合买退款。
fn should_refund_unfilled_group_buy(issue: &DrawIssue, now: &str) -> bool {
    matches!(
        issue.status,
        DrawIssueStatus::Closed | DrawIssueStatus::Drawn
    ) && is_due_at(&issue.sale_closed_at, now)
}

/// 判断期号是否已经到达开奖时间。
fn should_draw(
    issue: &DrawIssue,
    now: &str,
    lottery_configs: &HashMap<String, LotteryAutomationConfig>,
) -> bool {
    let delay_seconds = if issue.draw_mode == DrawMode::Api {
        lottery_configs
            .get(&issue.lottery_id)
            .map(|config| config.api_draw_delay_seconds)
            .unwrap_or_default()
    } else {
        0
    };

    matches!(
        issue.status,
        DrawIssueStatus::Open | DrawIssueStatus::Closed
    ) && is_due_at_with_delay(&issue.scheduled_at, now, delay_seconds)
}

/// 判断排期时间是否已经早于或等于本轮调度时间。
fn is_due_at(value: &str, now: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value <= now
}

/// 判断排期时间加上开奖源延迟后是否已经早于或等于本轮调度时间。
fn is_due_at_with_delay(value: &str, now: &str, delay_seconds: u32) -> bool {
    if delay_seconds == 0 {
        return is_due_at(value, now);
    }

    match (parse_timestamp(value), parse_timestamp(now)) {
        (Some(scheduled_at), Some(now)) => {
            scheduled_at + Duration::seconds(i64::from(delay_seconds)) <= now
        }
        _ => is_due_at(value, now),
    }
}
/// 解析业务时间字符串，失败时返回空值。
fn parse_timestamp(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value.trim(), TIMESTAMP_FORMAT).ok()
}

/// 构造调度跳过期号明细，供后台调度历史展示。
fn skipped_issue(issue: &DrawIssue, reason: &str) -> DrawAutomationSkippedIssue {
    DrawAutomationSkippedIssue {
        draw_issue_id: issue.id.clone(),
        lottery_id: issue.lottery_id.clone(),
        issue: issue.issue.clone(),
        reason: reason.to_string(),
    }
}
/// 记录跳过期号并避免同一期号重复写入跳过原因。
fn push_skipped_issue_once(run: &mut DrawAutomationRun, issue: &DrawIssue, reason: &str) {
    if !run
        .skipped_issues
        .iter()
        .any(|skipped| skipped.draw_issue_id == issue.id)
    {
        run.skipped_issues.push(skipped_issue(issue, reason));
    }
}

/// 构造调度内部彩种期号键，和合买机器人保护标记保持一致。
fn group_buy_issue_key(lottery_id: &str, issue: &str) -> String {
    format!("{lottery_id}:{issue}")
}

/// 对 API 开奖旧期号做重试上限判断，超过最新期号 5 期后不再请求旧期号开奖号码。
fn stale_api_issue_retry_reason(
    latest_issue_cache: &HashMap<String, ApiLatestIssueLookup>,
    issue: &DrawIssue,
) -> Option<String> {
    if issue.draw_mode != DrawMode::Api {
        return None;
    }

    let issue_sequence = parse_api_sequence_issue(&issue.issue)?;
    let latest_lookup = latest_issue_cache.get(&issue.lottery_id)?;
    let ApiLatestIssueLookup::Found(latest_issue) = latest_lookup else {
        return None;
    };
    let latest_sequence = parse_api_sequence_issue(&latest_issue)?;
    if latest_sequence <= issue_sequence {
        return None;
    }

    let distance = latest_sequence - issue_sequence;
    if distance <= API_DRAW_RETRY_MAX_LATEST_ISSUE_DISTANCE {
        return None;
    }

    tracing::warn!(
        draw_issue_id = %issue.id,
        lottery_id = %issue.lottery_id,
        issue = %issue.issue,
        latest_issue = %latest_issue,
        issue_distance = distance,
        retry_limit = API_DRAW_RETRY_MAX_LATEST_ISSUE_DISTANCE,
        "API开奖期号已落后最新期号超过限制，停止重试旧期号"
    );

    Some(format!(
        "距离开奖源最新期号 {latest_issue} 已超过 {API_DRAW_RETRY_MAX_LATEST_ISSUE_DISTANCE} 期，停止重试旧期号"
    ))
}
/// 并发预取自动开奖所需的彩种、开奖源和期号配置。
async fn lottery_automation_configs(
    lotteries: &crate::services::lottery::LotteryRepository,
) -> ApiResult<HashMap<String, LotteryAutomationConfig>> {
    let mut configs = HashMap::new();

    for lottery in lotteries.list().await? {
        let lottery_id = lottery.id.clone();
        configs.insert(
            lottery_id,
            LotteryAutomationConfig {
                api_draw_delay_seconds: lottery.api_draw_delay_seconds,
                draw_control_enabled: lottery.draw_control_enabled,
                lottery,
            },
        );
    }

    Ok(configs)
}

/// 判断当前彩种是否允许调度阶段使用开奖号码控制配置。
fn lottery_draw_control_enabled(
    issue: &DrawIssue,
    lottery_configs: &HashMap<String, LotteryAutomationConfig>,
) -> bool {
    lottery_configs
        .get(&issue.lottery_id)
        .map(|config| config.draw_control_enabled)
        .unwrap_or_default()
}

/// 判断彩种配置缺失时是否需要跳过自动封盘开奖。
///
/// 彩种停售只阻止后续生成新期号；已经创建的 open/closed 期号仍要继续封盘、
/// 流单退款、开奖和结算，避免运营停售后历史期号永久卡在封盘状态。
fn skip_issue_if_lottery_config_missing(
    issue: &DrawIssue,
    lottery_configs: &HashMap<String, LotteryAutomationConfig>,
) -> Option<&'static str> {
    if lottery_configs.contains_key(&issue.lottery_id) {
        None
    } else {
        Some("未找到彩种配置，跳过自动任务")
    }
}

/// 把自动开奖内部错误转换为后台调度历史可读的中文原因。
fn automation_error_reason(error: &ApiError) -> String {
    match error {
        ApiError::BadRequest(message) => format!("请求错误：{message}"),
        ApiError::Unauthorized(message) => format!("未授权：{message}"),
        ApiError::Forbidden(message) => format!("权限不足：{message}"),
        ApiError::NotFound(message) => format!("资源不存在：{message}"),
        ApiError::Conflict(message) => format!("资源冲突：{message}"),
        ApiError::Internal(message) => format!("内部错误：{message}"),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            draw::{
                CreateDrawIssueRequest, DrawAutomationRunRequest, DrawControlTargetScope,
                DrawIssue, DrawIssueResultRequest, DrawIssueStatus, SaveLotteryDrawControlRequest,
            },
            finance::LedgerEntryKind,
            group_buy::{CreateGroupBuyPlanRequest, GroupBuyPlanStatus},
            lottery::{
                DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType,
                LotteryPlayConfig, PlayCategory,
            },
            order::CreateOrderRequest,
            play::{PlayRuleCode, PlaySelection},
        },
        services::{
            access::AccessRepository,
            automation::{api_draw_number_failure_is_missing_old_issue, run_draw_automation},
            draw::DrawRepository,
            draw_api::ApiDrawSourceRepository,
            finance::FinanceRepository,
            group_buy::GroupBuyRepository,
            lottery::LotteryRepository,
            order::OrderRepository,
        },
    };

    const API68_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "操作成功",
        "result": {
            "businessCode": 0,
            "message": "操作成功",
            "data": [
                { "preDrawIssue": 2026143, "preDrawCode": "3,7,6", "preDrawTime": "2026-06-02 21:15:00" }
            ]
        }
    }"#;
    /// 验证 API 缺失期号判断只把超过重试窗口的旧期识别为可自动取消。
    #[test]
    fn api_missing_old_issue_detection_respects_retry_window() {
        let old_issue = api_detection_issue("202606131441");
        let near_issue = api_detection_issue("2026138");
        let future_issue = api_detection_issue("2099999");

        assert!(api_draw_number_failure_is_missing_old_issue(
            &old_issue,
            "资源不存在：API 开奖源未找到期号 `202606131441` 的开奖号码，当前返回期号 `20260701191`",
        ));
        assert!(!api_draw_number_failure_is_missing_old_issue(
            &near_issue,
            "资源不存在：API 开奖源未找到期号 `2026138` 的开奖号码，当前返回期号 `2026143`",
        ));
        assert!(!api_draw_number_failure_is_missing_old_issue(
            &future_issue,
            "资源不存在：API 开奖源未找到期号 `2099999` 的开奖号码，当前返回期号 `2026143`",
        ));
    }

    /// 验证自动开奖封盘draws结算和入账due期号。
    #[tokio::test]
    async fn automation_closes_draws_settles_and_credits_due_issue() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Api);
        let issue = draws
            .create(&lottery, create_request("AUTO20260602001"))
            .await
            .expect("issue can be created");
        orders
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: lottery.id.clone(),
                    issue: issue.issue.clone(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: full_direct_selection(),
                    unit_amount_minor: 1,
                },
            )
            .await
            .expect("order can be created");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can run");
        let stored = draws.get(&issue.id).await.expect("issue still exists");
        let accounts = finance.accounts().await.expect("accounts can be listed");
        let account = accounts
            .iter()
            .find(|account| account.user_id == "U10001")
            .expect("seeded account exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert_eq!(run.drawn_issues.len(), 1);
        assert_eq!(run.settlement_runs.len(), 1);
        assert_eq!(run.ledger_entries.len(), 1);
        assert_eq!(stored.status, DrawIssueStatus::Drawn);
        assert_eq!(account.available_balance_minor, 12_100);
        assert!(stored
            .draw_number
            .as_deref()
            .is_some_and(|number| { number.split(',').count() == 3 }));
    }

    /// 验证调度器会重试已开奖但未生成结算批次的期号，避免漏派奖后长期卡住。
    #[tokio::test]
    async fn automation_retries_unsettled_drawn_issue_payouts() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Api);
        let issue = draws
            .create(&lottery, create_request("DRAWN_UNSETTLED"))
            .await
            .expect("issue can be created");
        orders
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: lottery.id.clone(),
                    issue: issue.issue.clone(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: full_direct_selection(),
                    unit_amount_minor: 1,
                },
            )
            .await
            .expect("order can be created");
        draws
            .draw(
                &issue.id,
                DrawIssueResultRequest {
                    draw_number: Some("1,2,3".to_string()),
                },
            )
            .await
            .expect("issue can be marked as drawn without settlement");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can retry unsettled drawn issue");
        let accounts = finance.accounts().await.expect("accounts can be listed");
        let account = accounts
            .iter()
            .find(|account| account.user_id == "U10001")
            .expect("seeded account exists");

        assert!(run.drawn_issues.is_empty());
        assert_eq!(run.settlement_runs.len(), 1);
        assert_eq!(run.ledger_entries.len(), 1);
        assert_eq!(account.available_balance_minor, 12_100);
    }

    /// 验证彩种停售后，已经创建的期号仍会继续封盘和开奖。
    #[tokio::test]
    async fn automation_continues_existing_issue_when_lottery_stopped() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let mut lottery = lottery(DrawMode::Platform);
        lottery.sale_enabled = false;
        lotteries
            .update(&lottery.id, lottery.clone())
            .await
            .expect("lottery config can be saved");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let issue = draws
            .create(&lottery, create_request("AUTO_STOPPED"))
            .await
            .expect("issue can be created");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can run when lottery stopped");
        let stored = draws.get(&issue.id).await.expect("issue still exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert_eq!(run.drawn_issues.len(), 1);
        assert!(run.skipped_issues.is_empty());
        assert_eq!(stored.status, DrawIssueStatus::Drawn);
        assert!(stored.draw_number.is_some());
    }
    /// 验证自动开奖skipsdue人工期号without开奖号码。
    #[tokio::test]
    async fn automation_skips_due_manual_issue_without_draw_number() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Manual);
        let issue = draws
            .create(&lottery, create_request("MANUAL20260602001"))
            .await
            .expect("issue can be created");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can run");
        let stored = draws.get(&issue.id).await.expect("issue still exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert!(run.drawn_issues.is_empty());
        assert_eq!(run.skipped_issues.len(), 1);
        assert_eq!(stored.status, DrawIssueStatus::Closed);
        assert!(run.skipped_issues[0].reason.contains("管理员录入开奖号码"));
    }
    /// 验证自动开奖drawsdue人工期号带control号码。
    #[tokio::test]
    async fn automation_draws_due_manual_issue_with_control_number() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Manual);
        draws
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
                    target_scope: DrawControlTargetScope::Issue,
                    target_issue: Some("MANUAL20260602002".to_string()),
                    target_order_id: None,
                },
            )
            .await
            .expect("draw control can be saved");
        let issue = draws
            .create(&lottery, create_request("MANUAL20260602002"))
            .await
            .expect("issue can be created");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can draw controlled manual issue");
        let stored = draws.get(&issue.id).await.expect("issue still exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert_eq!(run.drawn_issues.len(), 1);
        assert!(run.skipped_issues.is_empty());
        assert_eq!(stored.status, DrawIssueStatus::Drawn);
        assert_eq!(stored.draw_number.as_deref(), Some("2,4,7"));
    }
    /// 验证自动开奖skipsAPI期号when开奖来源misses期号。
    #[tokio::test]
    async fn automation_skips_api_issue_when_draw_source_misses_issue() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Api);
        let issue = draws
            .create(&lottery, create_request("2099999"))
            .await
            .expect("issue can be created");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can skip api issue");
        let stored = draws.get(&issue.id).await.expect("issue still exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert!(run.drawn_issues.is_empty());
        assert_eq!(run.skipped_issues.len(), 1);
        assert_eq!(stored.status, DrawIssueStatus::Closed);
        assert!(stored.draw_number.is_none());
        assert!(run.skipped_issues[0].reason.contains("未找到"));
    }
    /// 验证超过重试窗口且无业务单据的 API 旧期号会自动取消，避免反复拖慢调度。
    #[tokio::test]
    async fn automation_cancels_stale_api_issue_without_pending_business() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Api);
        let issue = draws
            .create(&lottery, create_request("2026137"))
            .await
            .expect("stale api issue can be created");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can skip stale api issue");
        let stored = draws.get(&issue.id).await.expect("issue still exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert!(run.drawn_issues.is_empty());
        assert_eq!(run.skipped_issues.len(), 1);
        assert_eq!(stored.status, DrawIssueStatus::Cancelled);
        assert!(stored.draw_number.is_none());
        assert!(run.skipped_issues[0].reason.contains("停止重试旧期号"));
        assert!(run.skipped_issues[0].reason.contains("已自动取消"));
    }
    /// 验证超过重试窗口但存在待开奖订单的 API 旧期号不会自动取消。
    #[tokio::test]
    async fn automation_keeps_stale_api_issue_with_pending_order() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Api);
        let issue = draws
            .create(&lottery, create_request("2026137"))
            .await
            .expect("stale api issue can be created");
        orders
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: lottery.id.clone(),
                    issue: issue.issue.clone(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: full_direct_selection(),
                    unit_amount_minor: 1,
                },
            )
            .await
            .expect("pending order can be created");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can keep stale api issue with pending order");
        let stored = draws.get(&issue.id).await.expect("issue still exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert!(run.drawn_issues.is_empty());
        assert_eq!(run.skipped_issues.len(), 1);
        assert_eq!(stored.status, DrawIssueStatus::Closed);
        assert!(stored.draw_number.is_none());
        assert!(run.skipped_issues[0].reason.contains("停止重试旧期号"));
        assert!(!run.skipped_issues[0].reason.contains("已自动取消"));
    }
    /// 验证自动开奖keepsretryingAPI期号withinfive期号distance。
    #[tokio::test]
    async fn automation_keeps_retrying_api_issue_within_five_issue_distance() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Api);
        let issue = draws
            .create(&lottery, create_request("2026138"))
            .await
            .expect("api issue can be created");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 22:00:00".to_string(),
            },
        )
        .await
        .expect("automation can retry api issue in retry window");
        let stored = draws.get(&issue.id).await.expect("issue still exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert!(run.drawn_issues.is_empty());
        assert_eq!(run.skipped_issues.len(), 1);
        assert_eq!(stored.status, DrawIssueStatus::Closed);
        assert!(stored.draw_number.is_none());
        assert!(run.skipped_issues[0].reason.contains("未找到"));
        assert!(!run.skipped_issues[0].reason.contains("停止重试旧期号"));
    }
    /// 验证自动开奖退款unfilled合买合买when期号封盘。
    #[tokio::test]
    async fn automation_refunds_unfilled_group_buy_when_issue_closes() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let lottery = lottery(DrawMode::Api);
        let issue = draws
            .create(&lottery, create_request("GROUP_BUY_CLOSE"))
            .await
            .expect("issue can be created");
        let plan = group_buys
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-AUTO-CLOSE-001".to_string(),
                    lottery_id: lottery.id.clone(),
                    issue: issue.issue.clone(),
                    rule_code: "threeDirect".to_string(),
                    title: "封盘流单测试".to_string(),
                    numbers: "1|2|3".to_string(),
                    initiator_user_id: "U90001".to_string(),
                    total_amount_minor: 100_000,
                    initiator_amount_minor: 10_000,
                    note: "封盘流单测试".to_string(),
                },
                std::slice::from_ref(&lottery),
                &access.users,
            )
            .await
            .expect("group buy plan can be created");
        finance
            .debit_group_buy(
                &plan.initiator_user_id,
                plan.filled_amount_minor,
                "G-AUTO-CLOSE-001-P001",
                &plan.id,
            )
            .await
            .expect("group buy debit can be written");

        let run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 20:59:45".to_string(),
            },
        )
        .await
        .expect("automation can close and refund unfilled group buy");
        let stored_plan = group_buys.get(&plan.id).await.expect("plan exists");

        assert_eq!(run.closed_issues.len(), 1);
        assert!(run.drawn_issues.is_empty());
        assert_eq!(stored_plan.status, GroupBuyPlanStatus::Cancelled);
        assert_eq!(run.ledger_entries.len(), 1);
        assert_eq!(run.ledger_entries[0].kind, LedgerEntryKind::GroupBuyRefund);
    }
    /// 验证自动开奖waits用于API开奖delay之前requesting来源。
    #[tokio::test]
    async fn automation_waits_for_api_draw_delay_before_requesting_source() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let lotteries = LotteryRepository::memory_seeded();
        let mut api_lottery = lottery(DrawMode::Api);
        api_lottery.api_draw_delay_seconds = 30;
        lotteries
            .update(&api_lottery.id, api_lottery.clone())
            .await
            .expect("api lottery delay can be saved");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let issue = draws
            .create(&api_lottery, create_request("2026143"))
            .await
            .expect("issue can be created");

        let early_run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 21:00:30".to_string(),
            },
        )
        .await
        .expect("automation can run before api delay is reached");
        let early_issue = draws.get(&issue.id).await.expect("issue exists");

        assert_eq!(early_run.closed_issues.len(), 1);
        assert!(early_run.drawn_issues.is_empty());
        assert!(early_run.skipped_issues.is_empty());
        assert_eq!(early_issue.status, DrawIssueStatus::Closed);

        let due_run = run_draw_automation(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            DrawAutomationRunRequest {
                now: "2026-06-02 21:00:45".to_string(),
            },
        )
        .await
        .expect("automation can draw after api delay is reached");

        assert_eq!(due_run.drawn_issues.len(), 1);
        assert_eq!(
            due_run.drawn_issues[0].draw_number.as_deref(),
            Some("3,7,6")
        );
    }

    /// 构造测试用创建请求。
    fn create_request(issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: "fc3d".to_string(),
            issue: issue.to_string(),
            scheduled_at: "2026-06-02 21:00:15".to_string(),
            sale_closed_at: "2026-06-02 20:59:45".to_string(),
        }
    }

    /// 构造测试或种子使用的彩种配置。
    fn lottery(draw_mode: DrawMode) -> LotteryKind {
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            category: "regional".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode,
            api_draw_delay_seconds: 0,
            draw_control_enabled: true,
            avoid_winning_enabled: false,
            issue_format: crate::domain::lottery::DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            sale_close_lead_seconds: crate::domain::lottery::DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: true,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1_000,
            },
            play_categories: vec![PlayCategory::Direct],
            play_configs: vec![LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeDirect,
                enabled: true,
                odds_basis_points: 10_000,
                position_select_limits: Vec::new(),
            }],
        }
    }

    /// 构造用于 API 旧期判断的最小期号对象。
    fn api_detection_issue(issue: &str) -> DrawIssue {
        DrawIssue {
            id: "D-API-DETECTION".to_string(),
            lottery_id: "txffc".to_string(),
            lottery_name: "腾讯分分彩".to_string(),
            issue: issue.to_string(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Api,
            scheduled_at: "2026-06-02 21:00:15".to_string(),
            sale_closed_at: "2026-06-02 20:59:45".to_string(),
            status: DrawIssueStatus::Closed,
            draw_number: None,
            drawn_at: None,
            created_at: "2026-06-02 20:00:00".to_string(),
        }
    }

    /// 为自动开奖测试显式打开指定彩种销售状态。
    async fn enable_lottery_sale(lotteries: &LotteryRepository, lottery_id: &str) {
        lotteries
            .set_sale_enabled(lottery_id, true)
            .await
            .expect("lottery sale can be enabled for automation test");
    }

    /// 构造覆盖直选全部位置的测试选号。
    fn full_direct_selection() -> PlaySelection {
        let all_digits = (0..=9).collect::<Vec<_>>();
        PlaySelection {
            positions: vec![all_digits.clone(), all_digits.clone(), all_digits],
            ..PlaySelection::default()
        }
    }
}
