use std::{
    collections::VecDeque,
    error::Error,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::Local;
use serde::{Deserialize, Serialize};
use tokio::{task::JoinHandle, time::MissedTickBehavior};

use crate::{
    domain::{
        draw::{
            DrawAutomationRun, DrawAutomationRunRequest, DrawIssue, DrawIssueStatus,
            GenerateDrawIssuesRequest,
        },
        lottery::LotteryKind,
    },
    error::{ApiError, ApiResult},
    services::{
        automation::run_draw_automation,
        draw::DrawRepository,
        draw_generation::{generate_draw_issue_batch, DEFAULT_SALE_CLOSE_LEAD_SECONDS},
        finance::FinanceRepository,
        lottery::LotteryRepository,
        order::OrderRepository,
    },
};

const DEFAULT_SCHEDULER_INTERVAL_SECONDS: u64 = 60;
const DEFAULT_FUTURE_ISSUE_COUNT: u32 = 1;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawSchedulerRunStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawSchedulerRunTrigger {
    Automatic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
}

#[derive(Debug)]
struct DrawSchedulerStore {
    config: DrawSchedulerConfig,
    next_sequence: u64,
    runs: VecDeque<DrawSchedulerRunRecord>,
}

impl Default for DrawSchedulerConfig {
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
    pub fn new(config: DrawSchedulerConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DrawSchedulerStore {
                config,
                next_sequence: 0,
                runs: VecDeque::new(),
            })),
        }
    }

    pub fn status(&self) -> ApiResult<DrawSchedulerStatus> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw scheduler store lock poisoned".to_string()))
            .map(|store| store.status())
    }

    pub fn config(&self) -> ApiResult<DrawSchedulerConfig> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw scheduler store lock poisoned".to_string()))
            .map(|store| store.config.clone())
    }

    pub fn update_config(&self, config: DrawSchedulerConfig) -> ApiResult<DrawSchedulerStatus> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("draw scheduler store lock poisoned".to_string()))?
            .update_config(config)
    }

    pub fn record_success(
        &self,
        trigger: DrawSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        run: &DrawSchedulerRun,
    ) -> ApiResult<DrawSchedulerRunRecord> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("draw scheduler store lock poisoned".to_string()))?
            .record_success(trigger, started_at, finished_at, run)
    }

    pub fn record_failure(
        &self,
        trigger: DrawSchedulerRunTrigger,
        started_at: String,
        finished_at: String,
        now: String,
        error: String,
    ) -> ApiResult<DrawSchedulerRunRecord> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("draw scheduler store lock poisoned".to_string()))?
            .record_failure(trigger, started_at, finished_at, now, error)
    }
}

impl DrawSchedulerStore {
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

    fn update_config(&mut self, config: DrawSchedulerConfig) -> ApiResult<DrawSchedulerStatus> {
        config.validate()?;
        self.config = config;
        Ok(self.status())
    }

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
        });
        self.push_record(record)
    }

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
        });
        self.push_record(record)
    }

    fn next_record(&mut self, mut record: DrawSchedulerRunRecord) -> DrawSchedulerRunRecord {
        self.next_sequence += 1;
        record.id = format!("SCH{:012}", self.next_sequence);
        record
    }

    fn push_record(&mut self, record: DrawSchedulerRunRecord) -> ApiResult<DrawSchedulerRunRecord> {
        self.runs.push_front(record.clone());
        while self.runs.len() > MAX_SCHEDULER_HISTORY {
            self.runs.pop_back();
        }
        Ok(record)
    }
}

impl DrawSchedulerConfig {
    pub fn from_env() -> Result<Self, Box<dyn Error + Send + Sync>> {
        Self::from_getter(|key| std::env::var(key).ok())
    }

