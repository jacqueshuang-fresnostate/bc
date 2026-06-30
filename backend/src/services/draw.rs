//! 开奖期号与开奖控制领域模型，定义状态与开奖请求参数

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, Row};

use crate::{
    domain::{
        draw::{
            ApiDrawSourceCrawlSnapshotSummary, ApiDrawSourceIssueSnapshot, CreateDrawIssueRequest,
            DrawControlTargetScope, DrawIssue, DrawIssueGenerationPreview, DrawIssueResultRequest,
            DrawIssueStatus, DrawSourceSyncResult, LotteryDrawControl,
            SaveLotteryDrawControlRequest,
        },
        lottery::{DrawMode, DrawSource, LotteryKind, LotteryNumberType, SaveDrawSourceRequest},
        order::OrderDetail,
    },
    error::{ApiError, ApiResult},
};

use super::{
    business_database::{enum_from_string, enum_to_string, BusinessDatabase},
    draw_api::{
        ApiDrawSourceCrawlSnapshotQuery, ApiDrawSourceLatestIssue, ApiDrawSourceRepository,
    },
    draw_generation::plan_api_draw_source_target,
    draw_risk::{self, DrawRiskCandidate},
    pagination::{ListPage, PageRequest},
    redis_runtime::RedisRuntime,
};

#[derive(Clone)]
/// 开奖期号与开奖控制仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct DrawRepository {
    inner: Arc<RwLock<DrawStore>>,
    api_sources: ApiDrawSourceRepository,
    controls: Arc<RwLock<DrawControlStore>>,
    persistence: Option<BusinessDatabase>,
    redis: RedisRuntime,
}

/// 开奖期号与开奖控制仓储，负责该模块数据读取、业务变更和持久化协调。
impl DrawRepository {
    #[allow(dead_code)]
    /// 创建内存仓储实例。
    pub fn memory() -> Self {
        Self::memory_with_api_sources(ApiDrawSourceRepository::empty())
    }

