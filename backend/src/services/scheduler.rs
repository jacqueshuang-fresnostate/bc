//! 开奖调度服务，处理常驻调度配置、启动/关闭和历史记录

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tokio::{sync::Mutex as AsyncMutex, task::JoinHandle};

use crate::{
    domain::{
        draw::{
            CreateDrawIssueRequest, DrawAutomationRun, DrawAutomationRunRequest,
            DrawAutomationSkippedIssue, DrawIssue, DrawIssueGenerationPreview, DrawIssueStatus,
            GenerateDrawIssuesRequest,
        },
        lottery::{DrawMode, LotteryKind, DEFAULT_SALE_CLOSE_LEAD_SECONDS},
        robot::GroupBuyRobotRun,
    },
    error::{ApiError, ApiResult},
    services::{
        access::AccessRepository,
        automation::{
            close_due_draw_issues, draw_due_issues, merge_draw_automation_runs,
            refund_closed_unfilled_group_buys,
        },
        business_database::{
            enum_from_string, enum_to_string, from_json, to_json, BusinessDatabase,
        },
        draw::DrawRepository,
        draw_generation::{generate_draw_issue_batch, preview_draw_issue_generation},
        finance::FinanceRepository,
        group_buy::GroupBuyRepository,
        group_buy_robot::{force_fill_user_group_buy_plans_before_refund, run_group_buy_robots},
        lottery::LotteryRepository,
        order::OrderRepository,
        realtime::{
            balance_changed_event, draw_result_event, issue_closed_event, issue_opened_event,
            order_changed_event, RealtimeHub,
        },
        robot::RobotRepository,
    },
};

const DEFAULT_SCHEDULER_INTERVAL_SECONDS: u64 = 60;
const DEFAULT_FUTURE_ISSUE_COUNT: u32 = 1;
const DISABLED_SCHEDULER_POLL_SECONDS: u64 = 1;
const MAX_SCHEDULER_HISTORY: usize = 20;
const MAX_FUTURE_ISSUE_COUNT: u32 = 50;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

struct PendingApiIssueGeneration {
    lottery: LotteryKind,
    count: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 开奖调度器配置，控制启停、执行间隔和未来期号缓冲。
pub struct DrawSchedulerConfig {
    /// 功能开关。
    pub enabled: bool,
    /// intervalseconds字段。
    pub interval_seconds: u64,
    /// 未来期号count字段。
    pub future_issue_count: u32,
    /// 开奖前封盘提前秒数。
    pub sale_close_lead_seconds: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 调度器跳过彩种的原因记录。
pub struct DrawSchedulerSkippedLottery {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 申请、审核或跳过原因。
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 调度器单轮执行结果，汇总自动开奖、结算和补期信息。
pub struct DrawSchedulerRun {
    /// 当前业务时间字符串。
    pub now: String,
    /// automationrun字段。
    pub automation_run: DrawAutomationRun,
    /// 本次生成的新期号列表。
    pub generated_issues: Vec<DrawIssue>,
    /// 机器人run字段。
    pub robot_run: GroupBuyRobotRun,
    /// skipped彩种字段。
    pub skipped_lotteries: Vec<DrawSchedulerSkippedLottery>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 调度器运行记录状态。
pub enum DrawSchedulerRunStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 调度器触发来源，区分后台手动和后台工作线程。
pub enum DrawSchedulerRunTrigger {
    Automatic,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 调度器历史执行记录。
pub struct DrawSchedulerRunRecord {
    /// 业务唯一标识。
    pub id: String,
    /// trigger字段。
    pub trigger: DrawSchedulerRunTrigger,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: DrawSchedulerRunStatus,
    /// startedat字段。
    pub started_at: String,
    /// finishedat字段。
    pub finished_at: String,
    /// 当前业务时间字符串。
    pub now: String,
    /// 错误字段。
    pub error: Option<String>,
    /// 封盘期号count字段。
    pub closed_issue_count: usize,
    /// 已开奖期号count字段。
    pub drawn_issue_count: usize,
    /// 结算runcount字段。
    pub settlement_run_count: usize,
    /// 流水记录count字段。
    pub ledger_entry_count: usize,
    /// generated期号count字段。
    pub generated_issue_count: usize,
    /// skipped期号count字段。
    pub skipped_issue_count: usize,
    /// skipped彩种count字段。
    pub skipped_lottery_count: usize,
    /// 本轮跳过处理的期号及原因。
    pub skipped_issues: Vec<DrawAutomationSkippedIssue>,
    /// skipped彩种字段。
    pub skipped_lotteries: Vec<DrawSchedulerSkippedLottery>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台展示的调度器当前状态。
pub struct DrawSchedulerStatus {
    /// 功能开关。
    pub enabled: bool,
    /// 配置字段。
    pub config: DrawSchedulerConfig,
    /// runcount字段。
    pub run_count: usize,
    /// lastrun字段。
    pub last_run: Option<DrawSchedulerRunRecord>,
    /// 最近runs字段。
    pub recent_runs: Vec<DrawSchedulerRunRecord>,
}

#[derive(Clone)]
/// 开奖调度器仓储，保存配置、历史和运行时序号。
pub struct DrawSchedulerRepository {
    inner: Arc<RwLock<DrawSchedulerStore>>,
    persistence: Option<BusinessDatabase>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// 开奖调度器运行时快照。
struct DrawSchedulerStore {
    config: DrawSchedulerConfig,
    next_sequence: u64,
    runs: VecDeque<DrawSchedulerRunRecord>,
}

/// 调度器默认配置，初始关闭并保留最小未来期号缓冲。
impl Default for DrawSchedulerConfig {
    /// 返回默认配置。
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: DEFAULT_SCHEDULER_INTERVAL_SECONDS,
            future_issue_count: DEFAULT_FUTURE_ISSUE_COUNT,
            sale_close_lead_seconds: DEFAULT_SALE_CLOSE_LEAD_SECONDS,
        }
    }
}

/// 开奖调度器仓储的方法实现。
impl DrawSchedulerRepository {
    /// 初始化调度器状态。
    pub fn new(config: DrawSchedulerConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DrawSchedulerStore::new(config))),
            persistence: None,
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(
        config: DrawSchedulerConfig,
        persistence: BusinessDatabase,
    ) -> ApiResult<Self> {
        let store = load_scheduler_store(&persistence, config).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 读取当前调度器运行状态。
    pub fn status(&self) -> ApiResult<DrawSchedulerStatus> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw scheduler store lock poisoned".to_string()))
            .map(|store| store.status())
    }

    /// 读取调度器当前配置。
    pub fn config(&self) -> ApiResult<DrawSchedulerConfig> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw scheduler store lock poisoned".to_string()))
            .map(|store| store.config.clone())
    }

