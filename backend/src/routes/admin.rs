use axum::{routing::get, Json, Router};

use crate::{
    error::ApiResult,
    response::ApiEnvelope,
    services::dashboard::{dashboard_summary, DashboardSummary},
};

pub fn router() -> Router {
    Router::new().route("/dashboard", get(get_dashboard_summary))
}

async fn get_dashboard_summary() -> ApiResult<Json<ApiEnvelope<DashboardSummary>>> {
    Ok(Json(ApiEnvelope::success(dashboard_summary())))
}
