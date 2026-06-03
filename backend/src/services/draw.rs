use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    domain::{
        draw::{CreateDrawIssueRequest, DrawIssue, DrawIssueResultRequest, DrawIssueStatus},
        lottery::{DrawMode, LotteryKind, LotteryNumberType},
    },
    error::{ApiError, ApiResult},
};

use super::draw_api::ApiDrawSourceRepository;

#[derive(Clone)]
pub struct DrawRepository {
    inner: Arc<RwLock<DrawStore>>,
    api_sources: ApiDrawSourceRepository,
}

impl DrawRepository {
    #[allow(dead_code)]
    pub fn memory() -> Self {
        Self::memory_with_api_sources(ApiDrawSourceRepository::empty())
    }

    pub fn memory_with_api_sources(api_sources: ApiDrawSourceRepository) -> Self {
        Self {
            inner: Arc::new(RwLock::new(DrawStore::default())),
            api_sources,
        }
    }

    pub async fn list(&self) -> ApiResult<Vec<DrawIssue>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    pub async fn get(&self, id: &str) -> ApiResult<DrawIssue> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
            .get(id)
    }

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

    pub async fn create(
        &self,
        lottery: &LotteryKind,
        payload: CreateDrawIssueRequest,
    ) -> ApiResult<DrawIssue> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
            .create(lottery, payload)
    }

    pub async fn close(&self, id: &str) -> ApiResult<DrawIssue> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
            .close(id)
    }

    pub async fn draw(&self, id: &str, payload: DrawIssueResultRequest) -> ApiResult<DrawIssue> {
        let payload = self.resolve_draw_payload(id, payload).await?;

        self.inner
            .write()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
            .draw(id, payload)
    }

    pub async fn cancel(&self, id: &str) -> ApiResult<DrawIssue> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?
            .cancel(id)
    }

    async fn resolve_draw_payload(
        &self,
        id: &str,
        payload: DrawIssueResultRequest,
    ) -> ApiResult<DrawIssueResultRequest> {
        let issue = self.get(id).await?;

        if issue.draw_mode != DrawMode::Api {
            return Ok(payload);
        }

        if let Some(draw_number) = self.api_sources.draw_number_for(&issue).await? {
            return Ok(DrawIssueResultRequest {
                draw_number: Some(draw_number),
            });
        }

        Ok(DrawIssueResultRequest::default())
    }
}

#[derive(Debug, Default)]
struct DrawStore {
    next_sequence: u64,
    issues: BTreeMap<String, DrawIssue>,
}

impl DrawStore {
    fn list(&self) -> Vec<DrawIssue> {
        self.issues.values().rev().cloned().collect()
    }

    fn get(&self, id: &str) -> ApiResult<DrawIssue> {
        self.issues
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("draw issue `{id}` not found")))
    }

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

    fn draw(&mut self, id: &str, payload: DrawIssueResultRequest) -> ApiResult<DrawIssue> {
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
}

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

fn normalize_draw_number(draw_number: &str, number_type: &LotteryNumberType) -> ApiResult<String> {
    let expected_len = match number_type {
        LotteryNumberType::ThreeDigit => 3,
        LotteryNumberType::FiveDigit => 5,
    };
    let digits = draw_number_digits(draw_number)?;

    if digits.len() != expected_len {
        return Err(ApiError::BadRequest(format!(
            "draw number must be {expected_len} digits"
        )));
    }

    Ok(format_draw_number(&digits))
}

fn draw_number_digits(draw_number: &str) -> ApiResult<Vec<u8>> {
    let value = draw_number.trim();
    if value.contains(',') || value.contains('，') {
        return value
            .split([',', '，'])
            .map(|part| parse_draw_digit(part.trim()))
            .collect();
    }

    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "draw number must contain digits separated by commas".to_string(),
        ));
    }

    Ok(value.bytes().map(|byte| byte - b'0').collect())
}

