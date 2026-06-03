//! 后台 HTTP 路由汇总入口，挂载 health 与 admin 子路由。

mod admin;
mod health;

use axum::Router;

use crate::app::AppState;

/// 组装并返回当前模块对应的路由树。
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .nest("/admin", admin::router(state))
}
