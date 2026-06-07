//! 开奖自动化服务，统一编排手动/自动开奖执行链路

use std::collections::HashMap;

use crate::{
    domain::{
        draw::{
            DrawAutomationRun, DrawAutomationRunRequest, DrawAutomationSkippedIssue, DrawIssue,
            DrawIssueResultRequest, DrawIssueStatus,
        },
        lottery::DrawMode,
    },
    error::{ApiError, ApiResult},
    services::{
        draw::DrawRepository, finance::FinanceRepository, group_buy::GroupBuyRepository,
        order::OrderRepository,
    },
};

/// 触发自动化开奖一轮任务并返回执行结果。
pub async fn run_draw_automation(
    draws: &DrawRepository,
    lotteries: &crate::services::lottery::LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    payload: DrawAutomationRunRequest,
) -> ApiResult<DrawAutomationRun> {
    let now = payload.now.trim().to_string();
    if now.is_empty() {
        return Err(ApiError::BadRequest(
            "automation time is required".to_string(),
        ));
    }

    let mut run = DrawAutomationRun {
        now: now.clone(),
        closed_issues: Vec::new(),
        drawn_issues: Vec::new(),
        settlement_runs: Vec::new(),
        ledger_entries: Vec::new(),
        skipped_issues: Vec::new(),
    };

    let lottery_sale_status = lottery_sale_status(lotteries).await?;

    for issue in draws.list().await? {
        if let Some(reason) = skip_issue_if_lottery_disabled(&issue, &lottery_sale_status) {
            run.skipped_issues.push(skipped_issue(&issue, &reason));
            continue;
        }

        if should_close(&issue, &now) {
            let closed = draws.close(&issue.id).await?;
            let cancelled_plans = group_buys
                .cancel_unfilled_for_issue(&closed.lottery_id, &closed.issue)
                .await?;
            for plan in cancelled_plans {
                let entries = finance
                    .refund_group_buy_plan(&plan, "封盘未满员流单退款")
                    .await?;
                run.ledger_entries.extend(entries);
            }
            run.closed_issues.push(closed);
        }
    }

    for issue in draws.list().await? {
        if let Some(reason) = skip_issue_if_lottery_disabled(&issue, &lottery_sale_status) {
            if !run
                .skipped_issues
                .iter()
                .any(|skipped| skipped.draw_issue_id == issue.id)
            {
                run.skipped_issues.push(skipped_issue(&issue, &reason));
            }
            continue;
        }

        if !should_draw(&issue, &now) {
            continue;
        }
        if issue.draw_mode == DrawMode::Manual && !draws.has_active_draw_control(&issue).await? {
            run.skipped_issues
                .push(skipped_issue(&issue, "手动开奖需要管理员录入开奖号码"));
            continue;
        }

        let drawn = match draws
            .draw(&issue.id, DrawIssueResultRequest::default())
            .await
        {
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
        let settlement = orders.settle_draw_issue(&drawn).await?;
        let order_ids = settlement
            .orders
            .iter()
            .map(|order| order.order_id.clone())
            .collect::<Vec<_>>();
        let group_buy_plans = group_buys.plans_for_order_ids(&order_ids).await?;
        let entries = finance
            .credit_settlement_with_group_buys(&settlement, &group_buy_plans)
            .await?;
        group_buys.mark_settled_by_order_ids(&order_ids).await?;

        run.drawn_issues.push(drawn);
        run.settlement_runs.push(settlement);
        run.ledger_entries.extend(entries);
    }

    Ok(run)
}

/// 处理 should_close 的具体内部流程。
fn should_close(issue: &DrawIssue, now: &str) -> bool {
    issue.status == DrawIssueStatus::Open && is_due_at(&issue.sale_closed_at, now)
}

/// 处理 should_draw 的具体内部流程。
fn should_draw(issue: &DrawIssue, now: &str) -> bool {
    matches!(
        issue.status,
        DrawIssueStatus::Open | DrawIssueStatus::Closed
    ) && is_due_at(&issue.scheduled_at, now)
}

/// 判断并返回布尔结果。
fn is_due_at(value: &str, now: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value <= now
}

/// 处理 skipped_issue 的具体内部流程。
fn skipped_issue(issue: &DrawIssue, reason: &str) -> DrawAutomationSkippedIssue {
    DrawAutomationSkippedIssue {
        draw_issue_id: issue.id.clone(),
        lottery_id: issue.lottery_id.clone(),
        issue: issue.issue.clone(),
        reason: reason.to_string(),
    }
}

async fn lottery_sale_status(
    lotteries: &crate::services::lottery::LotteryRepository,
) -> ApiResult<HashMap<String, bool>> {
    let mut sale_status = HashMap::new();

    for lottery in lotteries.list().await? {
        sale_status.insert(lottery.id, lottery.sale_enabled);
    }

    Ok(sale_status)
}

/// 处理 skip_issue_if_lottery_disabled 的具体内部流程。
fn skip_issue_if_lottery_disabled(
    issue: &DrawIssue,
    lottery_sale_status: &HashMap<String, bool>,
) -> Option<&'static str> {
    let sale_enabled = match lottery_sale_status.get(&issue.lottery_id) {
        Some(enabled) => *enabled,
        None => {
            return Some("未找到彩种配置，跳过自动任务");
        }
    };

    if sale_enabled {
        None
    } else {
        Some("彩种已停售，跳过自动任务")
    }
}

