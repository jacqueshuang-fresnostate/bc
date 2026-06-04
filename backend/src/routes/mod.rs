//! HTTP 路由汇总入口，挂载 health、admin 与用户接口子路由。

mod admin;
mod health;
mod lottery;
mod openapi;
mod user;

use axum::Router;

use crate::app::AppState;

/// 组装并返回当前模块对应的路由树。
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(openapi::router())
        .nest("/admin", admin::router(state.clone()))
        .nest("/lottery", lottery::router())
        .nest("/user", user::router(state))
}
