//! 机器人独立调度服务，负责把常规合买发单和补单从开奖调度链路中拆出。

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tokio::task::JoinHandle;

use crate::{
    domain::robot::GroupBuyRobotRun,
    error::{ApiError, ApiResult},
    services::{
        access::AccessRepository,
        business_database::{
            enum_from_string, enum_to_string, from_json, to_json, BusinessDatabase,
        },
        draw::DrawRepository,
        finance::FinanceRepository,
        group_buy::GroupBuyRepository,
        group_buy_robot::run_group_buy_robots,
        lottery::LotteryRepository,
        order::OrderRepository,
        realtime::{balance_changed_event, order_changed_event, RealtimeHub},
        robot::RobotRepository,
    },
};

const DEFAULT_ROBOT_SCHEDULER_INTERVAL_SECONDS: u64 = 5;
const DISABLED_ROBOT_SCHEDULER_POLL_SECONDS: u64 = 1;
const MAX_ROBOT_SCHEDULER_HISTORY: usize = 20;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 机器人调度器配置，控制常规机器人是否自动执行以及执行周期。
pub struct RobotSchedulerConfig {
    /// 是否启用独立机器人调度。
    pub enabled: bool,
    /// 自动执行周期，单位秒。
    pub interval_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 机器人调度器单轮执行状态。
pub enum RobotSchedulerRunStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 机器人调度触发来源，当前只记录后台常驻自动触发。
pub enum RobotSchedulerRunTrigger {
    Automatic,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 机器人调度器最近执行记录，用于后台查看自动发单和补单健康状态。
pub struct RobotSchedulerRunRecord {
    /// 执行记录 ID。
    pub id: String,
    /// 触发来源。
    pub trigger: RobotSchedulerRunTrigger,
    /// 执行结果。
    pub status: RobotSchedulerRunStatus,
    /// 开始时间。
    pub started_at: String,
    /// 完成时间。
    pub finished_at: String,
    /// 本轮业务时间。
    pub now: String,
    /// 失败原因，成功时为空。
    pub error: Option<String>,
    /// 本轮机器人创建的合买计划数。
    pub created_plan_count: usize,
    /// 本轮机器人补满的合买计划数。
    pub filled_plan_count: usize,
    /// 本轮满单生成的投注订单数。
    pub created_order_count: usize,
    /// 本轮机器人产生的资金流水数。
    pub ledger_entry_count: usize,
    /// 本轮跳过明细数量。
    pub skipped_item_count: usize,
    /// 跳过明细，保留最近一轮用于后台排查。
    pub skipped_items: Vec<crate::domain::robot::GroupBuyRobotSkippedItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台展示的机器人调度器状态。
pub struct RobotSchedulerStatus {
    /// 是否启用独立机器人调度。
    pub enabled: bool,
    /// 当前配置。
    pub config: RobotSchedulerConfig,
    /// 已保留的运行记录数量。
    pub run_count: usize,
    /// 最近一次运行记录。
    pub last_run: Option<RobotSchedulerRunRecord>,
    /// 最近运行记录列表。
    pub recent_runs: Vec<RobotSchedulerRunRecord>,
}

#[derive(Clone)]
/// 机器人调度仓储，保存配置、运行历史和运行时序号。
pub struct RobotSchedulerRepository {
    inner: Arc<RwLock<RobotSchedulerStore>>,
    persistence: Option<BusinessDatabase>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// 机器人调度器运行时快照。
struct RobotSchedulerStore {
    config: RobotSchedulerConfig,
    next_sequence: u64,
    runs: VecDeque<RobotSchedulerRunRecord>,
}

impl Default for RobotSchedulerConfig {
    /// 返回默认关闭的机器人调度配置。
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: DEFAULT_ROBOT_SCHEDULER_INTERVAL_SECONDS,
        }
    }
}

impl RobotSchedulerRepository {
    /// 初始化内存模式机器人调度仓储。
    pub fn new(config: RobotSchedulerConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(RobotSchedulerStore::new(config))),
            persistence: None,
        }
    }

