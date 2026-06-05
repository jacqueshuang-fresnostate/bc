//! 合买机器人执行服务，负责按当前彩种、期号和玩法规则自动发起并补满合买。

use std::collections::BTreeMap;

use crate::{
    domain::{
        draw::{DrawIssue, DrawIssueStatus},
        group_buy::{
            AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyPlan,
            GroupBuyPlanStatus,
        },
        lottery::LotteryKind,
        order::CreateOrderRequest,
        play::PlayRuleCode,
        robot::{
            GroupBuyRobotRun, GroupBuyRobotSkippedItem, RobotConfigSummary, RobotKind, RobotStatus,
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

const ROBOT_GROUP_BUY_USER_ID: &str = "U90001";
const ROBOT_GROUP_BUY_USERNAME: &str = "agent_alpha";
const ROBOT_FILL_PARTICIPANT_SUFFIX: &str = "P-ROBOT-FILL";

/// 执行全部已启用的合买机器人，并返回本轮创建、满单、成单和跳过明细。
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
    let access = access.snapshot().await?;
    let robot_user = robot_user(&access.users)?;
    let lotteries_by_id = lotteries
        .list()
        .await?
        .into_iter()
        .map(|lottery| (lottery.id.clone(), lottery))
        .collect::<BTreeMap<_, _>>();
    let mut run = GroupBuyRobotRun {
        now: now.clone(),
        created_plans: Vec::new(),
        filled_plans: Vec::new(),
        created_orders: Vec::new(),
        ledger_entries: Vec::new(),
        skipped_items: Vec::new(),
    };

    for robot in robots.list().await? {
        if robot.kind != RobotKind::GroupBuy {
            continue;
        }
        if robot.status != RobotStatus::Enabled {
            push_skipped(&mut run, &robot, "", None, "机器人未启用，跳过执行");
            continue;
        }

        for lottery_id in &robot.lottery_ids {
            let Some(lottery) = lotteries_by_id.get(lottery_id) else {
                push_skipped(&mut run, &robot, lottery_id, None, "绑定彩种不存在");
                continue;
            };
            if !lottery.sale_enabled {
                push_skipped(&mut run, &robot, &lottery.id, None, "彩种已停售");
                continue;
            }
            if !lottery.group_buy.enabled {
                push_skipped(&mut run, &robot, &lottery.id, None, "彩种未开启合买");
                continue;
            }

            let Some(issue) = current_open_issue(draws, lottery, &now).await? else {
                push_skipped(&mut run, &robot, &lottery.id, None, "没有可销售的当前期号");
                continue;
            };

            if let Err(error) = execute_lottery_robot(
                &mut run,
                &robot,
                lottery,
                &issue,
                draws,
                orders,
                finance,
                group_buys,
                &access.users,
                robot_user,
            )
            .await
            {
                push_skipped(
                    &mut run,
                    &robot,
                    &lottery.id,
                    Some(issue.issue.clone()),
                    error.to_string(),
                );
            }
            fill_existing_group_buy_plans(
                &mut run,
                &robot,
                lottery,
                &issue,
                draws,
                orders,
                finance,
                group_buys,
                &access.users,
            )
            .await?;
        }
    }

    Ok(run)
}

/// 对单个机器人和彩种执行创建或补满合买。
async fn execute_lottery_robot(
    run: &mut GroupBuyRobotRun,
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    draws: &DrawRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    users: &[UserSummary],
    robot_user: &UserSummary,
) -> ApiResult<()> {
    let plan_id = robot_plan_id(robot, lottery, issue);
    let mut plan = match group_buys.get(&plan_id).await {
        Ok(existing) => existing,
        Err(ApiError::NotFound(_)) => {
            let draft = build_robot_plan_request(robot, lottery, issue, orders, robot_user).await?;
            finance
                .ensure_available(&robot_user.id, draft.total_amount_minor)
                .await?;
            let created = group_buys
                .create(draft.clone(), std::slice::from_ref(lottery), users)
                .await?;
            let participant_id = format!("{}-P001", created.id);
            match finance
                .debit_group_buy(
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

    match plan.status {
        GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open => {
            fill_robot_plan(
                run, lottery, &mut plan, draws, orders, finance, group_buys, users,
            )
            .await?;
        }
        GroupBuyPlanStatus::Filled if plan.order_id.is_none() => {
            attach_order_for_plan(run, lottery, &plan, draws, orders, group_buys).await?;
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

/// 补满同彩种当前期由用户或后台发起的未满单计划。
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
) -> ApiResult<()> {
    let robot_plan_id = robot_plan_id(robot, lottery, issue);
    let candidate_plans = group_buys
        .list_details()
        .await?
        .into_iter()
        .filter(|plan| {
            plan.lottery_id == lottery.id
                && plan.issue == issue.issue
                && plan.id != robot_plan_id
                && !plan.id.starts_with("G-ROBOT-")
                && matches!(
                    plan.status,
                    GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open
                )
                && plan.filled_amount_minor < plan.total_amount_minor
        })
        .collect::<Vec<_>>();

    for mut plan in candidate_plans {
        let plan_id = plan.id.clone();
        if let Err(error) = fill_robot_plan(
            run, lottery, &mut plan, draws, orders, finance, group_buys, users,
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

    Ok(())
}

/// 补足机器人合买剩余金额，满单后生成真实投注订单。
async fn fill_robot_plan(
    run: &mut GroupBuyRobotRun,
    lottery: &LotteryKind,
    plan: &mut GroupBuyPlan,
    draws: &DrawRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    users: &[UserSummary],
) -> ApiResult<()> {
    let remaining_amount_minor = plan
        .total_amount_minor
        .checked_sub(plan.filled_amount_minor)
        .ok_or_else(|| ApiError::BadRequest("合买剩余金额无效".to_string()))?;
    if remaining_amount_minor <= 0 {
        return Ok(());
    }

    let participant_id = robot_fill_participant_id(&plan.id);
    finance
        .ensure_available(ROBOT_GROUP_BUY_USER_ID, remaining_amount_minor)
        .await?;
    let next_plan = group_buys
        .add_participant(
            &plan.id,
            AddGroupBuyParticipantRequest {
                id: participant_id.clone(),
                user_id: ROBOT_GROUP_BUY_USER_ID.to_string(),
                amount_minor: remaining_amount_minor,
                note: "合买机器人自动补满".to_string(),
            },
            users,
        )
        .await?;
    *plan = next_plan;

    let mut created_order =
        match create_order_for_filled_group_buy(draws, orders, group_buys, lottery, plan).await {
            Ok(result) => result,
            Err(error) => {
                if let Err(rollback_error) = group_buys
                    .remove_unfunded_participant(&plan.id, &participant_id)
                    .await
                {
                    tracing::error!(
                        "合买计划ID" = %plan.id,
                        "参与记录ID" = %participant_id,
                        error = %rollback_error.log_message(),
                        "合买机器人满单成单失败后移除参与记录失败"
                    );
                }
                return Err(error);
            }
        };

    if let Some((_, attached_plan)) = &created_order {
        *plan = attached_plan.clone();
    }

    match finance
        .debit_group_buy(
            ROBOT_GROUP_BUY_USER_ID,
            remaining_amount_minor,
            &participant_id,
            &plan.id,
        )
        .await
    {
        Ok(entry) => run.ledger_entries.push(entry),
        Err(error) => {
            if let Some((order, _)) = created_order.take() {
                if let Err(rollback_error) = orders.remove_unfunded(&order.id).await {
                    tracing::error!(
                        "订单ID" = %order.id,
                        error = %rollback_error.log_message(),
                        "合买机器人扣款失败后移除满单订单失败"
                    );
                }
            }
            if let Err(rollback_error) = group_buys
                .remove_unfunded_participant(&plan.id, &participant_id)
                .await
            {
                tracing::error!(
                    "合买计划ID" = %plan.id,
                    "参与记录ID" = %participant_id,
                    error = %rollback_error.log_message(),
                    "合买机器人扣款失败后移除参与记录失败"
                );
            }
            return Err(error);
        }
    }

    if let Some((order, attached_plan)) = created_order {
        run.created_orders.push(order);
        run.filled_plans.push(attached_plan);
    }

    Ok(())
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
        run.created_orders.push(order);
        run.filled_plans.push(attached_plan);
    }
    Ok(())
}

/// 生成机器人计划请求，并选出当前彩种第一个可验证的启用玩法。
async fn build_robot_plan_request(
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    orders: &OrderRepository,
    robot_user: &UserSummary,
) -> ApiResult<CreateGroupBuyPlanRequest> {
    let mut skipped_reasons = Vec::new();
    for play in lottery.play_configs.iter().filter(|play| play.enabled) {
        let numbers = default_numbers_for_rule(&play.rule_code);
        let selection = match parse_group_buy_selection(&play.rule_code, numbers) {
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
        let (total_amount_minor, initiator_amount_minor) =
            robot_group_buy_amounts(lottery, i64::from(quote.stake_count))?;
        let rule_code = enum_to_string(&play.rule_code)?;

        return Ok(CreateGroupBuyPlanRequest {
            id: robot_plan_id(robot, lottery, issue),
            lottery_id: lottery.id.clone(),
            issue: issue.issue.clone(),
            rule_code,
            title: format!("{} {}", robot.name, issue.issue),
            numbers: numbers.to_string(),
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
fn robot_group_buy_amounts(lottery: &LotteryKind, stake_count: i64) -> ApiResult<(i64, i64)> {
    if stake_count <= 0 {
        return Err(ApiError::BadRequest("机器人投注注数无效".to_string()));
    }

    let min_share = lottery.group_buy.min_share_amount_minor.max(1);
    let participant_min = lottery
        .group_buy
        .participant_min_amount_minor
        .max(min_share);
    let total_unit = lcm(min_share, stake_count)?;
    let mut total = round_up_to_multiple(participant_min * 2, total_unit)?;
    total = total.max(round_up_to_multiple(min_share * 10, total_unit)?);

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

/// 选择当前仍可销售的最近一期。
async fn current_open_issue(
    draws: &DrawRepository,
    lottery: &LotteryKind,
    now: &str,
) -> ApiResult<Option<DrawIssue>> {
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

/// 返回指定玩法的默认一组投注文本。
fn default_numbers_for_rule(rule_code: &PlayRuleCode) -> &'static str {
    use PlayRuleCode::*;

    match rule_code {
        ThreeDirect | FiveFrontDirect | FiveMiddleDirect | FiveBackDirect => "1|2|3",
        FiveFrontDirectCombination | FiveMiddleDirectCombination | FiveBackDirectCombination => {
            "1,2,3"
        }
        ThreeGroupThree | FiveFrontGroupThree | FiveMiddleGroupThree | FiveBackGroupThree => "1,2",
        ThreeGroupThreeBanker
        | FiveFrontGroupThreeBanker
        | FiveMiddleGroupThreeBanker
        | FiveBackGroupThreeBanker => "1|2",
        ThreeGroupSix | FiveFrontGroupSix | FiveMiddleGroupSix | FiveBackGroupSix => "1,2,3",
        ThreeGroupSixBanker
        | FiveFrontGroupSixBanker
        | FiveMiddleGroupSixBanker
        | FiveBackGroupSixBanker => "1|2,3",
        FiveBigSmallOddEven => "大|单",
    }
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

/// 生成同一机器人、彩种、期号的确定性合买计划 ID。
fn robot_plan_id(robot: &RobotConfigSummary, lottery: &LotteryKind, issue: &DrawIssue) -> String {
    format!(
        "G-ROBOT-{}-{}-{}",
        slug_fragment(&robot.id),
        slug_fragment(&lottery.id),
        slug_fragment(&issue.issue)
    )
}

/// 生成机器人补满参与记录 ID。
fn robot_fill_participant_id(plan_id: &str) -> String {
    format!("{plan_id}-{ROBOT_FILL_PARTICIPANT_SUFFIX}")
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
            lottery::{DrawSchedule, GroupBuyConfig, LotteryNumberType, PlayCategory},
        },
        services::{
            draw::DrawRepository, finance::FinanceRepository, lottery::LotteryRepository,
            robot::RobotRepository,
        },
    };

    #[test]
    fn robot_amounts_keep_remaining_participation_valid() {
        let lottery = robot_test_lottery();
        let (total, initiator) =
            robot_group_buy_amounts(&lottery, 1).expect("amount can calculate");

        assert_eq!(total % lottery.group_buy.min_share_amount_minor, 0);
        assert!(initiator >= lottery.group_buy.participant_min_amount_minor);
        assert!(total - initiator >= lottery.group_buy.participant_min_amount_minor);
    }

    #[tokio::test]
    async fn robot_run_creates_fills_and_orders_group_buy() {
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

        let run = run_group_buy_robots(
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
        .expect("robot can run");

        assert_eq!(run.created_plans.len(), 1);
        assert_eq!(run.filled_plans.len(), 1);
        assert_eq!(run.created_orders.len(), 1);
        assert_eq!(run.ledger_entries.len(), 2);
        assert_eq!(
            run.filled_plans[0].order_id,
            Some(run.created_orders[0].id.clone())
        );
    }

    #[tokio::test]
    async fn robot_run_fills_existing_non_robot_group_buy_plan() {
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
                    total_amount_minor: 2_000,
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

        let run = run_group_buy_robots(
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
        .expect("robot can run");

        let filled_user_plan = run
            .filled_plans
            .iter()
            .find(|plan| plan.id == "G-USER-OPEN")
            .expect("existing user plan should be filled");
        assert!(filled_user_plan.order_id.is_some());
        assert!(run
            .created_orders
            .iter()
            .any(|order| Some(&order.id) == filled_user_plan.order_id.as_ref()));
    }

    fn robot_test_lottery() -> LotteryKind {
        LotteryKind {
            id: "robot-test".to_string(),
            name: "机器人测试彩".to_string(),
            category: "test".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: crate::domain::lottery::DrawMode::Platform,
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
            }],
        }
    }
}
