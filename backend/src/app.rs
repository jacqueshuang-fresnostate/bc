//! 后端应用状态和依赖仓储组装入口，统一创建内存/数据库模式下的状态

use std::{env::VarError, error::Error, io};

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
        recharge::RechargeRepository,
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
    pub recharges: RechargeRepository,
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
            recharges: RechargeRepository::memory(),
            robots: RobotRepository::memory_seeded(),
            scheduler,
            support: SupportRepository::memory_seeded(),
        }
    }

    /// 从环境变量初始化应用并根据配置加载调度器与业务服务。
    pub async fn from_env_with_scheduler(
        scheduler: DrawSchedulerRepository,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let Some(database_url) = database_url_from_env()? else {
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
            recharges: RechargeRepository::persistent(business_database.clone()).await?,
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

/// 读取并校验数据库连接串，避免 SQLx 输出难懂的相对 URL 错误。
fn database_url_from_env() -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    match std::env::var("DATABASE_URL") {
        Ok(value) => normalize_database_url_value(&value)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "DATABASE_URL 配置无效：必须是有效 UTF-8 文本",
        ))),
    }
}

/// 处理数据库连接串的空值和协议前缀，保证容器日志给出中文原因。
fn normalize_database_url_value(value: &str) -> Result<Option<String>, io::Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    if !trimmed.starts_with("postgres://") && !trimmed.starts_with("postgresql://") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "DATABASE_URL 配置无效：必须以 postgres:// 或 postgresql:// 开头。示例：postgres://用户名:密码@主机:端口/数据库",
        ));
    }

    Ok(Some(trimmed.to_string()))
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

#[cfg(test)]
mod tests {
    use super::normalize_database_url_value;

    #[test]
    /// 验证空数据库连接串等同于未配置，容器可继续使用内存模式。
    fn database_url_allows_empty_value_as_unconfigured() {
        assert_eq!(normalize_database_url_value("").unwrap(), None);
        assert_eq!(normalize_database_url_value("   ").unwrap(), None);
    }

    #[test]
    /// 验证数据库连接串会修剪空白并保留合法 PostgreSQL 协议。
    fn database_url_accepts_postgres_url() {
        assert_eq!(
            normalize_database_url_value(" postgres://bc:pw@postgres:5432/bc ").unwrap(),
            Some("postgres://bc:pw@postgres:5432/bc".to_string())
        );
        assert_eq!(
            normalize_database_url_value("postgresql://bc:pw@postgres:5432/bc").unwrap(),
            Some("postgresql://bc:pw@postgres:5432/bc".to_string())
        );
    }

    #[test]
    /// 验证缺少协议前缀时给出中文配置错误，避免出现 RelativeUrlWithoutBase。
    fn database_url_rejects_relative_value() {
        let error = normalize_database_url_value("bc:pw@postgres:5432/bc")
            .expect_err("缺少协议的连接串必须失败");

        assert!(error.to_string().contains("DATABASE_URL 配置无效"));
        assert!(error.to_string().contains("postgres://"));
    }
}
