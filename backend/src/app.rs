use std::sync::{Arc, RwLock};

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::{routes, services::lottery::LotteryStore};

#[derive(Clone)]
pub struct AppState {
    pub lotteries: Arc<RwLock<LotteryStore>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            lotteries: Arc::new(RwLock::new(LotteryStore::seeded())),
        }
    }
}

pub fn router() -> Router {
    let state = AppState::new();

    Router::new()
        .nest("/api", routes::router())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