    /// 更新并持久化调度器配置。
    pub async fn update_config(
        &self,
        config: DrawSchedulerConfig,
    ) -> ApiResult<DrawSchedulerStatus> {
        let (result, snapshot) = {
            let mut store = self.inner.write().map_err(|_| {
                ApiError::Internal("draw scheduler store lock poisoned".to_string())
            })?;
            let result = store.update_config(config)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 记录一次调度成功执行并更新统计。
    pub async fn record_success(
        &self,
        trigger: DrawSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        run: &DrawSchedulerRun,
    ) -> ApiResult<DrawSchedulerRunRecord> {
        let (result, snapshot) = {
            let mut store = self.inner.write().map_err(|_| {
                ApiError::Internal("draw scheduler store lock poisoned".to_string())
            })?;
            let result = store.record_success(trigger, started_at, finished_at, run)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 记录一次调度失败并更新统计信息。
    pub async fn record_failure(
        &self,
        trigger: DrawSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        now: String,
        error: String,
    ) -> ApiResult<DrawSchedulerRunRecord> {
        let (result, snapshot) = {
            let mut store = self.inner.write().map_err(|_| {
                ApiError::Internal("draw scheduler store lock poisoned".to_string())
            })?;
            let result = store.record_failure(trigger, started_at, finished_at, now, error)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 从数据库重新加载开奖调度配置和最近运行历史，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let current_config = self.config()?;
        let store = load_scheduler_store(persistence, current_config).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("开奖调度缓存刷新失败".to_string()))? = store;
        Ok(true)
    }
    /// 把当前仓储快照同步保存到持久化存储。
    async fn persist(&self, store: &DrawSchedulerStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_scheduler_store(persistence, store).await?;
        }

        Ok(())
    }
}

/// 从数据库加载开奖调度器配置和历史记录。
async fn load_scheduler_store(
    database: &BusinessDatabase,
    default_config: DrawSchedulerConfig,
) -> ApiResult<DrawSchedulerStore> {
    let pool = database.pool();
    let config = sqlx::query(
        "SELECT enabled, interval_seconds, future_issue_count, sale_close_lead_seconds
         FROM draw_scheduler_config
         WHERE id = 'default'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("开奖调度配置数据读取失败".to_string()))?
    .map(|row| {
        let interval_seconds: i64 = row
            .try_get("interval_seconds")
            .map_err(|_| ApiError::Internal("开奖调度配置数据读取失败".to_string()))?;
        let future_issue_count: i32 = row
            .try_get("future_issue_count")
            .map_err(|_| ApiError::Internal("开奖调度配置数据读取失败".to_string()))?;
        let sale_close_lead_seconds: i32 = row
            .try_get("sale_close_lead_seconds")
            .map_err(|_| ApiError::Internal("开奖调度配置数据读取失败".to_string()))?;
        Ok(DrawSchedulerConfig {
            enabled: row
                .try_get("enabled")
                .map_err(|_| ApiError::Internal("开奖调度配置数据读取失败".to_string()))?,
            interval_seconds: u64::try_from(interval_seconds)
                .map_err(|_| ApiError::Internal("开奖调度周期数据无效".to_string()))?,
            future_issue_count: u32::try_from(future_issue_count)
                .map_err(|_| ApiError::Internal("开奖调度预生成数量数据无效".to_string()))?,
            sale_close_lead_seconds: u32::try_from(sale_close_lead_seconds)
                .map_err(|_| ApiError::Internal("开奖调度封盘时间数据无效".to_string()))?,
        })
    })
    .transpose()?;

    let Some(config) = config else {
        let seeded = DrawSchedulerStore::new(default_config);
        save_scheduler_store(database, &seeded).await?;
        return Ok(seeded);
    };
    config.validate()?;

    let mut runs = VecDeque::new();
    let mut run_ids = Vec::new();
    for row in sqlx::query(
        "SELECT id, trigger, status, started_at, finished_at, now, error, closed_issue_count,
                drawn_issue_count, settlement_run_count, ledger_entry_count,
                generated_issue_count, skipped_issue_count, skipped_lottery_count,
                skipped_issues, skipped_lotteries
         FROM draw_scheduler_runs
         ORDER BY id DESC
         LIMIT 20",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("开奖调度历史数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("开奖调度历史数据读取失败".to_string()))?;
        run_ids.push(id.clone());
        runs.push_back(DrawSchedulerRunRecord {
            id,
            trigger: enum_from_string(
                row.try_get("trigger")
                    .map_err(|_| ApiError::Internal("开奖调度历史数据读取失败".to_string()))?,
            )?,
            status: enum_from_string(
                row.try_get("status")
                    .map_err(|_| ApiError::Internal("开奖调度历史数据读取失败".to_string()))?,
            )?,
            started_at: row
                .try_get("started_at")
                .map_err(|_| ApiError::Internal("开奖调度历史数据读取失败".to_string()))?,
            finished_at: row
                .try_get("finished_at")
                .map_err(|_| ApiError::Internal("开奖调度历史数据读取失败".to_string()))?,
            now: row
                .try_get("now")
                .map_err(|_| ApiError::Internal("开奖调度历史数据读取失败".to_string()))?,
            error: row
                .try_get("error")
                .map_err(|_| ApiError::Internal("开奖调度历史数据读取失败".to_string()))?,
            closed_issue_count: read_usize_count(&row, "closed_issue_count")?,
            drawn_issue_count: read_usize_count(&row, "drawn_issue_count")?,
            settlement_run_count: read_usize_count(&row, "settlement_run_count")?,
            ledger_entry_count: read_usize_count(&row, "ledger_entry_count")?,
            generated_issue_count: read_usize_count(&row, "generated_issue_count")?,
            skipped_issue_count: read_usize_count(&row, "skipped_issue_count")?,
            skipped_lottery_count: read_usize_count(&row, "skipped_lottery_count")?,
            skipped_issues: from_json(
                row.try_get("skipped_issues")
                    .map_err(|_| ApiError::Internal("开奖调度跳过期号明细读取失败".to_string()))?,
            )?,
            skipped_lotteries: from_json(
                row.try_get("skipped_lotteries")
                    .map_err(|_| ApiError::Internal("开奖调度跳过彩种明细读取失败".to_string()))?,
            )?,
        });
    }

    let next_sequence = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM draw_scheduler_runtime WHERE key = 'next_sequence'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("开奖调度运行数据读取失败".to_string()))?
    .unwrap_or_default();

    Ok(DrawSchedulerStore {
        config,
        next_sequence: u64::try_from(next_sequence)
            .unwrap_or_default()
            .max(max_sequence(&run_ids)),
        runs,
    })
}

/// 把开奖调度器配置、历史和运行时序号保存到数据库。
async fn save_scheduler_store(
    database: &BusinessDatabase,
    store: &DrawSchedulerStore,
) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("开奖调度事务开启失败".to_string()))?;

    for table in [
        "draw_scheduler_runs",
        "draw_scheduler_runtime",
        "draw_scheduler_config",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("开奖调度数据清理失败".to_string()))?;
    }

    sqlx::query(
        "INSERT INTO draw_scheduler_config
         (id, enabled, interval_seconds, future_issue_count, sale_close_lead_seconds)
         VALUES ('default', $1, $2, $3, $4)",
    )
    .bind(store.config.enabled)
    .bind(
        i64::try_from(store.config.interval_seconds)
            .map_err(|_| ApiError::Internal("开奖调度周期过大".to_string()))?,
    )
    .bind(
        i32::try_from(store.config.future_issue_count)
            .map_err(|_| ApiError::Internal("开奖调度预生成数量过大".to_string()))?,
    )
    .bind(
        i32::try_from(store.config.sale_close_lead_seconds)
            .map_err(|_| ApiError::Internal("开奖调度封盘时间过大".to_string()))?,
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("开奖调度配置数据保存失败".to_string()))?;

    for run in &store.runs {
        sqlx::query(
            "INSERT INTO draw_scheduler_runs
             (id, trigger, status, started_at, finished_at, now, error, closed_issue_count,
              drawn_issue_count, settlement_run_count, ledger_entry_count, generated_issue_count,
              skipped_issue_count, skipped_lottery_count, skipped_issues, skipped_lotteries)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
        )
        .bind(&run.id)
        .bind(enum_to_string(&run.trigger)?)
        .bind(enum_to_string(&run.status)?)
        .bind(&run.started_at)
        .bind(&run.finished_at)
        .bind(&run.now)
        .bind(&run.error)
        .bind(to_i32_count(run.closed_issue_count, "已封盘期号数量")?)
        .bind(to_i32_count(run.drawn_issue_count, "已开奖期号数量")?)
        .bind(to_i32_count(run.settlement_run_count, "结算批次数量")?)
        .bind(to_i32_count(run.ledger_entry_count, "资金流水数量")?)
        .bind(to_i32_count(run.generated_issue_count, "生成期号数量")?)
        .bind(to_i32_count(run.skipped_issue_count, "跳过期号数量")?)
        .bind(to_i32_count(run.skipped_lottery_count, "跳过彩种数量")?)
        .bind(to_json(&run.skipped_issues)?)
        .bind(to_json(&run.skipped_lotteries)?)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖调度历史数据保存失败".to_string()))?;
    }

