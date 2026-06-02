use std::{
    error::Error,
    io::{Error as IoError, ErrorKind},
    time::Duration,
};

use chrono::Local;
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
const MAX_FUTURE_ISSUE_COUNT: u32 = 50;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawSchedulerConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub future_issue_count: u32,
    pub sale_close_lead_seconds: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
) -> Option<JoinHandle<()>> {
    if !config.enabled {
        tracing::info!("draw scheduler disabled");
        return None;
    }

    tracing::info!(
        interval_seconds = config.interval_seconds,
        future_issue_count = config.future_issue_count,
        sale_close_lead_seconds = config.sale_close_lead_seconds,
        "draw scheduler enabled"
    );

    Some(tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;
            let now = current_scheduler_timestamp();
            match run_draw_scheduler_once(
                &draws,
                &lotteries,
                &orders,
                &finance,
                &config,
                now.clone(),
            )
            .await
            {
                Ok(run) => {
                    tracing::info!(
                        now = %run.now,
                        closed_issues = run.automation_run.closed_issues.len(),
                        drawn_issues = run.automation_run.drawn_issues.len(),
                        settlement_runs = run.automation_run.settlement_runs.len(),
                        ledger_entries = run.automation_run.ledger_entries.len(),
                        generated_issues = run.generated_issues.len(),
                        skipped_lotteries = run.skipped_lotteries.len(),
                        skipped_issues = run.automation_run.skipped_issues.len(),
                        "draw scheduler cycle completed"
                    );
                }
                Err(error) => {
                    tracing::error!(%now, %error, "draw scheduler cycle failed");
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
        let mut created = generate_draw_issue_batch(
            draws,
            &lottery,
            GenerateDrawIssuesRequest {
                lottery_id: lottery.id.clone(),
                now: now.to_string(),
                count,
                sale_close_lead_seconds: Some(config.sale_close_lead_seconds),
            },
        )
        .await?;
        generated_issues.append(&mut created);
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
            finance::FinanceRepository,
            lottery::LotteryRepository,
            order::OrderRepository,
            scheduler::{
                run_draw_scheduler_once, spawn_draw_scheduler, DrawSchedulerConfig,
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
        );

        assert!(handle.is_none());
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
