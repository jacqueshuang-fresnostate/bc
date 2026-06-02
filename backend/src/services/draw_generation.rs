use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

use crate::{
    domain::{
        draw::{CreateDrawIssueRequest, DrawIssue, GenerateDrawIssueRequest},
        lottery::{DrawSchedule, LotteryKind},
    },
    error::{ApiError, ApiResult},
    services::draw::DrawRepository,
};

const DEFAULT_SALE_CLOSE_LEAD_SECONDS: u32 = 30;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const ISSUE_FORMAT: &str = "%Y%m%d%H%M%S";

pub async fn generate_next_draw_issue(
    draws: &DrawRepository,
    lottery: &LotteryKind,
    payload: GenerateDrawIssueRequest,
) -> ApiResult<DrawIssue> {
    validate_request(lottery, &payload)?;
    let now = parse_timestamp(&payload.now, "now")?;
    let existing_issues = draws.list().await?;
    let baseline = latest_scheduled_at(&existing_issues, &lottery.id)?.unwrap_or(now);
    let baseline = if baseline > now { baseline } else { now };
    let sale_close_lead_seconds = payload
        .sale_close_lead_seconds
        .unwrap_or(DEFAULT_SALE_CLOSE_LEAD_SECONDS);

    if sale_close_lead_seconds == 0 {
        return Err(ApiError::BadRequest(
            "sale close lead seconds must be greater than zero".to_string(),
        ));
    }

    let mut scheduled_at = next_scheduled_at(&lottery.schedule, baseline)?;
    for _ in 0..100 {
        let issue = format_issue(scheduled_at);
        if !existing_issues
            .iter()
            .any(|existing| existing.lottery_id == lottery.id && existing.issue == issue)
        {
            let sale_closed_at = scheduled_at
                .checked_sub_signed(Duration::seconds(i64::from(sale_close_lead_seconds)))
                .ok_or_else(|| {
                    ApiError::BadRequest("sale close time is out of range".to_string())
                })?;

            return draws
                .create(
                    lottery,
                    CreateDrawIssueRequest {
                        lottery_id: lottery.id.clone(),
                        issue,
                        scheduled_at: format_timestamp(scheduled_at),
                        sale_closed_at: format_timestamp(sale_closed_at),
                    },
                )
                .await;
        }

        scheduled_at = next_scheduled_at(&lottery.schedule, scheduled_at)?;
    }

    Err(ApiError::Conflict(
        "unable to generate unique draw issue after 100 attempts".to_string(),
    ))
}

fn validate_request(lottery: &LotteryKind, payload: &GenerateDrawIssueRequest) -> ApiResult<()> {
    if payload.lottery_id.trim().is_empty() {
        return Err(ApiError::BadRequest("lottery id is required".to_string()));
    }
    if payload.lottery_id.trim() != lottery.id {
        return Err(ApiError::BadRequest(
            "request lottery id does not match lottery".to_string(),
        ));
    }
    if payload.now.trim().is_empty() {
        return Err(ApiError::BadRequest("now time is required".to_string()));
    }

    Ok(())
}

fn latest_scheduled_at(issues: &[DrawIssue], lottery_id: &str) -> ApiResult<Option<NaiveDateTime>> {
    let mut latest = None;

    for issue in issues
        .iter()
        .filter(|issue| issue.lottery_id == lottery_id.trim())
    {
        let scheduled_at = parse_timestamp(&issue.scheduled_at, "existing scheduled time")?;
        if latest.is_none_or(|current| scheduled_at > current) {
            latest = Some(scheduled_at);
        }
    }

    Ok(latest)
}

fn next_scheduled_at(schedule: &DrawSchedule, baseline: NaiveDateTime) -> ApiResult<NaiveDateTime> {
    match schedule {
        DrawSchedule::Periodic { interval_seconds } => {
            if *interval_seconds == 0 {
                return Err(ApiError::BadRequest(
                    "periodic interval must be greater than zero".to_string(),
                ));
            }
            baseline
                .checked_add_signed(Duration::seconds(i64::from(*interval_seconds)))
                .ok_or_else(|| ApiError::BadRequest("scheduled time is out of range".to_string()))
        }
        DrawSchedule::Daily { time } => {
            let draw_time = parse_time(time, "daily draw time")?;
            let today = combine_date_time(baseline.date(), draw_time)?;
            if today > baseline {
                Ok(today)
            } else {
                let next_day = baseline
                    .date()
                    .checked_add_signed(Duration::days(1))
                    .ok_or_else(|| {
                        ApiError::BadRequest("scheduled date is out of range".to_string())
                    })?;
                combine_date_time(next_day, draw_time)
            }
        }
        DrawSchedule::Weekly { weekdays, time } => {
            let draw_time = parse_time(time, "weekly draw time")?;
            let weekdays = parse_weekdays(weekdays)?;
            for day_offset in 0..14 {
                let date = baseline
                    .date()
                    .checked_add_signed(Duration::days(day_offset))
                    .ok_or_else(|| {
                        ApiError::BadRequest("scheduled date is out of range".to_string())
                    })?;
                if !weekdays.contains(&date.weekday()) {
                    continue;
                }

                let candidate = combine_date_time(date, draw_time)?;
                if candidate > baseline {
                    return Ok(candidate);
                }
            }

            Err(ApiError::BadRequest(
                "weekly schedule cannot produce next draw time".to_string(),
            ))
        }
    }
}