    sqlx::query("INSERT INTO draw_scheduler_runtime (key, value) VALUES ('next_sequence', $1)")
        .bind(
            i64::try_from(store.next_sequence)
                .map_err(|_| ApiError::Internal("开奖调度序号过大".to_string()))?,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖调度运行数据保存失败".to_string()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("开奖调度事务提交失败".to_string()))
}

/// 读取并转换数据库中计数值。
fn read_usize_count(row: &sqlx::postgres::PgRow, column: &str) -> ApiResult<usize> {
    let value: i32 = row
        .try_get(column)
        .map_err(|_| ApiError::Internal("开奖调度数量数据读取失败".to_string()))?;
    usize::try_from(value).map_err(|_| ApiError::Internal("开奖调度数量数据无效".to_string()))
}

/// 将 usize 转为 i32 并校验范围。
fn to_i32_count(value: usize, label: &str) -> ApiResult<i32> {
    i32::try_from(value).map_err(|_| ApiError::Internal(format!("{label}过大")))
}

/// 计算并返回序列号最大值。
fn max_sequence(ids: &[String]) -> u64 {
    ids.iter()
        .filter_map(|id| id.strip_prefix("SCH"))
        .filter_map(|value| value.parse::<u64>().ok())
        .max()
        .unwrap_or_default()
}

/// 开奖调度器快照的校验、记录和历史裁剪方法。
impl DrawSchedulerStore {
    /// 初始化内部状态容器。
    fn new(config: DrawSchedulerConfig) -> Self {
        Self {
            config,
            next_sequence: 0,
            runs: VecDeque::new(),
        }
    }

    /// 返回当前状态信息。
    fn status(&self) -> DrawSchedulerStatus {
        let recent_runs = self.runs.iter().cloned().collect::<Vec<_>>();
        DrawSchedulerStatus {
            enabled: self.config.enabled,
            config: self.config.clone(),
            run_count: self.runs.len(),
            last_run: recent_runs.first().cloned(),
            recent_runs,
        }
    }

    /// 更新开奖调度器配置并返回最新状态。
    fn update_config(&mut self, config: DrawSchedulerConfig) -> ApiResult<DrawSchedulerStatus> {
        config.validate()?;
        self.config = config;
        Ok(self.status())
    }

    /// 记录一轮调度成功结果和耗时指标。
    fn record_success(
        &mut self,
        trigger: DrawSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        run: &DrawSchedulerRun,
    ) -> ApiResult<DrawSchedulerRunRecord> {
        let record = self.next_record(DrawSchedulerRunRecord {
            id: String::new(),
            trigger,
            status: DrawSchedulerRunStatus::Success,
            started_at,
            finished_at,
            now: run.now.clone(),
            error: None,
            closed_issue_count: run.automation_run.closed_issues.len(),
            drawn_issue_count: run.automation_run.drawn_issues.len(),
            settlement_run_count: run.automation_run.settlement_runs.len(),
            ledger_entry_count: run.automation_run.ledger_entries.len()
                + run.robot_run.ledger_entries.len(),
            generated_issue_count: run.generated_issues.len(),
            skipped_issue_count: run.automation_run.skipped_issues.len(),
            skipped_lottery_count: run.skipped_lotteries.len(),
            skipped_issues: run.automation_run.skipped_issues.clone(),
            skipped_lotteries: run.skipped_lotteries.clone(),
        });
        self.push_record(record)
    }

    /// 记录一轮调度失败原因和耗时指标。
    fn record_failure(
        &mut self,
        trigger: DrawSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        now: String,
        error: String,
    ) -> ApiResult<DrawSchedulerRunRecord> {
        let record = self.next_record(DrawSchedulerRunRecord {
            id: String::new(),
            trigger,
            status: DrawSchedulerRunStatus::Failed,
            started_at,
            finished_at,
            now,
            error: Some(error),
            closed_issue_count: 0,
            drawn_issue_count: 0,
            settlement_run_count: 0,
            ledger_entry_count: 0,
            generated_issue_count: 0,
            skipped_issue_count: 0,
            skipped_lottery_count: 0,
            skipped_issues: Vec::new(),
            skipped_lotteries: Vec::new(),
        });
        self.push_record(record)
    }

    /// 为调度执行记录分配递增编号。
    fn next_record(&mut self, mut record: DrawSchedulerRunRecord) -> DrawSchedulerRunRecord {
        self.next_sequence += 1;
        record.id = format!("SCH{:012}", self.next_sequence);
        record
    }

