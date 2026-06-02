use std::error::Error;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::{
    routes,
    services::{
        access::AccessRepository,
        draw::DrawRepository,
        finance::FinanceRepository,
        lottery::LotteryRepository,
        order::OrderRepository,
        robot::RobotRepository,
        scheduler::{spawn_draw_scheduler, DrawSchedulerConfig, DrawSchedulerRepository},
    },
};

#[derive(Clone)]
pub struct AppState {
    pub access: AccessRepository,
    pub draws: DrawRepository,
    pub finance: FinanceRepository,
    pub lotteries: LotteryRepository,
    pub orders: OrderRepository,
    pub robots: RobotRepository,
    pub scheduler: DrawSchedulerRepository,
}

impl AppState {
    fn new_with_scheduler(scheduler: DrawSchedulerRepository) -> Self {
        Self {
            access: AccessRepository::memory_seeded(),
            draws: DrawRepository::memory(),
            finance: FinanceRepository::memory_seeded(),
            lotteries: LotteryRepository::memory_seeded(),
            orders: OrderRepository::memory(),
            robots: RobotRepository::memory_seeded(),
            scheduler,
        }
    }

    pub async fn from_env_with_scheduler(
        scheduler: DrawSchedulerRepository,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            tracing::info!("DATABASE_URL not configured; using in-memory lottery repository");
            return Ok(Self::new_with_scheduler(scheduler));
        };

        let lotteries = LotteryRepository::postgres(&database_url).await?;

        tracing::info!("DATABASE_URL configured; using PostgreSQL lottery repository");
        Ok(Self {
            access: AccessRepository::memory_seeded(),
            draws: DrawRepository::memory(),
            finance: FinanceRepository::memory_seeded(),
            lotteries,
            orders: OrderRepository::memory(),
            robots: RobotRepository::memory_seeded(),
            scheduler,
        })
    }
}

pub async fn router_from_env() -> Result<Router, Box<dyn Error + Send + Sync>> {
    let scheduler_config = DrawSchedulerConfig::from_env()?;
    let scheduler = DrawSchedulerRepository::new(scheduler_config.clone());
    let state = AppState::from_env_with_scheduler(scheduler.clone()).await?;
    spawn_draw_scheduler(
        state.draws.clone(),
        state.lotteries.clone(),
        state.orders.clone(),
        state.finance.clone(),
        scheduler_config,
        scheduler,
    );

    Ok(router_with_state(state))
}

fn router_with_state(state: AppState) -> Router {
    Router::new()
        .nest("/api", routes::router())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
