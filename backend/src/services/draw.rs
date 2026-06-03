//! 开奖期号与开奖控制领域模型，定义状态与开奖请求参数

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::{
        draw::{
            CreateDrawIssueRequest, DrawIssue, DrawIssueResultRequest, DrawIssueStatus,
            LotteryDrawControl, SaveLotteryDrawControlRequest,
        },
        lottery::{DrawMode, DrawSource, LotteryKind, LotteryNumberType, SaveDrawSourceRequest},
    },
    error::{ApiError, ApiResult},
};

use super::{
    business_database::{enum_from_string, enum_to_string, BusinessDatabase},
    draw_api::{ApiDrawSourceLatestIssue, ApiDrawSourceRepository},
};

#[derive(Clone)]
pub struct DrawRepository {
    inner: Arc<RwLock<DrawStore>>,
    api_sources: ApiDrawSourceRepository,
    controls: Arc<RwLock<DrawControlStore>>,
    persistence: Option<BusinessDatabase>,
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
            controls: Arc::new(RwLock::new(DrawControlStore::default())),
            persistence: None,
        }
    }

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

    pub async fn list(&self) -> ApiResult<Vec<DrawIssue>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    pub async fn list_by_lottery_id(&self, lottery_id: &str) -> ApiResult<Vec<DrawIssue>> {
        let lottery_id = lottery_id.trim();
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))
            .map(|store| store.list_by_lottery_id(lottery_id))
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
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            let result = store.create(lottery, payload)?;
            (result, store.clone())
        };
        self.persist_draws(&snapshot).await?;
        Ok(result)
    }

    pub async fn close(&self, id: &str) -> ApiResult<DrawIssue> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            let result = store.close(id)?;
            (result, store.clone())
        };
        self.persist_draws(&snapshot).await?;
        Ok(result)
    }

    pub async fn draw(&self, id: &str, payload: DrawIssueResultRequest) -> ApiResult<DrawIssue> {
        let (payload, uses_control_number) = self.resolve_draw_payload(id, payload).await?;

        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            let result = store.draw(id, payload, uses_control_number)?;
            (result, store.clone())
        };
        self.persist_draws(&snapshot).await?;
        Ok(result)
    }

    pub async fn cancel(&self, id: &str) -> ApiResult<DrawIssue> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw store lock poisoned".to_string()))?;
            let result = store.cancel(id)?;
            (result, store.clone())
        };
        self.persist_draws(&snapshot).await?;
        Ok(result)
    }

    pub async fn draw_sources(&self) -> ApiResult<Vec<DrawSource>> {
        let mut sources = self.api_sources.list().await?;
        sources.extend(super::draw_api::platform_draw_source_summaries());
        Ok(sources)
    }

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

    pub async fn get_draw_control(&self, lottery: &LotteryKind) -> ApiResult<LotteryDrawControl> {
        self.controls
            .read()
            .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))?
            .get(lottery)
    }

    pub async fn save_draw_control(
        &self,
        lottery: &LotteryKind,
        payload: SaveLotteryDrawControlRequest,
    ) -> ApiResult<LotteryDrawControl> {
        let draw_number = normalize_control_draw_number(lottery, &payload)?;
        let (result, snapshot) = {
            let mut store = self
                .controls
                .write()
                .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))?;

            store.save(DrawControlConfig {
                lottery_id: lottery.id.clone(),
                enabled: payload.enabled,
                draw_number,
                updated_at: current_timestamp_label(),
            });
            (store.get(lottery)?, store.clone())
        };
        self.persist_controls(&snapshot).await?;
        Ok(result)
    }

    pub async fn create_draw_source(
        &self,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
    ) -> ApiResult<DrawSource> {
        self.api_sources.create(payload, lotteries).await
    }

    pub async fn update_draw_source(
        &self,
        id: &str,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
    ) -> ApiResult<DrawSource> {
        self.api_sources.update(id, payload, lotteries).await
    }

    pub async fn delete_draw_source(&self, id: &str) -> ApiResult<DrawSource> {
        self.api_sources.delete(id).await
    }

    pub async fn has_active_draw_control(&self, lottery_id: &str) -> ApiResult<bool> {
        self.controls
            .read()
            .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))
            .map(|store| store.active_draw_number(lottery_id).is_some())
    }

    pub async fn latest_api_issue_for_lottery(
        &self,
        lottery_id: &str,
    ) -> ApiResult<Option<ApiDrawSourceLatestIssue>> {
        self.api_sources.latest_issue_for_lottery(lottery_id).await
    }

    async fn resolve_draw_payload(
        &self,
        id: &str,
        payload: DrawIssueResultRequest,
    ) -> ApiResult<(DrawIssueResultRequest, bool)> {
        let issue = self.get(id).await?;

        if let Some(draw_number) = self.active_draw_control_number(&issue.lottery_id).await? {
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

    async fn active_draw_control_number(&self, lottery_id: &str) -> ApiResult<Option<String>> {
        self.controls
            .read()
            .map_err(|_| ApiError::Internal("draw control store lock poisoned".to_string()))
            .map(|store| store.active_draw_number(lottery_id))
    }

    async fn persist_draws(&self, store: &DrawStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_draw_issues(persistence, store).await?;
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
struct DrawStore {
    next_sequence: u64,
    issues: BTreeMap<String, DrawIssue>,
}

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
    for row in sqlx::query("SELECT lottery_id, enabled, draw_number, updated_at FROM draw_controls")
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
            "INSERT INTO draw_controls (lottery_id, enabled, draw_number, updated_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&control.lottery_id)
        .bind(control.enabled)
        .bind(&control.draw_number)
        .bind(&control.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖控制数据保存失败".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("开奖控制事务提交失败".to_string()))
}

fn max_sequence<'a>(ids: impl Iterator<Item = &'a String>, prefix: char) -> u64 {
    ids.filter_map(|id| id.strip_prefix(prefix))
        .filter_map(|value| value.parse::<u64>().ok())
        .max()
        .unwrap_or_default()
}

impl DrawStore {
    fn list(&self) -> Vec<DrawIssue> {
        self.issues.values().rev().cloned().collect()
    }

    fn list_by_lottery_id(&self, lottery_id: &str) -> Vec<DrawIssue> {
        self.issues
            .values()
            .rev()
            .filter(|issue| issue.lottery_id == lottery_id)
            .cloned()
            .collect()
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DrawControlConfig {
    lottery_id: String,
    enabled: bool,
    draw_number: Option<String>,
    updated_at: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct DrawControlStore {
    controls: BTreeMap<String, DrawControlConfig>,
}

impl DrawControlStore {
    fn get(&self, lottery: &LotteryKind) -> ApiResult<LotteryDrawControl> {
        Ok(self.summary_for(lottery))
    }

    fn save(&mut self, config: DrawControlConfig) {
        self.controls.insert(config.lottery_id.clone(), config);
    }

    fn active_draw_number(&self, lottery_id: &str) -> Option<String> {
        self.controls.get(lottery_id).and_then(|config| {
            if config.enabled {
                config.draw_number.clone()
            } else {
                None
            }
        })
    }

    fn summary_for(&self, lottery: &LotteryKind) -> LotteryDrawControl {
        let config = self.controls.get(&lottery.id);
        LotteryDrawControl {
            lottery_id: lottery.id.clone(),
            lottery_name: lottery.name.clone(),
            number_type: lottery.number_type.clone(),
            enabled: config.is_some_and(|value| value.enabled),
            draw_number: config.and_then(|value| value.draw_number.clone()),
            updated_at: config.map(|value| value.updated_at.clone()),
        }
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
            draw::{
                CreateDrawIssueRequest, DrawIssueResultRequest, DrawIssueStatus,
                SaveLotteryDrawControlRequest,
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
    async fn repository_save_draw_control_validates_number_type() {
        let lottery = lottery(DrawMode::Platform, LotteryNumberType::FiveDigit);
        let repository = DrawRepository::memory();

        let error = repository
            .save_draw_control(
                &lottery,
                SaveLotteryDrawControlRequest {
                    enabled: true,
                    draw_number: Some("2,4,7".to_string()),
                },
            )
            .await
            .expect_err("short draw control number is rejected");

        assert!(error.to_string().contains("draw number must be 5 digits"));
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

    fn create_request(issue: &str) -> CreateDrawIssueRequest {
        create_request_for("fc3d", issue)
    }

    fn create_request_for(lottery_id: &str, issue: &str) -> CreateDrawIssueRequest {
        CreateDrawIssueRequest {
            lottery_id: lottery_id.to_string(),
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
