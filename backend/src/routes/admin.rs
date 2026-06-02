use axum::{
    extract::{Path, State},
    routing::{get, patch},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    app::AppState,
    domain::lottery::LotteryKind,
    error::{ApiError, ApiResult},
    response::ApiEnvelope,
    services::dashboard::{dashboard_summary, DashboardSummary},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(get_dashboard_summary))
        .route("/lotteries", get(list_lotteries).post(create_lottery))
        .route(
            "/lotteries/{id}",
            get(get_lottery).put(update_lottery).delete(delete_lottery),
        )
        .route("/lotteries/{id}/sale", patch(set_lottery_sale))
}

async fn get_dashboard_summary(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<DashboardSummary>>> {
    let lotteries = state
        .lotteries
        .read()
        .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
        .list();

    Ok(Json(ApiEnvelope::success(dashboard_summary(lotteries))))
}

async fn list_lotteries(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LotteryKind>>>> {
    let lotteries = state
        .lotteries
        .read()
        .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
        .list();

    Ok(Json(ApiEnvelope::success(lotteries)))
}

async fn get_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state
        .lotteries
        .read()
        .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
        .get(&id)?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn create_lottery(
    State(state): State<AppState>,
    Json(payload): Json<LotteryKind>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state
        .lotteries
        .write()
        .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
        .create(payload)?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn update_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<LotteryKind>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state
        .lotteries
        .write()
        .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
        .update(&id, payload)?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn delete_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state
        .lotteries
        .write()
        .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
        .delete(&id)?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn set_lottery_sale(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SaleStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state
        .lotteries
        .write()
        .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
        .set_sale_enabled(&id, payload.sale_enabled)?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaleStatusRequest {
    sale_enabled: bool,
}
