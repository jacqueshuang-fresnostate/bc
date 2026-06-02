use axum::{
    extract::{Path, State},
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    domain::{
        lottery::LotteryKind,
        play::{PlayRuleEvaluateRequest, PlayRuleEvaluation, PlayRuleSummary},
    },
    error::ApiResult,
    response::ApiEnvelope,
    services::{
        dashboard::{dashboard_summary, DashboardSummary},
        play_rules::{evaluate_play_rule, play_rule_summaries},
    },
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(get_dashboard_summary))
        .route("/play-rules", get(list_play_rules))
        .route("/play-rules/evaluate", post(evaluate_play_rule_request))
        .route("/lotteries", get(list_lotteries).post(create_lottery))
        .route(
            "/lotteries/{id}",
            get(get_lottery).put(update_lottery).delete(delete_lottery),
        )
        .route("/lotteries/{id}/sale", patch(set_lottery_sale))
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

    Ok(Json(ApiEnvelope::success(dashboard_summary(lotteries))))
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