    fn from_getter(
        mut get: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let defaults = Self::default();
        let config = Self {
            enabled: parse_bool(
                "DRAW_SCHEDULER_ENABLED",
                get("DRAW_SCHEDULER_ENABLED"),
                defaults.enabled,
            )?,
            interval_seconds: parse_u64(
                "DRAW_SCHEDULER_INTERVAL_SECONDS",
                get("DRAW_SCHEDULER_INTERVAL_SECONDS"),
                defaults.interval_seconds,
            )?,
            future_issue_count: parse_u32(
                "DRAW_SCHEDULER_FUTURE_ISSUE_COUNT",
                get("DRAW_SCHEDULER_FUTURE_ISSUE_COUNT"),
                defaults.future_issue_count,
            )?,
            sale_close_lead_seconds: parse_u32(
                "DRAW_SCHEDULER_SALE_CLOSE_LEAD_SECONDS",
                get("DRAW_SCHEDULER_SALE_CLOSE_LEAD_SECONDS"),
                defaults.sale_close_lead_seconds,
            )?,
        };

        config
            .validate()
            .map_err(|error| config_error(error.to_string()))?;
        Ok(config)
    }

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

pub fn spawn_draw_scheduler(
    draws: DrawRepository,
    lotteries: LotteryRepository,
    orders: OrderRepository,
    finance: FinanceRepository,
    config: DrawSchedulerConfig,
    scheduler: DrawSchedulerRepository,
) -> Option<JoinHandle<()>> {
    if !config.enabled {
        tracing::info!("开奖调度器已禁用");
        return None;
    }

    tracing::info!(
        interval_seconds = config.interval_seconds,
        future_issue_count = config.future_issue_count,
        sale_close_lead_seconds = config.sale_close_lead_seconds,
        "开奖调度器已启用"
    );

    Some(tokio::spawn(async move {
        let mut active_interval_seconds = config.interval_seconds;
        let mut interval = tokio::time::interval(Duration::from_secs(active_interval_seconds));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            let started_at = current_scheduler_timestamp();
            let now = started_at.clone();
            let current_config = match scheduler.config() {
                Ok(current_config) => current_config,
                Err(error) => {
                    tracing::error!(%error, "开奖调度器配置读取失败");
                    config.clone()
                }
            };
            if !current_config.enabled {
                tracing::debug!("开奖调度器因配置禁用跳过本轮执行");
                continue;
            }
            if current_config.interval_seconds != active_interval_seconds {
                active_interval_seconds = current_config.interval_seconds;
                interval = tokio::time::interval(Duration::from_secs(active_interval_seconds));
                interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
                tracing::info!(
                    interval_seconds = active_interval_seconds,
                    "开奖调度器执行周期已更新"
                );
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
                    if let Err(error) = scheduler.record_success(
                        DrawSchedulerRunTrigger::Automatic,
                        started_at,
                        finished_at,
                        &run,
                    ) {
                        tracing::error!(%error, "开奖调度器历史记录写入失败");
                    }
                    tracing::info!(
                        now = %run.now,
                        closed_issues = run.automation_run.closed_issues.len(),
                        drawn_issues = run.automation_run.drawn_issues.len(),
                        settlement_runs = run.automation_run.settlement_runs.len(),
                        ledger_entries = run.automation_run.ledger_entries.len(),
                        generated_issues = run.generated_issues.len(),
                        skipped_lotteries = run.skipped_lotteries.len(),
                        skipped_issues = run.automation_run.skipped_issues.len(),
                        "开奖调度器本轮执行完成"
                    );
                }
                Err(error) => {
                    let finished_at = current_scheduler_timestamp();
                    if let Err(record_error) = scheduler.record_failure(
                        DrawSchedulerRunTrigger::Automatic,
                        started_at,
                        finished_at,
                        now.clone(),
                        error.to_string(),
                    ) {
                        tracing::error!(%record_error, "开奖调度器历史记录写入失败");
                    }
                    tracing::error!(%now, %error, "开奖调度器本轮执行失败");
                }
            }
        }
    }))
}

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
                reason: "lottery sale is disabled".to_string(),
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
                    error = %error,
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

fn future_issue_count(issues: &[DrawIssue], lottery: &LotteryKind, now: &str) -> u32 {
    issues
        .iter()
        .filter(|issue| {
            issue.lottery_id == lottery.id
                && matches!(
                    issue.status,
                    DrawIssueStatus::Open | DrawIssueStatus::Closed
                )
                && issue.scheduled_at.as_str() >= now
        })
        .count() as u32
}

fn current_scheduler_timestamp() -> String {
    Local::now()
        .naive_local()
        .format(TIMESTAMP_FORMAT)
        .to_string()
}

fn parse_bool(
    key: &str,
    value: Option<String>,
    default: bool,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    let Some(value) = value else {
        return Ok(default);
    };

    match value.trim() {
        "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON" => Ok(true),
        "0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF" => Ok(false),
        _ => Err(config_error(format!(
            "{key} must be true/false, 1/0, yes/no, or on/off"
        ))),
    }
}

