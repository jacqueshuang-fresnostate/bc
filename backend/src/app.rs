//! 后端应用状态和依赖仓储组装入口，统一创建内存/数据库模式下的状态

use std::{env::VarError, error::Error, io};

use axum::Router;
use chrono::Local;
use serde::Serialize;
use tower_http::cors::CorsLayer;

use crate::{
    error::{ApiError, ApiResult},
    routes,
    services::{
        access::AccessRepository,
        advertisement::AdvertisementRepository,
        agent_application::AgentApplicationRepository,
        business_database::BusinessDatabase,
        chat_hall::ChatHallRepository,
        draw::DrawRepository,
        draw_api::ApiDrawSourceRepository,
        finance::FinanceRepository,
        group_buy::GroupBuyRepository,
        invite::InviteRepository,
        lottery::LotteryRepository,
        order::OrderRepository,
        realtime::RealtimeHub,
        rebate::RebateRepository,
        recharge::RechargeRepository,
        redis_runtime::RedisRuntime,
        robot::RobotRepository,
        scheduler::{spawn_draw_scheduler, DrawSchedulerConfig, DrawSchedulerRepository},
        support::SupportRepository,
        withdrawal::WithdrawalRepository,
    },
};

#[derive(Clone)]
/// 应用全局状态，集中持有每个业务模块的仓储和实时事件中心。
pub struct AppState {
    /// access字段。
    pub access: AccessRepository,
    /// advertisements字段。
    pub advertisements: AdvertisementRepository,
    /// agentapplications字段。
    pub agent_applications: AgentApplicationRepository,
    /// 聊天大厅字段。
    pub chat_hall: ChatHallRepository,
    /// draws字段。
    pub draws: DrawRepository,
    /// 资金字段。
    pub finance: FinanceRepository,
    /// 合买buys字段。
    pub group_buys: GroupBuyRepository,
    /// invites字段。
    pub invites: InviteRepository,
    /// 可选彩种列表。
    pub lotteries: LotteryRepository,
    /// 结算涉及的订单列表。
    pub orders: OrderRepository,
    /// rebates字段。
    pub rebates: RebateRepository,
    /// realtime字段。
    pub realtime: RealtimeHub,
    /// Redis 运行时，可选提供分布式锁和热点缓存失效。
    pub redis: RedisRuntime,
    /// recharges字段。
    pub recharges: RechargeRepository,
    /// robots字段。
    pub robots: RobotRepository,
    /// 调度器字段。
    pub scheduler: DrawSchedulerRepository,
    /// 客服字段。
    pub support: SupportRepository,
    /// withdrawals字段。
    pub withdrawals: WithdrawalRepository,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台手动刷新内存缓存后的结果摘要，用于提示哪些模块已经重新读取数据库。
pub struct MemoryCacheReloadResult {
    /// reloadedmodules字段。
    pub reloaded_modules: Vec<String>,
    /// 数据库直选modules字段。
    pub database_direct_modules: Vec<String>,
    /// skippedmodules字段。
    pub skipped_modules: Vec<String>,
    /// refreshedat字段。
    pub refreshed_at: String,
}

/// 手动刷新内存缓存的结果构建方法。
impl MemoryCacheReloadResult {
    /// 初始化空结果，等待各仓储按真实刷新结果填充。
    fn new() -> Self {
        Self {
            reloaded_modules: Vec::new(),
            database_direct_modules: Vec::new(),
            skipped_modules: Vec::new(),
            refreshed_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    /// 按仓储返回值记录刷新成功或内存模式跳过。
    fn record_reload(&mut self, module_name: &str, reloaded: bool) {
        if reloaded {
            self.reloaded_modules.push(module_name.to_string());
        } else {
            self.skipped_modules
                .push(format!("{module_name}（内存模式，无数据库快照）"));
        }
    }

    /// 记录数据库直读模块，这类模块每次查询都会读取数据库。
    fn record_database_direct(&mut self, module_name: &str) {
        self.database_direct_modules.push(module_name.to_string());
    }

    /// 判断本次维护动作是否真正触达了数据库。
    fn touched_database(&self) -> bool {
        !self.reloaded_modules.is_empty() || !self.database_direct_modules.is_empty()
    }
}

/// 应用状态构造和环境初始化方法。
impl AppState {
    /// 创建内存模式应用状态，主要用于未配置数据库的本地开发和测试。
    fn new_with_scheduler(scheduler: DrawSchedulerRepository, redis: RedisRuntime) -> Self {
        Self {
            access: AccessRepository::memory_seeded(),
            advertisements: AdvertisementRepository::memory(),
            agent_applications: AgentApplicationRepository::memory(),
            chat_hall: ChatHallRepository::memory(),
            draws: default_draw_repository(),
            finance: FinanceRepository::memory_seeded(),
            group_buys: GroupBuyRepository::memory_seeded(),
            invites: InviteRepository::memory_seeded(),
            lotteries: LotteryRepository::memory_seeded(),
            orders: OrderRepository::memory(),
            rebates: RebateRepository::memory_seeded(),
            realtime: RealtimeHub::new(),
            redis,
            recharges: RechargeRepository::memory(),
            robots: RobotRepository::memory_seeded(),
            scheduler,
            support: SupportRepository::memory_seeded(),
            withdrawals: WithdrawalRepository::memory(),
        }
    }

    /// 从环境变量初始化应用并根据配置加载调度器与业务服务。
    pub async fn from_env_with_scheduler(
        scheduler: DrawSchedulerRepository,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let redis = RedisRuntime::from_env().await?;
        let Some(database_url) = database_url_from_env()? else {
            tracing::info!("未配置 DATABASE_URL，使用内存业务仓储");
            return Ok(Self::new_with_scheduler(scheduler, redis));
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
            agent_applications: AgentApplicationRepository::persistent(business_database.clone())
                .await?,
            chat_hall: ChatHallRepository::persistent(business_database.clone()).await?,
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
            realtime: RealtimeHub::new(),
            redis,
            recharges: RechargeRepository::persistent(business_database.clone()).await?,
            robots: RobotRepository::persistent(business_database.clone()).await?,
            scheduler,
            support: SupportRepository::persistent(business_database.clone()).await?,
            withdrawals: WithdrawalRepository::persistent(business_database).await?,
        })
    }

    /// 重新从数据库加载所有快照型业务仓储，供后台手动清表或改库后的维护按钮使用。
    pub async fn reload_memory_cache_from_database(&self) -> ApiResult<MemoryCacheReloadResult> {
        let mut result = MemoryCacheReloadResult::new();

        if self.lotteries.is_database_backed() {
            result.record_database_direct("彩种配置");
        } else {
            result.record_reload("彩种配置", false);
        }

        result.record_reload(
            "广告管理",
            self.advertisements.reload_from_database().await?,
        );
        result.record_reload(
            "代理申请",
            self.agent_applications.reload_from_database().await?,
        );
        result.record_reload("聊天大厅", self.chat_hall.reload_from_database().await?);
        result.record_reload("开奖期号与控制", self.draws.reload_from_database().await?);
        result.record_reload("资金账户与流水", self.finance.reload_from_database().await?);
        result.record_reload("合买管理", self.group_buys.reload_from_database().await?);
        result.record_reload("邀请记录", self.invites.reload_from_database().await?);
        result.record_reload("注单与计奖派奖", self.orders.reload_from_database().await?);
        result.record_reload("邀请返利策略", self.rebates.reload_from_database().await?);
        result.record_reload("充值订单", self.recharges.reload_from_database().await?);
        result.record_reload("机器人配置", self.robots.reload_from_database().await?);
        result.record_reload("开奖调度配置", self.scheduler.reload_from_database().await?);
        result.record_reload("客服会话", self.support.reload_from_database().await?);
        result.record_reload("提现申请", self.withdrawals.reload_from_database().await?);
        result.record_reload(
            "用户权限与系统设置",
            self.access.reload_from_database().await?,
        );

        if !result.touched_database() {
            return Err(ApiError::BadRequest(
                "当前服务未启用数据库持久化，无法刷新内存缓存".to_string(),
            ));
        }

        Ok(result)
    }
}

/// 构建应用启动时使用的默认开奖仓储，优先走数据库持久化配置。
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
        state.access.clone(),
        state.draws.clone(),
        state.lotteries.clone(),
        state.orders.clone(),
        state.finance.clone(),
        state.group_buys.clone(),
        state.robots.clone(),
        state.realtime.clone(),
        scheduler_config,
        state.scheduler.clone(),
    );

    Ok(router_with_state(state))
}

/// 装配带共享状态的 Axum 路由树。
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
