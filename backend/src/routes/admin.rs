use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    domain::{
        draw::{
            CreateDrawIssueRequest, DrawAutomationRun, DrawAutomationRunRequest, DrawIssue,
            DrawIssueGenerationPreview, DrawIssueResultRequest, GenerateDrawIssueRequest,
            GenerateDrawIssuesRequest,
        },
        finance::{FinancialAccountSummary, LedgerEntry, ManualBalanceAdjustmentRequest},
        lottery::{DrawSource, LotteryKind},
        order::{CreateOrderRequest, OrderDetail},
        permission::{AdminRole, SystemSetting, UpdateSystemSettingRequest},
        play::{PlayRuleEvaluateRequest, PlayRuleEvaluation, PlayRuleSummary},
        robot::{RobotConfigSummary, RobotStatusRequest},
        settlement::SettlementRun,
        user::{
            AdminStatusRequest, AdminSummary, RegistrationConfig, UserStatusRequest, UserSummary,
        },
    },
    error::ApiResult,
    response::ApiEnvelope,
    services::{
        automation::run_draw_automation,
        dashboard::{dashboard_summary_with_orders, draw_sources, DashboardSummary},
        draw_generation::{
            generate_draw_issue_batch, generate_next_draw_issue, preview_draw_issue_generation,
        },
        order::validate_draw_issue_accepts_order,
        play_rules::{evaluate_play_rule, play_rule_summaries},
        scheduler::DrawSchedulerStatus,
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(get_dashboard_summary))
        .route("/financial-accounts", get(list_financial_accounts))
        .route("/ledger-entries", get(list_ledger_entries))
        .route("/financial-adjustments", post(manual_balance_adjustment))
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", get(get_user).put(update_user))
        .route("/users/{id}/status", patch(set_user_status))
        .route("/admins", get(list_admins).post(create_admin))
        .route("/admins/{id}", get(get_admin).put(update_admin))
        .route("/admins/{id}/status", patch(set_admin_status))
        .route("/roles", get(list_roles).post(create_role))
        .route(
            "/roles/{id}",
            get(get_role).put(update_role).delete(delete_role),
        )
        .route("/system-settings", get(list_system_settings))
        .route("/system-settings/{key}", patch(update_system_setting))
        .route(
            "/registration",
            get(get_registration_config).put(update_registration_config),
        )
        .route("/robots", get(list_robots).post(create_robot))
        .route(
            "/robots/{id}",
            get(get_robot).put(update_robot).delete(delete_robot),
        )
        .route("/robots/{id}/status", patch(set_robot_status))
        .route("/draw-sources", get(list_draw_sources))
        .route(
            "/draw-issues",
            get(list_draw_issues).post(create_draw_issue),
        )
        .route(
            "/draw-issues/generate-next",
            post(generate_next_draw_issue_request),
        )
        .route(
            "/draw-issues/preview-generation",
            post(preview_draw_issue_generation_request),
        )
        .route(
            "/draw-issues/generate-batch",
            post(generate_draw_issue_batch_request),
        )
        .route("/draw-issues/{id}", get(get_draw_issue))
        .route("/draw-issues/{id}/close", patch(close_draw_issue))
        .route("/draw-issues/{id}/draw", patch(draw_issue_result))
        .route("/draw-issues/{id}/cancel", patch(cancel_draw_issue))
        .route("/draw-scheduler/status", get(get_draw_scheduler_status))
        .route("/draw-automation/run", post(run_draw_automation_request))
        .route("/settlements", get(list_settlements))
        .route("/settlements/{id}", get(get_settlement))
        .route(
            "/settlements/draw-issues/{id}",
            post(settle_draw_issue_orders),
        )
        .route("/play-rules", get(list_play_rules))
        .route("/play-rules/evaluate", post(evaluate_play_rule_request))
        .route("/orders", get(list_orders).post(create_order))
        .route("/orders/{id}", get(get_order))
        .route("/orders/{id}/cancel", patch(cancel_order))
        .route("/lotteries", get(list_lotteries).post(create_lottery))
        .route(
            "/lotteries/{id}",
            get(get_lottery).put(update_lottery).delete(delete_lottery),
        )
        .route("/lotteries/{id}/sale", patch(set_lottery_sale))
}

