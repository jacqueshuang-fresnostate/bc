//! 开奖期号与开奖控制领域模型，定义状态与开奖请求参数

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::{
        draw::{
            ApiDrawSourceIssueSnapshot, CreateDrawIssueRequest, DrawControlTargetScope, DrawIssue,
            DrawIssueGenerationPreview, DrawIssueResultRequest, DrawIssueStatus,
            DrawSourceSyncResult, LotteryDrawControl, SaveLotteryDrawControlRequest,
        },
        lottery::{DrawMode, DrawSource, LotteryKind, LotteryNumberType, SaveDrawSourceRequest},
    },
    error::{ApiError, ApiResult},
};

use super::{
    business_database::{enum_from_string, enum_to_string, BusinessDatabase},
    draw_api::{ApiDrawSourceLatestIssue, ApiDrawSourceRepository},
    draw_generation::plan_api_draw_source_target,
};

#[derive(Clone)]
/// 开奖期号与开奖控制仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct DrawRepository {
    inner: Arc<RwLock<DrawStore>>,
    api_sources: ApiDrawSourceRepository,
    controls: Arc<RwLock<DrawControlStore>>,
    persistence: Option<BusinessDatabase>,
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
        })
    }

    /// 返回完整列表。
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

    /// 按 ID 查询单条记录。
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
        Ok(result)
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
    pub async fn draw(&self, id: &str, payload: DrawIssueResultRequest) -> ApiResult<DrawIssue> {
        let (payload, uses_control_number) = self.resolve_draw_payload(id, payload).await?;

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

    /// 使用已经并发预取的 API 开奖号码完成开奖，避免自动调度阶段重复等待外部接口。
    pub async fn draw_with_prefetched_api_number(
        &self,
        id: &str,
        api_draw_number: Option<String>,
    ) -> ApiResult<DrawIssue> {
        let (payload, uses_control_number) = self
            .resolve_prefetched_api_draw_payload(id, api_draw_number)
            .await?;

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

    async fn resolve_draw_payload(
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

    async fn resolve_prefetched_api_draw_payload(
        &self,
        id: &str,
        api_draw_number: Option<String>,
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

    async fn persist_draws(&self, store: &DrawStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_draw_issues(persistence, store).await?;
        }

        Ok(())
    }

    async fn persist_draw_issue(&self, issue: &DrawIssue) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            upsert_draw_issue(persistence, issue).await?;
        }

        Ok(())
    }

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
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("开奖期号数据读取失败".to_string()))?;
        issues.insert(
            id.clone(),
            DrawIssue {
                id,
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
            },
        );
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
    /// 返回完整数据列表。
    fn list(&self) -> Vec<DrawIssue> {
        self.issues.values().rev().cloned().collect()
    }

    /// 处理 list_by_lottery_id 的具体内部流程。
    fn list_by_lottery_id(&self, lottery_id: &str) -> Vec<DrawIssue> {
        self.issues
            .values()
            .rev()
            .filter(|issue| issue.lottery_id == lottery_id)
            .cloned()
            .collect()
    }

    /// 按标识查询并返回单条记录。
    fn get(&self, id: &str) -> ApiResult<DrawIssue> {
        self.issues
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("draw issue `{id}` not found")))
    }

    /// 处理 get_by_lottery_issue 的具体内部流程。
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

    /// 处理 close 的具体内部流程。
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

    /// 处理 draw 的具体内部流程。
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

    /// 处理 cancel 的具体内部流程。
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

    /// 处理 save 的具体内部流程。
    fn save(&mut self, config: DrawControlConfig) {
        self.controls.insert(config.lottery_id.clone(), config);
    }

    /// 处理 active_draw_number 的具体内部流程。
    fn active_draw_number(&self, issue: &DrawIssue) -> Option<String> {
        self.controls.get(&issue.lottery_id).and_then(|config| {
            if config.enabled && config.matches_issue(issue) {
                config.draw_number.clone()
            } else {
                None
            }
        })
    }

    /// 处理 summary_for 的具体内部流程。
    fn summary_for(&self, lottery: &LotteryKind) -> LotteryDrawControl {
        let config = self.controls.get(&lottery.id);
        LotteryDrawControl {
            lottery_id: lottery.id.clone(),
            lottery_name: lottery.name.clone(),
            number_type: lottery.number_type.clone(),
            enabled: config.is_some_and(|value| value.enabled),
            draw_number: config.and_then(|value| value.draw_number.clone()),
            target_scope: config
                .map(|value| value.target_scope.clone())
                .unwrap_or_default(),
            target_issue: config.and_then(|value| value.target_issue.clone()),
            target_order_id: config.and_then(|value| value.target_order_id.clone()),
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
            DrawControlTargetScope::Lottery => true,
            DrawControlTargetScope::Issue | DrawControlTargetScope::Order => self
                .target_issue
                .as_deref()
                .is_some_and(|target_issue| target_issue == issue.issue),
        }
    }
}

