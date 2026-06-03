use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;

use crate::{
    domain::{
        draw::DrawIssue,
        lottery::{DrawMode, DrawSource, DrawSourceProvider, LotteryKind, SaveDrawSourceRequest},
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{
    enum_from_string, enum_to_string, from_json, to_json, BusinessDatabase,
};

pub const API68_FC3D_SOURCE_ID: &str = "api68-fc3d";
pub const API68_FC3D_SOURCE_NAME: &str = "API68 福彩 3D/排列 3";
pub const API68_FC3D_LOTTERY_ID: &str = "fc3d";
pub const API68_PL3_LOTTERY_ID: &str = "pl3";
pub const API68_FC3D_LOT_CODE: &str = "10041";
pub const API68_AU5_SOURCE_ID: &str = "api68-au5";
pub const API68_AU5_SOURCE_NAME: &str = "API68 澳洲 5 分彩";
pub const API68_AU5_LOTTERY_ID: &str = "au5";
pub const API68_AU5_LOT_CODE: &str = "10010";
const DEFAULT_API68_QUANGUOCAI_ENDPOINT: &str =
    "https://api.api68.com/QuanGuoCai/getLotteryInfoList.do";
const DEFAULT_API68_CQSHICAI_ENDPOINT: &str =
    "https://api.api68.com/CQShiCai/getBaseCQShiCaiList.do";
const API_DRAW_SOURCE_TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiDrawSourceLatestIssue {
    pub issue: String,
}

#[derive(Clone)]
pub struct ApiDrawSourceRepository {
    client: reqwest::Client,
    inner: Arc<RwLock<ApiDrawSourceStore>>,
    static_responses: Arc<BTreeMap<String, String>>,
    persistence: Option<BusinessDatabase>,
}

impl ApiDrawSourceRepository {
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn api68_seeded() -> Self {
        Self::new(vec![
            ApiDrawSourceConfig::api68_fc3d(),
            ApiDrawSourceConfig::api68_au5(),
        ])
    }

    fn new(sources: Vec<ApiDrawSourceConfig>) -> Self {
        Self {
            client: reqwest::Client::new(),
            inner: Arc::new(RwLock::new(ApiDrawSourceStore::new(sources))),
            static_responses: Arc::new(BTreeMap::new()),
            persistence: None,
        }
    }

    pub async fn persistent_api68_seeded(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_draw_source_store(&persistence).await?;
        Ok(Self {
            client: reqwest::Client::new(),
            inner: Arc::new(RwLock::new(store)),
            static_responses: Arc::new(BTreeMap::new()),
            persistence: Some(persistence),
        })
    }

    #[cfg(test)]
    pub fn api68_seeded_with_static_response(response_body: impl Into<String>) -> Self {
        let mut static_responses = BTreeMap::new();
        let response_body = response_body.into();
        static_responses.insert(API68_FC3D_SOURCE_ID.to_string(), response_body.clone());
        static_responses.insert(API68_AU5_SOURCE_ID.to_string(), response_body);

        Self {
            client: reqwest::Client::new(),
            inner: Arc::new(RwLock::new(ApiDrawSourceStore::new(vec![
                ApiDrawSourceConfig::api68_fc3d(),
                ApiDrawSourceConfig::api68_au5(),
            ]))),
            static_responses: Arc::new(static_responses),
            persistence: None,
        }
    }

    pub async fn list(&self) -> ApiResult<Vec<DrawSource>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw source store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    pub async fn create(
        &self,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
    ) -> ApiResult<DrawSource> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw source store lock poisoned".to_string()))?;
            let result = store.create(payload, lotteries)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    pub async fn update(
        &self,
        id: &str,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
    ) -> ApiResult<DrawSource> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw source store lock poisoned".to_string()))?;
            let result = store.update(id, payload, lotteries)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    pub async fn delete(&self, id: &str) -> ApiResult<DrawSource> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("draw source store lock poisoned".to_string()))?;
            let result = store.delete(id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    pub async fn draw_number_for(&self, issue: &DrawIssue) -> ApiResult<Option<String>> {
        if issue.draw_mode != DrawMode::Api {
            return Ok(None);
        }

        let source = {
            self.inner
                .read()
                .map_err(|_| ApiError::Internal("draw source store lock poisoned".to_string()))?
                .find_for_lottery(&issue.lottery_id)
        };

        let Some(source) = source else {
            return Ok(None);
        };

        let result = match source.provider {
            DrawSourceProvider::Api68 => self.fetch_api68_draw_number(&source, &issue.issue).await,
        };

        if let Err(error) = &result {
            tracing::error!(
                source_id = %source.id,
                lottery_id = %issue.lottery_id,
                issue = %issue.issue,
                error = %error.log_message(),
                "API 开奖源获取开奖号码失败"
            );
        }

        result.map(Some)
    }

    pub async fn latest_issue_for_lottery(
        &self,
        lottery_id: &str,
    ) -> ApiResult<Option<ApiDrawSourceLatestIssue>> {
        let source = {
            self.inner
                .read()
                .map_err(|_| ApiError::Internal("draw source store lock poisoned".to_string()))?
                .find_for_lottery(lottery_id)
        };

        let Some(source) = source else {
            return Ok(None);
        };

        let result = match source.provider {
            DrawSourceProvider::Api68 => self.fetch_api68_latest_issue(&source).await,
        };

        if let Err(error) = &result {
            tracing::error!(
                source_id = %source.id,
                lottery_id = %lottery_id,
                error = %error.log_message(),
                "API 开奖源获取最新期号失败"
            );
        }

        result.map(Some)
    }

    async fn fetch_api68_draw_number(
        &self,
        source: &ApiDrawSourceConfig,
        issue: &str,
    ) -> ApiResult<String> {
        if let Some(response_body) = self.static_responses.get(&source.id) {
            return parse_api68_draw_number(response_body, issue);
        }

        let response = self
            .client
            .get(source.url())
            .timeout(Duration::from_secs(API_DRAW_SOURCE_TIMEOUT_SECONDS))
            .send()
            .await
            .map_err(|_| ApiError::Internal("api draw source request failed".to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Internal(format!(
                "api draw source returned http status {status}"
            )));
        }

        let response_body = response
            .text()
            .await
            .map_err(|_| ApiError::Internal("api draw source response read failed".to_string()))?;

        parse_api68_draw_number(&response_body, issue)
    }

    async fn fetch_api68_latest_issue(
        &self,
        source: &ApiDrawSourceConfig,
    ) -> ApiResult<ApiDrawSourceLatestIssue> {
        if let Some(response_body) = self.static_responses.get(&source.id) {
            return parse_api68_latest_issue(response_body);
        }

        let response = self
            .client
            .get(source.url())
            .timeout(Duration::from_secs(API_DRAW_SOURCE_TIMEOUT_SECONDS))
            .send()
            .await
            .map_err(|_| ApiError::Internal("api draw source request failed".to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Internal(format!(
                "api draw source returned http status {status}"
            )));
        }

        let response_body = response
            .text()
            .await
            .map_err(|_| ApiError::Internal("api draw source response read failed".to_string()))?;

        parse_api68_latest_issue(&response_body)
    }

    async fn persist(&self, store: &ApiDrawSourceStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_draw_source_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct ApiDrawSourceStore {
    sources: BTreeMap<String, ApiDrawSourceConfig>,
}

async fn load_draw_source_store(database: &BusinessDatabase) -> ApiResult<ApiDrawSourceStore> {
    let mut sources = Vec::new();
    for row in sqlx::query(
        "SELECT id, name, provider, lot_code, endpoint, reusable_for_lottery_ids
         FROM draw_sources
         ORDER BY id ASC",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("开奖源数据读取失败".to_string()))?
    {
        sources.push(ApiDrawSourceConfig {
            id: row
                .try_get("id")
                .map_err(|_| ApiError::Internal("开奖源数据读取失败".to_string()))?,
            name: row
                .try_get("name")
                .map_err(|_| ApiError::Internal("开奖源数据读取失败".to_string()))?,
            provider: enum_from_string(
                row.try_get("provider")
                    .map_err(|_| ApiError::Internal("开奖源数据读取失败".to_string()))?,
            )?,
            lot_code: row
                .try_get("lot_code")
                .map_err(|_| ApiError::Internal("开奖源数据读取失败".to_string()))?,
            endpoint: row
                .try_get("endpoint")
                .map_err(|_| ApiError::Internal("开奖源数据读取失败".to_string()))?,
            reusable_for_lottery_ids: from_json(
                row.try_get("reusable_for_lottery_ids")
                    .map_err(|_| ApiError::Internal("开奖源数据读取失败".to_string()))?,
            )?,
        });
    }

    if sources.is_empty() {
        let store = ApiDrawSourceStore::new(vec![
            ApiDrawSourceConfig::api68_fc3d(),
            ApiDrawSourceConfig::api68_au5(),
        ]);
        save_draw_source_store(database, &store).await?;
        return Ok(store);
    }

    Ok(ApiDrawSourceStore::new(sources))
}

async fn save_draw_source_store(
    database: &BusinessDatabase,
    store: &ApiDrawSourceStore,
) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("开奖源事务开启失败".to_string()))?;
    sqlx::query("DELETE FROM draw_sources")
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖源数据清理失败".to_string()))?;

    for source in store.sources.values() {
        sqlx::query(
            "INSERT INTO draw_sources
             (id, name, provider, lot_code, endpoint, reusable_for_lottery_ids)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(&source.id)
        .bind(&source.name)
        .bind(enum_to_string(&source.provider)?)
        .bind(&source.lot_code)
        .bind(&source.endpoint)
        .bind(to_json(&source.reusable_for_lottery_ids)?)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("开奖源数据保存失败".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("开奖源事务提交失败".to_string()))
}

impl ApiDrawSourceStore {
    fn new(sources: Vec<ApiDrawSourceConfig>) -> Self {
        Self {
            sources: sources
                .into_iter()
                .map(|source| (source.id.clone(), source))
                .collect(),
        }
    }

    fn list(&self) -> Vec<DrawSource> {
        self.sources
            .values()
            .map(ApiDrawSourceConfig::summary)
            .collect()
    }

    fn find_for_lottery(&self, lottery_id: &str) -> Option<ApiDrawSourceConfig> {
        self.sources
            .values()
            .find(|source| {
                source
                    .reusable_for_lottery_ids
                    .iter()
                    .any(|id| id == lottery_id)
            })
            .cloned()
    }

    fn create(
        &mut self,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
    ) -> ApiResult<DrawSource> {
        let source = self.source_from_request(payload, lotteries, None)?;
        if self.sources.contains_key(&source.id) {
            return Err(ApiError::Conflict(format!(
                "draw source `{}` already exists",
                source.id
            )));
        }

        self.validate_lottery_bindings(&source, None)?;
        self.sources.insert(source.id.clone(), source.clone());
        Ok(source.summary())
    }

    fn update(
        &mut self,
        id: &str,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
    ) -> ApiResult<DrawSource> {
        if !self.sources.contains_key(id) {
            return Err(ApiError::NotFound(format!("draw source `{id}` not found")));
        }

        let source = self.source_from_request(payload, lotteries, Some(id))?;
        self.validate_lottery_bindings(&source, Some(id))?;
        self.sources.insert(id.to_string(), source.clone());
        Ok(source.summary())
    }

    fn delete(&mut self, id: &str) -> ApiResult<DrawSource> {
        self.sources
            .remove(id)
            .map(|source| source.summary())
            .ok_or_else(|| ApiError::NotFound(format!("draw source `{id}` not found")))
    }

    fn source_from_request(
        &self,
        payload: SaveDrawSourceRequest,
        lotteries: &[LotteryKind],
        expected_id: Option<&str>,
    ) -> ApiResult<ApiDrawSourceConfig> {
        let id = payload.id.trim().to_string();
        if id.is_empty() {
            return Err(ApiError::BadRequest(
                "draw source id is required".to_string(),
            ));
        }
        if !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(ApiError::BadRequest(
                "draw source id can only contain letters, numbers, hyphen and underscore"
                    .to_string(),
            ));
        }
        if let Some(expected_id) = expected_id {
            if id != expected_id {
                return Err(ApiError::BadRequest(
                    "draw source id cannot be changed".to_string(),
                ));
            }
        }

        let name = payload.name.trim().to_string();
        if name.is_empty() {
            return Err(ApiError::BadRequest(
                "draw source name is required".to_string(),
            ));
        }

        let lot_code = payload.lot_code.trim().to_string();
        if lot_code.is_empty() {
            return Err(ApiError::BadRequest("lot code is required".to_string()));
        }
        if !lot_code.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ApiError::BadRequest(
                "lot code can only contain digits".to_string(),
            ));
        }

        let reusable_for_lottery_ids = reusable_lottery_ids(payload.reusable_for_lottery_ids)?;
        validate_reusable_lotteries(&reusable_for_lottery_ids, lotteries)?;

        Ok(ApiDrawSourceConfig {
            endpoint: normalized_endpoint(payload.endpoint.as_deref()),
            id,
            lot_code,
            name,
            provider: payload.provider,
            reusable_for_lottery_ids,
        })
    }

    fn validate_lottery_bindings(
        &self,
        source: &ApiDrawSourceConfig,
        current_id: Option<&str>,
    ) -> ApiResult<()> {
        for lottery_id in &source.reusable_for_lottery_ids {
            if let Some(existing) = self.sources.values().find(|existing| {
                Some(existing.id.as_str()) != current_id
                    && existing
                        .reusable_for_lottery_ids
                        .iter()
                        .any(|id| id == lottery_id)
            }) {
                return Err(ApiError::Conflict(format!(
                    "lottery `{lottery_id}` is already bound to draw source `{}`",
                    existing.id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ApiDrawSourceConfig {
    id: String,
    name: String,
    provider: DrawSourceProvider,
    lot_code: String,
    endpoint: String,
    reusable_for_lottery_ids: Vec<String>,
}

impl ApiDrawSourceConfig {
    fn api68_fc3d() -> Self {
        Self {
            id: API68_FC3D_SOURCE_ID.to_string(),
            name: API68_FC3D_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::Api68,
            lot_code: API68_FC3D_LOT_CODE.to_string(),
            endpoint: default_api68_quanguocai_endpoint(),
            reusable_for_lottery_ids: vec![
                API68_FC3D_LOTTERY_ID.to_string(),
                API68_PL3_LOTTERY_ID.to_string(),
            ],
        }
    }

    fn api68_au5() -> Self {
        Self {
            id: API68_AU5_SOURCE_ID.to_string(),
            name: API68_AU5_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::Api68,
            lot_code: API68_AU5_LOT_CODE.to_string(),
            endpoint: default_api68_cqshicai_endpoint(),
            reusable_for_lottery_ids: vec![API68_AU5_LOTTERY_ID.to_string()],
        }
    }

    fn url(&self) -> String {
        api68_url(&self.endpoint, &self.lot_code)
    }

    fn summary(&self) -> DrawSource {
        DrawSource {
            editable: true,
            endpoint: Some(self.endpoint.clone()),
            id: self.id.clone(),
            lot_code: Some(self.lot_code.clone()),
            name: self.name.clone(),
            mode: DrawMode::Api,
            provider: Some(self.provider.clone()),
            reusable_for_lottery_ids: self.reusable_for_lottery_ids.clone(),
        }
    }
}

pub fn platform_draw_source_summaries() -> Vec<DrawSource> {
    vec![DrawSource {
        editable: false,
        endpoint: None,
        id: "platform-random-5d".to_string(),
        lot_code: None,
        name: "平台 5 位随机生成器".to_string(),
        mode: DrawMode::Platform,
        provider: None,
        reusable_for_lottery_ids: vec!["ssc60".to_string()],
    }]
}

fn default_api68_quanguocai_endpoint() -> String {
    DEFAULT_API68_QUANGUOCAI_ENDPOINT.to_string()
}

fn default_api68_cqshicai_endpoint() -> String {
    DEFAULT_API68_CQSHICAI_ENDPOINT.to_string()
}

fn normalized_endpoint(endpoint: Option<&str>) -> String {
    endpoint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(default_api68_quanguocai_endpoint)
}

fn api68_url(endpoint: &str, lot_code: &str) -> String {
    format!("{endpoint}?lotCode={lot_code}")
}

fn reusable_lottery_ids(values: Vec<String>) -> ApiResult<Vec<String>> {
    let mut unique = BTreeSet::new();
    let ids = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| unique.insert(value.clone()))
        .collect::<Vec<_>>();

    if ids.is_empty() {
        return Err(ApiError::BadRequest(
            "reusable lottery ids are required".to_string(),
        ));
    }

    Ok(ids)
}

fn validate_reusable_lotteries(
    reusable_for_lottery_ids: &[String],
    lotteries: &[LotteryKind],
) -> ApiResult<()> {
    for lottery_id in reusable_for_lottery_ids {
        let lottery = lotteries
            .iter()
            .find(|lottery| lottery.id == *lottery_id)
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{lottery_id}` not found")))?;

        if lottery.draw_mode != DrawMode::Api {
            return Err(ApiError::BadRequest(format!(
                "lottery `{lottery_id}` is not api draw mode"
            )));
        }
    }

    Ok(())
}

pub(crate) fn parse_api68_draw_number(
    response_body: &str,
    expected_issue: &str,
) -> ApiResult<String> {
    let expected_issue = expected_issue.trim();
    if expected_issue.is_empty() {
        return Err(ApiError::BadRequest("issue is required".to_string()));
    }

    let result = parse_api68_result(response_body)?;

    let Some(draw) = result.data.into_iter().find(|draw| {
        api68_issue_value(&draw.pre_draw_issue)
            .as_deref()
            .is_some_and(|issue| issue == expected_issue)
    }) else {
        return Err(ApiError::NotFound(format!(
            "api draw number for issue `{expected_issue}` not found"
        )));
    };

    let draw_code = draw.pre_draw_code.trim();
    if draw_code.is_empty() {
        return Err(ApiError::Internal(
            "api draw source draw number is empty".to_string(),
        ));
    }

    Ok(draw_code.to_string())
}

pub(crate) fn parse_api68_latest_issue(response_body: &str) -> ApiResult<ApiDrawSourceLatestIssue> {
    let result = parse_api68_result(response_body)?;
    let Some(issue) = result
        .data
        .into_iter()
        .find_map(|draw| api68_issue_value(&draw.pre_draw_issue))
        .filter(|issue| !issue.trim().is_empty())
    else {
        return Err(ApiError::Internal(
            "api draw source latest issue is missing".to_string(),
        ));
    };

    Ok(ApiDrawSourceLatestIssue { issue })
}

fn parse_api68_result(response_body: &str) -> ApiResult<Api68Result> {
    let envelope = serde_json::from_str::<Api68Envelope>(response_body)
        .map_err(|_| ApiError::Internal("api draw source response cannot be parsed".to_string()))?;

    if envelope.error_code != 0 {
        return Err(ApiError::Internal(format!(
            "api draw source returned error code {}",
            envelope.error_code
        )));
    }

    let result = envelope
        .result
        .ok_or_else(|| ApiError::Internal("api draw source result is missing".to_string()))?;

    if result.business_code != 0 {
        return Err(ApiError::Internal(format!(
            "api draw source returned business code {}",
            result.business_code
        )));
    }

    Ok(result)
}

fn api68_issue_value(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.trim().to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Api68Envelope {
    error_code: i64,
    #[serde(default)]
    result: Option<Api68Result>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Api68Result {
    business_code: i64,
    #[serde(default)]
    data: Vec<Api68Draw>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Api68Draw {
    pre_draw_issue: Value,
    pre_draw_code: String,
}

#[cfg(test)]
mod tests {
    use super::{
        parse_api68_draw_number, parse_api68_latest_issue, ApiDrawSourceRepository,
        API68_AU5_SOURCE_ID, API68_FC3D_SOURCE_ID,
    };
    use crate::{
        domain::lottery::{DrawSourceProvider, SaveDrawSourceRequest},
        services::lottery::seed_lotteries,
    };

    const API68_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "操作成功",
        "result": {
            "businessCode": 0,
            "message": "操作成功",
            "data": [
                { "preDrawIssue": 2026143, "preDrawCode": "3,7,6", "preDrawTime": "2026-06-02 21:15:00" },
                { "preDrawIssue": "2026142", "preDrawCode": "8,9,4", "preDrawTime": "2026-06-01 21:15:00" }
            ]
        }
    }"#;

    #[test]
    fn parse_api68_draw_number_matches_numeric_issue() {
        let draw_number =
            parse_api68_draw_number(API68_SAMPLE, "2026143").expect("draw number can be parsed");

        assert_eq!(draw_number, "3,7,6");
    }

    #[test]
    fn parse_api68_draw_number_matches_string_issue() {
        let draw_number =
            parse_api68_draw_number(API68_SAMPLE, "2026142").expect("draw number can be parsed");

        assert_eq!(draw_number, "8,9,4");
    }

    #[test]
    fn parse_api68_draw_number_rejects_missing_issue() {
        let error = parse_api68_draw_number(API68_SAMPLE, "2099999")
            .expect_err("missing issue is rejected");

        assert!(error.to_string().contains("not found"));
    }

    #[test]
    fn parse_api68_draw_number_rejects_business_failure() {
        let error = parse_api68_draw_number(
            r#"{"errorCode":0,"result":{"businessCode":1,"data":[]}}"#,
            "2026143",
        )
        .expect_err("business failure is rejected");

        assert!(error.to_string().contains("business code 1"));
    }

    #[test]
    fn parse_api68_latest_issue_uses_first_result_issue() {
        let latest = parse_api68_latest_issue(API68_SAMPLE).expect("latest issue can be parsed");

        assert_eq!(latest.issue, "2026143");
    }

    #[tokio::test]
    async fn seeded_static_source_returns_latest_issue_for_reused_lotteries() {
        let repository = ApiDrawSourceRepository::api68_seeded_with_static_response(API68_SAMPLE);

        let fc3d = repository
            .latest_issue_for_lottery("fc3d")
            .await
            .expect("latest issue can be fetched")
            .expect("fc3d has source");
        let pl3 = repository
            .latest_issue_for_lottery("pl3")
            .await
            .expect("latest issue can be fetched")
            .expect("pl3 has source");

        assert_eq!(fc3d.issue, "2026143");
        assert_eq!(pl3.issue, "2026143");
    }

    #[tokio::test]
    async fn seeded_api68_source_reuses_fc3d_and_pl3() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let sources = repository.list().await.expect("sources can be listed");
        let source = sources
            .iter()
            .find(|source| source.id == API68_FC3D_SOURCE_ID)
            .expect("seeded source exists");

        assert_eq!(source.lot_code.as_deref(), Some("10041"));
        assert!(source
            .reusable_for_lottery_ids
            .contains(&"fc3d".to_string()));
        assert!(source.reusable_for_lottery_ids.contains(&"pl3".to_string()));
    }

    #[tokio::test]
    async fn seeded_api68_source_includes_au5() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let sources = repository.list().await.expect("sources can be listed");
        let source = sources
            .iter()
            .find(|source| source.id == API68_AU5_SOURCE_ID)
            .expect("au5 seeded source exists");

        assert_eq!(source.lot_code.as_deref(), Some("10010"));
        assert_eq!(
            source.endpoint.as_deref(),
            Some("https://api.api68.com/CQShiCai/getBaseCQShiCaiList.do")
        );
        assert_eq!(source.reusable_for_lottery_ids, vec!["au5".to_string()]);
    }

    #[tokio::test]
    async fn source_create_rejects_duplicate_lottery_binding() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let lotteries = seed_lotteries();

        let error = repository
            .create(draw_source_payload("api68-copy", &["pl3"]), &lotteries)
            .await
            .expect_err("duplicate lottery binding is rejected");

        assert!(error.to_string().contains("already bound"));
    }

    #[tokio::test]
    async fn source_update_can_split_reusable_lottery_binding() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let lotteries = seed_lotteries();

        let updated = repository
            .update(
                API68_FC3D_SOURCE_ID,
                draw_source_payload(API68_FC3D_SOURCE_ID, &["fc3d"]),
                &lotteries,
            )
            .await
            .expect("source can be updated");
        let created = repository
            .create(draw_source_payload("api68-pl3", &["pl3"]), &lotteries)
            .await
            .expect("pl3 source can be created after split");

        assert_eq!(updated.reusable_for_lottery_ids, vec!["fc3d".to_string()]);
        assert_eq!(created.reusable_for_lottery_ids, vec!["pl3".to_string()]);
    }

    #[tokio::test]
    async fn source_save_rejects_non_api_lottery() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let lotteries = seed_lotteries();

        let error = repository
            .update(
                API68_FC3D_SOURCE_ID,
                draw_source_payload(API68_FC3D_SOURCE_ID, &["ssc60"]),
                &lotteries,
            )
            .await
            .expect_err("platform lottery cannot bind api source");

        assert!(error.to_string().contains("not api draw mode"));
    }

    fn draw_source_payload(id: &str, lottery_ids: &[&str]) -> SaveDrawSourceRequest {
        SaveDrawSourceRequest {
            endpoint: None,
            id: id.to_string(),
            lot_code: "10041".to_string(),
            name: format!("测试来源 {id}"),
            provider: DrawSourceProvider::Api68,
            reusable_for_lottery_ids: lottery_ids.iter().map(|id| id.to_string()).collect(),
        }
    }
}