    /// 从数据库加载机器人调度配置和历史记录。
    pub async fn persistent(
        config: RobotSchedulerConfig,
        persistence: BusinessDatabase,
    ) -> ApiResult<Self> {
        let store = load_robot_scheduler_store(&persistence, config).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 读取机器人调度器状态。
    pub fn status(&self) -> ApiResult<RobotSchedulerStatus> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("机器人调度缓存读取失败".to_string()))
            .map(|store| store.status())
    }

    /// 读取机器人调度器配置。
    pub fn config(&self) -> ApiResult<RobotSchedulerConfig> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("机器人调度配置读取失败".to_string()))
            .map(|store| store.config.clone())
    }

    /// 更新机器人调度器配置并持久化。
    pub async fn update_config(
        &self,
        config: RobotSchedulerConfig,
    ) -> ApiResult<RobotSchedulerStatus> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("机器人调度配置更新失败".to_string()))?;
            let result = store.update_config(config)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 记录一次机器人调度成功执行。
    pub async fn record_success(
        &self,
        trigger: RobotSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        run: &GroupBuyRobotRun,
    ) -> ApiResult<RobotSchedulerRunRecord> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("机器人调度历史写入失败".to_string()))?;
            let result = store.record_success(trigger, started_at, finished_at, run)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 记录一次机器人调度失败执行。
    pub async fn record_failure(
        &self,
        trigger: RobotSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        now: String,
        error: String,
    ) -> ApiResult<RobotSchedulerRunRecord> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("机器人调度历史写入失败".to_string()))?;
            let result = store.record_failure(trigger, started_at, finished_at, now, error)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 从数据库重新加载机器人调度配置和最近执行记录，供后台缓存维护按钮使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let current_config = self.config()?;
        let store = load_robot_scheduler_store(persistence, current_config).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("机器人调度缓存刷新失败".to_string()))? = store;
        Ok(true)
    }

    /// 把当前机器人调度快照保存到数据库。
    async fn persist(&self, store: &RobotSchedulerStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_robot_scheduler_store(persistence, store).await?;
        }
        Ok(())
    }
}

/// 从数据库加载机器人调度器配置、历史记录和运行时序号。
async fn load_robot_scheduler_store(
    database: &BusinessDatabase,
    default_config: RobotSchedulerConfig,
) -> ApiResult<RobotSchedulerStore> {
    let pool = database.pool();
    let config = sqlx::query(
        "SELECT enabled, interval_seconds
         FROM robot_scheduler_config
         WHERE id = 'default'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("机器人调度配置数据读取失败".to_string()))?
    .map(|row| {
        let interval_seconds: i64 = row
            .try_get("interval_seconds")
            .map_err(|_| ApiError::Internal("机器人调度配置数据读取失败".to_string()))?;
        Ok(RobotSchedulerConfig {
            enabled: row
                .try_get("enabled")
                .map_err(|_| ApiError::Internal("机器人调度配置数据读取失败".to_string()))?,
            interval_seconds: u64::try_from(interval_seconds)
                .map_err(|_| ApiError::Internal("机器人调度周期数据无效".to_string()))?,
        })
    })
    .transpose()?;

    let Some(config) = config else {
        let seeded = RobotSchedulerStore::new(default_config);
        save_robot_scheduler_store(database, &seeded).await?;
        return Ok(seeded);
    };
    config.validate()?;

    let mut runs = VecDeque::new();
    let mut run_ids = Vec::new();
    for row in sqlx::query(
        "SELECT id, trigger, status, started_at, finished_at, now, error,
                created_plan_count, filled_plan_count, created_order_count,
                ledger_entry_count, skipped_item_count, skipped_items
         FROM robot_scheduler_runs
         ORDER BY id DESC
         LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("机器人调度历史数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("机器人调度历史数据读取失败".to_string()))?;
        run_ids.push(id.clone());
        runs.push_back(RobotSchedulerRunRecord {
            id,
            trigger: enum_from_string(
                row.try_get("trigger")
                    .map_err(|_| ApiError::Internal("机器人调度历史数据读取失败".to_string()))?,
            )?,
            status: enum_from_string(
                row.try_get("status")
                    .map_err(|_| ApiError::Internal("机器人调度历史数据读取失败".to_string()))?,
            )?,
            started_at: row
                .try_get("started_at")
                .map_err(|_| ApiError::Internal("机器人调度历史数据读取失败".to_string()))?,
            finished_at: row
                .try_get("finished_at")
                .map_err(|_| ApiError::Internal("机器人调度历史数据读取失败".to_string()))?,
            now: row
                .try_get("now")
                .map_err(|_| ApiError::Internal("机器人调度历史数据读取失败".to_string()))?,
            error: row
                .try_get("error")
                .map_err(|_| ApiError::Internal("机器人调度历史数据读取失败".to_string()))?,
            created_plan_count: read_usize_count(&row, "created_plan_count")?,
            filled_plan_count: read_usize_count(&row, "filled_plan_count")?,
            created_order_count: read_usize_count(&row, "created_order_count")?,
            ledger_entry_count: read_usize_count(&row, "ledger_entry_count")?,
            skipped_item_count: read_usize_count(&row, "skipped_item_count")?,
            skipped_items: from_json(
                row.try_get("skipped_items")
                    .map_err(|_| ApiError::Internal("机器人调度跳过明细读取失败".to_string()))?,
            )?,
        });
    }

    let next_sequence = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM robot_scheduler_runtime WHERE key = 'next_sequence'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("机器人调度运行数据读取失败".to_string()))?
    .unwrap_or_default();

    Ok(RobotSchedulerStore {
        config,
        next_sequence: u64::try_from(next_sequence)
            .unwrap_or_default()
            .max(max_sequence(&run_ids)),
        runs,
    })
}

