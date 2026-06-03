use std::error::Error;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::{
    routes,
    services::{
        access::AccessRepository,
        draw::DrawRepository,
        draw_api::ApiDrawSourceRepository,
        finance::FinanceRepository,
        group_buy::GroupBuyRepository,
        invite::InviteRepository,
        lottery::LotteryRepository,
        order::OrderRepository,
        rebate::RebateRepository,
        robot::RobotRepository,
        scheduler::{spawn_draw_scheduler, DrawSchedulerConfig, DrawSchedulerRepository},
        support::SupportRepository,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub access: AccessRepository,
    pub draws: DrawRepository,
    pub finance: FinanceRepository,
    pub group_buys: GroupBuyRepository,
    pub invites: InviteRepository,
    pub lotteries: LotteryRepository,
    pub orders: OrderRepository,
    pub rebates: RebateRepository,
    pub robots: RobotRepository,
    pub scheduler: DrawSchedulerRepository,
    pub support: SupportRepository,
}

impl AppState {
    fn new_with_scheduler(scheduler: DrawSchedulerRepository) -> Self {
        Self {
            access: AccessRepository::memory_seeded(),
            draws: default_draw_repository(),
            finance: FinanceRepository::memory_seeded(),
            group_buys: GroupBuyRepository::memory_seeded(),
            invites: InviteRepository::memory_seeded(),
            lotteries: LotteryRepository::memory_seeded(),
            orders: OrderRepository::memory(),
            rebates: RebateRepository::memory_seeded(),
            robots: RobotRepository::memory_seeded(),
            scheduler,
            support: SupportRepository::memory_seeded(),
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
            draws: default_draw_repository(),
            finance: FinanceRepository::memory_seeded(),
            group_buys: GroupBuyRepository::memory_seeded(),
            invites: InviteRepository::memory_seeded(),
            lotteries,
            orders: OrderRepository::memory(),
            rebates: RebateRepository::memory_seeded(),
            robots: RobotRepository::memory_seeded(),
            scheduler,
            support: SupportRepository::memory_seeded(),
        })
    }
}

fn default_draw_repository() -> DrawRepository {
    DrawRepository::memory_with_api_sources(ApiDrawSourceRepository::api68_seeded())
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
        .nest("/api", routes::router(state.clone()))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