    /// 创建绑定开奖源依赖的内存开奖期仓储。
    pub fn memory_with_api_sources(api_sources: ApiDrawSourceRepository) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DrawStore::default())),
            api_sources,
            controls: Arc::new(RwLock::new(DrawControlStore::default())),
            persistence: None,
            redis: RedisRuntime::disabled(),
        }
    }

    /// 加载数据库和开奖源后创建可持久化开奖期仓储。
    pub async fn persistent_with_api_sources(
        api_sources: ApiDrawSourceRepository,
        persistence: BusinessDatabase,
    ) -> ApiResult<Self> {
        let (store, controls) = load_draw_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            api_sources,
            controls: Arc::new(RwLock::new(controls)),
            persistence: Some(persistence),
            redis: RedisRuntime::disabled(),
        })
    }

    /// 绑定 Redis 运行时，供期号风险池初始化、下注风险累加和开奖最低赔付候选读取使用。
    pub fn with_redis(mut self, redis: RedisRuntime) -> Self {
        self.redis = redis;
        self
    }

    #[allow(dead_code)]
    /// 按当前仓储快照返回全部期号列表；保留给测试和少量兼容路径使用，生产列表优先调用分页/专用查询。
    pub async fn list(&self) -> ApiResult<Vec<DrawIssue>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 按彩种 ID 过滤列表数据。
    pub async fn list_by_lottery_id(&self, lottery_id: &str) -> ApiResult<Vec<DrawIssue>> {
        let lottery_id = lottery_id.trim();
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| store.list_by_lottery_id(lottery_id))
    }

    /// 读取生成下一期所需的近期历史期号；数据库模式只取指定彩种的有限窗口，避免生成期号时扫描全部历史。
    pub async fn list_generation_seed_issues(
        &self,
        lottery_id: &str,
        limit: usize,
    ) -> ApiResult<Vec<DrawIssue>> {
        let lottery_id = lottery_id.trim();
        if lottery_id.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        if let Some(persistence) = &self.persistence {
            return query_generation_seed_draw_issues(persistence, lottery_id, limit).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| {
                store
                    .list_by_lottery_id(lottery_id)
                    .into_iter()
                    .take(limit)
                    .collect()
            })
    }

    /// 读取手机端首页需要的当前期和最近开奖，数据库模式按彩种集合收敛查询范围。
    pub async fn list_mobile_home_issues(
        &self,
        lottery_ids: &[String],
    ) -> ApiResult<Vec<DrawIssue>> {
        let lottery_ids = normalized_lottery_ids(lottery_ids);
        if lottery_ids.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(persistence) = &self.persistence {
            return query_mobile_home_draw_issues(persistence, &lottery_ids).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| store.mobile_home_issues(&lottery_ids))
    }

    /// 读取指定彩种集合每个彩种最近一期已开奖数据，供手机端“最新开奖”使用。
    pub async fn list_latest_drawn_issues_for_lotteries(
        &self,
        lottery_ids: &[String],
    ) -> ApiResult<Vec<DrawIssue>> {
        let lottery_ids = normalized_lottery_ids(lottery_ids);
        if lottery_ids.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(persistence) = &self.persistence {
            return query_latest_drawn_issues_for_lotteries(persistence, &lottery_ids).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| store.latest_drawn_issues_for_lotteries(&lottery_ids))
    }

    /// 分页读取指定彩种集合的已开奖历史，供手机端开奖页避免先取全量再分页。
    pub async fn drawn_history_page(
        &self,
        lottery_ids: &[String],
        page: PageRequest,
    ) -> ApiResult<ListPage<DrawIssue>> {
        let lottery_ids = normalized_lottery_ids(lottery_ids);
        if lottery_ids.is_empty() {
            return Ok(ListPage::from_all(Vec::new(), page));
        }
        if let Some(persistence) = &self.persistence {
            return query_drawn_history_page(persistence, &lottery_ids, page).await;
        }

        let issues = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
            .drawn_history_for_lotteries(&lottery_ids);
        Ok(ListPage::from_all(issues, page))
    }

    /// 按条件分页读取开奖期号；数据库模式下把彩种、状态过滤和分页下推到 SQL。
    pub async fn list_page(
        &self,
        lottery_id: Option<&str>,
        status: Option<DrawIssueStatus>,
        page: PageRequest,
    ) -> ApiResult<ListPage<DrawIssue>> {
        let lottery_id = lottery_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        if let Some(persistence) = &self.persistence {
            return query_draw_issues_page(persistence, lottery_id.as_deref(), status, page).await;
        }

        let issues = self
            .inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| {
                store
                    .list()
                    .into_iter()
                    .filter(|issue| {
                        lottery_id
                            .as_deref()
                            .map_or(true, |lottery_id| issue.lottery_id == lottery_id)
                    })
                    .filter(|issue| {
                        status
                            .as_ref()
                            .map_or(true, |status| issue.status == *status)
                    })
                    .collect::<Vec<_>>()
            })?;
        Ok(ListPage::from_all(issues, page))
    }

    /// 返回调度器本轮需要关注的活跃期号，只包含销售中和封盘待开奖期。
    pub async fn list_scheduler_active(&self) -> ApiResult<Vec<DrawIssue>> {
        if let Some(persistence) = &self.persistence {
            return query_scheduler_active_draw_issues(persistence).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| {
                store
                    .list()
                    .into_iter()
                    .filter(|issue| {
                        matches!(
                            issue.status,
                            DrawIssueStatus::Open | DrawIssueStatus::Closed
                        )
                    })
                    .collect()
            })
    }

    /// 返回封盘后和已开奖的期号，供合买流单退款扫描已过期但未处理的合买计划。
    pub async fn list_refundable_draw_issues(&self) -> ApiResult<Vec<DrawIssue>> {
        if let Some(persistence) = &self.persistence {
            return query_refundable_draw_issues(persistence).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| {
                store
                    .list()
                    .into_iter()
                    .filter(|issue| matches!(issue.status, DrawIssueStatus::Closed))
                    .collect()
            })
    }

    /// 按业务标识读取单条记录，未命中时返回未找到错误。
    pub async fn get(&self, id: &str) -> ApiResult<DrawIssue> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
            .get(id)
    }

    /// 按彩种和期号定位单个开奖期。
    pub async fn get_by_lottery_issue(
        &self,
        lottery_id: &str,
        issue: &str,
    ) -> ApiResult<DrawIssue> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
            .get_by_lottery_issue(lottery_id, issue)
    }

    /// 校验入参并创建一条新记录。
    pub async fn create(
        &self,
        lottery: &LotteryKind,
        payload: CreateDrawIssueRequest,
    ) -> ApiResult<DrawIssue> {
        let result = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            store.create(lottery, payload)?
        };
        self.persist_draw_issue(&result).await?;
        self.initialize_avoidance_risk_pool(lottery, &result).await;
        Ok(result)
    }

    /// 将订单潜在派奖风险累加到 Redis 风险池；失败只记录日志，不回滚已经完成的主交易。
    pub async fn record_avoidance_order_risk(&self, order: &OrderDetail) {
        if let Err(error) = draw_risk::add_order_risk(&self.redis, order).await {
            tracing::warn!(
                order_id = %order.id,
                lottery_id = %order.lottery_id,
                issue = %order.issue,
                error = %error.log_message(),
                "订单赔付风险写入 Redis 风险池失败"
            );
        }
    }

    /// 从 Redis 风险池扣回订单潜在派奖风险；失败只记录日志，后续开奖仍会走数据库复核兜底。
    pub async fn remove_avoidance_order_risk(&self, order: &OrderDetail) {
        if let Err(error) = draw_risk::remove_order_risk(&self.redis, order).await {
            tracing::warn!(
                order_id = %order.id,
                lottery_id = %order.lottery_id,
                issue = %order.issue,
                error = %error.log_message(),
                "订单赔付风险从 Redis 风险池扣回失败"
            );
        }
    }

    /// 读取当前期号最低预计赔付候选号码，Redis 不可用时返回 None 让避奖策略回退数据库扫描。
    pub(crate) async fn lowest_avoidance_risk_candidate(
        &self,
        issue: &DrawIssue,
    ) -> ApiResult<Option<DrawRiskCandidate>> {
        draw_risk::lowest_risk_candidate(&self.redis, issue).await
    }

    /// 创建期号后初始化 Redis 赔付风险池；初始化失败不影响期号落库。
    async fn initialize_avoidance_risk_pool(&self, lottery: &LotteryKind, issue: &DrawIssue) {
        if let Err(error) =
            draw_risk::initialize_risk_pool(&self.redis, issue, lottery.avoid_winning_enabled).await
        {
            tracing::warn!(
                lottery_id = %issue.lottery_id,
                issue = %issue.issue,
                error = %error.log_message(),
                "开奖赔付风险池初始化失败，后续开奖将回退数据库避奖策略"
            );
        }
    }

    /// 将开奖期状态设置为关闭。
    pub async fn close(&self, id: &str) -> ApiResult<DrawIssue> {
        let result = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            store.close(id)?
        };
        self.persist_draw_issue(&result).await?;
        Ok(result)
    }

    /// 提交开奖结果并更新开奖期状态。
    #[allow(dead_code)]
    pub async fn draw(&self, id: &str, payload: DrawIssueResultRequest) -> ApiResult<DrawIssue> {
        let (payload, uses_control_number) = self.resolve_draw_payload(id, payload).await?;
        self.draw_resolved_payload(id, payload, uses_control_number)
            .await
    }

    /// 使用预取 API 开奖号完成开奖，并允许调用方决定是否可用后台控制号覆盖。
    #[allow(dead_code)]
    pub async fn draw_with_prefetched_api_number_with_control_policy(
        &self,
        id: &str,
        api_draw_number: Option<String>,
        allow_control_number: bool,
    ) -> ApiResult<DrawIssue> {
        let (payload, uses_control_number) = self
            .resolve_prefetched_api_draw_payload(id, api_draw_number, allow_control_number)
            .await?;

        self.draw_resolved_payload(id, payload, uses_control_number)
            .await
    }

    /// 使用已解析的开奖载荷写入开奖结果，供开奖策略在落库前调整号码。
    pub(crate) async fn draw_resolved_payload(
        &self,
        id: &str,
        payload: DrawIssueResultRequest,
        uses_control_number: bool,
    ) -> ApiResult<DrawIssue> {
        let result = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            store.draw(id, payload, uses_control_number)?
        };
        self.persist_draw_issue(&result).await?;
        Ok(result)
    }

    /// 取消开奖期并回退相关状态。
    pub async fn cancel(&self, id: &str) -> ApiResult<DrawIssue> {
        let result = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            store.cancel(id)?
        };
        self.persist_draw_issue(&result).await?;
        Ok(result)
    }

    /// 读取所有可用开奖源配置。
    pub async fn draw_sources(&self) -> ApiResult<Vec<DrawSource>> {
        let mut sources = self.api_sources.list().await?;
        sources.extend(super::draw_api::platform_draw_source_summaries());
        Ok(sources)
    }

    /// 分页读取 API 开奖源采集快照，供后台排查第三方期号与本地期号差异。
    pub async fn list_api_draw_source_crawl_snapshots(
        &self,
        query: ApiDrawSourceCrawlSnapshotQuery<'_>,
    ) -> ApiResult<ListPage<ApiDrawSourceCrawlSnapshotSummary>> {
        self.api_sources.list_crawl_snapshots(query).await
    }

    /// 清除 API 开奖源采集快照审计记录，不修改开奖源配置或本地开奖期号。
    pub async fn clear_api_draw_source_crawl_snapshots(&self) -> ApiResult<usize> {
        self.api_sources.clear_crawl_snapshots().await
    }

    /// 一键删除已结算的已开奖期号，不修改未结算期号、开奖源配置或开奖控制配置。
    pub async fn clear_settled_drawn_issues(
        &self,
        settled_draw_issue_ids: &BTreeSet<String>,
    ) -> ApiResult<usize> {
        if settled_draw_issue_ids.is_empty() {
            return Ok(0);
        }

        if let Some(persistence) = &self.persistence {
            let deleted_count =
                delete_settled_drawn_issues_in_database(persistence, settled_draw_issue_ids)
                    .await?;
            if deleted_count > 0 {
                self.inner
                    .write()
                    .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
                    .clear_settled_drawn_issues(settled_draw_issue_ids);
            }
            return Ok(deleted_count);
        }

        self.inner
            .write()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|mut store| store.clear_settled_drawn_issues(settled_draw_issue_ids))
    }

    /// 按彩种列表返回开奖控制配置。
    pub async fn list_draw_controls(
        &self,
        lotteries: &[LotteryKind],
    ) -> ApiResult<Vec<LotteryDrawControl>> {
        self.controls
            .read()
            .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))
            .map(|store| {
                lotteries
                    .iter()
                    .map(|lottery| store.summary_for(lottery))
                    .collect()
            })
    }

    /// 查询单个彩种的开奖控制配置。
    pub async fn get_draw_control(&self, lottery: &LotteryKind) -> ApiResult<LotteryDrawControl> {
        self.controls
            .read()
            .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))?
            .get(lottery)
    }

    /// 更新或创建彩种开奖控制设置。
    pub async fn save_draw_control(
        &self,
        lottery: &LotteryKind,
        payload: SaveLotteryDrawControlRequest,
    ) -> ApiResult<LotteryDrawControl> {
        if payload.enabled && !lottery.draw_control_enabled {
            return Err(ApiError::BadRequest("该彩种未开启开奖号码控制".to_string()));
        }

        let draw_number = normalize_control_draw_number(lottery, &payload)?;
        let (target_scope, target_issue, target_order_id) = normalize_control_target(&payload)?;
        let (result, snapshot) = {
            let mut store = self
                .controls
                .write()
                .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))?;

            store.save(DrawControlConfig {
                lottery_id: lottery.id.clone(),
                enabled: payload.enabled,
                draw_number,
                target_scope,
                target_issue,
                target_order_id,
                updated_at: current_timestamp_label(),
            });
            (store.get(lottery)?, store.clone())
        };
        self.persist_controls(&snapshot).await?;
        Ok(result)
    }

    /// 创建开奖源配置。
    pub async fn create_draw_source(
        &self,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
    ) -> ApiResult<DrawSource> {
        self.api_sources.create(payload, lotteries).await
    }

    /// 更新开奖源配置。
    pub async fn update_draw_source(
        &self,
        id: &str,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
    ) -> ApiResult<DrawSource> {
        self.api_sources.update(id, payload, lotteries).await
    }

    /// 删除开奖源配置。
    pub async fn delete_draw_source(&self, id: &str) -> ApiResult<DrawSource> {
        self.api_sources.delete(id).await
    }

    /// 判断指定彩种是否开启手工开奖控制。
    pub async fn has_active_draw_control(&self, issue: &DrawIssue) -> ApiResult<bool> {
        self.controls
            .read()
            .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))
            .map(|store| store.active_draw_number(issue).is_some())
    }

    /// 读取指定彩种外部 API 的最新期号信息。
    pub async fn latest_api_issue_for_lottery(
        &self,
        lottery_id: &str,
    ) -> ApiResult<Option<ApiDrawSourceLatestIssue>> {
        self.api_sources.latest_issue_for_lottery(lottery_id).await
    }

    /// 按期号所属 API 开奖源预取开奖号码，不修改本地期号状态。
    pub async fn api_draw_number_for_issue(&self, issue: &DrawIssue) -> ApiResult<Option<String>> {
        self.api_sources.draw_number_for(issue).await
    }

    /// 按外部 API 开奖源立即校准指定彩种的当前可销售期号。
    pub async fn sync_api_draw_source(
        &self,
        lottery: &LotteryKind,
        now: &str,
        sale_close_lead_seconds: u32,
        protected_issues: &BTreeSet<String>,
    ) -> ApiResult<DrawSourceSyncResult> {
        if lottery.draw_mode != DrawMode::Api {
            return Err(ApiError::BadRequest(
                "只有 API 开奖彩种可以同步开奖源".to_string(),
            ));
        }

        let latest_api_issue = self
            .latest_api_issue_for_lottery(&lottery.id)
            .await?
            .ok_or_else(|| ApiError::NotFound("当前彩种没有绑定 API 开奖源".to_string()))?;
        let target =
            plan_api_draw_source_target(lottery, &latest_api_issue, now, sale_close_lead_seconds)?;
        let api_snapshot = ApiDrawSourceIssueSnapshot {
            latest_draw_time: latest_api_issue.draw_time.clone(),
            latest_issue: latest_api_issue.issue.clone(),
            next_draw_time: latest_api_issue.next_draw_time.clone(),
            next_issue: latest_api_issue.next_issue.clone(),
        };

        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            let result = store.sync_api_target(lottery, api_snapshot, target, protected_issues)?;
            (result, store.clone())
        };
        self.persist_draws(&snapshot).await?;

        tracing::info!(
            lottery_id = %result.lottery_id,
            target_issue = %result.target_issue.issue,
            generated_count = result.generated_issues.len(),
            updated_count = result.updated_issues.len(),
            cancelled_count = result.cancelled_issues.len(),
            kept_count = result.kept_issues.len(),
            "手动同步开奖源完成"
        );

        Ok(result)
    }
    /// 根据开奖模式解析本次开奖需要使用的号码来源。
    pub(crate) async fn resolve_draw_payload(
        &self,
        id: &str,
        payload: DrawIssueResultRequest,
    ) -> ApiResult<(DrawIssueResultRequest, bool)> {
        let issue = self.get(id).await?;

        if let Some(draw_number) = self.active_draw_control_number(&issue).await? {
            return Ok((
                DrawIssueResultRequest {
                    draw_number: Some(draw_number),
                },
                true,
            ));
        }

        if issue.draw_mode != DrawMode::Api {
            return Ok((payload, false));
        }

        if let Some(draw_number) = self.api_sources.draw_number_for(&issue).await? {
            return Ok((
                DrawIssueResultRequest {
                    draw_number: Some(draw_number),
                },
                false,
            ));
        }

        Ok((DrawIssueResultRequest::default(), false))
    }
    /// 使用预抓取的 API 开奖号码解析当前期号的开奖载荷。
    pub(crate) async fn resolve_prefetched_api_draw_payload(
        &self,
        id: &str,
        api_draw_number: Option<String>,
        allow_control_number: bool,
    ) -> ApiResult<(DrawIssueResultRequest, bool)> {
        let issue = self.get(id).await?;

        if allow_control_number {
            if let Some(draw_number) = self.active_draw_control_number(&issue).await? {
                return Ok((
                    DrawIssueResultRequest {
                        draw_number: Some(draw_number),
                    },
                    true,
                ));
            }
        }

        if issue.draw_mode == DrawMode::Api {
            return Ok((
                DrawIssueResultRequest {
                    draw_number: api_draw_number,
                },
                false,
            ));
        }

        Ok((DrawIssueResultRequest::default(), false))
    }
    /// 读取命中当前期号或订单的控奖号码。
    async fn active_draw_control_number(&self, issue: &DrawIssue) -> ApiResult<Option<String>> {
        self.controls
            .read()
            .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))
            .map(|store| store.active_draw_number(issue))
    }

    /// 从数据库重新加载期号、开奖控制和 API 开奖源缓存，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            self.api_sources.reload_from_database().await?;
            return Ok(false);
        };
        let (store, controls) = load_draw_store(persistence).await?;
        self.api_sources.reload_from_database().await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("开奖期号缓存刷新失败".to_string()))? = store;
        *self
            .controls
            .write()
            .map_err(|_| ApiError::Internal("开奖控制缓存刷新失败".to_string()))? = controls;
        Ok(true)
    }
    /// 把开奖期号快照同步保存到持久化存储。
    async fn persist_draws(&self, store: &DrawStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_draw_issues(persistence, store).await?;
        }

        Ok(())
    }
    /// 把单个开奖期号保存到持久化存储。
    async fn persist_draw_issue(&self, issue: &DrawIssue) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            upsert_draw_issue(persistence, issue).await?;
        }

        Ok(())
    }
    /// 把控奖配置快照同步保存到持久化存储。
    async fn persist_controls(&self, store: &DrawControlStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_draw_controls(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
/// 开奖期号与开奖控制运行时数据快照，用于内存模式和数据库持久化前的业务校验。
struct DrawStore {
    next_sequence: u64,
    issues: BTreeMap<String, DrawIssue>,
}

/// 从数据库加载开奖期号与开奖控制运行时快照，空库时按模块规则初始化。
async fn load_draw_store(database: &BusinessDatabase) -> ApiResult<(DrawStore, DrawControlStore)> {
    let mut issues = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM draw_issues
         ORDER BY id ASC",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?
    {
        let issue = draw_issue_from_row(row)?;
        let id = issue.id.clone();
        issues.insert(id, issue);
    }

    let mut controls = BTreeMap::new();
    for row in sqlx::query(
        "SELECT lottery_id, enabled, draw_number, target_scope, target_issue, target_order_id, updated_at
         FROM draw_controls",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("开奖控制数据读取失败".to_string()))?
    {
        let lottery_id: String = row
            .try_get("lottery_id")
            .map_err(|_| ApiError::Internal("开奖控制数据读取失败".to_string()))?;
        controls.insert(
            lottery_id.clone(),
            DrawControlConfig {
                lottery_id,
                enabled: row
                    .try_get("enabled")
                    .map_err(|_| ApiError::Internal("开奖控制数据读取失败".to_string()))?,
                draw_number: row
                    .try_get("draw_number")
                    .map_err(|_| ApiError::Internal("开奖控制数据读取失败".to_string()))?,
                target_scope: enum_from_string(
                    row.try_get("target_scope")
                        .map_err(|_| ApiError::Internal("开奖控制范围数据读取失败".to_string()))?,
                )?,
                target_issue: row
                    .try_get("target_issue")
                    .map_err(|_| ApiError::Internal("开奖控制期号数据读取失败".to_string()))?,
                target_order_id: row
                    .try_get("target_order_id")
                    .map_err(|_| ApiError::Internal("开奖控制订单数据读取失败".to_string()))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|_| ApiError::Internal("开奖控制数据读取失败".to_string()))?,
            },
        );
    }

    Ok((
        DrawStore {
            next_sequence: max_sequence(issues.keys(), 'D'),
            issues,
        },
        DrawControlStore { controls },
    ))
}

/// 从数据库行恢复开奖期号结构，供全量加载和分页查询复用。
fn draw_issue_from_row(row: PgRow) -> ApiResult<DrawIssue> {
    Ok(DrawIssue {
        id: row
            .try_get("id")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        lottery_id: row
            .try_get("lottery_id")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        lottery_name: row
            .try_get("lottery_name")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        issue: row
            .try_get("issue")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        number_type: enum_from_string(
            row.try_get("number_type")
                .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        )?,
        draw_mode: enum_from_string(
            row.try_get("draw_mode")
                .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        )?,
        scheduled_at: row
            .try_get("scheduled_at")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        sale_closed_at: row
            .try_get("sale_closed_at")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        status: enum_from_string(
            row.try_get("status")
                .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        )?,
        draw_number: row
            .try_get("draw_number")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        drawn_at: row
            .try_get("drawn_at")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
        created_at: row
            .try_get("created_at")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?,
    })
}

/// 数据库模式下按彩种过滤并分页读取开奖期号，避免后台列表先拉全量再裁剪。
async fn query_draw_issues_page(
    database: &BusinessDatabase,
    lottery_id: Option<&str>,
    status: Option<DrawIssueStatus>,
    page: PageRequest,
) -> ApiResult<ListPage<DrawIssue>> {
    let status = status.as_ref().map(enum_to_string).transpose()?;
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM draw_issues
         WHERE ($1::text IS NULL OR lottery_id = $1)
           AND ($2::text IS NULL OR status = $2)",
    )
    .bind(lottery_id)
    .bind(status.as_deref())
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("开奖期号分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("开奖期号分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM draw_issues
         WHERE ($1::text IS NULL OR lottery_id = $1)
           AND ($2::text IS NULL OR status = $2)
         ORDER BY id DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(lottery_id)
    .bind(status.as_deref())
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("开奖期号分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(draw_issue_from_row)
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下读取调度器活跃期号，避免每轮扫描所有历史开奖期。
async fn query_scheduler_active_draw_issues(
    database: &BusinessDatabase,
) -> ApiResult<Vec<DrawIssue>> {
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM draw_issues
         WHERE status IN ('open', 'closed')
         ORDER BY id DESC",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("调度活跃期号数据读取失败".to_string()))?;

    rows.into_iter().map(draw_issue_from_row).collect()
}

/// 数据库模式下读取封盘和已开奖的期号，用于合买流单退款扫描。
async fn query_refundable_draw_issues(database: &BusinessDatabase) -> ApiResult<Vec<DrawIssue>> {
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM draw_issues
         WHERE status = 'closed'
         ORDER BY id DESC
         LIMIT 100",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("退款期号数据读取失败".to_string()))?;

    rows.into_iter().map(draw_issue_from_row).collect()
}

/// 数据库模式下读取期号生成所需的近期历史，支持恢复序号和避开重复期号。
async fn query_generation_seed_draw_issues(
    database: &BusinessDatabase,
    lottery_id: &str,
    limit: usize,
) -> ApiResult<Vec<DrawIssue>> {
    let limit =
        i64::try_from(limit).map_err(|_| ApiError::BadRequest("期号种子数量过大".to_string()))?;
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM draw_issues
         WHERE lottery_id = $1
         ORDER BY scheduled_at DESC, id DESC
         LIMIT $2",
    )
    .bind(lottery_id)
    .bind(limit)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("期号生成历史数据读取失败".to_string()))?;

    rows.into_iter().map(draw_issue_from_row).collect()
}

/// 数据库模式下读取首页所需期号，包含当前待处理期和每个彩种最近期开奖。
async fn query_mobile_home_draw_issues(
    database: &BusinessDatabase,
    lottery_ids: &[String],
) -> ApiResult<Vec<DrawIssue>> {
    let rows = sqlx::query(
        "WITH latest_drawn AS (
             SELECT DISTINCT ON (lottery_id)
                    id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                    sale_closed_at, status, draw_number, drawn_at, created_at
             FROM draw_issues
             WHERE lottery_id = ANY($1::text[])
               AND status = 'drawn'
               AND draw_number IS NOT NULL
               AND btrim(draw_number) <> ''
             ORDER BY lottery_id, scheduled_at DESC, id DESC
         )
         SELECT id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM draw_issues
         WHERE lottery_id = ANY($1::text[])
           AND status IN ('open', 'closed')
         UNION ALL
         SELECT id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM latest_drawn
         ORDER BY lottery_id ASC, scheduled_at DESC, id DESC",
    )
    .bind(lottery_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("手机端首页期号数据读取失败".to_string()))?;

    rows.into_iter().map(draw_issue_from_row).collect()
}

/// 数据库模式下按彩种集合读取每个彩种最近一期已开奖数据。
async fn query_latest_drawn_issues_for_lotteries(
    database: &BusinessDatabase,
    lottery_ids: &[String],
) -> ApiResult<Vec<DrawIssue>> {
    let rows = sqlx::query(
        "SELECT DISTINCT ON (lottery_id)
                id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM draw_issues
         WHERE lottery_id = ANY($1::text[])
           AND status = 'drawn'
           AND draw_number IS NOT NULL
           AND btrim(draw_number) <> ''
         ORDER BY lottery_id, scheduled_at DESC, id DESC",
    )
    .bind(lottery_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("最新开奖数据读取失败".to_string()))?;

    rows.into_iter().map(draw_issue_from_row).collect()
}

/// 数据库模式下分页读取已开奖历史。
async fn query_drawn_history_page(
    database: &BusinessDatabase,
    lottery_ids: &[String],
    page: PageRequest,
) -> ApiResult<ListPage<DrawIssue>> {
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM draw_issues
         WHERE lottery_id = ANY($1::text[])
           AND status = 'drawn'
           AND draw_number IS NOT NULL
           AND btrim(draw_number) <> ''",
    )
    .bind(lottery_ids)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("开奖历史分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("开奖历史分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
                sale_closed_at, status, draw_number, drawn_at, created_at
         FROM draw_issues
         WHERE lottery_id = ANY($1::text[])
           AND status = 'drawn'
           AND draw_number IS NOT NULL
           AND btrim(draw_number) <> ''
         ORDER BY scheduled_at DESC, issue DESC, id DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(lottery_ids)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("开奖历史分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(draw_issue_from_row)
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 归一化彩种 ID 集合，去空格和重复项，避免 SQL 查询绑定无效值。
fn normalized_lottery_ids(lottery_ids: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    lottery_ids
        .iter()
        .filter_map(|lottery_id| {
            let lottery_id = lottery_id.trim();
            if lottery_id.is_empty() || !seen.insert(lottery_id.to_string()) {
                None
            } else {
                Some(lottery_id.to_string())
            }
        })
        .collect()
}

/// 高频期号状态变更使用单行 upsert，避免调度时反复重写整张期号表。
async fn upsert_draw_issue(database: &BusinessDatabase, issue: &DrawIssue) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO draw_issues
         (id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
          sale_closed_at, status, draw_number, drawn_at, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         ON CONFLICT (id) DO UPDATE SET
          lottery_id = EXCLUDED.lottery_id,
          lottery_name = EXCLUDED.lottery_name,
          issue = EXCLUDED.issue,
          number_type = EXCLUDED.number_type,
          draw_mode = EXCLUDED.draw_mode,
          scheduled_at = EXCLUDED.scheduled_at,
          sale_closed_at = EXCLUDED.sale_closed_at,
          status = EXCLUDED.status,
          draw_number = EXCLUDED.draw_number,
          drawn_at = EXCLUDED.drawn_at,
          created_at = EXCLUDED.created_at",
    )
    .bind(&issue.id)
    .bind(&issue.lottery_id)
    .bind(&issue.lottery_name)
    .bind(&issue.issue)
    .bind(enum_to_string(&issue.number_type)?)
    .bind(enum_to_string(&issue.draw_mode)?)
    .bind(&issue.scheduled_at)
    .bind(&issue.sale_closed_at)
    .bind(enum_to_string(&issue.status)?)
    .bind(&issue.draw_number)
    .bind(&issue.drawn_at)
    .bind(&issue.created_at)
    .execute(database.pool())
    .await
    .map_err(|_| ApiError::Internal("开奖期号数据保存失败".to_string()))?;

    Ok(())
}

/// 保存开奖期号运行时快照，重写期号表和运行序号。
async fn save_draw_issues(database: &BusinessDatabase, store: &DrawStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("开奖期号事务开启失败".to_string()))?;
    sqlx::query("DELETE FROM draw_issues")
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖期号数据清理失败".to_string()))?;

    for issue in store.issues.values() {
        sqlx::query(
            "INSERT INTO draw_issues
             (id, lottery_id, lottery_name, issue, number_type, draw_mode, scheduled_at,
              sale_closed_at, status, draw_number, drawn_at, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&issue.id)
        .bind(&issue.lottery_id)
        .bind(&issue.lottery_name)
        .bind(&issue.issue)
        .bind(enum_to_string(&issue.number_type)?)
        .bind(enum_to_string(&issue.draw_mode)?)
        .bind(&issue.scheduled_at)
        .bind(&issue.sale_closed_at)
        .bind(enum_to_string(&issue.status)?)
        .bind(&issue.draw_number)
        .bind(&issue.drawn_at)
        .bind(&issue.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖期号数据保存失败".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("开奖期号事务提交失败".to_string()))
}

/// 删除数据库中已结算的已开奖期号，供后台期号列表一键清理历史记录使用。
async fn delete_settled_drawn_issues_in_database(
    database: &BusinessDatabase,
    settled_draw_issue_ids: &BTreeSet<String>,
) -> ApiResult<usize> {
    let settled_draw_issue_ids = settled_draw_issue_ids.iter().cloned().collect::<Vec<_>>();
    let result = sqlx::query("DELETE FROM draw_issues WHERE status = $1 AND id = ANY($2::text[])")
        .bind(enum_to_string(&DrawIssueStatus::Drawn)?)
        .bind(&settled_draw_issue_ids)
        .execute(database.pool())
        .await
        .map_err(|_| ApiError::Internal("已开奖期号清理失败".to_string()))?;

    usize::try_from(result.rows_affected())
        .map_err(|_| ApiError::Internal("已开奖期号清理数量超出范围".to_string()))
}

/// 保存开奖控制配置快照，供彩种控制台和自动开奖共同读取。
async fn save_draw_controls(
    database: &BusinessDatabase,
    store: &DrawControlStore,
) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("开奖控制事务开启失败".to_string()))?;
    sqlx::query("DELETE FROM draw_controls")
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖控制数据清理失败".to_string()))?;

    for control in store.controls.values() {
        sqlx::query(
            "INSERT INTO draw_controls
             (lottery_id, enabled, draw_number, target_scope, target_issue, target_order_id, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&control.lottery_id)
        .bind(control.enabled)
        .bind(&control.draw_number)
        .bind(enum_to_string(&control.target_scope)?)
        .bind(&control.target_issue)
        .bind(&control.target_order_id)
        .bind(&control.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖控制数据保存失败".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("开奖控制事务提交失败".to_string()))
}

/// 计算并返回序列号最大值。
fn max_sequence<'a>(ids: impl Iterator<Item = &'a String>, prefix: char) -> u64 {
    ids.filter_map(|id| id.strip_prefix(prefix))
        .filter_map(|value| value.parse::<u64>().ok())
        .max()
        .unwrap_or_default()
}

/// 开奖期号与开奖控制运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl DrawStore {
    /// 按当前仓储快照返回全部期号列表。
    fn list(&self) -> Vec<DrawIssue> {
        self.issues.values().rev().cloned().collect()
    }

    /// 按彩种筛选期号列表。
    fn list_by_lottery_id(&self, lottery_id: &str) -> Vec<DrawIssue> {
        self.issues
            .values()
            .rev()
            .filter(|issue| issue.lottery_id == lottery_id)
            .cloned()
            .collect()
    }

    /// 返回首页需要的活跃期号和最近期开奖。
    fn mobile_home_issues(&self, lottery_ids: &[String]) -> Vec<DrawIssue> {
        let lottery_ids = lottery_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut issues = self
            .issues
            .values()
            .filter(|issue| {
                lottery_ids.contains(issue.lottery_id.as_str())
                    && matches!(
                        issue.status,
                        DrawIssueStatus::Open | DrawIssueStatus::Closed
                    )
            })
            .cloned()
            .collect::<Vec<_>>();
        issues.extend(
            self.latest_drawn_issues_for_lotteries(
                &lottery_ids
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect::<Vec<_>>(),
            ),
        );
        issues.sort_by(|left, right| {
            left.lottery_id
                .cmp(&right.lottery_id)
                .then_with(|| right.scheduled_at.cmp(&left.scheduled_at))
                .then_with(|| right.id.cmp(&left.id))
        });
        issues
    }

    /// 返回指定彩种集合每个彩种最近一期已开奖数据。
    fn latest_drawn_issues_for_lotteries(&self, lottery_ids: &[String]) -> Vec<DrawIssue> {
        let lottery_ids = lottery_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut latest_by_lottery = BTreeMap::<String, DrawIssue>::new();
        for issue in self.issues.values() {
            if !lottery_ids.contains(issue.lottery_id.as_str())
                || issue.status != DrawIssueStatus::Drawn
                || issue
                    .draw_number
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none()
            {
                continue;
            }
            let should_replace = latest_by_lottery
                .get(&issue.lottery_id)
                .map(|current| {
                    issue.scheduled_at > current.scheduled_at
                        || (issue.scheduled_at == current.scheduled_at && issue.id > current.id)
                })
                .unwrap_or(true);
            if should_replace {
                latest_by_lottery.insert(issue.lottery_id.clone(), issue.clone());
            }
        }
        latest_by_lottery.into_values().collect()
    }

    /// 删除已结算的已开奖期号，保留未结算的已开奖期号作为计奖派奖入口。
    fn clear_settled_drawn_issues(&mut self, settled_draw_issue_ids: &BTreeSet<String>) -> usize {
        let before_count = self.issues.len();
        self.issues.retain(|_, issue| {
            issue.status != DrawIssueStatus::Drawn || !settled_draw_issue_ids.contains(&issue.id)
        });
        before_count - self.issues.len()
    }

    /// 返回指定彩种集合的已开奖历史，并按开奖时间倒序排列。
    fn drawn_history_for_lotteries(&self, lottery_ids: &[String]) -> Vec<DrawIssue> {
        let lottery_ids = lottery_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let mut issues = self
            .issues
            .values()
            .filter(|issue| {
                lottery_ids.contains(issue.lottery_id.as_str())
                    && issue.status == DrawIssueStatus::Drawn
                    && issue
                        .draw_number
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .is_some()
            })
            .cloned()
            .collect::<Vec<_>>();
        issues.sort_by(|left, right| {
            right
                .scheduled_at
                .cmp(&left.scheduled_at)
                .then_with(|| right.issue.cmp(&left.issue))
                .then_with(|| right.id.cmp(&left.id))
        });
        issues
    }

    /// 按标识查询并返回单条记录。
    fn get(&self, id: &str) -> ApiResult<DrawIssue> {
        self.issues
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("draw issue `{id}` not found")))
    }

    /// 按彩种和期号定位开奖期记录。
    fn get_by_lottery_issue(&self, lottery_id: &str, issue: &str) -> ApiResult<DrawIssue> {
        let lottery_id = lottery_id.trim();
        let issue = issue.trim();
        self.issues
            .values()
            .find(|draw_issue| draw_issue.lottery_id == lottery_id && draw_issue.issue == issue)
            .cloned()
            .ok_or_else(|| {
                ApiError::NotFound(format!(
                    "draw issue `{issue}` not found for lottery `{lottery_id}`"
                ))
            })
    }

    /// 校验入参并创建新记录。
    fn create(
        &mut self,
        lottery: &LotteryKind,
        payload: CreateDrawIssueRequest,
    ) -> ApiResult<DrawIssue> {
        validate_create_request(lottery, &payload)?;

        if self.issues.values().any(|issue| {
            issue.lottery_id == payload.lottery_id.trim() && issue.issue == payload.issue.trim()
        }) {
            return Err(ApiError::Conflict(format!(
                "draw issue `{}` already exists for lottery `{}`",
                payload.issue.trim(),
                payload.lottery_id.trim()
            )));
        }

        self.next_sequence += 1;
        let issue = DrawIssue {
            id: format!("D{:012}", self.next_sequence),
            lottery_id: lottery.id.clone(),
            lottery_name: lottery.name.clone(),
            issue: payload.issue.trim().to_string(),
            number_type: lottery.number_type.clone(),
            draw_mode: lottery.draw_mode.clone(),
            scheduled_at: payload.scheduled_at.trim().to_string(),
            sale_closed_at: payload.sale_closed_at.trim().to_string(),
            status: DrawIssueStatus::Open,
            draw_number: None,
            drawn_at: None,
            created_at: current_timestamp_label(),
        };

        self.issues.insert(issue.id.clone(), issue.clone());
        Ok(issue)
    }

    /// 将销售中的期号更新为已封盘。
    fn close(&mut self, id: &str) -> ApiResult<DrawIssue> {
        let issue = self
            .issues
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("draw issue `{id}` not found")))?;

        if issue.status != DrawIssueStatus::Open {
            return Err(ApiError::BadRequest(
                "only open draw issues can be closed".to_string(),
            ));
        }

        issue.status = DrawIssueStatus::Closed;
        Ok(issue.clone())
    }

    /// 写入开奖号码并把期号流转为已开奖。
    fn draw(
        &mut self,
        id: &str,
        payload: DrawIssueResultRequest,
        uses_control_number: bool,
    ) -> ApiResult<DrawIssue> {
        let issue = self
            .issues
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("draw issue `{id}` not found")))?;

        if matches!(
            issue.status,
            DrawIssueStatus::Drawn | DrawIssueStatus::Cancelled
        ) {
            return Err(ApiError::BadRequest(
                "draw issue cannot be drawn in current status".to_string(),
            ));
        }

        let draw_number = match issue.draw_mode {
            DrawMode::Manual => payload
                .draw_number
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    ApiError::BadRequest("manual draw requires draw number".to_string())
                })?
                .to_string(),
            DrawMode::Platform if uses_control_number => payload
                .draw_number
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ApiError::BadRequest("control draw number is required".to_string()))?
                .to_string(),
            DrawMode::Platform => {
                generated_draw_number(&issue.number_type, &issue.lottery_id, &issue.issue)
            }
            DrawMode::Api => payload
                .draw_number
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    generated_draw_number(&issue.number_type, &issue.lottery_id, &issue.issue)
                }),
        };

        let draw_number = normalize_draw_number(&draw_number, &issue.number_type)?;

        issue.draw_number = Some(draw_number);
        issue.drawn_at = Some(current_timestamp_label());
        issue.status = DrawIssueStatus::Drawn;
        Ok(issue.clone())
    }

    /// 取消未开奖期号并保留审计状态。
    fn cancel(&mut self, id: &str) -> ApiResult<DrawIssue> {
        let issue = self
            .issues
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("draw issue `{id}` not found")))?;

        if issue.status == DrawIssueStatus::Drawn {
            return Err(ApiError::BadRequest(
                "drawn draw issue cannot be cancelled".to_string(),
            ));
        }
        if issue.status == DrawIssueStatus::Cancelled {
            return Err(ApiError::BadRequest(
                "draw issue is already cancelled".to_string(),
            ));
        }

        issue.status = DrawIssueStatus::Cancelled;
        Ok(issue.clone())
    }

    /// 按 API 开奖源目标期校准本地未开奖期号。
    fn sync_api_target(
        &mut self,
        lottery: &LotteryKind,
        api_snapshot: ApiDrawSourceIssueSnapshot,
        target: DrawIssueGenerationPreview,
        protected_issues: &BTreeSet<String>,
    ) -> ApiResult<DrawSourceSyncResult> {
        let mut generated_issues = Vec::new();
        let mut updated_issues = Vec::new();
        let mut cancelled_issues = Vec::new();
        let mut kept_issues = Vec::new();

        let target_issue_id = self
            .issues
            .values()
            .find(|issue| issue.lottery_id == lottery.id && issue.issue == target.issue)
            .map(|issue| issue.id.clone());

        let target_issue = if let Some(issue_id) = target_issue_id {
            let issue = self
                .issues
                .get_mut(&issue_id)
                .ok_or_else(|| ApiError::Internal("目标期号数据不存在".to_string()))?;
            if issue.status == DrawIssueStatus::Drawn {
                return Err(ApiError::Conflict(
                    "开奖源目标期号已在本地开奖，无法自动校准".to_string(),
                ));
            }

            issue.lottery_name = lottery.name.clone();
            issue.number_type = lottery.number_type.clone();
            issue.draw_mode = lottery.draw_mode.clone();
            issue.scheduled_at = target.scheduled_at.clone();
            issue.sale_closed_at = target.sale_closed_at.clone();
            issue.status = DrawIssueStatus::Open;
            issue.draw_number = None;
            issue.drawn_at = None;
            updated_issues.push(issue.clone());
            issue.clone()
        } else {
            self.next_sequence += 1;
            let issue = DrawIssue {
                id: format!("D{:012}", self.next_sequence),
                lottery_id: lottery.id.clone(),
                lottery_name: lottery.name.clone(),
                issue: target.issue.clone(),
                number_type: lottery.number_type.clone(),
                draw_mode: lottery.draw_mode.clone(),
                scheduled_at: target.scheduled_at.clone(),
                sale_closed_at: target.sale_closed_at.clone(),
                status: DrawIssueStatus::Open,
                draw_number: None,
                drawn_at: None,
                created_at: current_timestamp_label(),
            };
            self.issues.insert(issue.id.clone(), issue.clone());
            generated_issues.push(issue.clone());
            issue
        };

        let stale_issue_ids = self
            .issues
            .values()
            .filter(|issue| {
                issue.lottery_id == lottery.id
                    && issue.issue != target.issue
                    && matches!(
                        issue.status,
                        DrawIssueStatus::Open | DrawIssueStatus::Closed
                    )
            })
            .map(|issue| issue.id.clone())
            .collect::<Vec<_>>();

        for issue_id in stale_issue_ids {
            let Some(issue) = self.issues.get_mut(&issue_id) else {
                continue;
            };
            if protected_issues.contains(&issue.issue) {
                kept_issues.push(issue.clone());
                continue;
            }

            issue.status = DrawIssueStatus::Cancelled;
            cancelled_issues.push(issue.clone());
        }

        let message = sync_result_message(
            &target_issue,
            generated_issues.len(),
            updated_issues.len(),
            cancelled_issues.len(),
            kept_issues.len(),
        );

        Ok(DrawSourceSyncResult {
            lottery_id: lottery.id.clone(),
            lottery_name: lottery.name.clone(),
            api_snapshot,
            target_issue,
            generated_issues,
            updated_issues,
            cancelled_issues,
            kept_issues,
            message,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DrawControlConfig {
    lottery_id: String,
    enabled: bool,
    draw_number: Option<String>,
    target_scope: DrawControlTargetScope,
    target_issue: Option<String>,
    target_order_id: Option<String>,
    updated_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
/// 开奖期号与开奖控制运行时数据快照，用于内存模式和数据库持久化前的业务校验。
struct DrawControlStore {
    controls: BTreeMap<String, DrawControlConfig>,
}

/// 开奖期号与开奖控制运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl DrawControlStore {
    /// 按标识查询并返回单条记录。
    fn get(&self, lottery: &LotteryKind) -> ApiResult<LotteryDrawControl> {
        Ok(self.summary_for(lottery))
    }

    /// 保存控奖配置到当前仓储快照。
    fn save(&mut self, config: DrawControlConfig) {
        self.controls.insert(config.lottery_id.clone(), config);
    }

    /// 判断控奖配置是否命中当前期号并返回控制号码。
    fn active_draw_number(&self, issue: &DrawIssue) -> Option<String> {
        self.controls.get(&issue.lottery_id).and_then(|config| {
            if config.enabled && config.matches_issue(issue) {
                config.draw_number.clone()
            } else {
                None
            }
        })
    }

    /// 按彩种组装后台控奖配置摘要。
    fn summary_for(&self, lottery: &LotteryKind) -> LotteryDrawControl {
        let config = self.controls.get(&lottery.id);
        let enabled = config.is_some_and(|value| {
            value.enabled
                && matches!(value.target_scope, DrawControlTargetScope::Issue)
                && value
                    .target_issue
                    .as_deref()
                    .is_some_and(|issue| !issue.trim().is_empty())
        });
        LotteryDrawControl {
            lottery_id: lottery.id.clone(),
            lottery_name: lottery.name.clone(),
            number_type: lottery.number_type.clone(),
            enabled,
            draw_number: config.and_then(|value| {
                if enabled {
                    value.draw_number.clone()
                } else {
                    None
                }
            }),
            target_scope: config
                .map(|value| value.target_scope.clone())
                .filter(|scope| matches!(scope, DrawControlTargetScope::Issue))
                .unwrap_or(DrawControlTargetScope::Issue),
            target_issue: config.and_then(|value| {
                if enabled {
                    value.target_issue.clone()
                } else {
                    None
                }
            }),
            target_order_id: None,
            updated_at: config.map(|value| value.updated_at.clone()),
        }
    }
}

/// 开奖控制配置的目标范围校验方法。
impl DrawControlConfig {
    /// 判断当前控制配置是否应该作用在传入期号上。
    fn matches_issue(&self, issue: &DrawIssue) -> bool {
        if self.lottery_id != issue.lottery_id {
            return false;
        }

        match self.target_scope {
            DrawControlTargetScope::Lottery => false,
            DrawControlTargetScope::Issue | DrawControlTargetScope::Order => self
                .target_issue
                .as_deref()
                .is_some_and(|target_issue| target_issue == issue.issue),
        }
    }
}

/// 校验新期号创建请求是否满足彩种和时间规则。
fn validate_create_request(
    lottery: &LotteryKind,
    payload: &CreateDrawIssueRequest,
) -> ApiResult<()> {
    if payload.lottery_id.trim().is_empty() {
        return Err(ApiError::BadRequest("lottery id is required".to_string()));
    }
    if payload.lottery_id.trim() != lottery.id {
        return Err(ApiError::BadRequest(
            "request lottery id does not match lottery".to_string(),
        ));
    }
    if payload.issue.trim().is_empty() {
        return Err(ApiError::BadRequest("issue is required".to_string()));
    }
    if payload.scheduled_at.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "scheduled time is required".to_string(),
        ));
    }
    if payload.sale_closed_at.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "sale close time is required".to_string(),
        ));
    }
    Ok(())
}

