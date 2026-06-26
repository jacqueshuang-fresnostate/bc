//! 机器人执行服务，合买机器人只负责发单，补单机器人只负责认购未满单合买。

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use chrono::NaiveDateTime;
use random_zh::{random_zh, RandomZhOptions};
use tokio::{
    sync::{Mutex as AsyncMutex, Semaphore},
    task::JoinHandle,
};

use crate::services::group_buy_flow::create_order_for_filled_group_buy_before_draw_guard;
use crate::{
    domain::{
        draw::{DrawIssue, DrawIssueStatus},
        finance::{LedgerEntry, ManualBalanceAdjustmentRequest},
        group_buy::{
            AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyPlan,
            GroupBuyPlanStatus,
        },
        lottery::{LotteryKind, LotteryPlayConfig, LotteryPlayPositionSelectLimit},
        order::CreateOrderRequest,
        play::PlayRuleCode,
        robot::{
            GroupBuyRobotFillStrategy, GroupBuyRobotRun, GroupBuyRobotSkippedItem,
            RobotConfigSummary, RobotKind, RobotStatus,
        },
        user::UserSummary,
    },
    error::{ApiError, ApiResult},
    services::{
        access::AccessRepository,
        business_database::enum_to_string,
        draw::DrawRepository,
        finance::FinanceRepository,
        group_buy::GroupBuyRepository,
        group_buy_flow::{create_order_for_filled_group_buy, parse_group_buy_selection},
        lottery::LotteryRepository,
        order::OrderRepository,
        robot::RobotRepository,
    },
};

/// 系统合买发单机器人固定用户 ID，资金过滤和删除保护都依赖该值。
pub const ROBOT_GROUP_BUY_USER_ID: &str = "U90001";
const ROBOT_GROUP_BUY_USERNAME: &str = "agent_alpha";
const ROBOT_FILL_PARTICIPANT_SUFFIX: &str = "P-ROBOT-FILL";
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const ROBOT_AUTO_CREDIT_RESERVE_MINOR: i64 = 100_000;
/// 合买兜底失败后最大保护秒数，超过后不再暂缓开奖，强制流单退款。
const ROBOT_GUARD_PROTECT_MAX_SECONDS: i64 = 180;
const ROBOT_FILL_STAGE_COUNT: i64 = 5;
const ROBOT_FILL_USERS_PER_STAGE_MIN: usize = 5;
const ROBOT_FILL_USERS_PER_STAGE_MAX: usize = 10;
/// 系统补单机器人预置用户 ID，资金过滤、清理保护和对外匿名展示都依赖该集合。
pub const ROBOT_GROUP_BUY_USER_IDS: [&str; 10] = [
    "U90001", "X90002", "X90003", "X90004", "X90005", "X90006", "X90007", "X90008", "X90009",
    "X90010",
];
const ROBOT_CONCURRENT_JOB_LIMIT: usize = 8;
const ROBOT_BASE_UNIT_AMOUNT_MINOR: i64 = 200;
const ROBOT_MAX_MULTIPLE: usize = 20;
const ROBOT_MAX_POSITION_DIGIT_COUNT: usize = 5;
const ROBOT_MAX_NUMBER_POOL_COUNT: usize = 6;
const ROBOT_MAX_BIG_SMALL_ATTRIBUTE_COUNT: usize = 3;
const ROBOT_DISPLAY_NAME_FALLBACKS: [&str; 8] = [
    "林清远",
    "沈知安",
    "许明澜",
    "顾云舟",
    "周景和",
    "陈亦然",
    "陆星河",
    "叶青岚",
];

type RobotFinanceMutationLock = Arc<AsyncMutex<()>>;
type RobotIssueMutationLock = Arc<AsyncMutex<()>>;

#[derive(Clone)]
struct GroupBuyRobotJob {
    robot: RobotConfigSummary,
    lottery: LotteryKind,
    issue: DrawIssue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RobotFillPolicy {
    GuaranteedUserPlan,
    Rhythm {
        max_percent: u32,
        stage_count: u32,
        fill_before_draw_seconds: i64,
    },
    BeforeDraw {
        fill_before_draw_seconds: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RobotFillStage {
    Stage { index: u32 },
    Final,
}

/// 判断用户 ID 是否为系统合买或补单机器人账户。
pub fn is_group_buy_robot_user_id(user_id: &str) -> bool {
    let user_id = user_id.trim();
    ROBOT_GROUP_BUY_USER_IDS
        .iter()
        .any(|robot_user_id| user_id == *robot_user_id)
}

/// 构造空的合买机器人执行结果，供串行和并发任务统一汇总。
fn empty_group_buy_robot_run(now: String) -> GroupBuyRobotRun {
    GroupBuyRobotRun {
        now,
        created_plans: Vec::new(),
        filled_plans: Vec::new(),
        created_orders: Vec::new(),
        ledger_entries: Vec::new(),
        skipped_items: Vec::new(),
        protected_plan_ids: Vec::new(),
        protected_issue_keys: Vec::new(),
    }
}

/// 将单个并发任务的执行结果合并到本轮总结果。
fn append_group_buy_robot_run(target: &mut GroupBuyRobotRun, source: GroupBuyRobotRun) {
    target.created_plans.extend(source.created_plans);
    target.filled_plans.extend(source.filled_plans);
    target.created_orders.extend(source.created_orders);
    target.ledger_entries.extend(source.ledger_entries);
    target.skipped_items.extend(source.skipped_items);
    extend_unique_strings(&mut target.protected_plan_ids, source.protected_plan_ids);
    extend_unique_strings(
        &mut target.protected_issue_keys,
        source.protected_issue_keys,
    );
}

/// 生成同一彩种同一期号的并发互斥键，避免多个机器人同时补同一个期号。
fn group_buy_robot_issue_key(lottery_id: &str, issue: &str) -> String {
    format!("{lottery_id}:{issue}")
}

/// 向字符串列表追加去重值，用于合并并发机器人内部保护标记。
fn push_unique_string(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

/// 合并字符串列表并保持首次出现顺序，避免调度保护项重复膨胀。
fn extend_unique_strings(target: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        push_unique_string(target, value);
    }
}

/// 标记合买计划在本轮不能流单退款，对应期号也不能继续开奖。
fn protect_group_buy_plan(run: &mut GroupBuyRobotRun, plan: &GroupBuyPlan) {
    push_unique_string(&mut run.protected_plan_ids, plan.id.clone());
    push_unique_string(
        &mut run.protected_issue_keys,
        group_buy_robot_issue_key(&plan.lottery_id, &plan.issue),
    );
}

/// 判断期号是否已超过最大保护时限，超期后不应再暂缓开奖。
/// 以期号计划开奖时间为基准，超过 ROBOT_GUARD_PROTECT_MAX_SECONDS 秒即视为超期。
fn issue_guard_protection_expired(issue: &DrawIssue, now_at: NaiveDateTime) -> bool {
    let Ok(scheduled_at) =
        NaiveDateTime::parse_from_str(issue.scheduled_at.trim(), TIMESTAMP_FORMAT)
    else {
        return true;
    };
    now_at - scheduled_at > chrono::Duration::seconds(ROBOT_GUARD_PROTECT_MAX_SECONDS)
}

/// 选择流单前兜底补单机器人，优先使用绑定当前彩种的配置，没有绑定时使用任意启用补单机器人。
fn guard_robot_for_lottery<'a>(
    robots: &'a [RobotConfigSummary],
    lottery_id: &str,
) -> Option<&'a RobotConfigSummary> {
    robots
        .iter()
        .find(|robot| {
            robot.kind == RobotKind::Purchase
                && robot.status == RobotStatus::Enabled
                && robot.lottery_ids.iter().any(|id| id == lottery_id)
        })
        .or_else(|| {
            robots.iter().find(|robot| {
                robot.kind == RobotKind::Purchase && robot.status == RobotStatus::Enabled
            })
        })
}

/// 执行全部已启用的机器人，并返回本轮创建、满单、成单和跳过明细。
pub async fn run_group_buy_robots(
    robots: &RobotRepository,
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    access: &AccessRepository,
    now: String,
) -> ApiResult<GroupBuyRobotRun> {
    let now = required_now(now)?;
    let now_at = parse_robot_timestamp(&now, "机器人执行时间")?;
    let access = access.snapshot().await?;
    let users = Arc::new(access.users);
    let robot_user = robot_user(users.as_ref().as_slice())?.clone();
    let lotteries_by_id = lotteries
        .list()
        .await?
        .into_iter()
        .map(|lottery| (lottery.id.clone(), lottery))
        .collect::<BTreeMap<_, _>>();
    let mut run = empty_group_buy_robot_run(now.clone());
    let mut jobs = Vec::new();
    let mut issue_locks = BTreeMap::<String, RobotIssueMutationLock>::new();

    let all_robots = robots.list().await?;
    for robot in &all_robots {
        if !matches!(robot.kind, RobotKind::GroupBuy | RobotKind::Purchase) {
            continue;
        }
        if robot.status != RobotStatus::Enabled {
            push_skipped(&mut run, robot, "", None, "机器人未启用，跳过执行");
            continue;
        }

        for lottery_id in &robot.lottery_ids {
            let Some(lottery) = lotteries_by_id.get(lottery_id) else {
                push_skipped(&mut run, robot, lottery_id, None, "绑定彩种不存在");
                continue;
            };
            if !lottery.sale_enabled {
                push_skipped(&mut run, robot, &lottery.id, None, "彩种已停售");
                continue;
            }
            if !lottery.group_buy.enabled {
                push_skipped(&mut run, robot, &lottery.id, None, "彩种未开启合买");
                continue;
            }

            let Some(issue) = current_open_issue(draws, lottery, &now).await? else {
                push_skipped(&mut run, robot, &lottery.id, None, "没有可销售的当前期号");
                continue;
            };

            let issue_key = group_buy_robot_issue_key(&lottery.id, &issue.issue);
            issue_locks
                .entry(issue_key)
                .or_insert_with(|| Arc::new(AsyncMutex::new(())));
            jobs.push(GroupBuyRobotJob {
                robot: robot.clone(),
                lottery: lottery.clone(),
                issue,
            });
        }
    }

    // 确保每个启用了合买的彩种至少有一个机器人发单
    let covered_lottery_ids: BTreeSet<String> = jobs
        .iter()
        .filter(|job| job.robot.kind == RobotKind::GroupBuy)
        .map(|job| job.lottery.id.clone())
        .collect();
    let default_group_buy_robot = all_robots
        .iter()
        .find(|r| r.kind == RobotKind::GroupBuy && r.status == RobotStatus::Enabled);
    if let Some(default_robot) = default_group_buy_robot {
        for (lottery_id, lottery) in &lotteries_by_id {
            if !lottery.sale_enabled || !lottery.group_buy.enabled {
                continue;
            }
            if covered_lottery_ids.contains(lottery_id) {
                continue;
            }
            if let Some(issue) = current_open_issue(draws, lottery, &now).await? {
                tracing::info!(
                    lottery_id = lottery_id.as_str(),
                    issue = %issue.issue,
                    "合买机器人未覆盖该彩种，使用默认机器人补发合买计划"
                );
                let issue_key = group_buy_robot_issue_key(lottery_id, &issue.issue);
                issue_locks
                    .entry(issue_key)
                    .or_insert_with(|| Arc::new(AsyncMutex::new(())));
                jobs.push(GroupBuyRobotJob {
                    robot: default_robot.clone(),
                    lottery: lottery.clone(),
                    issue,
                });
            }
        }
    }
    if jobs.is_empty() {
        return Ok(run);
    }

    let job_count = jobs.len();
    tracing::info!(
        "并发任务数" = job_count,
        "并发上限" = ROBOT_CONCURRENT_JOB_LIMIT,
        "合买机器人并发执行开始"
    );
    let semaphore = Arc::new(Semaphore::new(ROBOT_CONCURRENT_JOB_LIMIT));
    let finance_lock = Arc::new(AsyncMutex::new(()));
    let mut handles = Vec::<JoinHandle<ApiResult<GroupBuyRobotRun>>>::with_capacity(job_count);
    for job in jobs {
        let job_key = group_buy_robot_issue_key(&job.lottery.id, &job.issue.issue);
        let issue_lock = issue_locks
            .get(&job_key)
            .cloned()
            .ok_or_else(|| ApiError::Internal("合买机器人期号并发锁缺失".to_string()))?;
        let draws = draws.clone();
        let orders = orders.clone();
        let finance = finance.clone();
        let group_buys = group_buys.clone();
        let users = Arc::clone(&users);
        let robot_user = robot_user.clone();
        let now = now.clone();
        let semaphore = Arc::clone(&semaphore);
        let finance_lock = Arc::clone(&finance_lock);
        handles.push(tokio::spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .map_err(|_| ApiError::Internal("合买机器人并发许可获取失败".to_string()))?;
            let _issue_guard = issue_lock.lock().await;
            run_group_buy_robot_job(
                job,
                &draws,
                &orders,
                &finance,
                &group_buys,
                users,
                robot_user,
                finance_lock,
                now_at,
                now,
            )
            .await
        }));
    }

    for handle in handles {
        let job_run = handle.await.map_err(|error| {
            ApiError::Internal(format!("合买机器人并发任务执行失败：{error}"))
        })??;
        append_group_buy_robot_run(&mut run, job_run);
    }
    tracing::info!(
        "并发任务数" = job_count,
        "创建计划数" = run.created_plans.len(),
        "满单计划数" = run.filled_plans.len(),
        "创建订单数" = run.created_orders.len(),
        "跳过项数" = run.skipped_items.len(),
        "合买机器人并发执行完成"
    );

    Ok(run)
}

/// 执行单个“机器人 + 彩种 + 当前期号”任务，供并发调度器按任务粒度运行。
async fn run_group_buy_robot_job(
    job: GroupBuyRobotJob,
    draws: &DrawRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    users: Arc<Vec<UserSummary>>,
    robot_user: UserSummary,
    finance_lock: RobotFinanceMutationLock,
    now_at: NaiveDateTime,
    now: String,
) -> ApiResult<GroupBuyRobotRun> {
    let mut run = empty_group_buy_robot_run(now);
    if job.robot.kind == RobotKind::GroupBuy {
        if let Err(error) = execute_lottery_robot(
            &mut run,
            &job.robot,
            &job.lottery,
            &job.issue,
            orders,
            finance,
            group_buys,
            users.as_ref().as_slice(),
            &robot_user,
            &finance_lock,
            now_at,
        )
        .await
        {
            push_skipped(
                &mut run,
                &job.robot,
                &job.lottery.id,
                Some(job.issue.issue.clone()),
                error.to_string(),
            );
        }
    }
    if job.robot.kind == RobotKind::Purchase {
        fill_existing_group_buy_plans(
            &mut run,
            &job.robot,
            &job.lottery,
            &job.issue,
            draws,
            orders,
            finance,
            group_buys,
            users.as_ref().as_slice(),
            &finance_lock,
            now_at,
        )
        .await?;
    }

    Ok(run)
}

