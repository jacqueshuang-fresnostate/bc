//! 提供服务健康检查路由，供运维与联调探测服务状态

use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::{app::AppState, error::ApiResult, response::ApiEnvelope};

/// 组装并返回当前模块对应的路由树。
pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

/// 健康检查接口，用于确认后端服务和版本信息。
async fn health() -> ApiResult<Json<ApiEnvelope<HealthResponse>>> {
    Ok(Json(ApiEnvelope::success(HealthResponse {
        service: "bc-backend".to_string(),
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 健康检查响应数据。
struct HealthResponse {
    service: String,
    status: String,
    version: String,
}
