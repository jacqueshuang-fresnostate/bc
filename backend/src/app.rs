use std::error::Error;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::{
    routes,
    services::{
        draw::DrawRepository,
        finance::FinanceRepository,
        lottery::LotteryRepository,
        order::OrderRepository,
        scheduler::{spawn_draw_scheduler, DrawSchedulerConfig},
    },
};

#[derive(Clone)]
pub struct AppState {
    pub draws: DrawRepository,
    pub finance: FinanceRepository,
    pub lotteries: LotteryRepository,
    pub orders: OrderRepository,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            draws: DrawRepository::memory(),
            finance: FinanceRepository::memory_seeded(),
            lotteries: LotteryRepository::memory_seeded(),
            orders: OrderRepository::memory(),
        }
    }

    pub async fn from_env() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            tracing::info!("DATABASE_URL not configured; using in-memory lottery repository");
            return Ok(Self::new());
        };

        let lotteries = LotteryRepository::postgres(&database_url).await?;

        tracing::info!("DATABASE_URL configured; using PostgreSQL lottery repository");
        Ok(Self {
            draws: DrawRepository::memory(),
            finance: FinanceRepository::memory_seeded(),
            lotteries,
            orders: OrderRepository::memory(),
        })
    }
}

pub async fn router_from_env() -> Result<Router, Box<dyn Error + Send + Sync>> {
    let state = AppState::from_env().await?;
    let scheduler_config = DrawSchedulerConfig::from_env()?;
    spawn_draw_scheduler(
        state.draws.clone(),
        state.lotteries.clone(),
        state.orders.clone(),
        state.finance.clone(),
        scheduler_config,
    );

    Ok(router_with_state(state))
}

fn router_with_state(state: AppState) -> Router {
    Router::new()
        .nest("/api", routes::router())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
