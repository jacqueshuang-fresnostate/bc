//! 开奖调度服务，处理常驻调度配置、启动/关闭和历史记录

use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tokio::task::JoinHandle;

use crate::{
    domain::{
        draw::{
            DrawAutomationRun, DrawAutomationRunRequest, DrawAutomationSkippedIssue, DrawIssue,
            DrawIssueStatus, GenerateDrawIssuesRequest,
        },
        lottery::LotteryKind,
    },
    error::{ApiError, ApiResult},
    services::{
        automation::run_draw_automation,
        business_database::{
            enum_from_string, enum_to_string, from_json, to_json, BusinessDatabase,
        },
        draw::DrawRepository,
        draw_generation::{generate_draw_issue_batch, DEFAULT_SALE_CLOSE_LEAD_SECONDS},
        finance::FinanceRepository,
        lottery::LotteryRepository,
        order::OrderRepository,
    },
};

const DEFAULT_SCHEDULER_INTERVAL_SECONDS: u64 = 60;
const DEFAULT_FUTURE_ISSUE_COUNT: u32 = 1;
const DISABLED_SCHEDULER_POLL_SECONDS: u64 = 1;
const MAX_SCHEDULER_HISTORY: usize = 20;
const MAX_FUTURE_ISSUE_COUNT: u32 = 50;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawSchedulerConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub future_issue_count: u32,
    pub sale_close_lead_seconds: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawSchedulerSkippedLottery {
    pub lottery_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawSchedulerRun {
    pub now: String,
    pub automation_run: DrawAutomationRun,
    pub generated_issues: Vec<DrawIssue>,
    pub skipped_lotteries: Vec<DrawSchedulerSkippedLottery>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawSchedulerRunStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawSchedulerRunTrigger {
    Automatic,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawSchedulerRunRecord {
    pub id: String,
    pub trigger: DrawSchedulerRunTrigger,
    pub status: DrawSchedulerRunStatus,
    pub started_at: String,
    pub finished_at: String,
    pub now: String,
    pub error: Option<String>,
    pub closed_issue_count: usize,
    pub drawn_issue_count: usize,
    pub settlement_run_count: usize,
    pub ledger_entry_count: usize,
    pub generated_issue_count: usize,
    pub skipped_issue_count: usize,
    pub skipped_lottery_count: usize,
    pub skipped_issues: Vec<DrawAutomationSkippedIssue>,
    pub skipped_lotteries: Vec<DrawSchedulerSkippedLottery>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawSchedulerStatus {
    pub enabled: bool,
    pub config: DrawSchedulerConfig,
    pub run_count: usize,
    pub last_run: Option<DrawSchedulerRunRecord>,
    pub recent_runs: Vec<DrawSchedulerRunRecord>,
}

#[derive(Clone)]
pub struct DrawSchedulerRepository {
    inner: Arc<RwLock<DrawSchedulerStore>>,
    persistence: Option<BusinessDatabase>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct DrawSchedulerStore {
    config: DrawSchedulerConfig,
    next_sequence: u64,
    runs: VecDeque<DrawSchedulerRunRecord>,
}

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

    async fn persist(&self, store: &DrawSchedulerStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_scheduler_store(persistence, store).await?;
        }

        Ok(())
    }
}

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
                .map_err(|_| ApiError::Internal("开奖调度封盘提前量数据无效".to_string()))?,
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
            .map_err(|_| ApiError::Internal("开奖调度封盘提前量过大".to_string()))?,
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

impl DrawSchedulerStore {
    /// 创建并初始化新实例。
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

    /// 处理 update_config 的具体内部流程。
    fn update_config(&mut self, config: DrawSchedulerConfig) -> ApiResult<DrawSchedulerStatus> {
        config.validate()?;
        self.config = config;
        Ok(self.status())
    }

    /// 处理 record_success 的具体内部流程。
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
            ledger_entry_count: run.automation_run.ledger_entries.len(),
            generated_issue_count: run.generated_issues.len(),
            skipped_issue_count: run.automation_run.skipped_issues.len(),
            skipped_lottery_count: run.skipped_lotteries.len(),
            skipped_issues: run.automation_run.skipped_issues.clone(),
            skipped_lotteries: run.skipped_lotteries.clone(),
        });
        self.push_record(record)
    }

    /// 处理 record_failure 的具体内部流程。
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

    /// 处理 next_record 的具体内部流程。
    fn next_record(&mut self, mut record: DrawSchedulerRunRecord) -> DrawSchedulerRunRecord {
        self.next_sequence += 1;
        record.id = format!("SCH{:012}", self.next_sequence);
        record
    }

    /// 处理 push_record 的具体内部流程。
    fn push_record(&mut self, record: DrawSchedulerRunRecord) -> ApiResult<DrawSchedulerRunRecord> {
        self.runs.push_front(record.clone());
        while self.runs.len() > MAX_SCHEDULER_HISTORY {
            self.runs.pop_back();
        }
        Ok(record)
    }
}

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
                "draw scheduler sale close lead seconds must be greater than zero".to_string(),
            ));
        }

        Ok(())
    }
}