/// 封盘流单退款前强制补满用户合买，避免调度错过分阶段窗口导致用户计划被动流单。
pub async fn force_fill_user_group_buy_plans_before_refund(
    robots: &RobotRepository,
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    access: &AccessRepository,
    now: String,
) -> ApiResult<GroupBuyRobotRun> {
    let now = required_now(now)?;
    let now_at = parse_robot_timestamp(&now, "机器人兜底执行时间")?;
    let lotteries_by_id = lotteries
        .list()
        .await?
        .into_iter()
        .map(|lottery| (lottery.id.clone(), lottery))
        .collect::<BTreeMap<_, _>>();
    let robots = robots.list().await?;
    let candidate_issues = draws
        .list_scheduler_active()
        .await?
        .into_iter()
        .filter(|issue| issue.sale_closed_at.as_str() <= now.as_str())
        .map(|issue| ((issue.lottery_id.clone(), issue.issue.clone()), issue))
        .collect::<BTreeMap<_, _>>();
    let group_buy_plans = group_buys.list_details().await?;
    let candidate_plans = group_buy_plans
        .iter()
        .filter(|plan| {
            user_group_buy_plan_needs_guard_fill(plan)
                && candidate_issues.contains_key(&(plan.lottery_id.clone(), plan.issue.clone()))
        })
        .cloned()
        .collect::<Vec<_>>();
    let filled_without_order_plans = group_buy_plans
        .into_iter()
        .filter(|plan| {
            user_group_buy_plan_needs_guard_order(plan)
                && candidate_issues.contains_key(&(plan.lottery_id.clone(), plan.issue.clone()))
        })
        .collect::<Vec<_>>();
    let mut run = empty_group_buy_robot_run(now.clone());
    if candidate_plans.is_empty() && filled_without_order_plans.is_empty() {
        return Ok(run);
    }

    let access = access.snapshot().await?;
    let finance_lock = Arc::new(AsyncMutex::new(()));
    for mut plan in candidate_plans {
        let Some(issue) = candidate_issues.get(&(plan.lottery_id.clone(), plan.issue.clone()))
        else {
            continue;
        };
        let Some(lottery) = lotteries_by_id.get(&plan.lottery_id) else {
            tracing::warn!(
                "合买计划ID" = %plan.id,
                "彩种ID" = %plan.lottery_id,
                "期号" = %plan.issue,
                "合买兜底补满找不到彩种配置，计划将进入流单退款检查"
            );
            continue;
        };
        if !lottery.sale_enabled || !lottery.group_buy.enabled {
            tracing::warn!(
                "合买计划ID" = %plan.id,
                "彩种ID" = %plan.lottery_id,
                "期号" = %plan.issue,
                "合买兜底补满跳过停售或未开启合买的彩种"
            );
            continue;
        }
        let Some(robot) = guard_robot_for_lottery(&robots, &lottery.id) else {
            tracing::warn!(
                "合买计划ID" = %plan.id,
                "彩种ID" = %plan.lottery_id,
                "期号" = %plan.issue,
                "合买兜底补满没有可用机器人，本轮暂缓流单和开奖等待下一轮兜底"
            );
            if !issue_guard_protection_expired(issue, now_at) {
                protect_group_buy_plan(&mut run, &plan);
            }
            continue;
        };
        if !robot.lottery_ids.iter().any(|id| id == &lottery.id) {
            tracing::warn!(
                "机器人ID" = %robot.id,
                "彩种ID" = %lottery.id,
                "期号" = %plan.issue,
                "合买兜底补满使用未绑定当前彩种的启用机器人兜底"
            );
        }

        if let Err(error) = fill_robot_plan(
            &mut run,
            robot,
            lottery,
            issue,
            &mut plan,
            draws,
            orders,
            finance,
            group_buys,
            &access.users,
            &finance_lock,
            now_at,
            RobotFillPolicy::GuaranteedUserPlan,
            Some(now.as_str()),
        )
        .await
        {
            tracing::warn!(
                "合买计划ID" = %plan.id,
                "彩种ID" = %plan.lottery_id,
                "期号" = %plan.issue,
                error = %error.log_message(),
                "合买兜底补满失败，本轮暂缓流单和开奖等待下一轮兜底"
            );
            if !issue_guard_protection_expired(issue, now_at) {
                protect_group_buy_plan(&mut run, &plan);
            }
            push_skipped(
                &mut run,
                robot,
                &lottery.id,
                Some(issue.issue.clone()),
                format!("合买计划 {} 兜底补满失败：{error}", plan.id),
            );
        }
    }
    for plan in filled_without_order_plans {
        let Some(issue) = candidate_issues.get(&(plan.lottery_id.clone(), plan.issue.clone()))
        else {
            continue;
        };
        let Some(lottery) = lotteries_by_id.get(&plan.lottery_id) else {
            tracing::warn!(
                "合买计划ID" = %plan.id,
                "彩种ID" = %plan.lottery_id,
                "期号" = %plan.issue,
                "合买已满单补建订单找不到彩种配置，计划暂时保留"
            );
            continue;
        };
        if !lottery.sale_enabled || !lottery.group_buy.enabled {
            tracing::warn!(
                "合买计划ID" = %plan.id,
                "彩种ID" = %plan.lottery_id,
                "期号" = %plan.issue,
                "合买已满单补建订单跳过停售或未开启合买的彩种"
            );
            continue;
        }
        match create_order_for_filled_group_buy_before_draw_guard(
            draws,
            orders,
            group_buys,
            lottery,
            &plan,
            now.as_str(),
        )
        .await
        {
            Ok(Some((order, attached_plan))) => {
                draws.record_avoidance_order_risk(&order).await;
                run.created_orders.push(order);
                run.filled_plans.push(attached_plan);
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(
                    "合买计划ID" = %plan.id,
                    "彩种ID" = %plan.lottery_id,
                    "期号" = %issue.issue,
                    error = %error.log_message(),
                    "合买已满单补建订单失败，计划暂时保留等待下一轮"
                );
                if !issue_guard_protection_expired(issue, now_at) {
                    protect_group_buy_plan(&mut run, &plan);
                }
                if let Some(robot) = guard_robot_for_lottery(&robots, &lottery.id) {
                    push_skipped(
                        &mut run,
                        robot,
                        &lottery.id,
                        Some(issue.issue.clone()),
                        format!("合买计划 {} 已满单补建订单失败：{error}", plan.id),
                    );
                }
            }
        }
    }

    Ok(run)
}

/// 发单后立即用多个机器人用户认购一部分剩余金额，让合买大厅从一开始就有多个参与者。
async fn seed_robot_plan_initial_participants(
    run: &mut GroupBuyRobotRun,
    plan: &mut GroupBuyPlan,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    users: &[UserSummary],
    finance_lock: &RobotFinanceMutationLock,
) -> ApiResult<()> {
    let remaining = plan
        .total_amount_minor
        .saturating_sub(plan.filled_amount_minor);
    if remaining <= 0 {
        return Ok(());
    }
    if plan
        .participants
        .iter()
        .any(|participant| participant.note.starts_with("机器人初始认购"))
    {
        return Ok(());
    }

    let min_share = plan.min_share_amount_minor.max(1);
    let participant_min = plan.participant_min_amount_minor.max(min_share).max(1);
    // 种子认购只认购一份最低认购额（一个机器人用户），让大厅不只有发起人，
    // 同时保留足够空间给后续节奏补单。
    let seed_target = participant_min;
    if seed_target > remaining {
        return Ok(());
    }
    // 种子认购后剩余必须为 0 或不低于最低认购额，否则跳过种子认购交由补单兜底，
    // 避免触发"认购后剩余金额低于最低认购额"校验。
    let leftover_after_seed = remaining - seed_target;
    if leftover_after_seed > 0 && leftover_after_seed < participant_min {
        return Ok(());
    }

    // 拆分份数受限于每份不得低于最低认购额，避免拆出低于最低认购额的零头。
    let max_users_by_min = ((seed_target / participant_min) as usize).max(1);
    let users_per_stage =
        robot_fill_users_per_stage(seed_target, participant_min).min(max_users_by_min);
    let split_amounts =
        split_fill_amount_evenly(seed_target, users_per_stage, participant_min, min_share);

    let display_users = users_with_random_robot_display_name(users);
    for (index, &split_amount) in split_amounts.iter().enumerate() {
        let robot_user_id = ROBOT_GROUP_BUY_USER_IDS[(index + 1) % ROBOT_GROUP_BUY_USER_IDS.len()];
        if let Some(entry) = ensure_robot_balance_locked(
            finance,
            finance_lock,
            robot_user_id,
            split_amount,
            "发单机器人初始种子认购",
        )
        .await?
        {
            run.ledger_entries.push(entry);
        }
        if finance
            .ensure_available(robot_user_id, split_amount)
            .await
            .is_err()
        {
            continue;
        }
        let participant_id = next_robot_fill_participant_id(plan);
        let note = format!("机器人初始认购 ({}/{})", index + 1, split_amounts.len());
        // 先写参与记录，扣款失败再回滚，避免扣了钱没加参与记录导致下一轮 ID 冲突。
        match group_buys
            .add_participant(
                &plan.id,
                AddGroupBuyParticipantRequest {
                    id: participant_id.clone(),
                    user_id: robot_user_id.to_string(),
                    amount_minor: split_amount,
                    note,
                },
                &display_users,
            )
            .await
        {
            Ok(updated_plan) => {
                *plan = updated_plan;
                match debit_group_buy_locked(
                    finance,
                    finance_lock,
                    robot_user_id,
                    split_amount,
                    &participant_id,
                    &plan.id,
                )
                .await
                {
                    Ok(entry) => {
                        run.ledger_entries.push(entry);
                    }
                    Err(error) => {
                        tracing::warn!(
                            "合买计划ID" = %plan.id,
                            robot_user_id = robot_user_id,
                            error = %error.log_message(),
                            "发单机器人初始种子认购扣款失败，回滚参与记录"
                        );
                        if let Err(rollback_error) = group_buys
                            .remove_unfunded_participant(&plan.id, &participant_id)
                            .await
                        {
                            tracing::error!(
                                "合买计划ID" = %plan.id,
                                participant_id = %participant_id,
                                error = %rollback_error.log_message(),
                                "发单机器人初始种子认购回滚参与记录失败"
                            );
                        }
                        if let Ok(rolled_back) = group_buys.get(&plan.id).await {
                            *plan = rolled_back;
                        }
                    }
                }
            }
            Err(error) => {
                tracing::warn!(
                    "合买计划ID" = %plan.id,
                    robot_user_id = robot_user_id,
                    error = %error.log_message(),
                    "发单机器人初始种子认购记录写入失败，跳过该用户"
                );
            }
        }
    }

    Ok(())
}

/// 对单个合买机器人和彩种执行发单；合买机器人不参与补单。
async fn execute_lottery_robot(
    run: &mut GroupBuyRobotRun,
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    users: &[UserSummary],
    robot_user: &UserSummary,
    finance_lock: &RobotFinanceMutationLock,
    now_at: NaiveDateTime,
) -> ApiResult<()> {
    let plan_id = robot_plan_id(robot, lottery, issue);
    let mut plan = match group_buys.get(&plan_id).await {
        Ok(existing) => existing,
        Err(ApiError::NotFound(_)) => {
            if is_issue_sale_closed(issue, now_at)? {
                push_skipped(
                    run,
                    robot,
                    &lottery.id,
                    Some(issue.issue.clone()),
                    "已到封盘时间，机器人不再发起自己的合买计划",
                );
                return Ok(());
            }
            let draft = build_robot_plan_request(robot, lottery, issue, orders, robot_user).await?;
            if let Some(entry) = ensure_robot_balance_locked(
                finance,
                finance_lock,
                ROBOT_GROUP_BUY_USER_ID,
                draft.total_amount_minor,
                "发起合买计划",
            )
            .await?
            {
                run.ledger_entries.push(entry);
            }
            finance
                .ensure_available(&robot_user.id, draft.total_amount_minor)
                .await?;
            let display_users = users_with_random_robot_display_name(users);
            let created = group_buys
                .create(draft.clone(), std::slice::from_ref(lottery), &display_users)
                .await?;
            let participant_id = format!("{}-P001", created.id);
            match debit_group_buy_locked(
                finance,
                finance_lock,
                &robot_user.id,
                draft.initiator_amount_minor,
                &participant_id,
                &created.id,
            )
            .await
            {
                Ok(entry) => {
                    run.ledger_entries.push(entry);
                    run.created_plans.push(created.clone());
                    created
                }
                Err(error) => {
                    if let Err(rollback_error) = group_buys.remove_unfunded_plan(&created.id).await
                    {
                        tracing::error!(
                            "合买计划ID" = %created.id,
                            error = %rollback_error.log_message(),
                            "合买机器人创建计划扣款失败后移除计划失败"
                        );
                    }
                    return Err(error);
                }
            }
        }
        Err(error) => return Err(error),
    };
    // 发单后立即用多个机器人用户认购一部分，避免合买大厅长时间只有发起人一个人
    if matches!(
        plan.status,
        GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open
    ) {
        let _ = seed_robot_plan_initial_participants(
            run,
            &mut plan,
            finance,
            group_buys,
            users,
            finance_lock,
        )
        .await;
    }

    match plan.status {
        GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open => {
            push_skipped(
                run,
                robot,
                &lottery.id,
                Some(issue.issue.clone()),
                "合买机器人仅负责发单，本期计划已存在，等待补单机器人认购",
            );
        }
        GroupBuyPlanStatus::Filled
        | GroupBuyPlanStatus::Settled
        | GroupBuyPlanStatus::Cancelled => {
            push_skipped(
                run,
                robot,
                &lottery.id,
                Some(issue.issue.clone()),
                "本期机器人合买计划已处理",
            );
        }
    }

    Ok(())
}

/// 补单机器人补满同彩种当前期所有未满单合买大厅计划。
async fn fill_existing_group_buy_plans(
    run: &mut GroupBuyRobotRun,
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    draws: &DrawRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    users: &[UserSummary],
    finance_lock: &RobotFinanceMutationLock,
    now_at: NaiveDateTime,
) -> ApiResult<()> {
    let candidate_plans = group_buys
        .list_details()
        .await?
        .into_iter()
        .filter(|plan| {
            plan.lottery_id == lottery.id
                && plan.issue == issue.issue
                && matches!(
                    plan.status,
                    GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open
                )
                && plan.filled_amount_minor < plan.total_amount_minor
        })
        .collect::<Vec<_>>();
    let filled_without_order_plans = group_buys
        .list_details()
        .await?
        .into_iter()
        .filter(|plan| {
            plan.lottery_id == lottery.id
                && plan.issue == issue.issue
                && plan.status == GroupBuyPlanStatus::Filled
                && plan.order_id.is_none()
                && plan.filled_amount_minor >= plan.total_amount_minor
        })
        .collect::<Vec<_>>();

    for mut plan in candidate_plans {
        let plan_id = plan.id.clone();
        if let Err(error) = fill_robot_plan(
            run,
            robot,
            lottery,
            issue,
            &mut plan,
            draws,
            orders,
            finance,
            group_buys,
            users,
            finance_lock,
            now_at,
            fill_robot_policy(robot),
            None,
        )
        .await
        {
            push_skipped(
                run,
                robot,
                &lottery.id,
                Some(issue.issue.clone()),
                format!("合买计划 {plan_id} 补满失败：{error}"),
            );
        }
    }
    for plan in filled_without_order_plans {
        if let Err(error) =
            attach_order_for_plan(run, lottery, &plan, draws, orders, group_buys).await
        {
            push_skipped(
                run,
                robot,
                &lottery.id,
                Some(issue.issue.clone()),
                format!("合买计划 {} 已满单补建订单失败：{error}", plan.id),
            );
        }
    }

    Ok(())
}