/// 处理 automation_error_reason 的具体内部流程。
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
                DrawIssueStatus, SaveLotteryDrawControlRequest,
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
            access::AccessRepository, automation::run_draw_automation, draw::DrawRepository,
            draw_api::ApiDrawSourceRepository, finance::FinanceRepository,
            group_buy::GroupBuyRepository, lottery::LotteryRepository, order::OrderRepository,
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
        assert_eq!(account.available_balance_minor, 12_001);
        assert!(stored
            .draw_number
            .as_deref()
            .is_some_and(|number| { number.split(',').count() == 3 }));
    }

    #[tokio::test]
    async fn automation_skips_all_steps_when_lottery_stopped() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        lotteries
            .set_sale_enabled("fc3d", false)
            .await
            .expect("lottery sale can be disabled");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let lottery = lottery(DrawMode::Api);
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

        assert_eq!(run.closed_issues.len(), 0);
        assert!(run.drawn_issues.is_empty());
        assert_eq!(run.skipped_issues.len(), 1);
        assert_eq!(run.skipped_issues[0].lottery_id, "fc3d");
        assert!(run.skipped_issues[0].reason.contains("已停售"));
        assert_eq!(stored.status, DrawIssueStatus::Open);
    }

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
                    target_scope: DrawControlTargetScope::Lottery,
                    target_issue: None,
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

    /// 处理 create_request 的具体内部流程。
    fn create_request(issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: "fc3d".to_string(),
            issue: issue.to_string(),
            scheduled_at: "2026-06-02 21:00:15".to_string(),
            sale_closed_at: "2026-06-02 20:59:45".to_string(),
        }
    }

    /// 处理 lottery 的具体内部流程。
    fn lottery(draw_mode: DrawMode) -> LotteryKind {
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            category: "regional".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode,
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

    /// 为自动开奖测试显式打开指定彩种销售状态。
    async fn enable_lottery_sale(lotteries: &LotteryRepository, lottery_id: &str) {
        lotteries
            .set_sale_enabled(lottery_id, true)
            .await
            .expect("lottery sale can be enabled for automation test");
    }

    /// 处理 full_direct_selection 的具体内部流程。
    fn full_direct_selection() -> PlaySelection {
        let all_digits = (0..=9).collect::<Vec<_>>();
        PlaySelection {
            positions: vec![all_digits.clone(), all_digits.clone(), all_digits],
            ..PlaySelection::default()
        }
    }
}
