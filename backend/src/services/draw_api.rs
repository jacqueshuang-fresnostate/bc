use std::{collections::BTreeMap, sync::Arc, time::Duration};

use serde::Deserialize;
use serde_json::Value;

use crate::{
    domain::{
        draw::DrawIssue,
        lottery::{DrawMode, DrawSource},
    },
    error::{ApiError, ApiResult},
};

pub const API68_FC3D_SOURCE_ID: &str = "api68-fc3d";
pub const API68_FC3D_SOURCE_NAME: &str = "API68 福彩 3D";
pub const API68_FC3D_LOTTERY_ID: &str = "fc3d";
pub const API68_FC3D_LOT_CODE: &str = "10041";
pub const API68_QUANGUOCAI_ENDPOINT_ENV: &str = "API68_QUANGUOCAI_ENDPOINT";
const DEFAULT_API68_QUANGUOCAI_ENDPOINT: &str =
    "https://api.api68.com/QuanGuoCai/getLotteryInfoList.do";
const API_DRAW_SOURCE_TIMEOUT_SECONDS: u64 = 10;

#[derive(Clone)]
pub struct ApiDrawSourceRepository {
    client: reqwest::Client,
    sources: Arc<BTreeMap<String, ApiDrawSourceConfig>>,
    static_responses: Arc<BTreeMap<String, String>>,
}

impl ApiDrawSourceRepository {
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn api68_seeded() -> Self {
        Self::new(vec![ApiDrawSourceConfig::api68_fc3d()])
    }

    fn new(sources: Vec<ApiDrawSourceConfig>) -> Self {
        Self {
            client: reqwest::Client::new(),
            sources: Arc::new(
                sources
                    .into_iter()
                    .map(|source| (source.lottery_id.clone(), source))
                    .collect(),
            ),
            static_responses: Arc::new(BTreeMap::new()),
        }
    }

    #[cfg(test)]
    pub fn api68_seeded_with_static_response(response_body: impl Into<String>) -> Self {
        let mut static_responses = BTreeMap::new();
        static_responses.insert(API68_FC3D_SOURCE_ID.to_string(), response_body.into());

        Self {
            client: reqwest::Client::new(),
            sources: Arc::new(BTreeMap::from([(
                API68_FC3D_LOTTERY_ID.to_string(),
                ApiDrawSourceConfig::api68_fc3d(),
            )])),
            static_responses: Arc::new(static_responses),
        }
    }

    pub async fn draw_number_for(&self, issue: &DrawIssue) -> ApiResult<Option<String>> {
        if issue.draw_mode != DrawMode::Api {
            return Ok(None);
        }

        let Some(source) = self.sources.get(&issue.lottery_id) else {
            return Ok(None);
        };

        let result = match source.provider {
            ApiDrawProvider::Api68 => self.fetch_api68_draw_number(source, &issue.issue).await,
        };

        if let Err(error) = &result {
            tracing::error!(
                source_id = %source.id,
                lottery_id = %issue.lottery_id,
                issue = %issue.issue,
                error = %error,
                "api draw source failed"
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
            .get(&source.url)
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
}

#[derive(Debug, Clone)]
struct ApiDrawSourceConfig {
    id: String,
    name: String,
    lottery_id: String,
    url: String,
    provider: ApiDrawProvider,
}

impl ApiDrawSourceConfig {
    fn api68_fc3d() -> Self {
        Self {
            id: API68_FC3D_SOURCE_ID.to_string(),
            name: API68_FC3D_SOURCE_NAME.to_string(),
            lottery_id: API68_FC3D_LOTTERY_ID.to_string(),
            url: api68_url(API68_FC3D_LOT_CODE),
            provider: ApiDrawProvider::Api68,
        }
    }

    fn summary(&self) -> DrawSource {
        DrawSource {
            id: self.id.clone(),
            name: self.name.clone(),
            mode: DrawMode::Api,
            reusable_for_lottery_ids: vec![self.lottery_id.clone()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ApiDrawProvider {
    Api68,
}

pub fn api_draw_source_summaries() -> Vec<DrawSource> {
    vec![ApiDrawSourceConfig::api68_fc3d().summary()]
}

fn api68_url(lot_code: &str) -> String {
    let endpoint = std::env::var(API68_QUANGUOCAI_ENDPOINT_ENV)
        .unwrap_or_else(|_| DEFAULT_API68_QUANGUOCAI_ENDPOINT.to_string());
    format!("{endpoint}?lotCode={lot_code}")
}

pub(crate) fn parse_api68_draw_number(
    response_body: &str,
    expected_issue: &str,
) -> ApiResult<String> {
    let expected_issue = expected_issue.trim();
    if expected_issue.is_empty() {
        return Err(ApiError::BadRequest("issue is required".to_string()));
    }

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
    use super::parse_api68_draw_number;

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
}