    /// 保存调度执行记录并裁剪历史长度。
    fn push_record(&mut self, record: DrawSchedulerRunRecord) -> ApiResult<DrawSchedulerRunRecord> {
        self.runs.push_front(record.clone());
        while self.runs.len() > MAX_SCHEDULER_HISTORY {
            self.runs.pop_back();
        }
        Ok(record)
    }
}

/// 开奖调度器配置校验方法。
impl DrawSchedulerConfig {
    /// 校验参数并返回校验结果。
    fn validate(&self) -> ApiResult<()> {
        if self.interval_seconds == 0 {
            return Err(ApiError::BadRequest(
                "draw scheduler interval seconds must be greater than zero".to_string(),
            ));
        }
        if self.future_issue_count == 0 || self.future_issue_count > MAX_FUTURE_ISSUE_COUNT {
            return Err(ApiError::BadRequest(format!(
                "draw scheduler future issue count must be between 1 and {MAX_FUTURE_ISSUE_COUNT}"
            )));
        }
        if self.sale_close_lead_seconds == 0 {
            return Err(ApiError::BadRequest(
                "开奖调度封盘时间必须大于 0 秒".to_string(),
            ));
        }

        Ok(())
    }
}

/// 创建并启动异步开奖调度任务。
pub fn spawn_draw_scheduler(
    access: AccessRepository,
    draws: DrawRepository,
    lotteries: LotteryRepository,
    orders: OrderRepository,
    finance: FinanceRepository,
    group_buys: GroupBuyRepository,
    robots: RobotRepository,
    realtime: RealtimeHub,
    config: DrawSchedulerConfig,
    scheduler: DrawSchedulerRepository,
) -> JoinHandle<()> {
    tracing::info!(
        enabled = config.enabled,
        interval_seconds = config.interval_seconds,
        future_issue_count = config.future_issue_count,
        sale_close_lead_seconds = config.sale_close_lead_seconds,
        "开奖调度器后台任务已启动"
    );

    let slow_phase_lock = Arc::new(AsyncMutex::new(()));

    tokio::spawn(async move {
        let mut next_run_at = tokio::time::Instant::now();
        loop {
            let wait_duration = next_run_at.saturating_duration_since(tokio::time::Instant::now());
            if !wait_duration.is_zero() {
                tokio::time::sleep(wait_duration).await;
            }

            let started_at = current_scheduler_timestamp();
            let now = started_at.clone();
            let current_config = match scheduler.config() {
                Ok(current_config) => current_config,
                Err(error) => {
                    tracing::error!(error = %error.log_message(), "开奖调度器配置读取失败");
                    config.clone()
                }
            };

            if !current_config.enabled {
                tracing::debug!("开奖调度器因配置禁用跳过本轮执行");
                tokio::time::sleep(Duration::from_secs(DISABLED_SCHEDULER_POLL_SECONDS)).await;
                next_run_at = tokio::time::Instant::now();
                continue;
            }

            let interval = Duration::from_secs(current_config.interval_seconds.max(1));
            next_run_at += interval;
            let run_started = Instant::now();

            let pre_close_robot_run = match run_group_buy_robots(
                &robots,
                &draws,
                &lotteries,
                &orders,
                &finance,
                &group_buys,
                &access,
                now.clone(),
            )
            .await
            {
                Ok(run) => run,
                Err(error) => {
                    tracing::error!(
                        now = %now,
                        error = %error.log_message(),
                        "开奖调度器封盘前合买机器人兜底失败"
                    );
                    empty_group_buy_robot_run(now.clone())
                }
            };

            match run_draw_scheduler_opening_once_with_realtime(
                &draws,
                &lotteries,
                &current_config,
                now.clone(),
                Some(&realtime),
            )
            .await
            {
                Ok(mut run) => {
                    run.robot_run = pre_close_robot_run;
                    publish_scheduler_completion_events(&realtime, &finance, &run).await;
                    let run_elapsed_ms = run_started.elapsed().as_millis();
                    let finished_at = current_scheduler_timestamp();
                    if let Err(error) = scheduler
                        .record_success(
                            DrawSchedulerRunTrigger::Automatic,
                            started_at,
                            finished_at,
                            &run,
                        )
                        .await
                    {
                        tracing::error!(error = %error.log_message(), "开奖调度器历史记录写入失败");
                    }
                    tracing::info!(
                        "当前时间" = %run.now,
                        "本轮耗时毫秒" = run_elapsed_ms,
                        "封盘期数" = run.automation_run.closed_issues.len(),
                        "新增期号" = run.generated_issues.len(),
                        "封盘前机器人满单" = run.robot_run.filled_plans.len(),
                        "开奖调度器开盘阶段执行完成"
                    );

                    let slow_lock = slow_phase_lock.clone();
                    match slow_lock.try_lock_owned() {
                        Ok(guard) => {
                            let draws = draws.clone();
                            let lotteries = lotteries.clone();
                            let orders = orders.clone();
                            let finance = finance.clone();
                            let group_buys = group_buys.clone();
                            let robots = robots.clone();
                            let access = access.clone();
                            let realtime = realtime.clone();
                            let current_config = current_config.clone();
                            let slow_now = now.clone();
                            tokio::spawn(async move {
                                let _guard = guard;
                                let slow_started = Instant::now();
                                match run_draw_scheduler_slow_once_with_realtime(
                                    &draws,
                                    &lotteries,
                                    &orders,
                                    &finance,
                                    &group_buys,
                                    &robots,
                                    &access,
                                    &current_config,
                                    slow_now.clone(),
                                    Some(&realtime),
                                )
                                .await
                                {
                                    Ok(slow_run) => tracing::info!(
                                        "当前时间" = %slow_run.now,
                                        "慢阶段耗时毫秒" = slow_started.elapsed().as_millis(),
                                        "开奖期数" = slow_run.automation_run.drawn_issues.len(),
                                        "机器人新增合买" = slow_run.robot_run.created_plans.len(),
                                        "机器人满单" = slow_run.robot_run.filled_plans.len(),
                                        "开奖调度器慢阶段后台任务完成"
                                    ),
                                    Err(error) => tracing::error!(
                                        now = %slow_now,
                                        "慢阶段耗时毫秒" = slow_started.elapsed().as_millis(),
                                        error = %error.log_message(),
                                        "开奖调度器慢阶段后台任务失败"
                                    ),
                                }
                            });
                        }
                        Err(_) => {
                            tracing::debug!(
                                "当前时间" = %now,
                                "开奖调度器慢阶段仍在执行，本轮不重复启动慢阶段"
                            );
                        }
                    }
                }
                Err(error) => {
                    let run_elapsed_ms = run_started.elapsed().as_millis();
                    let finished_at = current_scheduler_timestamp();
                    if let Err(record_error) = scheduler
                        .record_failure(
                            DrawSchedulerRunTrigger::Automatic,
                            started_at,
                            finished_at,
                            now.clone(),
                            error.to_string(),
                        )
                        .await
                    {
                        tracing::error!(
                            error = %record_error.log_message(),
                            "开奖调度器历史记录写入失败"
                        );
                    }
                    tracing::error!(
                        %now,
                        "本轮耗时毫秒" = run_elapsed_ms,
                        error = %error.log_message(),
                        "开奖调度器本轮执行失败"
                    );
                }
            }

            if next_run_at <= tokio::time::Instant::now() {
                tracing::warn!(
                    interval_seconds = current_config.interval_seconds,
                    "开奖调度器本轮耗时超过调度周期，下一轮将立即追赶执行"
                );
                next_run_at = tokio::time::Instant::now();
            }
        }
    })
}

/// 将封盘和开盘事件优先广播，避免开奖源或结算耗时拖短下一期倒计时。
fn publish_scheduler_opening_events(
    realtime: &RealtimeHub,
    closed_issues: &[DrawIssue],
    generated_issues: &[DrawIssue],
) {
    for issue in closed_issues {
        realtime.publish_public(issue_closed_event(issue));
    }
    for issue in generated_issues {
        realtime.publish_public(issue_opened_event(issue));
    }
}

/// 将开奖结果、资金变化和机器人订单在慢阶段完成后广播给客户端。
async fn publish_scheduler_completion_events(
    realtime: &RealtimeHub,
    finance: &FinanceRepository,
    run: &DrawSchedulerRun,
) {
    for issue in &run.automation_run.drawn_issues {
        realtime.publish_public(draw_result_event(issue));
    }
    for entry in &run.automation_run.ledger_entries {
        match finance.account_or_create(&entry.user_id).await {
            Ok(account) => realtime.publish_user(
                &entry.user_id,
                balance_changed_event(&account, "settlement", entry.reference_id.as_deref()),
            ),
            Err(error) => tracing::warn!(
                user_id = %entry.user_id,
                error = %error.log_message(),
                "开奖调度器推送用户余额变化时读取资金账户失败"
            ),
        }
    }
    for entry in &run.robot_run.ledger_entries {
        match finance.account_or_create(&entry.user_id).await {
            Ok(account) => realtime.publish_user(
                &entry.user_id,
                balance_changed_event(&account, "group_buy_robot", entry.reference_id.as_deref()),
            ),
            Err(error) => tracing::warn!(
                user_id = %entry.user_id,
                error = %error.log_message(),
                "开奖调度器推送合买机器人余额变化时读取资金账户失败"
            ),
        }
    }
    for order in &run.robot_run.created_orders {
        realtime.publish_user(&order.user_id, order_changed_event(order, "created"));
    }
}

/// 执行一次完整开奖调度流程。
#[allow(dead_code)]
pub async fn run_draw_scheduler_once(
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    robots: &RobotRepository,
    access: &AccessRepository,
    config: &DrawSchedulerConfig,
    now: String,
) -> ApiResult<DrawSchedulerRun> {
    run_draw_scheduler_once_with_realtime(
        draws, lotteries, orders, finance, group_buys, robots, access, config, now, None,
    )
    .await
}
/// 执行一轮完整开奖调度并在关键节点推送实时事件。
async fn run_draw_scheduler_once_with_realtime(
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    robots: &RobotRepository,
    access: &AccessRepository,
    config: &DrawSchedulerConfig,
    now: String,
    realtime: Option<&RealtimeHub>,
) -> ApiResult<DrawSchedulerRun> {
    let pre_close_robot_run = run_group_buy_robots(
        robots,
        draws,
        lotteries,
        orders,
        finance,
        group_buys,
        access,
        now.clone(),
    )
    .await?;
    let mut opening_run = run_draw_scheduler_opening_once_with_realtime(
        draws,
        lotteries,
        config,
        now.clone(),
        realtime,
    )
    .await?;
    opening_run.robot_run = pre_close_robot_run;
    if let Some(realtime) = realtime {
        publish_scheduler_completion_events(realtime, finance, &opening_run).await;
    }
    let slow_run = run_draw_scheduler_slow_once_with_realtime(
        draws, lotteries, orders, finance, group_buys, robots, access, config, now, realtime,
    )
    .await?;

    Ok(DrawSchedulerRun {
        now: opening_run.now,
        automation_run: merge_draw_automation_runs(
            opening_run.automation_run,
            slow_run.automation_run,
        ),
        generated_issues: opening_run.generated_issues,
        robot_run: merge_group_buy_robot_runs(opening_run.robot_run, slow_run.robot_run),
        skipped_lotteries: opening_run.skipped_lotteries,
    })
}
/// 执行调度快阶段，负责开盘、补未来期号和推送开盘事件。
async fn run_draw_scheduler_opening_once_with_realtime(
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    config: &DrawSchedulerConfig,
    now: String,
    realtime: Option<&RealtimeHub>,
) -> ApiResult<DrawSchedulerRun> {
    config.validate()?;
    let now = now.trim().to_string();
    if now.is_empty() {
        return Err(ApiError::BadRequest(
            "draw scheduler time is required".to_string(),
        ));
    }

    let close_phase_started = Instant::now();
    let close_run = close_due_draw_issues(
        draws,
        lotteries,
        DrawAutomationRunRequest { now: now.clone() },
    )
    .await?;
    let close_phase_ms = close_phase_started.elapsed().as_millis();

    let generation_phase_started = Instant::now();
    let (mut generated_issues, mut skipped_lotteries, pending_api_generations) =
        ensure_non_api_future_draw_issues(draws, lotteries, config, &now).await?;
    let local_generation_phase_ms = generation_phase_started.elapsed().as_millis();

    if let Some(realtime) = realtime {
        publish_scheduler_opening_events(realtime, &close_run.closed_issues, &generated_issues);
    }

    tracing::info!(
        "当前时间" = %now,
        "封盘耗时毫秒" = close_phase_ms,
        "本地补期耗时毫秒" = local_generation_phase_ms,
        "封盘期数" = close_run.closed_issues.len(),
        "新增期号" = generated_issues.len(),
        "开奖调度器快阶段完成，已释放下一期开盘"
    );

    let api_generation_phase_started = Instant::now();
    let (mut api_generated_issues, mut api_skipped_lotteries) =
        ensure_api_future_draw_issues(draws, config, &now, pending_api_generations).await?;
    let api_generation_phase_ms = api_generation_phase_started.elapsed().as_millis();
    if let Some(realtime) = realtime {
        publish_scheduler_opening_events(realtime, &[], &api_generated_issues);
    }
    tracing::info!(
        "当前时间" = %now,
        "API补期耗时毫秒" = api_generation_phase_ms,
        "API新增期号" = api_generated_issues.len(),
        "API跳过彩种" = api_skipped_lotteries.len(),
        "开奖调度器 API 补期阶段完成"
    );
    generated_issues.append(&mut api_generated_issues);
    skipped_lotteries.append(&mut api_skipped_lotteries);

    Ok(DrawSchedulerRun {
        now: now.clone(),
        automation_run: close_run,
        generated_issues,
        robot_run: empty_group_buy_robot_run(now),
        skipped_lotteries,
    })
}
/// 执行调度慢阶段，负责开奖、结算、机器人和派奖。
async fn run_draw_scheduler_slow_once_with_realtime(
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    group_buys: &GroupBuyRepository,
    robots: &RobotRepository,
    access: &AccessRepository,
    config: &DrawSchedulerConfig,
    now: String,
    realtime: Option<&RealtimeHub>,
) -> ApiResult<DrawSchedulerRun> {
    config.validate()?;
    let now = now.trim().to_string();
    if now.is_empty() {
        return Err(ApiError::BadRequest(
            "draw scheduler time is required".to_string(),
        ));
    }

    let robot_phase_started = Instant::now();
    let robot_run = run_group_buy_robots(
        robots,
        draws,
        lotteries,
        orders,
        finance,
        group_buys,
        access,
        now.clone(),
    )
    .await?;
    let robot_phase_ms = robot_phase_started.elapsed().as_millis();

    let guard_phase_started = Instant::now();
    let guard_run = force_fill_user_group_buy_plans_before_refund(
        robots,
        draws,
        lotteries,
        orders,
        finance,
        group_buys,
        access,
        now.clone(),
    )
    .await?;
    let guard_phase_ms = guard_phase_started.elapsed().as_millis();
    let robot_run = merge_group_buy_robot_runs(robot_run, guard_run);

    let refund_phase_started = Instant::now();
    let refund_run = refund_closed_unfilled_group_buys(
        draws,
        lotteries,
        finance,
        group_buys,
        DrawAutomationRunRequest { now: now.clone() },
    )
    .await?;
    let refund_phase_ms = refund_phase_started.elapsed().as_millis();

    let draw_phase_started = Instant::now();
    let draw_run = draw_due_issues(
        draws,
        lotteries,
        orders,
        finance,
        group_buys,
        DrawAutomationRunRequest { now: now.clone() },
    )
    .await?;
    let draw_phase_ms = draw_phase_started.elapsed().as_millis();

    let automation_run = merge_draw_automation_runs(refund_run, draw_run);
    let run = DrawSchedulerRun {
        now,
        automation_run,
        generated_issues: Vec::new(),
        robot_run,
        skipped_lotteries: Vec::new(),
    };

    if let Some(realtime) = realtime {
        publish_scheduler_completion_events(realtime, finance, &run).await;
    }

    tracing::info!(
        "当前时间" = %run.now,
        "封盘流单退款耗时毫秒" = refund_phase_ms,
        "开奖结算耗时毫秒" = draw_phase_ms,
        "机器人耗时毫秒" = robot_phase_ms,
        "流单前兜底耗时毫秒" = guard_phase_ms,
        "开奖期数" = run.automation_run.drawn_issues.len(),
        "结算批次" = run.automation_run.settlement_runs.len(),
        "入账笔数" = run.automation_run.ledger_entries.len(),
        "机器人新增合买" = run.robot_run.created_plans.len(),
        "机器人满单" = run.robot_run.filled_plans.len(),
        "开奖调度器慢阶段完成"
    );

    Ok(run)
}
/// 构造空的合买合买机器人run。
fn empty_group_buy_robot_run(now: String) -> GroupBuyRobotRun {
    GroupBuyRobotRun {
        now,
        created_plans: Vec::new(),
        filled_plans: Vec::new(),
        created_orders: Vec::new(),
        ledger_entries: Vec::new(),
        skipped_items: Vec::new(),
    }
}

/// 合并封盘前兜底和慢阶段机器人结果，方便后台看到完整机器人执行明细。
fn merge_group_buy_robot_runs(
    mut first: GroupBuyRobotRun,
    second: GroupBuyRobotRun,
) -> GroupBuyRobotRun {
    first.created_plans.extend(second.created_plans);
    first.filled_plans.extend(second.filled_plans);
    first.created_orders.extend(second.created_orders);
    first.ledger_entries.extend(second.ledger_entries);
    first.skipped_items.extend(second.skipped_items);
    first
}
/// 确保平台或人工彩种存在足够的未来期号。
async fn ensure_non_api_future_draw_issues(
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    config: &DrawSchedulerConfig,
    now: &str,
) -> ApiResult<(
    Vec<DrawIssue>,
    Vec<DrawSchedulerSkippedLottery>,
    Vec<PendingApiIssueGeneration>,
)> {
    let existing_issues = draws.list_scheduler_active().await?;
    let mut generated_issues = Vec::new();
    let mut skipped_lotteries = Vec::new();
    let mut pending_api_generations = Vec::new();

    for lottery in lotteries.list().await? {
        if !lottery.sale_enabled {
            skipped_lotteries.push(DrawSchedulerSkippedLottery {
                lottery_id: lottery.id,
                reason: "彩种销售已关闭".to_string(),
            });
            continue;
        }

        let existing_future_count = future_issue_count(&existing_issues, &lottery, now);
        if existing_future_count >= config.future_issue_count {
            continue;
        }

        let count = config.future_issue_count - existing_future_count;
        if lottery.draw_mode == DrawMode::Api {
            pending_api_generations.push(PendingApiIssueGeneration { lottery, count });
            continue;
        }

        let result = generate_draw_issue_batch(
            draws,
            &lottery,
            GenerateDrawIssuesRequest {
                lottery_id: lottery.id.clone(),
                now: now.to_string(),
                count,
                sale_close_lead_seconds: Some(lottery.sale_close_lead_seconds),
            },
        )
        .await;

        match result {
            Ok(mut created) => generated_issues.append(&mut created),
            Err(error) => {
                tracing::warn!(
                    lottery_id = %lottery.id,
                    error = %error.log_message(),
                    "开奖调度器因期号生成失败跳过彩种"
                );
                skipped_lotteries.push(DrawSchedulerSkippedLottery {
                    lottery_id: lottery.id,
                    reason: error.to_string(),
                });
            }
        }
    }

    Ok((generated_issues, skipped_lotteries, pending_api_generations))
}
/// 按第三方开奖源锚点确保 API 彩种未来期号。
async fn ensure_api_future_draw_issues(
    draws: &DrawRepository,
    _config: &DrawSchedulerConfig,
    now: &str,
    pending_generations: Vec<PendingApiIssueGeneration>,
) -> ApiResult<(Vec<DrawIssue>, Vec<DrawSchedulerSkippedLottery>)> {
    let mut handles = Vec::new();
    for pending in pending_generations {
        let draws = draws.clone();
        let now = now.to_string();
        handles.push(tokio::spawn(async move {
            let lottery = pending.lottery;
            let result = preview_draw_issue_generation(
                &draws,
                &lottery,
                GenerateDrawIssuesRequest {
                    lottery_id: lottery.id.clone(),
                    now,
                    count: pending.count,
                    sale_close_lead_seconds: Some(lottery.sale_close_lead_seconds),
                },
            )
            .await;
            (lottery, result)
        }));
    }

    let mut planned = Vec::new();
    let mut skipped_lotteries = Vec::new();
    for handle in handles {
        match handle.await {
            Ok((lottery, Ok(plans))) => planned.push((lottery, plans)),
            Ok((lottery, Err(error))) => {
                tracing::warn!(
                    lottery_id = %lottery.id,
                    error = %error.log_message(),
                    "开奖调度器因 API 期号计划生成失败跳过彩种"
                );
                skipped_lotteries.push(DrawSchedulerSkippedLottery {
                    lottery_id: lottery.id,
                    reason: error.to_string(),
                });
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "开奖调度器 API 期号计划并发任务执行失败"
                );
                skipped_lotteries.push(DrawSchedulerSkippedLottery {
                    lottery_id: "unknown".to_string(),
                    reason: "API期号计划并发任务执行失败".to_string(),
                });
            }
        }
    }