/// 保存机器人调度器配置、最近历史和运行时序号。
async fn save_robot_scheduler_store(
    database: &BusinessDatabase,
    store: &RobotSchedulerStore,
) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("机器人调度事务开启失败".to_string()))?;

    for table in [
        "robot_scheduler_runs",
        "robot_scheduler_runtime",
        "robot_scheduler_config",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("机器人调度数据清理失败".to_string()))?;
    }

    sqlx::query(
        "INSERT INTO robot_scheduler_config (id, enabled, interval_seconds)
         VALUES ('default', $1, $2)",
    )
    .bind(store.config.enabled)
    .bind(
        i64::try_from(store.config.interval_seconds)
            .map_err(|_| ApiError::Internal("机器人调度周期过大".to_string()))?,
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("机器人调度配置数据保存失败".to_string()))?;

    for run in &store.runs {
        sqlx::query(
            "INSERT INTO robot_scheduler_runs
             (id, trigger, status, started_at, finished_at, now, error,
              created_plan_count, filled_plan_count, created_order_count,
              ledger_entry_count, skipped_item_count, skipped_items)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(&run.id)
        .bind(enum_to_string(&run.trigger)?)
        .bind(enum_to_string(&run.status)?)
        .bind(&run.started_at)
        .bind(&run.finished_at)
        .bind(&run.now)
        .bind(&run.error)
        .bind(to_i32_count(run.created_plan_count, "创建合买数量")?)
        .bind(to_i32_count(run.filled_plan_count, "补满合买数量")?)
        .bind(to_i32_count(run.created_order_count, "生成订单数量")?)
        .bind(to_i32_count(run.ledger_entry_count, "资金流水数量")?)
        .bind(to_i32_count(run.skipped_item_count, "跳过明细数量")?)
        .bind(to_json(&run.skipped_items)?)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("机器人调度历史数据保存失败".to_string()))?;
    }

    sqlx::query("INSERT INTO robot_scheduler_runtime (key, value) VALUES ('next_sequence', $1)")
        .bind(
            i64::try_from(store.next_sequence)
                .map_err(|_| ApiError::Internal("机器人调度序号过大".to_string()))?,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("机器人调度运行数据保存失败".to_string()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("机器人调度事务提交失败".to_string()))
}