/// 创建并启动异步开奖调度任务。
pub fn spawn_draw_scheduler(
    draws: DrawRepository,
    lotteries: LotteryRepository,
    orders: OrderRepository,
    finance: FinanceRepository,
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

    tokio::spawn(async move {
        loop {
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
                continue;
            }

            match run_draw_scheduler_once(
                &draws,
                &lotteries,
                &orders,
                &finance,
                &current_config,
                now.clone(),
            )
            .await
            {
                Ok(run) => {
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
                        "封盘期数" = run.automation_run.closed_issues.len(),
                        "开奖期数" = run.automation_run.drawn_issues.len(),
                        "结算批次" = run.automation_run.settlement_runs.len(),
                        "入账笔数" = run.automation_run.ledger_entries.len(),
                        "新增期号" = run.generated_issues.len(),
                        "跳过彩种" = run.skipped_lotteries.len(),
                        "跳过期号" = run.automation_run.skipped_issues.len(),
                        "开奖调度器本轮执行完成"
                    );
                }
                Err(error) => {
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
                        error = %error.log_message(),
                        "开奖调度器本轮执行失败"
                    );
                }
            }

            tokio::time::sleep(Duration::from_secs(current_config.interval_seconds)).await;
        }
    })
}

/// 执行一次完整开奖调度流程。
pub async fn run_draw_scheduler_once(
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    orders: &OrderRepository,
    finance: &FinanceRepository,
    config: &DrawSchedulerConfig,
    now: String,
) -> ApiResult<DrawSchedulerRun> {
    config.validate()?;
    let now = now.trim().to_string();
    if now.is_empty() {
        return Err(ApiError::BadRequest(
            "draw scheduler time is required".to_string(),
        ));
    }

    let automation_run = run_draw_automation(
        draws,
        lotteries,
        orders,
        finance,
        DrawAutomationRunRequest { now: now.clone() },
    )
    .await?;
    let (generated_issues, skipped_lotteries) =
        ensure_future_draw_issues(draws, lotteries, config, &now).await?;

    Ok(DrawSchedulerRun {
        now,
        automation_run,
        generated_issues,
        skipped_lotteries,
    })
}