/// 标准化输入并返回规范值。
fn normalize_control_draw_number(
    lottery: &LotteryKind,
    payload: &SaveLotteryDrawControlRequest,
) -> ApiResult<Option<String>> {
    let draw_number = payload
        .draw_number
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if payload.enabled && draw_number.is_none() {
        return Err(ApiError::BadRequest(
            "control draw number is required".to_string(),
        ));
    }

    draw_number
        .map(|value| normalize_draw_number(value, &lottery.number_type))
        .transpose()
}

/// 标准化开奖控制范围，关闭控制时清空目标以避免误读历史配置。
fn normalize_control_target(
    payload: &SaveLotteryDrawControlRequest,
) -> ApiResult<(DrawControlTargetScope, Option<String>, Option<String>)> {
    if !payload.enabled {
        return Ok((DrawControlTargetScope::Issue, None, None));
    }

    match payload.target_scope {
        DrawControlTargetScope::Lottery => {
            let issue = required_control_target(payload.target_issue.as_deref(), "控制期号")?;
            Ok((DrawControlTargetScope::Issue, Some(issue), None))
        }
        DrawControlTargetScope::Issue => {
            let issue = required_control_target(payload.target_issue.as_deref(), "控制期号")?;
            Ok((DrawControlTargetScope::Issue, Some(issue), None))
        }
        DrawControlTargetScope::Order => {
            let issue = required_control_target(payload.target_issue.as_deref(), "目标订单期号")?;
            Ok((DrawControlTargetScope::Issue, Some(issue), None))
        }
    }
}