/// 读取并转换机器人调度历史中的数量字段。
fn read_usize_count(row: &sqlx::postgres::PgRow, column: &str) -> ApiResult<usize> {
    let value: i32 = row
        .try_get(column)
        .map_err(|_| ApiError::Internal("机器人调度数量数据读取失败".to_string()))?;
    usize::try_from(value).map_err(|_| ApiError::Internal("机器人调度数量数据无效".to_string()))
}

/// 将 usize 转成 i32，避免过大数量写入数据库溢出。
fn to_i32_count(value: usize, label: &str) -> ApiResult<i32> {
    i32::try_from(value).map_err(|_| ApiError::Internal(format!("{label}过大")))
}

/// 从运行记录 ID 列表中恢复最大序号。
fn max_sequence(ids: &[String]) -> u64 {
    ids.iter()
        .filter_map(|id| id.strip_prefix("RSC"))
        .filter_map(|value| value.parse::<u64>().ok())
        .max()
        .unwrap_or_default()
}

impl RobotSchedulerStore {
    /// 初始化机器人调度运行时快照。
    fn new(config: RobotSchedulerConfig) -> Self {
        Self {
            config,
            next_sequence: 0,
            runs: VecDeque::new(),
        }
    }

    /// 返回后台可展示的机器人调度器状态。
    fn status(&self) -> RobotSchedulerStatus {
        let recent_runs = self.runs.iter().cloned().collect::<Vec<_>>();
        RobotSchedulerStatus {
            enabled: self.config.enabled,
            config: self.config.clone(),
            run_count: self.runs.len(),
            last_run: recent_runs.first().cloned(),
            recent_runs,
        }
    }

    /// 更新配置并返回状态。
    fn update_config(&mut self, config: RobotSchedulerConfig) -> ApiResult<RobotSchedulerStatus> {
        config.validate()?;
        self.config = config;
        Ok(self.status())
    }

    /// 写入成功执行记录。
    fn record_success(
        &mut self,
        trigger: RobotSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        run: &GroupBuyRobotRun,
    ) -> ApiResult<RobotSchedulerRunRecord> {
        let record = self.next_record(RobotSchedulerRunRecord {
            id: String::new(),
            trigger,
            status: RobotSchedulerRunStatus::Success,
            started_at,
            finished_at,
            now: run.now.clone(),
            error: None,
            created_plan_count: run.created_plans.len(),
            filled_plan_count: run.filled_plans.len(),
            created_order_count: run.created_orders.len(),
            ledger_entry_count: run.ledger_entries.len(),
            skipped_item_count: run.skipped_items.len(),
            skipped_items: run.skipped_items.clone(),
        });
        self.push_record(record)
    }

    /// 写入失败执行记录。
    fn record_failure(
        &mut self,
        trigger: RobotSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        now: String,
        error: String,
    ) -> ApiResult<RobotSchedulerRunRecord> {
        let record = self.next_record(RobotSchedulerRunRecord {
            id: String::new(),
            trigger,
            status: RobotSchedulerRunStatus::Failed,
            started_at,
            finished_at,
            now,
            error: Some(error),
            created_plan_count: 0,
            filled_plan_count: 0,
            created_order_count: 0,
            ledger_entry_count: 0,
            skipped_item_count: 0,
            skipped_items: Vec::new(),
        });
        self.push_record(record)
    }

    /// 为运行记录分配递增 ID。
    fn next_record(&mut self, mut record: RobotSchedulerRunRecord) -> RobotSchedulerRunRecord {
        self.next_sequence += 1;
        record.id = format!("RSC{:012}", self.next_sequence);
        record
    }

    /// 保存运行记录并裁剪最近历史。
    fn push_record(
        &mut self,
        record: RobotSchedulerRunRecord,
    ) -> ApiResult<RobotSchedulerRunRecord> {
        self.runs.push_front(record.clone());
        while self.runs.len() > MAX_ROBOT_SCHEDULER_HISTORY {
            self.runs.pop_back();
        }
        Ok(record)
    }
}

impl RobotSchedulerConfig {
    /// 校验机器人调度配置，避免过短或无效周期导致空转。
    fn validate(&self) -> ApiResult<()> {
        if self.interval_seconds == 0 {
            return Err(ApiError::BadRequest(
                "机器人调度周期必须大于 0 秒".to_string(),
            ));
        }
        Ok(())
    }
}