fn parse_draw_digit(value: &str) -> ApiResult<u8> {
    if value.len() != 1 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "draw number must contain digits separated by commas".to_string(),
        ));
    }

    Ok(value.as_bytes()[0] - b'0')
}

fn format_draw_number(digits: &[u8]) -> String {
    digits
        .iter()
        .map(|digit| digit.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn generated_draw_number(number_type: &LotteryNumberType, lottery_id: &str, issue: &str) -> String {
    let len = match number_type {
        LotteryNumberType::ThreeDigit => 3,
        LotteryNumberType::FiveDigit => 5,
    };
    let mut seed = 14_695_981_039_346_656_037u64;
    for byte in lottery_id.bytes().chain(issue.bytes()) {
        seed ^= u64::from(byte);
        seed = seed.wrapping_mul(1_099_511_628_211);
    }

    let digits = (0..len)
        .map(|index| {
            seed = seed
                .wrapping_mul(1_103_515_245)
                .wrapping_add(12_345 + index as u64);
            (seed % 10) as u8
        })
        .collect::<Vec<_>>();
    format_draw_number(&digits)
}

fn current_timestamp_label() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            draw::{CreateDrawIssueRequest, DrawIssueResultRequest, DrawIssueStatus},
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

    #[test]
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
    fn manual_draw_requires_valid_draw_number() {
        let lottery = lottery(DrawMode::Manual, LotteryNumberType::FiveDigit);
        let mut store = DrawStore::default();
        let issue = store
            .create(&lottery, create_request("20260602-test"))
            .expect("issue can be created");

        assert!(store
            .draw(&issue.id, DrawIssueResultRequest { draw_number: None })
            .expect_err("manual draw without number is invalid")
            .to_string()
            .contains("manual draw requires draw number"));

        let drawn = store
            .draw(
                &issue.id,
                DrawIssueResultRequest {
                    draw_number: Some("7,8,9,4,2".to_string()),
                },
            )
            .expect("manual draw can be recorded");

        assert_eq!(drawn.status, DrawIssueStatus::Drawn);
        assert_eq!(drawn.draw_number.as_deref(), Some("7,8,9,4,2"));
    }

    #[test]
    fn platform_draw_generates_number_for_number_type() {
        let lottery = lottery(DrawMode::Platform, LotteryNumberType::FiveDigit);
        let mut store = DrawStore::default();
        let issue = store
            .create(&lottery, create_request("20260602-001"))
            .expect("issue can be created");

        let drawn = store
            .draw(&issue.id, DrawIssueResultRequest::default())
            .expect("platform draw can be generated");

        let draw_number = drawn.draw_number.expect("draw number exists");
        assert_eq!(draw_number.split(',').count(), 5);
        assert!(draw_number
            .split(',')
            .all(|part| part.len() == 1 && part.bytes().all(|byte| byte.is_ascii_digit())));
    }

    #[test]
    fn drawn_issue_cannot_be_cancelled_or_redrawn() {
        let lottery = lottery(DrawMode::Api, LotteryNumberType::ThreeDigit);
        let mut store = DrawStore::default();
        let issue = store
            .create(&lottery, create_request("2026156"))
            .expect("issue can be created");

        store
            .draw(&issue.id, DrawIssueResultRequest::default())
            .expect("issue can be drawn");

        assert!(store
            .cancel(&issue.id)
            .expect_err("drawn issue cannot be cancelled")
            .to_string()
            .contains("drawn draw issue cannot be cancelled"));
        assert!(store
            .draw(&issue.id, DrawIssueResultRequest::default())
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

    fn create_request(issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: "fc3d".to_string(),
            issue: issue.to_string(),
            scheduled_at: "2026-06-02 21:00:15".to_string(),
            sale_closed_at: "2026-06-02 20:59:45".to_string(),
        }
    }

    fn lottery(draw_mode: DrawMode, number_type: LotteryNumberType) -> LotteryKind {
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            number_type,
            draw_mode,
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
}