/// 校验输入参数并返回校验结果。
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
        return Ok((DrawControlTargetScope::Lottery, None, None));
    }

    match payload.target_scope {
        DrawControlTargetScope::Lottery => Ok((DrawControlTargetScope::Lottery, None, None)),
        DrawControlTargetScope::Issue => {
            let issue = required_control_target(payload.target_issue.as_deref(), "控制期号")?;
            Ok((DrawControlTargetScope::Issue, Some(issue), None))
        }
        DrawControlTargetScope::Order => {
            let issue = required_control_target(payload.target_issue.as_deref(), "目标订单期号")?;
            let order_id = required_control_target(payload.target_order_id.as_deref(), "目标订单")?;
            Ok((DrawControlTargetScope::Order, Some(issue), Some(order_id)))
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
fn normalize_draw_number(draw_number: &str, number_type: &LotteryNumberType) -> ApiResult<String> {
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

/// 处理 draw_number_digits 的具体内部流程。
fn draw_number_digits(draw_number: &str, number_type: &LotteryNumberType) -> ApiResult<Vec<u8>> {
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
fn format_draw_number(digits: &[u8]) -> String {
    digits
        .iter()
        .map(|digit| digit.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// 处理 generated_draw_number 的具体内部流程。
fn generated_draw_number(number_type: &LotteryNumberType, lottery_id: &str, issue: &str) -> String {
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
struct DrawNumberSpec {
    len: usize,
    min: u8,
    max: u8,
    unique: bool,
}

/// 返回不同彩种号码类型的开奖号码长度、范围和是否去重。
fn draw_number_spec(number_type: &LotteryNumberType) -> DrawNumberSpec {
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

/// 处理 current_timestamp_label 的具体内部流程。
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
    /// 处理 store_creates_and_closes_draw_issue 的具体内部流程。
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
    /// 处理 manual_draw_requires_valid_draw_number 的具体内部流程。
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
    /// 处理 platform_draw_generates_number_for_number_type 的具体内部流程。
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
    /// 处理 platform_draw_uses_control_number_when_resolved 的具体内部流程。
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
    /// 处理 drawn_issue_cannot_be_cancelled_or_redrawn 的具体内部流程。
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
                    target_scope: DrawControlTargetScope::Lottery,
                    target_issue: None,
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
                    target_scope: DrawControlTargetScope::Order,
                    target_issue: Some("2026143".to_string()),
                    target_order_id: Some("O000000000001".to_string()),
                },
            )
            .await
            .expect("order control can be saved");
        let issue = repository
            .create(&lottery, create_request("2026143"))
            .await
            .expect("issue can be created");

        let drawn = repository
            .draw(&issue.id, DrawIssueResultRequest::default())
            .await
            .expect("order scoped control can override api draw");

        assert_eq!(drawn.draw_number.as_deref(), Some("2,4,7"));
    }

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
                    target_scope: DrawControlTargetScope::Lottery,
                    target_issue: None,
                    target_order_id: None,
                },
            )
            .await
            .expect_err("short draw control number is rejected");

        assert!(error
            .to_string()
            .contains("draw number must contain 5 numbers"));
    }

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
            .draw_with_prefetched_api_number(&issue.id, Some("1,2,3".to_string()))
            .await
            .expect("prefetched draw number can be used");

        assert_eq!(drawn.status, DrawIssueStatus::Drawn);
        assert_eq!(drawn.draw_number.as_deref(), Some("1,2,3"));
    }

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

    /// 处理 create_request 的具体内部流程。
    fn create_request(issue: &str) -> CreateDrawIssueRequest {
        create_request_for("fc3d", issue)
    }

    /// 处理 create_request_for 的具体内部流程。
    fn create_request_for(lottery_id: &str, issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: lottery_id.to_string(),
            issue: issue.to_string(),
            scheduled_at: "2026-06-02 21:00:15".to_string(),
            sale_closed_at: "2026-06-02 20:59:45".to_string(),
        }
    }

    /// 处理 lottery 的具体内部流程。
    fn lottery(draw_mode: DrawMode, number_type: LotteryNumberType) -> LotteryKind {
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            category: "regional".to_string(),
            logo_url: String::new(),
            number_type,
            draw_mode,
            api_draw_delay_seconds: 0,
            issue_format: crate::domain::lottery::DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
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

    fn txffc_create_request(issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: "txffc".to_string(),
            issue: issue.to_string(),
            scheduled_at: "2026-06-03 19:30:00".to_string(),
            sale_closed_at: "2026-06-03 19:29:59".to_string(),
        }
    }

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
