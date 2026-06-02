use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::{app::AppState, error::ApiResult, response::ApiEnvelope};

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

async fn health() -> ApiResult<Json<ApiEnvelope<HealthResponse>>> {
    Ok(Json(ApiEnvelope::success(HealthResponse {
        service: "bc-backend".to_string(),
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    service: String,
    status: String,
    version: String,
}