async fn run_draw_automation_request(
    State(state): State<AppState>,
    Json(payload): Json<DrawAutomationRunRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawAutomationRun>>> {
    let run = run_draw_automation(&state.draws, &state.orders, &state.finance, payload).await?;

    Ok(Json(ApiEnvelope::success(run)))
}

async fn get_draw_scheduler_status(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<DrawSchedulerStatus>>> {
    let status = state.scheduler.status()?;

    Ok(Json(ApiEnvelope::success(status)))
}

async fn list_draw_sources() -> ApiResult<Json<ApiEnvelope<Vec<DrawSource>>>> {
    Ok(Json(ApiEnvelope::success(draw_sources())))
}

async fn list_draw_issues(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawIssue>>>> {
    let issues = state.draws.list().await?;

    Ok(Json(ApiEnvelope::success(issues)))
}

async fn get_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.get(&id).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn create_draw_issue(
    State(state): State<AppState>,
    Json(payload): Json<CreateDrawIssueRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issue = state.draws.create(&lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn generate_next_draw_issue_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssueRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issue = generate_next_draw_issue(&state.draws, &lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn preview_draw_issue_generation_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssuesRequest>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawIssueGenerationPreview>>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let plans = preview_draw_issue_generation(&state.draws, &lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(plans)))
}

async fn generate_draw_issue_batch_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssuesRequest>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawIssue>>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issues = generate_draw_issue_batch(&state.draws, &lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(issues)))
}

async fn close_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.close(&id).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn draw_issue_result(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DrawIssueResultRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.draw(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn cancel_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.cancel(&id).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn list_settlements(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<SettlementRun>>>> {
    let settlements = state.orders.settlement_runs().await?;

    Ok(Json(ApiEnvelope::success(settlements)))
}

async fn get_settlement(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SettlementRun>>> {
    let settlement = state.orders.get_settlement(&id).await?;

    Ok(Json(ApiEnvelope::success(settlement)))
}

async fn settle_draw_issue_orders(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SettlementRun>>> {
    let draw_issue = state.draws.get(&id).await?;
    let settlement = state.orders.settle_draw_issue(&draw_issue).await?;
    state.finance.credit_settlement(&settlement).await?;

    Ok(Json(ApiEnvelope::success(settlement)))
}

async fn list_play_rules() -> ApiResult<Json<ApiEnvelope<Vec<PlayRuleSummary>>>> {
    Ok(Json(ApiEnvelope::success(play_rule_summaries())))
}

async fn evaluate_play_rule_request(
    Json(payload): Json<PlayRuleEvaluateRequest>,
) -> ApiResult<Json<ApiEnvelope<PlayRuleEvaluation>>> {
    Ok(Json(ApiEnvelope::success(evaluate_play_rule(payload)?)))
}

async fn get_dashboard_summary(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<DashboardSummary>>> {
    let lotteries = state.lotteries.list().await?;
    let recent_orders = state.orders.recent_summaries(8).await?;
    let finance = state.finance.overview().await?;
    let financial_accounts = state.finance.accounts().await?;
    let access = state.access.snapshot().await?;
    let robots = state.robots.list().await?;

    Ok(Json(ApiEnvelope::success(dashboard_summary_with_orders(
        lotteries,
        recent_orders,
        finance,
        financial_accounts,
        access,
        robots,
    ))))
}

async fn list_users(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<UserSummary>>>> {
    let users = state.access.users().await?;

    Ok(Json(ApiEnvelope::success(users)))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.get_user(&id).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<UserSummary>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.create_user(payload).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UserSummary>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.update_user(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

async fn set_user_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UserStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.set_user_status(&id, payload.status).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

async fn list_admins(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<AdminSummary>>>> {
    let admins = state.access.admins().await?;

    Ok(Json(ApiEnvelope::success(admins)))
}

async fn get_admin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.get_admin(&id).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn create_admin(
    State(state): State<AppState>,
    Json(payload): Json<AdminSummary>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.create_admin(payload).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn update_admin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminSummary>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.update_admin(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn set_admin_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.set_admin_status(&id, payload.status).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn list_roles(State(state): State<AppState>) -> ApiResult<Json<ApiEnvelope<Vec<AdminRole>>>> {
    let roles = state.access.roles().await?;

    Ok(Json(ApiEnvelope::success(roles)))
}

async fn get_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.get_role(&id).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

async fn create_role(
    State(state): State<AppState>,
    Json(payload): Json<AdminRole>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.create_role(payload).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminRole>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.update_role(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.delete_role(&id).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

async fn list_system_settings(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<SystemSetting>>>> {
    let settings = state.access.settings().await?;

    Ok(Json(ApiEnvelope::success(settings)))
}

async fn update_system_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<UpdateSystemSettingRequest>,
) -> ApiResult<Json<ApiEnvelope<SystemSetting>>> {
    let setting = state.access.update_setting(&key, payload).await?;

    Ok(Json(ApiEnvelope::success(setting)))
}

async fn get_registration_config(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<RegistrationConfig>>> {
    let registration = state.access.registration().await?;

    Ok(Json(ApiEnvelope::success(registration)))
}

async fn update_registration_config(
    State(state): State<AppState>,
    Json(payload): Json<RegistrationConfig>,
) -> ApiResult<Json<ApiEnvelope<RegistrationConfig>>> {
    let registration = state.access.update_registration(payload).await?;

    Ok(Json(ApiEnvelope::success(registration)))
}

async fn list_robots(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<RobotConfigSummary>>>> {
    let robots = state.robots.list().await?;

    Ok(Json(ApiEnvelope::success(robots)))
}

async fn get_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.get(&id).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn create_robot(
    State(state): State<AppState>,
    Json(payload): Json<RobotConfigSummary>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let lotteries = state.lotteries.list().await?;
    let robot = state.robots.create(payload, &lotteries).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn update_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RobotConfigSummary>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let lotteries = state.lotteries.list().await?;
    let robot = state.robots.update(&id, payload, &lotteries).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn delete_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.delete(&id).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn set_robot_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RobotStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.set_status(&id, payload.status).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn list_financial_accounts(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<FinancialAccountSummary>>>> {
    let accounts = state.finance.accounts().await?;

    Ok(Json(ApiEnvelope::success(accounts)))
}

async fn list_ledger_entries(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LedgerEntry>>>> {
    let entries = state.finance.ledger_entries().await?;

    Ok(Json(ApiEnvelope::success(entries)))
}

async fn manual_balance_adjustment(
    State(state): State<AppState>,
    Json(payload): Json<ManualBalanceAdjustmentRequest>,
) -> ApiResult<Json<ApiEnvelope<LedgerEntry>>> {
    let entry = state.finance.manual_adjust(payload).await?;

    Ok(Json(ApiEnvelope::success(entry)))
}

async fn list_orders(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<OrderDetail>>>> {
    let orders = state.orders.list().await?;

    Ok(Json(ApiEnvelope::success(orders)))
}

async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<OrderDetail>>> {
    let order = state.orders.get(&id).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

async fn create_order(
    State(state): State<AppState>,
    Json(payload): Json<CreateOrderRequest>,
) -> ApiResult<Json<ApiEnvelope<OrderDetail>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let draw_issue = state
        .draws
        .get_by_lottery_issue(&payload.lottery_id, &payload.issue)
        .await?;
    validate_draw_issue_accepts_order(&draw_issue, &lottery, &payload.issue)?;
    let quote = state.orders.quote(&lottery, &payload).await?;
    state
        .finance
        .ensure_available(&payload.user_id, quote.amount_minor)
        .await?;
    let order = state.orders.create(&lottery, payload).await?;
    if let Err(error) = state.finance.debit_order(&order).await {
        if let Err(rollback_error) = state.orders.remove_unfunded(&order.id).await {
            tracing::error!(
                order_id = %order.id,
                error = %rollback_error,
                "failed to remove unfunded order after debit failure"
            );
        }
        return Err(error);
    }

    Ok(Json(ApiEnvelope::success(order)))
}

async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<OrderDetail>>> {
    let existing = state.orders.get(&id).await?;
    state.finance.ensure_order_can_refund(&existing).await?;
    let order = state.orders.cancel(&id).await?;
    state.finance.refund_order(&order).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

async fn list_lotteries(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LotteryKind>>>> {
    let lotteries = state.lotteries.list().await?;

    Ok(Json(ApiEnvelope::success(lotteries)))
}

async fn get_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.get(&id).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn create_lottery(
    State(state): State<AppState>,
    Json(payload): Json<LotteryKind>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.create(payload).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn update_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<LotteryKind>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.update(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn delete_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.delete(&id).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn set_lottery_sale(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SaleStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state
        .lotteries
        .set_sale_enabled(&id, payload.sale_enabled)
        .await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaleStatusRequest {
    sale_enabled: bool,
}