/// 创建并启动独立机器人调度后台任务。
pub fn spawn_robot_scheduler(
    access: AccessRepository,
    draws: DrawRepository,
    lotteries: LotteryRepository,
    orders: OrderRepository,
    finance: FinanceRepository,
    group_buys: GroupBuyRepository,
    robots: RobotRepository,
    realtime: RealtimeHub,
    config: RobotSchedulerConfig,
    scheduler: RobotSchedulerRepository,
) -> JoinHandle<()> {
    tracing::info!(
        enabled = config.enabled,
        interval_seconds = config.interval_seconds,
        "机器人调度器后台任务已启动"
    );

    tokio::spawn(async move {
        let mut next_run_at = tokio::time::Instant::now();
        loop {
            let wait_duration = next_run_at.saturating_duration_since(tokio::time::Instant::now());
            if !wait_duration.is_zero() {
                tokio::time::sleep(wait_duration).await;
            }

            let started_at = current_robot_scheduler_timestamp();
            let now = started_at.clone();
            let current_config = match scheduler.config() {
                Ok(current_config) => current_config,
                Err(error) => {
                    tracing::error!(error = %error.log_message(), "机器人调度器配置读取失败");
                    config.clone()
                }
            };

            if !current_config.enabled {
                // tracing::debug!("机器人调度器因配置禁用跳过本轮执行");
                tokio::time::sleep(Duration::from_secs(DISABLED_ROBOT_SCHEDULER_POLL_SECONDS))
                    .await;
                next_run_at = tokio::time::Instant::now();
                continue;
            }

            let interval = Duration::from_secs(current_config.interval_seconds.max(1));
            next_run_at += interval;
            let run_started = Instant::now();

            match run_robot_scheduler_once_with_realtime(
                &robots,
                &draws,
                &lotteries,
                &orders,
                &finance,
                &group_buys,
                &access,
                now.clone(),
                Some(&realtime),
            )
            .await
            {
                Ok(run) => {
                    let finished_at = current_robot_scheduler_timestamp();
                    if let Err(error) = scheduler
                        .record_success(
                            RobotSchedulerRunTrigger::Automatic,
                            started_at,
                            finished_at,
                            &run,
                        )
                        .await
                    {
                        tracing::error!(error = %error.log_message(), "机器人调度器历史记录写入失败");
                    }
                    tracing::info!(
                        "当前时间" = %run.now,
                        "本轮耗时毫秒" = run_started.elapsed().as_millis(),
                        "机器人新增合买" = run.created_plans.len(),
                        "机器人满单" = run.filled_plans.len(),
                        "机器人生成订单" = run.created_orders.len(),
                        "机器人跳过明细" = run.skipped_items.len(),
                        "机器人调度器本轮执行完成"
                    );
                }
                Err(error) => {
                    let finished_at = current_robot_scheduler_timestamp();
                    if let Err(record_error) = scheduler
                        .record_failure(
                            RobotSchedulerRunTrigger::Automatic,
                            started_at,
                            finished_at,
                            now.clone(),
                            error.to_string(),
                        )
                        .await
                    {
                        tracing::error!(
                            error = %record_error.log_message(),
                            "机器人调度器历史记录写入失败"
                        );
                    }
                    tracing::error!(
                        "当前时间" = %now,
                        "本轮耗时毫秒" = run_started.elapsed().as_millis(),
                        error = %error.log_message(),
                        "机器人调度器本轮执行失败"
                    );
                }
            }

            if next_run_at <= tokio::time::Instant::now() {
                tracing::warn!(
                    interval_seconds = current_config.interval_seconds,
                    "机器人调度器本轮耗时超过调度周期，下一轮将立即追赶执行"
                );
                next_run_at = tokio::time::Instant::now();
            }
        }
    })
}

/// 执行一轮独立机器人调度，供测试或后续手动入口复用。
#[allow(dead_code)]
pub async fn run_robot_scheduler_once(
    robots: &RobotRepository,
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    access: &AccessRepository,
    now: String,
) -> ApiResult<GroupBuyRobotRun> {
    run_robot_scheduler_once_with_realtime(
        robots, draws, lotteries, orders, finance, group_buys, access, now, None,
    )
    .await
}

