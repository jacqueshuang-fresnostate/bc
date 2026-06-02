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
        play::{PlayRuleEvaluateRequest, PlayRuleEvaluation, PlayRuleSummary},
        settlement::SettlementRun,
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
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(get_dashboard_summary))
        .route("/financial-accounts", get(list_financial_accounts))
        .route("/ledger-entries", get(list_ledger_entries))
        .route("/financial-adjustments", post(manual_balance_adjustment))
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

    Ok(Json(ApiEnvelope::success(dashboard_summary_with_orders(
        lotteries,
        recent_orders,
        finance,
        financial_accounts,
    ))))
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