/// 读取并修剪控制目标字段，确保启用控制时不会保存空目标。
fn required_control_target(value: Option<&str>, label: &str) -> ApiResult<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    value
        .map(ToString::to_string)
        .ok_or_else(|| ApiError::BadRequest(format!("{label}不能为空")))
}

/// 标准化输入并返回规范值。
pub(crate) fn normalize_draw_number(
    draw_number: &str,
    number_type: &LotteryNumberType,
) -> ApiResult<String> {
    let spec = draw_number_spec(number_type);
    let digits = draw_number_digits(draw_number, number_type)?;

    if digits.len() != spec.len {
        return Err(ApiError::BadRequest(format!(
            "draw number must contain {} numbers",
            spec.len
        )));
    }
    for digit in &digits {
        if *digit < spec.min || *digit > spec.max {
            return Err(ApiError::BadRequest(format!(
                "draw number must be between {} and {}",
                spec.min, spec.max
            )));
        }
    }
    if spec.unique {
        let mut seen = Vec::new();
        for digit in &digits {
            if seen.contains(digit) {
                return Err(ApiError::BadRequest(
                    "draw number must not contain duplicate numbers".to_string(),
                ));
            }
            seen.push(*digit);
        }
    }

    Ok(format_draw_number(&digits))
}

/// 解析并校验开奖号码位数和数字范围。
pub(crate) fn draw_number_digits(
    draw_number: &str,
    number_type: &LotteryNumberType,
) -> ApiResult<Vec<u8>> {
    let value = draw_number.trim();
    if value.contains(',') || value.contains('，') {
        return value
            .split([',', '，'])
            .map(|part| parse_draw_digit(part.trim()))
            .collect();
    }

    if !matches!(
        number_type,
        LotteryNumberType::ThreeDigit | LotteryNumberType::FiveDigit
    ) {
        return Err(ApiError::BadRequest(
            "draw number must contain numbers separated by commas".to_string(),
        ));
    }

    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "draw number must contain numbers separated by commas".to_string(),
        ));
    }

    Ok(value.bytes().map(|byte| byte - b'0').collect())
}