/// 补足机器人合买剩余金额，满单后生成真实投注订单。
async fn fill_robot_plan(
    run: &mut GroupBuyRobotRun,
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    plan: &mut GroupBuyPlan,
    draws: &DrawRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    users: &[UserSummary],
    finance_lock: &RobotFinanceMutationLock,
    now_at: NaiveDateTime,
    policy: RobotFillPolicy,
    before_draw_guard_now: Option<&str>,
) -> ApiResult<()> {
    let decision = match robot_fill_decision(plan, issue, now_at, policy)? {
        RobotFillDecision::Skip(reason) => {
            push_skipped(run, robot, &lottery.id, Some(issue.issue.clone()), reason);
            return Ok(());
        }
        RobotFillDecision::Add(decision) => decision,
    };
    if decision.amount_minor <= 0 {
        return Ok(());
    }
    let fill_amount_minor = decision.amount_minor;
    let fill_note = decision.note.clone();

    let min_share = plan.min_share_amount_minor.max(1);
    let participant_min = plan.participant_min_amount_minor.max(min_share).max(1);
    let split_seed = format!("{}:{}:{}", plan.id, issue.issue, fill_note);
    let split_amounts = robot_fill_amount_splits(
        fill_amount_minor,
        participant_min,
        min_share,
        policy,
        &split_seed,
    );

    let mut participant_ids = Vec::with_capacity(split_amounts.len());
    let mut rollback_participant_ids = Vec::new();
    for (index, &split_amount) in split_amounts.iter().enumerate() {
        let robot_user_id = ROBOT_GROUP_BUY_USER_IDS[index % ROBOT_GROUP_BUY_USER_IDS.len()];
        if let Some(entry) = ensure_robot_balance_locked(
            finance,
            finance_lock,
            robot_user_id,
            split_amount,
            "补单机器人认购合买",
        )
        .await?
        {
            run.ledger_entries.push(entry);
        }
        finance
            .ensure_available(robot_user_id, split_amount)
            .await?;
        let participant_id = next_robot_fill_participant_id(plan);
        participant_ids.push(participant_id.clone());
        let stage_note = format!("{} ({}/{})", fill_note, index + 1, split_amounts.len());
        match group_buys
            .add_participant(
                &plan.id,
                AddGroupBuyParticipantRequest {
                    id: participant_id.clone(),
                    user_id: robot_user_id.to_string(),
                    amount_minor: split_amount,
                    note: stage_note,
                },
                &users_with_random_robot_display_name(users),
            )
            .await
        {
            Ok(next) => {
                rollback_participant_ids.push(participant_id.clone());
                *plan = next;
            }
            Err(error) => {
                for rollback_id in rollback_participant_ids.iter().rev() {
                    if let Err(e) = group_buys
                        .remove_unfunded_participant(&plan.id, rollback_id)
                        .await
                    {
                        tracing::error!(
                            plan_id = %plan.id,
                            participant_id = %rollback_id,
                            error = %e.log_message(),
                            "补单机器人多用户分段回滚失败"
                        );
                    }
                }
                return Err(error);
            }
        };

        if matches!(plan.status, GroupBuyPlanStatus::Filled) {
            break;
        }
    }

    if !matches!(plan.status, GroupBuyPlanStatus::Filled) {
        for (index, &split_amount) in split_amounts.iter().enumerate() {
            if index >= participant_ids.len() {
                break;
            }
            let participant_id = &participant_ids[index];
            let robot_user_id = ROBOT_GROUP_BUY_USER_IDS[index % ROBOT_GROUP_BUY_USER_IDS.len()];
            match debit_group_buy_locked(
                finance,
                finance_lock,
                robot_user_id,
                split_amount,
                participant_id,
                &plan.id,
            )
            .await
            {
                Ok(entry) => {
                    run.ledger_entries.push(entry);
                }
                Err(error) => {
                    if let Err(rollback_error) = group_buys
                        .remove_unfunded_participant(&plan.id, participant_id)
                        .await
                    {
                        tracing::error!(
                            plan_id = %plan.id,
                            participant_id = %participant_id,
                            error = %rollback_error.log_message(),
                            "补单机器人扣款失败后移除分段参与记录失败"
                        );
                    }
                    return Err(error);
                }
            }
        }
        return Ok(());
    }

    let mut created_order = match if let Some(now) = before_draw_guard_now {
        create_order_for_filled_group_buy_before_draw_guard(
            draws, orders, group_buys, lottery, plan, now,
        )
        .await
    } else {
        create_order_for_filled_group_buy(draws, orders, group_buys, lottery, plan).await
    } {
        Ok(result) => result,
        Err(error) => {
            for participant_id in rollback_participant_ids.iter().rev() {
                if let Err(rollback_error) = group_buys
                    .remove_unfunded_participant(&plan.id, participant_id)
                    .await
                {
                    tracing::error!(
                        plan_id = %plan.id,
                        participant_id = %participant_id,
                        error = %rollback_error.log_message(),
                        "补单机器人满单成单失败后移除参与记录失败"
                    );
                }
            }
            return Err(error);
        }
    };

    if let Some((_, attached_plan)) = &created_order {
        *plan = attached_plan.clone();
    }

    for (index, &split_amount) in split_amounts.iter().enumerate() {
        if index >= participant_ids.len() {
            break;
        }
        let participant_id = &participant_ids[index];
        let robot_user_id = ROBOT_GROUP_BUY_USER_IDS[index % ROBOT_GROUP_BUY_USER_IDS.len()];
        match debit_group_buy_locked(
            finance,
            finance_lock,
            robot_user_id,
            split_amount,
            participant_id,
            &plan.id,
        )
        .await
        {
            Ok(entry) => run.ledger_entries.push(entry),
            Err(error) => {
                if let Some((order, _)) = created_order.take() {
                    if let Err(rollback_error) = orders.remove_unfunded(&order.id).await {
                        tracing::error!(
                            order_id = %order.id,
                            error = %rollback_error.log_message(),
                            "补单机器人扣款失败后移除满单订单失败"
                        );
                    }
                }
                for rollback_id in participant_ids.iter().rev() {
                    if let Err(rollback_error) = group_buys
                        .remove_unfunded_participant(&plan.id, rollback_id)
                        .await
                    {
                        tracing::error!(
                            plan_id = %plan.id,
                            participant_id = %rollback_id,
                            error = %rollback_error.log_message(),
                            "补单机器人扣款失败后移除参与记录失败"
                        );
                    }
                }
                return Err(error);
            }
        }
    }

    if let Some((order, attached_plan)) = created_order {
        draws.record_avoidance_order_risk(&order).await;
        run.created_orders.push(order);
        run.filled_plans.push(attached_plan);
    }

    Ok(())
}

/// 根据补单策略生成本轮认购金额拆分。
/// 开奖前补满策略只使用一个补单机器人吃掉剩余金额，阶段性补单仍保留多用户模拟。
fn robot_fill_amount_splits(
    fill_amount_minor: i64,
    participant_min: i64,
    min_share: i64,
    policy: RobotFillPolicy,
    split_seed: &str,
) -> Vec<i64> {
    if matches!(policy, RobotFillPolicy::BeforeDraw { .. }) {
        return vec![fill_amount_minor];
    }

    // 拆分份数受限于每份不得低于最低认购额，避免拆出低于最低认购额的零头。
    let max_users_by_min = ((fill_amount_minor / participant_min) as usize).max(1);
    let users_per_stage =
        robot_fill_users_per_stage(fill_amount_minor, participant_min).min(max_users_by_min);
    split_fill_amount_varied(
        fill_amount_minor,
        users_per_stage,
        participant_min,
        min_share,
        split_seed,
    )
}

/// 为历史遗留的已满单但未成单计划补建真实投注订单。
async fn attach_order_for_plan(
    run: &mut GroupBuyRobotRun,
    lottery: &LotteryKind,
    plan: &GroupBuyPlan,
    draws: &DrawRepository,
    orders: &OrderRepository,
    group_buys: &GroupBuyRepository,
) -> ApiResult<()> {
    if let Some((order, attached_plan)) =
        create_order_for_filled_group_buy(draws, orders, group_buys, lottery, plan).await?
    {
        draws.record_avoidance_order_risk(&order).await;
        run.created_orders.push(order);
        run.filled_plans.push(attached_plan);
    }
    Ok(())
}

/// 生成机器人计划请求，随机选择一个可验证的启用玩法、投注注数和隐含倍数。
async fn build_robot_plan_request(
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    orders: &OrderRepository,
    robot_user: &UserSummary,
) -> ApiResult<CreateGroupBuyPlanRequest> {
    let mut skipped_reasons = Vec::new();
    for (play_index, play) in randomized_robot_play_candidates(robot, lottery, issue) {
        let numbers = robot_numbers_for_play(robot, lottery, issue, play, play_index);
        let selection = match parse_group_buy_selection(&play.rule_code, &numbers) {
            Ok(selection) => selection,
            Err(error) => {
                skipped_reasons.push(error.to_string());
                continue;
            }
        };
        let quote = orders
            .quote(
                lottery,
                &CreateOrderRequest {
                    user_id: robot_user.id.clone(),
                    lottery_id: lottery.id.clone(),
                    issue: issue.issue.clone(),
                    rule_code: play.rule_code.clone(),
                    selection,
                    unit_amount_minor: 1,
                },
            )
            .await;
        let quote = match quote {
            Ok(quote) => quote,
            Err(error) => {
                skipped_reasons.push(error.to_string());
                continue;
            }
        };
        let unit_amount_minor =
            robot_unit_amount_minor(robot, lottery, issue, &play.rule_code, play_index)?;
        let (total_amount_minor, initiator_amount_minor) =
            robot_group_buy_amounts(lottery, i64::from(quote.stake_count), unit_amount_minor)?;
        let rule_code = enum_to_string(&play.rule_code)?;

        return Ok(CreateGroupBuyPlanRequest {
            id: robot_plan_id(robot, lottery, issue),
            lottery_id: lottery.id.clone(),
            issue: issue.issue.clone(),
            rule_code,
            title: format!("{} 第{}期合买", lottery.name, issue.issue),
            numbers,
            initiator_user_id: robot_user.id.clone(),
            total_amount_minor,
            initiator_amount_minor,
            note: format!("合买机器人自动发起：{}", robot.id),
        });
    }

    Err(ApiError::BadRequest(format!(
        "没有可用于机器人合买的启用玩法：{}",
        skipped_reasons.join("；")
    )))
}

/// 计算机器人合买总额和发起人自购金额。
fn robot_group_buy_amounts(
    lottery: &LotteryKind,
    stake_count: i64,
    unit_amount_minor: i64,
) -> ApiResult<(i64, i64)> {
    if stake_count <= 0 {
        return Err(ApiError::BadRequest("机器人投注注数无效".to_string()));
    }
    if unit_amount_minor <= 0 {
        return Err(ApiError::BadRequest("机器人投注倍数金额无效".to_string()));
    }

    let min_share = lottery.group_buy.min_share_amount_minor.max(1);
    let participant_min = lottery
        .group_buy
        .participant_min_amount_minor
        .max(min_share);
    let stake_unit = stake_count
        .checked_mul(ROBOT_BASE_UNIT_AMOUNT_MINOR)
        .ok_or_else(|| ApiError::BadRequest("机器人投注金额步进过大".to_string()))?;
    let total_unit = lcm(min_share, stake_unit)?;
    let target_total = stake_count
        .checked_mul(unit_amount_minor)
        .ok_or_else(|| ApiError::BadRequest("机器人合买金额过大".to_string()))?;
    let mut total = round_up_to_multiple(target_total.max(participant_min * 2), total_unit)?;
    total = total.max(round_up_to_multiple(min_share * 10, total_unit)?);
    total = total.max(round_up_to_multiple(
        participant_min
            .checked_mul(ROBOT_FILL_STAGE_COUNT)
            .ok_or_else(|| ApiError::BadRequest("机器人合买金额过大".to_string()))?,
        total_unit,
    )?);

    for _ in 0..8 {
        let required_by_percent =
            minimum_amount_by_percent(total, lottery.group_buy.initiator_min_percent)?;
        let initiator = round_up_to_multiple(required_by_percent.max(participant_min), min_share)?;
        let remaining = total
            .checked_sub(initiator)
            .ok_or_else(|| ApiError::BadRequest("机器人合买金额无效".to_string()))?;
        if remaining >= participant_min {
            return Ok((total, initiator));
        }
        total = round_up_to_multiple(
            initiator
                .checked_add(participant_min)
                .ok_or_else(|| ApiError::BadRequest("机器人合买金额过大".to_string()))?,
            total_unit,
        )?;
    }

    Err(ApiError::BadRequest(
        "机器人合买金额无法满足合买阈值".to_string(),
    ))
}

/// 选择当前仍可由机器人处理的最近一期。
async fn current_open_issue(
    draws: &DrawRepository,
    lottery: &LotteryKind,
    now: &str,
) -> ApiResult<Option<DrawIssue>> {
    // 选择当前可销售的 Open 期号：封盘时间还没到才算可销售。
    // 之前用 scheduled_at >= now 会漏掉已到开奖时间但尚未开奖的期号，
    // 导致机器人发单被跳过、合买大厅缺少机器人合买。
    Ok(draws
        .list_by_lottery_id(&lottery.id)
        .await?
        .into_iter()
        .filter(|issue| {
            issue.status == DrawIssueStatus::Open && issue.sale_closed_at.as_str() > now
        })
        .min_by(|left, right| {
            left.sale_closed_at
                .cmp(&right.sale_closed_at)
                .then(left.scheduled_at.cmp(&right.scheduled_at))
                .then(left.issue.cmp(&right.issue))
        }))
}

/// 按机器人、彩种和期号把启用玩法确定性打散，让发单玩法不再固定为第一个配置。
fn randomized_robot_play_candidates<'a>(
    robot: &RobotConfigSummary,
    lottery: &'a LotteryKind,
    issue: &DrawIssue,
) -> Vec<(usize, &'a LotteryPlayConfig)> {
    let mut candidates = lottery
        .play_configs
        .iter()
        .enumerate()
        .filter(|(_, play)| play.enabled)
        .collect::<Vec<_>>();
    let mut picker = RobotNumberPicker::for_purpose(robot, lottery, issue, "play-order");
    for index in 0..candidates.len() {
        let swap_index = index + picker.next_index(candidates.len() - index);
        candidates.swap(index, swap_index);
    }
    candidates
}

/// 按机器人、彩种、期号和玩法配置派生一组可校验的随机投注文本。
fn robot_numbers_for_play(
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    play: &LotteryPlayConfig,
    play_index: usize,
) -> String {
    let mut picker = RobotNumberPicker::new(robot, lottery, issue, &play.rule_code, play_index);
    robot_numbers_with_limits(&mut picker, &play.rule_code, &play.position_select_limits)
}

