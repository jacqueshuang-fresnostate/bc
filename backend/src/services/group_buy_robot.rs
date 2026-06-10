//! 合买机器人执行服务，负责按当前彩种、期号和玩法规则自动发起并按节奏补单。

use std::collections::BTreeMap;

use chrono::NaiveDateTime;

use crate::{
    domain::{
        draw::{DrawIssue, DrawIssueStatus},
        finance::{LedgerEntry, ManualBalanceAdjustmentRequest},
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

pub const ROBOT_GROUP_BUY_USER_ID: &str = "U90001";
const ROBOT_GROUP_BUY_USERNAME: &str = "agent_alpha";
const ROBOT_FILL_PARTICIPANT_SUFFIX: &str = "P-ROBOT-FILL";
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const ROBOT_AUTO_CREDIT_RESERVE_MINOR: i64 = 100_000;
const ROBOT_FILL_WINDOW_SECONDS: i64 = 90;
const ROBOT_FILL_STAGE_ONE_SECONDS: i64 = 60;
const ROBOT_FILL_STAGE_TWO_SECONDS: i64 = 30;
const ROBOT_FILL_FINAL_STAGE_SECONDS: i64 = 15;
const ROBOT_FILL_STAGE_ONE_TARGET_PERCENT: i64 = 40;
const ROBOT_FILL_STAGE_TWO_TARGET_PERCENT: i64 = 60;
const ROBOT_FILL_STAGE_THREE_TARGET_PERCENT: i64 = 80;
const ROBOT_FILL_FINAL_TARGET_PERCENT: i64 = 100;
const ROBOT_FILL_STAGE_COUNT: i64 = 5;

/// 判断用户 ID 是否为系统合买机器人账户。
pub fn is_group_buy_robot_user_id(user_id: &str) -> bool {
    user_id.trim() == ROBOT_GROUP_BUY_USER_ID
}

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
    let now_at = parse_robot_timestamp(&now, "机器人执行时间")?;
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
                now_at,
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
                now_at,
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
    now_at: NaiveDateTime,
) -> ApiResult<()> {
    let plan_id = robot_plan_id(robot, lottery, issue);
    let mut plan = match group_buys.get(&plan_id).await {
        Ok(existing) => existing,
        Err(ApiError::NotFound(_)) => {
            let draft = build_robot_plan_request(robot, lottery, issue, orders, robot_user).await?;
            if let Some(entry) =
                ensure_robot_balance(finance, draft.total_amount_minor, "发起合买计划").await?
            {
                run.ledger_entries.push(entry);
            }
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
                run, robot, lottery, issue, &mut plan, draws, orders, finance, group_buys, users,
                now_at,
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
    now_at: NaiveDateTime,
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
            run, robot, lottery, issue, &mut plan, draws, orders, finance, group_buys, users,
            now_at,
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
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    plan: &mut GroupBuyPlan,
    draws: &DrawRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    users: &[UserSummary],
    now_at: NaiveDateTime,
) -> ApiResult<()> {
    let decision = match robot_fill_decision(plan, issue, now_at)? {
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

    let participant_id = next_robot_fill_participant_id(plan);
    if let Some(entry) = ensure_robot_balance(finance, fill_amount_minor, "合买分阶段补单").await?
    {
        run.ledger_entries.push(entry);
    }
    finance
        .ensure_available(ROBOT_GROUP_BUY_USER_ID, fill_amount_minor)
        .await?;
    let next_plan = group_buys
        .add_participant(
            &plan.id,
            AddGroupBuyParticipantRequest {
                id: participant_id.clone(),
                user_id: ROBOT_GROUP_BUY_USER_ID.to_string(),
                amount_minor: fill_amount_minor,
                note: fill_note,
            },
            users,
        )
        .await?;
    *plan = next_plan;

    if !matches!(plan.status, GroupBuyPlanStatus::Filled) {
        match finance
            .debit_group_buy(
                ROBOT_GROUP_BUY_USER_ID,
                fill_amount_minor,
                &participant_id,
                &plan.id,
            )
            .await
        {
            Ok(entry) => {
                run.ledger_entries.push(entry);
                return Ok(());
            }
            Err(error) => {
                if let Err(rollback_error) = group_buys
                    .remove_unfunded_participant(&plan.id, &participant_id)
                    .await
                {
                    tracing::error!(
                        "合买计划ID" = %plan.id,
                        "参与记录ID" = %participant_id,
                        error = %rollback_error.log_message(),
                        "合买机器人扣款失败后移除分段参与记录失败"
                    );
                }
                return Err(error);
            }
        }
    }

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
            fill_amount_minor,
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
    for (play_index, play) in lottery
        .play_configs
        .iter()
        .filter(|play| play.enabled)
        .enumerate()
    {
        let numbers = robot_numbers_for_rule(robot, lottery, issue, &play.rule_code, play_index);
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
        let (total_amount_minor, initiator_amount_minor) =
            robot_group_buy_amounts(lottery, i64::from(quote.stake_count))?;
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

/// 按机器人、彩种、期号和玩法派生一组可校验的随机投注文本。
fn robot_numbers_for_rule(
    robot: &RobotConfigSummary,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    rule_code: &PlayRuleCode,
    play_index: usize,
) -> String {
    use PlayRuleCode::*;

    let mut picker = RobotNumberPicker::new(robot, lottery, issue, rule_code, play_index);
    match rule_code {
        ThreeDirect | FiveFrontDirect | FiveMiddleDirect | FiveBackDirect => {
            format!("{}|{}|{}", picker.digit(), picker.digit(), picker.digit())
        }
        FiveFrontDirectCombination | FiveMiddleDirectCombination | FiveBackDirectCombination => {
            join_digits(&picker.unique_digits(3))
        }
        ThreeGroupThree | FiveFrontGroupThree | FiveMiddleGroupThree | FiveBackGroupThree => {
            join_digits(&picker.unique_digits(2))
        }
        ThreeGroupThreeBanker
        | FiveFrontGroupThreeBanker
        | FiveMiddleGroupThreeBanker
        | FiveBackGroupThreeBanker => {
            let digits = picker.unique_digits(2);
            format!("{}|{}", digits[0], digits[1])
        }
        ThreeGroupSix | FiveFrontGroupSix | FiveMiddleGroupSix | FiveBackGroupSix => {
            join_digits(&picker.unique_digits(3))
        }
        ThreeGroupSixBanker
        | FiveFrontGroupSixBanker
        | FiveMiddleGroupSixBanker
        | FiveBackGroupSixBanker => {
            let digits = picker.unique_digits(3);
            format!("{}|{},{}", digits[0], digits[1], digits[2])
        }
        FiveBigSmallOddEven => {
            format!("tens:{}|ones:{}", picker.attribute(), picker.attribute())
        }
    }
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
        let mut state = 0xcbf2_9ce4_8422_2325_u64;
        let rule_text = match enum_to_string(rule_code) {
            Ok(value) => value,
            Err(_) => format!("{rule_code:?}"),
        };
        let play_index_text = play_index.to_string();
        for part in [
            robot.id.as_str(),
            lottery.id.as_str(),
            issue.issue.as_str(),
            rule_text.as_str(),
            play_index_text.as_str(),
        ] {
            mix_seed_part(&mut state, part);
        }
        if state == 0 {
            state = 0x9e37_79b9_7f4a_7c15;
        }
        Self { state }
    }

    /// 生成 0-9 的单个数字。
    fn digit(&mut self) -> u8 {
        self.next_index(10) as u8
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

    /// 返回大小单双属性文本，使用后端解析器支持的英文标准值。
    fn attribute(&mut self) -> &'static str {
        const ATTRIBUTES: [&str; 4] = ["big", "small", "odd", "even"];
        ATTRIBUTES[self.next_index(ATTRIBUTES.len())]
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

/// 机器人账户余额不足时自动授信补足，并返回授信流水供后台实时事件广播。
async fn ensure_robot_balance(
    finance: &FinanceRepository,
    required_amount_minor: i64,
    reason: &str,
) -> ApiResult<Option<LedgerEntry>> {
    if required_amount_minor <= 0 {
        return Err(ApiError::BadRequest("机器人授信金额必须大于 0".to_string()));
    }

    let account = finance.account_or_create(ROBOT_GROUP_BUY_USER_ID).await?;
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
            user_id: ROBOT_GROUP_BUY_USER_ID.to_string(),
            amount_minor: top_up_amount_minor,
            description: format!("机器人账户自动授信补余额：{reason}"),
        })
        .await?;

    Ok(Some(entry))
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
fn next_robot_fill_participant_id(plan: &GroupBuyPlan) -> String {
    let mut index = 1;
    loop {
        let participant_id = format!("{}-{}-{:03}", plan.id, ROBOT_FILL_PARTICIPANT_SUFFIX, index);
        if !plan
            .participants
            .iter()
            .any(|participant| participant.id == participant_id)
        {
            return participant_id;
        }
        index += 1;
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

/// 按临近封盘的阶段目标计算本轮机器人应补金额。
fn robot_fill_decision(
    plan: &GroupBuyPlan,
    issue: &DrawIssue,
    now_at: NaiveDateTime,
) -> ApiResult<RobotFillDecision> {
    let remaining_amount_minor = plan
        .total_amount_minor
        .checked_sub(plan.filled_amount_minor)
        .ok_or_else(|| ApiError::BadRequest("合买剩余金额无效".to_string()))?;
    if remaining_amount_minor <= 0 {
        return Ok(RobotFillDecision::Skip("合买计划已满单".to_string()));
    }

    let sale_closed_at = parse_robot_timestamp(&issue.sale_closed_at, "封盘时间")?;
    let seconds_until_sale_close = (sale_closed_at - now_at).num_seconds();
    if seconds_until_sale_close <= 0 {
        return Ok(RobotFillDecision::Skip(
            "已到封盘时间，机器人不再补单".to_string(),
        ));
    }
    if seconds_until_sale_close > ROBOT_FILL_WINDOW_SECONDS {
        return Ok(RobotFillDecision::Skip(format!(
            "未到合买机器人补单窗口，距离封盘还有 {seconds_until_sale_close} 秒"
        )));
    }

    let (target_percent, stage_label) = robot_fill_stage(seconds_until_sale_close);
    let target_amount = target_fill_amount(plan, target_percent)?;
    if target_amount <= plan.filled_amount_minor {
        return Ok(RobotFillDecision::Skip(format!(
            "当前合买进度已达到机器人{stage_label}节奏目标 {target_percent}%"
        )));
    }

    let mut amount_minor = target_amount
        .checked_sub(plan.filled_amount_minor)
        .ok_or_else(|| ApiError::BadRequest("机器人补单金额无效".to_string()))?;
    amount_minor = amount_minor.min(remaining_amount_minor);
    amount_minor = round_down_to_multiple(amount_minor, plan.min_share_amount_minor.max(1));
    if amount_minor <= 0 {
        return Ok(RobotFillDecision::Skip(
            "本轮机器人补单金额小于最小份额，等待下一阶段".to_string(),
        ));
    }

    let participant_min = plan
        .participant_min_amount_minor
        .max(plan.min_share_amount_minor)
        .max(1);
    if amount_minor < participant_min {
        return Ok(RobotFillDecision::Skip(format!(
            "本轮机器人补单金额低于参与最低金额 {participant_min}，等待下一阶段"
        )));
    }

    let remaining_after = remaining_amount_minor
        .checked_sub(amount_minor)
        .ok_or_else(|| ApiError::BadRequest("机器人补单后剩余金额无效".to_string()))?;
    if target_percent < ROBOT_FILL_FINAL_TARGET_PERCENT
        && remaining_after > 0
        && remaining_after < participant_min
    {
        return Ok(RobotFillDecision::Skip(
            "本轮补单会导致剩余金额低于参与最低金额，等待最终阶段".to_string(),
        ));
    }

    Ok(RobotFillDecision::Add(RobotFillAmount {
        amount_minor,
        note: format!("合买机器人{stage_label}节奏补单"),
    }))
}

/// 根据距离封盘秒数返回机器人当前阶段目标。
fn robot_fill_stage(seconds_until_sale_close: i64) -> (i64, &'static str) {
    if seconds_until_sale_close > ROBOT_FILL_STAGE_ONE_SECONDS {
        (ROBOT_FILL_STAGE_ONE_TARGET_PERCENT, "第一阶段")
    } else if seconds_until_sale_close > ROBOT_FILL_STAGE_TWO_SECONDS {
        (ROBOT_FILL_STAGE_TWO_TARGET_PERCENT, "第二阶段")
    } else if seconds_until_sale_close > ROBOT_FILL_FINAL_STAGE_SECONDS {
        (ROBOT_FILL_STAGE_THREE_TARGET_PERCENT, "第三阶段")
    } else {
        (ROBOT_FILL_FINAL_TARGET_PERCENT, "临近封盘")
    }
}

/// 计算当前阶段应达到的认购金额，按最小份额向下取整。
fn target_fill_amount(plan: &GroupBuyPlan, target_percent: i64) -> ApiResult<i64> {
    let raw = plan
        .total_amount_minor
        .checked_mul(target_percent)
        .ok_or_else(|| ApiError::BadRequest("机器人节奏目标金额过大".to_string()))?
        / 100;
    Ok(round_down_to_multiple(
        raw.min(plan.total_amount_minor),
        plan.min_share_amount_minor.max(1),
    ))
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
            draw::DrawRepository, finance::FinanceRepository, lottery::LotteryRepository,
            play_rules::expanded_bets_for_rule, robot::RobotRepository,
        },
    };

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

    #[test]
    fn robot_amounts_keep_remaining_participation_valid() {
        let lottery = robot_test_lottery();
        let (total, initiator) =
            robot_group_buy_amounts(&lottery, 1).expect("amount can calculate");

        assert_eq!(total % lottery.group_buy.min_share_amount_minor, 0);
        assert!(initiator >= lottery.group_buy.participant_min_amount_minor);
        assert!(total - initiator >= lottery.group_buy.participant_min_amount_minor);
        assert!(
            total >= lottery.group_buy.participant_min_amount_minor * ROBOT_FILL_STAGE_COUNT,
            "机器人发起金额需要支持多阶段补单"
        );
    }

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
        assert_eq!(before_window_run.ledger_entries.len(), 1);
        assert!(before_window_run
            .skipped_items
            .iter()
            .any(|item| item.reason.contains("未到合买机器人补单窗口")));
        let plan_id = before_window_run.created_plans[0].id.clone();
        assert_robot_plan_progress(&group_buys, &plan_id, 5_000, 1_000, 1, false).await;

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
        assert_eq!(stage_one_run.ledger_entries.len(), 1);
        assert_robot_plan_progress(&group_buys, &plan_id, 5_000, 2_000, 2, false).await;

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
        assert_eq!(stage_two_run.ledger_entries.len(), 1);
        assert_robot_plan_progress(&group_buys, &plan_id, 5_000, 3_000, 3, false).await;

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
        assert_eq!(stage_three_run.ledger_entries.len(), 1);
        assert_robot_plan_progress(&group_buys, &plan_id, 5_000, 4_000, 4, false).await;

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

        assert_eq!(final_stage_run.filled_plans.len(), 1);
        assert_eq!(final_stage_run.created_orders.len(), 1);
        assert_eq!(final_stage_run.ledger_entries.len(), 1);
        assert_eq!(
            final_stage_run.created_orders[0].order_source,
            OrderSource::GroupBuy
        );
        assert_eq!(
            final_stage_run.filled_plans[0].order_id,
            Some(final_stage_run.created_orders[0].id.clone())
        );
        assert_robot_plan_progress(&group_buys, &plan_id, 5_000, 5_000, 5, true).await;
    }

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
        .expect("robot can run before fill window");
        assert!(before_window_run
            .skipped_items
            .iter()
            .any(|item| item.reason.contains("未到合买机器人补单窗口")));
        assert_robot_plan_progress(&group_buys, "G-USER-OPEN", 5_000, 1_000, 1, false).await;

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
        assert_robot_plan_progress(&group_buys, "G-USER-OPEN", 5_000, 2_000, 2, false).await;

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
        assert_robot_plan_progress(&group_buys, "G-USER-OPEN", 5_000, 3_000, 3, false).await;

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
        assert_robot_plan_progress(&group_buys, "G-USER-OPEN", 5_000, 4_000, 4, false).await;

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

    async fn assert_robot_plan_progress(
        group_buys: &GroupBuyRepository,
        plan_id: &str,
        expected_total: i64,
        expected_filled: i64,
        expected_participants: usize,
        should_be_filled: bool,
    ) {
        let plan = group_buys
            .get(plan_id)
            .await
            .expect("group buy plan exists");
        assert_eq!(plan.total_amount_minor, expected_total);
        assert_eq!(plan.filled_amount_minor, expected_filled);
        assert_eq!(plan.participants.len(), expected_participants);
        if should_be_filled {
            assert!(matches!(plan.status, GroupBuyPlanStatus::Filled));
            assert!(plan.order_id.is_some());
        } else {
            assert!(matches!(plan.status, GroupBuyPlanStatus::Open));
            assert!(plan.order_id.is_none());
        }
    }

    fn robot_test_lottery() -> LotteryKind {
        LotteryKind {
            id: "robot-test".to_string(),
            name: "机器人测试彩".to_string(),
            category: "test".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: crate::domain::lottery::DrawMode::Platform,
            api_draw_delay_seconds: 0,
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

    fn robot_test_config() -> RobotConfigSummary {
        RobotConfigSummary {
            id: "R-BUY-TEST".to_string(),
            name: "测试合买机器人".to_string(),
            kind: RobotKind::GroupBuy,
            lottery_ids: vec!["robot-test".to_string()],
            status: RobotStatus::Enabled,
            description: "测试机器人".to_string(),
            deletable: true,
        }
    }

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