/// 解析输入并返回结构化值。
fn parse_draw_digit(value: &str) -> ApiResult<u8> {
    if value.is_empty() || value.len() > 2 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "draw number must contain numbers separated by commas".to_string(),
        ));
    }

    value.parse::<u8>().map_err(|_| {
        ApiError::BadRequest("draw number must contain numbers separated by commas".to_string())
    })
}

/// 按固定格式转换输出。
pub(crate) fn format_draw_number(digits: &[u8]) -> String {
    digits
        .iter()
        .map(|digit| digit.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// 按号码类型生成平台随机开奖号码。
pub(crate) fn generated_draw_number(
    number_type: &LotteryNumberType,
    lottery_id: &str,
    issue: &str,
) -> String {
    let spec = draw_number_spec(number_type);
    let mut seed = 14_695_981_039_346_656_037u64;
    for byte in lottery_id.bytes().chain(issue.bytes()) {
        seed ^= u64::from(byte);
        seed = seed.wrapping_mul(1_099_511_628_211);
    }

    let mut digits = Vec::with_capacity(spec.len);
    let range = u64::from(spec.max - spec.min + 1);
    for index in 0..(spec.len * 20) {
        seed = seed
            .wrapping_mul(1_103_515_245)
            .wrapping_add(12_345 + index as u64);
        let digit = spec.min + (seed % range) as u8;
        if spec.unique && digits.contains(&digit) {
            continue;
        }
        digits.push(digit);
        if digits.len() == spec.len {
            break;
        }
    }
    if spec.unique && digits.len() < spec.len {
        for digit in spec.min..=spec.max {
            if !digits.contains(&digit) {
                digits.push(digit);
            }
            if digits.len() == spec.len {
                break;
            }
        }
    }

    format_draw_number(&digits)
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct DrawNumberSpec {
    pub(crate) len: usize,
    pub(crate) min: u8,
    pub(crate) max: u8,
    pub(crate) unique: bool,
}

/// 返回不同彩种号码类型的开奖号码长度、范围和是否去重。
pub(crate) fn draw_number_spec(number_type: &LotteryNumberType) -> DrawNumberSpec {
    match number_type {
        LotteryNumberType::ThreeDigit => DrawNumberSpec {
            len: 3,
            min: 0,
            max: 9,
            unique: false,
        },
        LotteryNumberType::FiveDigit => DrawNumberSpec {
            len: 5,
            min: 0,
            max: 9,
            unique: false,
        },
        LotteryNumberType::Pk10 => DrawNumberSpec {
            len: 10,
            min: 1,
            max: 10,
            unique: true,
        },
        LotteryNumberType::ElevenFive => DrawNumberSpec {
            len: 5,
            min: 1,
            max: 11,
            unique: true,
        },
        LotteryNumberType::FastThree => DrawNumberSpec {
            len: 3,
            min: 1,
            max: 6,
            unique: false,
        },
        LotteryNumberType::LuckTwenty => DrawNumberSpec {
            len: 20,
            min: 1,
            max: 80,
            unique: true,
        },
    }
}

/// 生成当前本地时间字符串。
fn current_timestamp_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

/// 汇总同步动作结果，给后台 Toast 和排查日志直接展示中文说明。
fn sync_result_message(
    target_issue: &DrawIssue,
    generated_count: usize,
    updated_count: usize,
    cancelled_count: usize,
    kept_count: usize,
) -> String {
    let action = if generated_count > 0 {
        "已生成"
    } else if updated_count > 0 {
        "已更新"
    } else {
        "已确认"
    };
    let kept_text = if kept_count > 0 {
        format!("，{kept_count} 个有待开奖订单的旧期已保留")
    } else {
        String::new()
    };

    format!(
        "{action}第 {} 期，取消 {} 个无订单旧期{}",
        target_issue.issue, cancelled_count, kept_text
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::{
        domain::{
            draw::{
                CreateDrawIssueRequest, DrawControlTargetScope, DrawIssueResultRequest,
                DrawIssueStatus, SaveLotteryDrawControlRequest,
            },
            lottery::{DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType},
        },
        services::{
            draw::{DrawRepository, DrawStore},
            draw_api::ApiDrawSourceRepository,
            pagination::PageRequest,
        },
    };

    const API68_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "操作成功",
        "result": {
            "businessCode": 0,
            "message": "操作成功",
            "data": [
                { "preDrawIssue": 2026143, "preDrawCode": "3,7,6", "preDrawTime": "2026-06-02 21:15:00" }
            ]
        }
    }"#;
    const KJ_TXFFC_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "",
        "result": {
            "businessCode": "202606031178",
            "message": "",
            "data": {
                "lotKey": "txffc",
                "lotName": "腾讯分分彩",
                "preDrawIssue": "202606031178",
                "preDrawCode": "9,9,8,7,2",
                "preDrawTime": "2026-06-03 19:38:01",
                "drawIssue": 202606031179,
                "drawTime": "2026-06-03 19:39:00"
            }
        }
    }"#;

    #[test]
    /// 验证期号可以创建并按流程封盘。
    fn store_creates_and_closes_draw_issue() {
        let lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        let mut store = DrawStore::default();

        let issue = store
            .create(&lottery, create_request("2026156"))
            .expect("issue can be created");
        let found = store
            .get_by_lottery_issue("fc3d", "2026156")
            .expect("issue can be found by lottery and issue");
        let closed = store.close(&issue.id).expect("issue can be closed");

        assert_eq!(issue.status, DrawIssueStatus::Open);
        assert_eq!(found.id, issue.id);
        assert_eq!(closed.status, DrawIssueStatus::Closed);
    }

    #[test]
    /// 验证一键清理只删除已结算的已开奖期号，未结算期号仍保留用于计奖派奖。
    fn store_clear_settled_drawn_issues_keeps_unsettled_drawn_issue() {
        let lottery = lottery(DrawMode::Manual, LotteryNumberType::ThreeDigit);
        let mut store = DrawStore::default();
        let open = store
            .create(&lottery, create_request("20260602-open"))
            .expect("open issue can be created");
        let closed = store
            .create(&lottery, create_request("20260602-closed"))
            .expect("closed issue can be created");
        store.close(&closed.id).expect("issue can be closed");
        let cancelled = store
            .create(&lottery, create_request("20260602-cancelled"))
            .expect("cancelled issue can be created");
        store.cancel(&cancelled.id).expect("issue can be cancelled");
        let drawn = store
            .create(&lottery, create_request("20260602-drawn"))
            .expect("drawn issue can be created");
        store
            .draw(
                &drawn.id,
                DrawIssueResultRequest {
                    draw_number: Some("1,2,3".to_string()),
                },
                false,
            )
            .expect("issue can be drawn");

        let deleted_count = store.clear_settled_drawn_issues(&BTreeSet::new());

        assert_eq!(deleted_count, 0);
        assert_eq!(
            store
                .get(&drawn.id)
                .expect("unsettled drawn issue exists")
                .status,
            DrawIssueStatus::Drawn
        );

        let settled_ids = BTreeSet::from([drawn.id.clone()]);
        let deleted_count = store.clear_settled_drawn_issues(&settled_ids);

        assert_eq!(deleted_count, 1);
        assert!(store.get(&drawn.id).is_err());
        assert_eq!(
            store.get(&open.id).expect("open issue exists").status,
            DrawIssueStatus::Open
        );
        assert_eq!(
            store.get(&closed.id).expect("closed issue exists").status,
            DrawIssueStatus::Closed
        );
        assert_eq!(
            store
                .get(&cancelled.id)
                .expect("cancelled issue exists")
                .status,
            DrawIssueStatus::Cancelled
        );
    }

    #[tokio::test]
    /// 验证期号分页入口可以按状态过滤，后台列表不会只在前端做假筛选。
    async fn repository_list_page_filters_by_status() {
        let repository = DrawRepository::memory();
        let lottery = lottery(DrawMode::Manual, LotteryNumberType::ThreeDigit);
        let open = repository
            .create(&lottery, create_request("20260602-open"))
            .await
            .expect("open issue can be created");
        let closed = repository
            .create(&lottery, create_request("20260602-closed"))
            .await
            .expect("closed issue can be created");
        repository
            .close(&closed.id)
            .await
            .expect("issue can be closed");

        let page = repository
            .list_page(
                Some("fc3d"),
                Some(DrawIssueStatus::Closed),
                PageRequest::new(Some(1), Some(20)),
            )
            .await
            .expect("filtered page can be loaded");

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, closed.id);
        assert!(page.items.iter().all(|issue| issue.id != open.id));
    }

    #[test]
    /// 验证人工计划开奖必须提供合法开奖号码。
    fn manual_draw_requires_valid_draw_number() {
        let lottery = lottery(DrawMode::Manual, LotteryNumberType::FiveDigit);
        let mut store = DrawStore::default();
        let issue = store
            .create(&lottery, create_request("20260602-test"))
            .expect("issue can be created");

        assert!(store
            .draw(
                &issue.id,
                DrawIssueResultRequest { draw_number: None },
                false
            )
            .expect_err("manual draw without number is invalid")
            .to_string()
            .contains("manual draw requires draw number"));

        let drawn = store
            .draw(
                &issue.id,
                DrawIssueResultRequest {
                    draw_number: Some("7,8,9,4,2".to_string()),
                },
                false,
            )
            .expect("manual draw can be recorded");

        assert_eq!(drawn.status, DrawIssueStatus::Drawn);
        assert_eq!(drawn.draw_number.as_deref(), Some("7,8,9,4,2"));
    }

    #[test]
    /// 验证平台开奖会按号码类型生成正确位数。
    fn platform_draw_generates_number_for_number_type() {
        let lottery = lottery(DrawMode::Platform, LotteryNumberType::FiveDigit);
        let mut store = DrawStore::default();
        let issue = store
            .create(&lottery, create_request("20260602-001"))
            .expect("issue can be created");

        let drawn = store
            .draw(&issue.id, DrawIssueResultRequest::default(), false)
            .expect("platform draw can be generated");

        let draw_number = drawn.draw_number.expect("draw number exists");
        assert_eq!(draw_number.split(',').count(), 5);
        assert!(draw_number
            .split(',')
            .all(|part| part.len() == 1 && part.bytes().all(|byte| byte.is_ascii_digit())));
    }

    #[test]
    /// 验证命中控奖配置时优先使用控制号码。
    fn platform_draw_uses_control_number_when_resolved() {
        let lottery = lottery(DrawMode::Platform, LotteryNumberType::FiveDigit);
        let mut store = DrawStore::default();
        let issue = store
            .create(&lottery, create_request("20260602-control"))
            .expect("issue can be created");

        let drawn = store
            .draw(
                &issue.id,
                DrawIssueResultRequest {
                    draw_number: Some("7,8,9,4,2".to_string()),
                },
                true,
            )
            .expect("platform control draw can be recorded");

        assert_eq!(drawn.status, DrawIssueStatus::Drawn);
        assert_eq!(drawn.draw_number.as_deref(), Some("7,8,9,4,2"));
    }

    #[test]
    /// 验证已开奖期号不能取消或重复开奖。
    fn drawn_issue_cannot_be_cancelled_or_redrawn() {
        let lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        let mut store = DrawStore::default();
        let issue = store
            .create(&lottery, create_request("2026156"))
            .expect("issue can be created");

        store
            .draw(&issue.id, DrawIssueResultRequest::default(), false)
            .expect("issue can be drawn");

        assert!(store
            .cancel(&issue.id)
            .expect_err("drawn issue cannot be cancelled")
            .to_string()
            .contains("drawn draw issue cannot be cancelled"));
        assert!(store
            .draw(&issue.id, DrawIssueResultRequest::default(), false)
            .expect_err("drawn issue cannot be drawn again")
            .to_string()
            .contains("draw issue cannot be drawn in current status"));
    }
    /// 验证仓储使用api68来源用于API开奖。
    #[tokio::test]
    async fn repository_uses_api68_source_for_api_draw() {
        let lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        let repository = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let issue = repository
            .create(&lottery, create_request("2026143"))
            .await
            .expect("issue can be created");

        let drawn = repository
            .draw(&issue.id, DrawIssueResultRequest::default())
            .await
            .expect("api draw can be resolved");

        assert_eq!(drawn.status, DrawIssueStatus::Drawn);
        assert_eq!(drawn.draw_number.as_deref(), Some("3,7,6"));
    }
    /// 验证仓储control号码overridesAPI来源。
    #[tokio::test]
    async fn repository_control_number_overrides_api_source() {
        let lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        let repository = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        repository
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
                    target_scope: DrawControlTargetScope::Issue,
                    target_issue: Some("2026143".to_string()),
                    target_order_id: None,
                },
            )
            .await
            .expect("draw control can be saved");
        let issue = repository
            .create(&lottery, create_request("2026143"))
            .await
            .expect("issue can be created");

        let drawn = repository
            .draw(&issue.id, DrawIssueResultRequest::default())
            .await
            .expect("api draw can be controlled");

        assert_eq!(drawn.status, DrawIssueStatus::Drawn);
        assert_eq!(drawn.draw_number.as_deref(), Some("2,4,7"));
    }
    /// 验证仓储期号scopedcontrol仅匹配target期号。
    #[tokio::test]
    async fn repository_issue_scoped_control_only_matches_target_issue() {
        let lottery = lottery(DrawMode::Platform, LotteryNumberType::ThreeDigit);
        let repository = DrawRepository::memory();
        repository
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
                    target_scope: DrawControlTargetScope::Issue,
                    target_issue: Some("2026143".to_string()),
                    target_order_id: None,
                },
            )
            .await
            .expect("issue control can be saved");
        let target_issue = repository
            .create(&lottery, create_request("2026143"))
            .await
            .expect("target issue can be created");
        let other_issue = repository
            .create(&lottery, create_request("2026144"))
            .await
            .expect("other issue can be created");

        let target_drawn = repository
            .draw(&target_issue.id, DrawIssueResultRequest::default())
            .await
            .expect("target issue can use control number");
        let other_drawn = repository
            .draw(&other_issue.id, DrawIssueResultRequest::default())
            .await
            .expect("other issue should use platform generated number");

        assert_eq!(target_drawn.draw_number.as_deref(), Some("2,4,7"));
        assert_ne!(other_drawn.draw_number.as_deref(), Some("2,4,7"));
    }
    /// 验证仓储顺序scopedcontrol匹配顺序期号。
    #[tokio::test]
    async fn repository_order_scoped_control_matches_order_issue() {
        let lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        let repository = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        repository
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
                    target_scope: DrawControlTargetScope::Issue,
                    target_issue: Some("2026143".to_string()),
                    target_order_id: None,
                },
            )
            .await
            .expect("issue control can be saved");
        let issue = repository
            .create(&lottery, create_request("2026143"))
            .await
            .expect("issue can be created");

        let drawn = repository
            .draw(&issue.id, DrawIssueResultRequest::default())
            .await
            .expect("issue scoped control can override api draw");

        assert_eq!(drawn.draw_number.as_deref(), Some("2,4,7"));
    }
    /// 验证仓储保存开奖controlvalidates号码type。
    #[tokio::test]
    async fn repository_save_draw_control_validates_number_type() {
        let lottery = lottery(DrawMode::Platform, LotteryNumberType::FiveDigit);
        let repository = DrawRepository::memory();

        let error = repository
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
                    target_scope: DrawControlTargetScope::Issue,
                    target_issue: Some("2026143".to_string()),
                    target_order_id: None,
                },
            )
            .await
            .expect_err("short draw control number is rejected");

        assert!(error
            .to_string()
            .contains("draw number must contain 5 numbers"));
    }
    /// 验证仓储保存开奖control拒绝missingtarget期号。
    #[tokio::test]
    async fn repository_save_draw_control_rejects_missing_target_issue() {
        let lottery = lottery(DrawMode::Platform, LotteryNumberType::ThreeDigit);
        let repository = DrawRepository::memory();

        let error = repository
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
                    target_scope: DrawControlTargetScope::Issue,
                    target_issue: None,
                    target_order_id: None,
                },
            )
            .await
            .expect_err("issue scoped control requires target issue");

        assert!(error.to_string().contains("控制期号不能为空"));
    }
    /// 验证仓储保存开奖control拒绝停用彩种control。
    #[tokio::test]
    async fn repository_save_draw_control_rejects_disabled_lottery_control() {
        let mut lottery = lottery(DrawMode::Platform, LotteryNumberType::ThreeDigit);
        lottery.draw_control_enabled = false;
        let repository = DrawRepository::memory();

        let error = repository
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
                    target_scope: DrawControlTargetScope::Issue,
                    target_issue: Some("2026143".to_string()),
                    target_order_id: None,
                },
            )
            .await
            .expect_err("disabled lottery control cannot be enabled");

        assert!(error.to_string().contains("未开启开奖号码控制"));
    }

    #[test]
    /// 新增号码类型按各自长度、范围和去重规则校验开奖号码。
    fn normalize_draw_number_supports_new_lottery_number_types() {
        assert_eq!(
            super::normalize_draw_number("01,06,02,04,03,05,07,09,10,08", &LotteryNumberType::Pk10)
                .expect("pk10 draw number is valid"),
            "1,6,2,4,3,5,7,9,10,8"
        );
        assert_eq!(
            super::normalize_draw_number("1,3,5,7,11", &LotteryNumberType::ElevenFive)
                .expect("eleven five draw number is valid"),
            "1,3,5,7,11"
        );
        assert_eq!(
            super::normalize_draw_number("6,6,1", &LotteryNumberType::FastThree)
                .expect("fast three draw number can repeat"),
            "6,6,1"
        );

        let duplicated =
            super::normalize_draw_number("1,2,3,4,5,6,7,8,9,9", &LotteryNumberType::Pk10)
                .expect_err("pk10 duplicate is rejected");
        assert!(duplicated.to_string().contains("duplicate"));

        let compact = super::normalize_draw_number("12345678910", &LotteryNumberType::Pk10)
            .expect_err("pk10 compact format is rejected");
        assert!(compact.to_string().contains("separated by commas"));
    }
    /// 验证仓储reusesapi68来源用于pl3API开奖。
    #[tokio::test]
    async fn repository_reuses_api68_source_for_pl3_api_draw() {
        let mut lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        lottery.id = "pl3".to_string();
        lottery.name = "排列 3".to_string();
        let repository = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let issue = repository
            .create(&lottery, create_request_for("pl3", "2026143"))
            .await
            .expect("issue can be created");

        let drawn = repository
            .draw(&issue.id, DrawIssueResultRequest::default())
            .await
            .expect("pl3 can reuse api68 draw");

        assert_eq!(drawn.status, DrawIssueStatus::Drawn);
        assert_eq!(drawn.draw_number.as_deref(), Some("3,7,6"));
    }
    /// 验证仓储拒绝API开奖when来源misses期号。
    #[tokio::test]
    async fn repository_rejects_api_draw_when_source_misses_issue() {
        let lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        let repository = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let issue = repository
            .create(&lottery, create_request("2099999"))
            .await
            .expect("issue can be created");

        let error = repository
            .draw(&issue.id, DrawIssueResultRequest::default())
            .await
            .expect_err("api draw without matching issue is rejected");
        let stored = repository.get(&issue.id).await.expect("issue still exists");

        assert!(error.to_string().contains("not found"));
        assert_eq!(stored.status, DrawIssueStatus::Open);
        assert!(stored.draw_number.is_none());
    }
    /// 验证仓储drawsAPI期号带prefetched号码withoutrefetching来源。
    #[tokio::test]
    async fn repository_draws_api_issue_with_prefetched_number_without_refetching_source() {
        let lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        let repository = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE),
        );
        let issue = repository
            .create(&lottery, create_request("2099999"))
            .await
            .expect("issue can be created");

        let drawn = repository
            .draw_with_prefetched_api_number_with_control_policy(
                &issue.id,
                Some("1,2,3".to_string()),
                true,
            )
            .await
            .expect("prefetched draw number can be used");

        assert_eq!(drawn.status, DrawIssueStatus::Drawn);
        assert_eq!(drawn.draw_number.as_deref(), Some("1,2,3"));
    }
    /// 验证仓储syncAPI开奖来源生成target和取消stale期号。
    #[tokio::test]
    async fn repository_sync_api_draw_source_generates_target_and_cancels_stale_issue() {
        let lottery = txffc_lottery();
        let repository = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::kj_seeded_with_static_response(KJ_TXFFC_SAMPLE),
        );
        let stale_issue = repository
            .create(&lottery, txffc_create_request("202606031170"))
            .await
            .expect("stale issue can be created");

        let result = repository
            .sync_api_draw_source(&lottery, "2026-06-03 19:38:20", 1, &BTreeSet::new())
            .await
            .expect("api source can be synced");
        let stored_stale = repository
            .get(&stale_issue.id)
            .await
            .expect("stale issue still exists");

        assert_eq!(result.target_issue.issue, "202606031179");
        assert_eq!(result.target_issue.scheduled_at, "2026-06-03 19:39:00");
        assert_eq!(result.generated_issues.len(), 1);
        assert_eq!(result.cancelled_issues.len(), 1);
        assert_eq!(stored_stale.status, DrawIssueStatus::Cancelled);
    }
    /// 验证仓储syncAPI开奖来源keepsstale期号带待处理订单。
    #[tokio::test]
    async fn repository_sync_api_draw_source_keeps_stale_issue_with_pending_orders() {
        let lottery = txffc_lottery();
        let repository = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::kj_seeded_with_static_response(KJ_TXFFC_SAMPLE),
        );
        let stale_issue = repository
            .create(&lottery, txffc_create_request("202606031170"))
            .await
            .expect("stale issue can be created");
        let mut protected_issues = BTreeSet::new();
        protected_issues.insert("202606031170".to_string());

        let result = repository
            .sync_api_draw_source(&lottery, "2026-06-03 19:38:20", 1, &protected_issues)
            .await
            .expect("api source can be synced");
        let stored_stale = repository
            .get(&stale_issue.id)
            .await
            .expect("stale issue still exists");

        assert_eq!(result.target_issue.issue, "202606031179");
        assert_eq!(result.cancelled_issues.len(), 0);
        assert_eq!(result.kept_issues.len(), 1);
        assert_eq!(stored_stale.status, DrawIssueStatus::Open);
    }

    /// 构造测试用创建请求。
    fn create_request(issue: &str) -> CreateDrawIssueRequest {
        create_request_for("fc3d", issue)
    }

    /// 按指定彩种和期号构造测试请求。
    fn create_request_for(lottery_id: &str, issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: lottery_id.to_string(),
            issue: issue.to_string(),
            scheduled_at: "2026-06-02 21:00:15".to_string(),
            sale_closed_at: "2026-06-02 20:59:45".to_string(),
        }
    }

    /// 构造测试或种子使用的彩种配置。
    fn lottery(draw_mode: DrawMode, number_type: LotteryNumberType) -> LotteryKind {
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            category: "regional".to_string(),
            logo_url: String::new(),
            number_type,
            draw_mode,
            api_draw_delay_seconds: 0,
            draw_control_enabled: true,
            avoid_winning_enabled: false,
            issue_format: crate::domain::lottery::DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            sale_close_lead_seconds: crate::domain::lottery::DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: true,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1000,
            },
            play_categories: Vec::new(),
            play_configs: Vec::new(),
        }
    }
    /// 构造腾讯分分彩测试期号创建请求。
    fn txffc_create_request(issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: "txffc".to_string(),
            issue: issue.to_string(),
            scheduled_at: "2026-06-03 19:30:00".to_string(),
            sale_closed_at: "2026-06-03 19:29:59".to_string(),
        }
    }
    /// 构造腾讯分分彩测试彩种配置。
    fn txffc_lottery() -> LotteryKind {
        let mut lottery = lottery(DrawMode::Api, LotteryNumberType::FiveDigit);
        lottery.id = "txffc".to_string();
        lottery.name = "腾讯分分彩".to_string();
        lottery.category = "overseas".to_string();
        lottery.schedule = DrawSchedule::Periodic {
            interval_seconds: 60,
        };
        lottery
    }
}