/// 按机器人、彩种、期号和玩法派生一组可校验的随机投注文本。
#[cfg(test)]
fn robot_numbers_for_rule(
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    rule_code: &PlayRuleCode,
    play_index: usize,
) -> String {
    let mut picker = RobotNumberPicker::new(robot, lottery, issue, rule_code, play_index);
    robot_numbers_with_limits(&mut picker, rule_code, &[])
}

/// 根据玩法和选号上限生成随机投注文本，改变选号数量即可改变真实注数。
fn robot_numbers_with_limits(
    picker: &mut RobotNumberPicker,
    rule_code: &PlayRuleCode,
    limits: &[LotteryPlayPositionSelectLimit],
) -> String {
    use PlayRuleCode::*;

    match rule_code {
        ThreeDirect | FiveFrontDirect | FiveMiddleDirect | FiveBackDirect => {
            direct_robot_numbers(picker, rule_code, limits)
        }
        FiveFrontDirectCombination | FiveMiddleDirectCombination | FiveBackDirectCombination => {
            let count =
                robot_digit_count(picker, limits, "numbers", 3, ROBOT_MAX_NUMBER_POOL_COUNT);
            join_digits(&picker.unique_digits(count))
        }
        ThreeGroupThree | FiveFrontGroupThree | FiveMiddleGroupThree | FiveBackGroupThree => {
            let count =
                robot_digit_count(picker, limits, "numbers", 2, ROBOT_MAX_NUMBER_POOL_COUNT);
            join_digits(&picker.unique_digits(count))
        }
        ThreeGroupThreeBanker
        | FiveFrontGroupThreeBanker
        | FiveMiddleGroupThreeBanker
        | FiveBackGroupThreeBanker => {
            let drag_count =
                robot_digit_count(picker, limits, "drag", 1, ROBOT_MAX_NUMBER_POOL_COUNT);
            let digits = picker.unique_digits(1 + drag_count);
            format!("{}|{}", digits[0], join_digits(&digits[1..]))
        }
        ThreeGroupSix | FiveFrontGroupSix | FiveMiddleGroupSix | FiveBackGroupSix => {
            let count =
                robot_digit_count(picker, limits, "numbers", 3, ROBOT_MAX_NUMBER_POOL_COUNT);
            join_digits(&picker.unique_digits(count))
        }
        ThreeGroupSixBanker
        | FiveFrontGroupSixBanker
        | FiveMiddleGroupSixBanker
        | FiveBackGroupSixBanker => group_six_banker_robot_numbers(picker, limits),
        FiveBigSmallOddEven => {
            let tens_count = robot_digit_count(
                picker,
                limits,
                "tens",
                1,
                ROBOT_MAX_BIG_SMALL_ATTRIBUTE_COUNT,
            );
            let ones_count = robot_digit_count(
                picker,
                limits,
                "ones",
                1,
                ROBOT_MAX_BIG_SMALL_ATTRIBUTE_COUNT,
            );
            format!(
                "tens:{}|ones:{}",
                join_texts(&picker.unique_attributes(tens_count)),
                join_texts(&picker.unique_attributes(ones_count))
            )
        }
    }
}

/// 生成直选三位置随机号码，位置数量受后台最大可选数限制。
fn direct_robot_numbers(
    picker: &mut RobotNumberPicker,
    rule_code: &PlayRuleCode,
    limits: &[LotteryPlayPositionSelectLimit],
) -> String {
    crate::services::play_rules::play_position_select_limit_targets(rule_code)
        .into_iter()
        .take(3)
        .map(|(key, _)| {
            let count = robot_digit_count(picker, limits, key, 1, ROBOT_MAX_POSITION_DIGIT_COUNT);
            join_digits(&picker.unique_digits(count))
        })
        .collect::<Vec<_>>()
        .join("|")
}

/// 生成组六胆拖投注，胆码 1-2 个、拖码数量随机，仍保证至少可以组成一注。
fn group_six_banker_robot_numbers(
    picker: &mut RobotNumberPicker,
    limits: &[LotteryPlayPositionSelectLimit],
) -> String {
    let banker_max = robot_max_count(limits, "banker", 2).clamp(1, 2);
    let banker_count = picker.next_inclusive(1, banker_max);
    let min_drag_count = 3usize.saturating_sub(banker_count).max(1);
    let drag_count = robot_digit_count(
        picker,
        limits,
        "drag",
        min_drag_count,
        ROBOT_MAX_NUMBER_POOL_COUNT,
    );
    let digits = picker.unique_digits(banker_count + drag_count);
    format!(
        "{}|{}",
        join_digits(&digits[..banker_count]),
        join_digits(&digits[banker_count..])
    )
}

/// 读取位置最大选号数量，未配置时使用机器人默认上限。
fn robot_max_count(
    limits: &[LotteryPlayPositionSelectLimit],
    position_key: &str,
    default_max: usize,
) -> usize {
    limits
        .iter()
        .find(|limit| limit.position_key == position_key)
        .map(|limit| limit.max_select_count as usize)
        .unwrap_or(default_max)
        .min(10)
}

/// 在最小可用数量和位置上限之间随机选择选号数量。
fn robot_digit_count(
    picker: &mut RobotNumberPicker,
    limits: &[LotteryPlayPositionSelectLimit],
    position_key: &str,
    min_count: usize,
    default_max: usize,
) -> usize {
    let max_count = robot_max_count(limits, position_key, default_max)
        .min(default_max)
        .max(min_count);
    picker.next_inclusive(min_count, max_count)
}

/// 为机器人计划随机一个隐含投注倍数，最终会体现在合买总额和成单单注金额上。
fn robot_unit_amount_minor(
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    rule_code: &PlayRuleCode,
    play_index: usize,
) -> ApiResult<i64> {
    let mut picker = RobotNumberPicker::for_rule_purpose(
        robot,
        lottery,
        issue,
        rule_code,
        play_index,
        "unit-amount",
    );
    let multiplier = picker.next_inclusive(1, ROBOT_MAX_MULTIPLE) as i64;
    ROBOT_BASE_UNIT_AMOUNT_MINOR
        .checked_mul(multiplier)
        .ok_or_else(|| ApiError::BadRequest("机器人投注倍数金额过大".to_string()))
}

/// 合买机器人选号器，按玩法生成合法随机投注内容。
struct RobotNumberPicker {
    state: u64,
}

/// 合买机器人选号器，按玩法生成合法随机投注内容。
impl RobotNumberPicker {
    /// 用业务上下文构造确定性随机种子，让同一计划可复现、不同期号会变化。
    fn new(
        robot: &RobotConfigSummary,
        lottery: &LotteryKind,
        issue: &DrawIssue,
        rule_code: &PlayRuleCode,
        play_index: usize,
    ) -> Self {
        let rule_text = match enum_to_string(rule_code) {
            Ok(value) => value,
            Err(_) => format!("{rule_code:?}"),
        };
        let play_index_text = play_index.to_string();
        Self::from_seed_parts(&[
            robot.id.as_str(),
            lottery.id.as_str(),
            issue.issue.as_str(),
            rule_text.as_str(),
            play_index_text.as_str(),
        ])
    }

    /// 为玩法之外的随机用途构造独立种子，例如玩法排序。
    fn for_purpose(
        robot: &RobotConfigSummary,
        lottery: &LotteryKind,
        issue: &DrawIssue,
        purpose: &str,
    ) -> Self {
        Self::from_seed_parts(&[
            robot.id.as_str(),
            lottery.id.as_str(),
            issue.issue.as_str(),
            purpose,
        ])
    }

    /// 为同一玩法下不同随机用途构造独立种子，例如隐含倍数。
    fn for_rule_purpose(
        robot: &RobotConfigSummary,
        lottery: &LotteryKind,
        issue: &DrawIssue,
        rule_code: &PlayRuleCode,
        play_index: usize,
        purpose: &str,
    ) -> Self {
        let rule_text = match enum_to_string(rule_code) {
            Ok(value) => value,
            Err(_) => format!("{rule_code:?}"),
        };
        let play_index_text = play_index.to_string();
        Self::from_seed_parts(&[
            robot.id.as_str(),
            lottery.id.as_str(),
            issue.issue.as_str(),
            rule_text.as_str(),
            play_index_text.as_str(),
            purpose,
        ])
    }

    /// 用一组字符串种子片段初始化确定性随机状态。
    fn from_seed_parts(parts: &[&str]) -> Self {
        let mut state = 0xcbf2_9ce4_8422_2325_u64;
        for part in parts {
            mix_seed_part(&mut state, part);
        }
        if state == 0 {
            state = 0x9e37_79b9_7f4a_7c15;
        }
        Self { state }
    }

    /// 从 0-9 中抽取不重复数字，保证组选和胆拖玩法可通过校验。
    fn unique_digits(&mut self, count: usize) -> Vec<u8> {
        let mut digits = (0_u8..=9).collect::<Vec<_>>();
        for index in 0..count.min(digits.len()) {
            let swap_index = index + self.next_index(digits.len() - index);
            digits.swap(index, swap_index);
        }
        digits.truncate(count.min(digits.len()));
        digits
    }

    /// 抽取不重复的大小单双属性，避免同一个位置重复选择同一个属性。
    fn unique_attributes(&mut self, count: usize) -> Vec<&'static str> {
        let mut attributes = ["big", "small", "odd", "even"].to_vec();
        for index in 0..count.min(attributes.len()) {
            let swap_index = index + self.next_index(attributes.len() - index);
            attributes.swap(index, swap_index);
        }
        attributes.truncate(count.min(attributes.len()));
        attributes
    }

    /// 返回指定闭区间里的随机整数。
    fn next_inclusive(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        min + self.next_index(max - min + 1)
    }

    /// 返回指定长度范围内的随机索引。
    fn next_index(&mut self, len: usize) -> usize {
        if len <= 1 {
            return 0;
        }
        (self.next_u64() % len as u64) as usize
    }

    /// 使用 xorshift64* 推进确定性随机状态。
    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
}

/// 把种子片段混入 FNV-1a 状态。
fn mix_seed_part(state: &mut u64, part: &str) {
    for byte in part.as_bytes() {
        *state ^= u64::from(*byte);
        *state = state.wrapping_mul(0x0000_0100_0000_01b3);
    }
    *state ^= 0xff;
    *state = state.wrapping_mul(0x0000_0100_0000_01b3);
}

/// 将数字列表格式化为合买解析器支持的逗号分隔文本。
fn join_digits(digits: &[u8]) -> String {
    digits
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

/// 将文本列表格式化为逗号分隔内容。
fn join_texts(values: &[&str]) -> String {
    values.join(",")
}

/// 定位机器人系统账号，避免用不存在的用户创建合买。
fn robot_user(users: &[UserSummary]) -> ApiResult<&UserSummary> {
    users
        .iter()
        .find(|user| user.id == ROBOT_GROUP_BUY_USER_ID)
        .or_else(|| {
            users
                .iter()
                .find(|user| user.username == ROBOT_GROUP_BUY_USERNAME)
        })
        .ok_or_else(|| ApiError::NotFound("合买机器人资金账号不存在".to_string()))
}

/// 复制用户快照并替换合买/补单机器人展示名，真实用户 ID 和资金账户保持不变。
fn users_with_random_robot_display_name(users: &[UserSummary]) -> Vec<UserSummary> {
    users
        .iter()
        .cloned()
        .map(|mut user| {
            if is_group_buy_robot_user_id(&user.id) {
                user.username = random_robot_display_name();
            }
            user
        })
        .collect()
}

/// 使用 random-zh 生成机器人对外展示的中文姓名，按“姓 + 名”组合。
fn random_robot_display_name() -> String {
    let surname = random_zh(RandomZhOptions {
        count: Some(1),
        level_range: Some((1, 1)),
        allow_duplicates: true,
        ..Default::default()
    })
    .into_iter()
    .next();
    let length_seed = random_zh(RandomZhOptions {
        count: Some(1),
        level_range: Some((1, 2)),
        allow_duplicates: true,
        ..Default::default()
    })
    .into_iter()
    .next();
    let firstname_len = match length_seed.map(|character| character as u32 % 100) {
        Some(0..=44) => 1,
        Some(45..=89) | None => 2,
        Some(_) => 3,
    };
    let firstname = random_zh(RandomZhOptions {
        count: Some(firstname_len),
        level_range: Some((1, 2)),
        allow_duplicates: true,
        ..Default::default()
    });
    let mut name = String::new();
    if let Some(surname) = surname {
        name.push(surname);
    }
    name.extend(firstname);

    if is_valid_robot_display_name(&name) {
        name
    } else {
        fallback_robot_display_name()
    }
}

/// 校验机器人展示名不能暴露机器人、会员或内部账号痕迹。
fn is_valid_robot_display_name(name: &str) -> bool {
    let char_count = name.chars().count();
    (2..=4).contains(&char_count)
        && !name.contains("会员")
        && !name.contains("机器人")
        && !name.to_ascii_lowercase().contains("agent")
        && name.chars().all(|character| {
            ('\u{4e00}'..='\u{9fff}').contains(&character)
                || ('\u{3400}'..='\u{4dbf}').contains(&character)
        })
}

/// 当随机库没有返回可用字符时使用固定中文姓名兜底，避免机器人流程失败。
fn fallback_robot_display_name() -> String {
    let index = current_robot_name_seed() % ROBOT_DISPLAY_NAME_FALLBACKS.len();
    ROBOT_DISPLAY_NAME_FALLBACKS[index].to_string()
}

/// 生成兜底名称索引，低频路径只用于随机库异常时。
fn current_robot_name_seed() -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos() as usize)
        .unwrap_or(0)
}

/// 机器人账户余额不足时自动授信补足，并返回授信流水供后台实时事件广播。
async fn ensure_robot_balance(
    finance: &FinanceRepository,
    user_id: &str,
    required_amount_minor: i64,
    reason: &str,
) -> ApiResult<Option<LedgerEntry>> {
    if required_amount_minor <= 0 {
        return Err(ApiError::BadRequest("机器人授信金额必须大于 0".to_string()));
    }

    let account = finance.account_or_create(user_id).await?;
    if account.available_balance_minor >= required_amount_minor {
        return Ok(None);
    }

    let target_balance_minor = required_amount_minor
        .checked_add(ROBOT_AUTO_CREDIT_RESERVE_MINOR)
        .ok_or_else(|| ApiError::BadRequest("机器人授信金额过大".to_string()))?;
    let top_up_amount_minor = target_balance_minor
        .checked_sub(account.available_balance_minor)
        .ok_or_else(|| ApiError::BadRequest("机器人授信金额无效".to_string()))?;
    let entry = finance
        .manual_adjust(ManualBalanceAdjustmentRequest {
            user_id: user_id.to_string(),
            amount_minor: top_up_amount_minor,
            description: format!("机器人账户自动授信补余额：{reason}"),
        })
        .await?;

    Ok(Some(entry))
}

/// 在机器人并发执行时串行化自动授信，避免多个资金快照异步落库互相覆盖。
async fn ensure_robot_balance_locked(
    finance: &FinanceRepository,
    finance_lock: &RobotFinanceMutationLock,
    user_id: &str,
    required_amount_minor: i64,
    reason: &str,
) -> ApiResult<Option<LedgerEntry>> {
    let _guard = finance_lock.lock().await;
    ensure_robot_balance(finance, user_id, required_amount_minor, reason).await
}