/// 执行一轮机器人任务并按执行结果推送余额和订单实时事件。
async fn run_robot_scheduler_once_with_realtime(
    robots: &RobotRepository,
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    access: &AccessRepository,
    now: String,
    realtime: Option<&RealtimeHub>,
) -> ApiResult<GroupBuyRobotRun> {
    let now = now.trim().to_string();
    if now.is_empty() {
        return Err(ApiError::BadRequest("机器人调度时间不能为空".to_string()));
    }

    let run = run_group_buy_robots(
        robots, draws, lotteries, orders, finance, group_buys, access, now,
    )
    .await?;
    if let Some(realtime) = realtime {
        publish_robot_realtime_events(realtime, finance, &run).await;
    }
    Ok(run)
}

/// 推送机器人产生的余额变化和投注订单事件。
pub(crate) async fn publish_robot_realtime_events(
    realtime: &RealtimeHub,
    finance: &FinanceRepository,
    run: &GroupBuyRobotRun,
) {
    for entry in &run.ledger_entries {
        match finance.account_or_create(&entry.user_id).await {
            Ok(account) => realtime.publish_user(
                &entry.user_id,
                balance_changed_event(&account, "group_buy_robot", entry.reference_id.as_deref()),
            ),
            Err(error) => tracing::warn!(
                user_id = %entry.user_id,
                error = %error.log_message(),
                "机器人调度器推送用户余额变化时读取资金账户失败"
            ),
        }
    }
    for order in &run.created_orders {
        realtime.publish_user(&order.user_id, order_changed_event(order, "created"));
    }
}

/// 返回当前机器人调度时间字符串。
fn current_robot_scheduler_timestamp() -> String {
    Local::now()
        .naive_local()
        .format(TIMESTAMP_FORMAT)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{RobotSchedulerConfig, RobotSchedulerRepository, RobotSchedulerRunTrigger};
    use crate::domain::robot::GroupBuyRobotRun;

    #[test]
    /// 验证机器人调度周期必须大于 0 秒。
    fn robot_scheduler_config_rejects_zero_interval() {
        let error = RobotSchedulerConfig {
            enabled: true,
            interval_seconds: 0,
        }
        .validate()
        .expect_err("zero interval should be rejected");

        assert!(error.to_string().contains("机器人调度周期必须大于 0 秒"));
    }

    #[tokio::test]
    /// 验证内存模式机器人调度仓储可以更新配置。
    async fn robot_scheduler_repository_updates_config() {
        let repository = RobotSchedulerRepository::new(RobotSchedulerConfig::default());

        let status = repository
            .update_config(RobotSchedulerConfig {
                enabled: true,
                interval_seconds: 3,
            })
            .await
            .expect("config can be updated");

        assert!(status.enabled);
        assert_eq!(status.config.interval_seconds, 3);
    }

    #[tokio::test]
    /// 验证机器人调度成功记录会统计本轮执行结果。
    async fn robot_scheduler_repository_records_success_summary() {
        let repository = RobotSchedulerRepository::new(RobotSchedulerConfig::default());
        let run = GroupBuyRobotRun {
            now: "2026-06-23 08:00:00".to_string(),
            created_plans: Vec::new(),
            filled_plans: Vec::new(),
            created_orders: Vec::new(),
            ledger_entries: Vec::new(),
            skipped_items: Vec::new(),
            protected_plan_ids: Vec::new(),
            protected_issue_keys: Vec::new(),
        };

        let record = repository
            .record_success(
                RobotSchedulerRunTrigger::Automatic,
                "2026-06-23 08:00:00".to_string(),
                "2026-06-23 08:00:01".to_string(),
                &run,
            )
            .await
            .expect("success record can be stored");

        assert_eq!(record.id, "RSC000000000001");
        assert_eq!(record.now, "2026-06-23 08:00:00");
        assert_eq!(repository.status().expect("status exists").run_count, 1);
    }
}