async fn ensure_future_draw_issues(
    draws: &DrawRepository,
    lotteries: &LotteryRepository,
    config: &DrawSchedulerConfig,
    now: &str,
) -> ApiResult<(Vec<DrawIssue>, Vec<DrawSchedulerSkippedLottery>)> {
    let existing_issues = draws.list().await?;
    let mut generated_issues = Vec::new();
    let mut skipped_lotteries = Vec::new();

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
        let result = generate_draw_issue_batch(
            draws,
            &lottery,
            GenerateDrawIssuesRequest {
                lottery_id: lottery.id.clone(),
                now: now.to_string(),
                count,
                sale_close_lead_seconds: Some(config.sale_close_lead_seconds),
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

    Ok((generated_issues, skipped_lotteries))
}

/// 统计未来待处理期号数量。
fn future_issue_count(issues: &[DrawIssue], lottery: &LotteryKind, now: &str) -> u32 {
    issues
        .iter()
        .filter(|issue| {
            issue.lottery_id == lottery.id
                && issue.status == DrawIssueStatus::Open
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
            lottery::DrawMode,
        },
        services::{
            draw::DrawRepository,
            draw_api::ApiDrawSourceRepository,
            finance::FinanceRepository,
            lottery::LotteryRepository,
            order::OrderRepository,
            scheduler::{
                run_draw_scheduler_once, spawn_draw_scheduler, DrawSchedulerConfig,
                DrawSchedulerRepository, DrawSchedulerRunStatus, DrawSchedulerRunTrigger,
                DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            },
        },
    };

    #[tokio::test]
    async fn scheduler_generates_future_issues_for_enabled_lotteries() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let config = enabled_config(2);

        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
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

    #[tokio::test]
    async fn scheduler_skips_lottery_when_api_issue_generation_fails() {
        let draws = DrawRepository::memory_with_api_sources(
            ApiDrawSourceRepository::api68_seeded_with_static_response(
                r#"{"errorCode":0,"result":{"businessCode":0,"data":[]}}"#,
            ),
        );
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let config = enabled_config(1);

        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
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

    #[tokio::test]
    async fn scheduler_does_not_duplicate_when_future_buffer_is_satisfied() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let config = enabled_config(1);

        run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
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
            &config,
            "2026-06-02 20:00:00".to_string(),
        )
        .await
        .expect("second scheduler run can skip generation");

        assert!(second.generated_issues.is_empty());
    }

    #[tokio::test]
    async fn scheduler_runs_due_automation_before_generating_future_issues() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
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
                && issue.issue == "20260602200100"
                && issue.draw_mode == DrawMode::Platform));
    }

    #[tokio::test]
    async fn scheduler_opens_next_issue_after_current_issue_closes() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let config = enabled_config(1);
        let lottery = lotteries.get("ssc60").await.expect("lottery exists");
        let current_issue = draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "20260602200100".to_string(),
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
            &config,
            "2026-06-02 20:00:30".to_string(),
        )
        .await
        .expect("scheduler can run at sale close time");
        let stored_current = draws.get(&current_issue.id).await.expect("issue exists");

        assert_eq!(stored_current.status, DrawIssueStatus::Closed);
        assert!(run
            .generated_issues
            .iter()
            .any(|issue| issue.lottery_id == "ssc60"
                && issue.issue == "20260602200200"
                && issue.status == DrawIssueStatus::Open));
    }

    #[tokio::test]
    async fn scheduler_spawn_starts_worker_even_when_disabled() {
        let handle = spawn_draw_scheduler(
            DrawRepository::memory(),
            LotteryRepository::memory_seeded(),
            OrderRepository::memory(),
            FinanceRepository::memory_seeded(),
            DrawSchedulerConfig::default(),
            DrawSchedulerRepository::new(DrawSchedulerConfig::default()),
        );

        assert!(!handle.is_finished());
        handle.abort();
    }

    #[tokio::test]
    async fn scheduler_worker_runs_after_backend_config_enables_it() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let scheduler = DrawSchedulerRepository::new(DrawSchedulerConfig::default());
        let handle = spawn_draw_scheduler(
            draws,
            lotteries,
            orders,
            finance,
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

    #[tokio::test]
    async fn scheduler_repository_records_success_summary() {
        let draws = DrawRepository::memory();
        let lotteries = LotteryRepository::memory_seeded();
        let orders = OrderRepository::memory();
        let finance = FinanceRepository::memory_seeded();
        let config = enabled_config(1);
        let scheduler = DrawSchedulerRepository::new(config.clone());
        let run = run_draw_scheduler_once(
            &draws,
            &lotteries,
            &orders,
            &finance,
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
    /// 处理 scheduler_config_uses_database_seed_defaults 的具体内部流程。
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
    /// 处理 scheduler_config_rejects_invalid_values 的具体内部流程。
    fn scheduler_config_rejects_invalid_values() {
        let mut config = DrawSchedulerConfig::default();
        config.interval_seconds = 0;
        let error = config.validate().expect_err("invalid config is rejected");

        assert!(error
            .to_string()
            .contains("interval seconds must be greater than zero"));
    }

    /// 处理 enabled_config 的具体内部流程。
    fn enabled_config(future_issue_count: u32) -> DrawSchedulerConfig {
        DrawSchedulerConfig {
            enabled: true,
            interval_seconds: 60,
            future_issue_count,
            sale_close_lead_seconds: DEFAULT_SALE_CLOSE_LEAD_SECONDS,
        }
    }
}