/// 在机器人并发执行时串行化合买扣款，保证余额和资金流水持久化顺序稳定。
async fn debit_group_buy_locked(
    finance: &FinanceRepository,
    finance_lock: &RobotFinanceMutationLock,
    user_id: &str,
    amount_minor: i64,
    participant_id: &str,
    plan_id: &str,
) -> ApiResult<LedgerEntry> {
    let _guard = finance_lock.lock().await;
    finance
        .debit_group_buy(user_id, amount_minor, participant_id, plan_id)
        .await
}

/// 生成同一机器人、彩种、期号的确定性合买计划 ID。
fn robot_plan_id(robot: &RobotConfigSummary, lottery: &LotteryKind, issue: &DrawIssue) -> String {
    format!(
        "G-ROBOT-{}-{}-{}",
        slug_fragment(&robot.id),
        slug_fragment(&lottery.id),
        slug_fragment(&issue.issue)
    )
}

/// 根据补单金额和最低参与金额决定本阶段应拆分为几个机器人用户。
fn robot_fill_users_per_stage(fill_amount_minor: i64, participant_min: i64) -> usize {
    if participant_min <= 0 {
        return ROBOT_FILL_USERS_PER_STAGE_MAX;
    }
    let max_by_min = (fill_amount_minor / participant_min) as usize;
    let users = if max_by_min < ROBOT_FILL_USERS_PER_STAGE_MIN {
        ROBOT_FILL_USERS_PER_STAGE_MIN.min(max_by_min.max(1))
    } else {
        max_by_min.min(ROBOT_FILL_USERS_PER_STAGE_MAX)
    };
    if users < ROBOT_FILL_USERS_PER_STAGE_MIN && fill_amount_minor > 0 {
        tracing::info!(
            fill_amount = fill_amount_minor,
            users,
            participant_min,
            "补单阶段金额较小，使用最大可拆分用户数"
        );
    }
    users
}

/// 将补单金额均匀拆分为多份，每份不低于参与最低金额且整除最小份额。
/// 以 min_share 为最小单位精确拆分，保证总和等于原始金额。
/// 当拆分金额低于参与最低金额时，退回最大可拆份数而非退回单份。
fn split_fill_amount_evenly(
    total: i64,
    users: usize,
    participant_min: i64,
    min_share: i64,
) -> Vec<i64> {
    let min_unit = min_share.max(1);
    // 入参先向下对齐到最低份额整数倍，避免拆分出无法整除的零头。
    let total = total / min_unit * min_unit;
    if users <= 1 || total <= 0 {
        return vec![total];
    }
    let total_units = total / min_unit;
    let base_units = total_units / users as i64;
    let remainder_units = total_units - base_units * users as i64;
    let mut amounts = Vec::with_capacity(users);
    for i in 0..users {
        let units = if (i as i64) < remainder_units {
            base_units + 1
        } else {
            base_units
        };
        let amount = units * min_unit;
        if amount < participant_min {
            let max_users = ((total / participant_min) as usize).max(1);
            if max_users <= 1 || max_users >= users {
                return vec![total];
            }
            tracing::info!(
                total,
                users,
                participant_min,
                max_users,
                "补单金额拆分后低于最低认购额，降为 {max_users} 份"
            );
            return split_fill_amount_evenly(total, max_users, participant_min, min_share);
        }
        amounts.push(amount);
    }
    amounts
}

/// 将补单金额按稳定随机权重拆成多份，避免阶段性补单每个机器人认购金额完全一致。
fn split_fill_amount_varied(
    total: i64,
    users: usize,
    participant_min: i64,
    min_share: i64,
    seed: &str,
) -> Vec<i64> {
    let min_unit = min_share.max(1);
    let total = total / min_unit * min_unit;
    if users <= 1 || total <= 0 {
        return vec![total];
    }

    let min_units = (participant_min.max(1) + min_unit - 1) / min_unit;
    let total_units = total / min_unit;
    let required_units = min_units.saturating_mul(users as i64);
    if required_units > total_units {
        let max_users = (total_units / min_units).max(1) as usize;
        if max_users <= 1 || max_users >= users {
            return vec![total];
        }
        tracing::info!(
            total,
            users,
            participant_min,
            max_users,
            "补单金额随机拆分后低于最低认购额，降为 {max_users} 份"
        );
        return split_fill_amount_varied(total, max_users, participant_min, min_share, seed);
    }

    let remaining_units = total_units - required_units;
    if remaining_units <= 0 {
        return vec![min_units * min_unit; users];
    }

    let weights = (0..users)
        .map(|index| {
            let hash = stable_robot_fill_hash(seed, "split", index as u64);
            (hash % 97 + 3) as i64
        })
        .collect::<Vec<_>>();
    let total_weight = weights.iter().sum::<i64>().max(1);
    let mut extra_units = vec![0_i64; users];
    let mut allocated_units = 0_i64;
    for (index, weight) in weights.iter().enumerate() {
        let units = remaining_units * *weight / total_weight;
        extra_units[index] = units;
        allocated_units += units;
    }

    let mut order = (0..users).collect::<Vec<_>>();
    order.sort_by_key(|index| stable_robot_fill_hash(seed, "split-remainder", *index as u64));
    for index in order
        .into_iter()
        .take((remaining_units - allocated_units) as usize)
    {
        extra_units[index] += 1;
    }

    extra_units
        .into_iter()
        .map(|units| (min_units + units) * min_unit)
        .collect()
}

/// 生成机器人补满参与记录 ID。
fn next_robot_fill_participant_id(plan: &GroupBuyPlan) -> String {
    // 用已有参与记录数量作为基数，加上当前时间戳毫秒后缀生成唯一 ID，
    // 避免跨轮次内存快照不一致导致 ID 冲突。
    let base = plan.participants.len() as u64;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let sequence = (millis % 100_000) as u64;
    let mut counter = 0_u64;
    loop {
        let participant_id = format!(
            "{}-{}-{:05}{:03}",
            plan.id,
            ROBOT_FILL_PARTICIPANT_SUFFIX,
            sequence,
            base + counter
        );
        if !plan
            .participants
            .iter()
            .any(|participant| participant.id == participant_id)
        {
            return participant_id;
        }
        counter += 1;
    }
}

/// 将外部标识收敛为适合作为计划 ID 的片段。
fn slug_fragment(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_uppercase());
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }
    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        "UNKNOWN".to_string()
    } else {
        output
    }
}