fn parse_u64(
    key: &str,
    value: Option<String>,
    default: u64,
) -> Result<u64, Box<dyn Error + Send + Sync>> {
    value
        .map(|value| {
            value
                .trim()
                .parse::<u64>()
                .map_err(|_| config_error(format!("{key} must be a positive integer")))
        })
        .unwrap_or(Ok(default))
}

fn parse_u32(
    key: &str,
    value: Option<String>,
    default: u32,
) -> Result<u32, Box<dyn Error + Send + Sync>> {
    value
        .map(|value| {
            value
                .trim()
                .parse::<u32>()
                .map_err(|_| config_error(format!("{key} must be a positive integer")))
        })
        .unwrap_or(Ok(default))
}

fn config_error(message: impl Into<String>) -> Box<dyn Error + Send + Sync> {
    Box::new(IoError::new(ErrorKind::InvalidInput, message.into()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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

        assert!(run
            .skipped_lotteries
            .iter()
            .any(|lottery| lottery.lottery_id == "fc3d"
                && lottery.reason.contains("latest issue is missing")));
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

    #[test]
    fn scheduler_spawn_returns_none_when_disabled() {
        let handle = spawn_draw_scheduler(
            DrawRepository::memory(),
            LotteryRepository::memory_seeded(),
            OrderRepository::memory(),
            FinanceRepository::memory_seeded(),
            DrawSchedulerConfig::default(),
            DrawSchedulerRepository::new(DrawSchedulerConfig::default()),
        );

        assert!(handle.is_none());
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
    }

    #[test]
    fn scheduler_repository_records_failure_summary() {
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
            .expect("failure record can be saved");
        let status = scheduler.status().expect("status can be read");

        assert_eq!(record.status, DrawSchedulerRunStatus::Failed);
        assert_eq!(record.error.as_deref(), Some("boom"));
        assert_eq!(status.run_count, 1);
        assert_eq!(status.recent_runs[0].status, DrawSchedulerRunStatus::Failed);
    }

    #[test]
    fn scheduler_repository_updates_config_with_validation() {
        let scheduler = DrawSchedulerRepository::new(DrawSchedulerConfig::default());

        let updated = scheduler
            .update_config(DrawSchedulerConfig {
                enabled: true,
                interval_seconds: 5,
                future_issue_count: 3,
                sale_close_lead_seconds: 20,
            })
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
            .expect_err("invalid interval must be rejected");

        assert!(error.to_string().contains("interval seconds"));
    }

    #[test]
    fn scheduler_repository_keeps_recent_history_limit() {
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
                .expect("failure record can be saved");
        }

        let status = scheduler.status().expect("status can be read");

        assert_eq!(status.run_count, 20);
        assert_eq!(status.recent_runs[0].id, "SCH000000000025");
        assert_eq!(status.recent_runs[19].id, "SCH000000000006");
    }

    #[test]
    fn scheduler_config_reads_environment_contract() {
        let values = HashMap::from([
            ("DRAW_SCHEDULER_ENABLED", "true"),
            ("DRAW_SCHEDULER_INTERVAL_SECONDS", "5"),
            ("DRAW_SCHEDULER_FUTURE_ISSUE_COUNT", "3"),
            ("DRAW_SCHEDULER_SALE_CLOSE_LEAD_SECONDS", "45"),
        ]);
        let config = DrawSchedulerConfig::from_getter(|key| {
            values.get(key).map(|value| (*value).to_string())
        })
        .expect("config can be parsed");

        assert!(config.enabled);
        assert_eq!(config.interval_seconds, 5);
        assert_eq!(config.future_issue_count, 3);
        assert_eq!(config.sale_close_lead_seconds, 45);
    }

    #[test]
    fn scheduler_config_rejects_invalid_values() {
        let values = HashMap::from([
            ("DRAW_SCHEDULER_ENABLED", "maybe"),
            ("DRAW_SCHEDULER_INTERVAL_SECONDS", "0"),
        ]);
        let error = DrawSchedulerConfig::from_getter(|key| {
            values.get(key).map(|value| (*value).to_string())
        })
        .expect_err("invalid bool is rejected");

        assert!(error.to_string().contains("DRAW_SCHEDULER_ENABLED"));
    }

    fn enabled_config(future_issue_count: u32) -> DrawSchedulerConfig {
        DrawSchedulerConfig {
            enabled: true,
            interval_seconds: 60,
            future_issue_count,
            sale_close_lead_seconds: DEFAULT_SALE_CLOSE_LEAD_SECONDS,
        }
    }
}