    let mut generated_issues = Vec::new();
    for (lottery, plans) in planned {
        for plan in plans {
            match create_draw_issue_from_plan(draws, &lottery, plan).await {
                Ok(issue) => generated_issues.push(issue),
                Err(error) => {
                    tracing::warn!(
                        lottery_id = %lottery.id,
                        error = %error.log_message(),
                        "开奖调度器因 API 期号写入失败跳过彩种"
                    );
                    skipped_lotteries.push(DrawSchedulerSkippedLottery {
                        lottery_id: lottery.id.clone(),
                        reason: error.to_string(),
                    });
                    break;
                }
            }
        }
    }

    Ok((generated_issues, skipped_lotteries))
}
/// 把期号生成计划转换为实际开奖期号。
async fn create_draw_issue_from_plan(
    draws: &DrawRepository,
    lottery: &LotteryKind,
    plan: DrawIssueGenerationPreview,
) -> ApiResult<DrawIssue> {
    draws
        .create(
            lottery,
            CreateDrawIssueRequest {
                lottery_id: lottery.id.clone(),
                issue: plan.issue,
                scheduled_at: plan.scheduled_at,
                sale_closed_at: plan.sale_closed_at,
            },
        )
        .await
}

/// 统计未来待处理期号数量；已封盘但未到开奖时间的当前期也会占用缓冲，避免提前开下一期。
fn future_issue_count(issues: &[DrawIssue], lottery: &LotteryKind, now: &str) -> u32 {
    issues
        .iter()
        .filter(|issue| {
            issue.lottery_id == lottery.id
                && matches!(
                    issue.status,
                    DrawIssueStatus::Open | DrawIssueStatus::Closed
                )
                && issue.scheduled_at.as_str() > now
        })
        .count() as u32
}

/// 返回调度器当前时间戳字符串。
fn current_scheduler_timestamp() -> String {
    Local::now()
        .naive_local()
        .format(TIMESTAMP_FORMAT)
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        domain::{
            draw::{CreateDrawIssueRequest, DrawIssueStatus},
            group_buy::{CreateGroupBuyPlanRequest, GroupBuyPlanStatus},
            lottery::DrawMode,
        },
        services::{
            access::AccessRepository,
            draw::DrawRepository,
            draw_api::ApiDrawSourceRepository,
            finance::FinanceRepository,
            group_buy::GroupBuyRepository,
            lottery::LotteryRepository,
            order::OrderRepository,
            realtime::RealtimeHub,
            robot::RobotRepository,
            scheduler::{
                run_draw_scheduler_once, spawn_draw_scheduler, DrawSchedulerConfig,
                DrawSchedulerRepository, DrawSchedulerRunStatus, DrawSchedulerRunTrigger,
                DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            },
        },
    };
    /// 验证调度器会为销售中彩种生成未来期号。
    #[tokio::test]
    async fn scheduler_generates_future_issues_for_enabled_lotteries() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "ssc60").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let config = enabled_config(2);

        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:00:00".to_string(),
        )
        .await
        .expect("scheduler can run");

        assert!(
            run.generated_issues
                .iter()
                .filter(|issue| issue.lottery_id == "ssc60")
                .count()
                >= 2
        );
        assert!(run
            .skipped_lotteries
            .iter()
            .any(|lottery| lottery.lottery_id == "manual-test"));
    }
    /// 验证 API 期号生成失败时调度器会跳过该彩种。
    #[tokio::test]
    async fn scheduler_skips_lottery_when_api_issue_generation_fails() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(
                r#"{"errorCode":0,"result":{"businessCode":0,"data":[]}}"#,
            ),
        );
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        enable_lottery_sale(&lotteries, "ssc60").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let config = enabled_config(1);

        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:00:00".to_string(),
        )
        .await
        .expect("scheduler can skip failed api generation");

        assert!(
            run.skipped_lotteries
                .iter()
                .any(|lottery| lottery.lottery_id == "fc3d"
                    && lottery.reason.contains("最新期号为空"))
        );
        assert!(run
            .generated_issues
            .iter()
            .any(|issue| issue.lottery_id == "ssc60"));
    }
    /// 验证非 API 彩种期号生成早于 API 期号生成。
    #[tokio::test]
    async fn scheduler_generates_non_api_issues_before_api_generation() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(
                r#"{"errorCode":0,"result":{"businessCode":0,"data":[]}}"#,
            ),
        );
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "fc3d").await;
        enable_lottery_sale(&lotteries, "txffc").await;
        enable_lottery_sale(&lotteries, "ssc60").await;
        let config = enabled_config(1);

        let (generated, skipped, pending_api) = super::ensure_non_api_future_draw_issues(
            &draws,
            &lotteries,
            &config,
            "2026-06-02 20:00:00",
        )
        .await
        .expect("non api generation can run without waiting api sources");

        assert!(generated.iter().any(|issue| issue.lottery_id == "ssc60"));
        assert!(pending_api
            .iter()
            .any(|pending| pending.lottery.id == "fc3d"));
        assert!(pending_api
            .iter()
            .any(|pending| pending.lottery.id == "txffc"));
        assert!(!skipped.iter().any(|lottery| lottery.lottery_id == "ssc60"));
    }
    /// 验证未来期号数量足够时调度器不会重复生成。
    #[tokio::test]
    async fn scheduler_does_not_duplicate_when_future_buffer_is_satisfied() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "ssc60").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let config = enabled_config(1);

        run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:00:00".to_string(),
        )
        .await
        .expect("first scheduler run can generate issues");
        let second = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:00:00".to_string(),
        )
        .await
        .expect("second scheduler run can skip generation");

        assert!(second.generated_issues.is_empty());
    }
    /// 验证到期开奖自动化先于未来期号补齐执行。
    #[tokio::test]
    async fn scheduler_runs_due_automation_before_generating_future_issues() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "ssc60").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let config = enabled_config(1);
        let lottery = lotteries.get("ssc60").await.expect("lottery exists");
        let issue = draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "DUE20260602200000".to_string(),
                    scheduled_at: "2026-06-02 20:00:00".to_string(),
                    sale_closed_at: "2026-06-02 19:59:30".to_string(),
                },
            )
            .await
            .expect("draw issue can be created");

        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:00:00".to_string(),
        )
        .await
        .expect("scheduler can run");
        let stored = draws.get(&issue.id).await.expect("issue exists");

        assert_eq!(stored.status, DrawIssueStatus::Drawn);
        assert_eq!(run.automation_run.drawn_issues.len(), 1);
        assert!(run
            .generated_issues
            .iter()
            .any(|issue| issue.lottery_id == "ssc60"
                && issue.issue == "202606020001"
                && issue.draw_mode == DrawMode::Platform));
    }
    /// 验证封盘前会先尝试补满用户合买计划。
    #[tokio::test]
    async fn scheduler_fills_user_group_buy_before_closing_issue() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "ssc60").await;
        let mut lottery = lotteries.get("ssc60").await.expect("lottery exists");
        lottery.group_buy.enabled = true;
        lotteries
            .update("ssc60", lottery.clone())
            .await
            .expect("lottery group buy can be enabled");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let users = access.snapshot().await.expect("access can load").users;
        let config = enabled_config(1);
        let issue = draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "GROUPBUY20260602200000".to_string(),
                    scheduled_at: "2026-06-02 20:00:00".to_string(),
                    sale_closed_at: "2026-06-02 19:59:30".to_string(),
                },
            )
            .await
            .expect("draw issue can be created");
        let plan = group_buys
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-USER-SCHEDULER-FILL".to_string(),
                    lottery_id: lottery.id.clone(),
                    issue: issue.issue.clone(),
                    rule_code: "fiveFrontDirect".to_string(),
                    title: "用户发起封盘兜底合买".to_string(),
                    numbers: "1|2|3".to_string(),
                    initiator_user_id: "U10001".to_string(),
                    total_amount_minor: 5_000,
                    initiator_amount_minor: 1_000,
                    note: "调度器封盘前兜底测试".to_string(),
                },
                std::slice::from_ref(&lottery),
                &users,
            )
            .await
            .expect("user group buy plan can be created");
        finance
            .debit_group_buy(
                &plan.initiator_user_id,
                plan.filled_amount_minor,
                "G-USER-SCHEDULER-FILL-P001",
                &plan.id,
            )
            .await
            .expect("initiator group buy debit can be written");

        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 19:59:30".to_string(),
        )
        .await
        .expect("scheduler can fill user group buy before close");
        let stored_issue = draws.get(&issue.id).await.expect("issue exists");
        let stored_plan = group_buys.get(&plan.id).await.expect("plan exists");

        assert_eq!(stored_issue.status, DrawIssueStatus::Closed);
        assert_eq!(stored_plan.status, GroupBuyPlanStatus::Filled);
        assert!(stored_plan.order_id.is_some());
        assert!(run
            .robot_run
            .filled_plans
            .iter()
            .any(|filled| filled.id == plan.id && filled.order_id.is_some()));
        assert!(run.automation_run.ledger_entries.is_empty());
    }
    /// 验证已封盘用户合买会在退款前执行兜底补满。
    #[tokio::test]
    async fn scheduler_force_fills_closed_user_group_buy_before_refund() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "ssc60").await;
        let mut lottery = lotteries.get("ssc60").await.expect("lottery exists");
        lottery.group_buy.enabled = true;
        lotteries
            .update("ssc60", lottery.clone())
            .await
            .expect("lottery group buy can be enabled");
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let users = access.snapshot().await.expect("access can load").users;
        let config = enabled_config(1);
        let issue = draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "GROUPBUY20260602201000".to_string(),
                    scheduled_at: "2026-06-02 20:10:00".to_string(),
                    sale_closed_at: "2026-06-02 20:09:30".to_string(),
                },
            )
            .await
            .expect("draw issue can be created");
        draws.close(&issue.id).await.expect("issue can be closed");
        let plan = group_buys
            .create(
                CreateGroupBuyPlanRequest {
                    id: "G-USER-SCHEDULER-GUARD".to_string(),
                    lottery_id: lottery.id.clone(),
                    issue: issue.issue.clone(),
                    rule_code: "fiveFrontDirect".to_string(),
                    title: "用户封盘后兜底合买".to_string(),
                    numbers: "1|2|3".to_string(),
                    initiator_user_id: "U10001".to_string(),
                    total_amount_minor: 5_000,
                    initiator_amount_minor: 1_000,
                    note: "调度器流单前兜底测试".to_string(),
                },
                std::slice::from_ref(&lottery),
                &users,
            )
            .await
            .expect("user group buy plan can be created");
        finance
            .debit_group_buy(
                &plan.initiator_user_id,
                plan.filled_amount_minor,
                "G-USER-SCHEDULER-GUARD-P001",
                &plan.id,
            )
            .await
            .expect("initiator group buy debit can be written");

        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:09:45".to_string(),
        )
        .await
        .expect("scheduler can force fill closed user group buy before refund");
        let stored_issue = draws.get(&issue.id).await.expect("issue exists");
        let stored_plan = group_buys.get(&plan.id).await.expect("plan exists");

        assert_eq!(stored_issue.status, DrawIssueStatus::Closed);
        assert_eq!(stored_plan.status, GroupBuyPlanStatus::Filled);
        assert!(stored_plan.order_id.is_some());
        assert!(run
            .robot_run
            .filled_plans
            .iter()
            .any(|filled| filled.id == plan.id && filled.order_id.is_some()));
        assert!(run.automation_run.ledger_entries.is_empty());
    }
    /// 验证调度器先推送新期开奖事件再推送开奖结果。
    #[tokio::test]
    async fn scheduler_publishes_opening_events_before_draw_result() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "ssc60").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let config = enabled_config(1);
        let realtime = RealtimeHub::new();
        let mut receiver = realtime.subscribe();
        let lottery = lotteries.get("ssc60").await.expect("lottery exists");
        draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "DUE20260602200000".to_string(),
                    scheduled_at: "2026-06-02 20:00:00".to_string(),
                    sale_closed_at: "2026-06-02 19:59:59".to_string(),
                },
            )
            .await
            .expect("draw issue can be created");

        let run = super::run_draw_scheduler_once_with_realtime(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:00:00".to_string(),
            Some(&realtime),
        )
        .await
        .expect("scheduler can run with realtime events");

        let mut events = Vec::new();
        while let Ok(message) = receiver.try_recv() {
            events.push(
                message
                    .payload
                    .get("event")
                    .and_then(|event| event.as_str())
                    .unwrap_or_default()
                    .to_string(),
            );
        }

        assert_eq!(run.automation_run.closed_issues.len(), 1);
        assert_eq!(run.generated_issues.len(), 1);
        assert_eq!(run.automation_run.drawn_issues.len(), 1);
        assert_eq!(
            events,
            vec![
                "lottery.issue_closed",
                "lottery.issue_opened",
                "lottery.draw_result"
            ]
        );
    }
    /// 验证未到开奖时间不会提前开启下一期。
    #[tokio::test]
    async fn scheduler_waits_until_draw_time_before_opening_next_issue() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        enable_lottery_sale(&lotteries, "ssc60").await;
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let config = enabled_config(1);
        let lottery = lotteries.get("ssc60").await.expect("lottery exists");
        let current_issue = draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "202606020001".to_string(),
                    scheduled_at: "2026-06-02 20:01:00".to_string(),
                    sale_closed_at: "2026-06-02 20:00:30".to_string(),
                },
            )
            .await
            .expect("current issue can be created");

        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:00:30".to_string(),
        )
        .await
        .expect("scheduler can run at sale close time");
        let stored_current = draws.get(&current_issue.id).await.expect("issue exists");

        assert_eq!(stored_current.status, DrawIssueStatus::Closed);
        assert!(run.generated_issues.is_empty());

        let draw_time_run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:01:00".to_string(),
        )
        .await
        .expect("scheduler can run at draw time");

        assert!(draw_time_run
            .generated_issues
            .iter()
            .any(|issue| issue.lottery_id == "ssc60"
                && issue.issue == "202606020002"
                && issue.status == DrawIssueStatus::Open));
    }
    /// 验证调度器线程可启动但会按后台配置决定是否执行。
    #[tokio::test]
    async fn scheduler_spawn_starts_worker_even_when_disabled() {
        let handle = spawn_draw_scheduler(
            AccessRepository::memory_seeded(),
            DrawRepository::memory(),
            LotteryRepository::memory_seeded(),
            OrderRepository::memory(),
            FinanceRepository::memory_seeded(),
            GroupBuyRepository::memory_seeded(),
            RobotRepository::memory_seeded(),
            RealtimeHub::new(),
            DrawSchedulerConfig::default(),
            DrawSchedulerRepository::new(DrawSchedulerConfig::default()),
        );

        assert!(!handle.is_finished());
        handle.abort();
    }
    /// 验证后台开启调度后工作线程开始执行。
    #[tokio::test]
    async fn scheduler_worker_runs_after_backend_config_enables_it() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let scheduler = DrawSchedulerRepository::new(DrawSchedulerConfig::default());
        let handle = spawn_draw_scheduler(
            access,
            draws,
            lotteries,
            orders,
            finance,
            group_buys,
            robots,
            RealtimeHub::new(),
            DrawSchedulerConfig::default(),
            scheduler.clone(),
        );

        scheduler
            .update_config(DrawSchedulerConfig {
                enabled: true,
                interval_seconds: 1,
                future_issue_count: 1,
                sale_close_lead_seconds: DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            })
            .await
            .expect("scheduler can be enabled from backend config");

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if scheduler.status().expect("status can load").run_count > 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("scheduler should run after backend config enables it");

        handle.abort();
    }
    /// 验证调度仓储会记录成功执行摘要。
    #[tokio::test]
    async fn scheduler_repository_records_success_summary() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let group_buys = GroupBuyRepository::memory_seeded();
        let robots = RobotRepository::memory_seeded();
        let access = AccessRepository::memory_seeded();
        let config = enabled_config(1);
        let scheduler = DrawSchedulerRepository::new(config.clone());
        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
            &group_buys,
            &robots,
            &access,
            &config,
            "2026-06-02 20:00:00".to_string(),
        )
        .await
        .expect("scheduler can run");

        let record = scheduler
            .record_success(
                DrawSchedulerRunTrigger::Automatic,
                "2026-06-02 20:00:00".to_string(),
                "2026-06-02 20:00:01".to_string(),
                &run,
            )
            .await
            .expect("success record can be saved");
        let status = scheduler.status().expect("status can be read");

        assert_eq!(record.status, DrawSchedulerRunStatus::Success);
        assert_eq!(status.enabled, true);
        assert_eq!(status.run_count, 1);
        assert_eq!(
            status.last_run.as_ref().map(|run| run.id.as_str()),
            Some("SCH000000000001")
        );
        assert_eq!(
            status
                .last_run
                .as_ref()
                .map(|run| run.generated_issue_count),
            Some(run.generated_issues.len())
        );
        assert_eq!(record.skipped_lotteries.len(), run.skipped_lotteries.len());
        assert!(record
            .skipped_lotteries
            .iter()
            .any(|lottery| lottery.lottery_id == "manual-test"));
    }
    /// 验证调度仓储会记录失败执行摘要。
    #[tokio::test]
    async fn scheduler_repository_records_failure_summary() {
        let config = enabled_config(1);
        let scheduler = DrawSchedulerRepository::new(config);

        let record = scheduler
            .record_failure(
                DrawSchedulerRunTrigger::Automatic,
                "2026-06-02 20:00:00".to_string(),
                "2026-06-02 20:00:01".to_string(),
                "2026-06-02 20:00:00".to_string(),
                "boom".to_string(),
            )
            .await
            .expect("failure record can be saved");
        let status = scheduler.status().expect("status can be read");

        assert_eq!(record.status, DrawSchedulerRunStatus::Failed);
        assert_eq!(record.error.as_deref(), Some("boom"));
        assert!(record.skipped_issues.is_empty());
        assert!(record.skipped_lotteries.is_empty());
        assert_eq!(status.run_count, 1);
        assert_eq!(status.recent_runs[0].status, DrawSchedulerRunStatus::Failed);
    }
    /// 验证调度配置更新会先做参数校验。
    #[tokio::test]
    async fn scheduler_repository_updates_config_with_validation() {
        let scheduler = DrawSchedulerRepository::new(DrawSchedulerConfig::default());

        let updated = scheduler
            .update_config(DrawSchedulerConfig {
                enabled: true,
                interval_seconds: 5,
                future_issue_count: 3,
                sale_close_lead_seconds: 20,
            })
            .await
            .expect("valid scheduler config can be saved");

        assert_eq!(updated.enabled, true);
        assert_eq!(updated.config.interval_seconds, 5);
        assert_eq!(updated.config.future_issue_count, 3);
        assert_eq!(updated.config.sale_close_lead_seconds, 20);

        let error = scheduler
            .update_config(DrawSchedulerConfig {
                enabled: true,
                interval_seconds: 0,
                future_issue_count: 3,
                sale_close_lead_seconds: 20,
            })
            .await
            .expect_err("invalid interval must be rejected");

        assert!(error.to_string().contains("interval seconds"));
    }
    /// 验证调度执行历史只保留最近记录。
    #[tokio::test]
    async fn scheduler_repository_keeps_recent_history_limit() {
        let scheduler = DrawSchedulerRepository::new(enabled_config(1));
        for index in 0..25 {
            scheduler
                .record_failure(
                    DrawSchedulerRunTrigger::Automatic,
                    format!("2026-06-02 20:{index:02}:00"),
                    format!("2026-06-02 20:{index:02}:01"),
                    format!("2026-06-02 20:{index:02}:00"),
                    format!("error-{index}"),
                )
                .await
                .expect("failure record can be saved");
        }

        let status = scheduler.status().expect("status can be read");

        assert_eq!(status.run_count, 20);
        assert_eq!(status.recent_runs[0].id, "SCH000000000025");
        assert_eq!(status.recent_runs[19].id, "SCH000000000006");
    }

    #[test]
    /// 验证调度配置默认值来自数据库种子。
    fn scheduler_config_uses_database_seed_defaults() {
        let config = DrawSchedulerConfig::default();

        assert!(!config.enabled);
        assert_eq!(config.interval_seconds, 60);
        assert_eq!(config.future_issue_count, 1);
        assert_eq!(
            config.sale_close_lead_seconds,
            DEFAULT_SALE_CLOSE_LEAD_SECONDS
        );
    }

    #[test]
    /// 验证调度器拒绝非法配置值。
    fn scheduler_config_rejects_invalid_values() {
        let mut config = DrawSchedulerConfig::default();
        config.interval_seconds = 0;
        let error = config.validate().expect_err("invalid config is rejected");

        assert!(error
            .to_string()
            .contains("interval seconds must be greater than zero"));
    }

    /// 构造测试用已启用调度配置。
    fn enabled_config(future_issue_count: u32) -> DrawSchedulerConfig {
        DrawSchedulerConfig {
            enabled: true,
            interval_seconds: 60,
            future_issue_count,
            sale_close_lead_seconds: DEFAULT_SALE_CLOSE_LEAD_SECONDS,
        }
    }

    /// 为调度测试显式打开指定彩种销售状态。
    async fn enable_lottery_sale(lotteries: &LotteryRepository, lottery_id: &str) {
        lotteries
            .set_sale_enabled(lottery_id, true)
            .await
            .expect("lottery sale can be enabled for scheduler test");
    }
}