fn parse_timestamp(value: &str, label: &str) -> ApiResult<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value.trim(), TIMESTAMP_FORMAT)
        .map_err(|_| ApiError::BadRequest(format!("{label} must use YYYY-MM-DD HH:mm:ss format")))
}

fn parse_time(value: &str, label: &str) -> ApiResult<NaiveTime> {
    NaiveTime::parse_from_str(value.trim(), "%H:%M:%S")
        .map_err(|_| ApiError::BadRequest(format!("{label} must use HH:mm:ss format")))
}

fn parse_weekdays(values: &[String]) -> ApiResult<Vec<Weekday>> {
    if values.is_empty() {
        return Err(ApiError::BadRequest(
            "weekly weekdays are required".to_string(),
        ));
    }

    values
        .iter()
        .map(|value| match value.trim() {
            "Monday" => Ok(Weekday::Mon),
            "Tuesday" => Ok(Weekday::Tue),
            "Wednesday" => Ok(Weekday::Wed),
            "Thursday" => Ok(Weekday::Thu),
            "Friday" => Ok(Weekday::Fri),
            "Saturday" => Ok(Weekday::Sat),
            "Sunday" => Ok(Weekday::Sun),
            weekday => Err(ApiError::BadRequest(format!(
                "unsupported weekly weekday `{weekday}`"
            ))),
        })
        .collect()
}

fn combine_date_time(date: NaiveDate, time: NaiveTime) -> ApiResult<NaiveDateTime> {
    Ok(date.and_time(time))
}

fn format_timestamp(value: NaiveDateTime) -> String {
    value.format(TIMESTAMP_FORMAT).to_string()
}

fn format_issue(value: NaiveDateTime) -> String {
    value.format(ISSUE_FORMAT).to_string()
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            draw::GenerateDrawIssueRequest,
            lottery::{
                DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType,
                PlayCategory,
            },
        },
        services::{draw::DrawRepository, draw_generation::generate_next_draw_issue},
    };

    #[tokio::test]
    async fn periodic_schedule_generates_next_interval_after_now() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("issue can be generated");

        assert_eq!(issue.issue, "20260602200100");
        assert_eq!(issue.scheduled_at, "2026-06-02 20:01:00");
        assert_eq!(issue.sale_closed_at, "2026-06-02 20:00:30");
    }

    #[tokio::test]
    async fn generation_uses_latest_existing_issue_as_baseline() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Periodic {
            interval_seconds: 60,
        });

        let first = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("first issue can be generated");
        let second = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 20:00:00"))
            .await
            .expect("second issue can be generated");

        assert_eq!(first.issue, "20260602200100");
        assert_eq!(second.issue, "20260602200200");
    }

    #[tokio::test]
    async fn daily_schedule_rolls_to_next_day_after_draw_time() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Daily {
            time: "21:00:15".to_string(),
        });

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 22:00:00"))
            .await
            .expect("issue can be generated");

        assert_eq!(issue.issue, "20260603210015");
        assert_eq!(issue.sale_closed_at, "2026-06-03 20:59:45");
    }

    #[tokio::test]
    async fn weekly_schedule_picks_next_matching_weekday() {
        let draws = DrawRepository::memory();
        let lottery = lottery(DrawSchedule::Weekly {
            weekdays: vec!["Tuesday".to_string(), "Thursday".to_string()],
            time: "21:00:00".to_string(),
        });

        let issue = generate_next_draw_issue(&draws, &lottery, request("2026-06-02 22:00:00"))
            .await
            .expect("issue can be generated");

        assert_eq!(issue.issue, "20260604210000");
        assert_eq!(issue.scheduled_at, "2026-06-04 21:00:00");
    }

    fn request(now: &str) -> GenerateDrawIssueRequest {
        GenerateDrawIssueRequest {
            lottery_id: "fc3d".to_string(),
            now: now.to_string(),
            sale_close_lead_seconds: None,
        }
    }

    fn lottery(schedule: DrawSchedule) -> LotteryKind {
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            schedule,
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: true,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1_000,
            },
            play_categories: vec![PlayCategory::Direct],
            play_configs: Vec::new(),
        }
    }
}