/// 修剪本轮执行时间。
fn required_now(now: String) -> ApiResult<String> {
    let now = now.trim().to_string();
    if now.is_empty() {
        return Err(ApiError::BadRequest("机器人执行时间不能为空".to_string()));
    }
    Ok(now)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RobotFillDecision {
    Skip(String),
    Add(RobotFillAmount),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RobotFillAmount {
    amount_minor: i64,
    note: String,
}

/// 按动态阶段目标计算本轮机器人应补金额。
fn robot_fill_decision(
    plan: &GroupBuyPlan,
    issue: &DrawIssue,
    now_at: NaiveDateTime,
    policy: RobotFillPolicy,
) -> ApiResult<RobotFillDecision> {
    let remaining_amount_minor = plan
        .total_amount_minor
        .checked_sub(plan.filled_amount_minor)
        .ok_or_else(|| ApiError::BadRequest("合买剩余金额无效".to_string()))?;
    if remaining_amount_minor <= 0 {
        return Ok(RobotFillDecision::Skip("合买计划已满单".to_string()));
    }

    let scheduled_at = parse_robot_timestamp(&issue.scheduled_at, "开奖时间")?;
    let seconds_until_draw = (scheduled_at - now_at).num_seconds();
    if seconds_until_draw < 0 {
        return Ok(RobotFillDecision::Skip(
            "已到开奖时间，机器人不再补单".to_string(),
        ));
    }

    if let RobotFillPolicy::BeforeDraw {
        fill_before_draw_seconds,
    } = policy
    {
        if seconds_until_draw > fill_before_draw_seconds {
            return Ok(RobotFillDecision::Skip(format!(
                "未到开奖前补满窗口，距离开奖还有 {seconds_until_draw} 秒"
            )));
        }
        return Ok(RobotFillDecision::Add(RobotFillAmount {
            amount_minor: remaining_amount_minor,
            note: format!("补单机器人开奖前 {fill_before_draw_seconds} 秒补满"),
        }));
    }

    if let RobotFillPolicy::Rhythm {
        fill_before_draw_seconds,
        ..
    } = policy
    {
        if seconds_until_draw <= fill_before_draw_seconds {
            return Ok(RobotFillDecision::Add(RobotFillAmount {
                amount_minor: remaining_amount_minor,
                note: format!("补单机器人开奖前 {fill_before_draw_seconds} 秒最终补满"),
            }));
        }
    }

    let sale_closed_at = parse_robot_timestamp(&issue.sale_closed_at, "封盘时间")?;
    let seconds_until_sale_close = (sale_closed_at - now_at).num_seconds();
    if seconds_until_sale_close <= 0 {
        return match policy {
            RobotFillPolicy::GuaranteedUserPlan => Ok(RobotFillDecision::Add(RobotFillAmount {
                amount_minor: remaining_amount_minor,
                note: "补单机器人封盘兜底补满合买".to_string(),
            })),
            RobotFillPolicy::Rhythm { .. } => Ok(RobotFillDecision::Skip(
                "已到封盘时间且未到开奖前最终补满窗口，阶段性补单暂不追加认购".to_string(),
            )),
            RobotFillPolicy::BeforeDraw { .. } => unreachable!("开奖前补满策略已提前处理"),
        };
    }

    let RobotFillPolicy::Rhythm {
        max_percent,
        stage_count,
        fill_before_draw_seconds,
    } = policy
    else {
        return Err(ApiError::Internal("补单机器人策略无效".to_string()));
    };
    let Some((stage, stage_label)) = robot_fill_stage(
        plan,
        &issue.created_at,
        scheduled_at,
        now_at,
        fill_before_draw_seconds,
        stage_count,
    )?
    else {
        return Ok(RobotFillDecision::Skip(
            "未到阶段性补单起始时间，等待下一阶段".to_string(),
        ));
    };
    if stage != RobotFillStage::Final && robot_plan_has_rhythm_stage_fill(plan, &stage_label) {
        return Ok(RobotFillDecision::Skip(format!(
            "补单机器人{stage_label}节奏补单已执行，等待下一阶段"
        )));
    }

    let amount_minor = if stage == RobotFillStage::Final {
        remaining_amount_minor
    } else {
        rhythm_stage_fill_amount(plan, stage, max_percent, remaining_amount_minor)?
    };
    if stage != RobotFillStage::Final && amount_minor >= remaining_amount_minor {
        return Ok(RobotFillDecision::Skip(
            "本轮补单会直接满单，等待最终阶段补满".to_string(),
        ));
    }
    if amount_minor <= 0 {
        return Ok(RobotFillDecision::Skip(
            "本轮机器人补单金额小于最小份额，等待下一阶段".to_string(),
        ));
    }

    let participant_min = plan
        .participant_min_amount_minor
        .max(plan.min_share_amount_minor)
        .max(1);
    if amount_minor < participant_min && amount_minor != remaining_amount_minor {
        return Ok(RobotFillDecision::Skip(format!(
            "本轮机器人补单金额低于参与最低金额 {participant_min}，等待下一阶段"
        )));
    }

    let remaining_after = remaining_amount_minor
        .checked_sub(amount_minor)
        .ok_or_else(|| ApiError::BadRequest("机器人补单后剩余金额无效".to_string()))?;
    if stage != RobotFillStage::Final && remaining_after > 0 && remaining_after < participant_min {
        return Ok(RobotFillDecision::Skip(
            "本轮补单会导致剩余金额低于参与最低金额，等待最终阶段".to_string(),
        ));
    }

    Ok(RobotFillDecision::Add(RobotFillAmount {
        amount_minor,
        note: format!("补单机器人{stage_label}节奏补单"),
    }))
}

/// 计算补单机器人的补满策略，默认保留封盘兜底能力。
fn fill_robot_policy(robot: &RobotConfigSummary) -> RobotFillPolicy {
    match robot.group_buy_fill_strategy {
        GroupBuyRobotFillStrategy::Rhythm => RobotFillPolicy::Rhythm {
            max_percent: robot.group_buy_rhythm_fill_max_percent,
            stage_count: robot.group_buy_rhythm_stage_count,
            fill_before_draw_seconds: i64::from(robot.group_buy_fill_before_draw_seconds),
        },
        GroupBuyRobotFillStrategy::BeforeDraw => RobotFillPolicy::BeforeDraw {
            fill_before_draw_seconds: i64::from(robot.group_buy_fill_before_draw_seconds),
        },
    }
}

/// 判断用户合买计划是否需要在流单退款前由机器人强制补满。
fn user_group_buy_plan_needs_guard_fill(plan: &GroupBuyPlan) -> bool {
    !plan.id.starts_with("G-ROBOT-")
        && !is_group_buy_robot_user_id(&plan.initiator_user_id)
        && matches!(
            plan.status,
            GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open
        )
        && plan.filled_amount_minor < plan.total_amount_minor
}

/// 判断用户合买计划是否已经满额但缺少真实投注订单，需要在开奖前补建订单。
fn user_group_buy_plan_needs_guard_order(plan: &GroupBuyPlan) -> bool {
    !plan.id.starts_with("G-ROBOT-")
        && !is_group_buy_robot_user_id(&plan.initiator_user_id)
        && plan.status == GroupBuyPlanStatus::Filled
        && plan.order_id.is_none()
        && plan.filled_amount_minor >= plan.total_amount_minor
}

/// 判断当前执行时间是否已经到达期号封盘点。
fn is_issue_sale_closed(issue: &DrawIssue, now_at: NaiveDateTime) -> ApiResult<bool> {
    Ok(now_at >= parse_robot_timestamp(&issue.sale_closed_at, "封盘时间")?)
}

/// 根据合买创建时间和开奖前最终补满点动态返回当前阶段。
fn robot_fill_stage(
    plan: &GroupBuyPlan,
    issue_created_at: &str,
    scheduled_at: NaiveDateTime,
    now_at: NaiveDateTime,
    fill_before_draw_seconds: i64,
    stage_count: u32,
) -> ApiResult<Option<(RobotFillStage, String)>> {
    let plan_created_at = parse_robot_timestamp(&plan.created_at, "合买创建时间")?;
    let issue_created_at =
        NaiveDateTime::parse_from_str(issue_created_at.trim(), TIMESTAMP_FORMAT).ok();
    let created_at = if plan_created_at <= now_at {
        plan_created_at
    } else if let Some(issue_created_at) = issue_created_at.filter(|value| *value <= now_at) {
        issue_created_at
    } else {
        scheduled_at - chrono::Duration::seconds(300)
    };

    let final_fill_at = scheduled_at - chrono::Duration::seconds(fill_before_draw_seconds);
    if now_at >= final_fill_at {
        return Ok(Some((RobotFillStage::Final, "开奖前最终".to_string())));
    }

    let stage_seconds = (final_fill_at - created_at).num_seconds();
    if stage_seconds <= 0 {
        return Ok(Some((RobotFillStage::Final, "开奖前最终".to_string())));
    }

    let elapsed_seconds = (now_at - created_at)
        .num_seconds()
        .max(0)
        .min(stage_seconds - 1);
    let stage_count_i64 = i64::from(stage_count.max(1));
    let index =
        ((elapsed_seconds * stage_count_i64) / stage_seconds + 1).min(stage_count_i64) as u32;

    Ok(Some((
        RobotFillStage::Stage { index },
        format!("第{index}阶段"),
    )))
}

/// 使用 FNV-1a 派生稳定散列，避免机器人节奏在调度重试时抖动。
fn stable_robot_fill_hash(seed: &str, salt: &str, number: u64) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in seed
        .as_bytes()
        .iter()
        .chain(salt.as_bytes())
        .chain(&number.to_le_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// 计算阶段性补单单阶段金额：按后台上限随机百分比，不直接补满。
fn rhythm_stage_fill_amount(
    plan: &GroupBuyPlan,
    stage: RobotFillStage,
    max_percent: u32,
    remaining_amount_minor: i64,
) -> ApiResult<i64> {
    let min_share = plan.min_share_amount_minor.max(1);
    let participant_min = plan.participant_min_amount_minor.max(min_share).max(1);
    let percent = rhythm_stage_fill_percent(plan, stage, max_percent, participant_min);
    let raw = plan
        .total_amount_minor
        .checked_mul(i64::from(percent))
        .ok_or_else(|| ApiError::BadRequest("机器人阶段补单金额过大".to_string()))?
        / 100;
    let max_amount = round_down_to_multiple(raw, min_share).min(remaining_amount_minor);
    if max_amount < participant_min {
        return Ok(0);
    }
    Ok(max_amount)
}

/// 计算阶段性补单单阶段上限金额，按合买总金额和最小份额向下取整。
#[cfg(test)]
fn rhythm_stage_fill_cap(plan: &GroupBuyPlan, max_percent: u32) -> ApiResult<i64> {
    let raw = plan
        .total_amount_minor
        .checked_mul(i64::from(max_percent))
        .ok_or_else(|| ApiError::BadRequest("机器人阶段补单上限金额过大".to_string()))?
        / 100;
    Ok(round_down_to_multiple(
        raw,
        plan.min_share_amount_minor.max(1),
    ))
}

/// 返回阶段散列盐值。
fn rhythm_stage_fill_percent(
    plan: &GroupBuyPlan,
    stage: RobotFillStage,
    max_percent: u32,
    participant_min: i64,
) -> u32 {
    let minimum_percent = if plan.total_amount_minor <= 0 {
        1
    } else {
        ((participant_min * 100 + plan.total_amount_minor - 1) / plan.total_amount_minor).max(1)
            as u32
    };
    let min_percent = minimum_percent.min(max_percent).max(1);
    if min_percent >= max_percent {
        return max_percent;
    }

    let span = max_percent - min_percent + 1;
    let salt = robot_fill_stage_salt(stage);
    min_percent
        + (stable_robot_fill_hash(
            &plan.id,
            &salt,
            plan.total_amount_minor as u64 + u64::from(max_percent),
        ) % u64::from(span)) as u32
}

/// 返回阶段散列盐值。
fn robot_fill_stage_salt(stage: RobotFillStage) -> String {
    match stage {
        RobotFillStage::Stage { index } => format!("stage-{index}"),
        RobotFillStage::Final => "stage-final".to_string(),
    }
}

/// 判断某个阶段是否已写入过机器人补单参与记录，避免调度重复执行同一阶段。
fn robot_plan_has_rhythm_stage_fill(plan: &GroupBuyPlan, stage_label: &str) -> bool {
    let expected_note = format!("补单机器人{stage_label}节奏补单");
    plan.participants
        .iter()
        .any(|participant| participant.note.starts_with(&expected_note))
}

/// 解析机器人使用的时间字符串。
fn parse_robot_timestamp(value: &str, label: &str) -> ApiResult<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value.trim(), TIMESTAMP_FORMAT)
        .map_err(|_| ApiError::BadRequest(format!("{label}格式无效")))
}

/// 根据百分比计算最低自购金额。
fn minimum_amount_by_percent(total: i64, percent: u8) -> ApiResult<i64> {
    let product = total
        .checked_mul(i64::from(percent))
        .ok_or_else(|| ApiError::BadRequest("机器人合买金额过大".to_string()))?;
    Ok((product + 99) / 100)
}

/// 计算两个正整数的最小公倍数。
fn lcm(left: i64, right: i64) -> ApiResult<i64> {
    if left <= 0 || right <= 0 {
        return Err(ApiError::BadRequest("机器人金额步进无效".to_string()));
    }
    left.checked_div(gcd(left, right))
        .and_then(|value| value.checked_mul(right))
        .ok_or_else(|| ApiError::BadRequest("机器人金额步进过大".to_string()))
}

/// 计算两个正整数的最大公约数。
fn gcd(mut left: i64, mut right: i64) -> i64 {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left.abs()
}

/// 按指定倍数向上取整。
fn round_up_to_multiple(value: i64, multiple: i64) -> ApiResult<i64> {
    if value <= 0 || multiple <= 0 {
        return Err(ApiError::BadRequest("机器人金额必须大于 0".to_string()));
    }
    let remainder = value % multiple;
    if remainder == 0 {
        return Ok(value);
    }
    value
        .checked_add(multiple - remainder)
        .ok_or_else(|| ApiError::BadRequest("机器人金额过大".to_string()))
}

/// 按指定倍数向下取整。
fn round_down_to_multiple(value: i64, multiple: i64) -> i64 {
    if value <= 0 || multiple <= 0 {
        return 0;
    }
    value - (value % multiple)
}

/// 记录跳过原因，便于后台定位机器人为什么没有执行。
fn push_skipped(
    run: &mut GroupBuyRobotRun,
    robot: &RobotConfigSummary,
    lottery_id: &str,
    issue: Option<String>,
    reason: impl Into<String>,
) {
    run.skipped_items.push(GroupBuyRobotSkippedItem {
        robot_id: robot.id.clone(),
        robot_name: robot.name.clone(),
        lottery_id: lottery_id.to_string(),
        issue,
        reason: reason.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            draw::CreateDrawIssueRequest,
            finance::{LedgerEntryKind, ManualBalanceAdjustmentRequest},
            lottery::{DrawSchedule, GroupBuyConfig, LotteryNumberType, PlayCategory},
            order::OrderSource,
        },
        services::{
            draw::DrawRepository,
            finance::FinanceRepository,
            group_buy_flow::parse_group_buy_rule_code,
            lottery::LotteryRepository,
            play_rules::{expanded_bets_for_rule, number_type_for_rule},
            robot::RobotRepository,
        },
    };
    /// 验证机器人号码匹配everysupported玩法玩法。
    #[test]
    fn robot_numbers_match_every_supported_play_rule() {
        let robot = robot_test_config();
        let lottery = robot_test_lottery();
        let issue = robot_test_issue("20260605200000");

        for (index, rule_code) in robot_supported_play_rules().into_iter().enumerate() {
            let numbers = robot_numbers_for_rule(&robot, &lottery, &issue, &rule_code, index);
            let selection =
                parse_group_buy_selection(&rule_code, &numbers).expect("机器人选号必须可解析");
            let expanded =
                expanded_bets_for_rule(&rule_code, &selection).expect("机器人选号必须可展开");

            assert!(
                !expanded.is_empty(),
                "玩法 {:?} 生成的投注内容应至少展开一注",
                rule_code
            );
        }
    }
    /// 验证机器人直选号码vary跨越期号。
    #[test]
    fn robot_direct_numbers_vary_across_issues() {
        let robot = robot_test_config();
        let lottery = robot_test_lottery();
        let numbers = (0..8)
            .map(|index| {
                let issue = robot_test_issue(&format!("2026060520000{index}"));
                robot_numbers_for_rule(&robot, &lottery, &issue, &PlayRuleCode::FiveFrontDirect, 0)
            })
            .collect::<std::collections::BTreeSet<_>>();

        assert!(numbers.len() > 1, "机器人直选投注内容需要随期号随机变化");
        assert!(
            numbers.iter().any(|numbers| numbers != "1|2|3"),
            "机器人直选投注内容不能总是固定的 1|2|3"
        );
    }

    /// 验证合买机器人发单时玩法、注数和隐含倍数都会随期号随机变化。
    #[tokio::test]
    async fn robot_plan_request_randomizes_play_rule_stake_count_and_multiplier() {
        let robot = robot_test_config();
        let mut lottery = robot_test_lottery();
        let lottery_number_type = lottery.number_type.clone();
        lottery.play_configs = robot_supported_play_rules()
            .into_iter()
            .filter(|rule_code| number_type_for_rule(rule_code) == lottery_number_type)
            .map(|rule_code| crate::domain::lottery::LotteryPlayConfig {
                rule_code,
                enabled: true,
                odds_basis_points: 95_000,
                position_select_limits: Vec::new(),
            })
            .collect();
        let orders = OrderRepository::memory();
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let robot_user = robot_user(&access.users)
            .expect("robot user exists")
            .clone();
        let mut rule_codes = std::collections::BTreeSet::new();
        let mut stake_counts = std::collections::BTreeSet::new();
        let mut multiples = std::collections::BTreeSet::new();

        for index in 0..40 {
            let issue = robot_test_issue(&format!("2026060521{index:04}"));
            let request = build_robot_plan_request(&robot, &lottery, &issue, &orders, &robot_user)
                .await
                .expect("robot request can build");
            let rule_code =
                parse_group_buy_rule_code(&request.rule_code).expect("rule code can parse");
            let selection = parse_group_buy_selection(&rule_code, &request.numbers)
                .expect("selection can parse");
            let quote = orders
                .quote(
                    &lottery,
                    &CreateOrderRequest {
                        user_id: robot_user.id.clone(),
                        lottery_id: lottery.id.clone(),
                        issue: issue.issue,
                        rule_code,
                        selection,
                        unit_amount_minor: 1,
                    },
                )
                .await
                .expect("robot selection can quote");
            let unit_amount_minor = request.total_amount_minor / i64::from(quote.stake_count);

            rule_codes.insert(request.rule_code);
            stake_counts.insert(quote.stake_count);
            multiples.insert(unit_amount_minor / ROBOT_BASE_UNIT_AMOUNT_MINOR);
            assert_eq!(unit_amount_minor % ROBOT_BASE_UNIT_AMOUNT_MINOR, 0);
        }

        assert!(rule_codes.len() > 1, "机器人发单玩法不能固定不变");
        assert!(stake_counts.len() > 1, "机器人投注注数不能固定不变");
        assert!(multiples.len() > 1, "机器人投注倍数不能固定不变");
    }

    /// 验证机器人展示名使用中文姓名且不暴露内部机器人痕迹。
    #[test]
    fn robot_display_name_uses_random_chinese_name() {
        let name = random_robot_display_name();

        assert!(is_valid_robot_display_name(&name));
        assert!(!name.contains("会员"));
        assert!(!name.contains("机器人"));
        assert!(!name.to_ascii_lowercase().contains("agent"));
    }

    /// 验证替换用户快照时只改变机器人展示名，不改变机器人用户 ID。
    #[test]
    fn robot_display_users_keep_robot_id_and_replace_username() {
        let users = vec![
            UserSummary {
                id: "U10001".to_string(),
                username: "真实用户".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: crate::domain::user::UserKind::Regular,
                status: crate::domain::user::UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: "ABCD1234".to_string(),
                registration_location: crate::domain::user::UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            },
            UserSummary {
                id: ROBOT_GROUP_BUY_USER_ID.to_string(),
                username: ROBOT_GROUP_BUY_USERNAME.to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: crate::domain::user::UserKind::Agent,
                status: crate::domain::user::UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: "ROBOT001".to_string(),
                registration_location: crate::domain::user::UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            },
            UserSummary {
                id: "X90002".to_string(),
                username: "robot_fill_02".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: crate::domain::user::UserKind::Regular,
                status: crate::domain::user::UserStatus::Active,
                balance_minor: 0,
                agent_id: Some(ROBOT_GROUP_BUY_USER_ID.to_string()),
                invite_code: "ROBOT-X90002".to_string(),
                registration_location: crate::domain::user::UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            },
        ];

        let display_users = users_with_random_robot_display_name(&users);
        let robot = display_users
            .iter()
            .find(|user| user.id == ROBOT_GROUP_BUY_USER_ID)
            .expect("robot user exists");
        let fill_robot = display_users
            .iter()
            .find(|user| user.id == "X90002")
            .expect("fill robot user exists");
        let real_user = display_users
            .iter()
            .find(|user| user.id == "U10001")
            .expect("real user exists");

        assert_eq!(robot.id, ROBOT_GROUP_BUY_USER_ID);
        assert_ne!(robot.username, ROBOT_GROUP_BUY_USERNAME);
        assert!(is_valid_robot_display_name(&robot.username));
        assert_eq!(fill_robot.id, "X90002");
        assert_ne!(fill_robot.username, "robot_fill_02");
        assert!(is_valid_robot_display_name(&fill_robot.username));
        assert_eq!(real_user.username, "真实用户");
    }

    /// 验证拆分金额每份都是最低份额整数倍且不低于最低认购额。
    #[test]
    fn robot_split_amounts_align_to_min_share_and_participant_min() {
        // remaining=1400, min_share=100, participant_min=500：
        // 1400/3=466 向下对齐到 400，再 .max(500) => seed_target=500，
        // 只能拆 1 份（500/500=1），每份整除 100 且 >= 500。
        let min_share = 100;
        let participant_min = 500;
        let seed_target = ((1400 / 3) / min_share * min_share).max(participant_min);
        assert_eq!(seed_target, 500);
        let max_users = ((seed_target / participant_min) as usize).max(1);
        assert_eq!(max_users, 1);
        let users = robot_fill_users_per_stage(seed_target, participant_min).min(max_users);
        let amounts = split_fill_amount_evenly(seed_target, users, participant_min, min_share);
        for &amount in &amounts {
            assert_eq!(amount % min_share, 0, "每份必须整除最低份额");
            assert!(amount >= participant_min, "每份必须不低于最低认购额");
        }

        // remaining=3000, min_share=100, participant_min=500：
        // seed_target=1000，可拆 2 份，每份 500，整除且 >= participant_min。
        let seed_target_2 = ((3000 / 3) / min_share * min_share).max(participant_min);
        assert_eq!(seed_target_2, 1000);
        let max_users_2 = ((seed_target_2 / participant_min) as usize).max(1);
        let users_2 = robot_fill_users_per_stage(seed_target_2, participant_min).min(max_users_2);
        let amounts_2 =
            split_fill_amount_evenly(seed_target_2, users_2, participant_min, min_share);
        assert_eq!(amounts_2.iter().sum::<i64>(), seed_target_2);
        for &amount in &amounts_2 {
            assert_eq!(amount % min_share, 0);
            assert!(amount >= participant_min);
        }
    }

    /// 验证阶段性补单金额不会超过后台配置的单阶段最高百分比。
    #[test]
    fn robot_rhythm_stage_amounts_respect_configured_max_percent() {
        let mut plan = robot_test_group_buy_plan("G-RHYTHM-MAX", 30_000, 3_000);
        let max_percent = 20;
        let cap = rhythm_stage_fill_cap(&plan, max_percent).expect("cap can calculate");
        let mut stage_amounts = Vec::new();
        for index in 1..=3 {
            let stage = RobotFillStage::Stage { index };
            let remaining = plan.total_amount_minor - plan.filled_amount_minor;
            let amount = rhythm_stage_fill_amount(&plan, stage, max_percent, remaining)
                .expect("stage amount can calculate");
            assert!(amount > 0);
            assert!(
                amount <= cap,
                "单阶段补单金额 {amount} 不能超过配置上限 {cap}"
            );
            stage_amounts.push(amount);
            plan.filled_amount_minor += amount;
        }

        assert!(
            stage_amounts
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 1,
            "金额充足时阶段性补单金额应该自然起伏：{stage_amounts:?}"
        );
    }

    /// 验证阶段性补单按配置阶段数量动态切分到开奖前最终补满点。
    #[test]
    fn robot_rhythm_stage_uses_configured_dynamic_stage_count() {
        let mut plan = robot_test_group_buy_plan("G-RHYTHM-STAGE-COUNT", 30_000, 3_000);
        plan.created_at = "2026-06-05 20:00:40".to_string();
        let scheduled_at =
            parse_robot_timestamp("2026-06-05 20:05:00", "开奖时间").expect("time can parse");
        let first_at =
            parse_robot_timestamp("2026-06-05 20:00:40", "当前时间").expect("time can parse");
        let second_at =
            parse_robot_timestamp("2026-06-05 20:02:00", "当前时间").expect("time can parse");
        let third_at =
            parse_robot_timestamp("2026-06-05 20:03:30", "当前时间").expect("time can parse");
        let final_at =
            parse_robot_timestamp("2026-06-05 20:04:20", "当前时间").expect("time can parse");

        assert_eq!(
            robot_fill_stage(&plan, "2026-06-05 20:00:00", scheduled_at, first_at, 40, 3)
                .expect("stage can calculate"),
            Some((RobotFillStage::Stage { index: 1 }, "第1阶段".to_string()))
        );
        assert_eq!(
            robot_fill_stage(&plan, "2026-06-05 20:00:00", scheduled_at, second_at, 40, 3)
                .expect("stage can calculate"),
            Some((RobotFillStage::Stage { index: 2 }, "第2阶段".to_string()))
        );
        assert_eq!(
            robot_fill_stage(&plan, "2026-06-05 20:00:00", scheduled_at, third_at, 40, 3)
                .expect("stage can calculate"),
            Some((RobotFillStage::Stage { index: 3 }, "第3阶段".to_string()))
        );
        assert_eq!(
            robot_fill_stage(&plan, "2026-06-05 20:00:00", scheduled_at, final_at, 40, 3)
                .expect("stage can calculate"),
            Some((RobotFillStage::Final, "开奖前最终".to_string()))
        );
    }

    /// 验证阶段性补单的多用户拆分在金额充足时不会完全均分。
    #[test]
    fn robot_varied_split_amounts_are_not_all_equal_when_possible() {
        let min_share = 100;
        let participant_min = 500;
        let amounts =
            split_fill_amount_varied(10_000, 10, participant_min, min_share, "G-RHYTHM-SPLIT");

        assert_eq!(amounts.iter().sum::<i64>(), 10_000);
        for &amount in &amounts {
            assert_eq!(amount % min_share, 0);
            assert!(amount >= participant_min);
        }
        assert!(
            amounts
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 1,
            "金额充足时多用户补单不应完全均分：{amounts:?}"
        );
    }

    /// 验证机器人金额keep剩余参与有效。
    #[test]
    fn robot_amounts_keep_remaining_participation_valid() {
        let lottery = robot_test_lottery();
        let stake_count = 3;
        let (total, initiator) =
            robot_group_buy_amounts(&lottery, stake_count, ROBOT_BASE_UNIT_AMOUNT_MINOR * 3)
                .expect("amount can calculate");

        assert_eq!(total % lottery.group_buy.min_share_amount_minor, 0);
        assert_eq!(
            (total / stake_count) % ROBOT_BASE_UNIT_AMOUNT_MINOR,
            0,
            "机器人合买成单后的单注金额需要能换算为整数倍数"
        );
        assert!(initiator >= lottery.group_buy.participant_min_amount_minor);
        assert!(total - initiator >= lottery.group_buy.participant_min_amount_minor);
        assert!(
            total >= lottery.group_buy.participant_min_amount_minor * ROBOT_FILL_STAGE_COUNT,
            "机器人发起金额需要支持多阶段补单"
        );
    }
    /// 验证机器人run自动入账机器人账户when余额is低。
    #[tokio::test]
    async fn robot_run_auto_credits_robot_account_when_balance_is_low() {
        let access = AccessRepository::memory_seeded();
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        lotteries
            .set_sale_enabled("ssc60", true)
            .await
            .expect("lottery sale can be enabled");
        let mut lottery = lotteries.get("ssc60").await.expect("lottery exists");
        lottery.group_buy.enabled = true;
        lotteries
            .update("ssc60", lottery.clone())
            .await
            .expect("lottery can enable group buy");
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260605200500".to_string(),
                    scheduled_at: "2026-06-05 20:05:00".to_string(),
                    sale_closed_at: "2026-06-05 20:04:30".to_string(),
                },
            )
            .await
            .expect("issue can be created");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        finance
            .manual_adjust(ManualBalanceAdjustmentRequest {
                user_id: ROBOT_GROUP_BUY_USER_ID.to_string(),
                amount_minor: -520_000,
                description: "测试扣空机器人账户".to_string(),
            })
            .await
            .expect("robot balance can be reduced for test");
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();

        let run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:02:00".to_string(),
        )
        .await
        .expect("robot can run after auto credit");

        assert_eq!(run.created_plans.len(), 1);
        let created_plan = &run.created_plans[0];
        assert_eq!(created_plan.initiator_user_id, ROBOT_GROUP_BUY_USER_ID);
        assert_ne!(created_plan.initiator_username, ROBOT_GROUP_BUY_USERNAME);
        assert!(is_valid_robot_display_name(
            &created_plan.initiator_username
        ));
        assert_eq!(
            created_plan.participants[0].user_id,
            ROBOT_GROUP_BUY_USER_ID
        );
        assert_eq!(
            created_plan.participants[0].username,
            created_plan.initiator_username
        );
        assert!(run.ledger_entries.iter().any(|entry| {
            entry.user_id == ROBOT_GROUP_BUY_USER_ID
                && entry.kind == LedgerEntryKind::ManualAdjustment
                && entry.amount_minor > 0
                && entry.description.contains("机器人账户自动授信补余额")
        }));
        assert!(run.ledger_entries.iter().any(|entry| {
            entry.user_id == ROBOT_GROUP_BUY_USER_ID && entry.kind == LedgerEntryKind::GroupBuyDebit
        }));
        finance
            .ensure_available(ROBOT_GROUP_BUY_USER_ID, 1)
            .await
            .expect("robot account keeps positive balance after auto credit");
    }

    /// 验证合买机器人一轮可以并发处理同一机器人绑定的多个彩种任务。
    #[tokio::test]
    async fn robot_run_executes_multiple_lottery_jobs_in_one_round() {
        let access = AccessRepository::memory_seeded();
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let mut enabled_lotteries = Vec::new();
        for (lottery_id, issue) in [("ssc60", "20260605200500"), ("fc3d", "20260605200501")] {
            lotteries
                .set_sale_enabled(lottery_id, true)
                .await
                .expect("lottery sale can be enabled");
            let mut lottery = lotteries.get(lottery_id).await.expect("lottery exists");
            lottery.group_buy.enabled = true;
            let lottery = lotteries
                .update(lottery_id, lottery)
                .await
                .expect("lottery can enable group buy");
            draws
                .create(
                    &lottery,
                    CreateDrawIssueRequest {
                        lottery_id: lottery.id.clone(),
                        issue: issue.to_string(),
                        scheduled_at: "2026-06-05 20:05:00".to_string(),
                        sale_closed_at: "2026-06-05 20:04:30".to_string(),
                    },
                )
                .await
                .expect("issue can be created");
            enabled_lotteries.push(lottery.id);
        }
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        configure_seed_fill_robot_strategy(
            &robots,
            &lotteries,
            GroupBuyRobotFillStrategy::Rhythm,
            40,
        )
        .await;

        let run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:02:00".to_string(),
        )
        .await
        .expect("robot can run multiple lottery jobs");

        enabled_lotteries.sort();
        let mut created_lottery_ids = run
            .created_plans
            .iter()
            .map(|plan| plan.lottery_id.clone())
            .collect::<Vec<_>>();
        created_lottery_ids.sort();
        assert_eq!(created_lottery_ids, enabled_lotteries);
        assert_eq!(run.created_orders.len(), 0);
        assert!(
            run.ledger_entries.len() >= 2,
            "应至少有 2 条流水（发起人扣款 + 种子认购）"
        );
    }

    /// 验证机器人run创建计划then补满合买合买带节奏。
    #[tokio::test]
    async fn robot_run_creates_plan_then_fills_group_buy_with_rhythm() {
        let access = AccessRepository::memory_seeded();
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        lotteries
            .set_sale_enabled("ssc60", true)
            .await
            .expect("lottery sale can be enabled");
        let mut lottery = lotteries.get("ssc60").await.expect("lottery exists");
        lottery.group_buy.enabled = true;
        lotteries
            .update("ssc60", lottery.clone())
            .await
            .expect("lottery can enable group buy");
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260605200000".to_string(),
                    scheduled_at: "2026-06-05 20:00:00".to_string(),
                    sale_closed_at: "2026-06-05 19:59:30".to_string(),
                },
            )
            .await
            .expect("issue can be created");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        configure_seed_fill_robot_strategy(
            &robots,
            &lotteries,
            GroupBuyRobotFillStrategy::Rhythm,
            40,
        )
        .await;

        let before_window_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 19:57:00".to_string(),
        )
        .await
        .expect("robot can run");

        assert_eq!(before_window_run.created_plans.len(), 1);
        assert_eq!(
            before_window_run.created_plans[0].title,
            format!("{} 第20260605200000期合买", lottery.name)
        );
        assert!(!before_window_run.created_plans[0].title.contains("机器人"));
        assert_eq!(before_window_run.filled_plans.len(), 0);
        assert_eq!(before_window_run.created_orders.len(), 0);
        assert!(before_window_run.ledger_entries.len() >= 1);
        let plan_id = before_window_run.created_plans[0].id.clone();
        // 种子认购后 filled_amount 会大于发起人金额
        let seeded_plan = group_buys.get(&plan_id).await.expect("plan exists");
        assert!(seeded_plan.filled_amount_minor >= 1_000);
        assert!(seeded_plan.participants.len() >= 1);

        let stage_one_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 19:58:10".to_string(),
        )
        .await
        .expect("robot can run first fill stage");
        assert_eq!(stage_one_run.created_plans.len(), 0);
        assert_eq!(stage_one_run.filled_plans.len(), 0);
        assert_eq!(stage_one_run.created_orders.len(), 0);
        let p1 = group_buys.get(&plan_id).await.expect("plan exists");
        assert!(p1.filled_amount_minor >= seeded_plan.filled_amount_minor);

        let stage_two_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 19:58:40".to_string(),
        )
        .await
        .expect("robot can run second fill stage");
        assert_eq!(stage_two_run.filled_plans.len(), 0);
        assert_eq!(stage_two_run.created_orders.len(), 0);
        let p2 = group_buys.get(&plan_id).await.expect("plan exists");
        assert!(p2.filled_amount_minor >= p1.filled_amount_minor);

        let stage_three_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 19:59:05".to_string(),
        )
        .await
        .expect("robot can run third fill stage");
        assert_eq!(stage_three_run.filled_plans.len(), 0);
        assert_eq!(stage_three_run.created_orders.len(), 0);
        let p3 = group_buys.get(&plan_id).await.expect("plan exists");
        assert!(p3.filled_amount_minor >= p2.filled_amount_minor);

        let final_stage_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 19:59:20".to_string(),
        )
        .await
        .expect("robot can run final fill stage");

        assert_eq!(
            final_stage_run.filled_plans.len(),
            1,
            "最终阶段应补满，跳过原因：{:?}",
            final_stage_run.skipped_items
        );
        assert_eq!(final_stage_run.created_orders.len(), 1);
        assert_eq!(
            final_stage_run.created_orders[0].order_source,
            OrderSource::GroupBuy
        );
        assert_eq!(
            final_stage_run.filled_plans[0].order_id,
            Some(final_stage_run.created_orders[0].id.clone())
        );
        assert_robot_plan_progress(&group_buys, &plan_id, 5_000, 5_000, 1, true).await;
    }
    /// 验证机器人run补满已有非机器人合买合买计划带节奏。
    #[tokio::test]
    async fn robot_run_fills_existing_non_robot_group_buy_plan_with_rhythm() {
        let access = AccessRepository::memory_seeded();
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        lotteries
            .set_sale_enabled("ssc60", true)
            .await
            .expect("lottery sale can be enabled");
        let mut lottery = lotteries.get("ssc60").await.expect("lottery exists");
        lottery.group_buy.enabled = true;
        lotteries
            .update("ssc60", lottery.clone())
            .await
            .expect("lottery can enable group buy");
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260605200100".to_string(),
                    scheduled_at: "2026-06-05 20:01:00".to_string(),
                    sale_closed_at: "2026-06-05 20:00:30".to_string(),
                },
            )
            .await
            .expect("issue can be created");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let users = access.snapshot().await.expect("access can load").users;
        let user_plan = group_buys
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-USER-OPEN".to_string(),
                    lottery_id: "ssc60".to_string(),
                    issue: "20260605200100".to_string(),
                    rule_code: "fiveFrontDirect".to_string(),
                    title: "用户发起未满单合买".to_string(),
                    numbers: "1|2|3".to_string(),
                    initiator_user_id: "U10001".to_string(),
                    total_amount_minor: 5_000,
                    initiator_amount_minor: 1_000,
                    note: "测试用户计划".to_string(),
                },
                std::slice::from_ref(&lottery),
                &users,
            )
            .await
            .expect("user plan can be created");
        finance
            .debit_group_buy("U10001", 1_000, "G-USER-OPEN-P001", &user_plan.id)
            .await
            .expect("user initiator can be debited");
        let robots = RobotRepository::memory_seeded();
        configure_seed_fill_robot_strategy(
            &robots,
            &lotteries,
            GroupBuyRobotFillStrategy::Rhythm,
            40,
        )
        .await;

        let before_window_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 19:58:00".to_string(),
        )
        .await
        .expect("robot can run first dynamic fill stage");
        assert!(before_window_run
            .filled_plans
            .iter()
            .all(|plan| plan.id != "G-USER-OPEN"));
        let early_stage_plan = group_buys.get("G-USER-OPEN").await.expect("plan exists");
        assert!(early_stage_plan.filled_amount_minor >= 1_000);
        assert!(early_stage_plan.filled_amount_minor < 5_000);

        let stage_one_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 19:59:00".to_string(),
        )
        .await
        .expect("robot can run first fill stage");
        assert!(stage_one_run
            .filled_plans
            .iter()
            .all(|plan| plan.id != "G-USER-OPEN"));
        let stage_one_plan = group_buys.get("G-USER-OPEN").await.expect("plan exists");
        assert!(stage_one_plan.filled_amount_minor >= early_stage_plan.filled_amount_minor);
        assert!(stage_one_plan.filled_amount_minor < 5_000);
        assert!(stage_one_plan.participants.len() >= 2);

        run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 19:59:30".to_string(),
        )
        .await
        .expect("robot can run second fill stage");
        let stage_two_plan = group_buys.get("G-USER-OPEN").await.expect("plan exists");
        assert!(stage_two_plan.filled_amount_minor >= stage_one_plan.filled_amount_minor);
        assert!(stage_two_plan.filled_amount_minor < 5_000);

        run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:00:05".to_string(),
        )
        .await
        .expect("robot can run third fill stage");
        let stage_three_plan = group_buys.get("G-USER-OPEN").await.expect("plan exists");
        assert!(stage_three_plan.filled_amount_minor >= stage_two_plan.filled_amount_minor);
        assert!(stage_three_plan.filled_amount_minor < 5_000);

        let final_stage_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:00:20".to_string(),
        )
        .await
        .expect("robot can run final fill stage");

        let filled_user_plan = final_stage_run
            .filled_plans
            .iter()
            .find(|plan| plan.id == "G-USER-OPEN")
            .expect("existing user plan should be filled");
        assert!(filled_user_plan.order_id.is_some());
        assert!(final_stage_run
            .created_orders
            .iter()
            .any(|order| Some(&order.id) == filled_user_plan.order_id.as_ref()));
        assert_robot_plan_progress(&group_buys, "G-USER-OPEN", 5_000, 5_000, 5, true).await;
    }
    /// 验证机器人兜底补满封盘用户合买合买之前退款。
    #[tokio::test]
    async fn robot_guard_fills_closed_user_group_buy_before_refund() {
        let access = AccessRepository::memory_seeded();
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        lotteries
            .set_sale_enabled("ssc60", true)
            .await
            .expect("lottery sale can be enabled");
        let mut lottery = lotteries.get("ssc60").await.expect("lottery exists");
        lottery.group_buy.enabled = true;
        lotteries
            .update("ssc60", lottery.clone())
            .await
            .expect("lottery can enable group buy");
        let issue = draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260605200200".to_string(),
                    scheduled_at: "2026-06-05 20:02:00".to_string(),
                    sale_closed_at: "2026-06-05 20:01:30".to_string(),
                },
            )
            .await
            .expect("issue can be created");
        draws.close(&issue.id).await.expect("issue can be closed");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let users = access.snapshot().await.expect("access can load").users;
        let user_plan = group_buys
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-USER-GUARD".to_string(),
                    lottery_id: "ssc60".to_string(),
                    issue: "20260605200200".to_string(),
                    rule_code: "fiveFrontDirect".to_string(),
                    title: "用户封盘后兜底合买".to_string(),
                    numbers: "1|2|3".to_string(),
                    initiator_user_id: "U10001".to_string(),
                    total_amount_minor: 5_000,
                    initiator_amount_minor: 1_000,
                    note: "测试封盘后兜底".to_string(),
                },
                std::slice::from_ref(&lottery),
                &users,
            )
            .await
            .expect("user plan can be created");
        finance
            .debit_group_buy("U10001", 1_000, "G-USER-GUARD-P001", &user_plan.id)
            .await
            .expect("user initiator can be debited");
        let robots = RobotRepository::memory_seeded();

        let run = force_fill_user_group_buy_plans_before_refund(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:01:45".to_string(),
        )
        .await
        .expect("guard can fill closed issue before draw");

        let filled_plan = run
            .filled_plans
            .iter()
            .find(|plan| plan.id == "G-USER-GUARD")
            .expect("guard should fill user plan");
        assert_eq!(run.created_orders.len(), 1);
        assert_eq!(run.created_orders[0].order_source, OrderSource::GroupBuy);
        assert_eq!(filled_plan.order_id, Some(run.created_orders[0].id.clone()));
        assert_robot_plan_progress(&group_buys, "G-USER-GUARD", 5_000, 5_000, 2, true).await;
    }
    /// 验证补单机器人按开奖前策略补满合买机器人发起的计划。
    #[tokio::test]
    async fn robot_run_fills_own_plan_by_before_draw_strategy() {
        let access = AccessRepository::memory_seeded();
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        lotteries
            .set_sale_enabled("ssc60", true)
            .await
            .expect("lottery sale can be enabled");
        let mut lottery = lotteries.get("ssc60").await.expect("lottery exists");
        lottery.group_buy.enabled = true;
        lotteries
            .update("ssc60", lottery.clone())
            .await
            .expect("lottery can enable group buy");
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260605201000".to_string(),
                    scheduled_at: "2026-06-05 20:10:00".to_string(),
                    sale_closed_at: "2026-06-05 20:09:30".to_string(),
                },
            )
            .await
            .expect("issue can be created");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        configure_seed_fill_robot_strategy(
            &robots,
            &lotteries,
            GroupBuyRobotFillStrategy::BeforeDraw,
            45,
        )
        .await;

        let before_threshold_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:08:50".to_string(),
        )
        .await
        .expect("robot can create plan before threshold");

        assert_eq!(before_threshold_run.created_plans.len(), 1);
        let created_plan = &before_threshold_run.created_plans[0];
        let plan_id = created_plan.id.clone();
        let expected_total = created_plan.total_amount_minor;
        // 种子认购会改变实际 filled_amount，从 DB 读取真实状态
        let actual_plan = group_buys.get(&plan_id).await.expect("plan exists");
        let participants_before_fill = actual_plan.participants.len();
        assert!(expected_total > actual_plan.filled_amount_minor);
        assert_robot_plan_progress(
            &group_buys,
            &plan_id,
            expected_total,
            actual_plan.filled_amount_minor,
            1,
            false,
        )
        .await;

        let fill_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:09:20".to_string(),
        )
        .await
        .expect("robot can fill plan in before draw window");

        assert_eq!(fill_run.filled_plans.len(), 1);
        assert_eq!(fill_run.created_orders.len(), 1);
        assert_robot_plan_progress(
            &group_buys,
            &plan_id,
            expected_total,
            expected_total,
            2,
            true,
        )
        .await;
        let filled_plan = group_buys.get(&plan_id).await.expect("plan exists");
        assert_eq!(
            filled_plan.participants.len(),
            participants_before_fill + 1,
            "开奖前补满策略应只新增一个补单机器人参与记录"
        );
    }
    /// 验证补单机器人按开奖前策略补满用户发起的计划。
    #[tokio::test]
    async fn robot_run_fills_user_plan_by_before_draw_strategy() {
        let access = AccessRepository::memory_seeded();
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        lotteries
            .set_sale_enabled("ssc60", true)
            .await
            .expect("lottery sale can be enabled");
        let mut lottery = lotteries.get("ssc60").await.expect("lottery exists");
        lottery.group_buy.enabled = true;
        lotteries
            .update("ssc60", lottery.clone())
            .await
            .expect("lottery can enable group buy");
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260605201100".to_string(),
                    scheduled_at: "2026-06-05 20:11:00".to_string(),
                    sale_closed_at: "2026-06-05 20:10:30".to_string(),
                },
            )
            .await
            .expect("issue can be created");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let users = access.snapshot().await.expect("access can load").users;
        let user_plan = group_buys
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-USER-BEFORE-DRAW".to_string(),
                    lottery_id: "ssc60".to_string(),
                    issue: "20260605201100".to_string(),
                    rule_code: "fiveFrontDirect".to_string(),
                    title: "用户开奖前补满测试合买".to_string(),
                    numbers: "1|2|3".to_string(),
                    initiator_user_id: "U10001".to_string(),
                    total_amount_minor: 5_000,
                    initiator_amount_minor: 1_000,
                    note: "测试开奖前补满".to_string(),
                },
                std::slice::from_ref(&lottery),
                &users,
            )
            .await
            .expect("user plan can be created");
        finance
            .debit_group_buy("U10001", 1_000, "G-USER-BEFORE-DRAW-P001", &user_plan.id)
            .await
            .expect("user initiator can be debited");
        let robots = RobotRepository::memory_seeded();
        configure_seed_fill_robot_strategy(
            &robots,
            &lotteries,
            GroupBuyRobotFillStrategy::BeforeDraw,
            45,
        )
        .await;

        run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:09:30".to_string(),
        )
        .await
        .expect("robot can run before before-draw window");
        assert_robot_plan_progress(&group_buys, "G-USER-BEFORE-DRAW", 5_000, 1_000, 1, false).await;

        let fill_run = run_group_buy_robots(
            &robots,
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &access,
            "2026-06-05 20:10:20".to_string(),
        )
        .await
        .expect("robot can fill user plan in before-draw window");

        let filled_user_plan = fill_run
            .filled_plans
            .iter()
            .find(|plan| plan.id == "G-USER-BEFORE-DRAW")
            .expect("user plan should be filled by before-draw strategy");
        assert!(filled_user_plan.order_id.is_some());
        assert_robot_plan_progress(&group_buys, "G-USER-BEFORE-DRAW", 5_000, 5_000, 2, true).await;
        let plan_after_fill = group_buys
            .get("G-USER-BEFORE-DRAW")
            .await
            .expect("plan exists");
        assert_eq!(
            plan_after_fill.participants.len(),
            2,
            "用户合买开奖前补满应由一个补单机器人吃掉剩余份额"
        );
    }
    /// 断言机器人计划progress满足测试要求。
    async fn assert_robot_plan_progress(
        group_buys: &GroupBuyRepository,
        plan_id: &str,
        expected_total: i64,
        expected_filled: i64,
        min_participants: usize,
        should_be_filled: bool,
    ) {
        let plan = group_buys
            .get(plan_id)
            .await
            .expect("group buy plan exists");
        assert_eq!(plan.total_amount_minor, expected_total);
        assert_eq!(plan.filled_amount_minor, expected_filled);
        assert!(
            plan.participants.len() >= min_participants,
            "参与者数量 {} 应 >= {}",
            plan.participants.len(),
            min_participants
        );
        if should_be_filled {
            assert!(matches!(plan.status, GroupBuyPlanStatus::Filled));
            assert!(plan.order_id.is_some());
        } else {
            assert!(matches!(plan.status, GroupBuyPlanStatus::Open));
            assert!(plan.order_id.is_none());
        }
    }
    /// 验证机器人测试彩种。
    fn robot_test_lottery() -> LotteryKind {
        LotteryKind {
            id: "robot-test".to_string(),
            name: "机器人测试彩".to_string(),
            category: "test".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: crate::domain::lottery::DrawMode::Platform,
            api_draw_delay_seconds: 0,
            draw_control_enabled: true,
            avoid_winning_enabled: false,
            issue_format: crate::domain::lottery::DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            sale_close_lead_seconds: crate::domain::lottery::DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: true,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1_000,
            },
            play_categories: vec![PlayCategory::Direct],
            play_configs: vec![crate::domain::lottery::LotteryPlayConfig {
                rule_code: PlayRuleCode::FiveFrontDirect,
                enabled: true,
                odds_basis_points: 95_000,
                position_select_limits: Vec::new(),
            }],
        }
    }
    /// 验证机器人测试配置。
    fn robot_test_config() -> RobotConfigSummary {
        RobotConfigSummary {
            id: "R-BUY-TEST".to_string(),
            name: "测试合买机器人".to_string(),
            kind: RobotKind::GroupBuy,
            lottery_ids: vec!["robot-test".to_string()],
            status: RobotStatus::Enabled,
            description: "测试机器人".to_string(),
            group_buy_fill_strategy: GroupBuyRobotFillStrategy::Rhythm,
            group_buy_fill_before_draw_seconds: 15,
            group_buy_rhythm_fill_max_percent: 20,
            group_buy_rhythm_stage_count: 3,
            deletable: true,
        }
    }
    /// 构造测试合买计划，供纯函数测试阶段金额。
    fn robot_test_group_buy_plan(
        id: &str,
        total_amount_minor: i64,
        filled_amount_minor: i64,
    ) -> GroupBuyPlan {
        GroupBuyPlan {
            id: id.to_string(),
            lottery_id: "robot-test".to_string(),
            lottery_name: "机器人测试彩".to_string(),
            order_id: None,
            issue: "20260605200000".to_string(),
            rule_code: "fiveFrontDirect".to_string(),
            title: "测试合买".to_string(),
            numbers: "1|2|3".to_string(),
            initiator_user_id: "U10001".to_string(),
            initiator_username: "真实用户".to_string(),
            total_amount_minor,
            filled_amount_minor,
            min_share_amount_minor: 100,
            participant_min_amount_minor: 500,
            share_count: (total_amount_minor / 100) as u32,
            status: GroupBuyPlanStatus::Open,
            participants: Vec::new(),
            note: "测试".to_string(),
            created_at: "2026-06-05 19:00:00".to_string(),
            updated_at: "2026-06-05 19:00:00".to_string(),
        }
    }
    /// 配置种子补单机器人的补满策略。
    async fn configure_seed_fill_robot_strategy(
        robots: &RobotRepository,
        lotteries: &LotteryRepository,
        strategy: GroupBuyRobotFillStrategy,
        before_draw_seconds: u32,
    ) {
        let mut robot = robots
            .get("R-BUY-001")
            .await
            .expect("seed fill robot exists");
        robot.group_buy_fill_strategy = strategy;
        robot.group_buy_fill_before_draw_seconds = before_draw_seconds;
        robot.status = RobotStatus::Enabled;
        let lottery_snapshot = lotteries.list().await.expect("lotteries can load");
        robots
            .update("R-BUY-001", robot, &lottery_snapshot)
            .await
            .expect("seed fill robot can update strategy");
    }
    /// 验证机器人测试期号。
    fn robot_test_issue(issue: &str) -> DrawIssue {
        DrawIssue {
            id: format!("I-{issue}"),
            lottery_id: "robot-test".to_string(),
            lottery_name: "机器人测试彩".to_string(),
            issue: issue.to_string(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: crate::domain::lottery::DrawMode::Platform,
            scheduled_at: "2026-06-05 20:00:00".to_string(),
            sale_closed_at: "2026-06-05 19:59:30".to_string(),
            status: DrawIssueStatus::Open,
            draw_number: None,
            drawn_at: None,
            created_at: "2026-06-05 19:00:00".to_string(),
        }
    }
    /// 验证机器人supported玩法玩法。
    fn robot_supported_play_rules() -> Vec<PlayRuleCode> {
        use PlayRuleCode::*;
        vec![
            ThreeDirect,
            ThreeGroupThree,
            ThreeGroupThreeBanker,
            ThreeGroupSix,
            ThreeGroupSixBanker,
            FiveFrontDirect,
            FiveMiddleDirect,
            FiveBackDirect,
            FiveFrontDirectCombination,
            FiveMiddleDirectCombination,
            FiveBackDirectCombination,
            FiveFrontGroupThree,
            FiveMiddleGroupThree,
            FiveBackGroupThree,
            FiveFrontGroupThreeBanker,
            FiveMiddleGroupThreeBanker,
            FiveBackGroupThreeBanker,
            FiveFrontGroupSix,
            FiveMiddleGroupSix,
            FiveBackGroupSix,
            FiveFrontGroupSixBanker,
            FiveMiddleGroupSixBanker,
            FiveBackGroupSixBanker,
            FiveBigSmallOddEven,
        ]
    }
}
