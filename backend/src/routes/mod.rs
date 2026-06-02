mod admin;
mod health;

use axum::Router;

use crate::app::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .nest("/admin", admin::router(state))
}
