//! 后端应用状态和依赖仓储组装入口，统一创建内存/数据库模式下的状态

use std::error::Error;

use axum::Router;
use tower_http::cors::CorsLayer;

use crate::{
    routes,
    services::{
        access::AccessRepository,
        advertisement::AdvertisementRepository,
        business_database::BusinessDatabase,
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
    pub advertisements: AdvertisementRepository,
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
    /// 处理 new_with_scheduler 的具体内部流程。
    fn new_with_scheduler(scheduler: DrawSchedulerRepository) -> Self {
        Self {
            access: AccessRepository::memory_seeded(),
            advertisements: AdvertisementRepository::memory(),
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

    /// 从环境变量初始化应用并根据配置加载调度器与业务服务。
    pub async fn from_env_with_scheduler(
        scheduler: DrawSchedulerRepository,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let Ok(database_url) = std::env::var("DATABASE_URL") else {
            tracing::info!("未配置 DATABASE_URL，使用内存业务仓储");
            return Ok(Self::new_with_scheduler(scheduler));
        };

        let lotteries = LotteryRepository::postgres(&database_url).await?;
        let business_database = BusinessDatabase::postgres(&database_url).await?;
        let api_sources =
            ApiDrawSourceRepository::persistent_api68_seeded(business_database.clone()).await?;
        let scheduler =
            DrawSchedulerRepository::persistent(scheduler.config()?, business_database.clone())
                .await?;

        tracing::info!("已配置 DATABASE_URL，使用 PostgreSQL 持久化所有后台业务仓储");
        Ok(Self {
            access: AccessRepository::persistent(business_database.clone()).await?,
            advertisements: AdvertisementRepository::persistent(business_database.clone()).await?,
            draws: DrawRepository::persistent_with_api_sources(
                api_sources,
                business_database.clone(),
            )
            .await?,
            finance: FinanceRepository::persistent(business_database.clone()).await?,
            group_buys: GroupBuyRepository::persistent(business_database.clone()).await?,
            invites: InviteRepository::persistent(business_database.clone()).await?,
            lotteries,
            orders: OrderRepository::persistent(business_database.clone()).await?,
            rebates: RebateRepository::persistent(business_database.clone()).await?,
            robots: RobotRepository::persistent(business_database.clone()).await?,
            scheduler,
            support: SupportRepository::persistent(business_database).await?,
        })
    }
}

/// 处理 default_draw_repository 的具体内部流程。
fn default_draw_repository() -> DrawRepository {
    DrawRepository::memory_with_api_sources(ApiDrawSourceRepository::api68_seeded())
}

/// 读取环境变量并返回可启动的应用路由实例。
pub async fn router_from_env() -> Result<Router, Box<dyn Error + Send + Sync>> {
    let scheduler = DrawSchedulerRepository::new(DrawSchedulerConfig::default());
    let state = AppState::from_env_with_scheduler(scheduler).await?;
    let scheduler_config = state.scheduler.config()?;
    spawn_draw_scheduler(
        state.draws.clone(),
        state.lotteries.clone(),
        state.orders.clone(),
        state.finance.clone(),
        scheduler_config,
        state.scheduler.clone(),
    );

    Ok(router_with_state(state))
}

/// 处理 router_with_state 的具体内部流程。
fn router_with_state(state: AppState) -> Router {
    Router::new()
        .nest("/api", routes::router(state.clone()))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
