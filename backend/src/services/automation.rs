use crate::{
    domain::{
        draw::{
            DrawAutomationRun, DrawAutomationRunRequest, DrawAutomationSkippedIssue, DrawIssue,
            DrawIssueResultRequest, DrawIssueStatus,
        },
        lottery::DrawMode,
    },
    error::{ApiError, ApiResult},
    services::{draw::DrawRepository, finance::FinanceRepository, order::OrderRepository},
};

pub async fn run_draw_automation(
    draws: &DrawRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
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

    for issue in draws.list().await? {
        if should_close(&issue, &now) {
            let closed = draws.close(&issue.id).await?;
            run.closed_issues.push(closed);
        }
    }

    for issue in draws.list().await? {
        if !should_draw(&issue, &now) {
            continue;
        }
        if issue.draw_mode == DrawMode::Manual
            && !draws.has_active_draw_control(&issue.lottery_id).await?
        {
            run.skipped_issues.push(skipped_issue(
                &issue,
                "manual draw requires administrator draw number",
            ));
            continue;
        }

        let drawn = match draws
            .draw(&issue.id, DrawIssueResultRequest::default())
            .await
        {
            Ok(drawn) => drawn,
            Err(error) => {
                tracing::warn!(
                    draw_issue_id = %issue.id,
                    lottery_id = %issue.lottery_id,
                    issue = %issue.issue,
                    error = %error.log_message(),
                    "自动开奖因开奖失败跳过期号"
                );
                run.skipped_issues
                    .push(skipped_issue(&issue, &error.to_string()));
                continue;
            }
        };
        let settlement = orders.settle_draw_issue(&drawn).await?;
        let entries = finance.credit_settlement(&settlement).await?;

        run.drawn_issues.push(drawn);
        run.settlement_runs.push(settlement);
        run.ledger_entries.extend(entries);
    }

    Ok(run)
}

fn should_close(issue: &DrawIssue, now: &str) -> bool {
    issue.status == DrawIssueStatus::Open && is_due_at(&issue.sale_closed_at, now)
}

fn should_draw(issue: &DrawIssue, now: &str) -> bool {
    matches!(
        issue.status,
        DrawIssueStatus::Open | DrawIssueStatus::Closed
    ) && is_due_at(&issue.scheduled_at, now)
}

fn is_due_at(value: &str, now: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && value <= now
}

fn skipped_issue(issue: &DrawIssue, reason: &str) -> DrawAutomationSkippedIssue {
    DrawAutomationSkippedIssue {
        draw_issue_id: issue.id.clone(),
        lottery_id: issue.lottery_id.clone(),
        issue: issue.issue.clone(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            draw::{
                CreateDrawIssueRequest, DrawAutomationRunRequest, DrawIssueStatus,
                SaveLotteryDrawControlRequest,
            },
            lottery::{
                DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType,
                LotteryPlayConfig, PlayCategory,
            },
            order::CreateOrderRequest,
            play::{PlayRuleCode, PlaySelection},
        },
        services::{
            automation::run_draw_automation, draw::DrawRepository,
            draw_api::ApiDrawSourceRepository, finance::FinanceRepository, order::OrderRepository,
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
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
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
            &orders,
            &finance,
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
    async fn automation_skips_due_manual_issue_without_draw_number() {
        let draws = DrawRepository::memory();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let lottery = lottery(DrawMode::Manual);
        let issue = draws
            .create(&lottery, create_request("MANUAL20260602001"))
            .await
            .expect("issue can be created");

        let run = run_draw_automation(
            &draws,
            &orders,
            &finance,
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
        assert!(run.skipped_issues[0]
            .reason
            .contains("administrator draw number"));
    }

    #[tokio::test]
    async fn automation_draws_due_manual_issue_with_control_number() {
        let draws = DrawRepository::memory();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let lottery = lottery(DrawMode::Manual);
        draws
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
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
            &orders,
            &finance,
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
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let lottery = lottery(DrawMode::Api);
        let issue = draws
            .create(&lottery, create_request("2099999"))
            .await
            .expect("issue can be created");

        let run = run_draw_automation(
            &draws,
            &orders,
            &finance,
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
        assert!(run.skipped_issues[0].reason.contains("not found"));
    }

    fn create_request(issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: "fc3d".to_string(),
            issue: issue.to_string(),
            scheduled_at: "2026-06-02 21:00:15".to_string(),
            sale_closed_at: "2026-06-02 20:59:45".to_string(),
        }
    }

    fn lottery(draw_mode: DrawMode) -> LotteryKind {
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
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
            }],
        }
    }

    fn full_direct_selection() -> PlaySelection {
        let all_digits = (0..=9).collect::<Vec<_>>();
        PlaySelection {
            positions: vec![all_digits.clone(), all_digits.clone(), all_digits],
            ..PlaySelection::default()
        }
    }
}
