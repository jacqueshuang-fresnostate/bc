//! 开奖源服务层，管理第三方开奖接口和开奖结果解析

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rand_core::{OsRng, RngCore};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgRow, Row};

use crate::{
    domain::{
        draw::{ApiDrawSourceCrawlSnapshotSummary, DrawIssue},
        lottery::{DrawMode, DrawSource, DrawSourceProvider, LotteryKind, SaveDrawSourceRequest},
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{
    enum_from_string, enum_to_string, from_json, to_json, BusinessDatabase,
};
use super::pagination::{ListPage, PageRequest};

/// API68 福彩3D 默认开奖源 ID。
pub const API68_FC3D_SOURCE_ID: &str = "api68-fc3d";
/// API68 福彩3D 默认开奖源名称。
pub const API68_FC3D_SOURCE_NAME: &str = "API68 福彩 3D";
/// API68 福彩3D 对应的本地彩种 ID。
pub const API68_FC3D_LOTTERY_ID: &str = "fc3d";
/// API68 体彩排列3 默认开奖源 ID。
pub const API68_PL3_SOURCE_ID: &str = "api68-pl3";
/// API68 体彩排列3 默认开奖源名称。
pub const API68_PL3_SOURCE_NAME: &str = "API68 体彩排列3";
/// API68 体彩排列3 对应的本地彩种 ID。
pub const API68_PL3_LOTTERY_ID: &str = "pl3";
/// API68 福彩3D 第三方彩种编码。
pub const API68_FC3D_LOT_CODE: &str = "10041";
/// API68 体彩排列3 第三方彩种编码。
pub const API68_PL3_LOT_CODE: &str = "10043";
/// API68 体彩排列5 默认开奖源 ID。
pub const API68_PL5_SOURCE_ID: &str = "api68-pl5";
/// API68 体彩排列5 默认开奖源名称。
pub const API68_PL5_SOURCE_NAME: &str = "API68 体彩排列5";
/// API68 体彩排列5 对应的本地彩种 ID。
pub const API68_PL5_LOTTERY_ID: &str = "pl5";
/// API68 体彩排列5 第三方彩种编码。
pub const API68_PL5_LOT_CODE: &str = "10044";
/// API68 澳洲幸运5 默认开奖源 ID。
pub const API68_AU5_SOURCE_ID: &str = "api68-au5";
/// API68 澳洲幸运5 默认开奖源名称。
pub const API68_AU5_SOURCE_NAME: &str = "API68 澳洲幸运5";
/// API68 澳洲幸运5 对应的本地彩种 ID。
pub const API68_AU5_LOTTERY_ID: &str = "au5";
/// API68 澳洲幸运5 第三方彩种编码。
pub const API68_AU5_LOT_CODE: &str = "10010";
/// KJAPI 腾讯分分彩默认开奖源 ID。
pub const KJ_TXFFC_SOURCE_ID: &str = "kj-txffc";
/// KJAPI 腾讯分分彩默认开奖源名称。
pub const KJ_TXFFC_SOURCE_NAME: &str = "KJAPI 腾讯分分彩";
/// KJAPI 腾讯分分彩对应的本地彩种 ID。
pub const KJ_TXFFC_LOTTERY_ID: &str = "txffc";
/// KJAPI 腾讯分分彩第三方彩种键。
pub const KJ_TXFFC_LOT_KEY: &str = "txffc";
/// BB 开奖河内5分彩默认开奖源 ID。
pub const BB_HN5_SOURCE_ID: &str = "bb-hn5";
/// BB 开奖河内5分彩默认开奖源名称。
pub const BB_HN5_SOURCE_NAME: &str = "BB开奖 河内5分彩";
/// BB 开奖河内5分彩对应的本地彩种 ID。
pub const BB_HN5_LOTTERY_ID: &str = "hn5";
/// BB 开奖河内5分彩第三方游戏编码。
pub const BB_HN5_GAME_CODE: &str = "VIFFC5";
/// 印尼开奖印尼5分彩默认开奖源 ID。
pub const IDN5_SOURCE_ID: &str = "indonesia-id5";
/// 印尼开奖印尼5分彩默认开奖源名称。
pub const IDN5_SOURCE_NAME: &str = "印尼开奖 印尼5分彩";
/// 印尼开奖印尼5分彩对应的本地彩种 ID。
pub const IDN5_LOTTERY_ID: &str = "id5";
/// 印尼开奖印尼5分彩本地归一化彩种编码。
pub const IDN5_LOT_CODE: &str = "IDFFC5";
const DEFAULT_API68_QUANGUOCAI_ENDPOINT: &str =
    "https://api.api68.com/QuanGuoCai/getLotteryInfoList.do";
const DEFAULT_API68_PL3_ENDPOINT: &str = "https://api.api68.com/QuanGuoCai/getLotteryInfo1.do";
const DEFAULT_API68_PL5_ENDPOINT: &str = "https://api.api68.com/QuanGuoCai/getLotteryInfo.do";
const DEFAULT_API68_CQSHICAI_SINGLE_ENDPOINT: &str =
    "https://api.api68.com/CQShiCai/getBaseCQShiCai.do";
const DEFAULT_API68_PKS_ENDPOINT: &str = "https://api.api68.com/pks/getLotteryPksInfo.do";
const DEFAULT_API68_ELEVEN_FIVE_ENDPOINT: &str =
    "https://api.api68.com/ElevenFive/getElevenFiveInfo.do";
const DEFAULT_API68_LUCK_TWENTY_ENDPOINT: &str =
    "https://api.api68.com/LuckTwenty/getBaseLuckTewnty.do";
const DEFAULT_KJ_ENDPOINT: &str = "https://kjapi.net/hall/hallajax/getLotteryInfo";
const DEFAULT_BB_ENDPOINT: &str =
    "https://www.bbkaijiang.com/api/st-lottery-open/open-result/list-newest-result";
const DEFAULT_INDONESIA_ENDPOINT: &str = "https://draw.indonesia-lottery.org/others/draw.php";
const API_DRAW_SOURCE_TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 外部开奖源返回的最新期号和开奖时间摘要。
pub struct ApiDrawSourceLatestIssue {
    /// 彩票期号。
    pub issue: String,
    /// 开奖时间字段。
    pub draw_time: Option<String>,
    /// 外部开奖源提示的下一期期号。
    pub next_issue: Option<String>,
    /// 外部开奖源提示的下一期开奖时间。
    pub next_draw_time: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// API 开奖源采集快照的请求用途。
enum ApiDrawSourceRequestKind {
    LatestIssue,
    DrawNumber,
}

/// API 开奖源采集快照用途的字符串映射。
impl ApiDrawSourceRequestKind {
    /// 返回数据库保存的稳定枚举值。
    fn as_str(self) -> &'static str {
        match self {
            Self::LatestIssue => "latestIssue",
            Self::DrawNumber => "drawNumber",
        }
    }
}

#[derive(Debug, Clone)]
/// 第三方开奖接口的原始响应摘要。
struct ApiDrawSourceHttpResponse {
    status: Option<u16>,
    body: String,
}

/// 第三方开奖接口响应的状态辅助方法。
impl ApiDrawSourceHttpResponse {
    /// 判断当前响应是否可以进入业务解析；静态测试响应没有 HTTP 状态，视为成功。
    fn is_success(&self) -> bool {
        self.status
            .map(|status| (200..300).contains(&status))
            .unwrap_or(true)
    }
}

#[derive(Debug, Clone)]
/// API 开奖源采集快照，保存请求、解析结果和原始响应，便于后续对比外部数据。
struct ApiDrawSourceCrawlSnapshot {
    id: String,
    source_id: String,
    source_name: String,
    provider: String,
    lottery_id: String,
    request_kind: String,
    requested_issue: Option<String>,
    latest_issue: Option<String>,
    latest_draw_time: Option<String>,
    next_issue: Option<String>,
    next_draw_time: Option<String>,
    draw_number: Option<String>,
    endpoint: String,
    lot_code: String,
    http_status: Option<i32>,
    success: bool,
    error_message: Option<String>,
    raw_response: Option<Value>,
    raw_response_text: String,
}

#[derive(Clone, Copy, Debug, Default)]
/// API 开奖源采集快照列表查询条件，负责把后台筛选下推到数据库。
pub struct ApiDrawSourceCrawlSnapshotQuery<'a> {
    /// 彩种 ID 筛选。
    pub lottery_id: Option<&'a str>,
    /// 开奖源 ID 筛选。
    pub source_id: Option<&'a str>,
    /// 采集用途筛选。
    pub request_kind: Option<&'a str>,
    /// 成功状态筛选。
    pub success: Option<bool>,
    /// 期号筛选，会匹配请求期号、最新期号和下一期期号。
    pub issue: Option<&'a str>,
    /// 分页参数。
    pub page: PageRequest,
}

#[derive(Clone)]
/// API 开奖源仓储，管理外部接口配置和采集解析。
pub struct ApiDrawSourceRepository {
    client: reqwest::Client,
    inner: Arc<RwLock<ApiDrawSourceStore>>,
    static_responses: Arc<BTreeMap<String, String>>,
    persistence: Option<BusinessDatabase>,
}

/// API 开奖源仓储的方法实现。
impl ApiDrawSourceRepository {
    #[allow(dead_code)]
    /// 创建一个空的开奖源仓储实例（用于测试场景）。
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// 创建预置 API68 和 KJ API 的开奖源仓储实例。
    pub fn api68_seeded() -> Self {
        Self::new(default_api_draw_sources())
    }

    /// 初始化内部状态容器。
    fn new(sources: Vec<ApiDrawSourceConfig>) -> Self {
        Self {
            client: reqwest::Client::new(),
            inner: Arc::new(RwLock::new(ApiDrawSourceStore::new(sources))),
            static_responses: Arc::new(BTreeMap::new()),
            persistence: None,
        }
    }

    /// 从数据库恢复开奖源并构建持久化仓储。
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
    /// 使用静态响应内容创建 API68 预置源，便于测试。
    pub fn api68_seeded_with_static_response(response_body: impl Into<String>) -> Self {
        let mut static_responses = BTreeMap::new();
        let response_body = response_body.into();
        static_responses.insert(API68_FC3D_SOURCE_ID.to_string(), response_body.clone());
        static_responses.insert(API68_PL3_SOURCE_ID.to_string(), response_body.clone());
        static_responses.insert(API68_PL5_SOURCE_ID.to_string(), response_body.clone());
        static_responses.insert(API68_AU5_SOURCE_ID.to_string(), response_body);

        Self {
            client: reqwest::Client::new(),
            inner: Arc::new(RwLock::new(ApiDrawSourceStore::new(
                default_api_draw_sources(),
            ))),
            static_responses: Arc::new(static_responses),
            persistence: None,
        }
    }

    #[cfg(test)]
    /// 使用静态响应内容创建 KJAPI 预置源，便于测试。
    pub fn kj_seeded_with_static_response(response_body: impl Into<String>) -> Self {
        let mut static_responses = BTreeMap::new();
        static_responses.insert(KJ_TXFFC_SOURCE_ID.to_string(), response_body.into());

        Self {
            client: reqwest::Client::new(),
            inner: Arc::new(RwLock::new(ApiDrawSourceStore::new(
                default_api_draw_sources(),
            ))),
            static_responses: Arc::new(static_responses),
            persistence: None,
        }
    }

    #[cfg(test)]
    /// 使用静态响应内容创建 BB 开奖预置源，便于测试河内 5 分彩解析。
    pub fn bb_seeded_with_static_response(response_body: impl Into<String>) -> Self {
        let mut static_responses = BTreeMap::new();
        static_responses.insert(BB_HN5_SOURCE_ID.to_string(), response_body.into());

        Self {
            client: reqwest::Client::new(),
            inner: Arc::new(RwLock::new(ApiDrawSourceStore::new(
                default_api_draw_sources(),
            ))),
            static_responses: Arc::new(static_responses),
            persistence: None,
        }
    }

    #[cfg(test)]
    /// 使用静态响应内容创建印尼开奖预置源，便于测试印尼 5 分彩解析。
    pub fn indonesia_seeded_with_static_response(response_body: impl Into<String>) -> Self {
        let mut static_responses = BTreeMap::new();
        static_responses.insert(IDN5_SOURCE_ID.to_string(), response_body.into());

        Self {
            client: reqwest::Client::new(),
            inner: Arc::new(RwLock::new(ApiDrawSourceStore::new(
                default_api_draw_sources(),
            ))),
            static_responses: Arc::new(static_responses),
            persistence: None,
        }
    }

    /// 按当前仓储快照返回全部开奖源列表。
    pub async fn list(&self) -> ApiResult<Vec<DrawSource>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("draw source store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 校验入参并创建一条新记录。
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

    /// 更新现有记录并持久化变更。
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

    /// 删除现有记录并返回被删对象。
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

    /// 按期号所属彩种从对应开奖源抓取开奖号码；非 API 模式返回空。
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
            DrawSourceProvider::Api68 => {
                self.fetch_api68_draw_number(&source, &issue.lottery_id, &issue.issue)
                    .await
            }
            DrawSourceProvider::KjApi => {
                self.fetch_kj_draw_number(&source, &issue.lottery_id, &issue.issue)
                    .await
            }
            DrawSourceProvider::BbKaijiang => {
                self.fetch_bb_draw_number(&source, &issue.lottery_id, &issue.issue)
                    .await
            }
            DrawSourceProvider::IndonesiaLottery => {
                self.fetch_indonesia_draw_number(&source, &issue.lottery_id, &issue.issue)
                    .await
            }
        };

        if let Err(error) = &result {
            if is_pending_api_draw_number(error) {
                tracing::warn!(
                    source_id = %source.id,
                    lottery_id = %issue.lottery_id,
                    issue = %issue.issue,
                    error = %error.log_message(),
                    "API 开奖源暂未返回开奖号码"
                );
            } else {
                tracing::error!(
                    source_id = %source.id,
                    lottery_id = %issue.lottery_id,
                    issue = %issue.issue,
                    error = %error.log_message(),
                    "API 开奖源获取开奖号码失败"
                );
            }
        }

        result.map(Some)
    }

    /// 获取指定彩种的开奖源最新期号信息。
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
            DrawSourceProvider::Api68 => self.fetch_api68_latest_issue(&source, lottery_id).await,
            DrawSourceProvider::KjApi => self.fetch_kj_latest_issue(&source, lottery_id).await,
            DrawSourceProvider::BbKaijiang => self.fetch_bb_latest_issue(&source, lottery_id).await,
            DrawSourceProvider::IndonesiaLottery => {
                self.fetch_indonesia_latest_issue(&source, lottery_id).await
            }
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
    /// 请求 API68 并解析指定期号开奖号码。
    async fn fetch_api68_draw_number(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        issue: &str,
    ) -> ApiResult<String> {
        self.fetch_draw_number_with_snapshot(source, lottery_id, issue, parse_api68_draw_number)
            .await
    }
    /// 请求 API68 并解析最新期号信息。
    async fn fetch_api68_latest_issue(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
    ) -> ApiResult<ApiDrawSourceLatestIssue> {
        self.fetch_latest_issue_with_snapshot(source, lottery_id, parse_api68_latest_issue)
            .await
    }
    /// 请求 KJAPI 并解析指定期号开奖号码。
    async fn fetch_kj_draw_number(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        issue: &str,
    ) -> ApiResult<String> {
        self.fetch_draw_number_with_snapshot(source, lottery_id, issue, parse_kj_draw_number)
            .await
    }
    /// 请求 KJAPI 并解析最新期号信息。
    async fn fetch_kj_latest_issue(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
    ) -> ApiResult<ApiDrawSourceLatestIssue> {
        self.fetch_latest_issue_with_snapshot(source, lottery_id, parse_kj_latest_issue)
            .await
    }
    /// 请求 BB 开奖并解析指定期号开奖号码。
    async fn fetch_bb_draw_number(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        issue: &str,
    ) -> ApiResult<String> {
        self.fetch_draw_number_with_snapshot(source, lottery_id, issue, parse_bb_draw_number)
            .await
    }
    /// 请求 BB 开奖并解析最新期号信息。
    async fn fetch_bb_latest_issue(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
    ) -> ApiResult<ApiDrawSourceLatestIssue> {
        self.fetch_latest_issue_with_snapshot(source, lottery_id, parse_bb_latest_issue)
            .await
    }
    /// 请求印尼开奖并解析指定期号开奖号码。
    async fn fetch_indonesia_draw_number(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        issue: &str,
    ) -> ApiResult<String> {
        self.fetch_draw_number_with_snapshot(source, lottery_id, issue, parse_indonesia_draw_number)
            .await
    }
    /// 请求印尼开奖并解析最新期号信息。
    async fn fetch_indonesia_latest_issue(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
    ) -> ApiResult<ApiDrawSourceLatestIssue> {
        self.fetch_latest_issue_with_snapshot(source, lottery_id, parse_indonesia_latest_issue)
            .await
    }
    /// 请求第三方开奖号码并保存采集快照。
    async fn fetch_draw_number_with_snapshot(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        issue: &str,
        parser: fn(&str, &str) -> ApiResult<String>,
    ) -> ApiResult<String> {
        let response = match self.fetch_source_response(source).await {
            Ok(response) => response,
            Err(error) => {
                let result: ApiResult<String> = Err(error);
                self.record_draw_number_snapshot(source, lottery_id, issue, None, &result)
                    .await?;
                return result;
            }
        };
        let result = if response.is_success() {
            parser(&response.body, issue)
        } else {
            Err(ApiError::Internal(format!(
                "API 开奖源返回 HTTP 状态 {}",
                response.status.unwrap_or_default()
            )))
        };
        self.record_draw_number_snapshot(source, lottery_id, issue, Some(&response), &result)
            .await?;
        result
    }
    /// 请求第三方最新期号并保存采集快照。
    async fn fetch_latest_issue_with_snapshot(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        parser: fn(&str) -> ApiResult<ApiDrawSourceLatestIssue>,
    ) -> ApiResult<ApiDrawSourceLatestIssue> {
        let response = match self.fetch_source_response(source).await {
            Ok(response) => response,
            Err(error) => {
                let result: ApiResult<ApiDrawSourceLatestIssue> = Err(error);
                self.record_latest_issue_snapshot(source, lottery_id, None, &result)
                    .await?;
                return result;
            }
        };
        let result = if response.is_success() {
            parser(&response.body)
        } else {
            Err(ApiError::Internal(format!(
                "API 开奖源返回 HTTP 状态 {}",
                response.status.unwrap_or_default()
            )))
        };
        self.record_latest_issue_snapshot(source, lottery_id, Some(&response), &result)
            .await?;
        result
    }
    /// 按开奖源配置发起 HTTP 请求并返回原始响应。
    async fn fetch_source_response(
        &self,
        source: &ApiDrawSourceConfig,
    ) -> ApiResult<ApiDrawSourceHttpResponse> {
        if let Some(response_body) = self.static_responses.get(&source.id) {
            return Ok(ApiDrawSourceHttpResponse {
                status: None,
                body: response_body.clone(),
            });
        }

        let response = self
            .client
            .get(source.url())
            .timeout(Duration::from_secs(API_DRAW_SOURCE_TIMEOUT_SECONDS))
            .send()
            .await
            .map_err(|error| ApiError::Internal(format!("API 开奖源请求失败：{error}")))?;

        let status = response.status().as_u16();
        let response_body = response
            .text()
            .await
            .map_err(|error| ApiError::Internal(format!("API 开奖源响应读取失败：{error}")))?;

        Ok(ApiDrawSourceHttpResponse {
            status: Some(status),
            body: response_body,
        })
    }
    /// 记录指定期号开奖号码采集快照。
    async fn record_draw_number_snapshot(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        issue: &str,
        response: Option<&ApiDrawSourceHttpResponse>,
        result: &ApiResult<String>,
    ) -> ApiResult<()> {
        let snapshot = self.crawl_snapshot_base(
            source,
            lottery_id,
            ApiDrawSourceRequestKind::DrawNumber,
            response,
            result.as_ref().err(),
        )?;
        let snapshot = ApiDrawSourceCrawlSnapshot {
            requested_issue: Some(issue.to_string()),
            draw_number: result.as_ref().ok().cloned(),
            ..snapshot
        };
        self.persist_crawl_snapshot(snapshot).await
    }
    /// 记录最新期号采集快照。
    async fn record_latest_issue_snapshot(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        response: Option<&ApiDrawSourceHttpResponse>,
        result: &ApiResult<ApiDrawSourceLatestIssue>,
    ) -> ApiResult<()> {
        let latest = result.as_ref().ok();
        let snapshot = self.crawl_snapshot_base(
            source,
            lottery_id,
            ApiDrawSourceRequestKind::LatestIssue,
            response,
            result.as_ref().err(),
        )?;
        let snapshot = ApiDrawSourceCrawlSnapshot {
            latest_issue: latest.map(|issue| issue.issue.clone()),
            latest_draw_time: latest.and_then(|issue| issue.draw_time.clone()),
            next_issue: latest.and_then(|issue| issue.next_issue.clone()),
            next_draw_time: latest.and_then(|issue| issue.next_draw_time.clone()),
            ..snapshot
        };
        self.persist_crawl_snapshot(snapshot).await
    }
    /// 构造 API 开奖源采集快照的公共字段。
    fn crawl_snapshot_base(
        &self,
        source: &ApiDrawSourceConfig,
        lottery_id: &str,
        request_kind: ApiDrawSourceRequestKind,
        response: Option<&ApiDrawSourceHttpResponse>,
        error: Option<&ApiError>,
    ) -> ApiResult<ApiDrawSourceCrawlSnapshot> {
        let raw_response_text = response
            .map(|response| response.body.clone())
            .unwrap_or_default();
        let raw_response = serde_json::from_str::<Value>(&raw_response_text).ok();
        Ok(ApiDrawSourceCrawlSnapshot {
            id: next_crawl_snapshot_id(),
            source_id: source.id.clone(),
            source_name: source.name.clone(),
            provider: enum_to_string(&source.provider)?,
            lottery_id: lottery_id.to_string(),
            request_kind: request_kind.as_str().to_string(),
            requested_issue: None,
            latest_issue: None,
            latest_draw_time: None,
            next_issue: None,
            next_draw_time: None,
            draw_number: None,
            endpoint: source.url(),
            lot_code: source.lot_code.clone(),
            http_status: response.and_then(|response| response.status.map(i32::from)),
            success: error.is_none(),
            error_message: error.map(ApiError::log_message),
            raw_response,
            raw_response_text,
        })
    }
    /// 保存 API 开奖源采集快照。
    async fn persist_crawl_snapshot(&self, snapshot: ApiDrawSourceCrawlSnapshot) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_api_draw_source_crawl_snapshot(persistence, &snapshot).await?;
        }

        Ok(())
    }

    /// 分页读取 API 开奖源采集快照；内存模式不落库，因此返回空列表。
    pub async fn list_crawl_snapshots(
        &self,
        query: ApiDrawSourceCrawlSnapshotQuery<'_>,
    ) -> ApiResult<ListPage<ApiDrawSourceCrawlSnapshotSummary>> {
        let Some(persistence) = &self.persistence else {
            return Ok(ListPage::from_all(Vec::new(), query.page));
        };

        query_api_draw_source_crawl_snapshot_page(persistence, query).await
    }

    /// 清除数据库中全部 API 开奖源采集快照；内存模式没有快照表，直接返回 0。
    pub async fn clear_crawl_snapshots(&self) -> ApiResult<usize> {
        let Some(persistence) = &self.persistence else {
            return Ok(0);
        };

        clear_api_draw_source_crawl_snapshots(persistence).await
    }

    /// 从数据库重新加载 API 开奖源配置，供后台缓存维护和手动改库后校准使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_draw_source_store(persistence).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("API 开奖源缓存刷新失败".to_string()))? = store;
        Ok(true)
    }
    /// 把当前仓储快照同步保存到持久化存储。
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
/// 从数据库加载开奖源配置仓储。
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

    let mut store = ApiDrawSourceStore::new(sources);
    if fill_missing_default_sources(&mut store) {
        save_draw_source_store(database, &store).await?;
    }

    Ok(store)
}

/// 补齐系统内置开奖源；已存在同 ID 或已绑定对应彩种的来源不会被覆盖。
fn fill_missing_default_sources(store: &mut ApiDrawSourceStore) -> bool {
    let mut changed = false;
    for source in default_api_draw_sources() {
        let source_exists = store.sources.contains_key(&source.id);
        let lottery_already_bound = source
            .reusable_for_lottery_ids
            .iter()
            .any(|lottery_id| store.find_for_lottery(lottery_id).is_some());
        if source_exists || lottery_already_bound {
            continue;
        }

        store.sources.insert(source.id.clone(), source);
        changed = true;
    }

    changed
}
/// 保存开奖源配置仓储快照。
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
/// 把单次 API 开奖源抓取快照写入数据库。
async fn save_api_draw_source_crawl_snapshot(
    database: &BusinessDatabase,
    snapshot: &ApiDrawSourceCrawlSnapshot,
) -> ApiResult<()> {
    sqlx::query(
        "INSERT INTO api_draw_source_snapshots
         (id, source_id, source_name, provider, lottery_id, request_kind, requested_issue,
          latest_issue, latest_draw_time, next_issue, next_draw_time, draw_number,
          endpoint, lot_code, http_status, success, error_message, raw_response,
          raw_response_text)
         VALUES
         ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
          $16, $17, $18, $19)",
    )
    .bind(&snapshot.id)
    .bind(&snapshot.source_id)
    .bind(&snapshot.source_name)
    .bind(&snapshot.provider)
    .bind(&snapshot.lottery_id)
    .bind(&snapshot.request_kind)
    .bind(&snapshot.requested_issue)
    .bind(&snapshot.latest_issue)
    .bind(&snapshot.latest_draw_time)
    .bind(&snapshot.next_issue)
    .bind(&snapshot.next_draw_time)
    .bind(&snapshot.draw_number)
    .bind(&snapshot.endpoint)
    .bind(&snapshot.lot_code)
    .bind(snapshot.http_status)
    .bind(snapshot.success)
    .bind(&snapshot.error_message)
    .bind(&snapshot.raw_response)
    .bind(&snapshot.raw_response_text)
    .execute(database.pool())
    .await
    .map_err(|error| {
        tracing::error!(error = %error, "API 开奖源采集快照保存失败");
        ApiError::Internal(format!("API 开奖源采集快照保存失败：{error}"))
    })?;

    Ok(())
}

/// 数据库模式下分页查询 API 开奖源采集快照，供后台按彩种、来源、用途和期号比对。
async fn query_api_draw_source_crawl_snapshot_page(
    database: &BusinessDatabase,
    query: ApiDrawSourceCrawlSnapshotQuery<'_>,
) -> ApiResult<ListPage<ApiDrawSourceCrawlSnapshotSummary>> {
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM api_draw_source_snapshots
         WHERE ($1::text IS NULL OR lottery_id = $1)
           AND ($2::text IS NULL OR source_id = $2)
           AND ($3::text IS NULL OR request_kind = $3)
           AND ($4::boolean IS NULL OR success = $4)
           AND (
               $5::text IS NULL
               OR requested_issue = $5
               OR latest_issue = $5
               OR next_issue = $5
           )",
    )
    .bind(query.lottery_id)
    .bind(query.source_id)
    .bind(query.request_kind)
    .bind(query.success)
    .bind(query.issue)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("API 开奖源采集快照总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("API 开奖源采集快照总数无效".to_string()))?;
    let resolved = query.page.resolve(total_count);
    let rows = sqlx::query(
        "SELECT id, source_id, source_name, provider, lottery_id, request_kind,
                requested_issue, latest_issue, latest_draw_time, next_issue, next_draw_time,
                draw_number, endpoint, lot_code, http_status, success, error_message,
                raw_response, raw_response_text,
                to_char(crawled_at AT TIME ZONE 'Asia/Shanghai', 'YYYY-MM-DD HH24:MI:SS')
                    AS crawled_at
         FROM api_draw_source_snapshots
         WHERE ($1::text IS NULL OR lottery_id = $1)
           AND ($2::text IS NULL OR source_id = $2)
           AND ($3::text IS NULL OR request_kind = $3)
           AND ($4::boolean IS NULL OR success = $4)
           AND (
               $5::text IS NULL
               OR requested_issue = $5
               OR latest_issue = $5
               OR next_issue = $5
           )
         ORDER BY api_draw_source_snapshots.crawled_at DESC, id DESC
         LIMIT $6 OFFSET $7",
    )
    .bind(query.lottery_id)
    .bind(query.source_id)
    .bind(query.request_kind)
    .bind(query.success)
    .bind(query.issue)
    .bind(resolved.limit_i64()?)
    .bind(resolved.offset_i64()?)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(api_draw_source_crawl_snapshot_from_row)
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 将数据库行转换成后台采集快照摘要，统一处理 JSON 和时间字段解码。
fn api_draw_source_crawl_snapshot_from_row(
    row: PgRow,
) -> ApiResult<ApiDrawSourceCrawlSnapshotSummary> {
    Ok(ApiDrawSourceCrawlSnapshotSummary {
        id: row
            .try_get("id")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        source_id: row
            .try_get("source_id")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        source_name: row
            .try_get("source_name")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        provider: row
            .try_get("provider")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        lottery_id: row
            .try_get("lottery_id")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        request_kind: row
            .try_get("request_kind")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        requested_issue: row
            .try_get("requested_issue")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        latest_issue: row
            .try_get("latest_issue")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        latest_draw_time: row
            .try_get("latest_draw_time")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        next_issue: row
            .try_get("next_issue")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        next_draw_time: row
            .try_get("next_draw_time")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        draw_number: row
            .try_get("draw_number")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        endpoint: row
            .try_get("endpoint")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        lot_code: row
            .try_get("lot_code")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        http_status: row
            .try_get("http_status")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        success: row
            .try_get("success")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        error_message: row
            .try_get("error_message")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        raw_response: row
            .try_get("raw_response")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        raw_response_text: row
            .try_get("raw_response_text")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
        crawled_at: row
            .try_get("crawled_at")
            .map_err(|_| ApiError::Internal("API 开奖源采集快照数据读取失败".to_string()))?,
    })
}

/// 清空 API 开奖源采集快照表，只删除审计数据，不影响开奖源配置和开奖期号。
async fn clear_api_draw_source_crawl_snapshots(database: &BusinessDatabase) -> ApiResult<usize> {
    let result = sqlx::query("DELETE FROM api_draw_source_snapshots")
        .execute(database.pool())
        .await
        .map_err(|error| {
            tracing::error!(error = %error, "API 开奖源采集快照清理失败");
            ApiError::Internal(format!("API 开奖源采集快照清理失败：{error}"))
        })?;

    usize::try_from(result.rows_affected())
        .map_err(|_| ApiError::Internal("API 开奖源采集快照清理数量无效".to_string()))
}

impl ApiDrawSourceStore {
    /// 初始化内部状态容器。
    fn new(sources: Vec<ApiDrawSourceConfig>) -> Self {
        Self {
            sources: sources
                .into_iter()
                .map(|source| (source.id.clone(), source))
                .collect(),
        }
    }

    /// 按当前仓储快照返回全部开奖源列表。
    fn list(&self) -> Vec<DrawSource> {
        self.sources
            .values()
            .map(ApiDrawSourceConfig::summary)
            .collect()
    }

    /// 按彩种查找绑定配置。
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

    /// 校验入参并创建新记录。
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

    /// 校验入参并更新指定记录。
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

    /// 删除指定记录并返回删除结果。
    fn delete(&mut self, id: &str) -> ApiResult<DrawSource> {
        self.sources
            .remove(id)
            .map(|source| source.summary())
            .ok_or_else(|| ApiError::NotFound(format!("draw source `{id}` not found")))
    }

    /// 把后台保存请求转换为内部开奖源配置。
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
        if !lot_code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(ApiError::BadRequest(
                "lot code can only contain letters, numbers, hyphen and underscore".to_string(),
            ));
        }

        let reusable_for_lottery_ids = reusable_lottery_ids(payload.reusable_for_lottery_ids)?;
        validate_reusable_lotteries(&reusable_for_lottery_ids, lotteries)?;

        Ok(ApiDrawSourceConfig {
            endpoint: normalized_endpoint(payload.endpoint.as_deref(), &payload.provider),
            id,
            lot_code,
            name,
            provider: payload.provider,
            reusable_for_lottery_ids,
        })
    }

    /// 校验开奖源绑定的彩种是否存在且可复用。
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
    /// 返回 API68 福彩3D/排列3 默认开奖源配置。
    fn api68_fc3d() -> Self {
        Self {
            id: API68_FC3D_SOURCE_ID.to_string(),
            name: API68_FC3D_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::Api68,
            lot_code: API68_FC3D_LOT_CODE.to_string(),
            endpoint: default_api68_quanguocai_endpoint(),
            reusable_for_lottery_ids: vec![API68_FC3D_LOTTERY_ID.to_string()],
        }
    }

    /// 构造 API68 体彩排列 3 默认开奖源。
    fn api68_pl3() -> Self {
        Self {
            id: API68_PL3_SOURCE_ID.to_string(),
            name: API68_PL3_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::Api68,
            lot_code: API68_PL3_LOT_CODE.to_string(),
            endpoint: DEFAULT_API68_PL3_ENDPOINT.to_string(),
            reusable_for_lottery_ids: vec![API68_PL3_LOTTERY_ID.to_string()],
        }
    }

    /// 构造 API68 体彩排列 5 默认开奖源。
    fn api68_pl5() -> Self {
        Self {
            id: API68_PL5_SOURCE_ID.to_string(),
            name: API68_PL5_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::Api68,
            lot_code: API68_PL5_LOT_CODE.to_string(),
            endpoint: DEFAULT_API68_PL5_ENDPOINT.to_string(),
            reusable_for_lottery_ids: vec![API68_PL5_LOTTERY_ID.to_string()],
        }
    }

    /// 返回 API68 澳洲幸运5 默认开奖源配置。
    fn api68_au5() -> Self {
        Self {
            id: API68_AU5_SOURCE_ID.to_string(),
            name: API68_AU5_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::Api68,
            lot_code: API68_AU5_LOT_CODE.to_string(),
            endpoint: default_api68_cqshicai_single_endpoint(),
            reusable_for_lottery_ids: vec![API68_AU5_LOTTERY_ID.to_string()],
        }
    }

    /// 构造 API68 单彩种默认开奖源。
    fn api68_source(lottery_id: &str, lottery_name: &str, lot_code: &str, endpoint: &str) -> Self {
        Self {
            id: format!("api68-{lottery_id}"),
            name: format!("API68 {lottery_name}"),
            provider: DrawSourceProvider::Api68,
            lot_code: lot_code.to_string(),
            endpoint: endpoint.to_string(),
            reusable_for_lottery_ids: vec![lottery_id.to_string()],
        }
    }

    /// 返回 KJAPI 腾讯分分彩默认开奖源配置。
    fn kj_txffc() -> Self {
        Self {
            id: KJ_TXFFC_SOURCE_ID.to_string(),
            name: KJ_TXFFC_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::KjApi,
            lot_code: KJ_TXFFC_LOT_KEY.to_string(),
            endpoint: default_kj_endpoint(),
            reusable_for_lottery_ids: vec![KJ_TXFFC_LOTTERY_ID.to_string()],
        }
    }

    /// 构造 BB 开奖河内 5 分彩默认开奖源。
    fn bb_hn5() -> Self {
        Self {
            id: BB_HN5_SOURCE_ID.to_string(),
            name: BB_HN5_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::BbKaijiang,
            lot_code: BB_HN5_GAME_CODE.to_string(),
            endpoint: default_bb_endpoint(),
            reusable_for_lottery_ids: vec![BB_HN5_LOTTERY_ID.to_string()],
        }
    }

    /// 构造印尼开奖印尼 5 分彩默认开奖源。
    fn indonesia_id5() -> Self {
        Self {
            id: IDN5_SOURCE_ID.to_string(),
            name: IDN5_SOURCE_NAME.to_string(),
            provider: DrawSourceProvider::IndonesiaLottery,
            lot_code: IDN5_LOT_CODE.to_string(),
            endpoint: default_indonesia_endpoint(),
            reusable_for_lottery_ids: vec![IDN5_LOTTERY_ID.to_string()],
        }
    }

    /// 拼接当前开奖源的完整请求地址。
    fn url(&self) -> String {
        match self.provider {
            DrawSourceProvider::Api68 => source_url(&self.endpoint, "lotCode", &self.lot_code),
            DrawSourceProvider::KjApi => source_url(&self.endpoint, "lotKey", &self.lot_code),
            DrawSourceProvider::BbKaijiang => {
                source_url(&self.endpoint, "gameCodeList", &self.lot_code)
            }
            DrawSourceProvider::IndonesiaLottery => self.endpoint.clone(),
        }
    }

    /// 转换为接口可返回的摘要结构。
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

/// 返回系统内置开奖源配置快照。
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

/// 读取 API68 全国彩默认接口地址。
fn default_api68_quanguocai_endpoint() -> String {
    DEFAULT_API68_QUANGUOCAI_ENDPOINT.to_string()
}

/// 读取 API68 时时彩单彩种默认接口地址。
fn default_api68_cqshicai_single_endpoint() -> String {
    DEFAULT_API68_CQSHICAI_SINGLE_ENDPOINT.to_string()
}

/// 读取 KJAPI 默认接口地址。
fn default_kj_endpoint() -> String {
    DEFAULT_KJ_ENDPOINT.to_string()
}

/// 读取 BB 开奖默认接口地址。
fn default_bb_endpoint() -> String {
    DEFAULT_BB_ENDPOINT.to_string()
}

/// 读取印尼开奖默认接口地址。
fn default_indonesia_endpoint() -> String {
    DEFAULT_INDONESIA_ENDPOINT.to_string()
}

/// 生成 API 开奖源采集快照编号，避免高并发采集时主键冲突。
fn next_crawl_snapshot_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let mut bytes = [0u8; 4];
    OsRng.fill_bytes(&mut bytes);
    let suffix = u32::from_be_bytes(bytes);

    format!("ADS-{millis}-{suffix:08X}")
}

/// 清洗后台保存的开奖源地址并补齐供应商默认值。
fn normalized_endpoint(endpoint: Option<&str>, provider: &DrawSourceProvider) -> String {
    endpoint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| match provider {
            DrawSourceProvider::Api68 => default_api68_quanguocai_endpoint(),
            DrawSourceProvider::KjApi => default_kj_endpoint(),
            DrawSourceProvider::BbKaijiang => default_bb_endpoint(),
            DrawSourceProvider::IndonesiaLottery => default_indonesia_endpoint(),
        })
}

/// 返回系统内置 API 开奖源配置。
fn default_api_draw_sources() -> Vec<ApiDrawSourceConfig> {
    let mut sources = vec![
        ApiDrawSourceConfig::api68_fc3d(),
        ApiDrawSourceConfig::api68_pl3(),
        ApiDrawSourceConfig::api68_pl5(),
        ApiDrawSourceConfig::api68_au5(),
        ApiDrawSourceConfig::kj_txffc(),
        ApiDrawSourceConfig::bb_hn5(),
        ApiDrawSourceConfig::indonesia_id5(),
    ];
    sources.extend(extra_api68_draw_sources());
    sources
}

/// 返回用户要求新增接入的 API68 开奖源默认配置。
fn extra_api68_draw_sources() -> Vec<ApiDrawSourceConfig> {
    vec![
        ApiDrawSourceConfig::api68_source(
            "bjpk10",
            "北京PK10",
            "10001",
            DEFAULT_API68_PKS_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "tjssc",
            "天津时时彩",
            "10003",
            DEFAULT_API68_CQSHICAI_SINGLE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "xjssc",
            "新疆时时彩",
            "10004",
            DEFAULT_API68_CQSHICAI_SINGLE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "gd11x5",
            "广东11选5",
            "10006",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "au10",
            "澳洲幸运10",
            "10012",
            DEFAULT_API68_PKS_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "au20",
            "澳洲幸运20",
            "10013",
            DEFAULT_API68_LUCK_TWENTY_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "jx11x5",
            "江西11选5",
            "10015",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "js11x5",
            "江苏11选5",
            "10016",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "ah11x5",
            "安徽11选5",
            "10017",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "sh11x5",
            "上海11选5",
            "10018",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "ln11x5",
            "辽宁11选5",
            "10019",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "hb11x5",
            "湖北11选5",
            "10020",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "gx11x5",
            "广西11选5",
            "10022",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "jl11x5",
            "吉林11选5",
            "10023",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "nmg11x5",
            "内蒙古11选5",
            "10024",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
        ApiDrawSourceConfig::api68_source(
            "zj11x5",
            "浙江11选5",
            "10025",
            DEFAULT_API68_ELEVEN_FIVE_ENDPOINT,
        ),
    ]
}

/// 按接口地址、查询参数名和彩种编码拼接请求 URL。
fn source_url(endpoint: &str, query_key: &str, lot_code: &str) -> String {
    if endpoint.contains(&format!("{query_key}=")) {
        return endpoint.to_string();
    }
    let separator = if endpoint.contains('?') { '&' } else { '?' };
    format!("{endpoint}{separator}{query_key}={lot_code}")
}

/// 清洗并去重开奖源可复用彩种 ID。
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

/// 校验复用彩种列表没有重复且彩种存在。
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

/// 解析 API68 开奖响应，按指定期号提取开奖号码。
pub(crate) fn parse_api68_draw_number(
    response_body: &str,
    expected_issue: &str,
) -> ApiResult<String> {
    let expected_issue = expected_issue.trim();
    if expected_issue.is_empty() {
        return Err(ApiError::BadRequest("issue is required".to_string()));
    }

    let result = parse_api68_result(response_body)?;
    let returned_issue = result
        .data
        .first()
        .and_then(|draw| api68_issue_value(&draw.pre_draw_issue));

    let Some(draw) = result.data.into_iter().find(|draw| {
        api68_issue_value(&draw.pre_draw_issue)
            .as_deref()
            .is_some_and(|issue| issue == expected_issue)
    }) else {
        let detail = returned_issue
            .map(|issue| format!("，当前返回期号 `{issue}`"))
            .unwrap_or_default();
        return Err(ApiError::NotFound(format!(
            "API 开奖源未找到期号 `{expected_issue}` 的开奖号码{detail}"
        )));
    };

    let draw_code = draw.pre_draw_code.trim();
    if draw_code.is_empty() {
        return Err(ApiError::Internal("API 开奖源开奖号码为空".to_string()));
    }

    Ok(draw_code.to_string())
}

/// 解析 API68 开奖响应，提取最近已开奖期号和下一期参考信息。
pub(crate) fn parse_api68_latest_issue(response_body: &str) -> ApiResult<ApiDrawSourceLatestIssue> {
    let result = parse_api68_result(response_body)?;
    let Some(draw) = result.data.into_iter().find(|draw| {
        api68_issue_value(&draw.pre_draw_issue).is_some_and(|issue| !issue.trim().is_empty())
    }) else {
        return Err(ApiError::Internal("API 开奖源最新期号为空".to_string()));
    };
    let issue = api68_issue_value(&draw.pre_draw_issue)
        .ok_or_else(|| ApiError::Internal("API 开奖源最新期号为空".to_string()))?;
    let draw_time = draw
        .pre_draw_time
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let next_issue = draw
        .draw_issue
        .as_ref()
        .and_then(api68_issue_value)
        .filter(|value| !value.trim().is_empty());
    let next_draw_time = draw
        .draw_time
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Ok(ApiDrawSourceLatestIssue {
        issue,
        draw_time,
        next_issue,
        next_draw_time,
    })
}

/// 解析 KJAPI 开奖响应，按指定期号提取开奖号码。
pub(crate) fn parse_kj_draw_number(response_body: &str, expected_issue: &str) -> ApiResult<String> {
    let expected_issue = expected_issue.trim();
    if expected_issue.is_empty() {
        return Err(ApiError::BadRequest("issue is required".to_string()));
    }

    let data = parse_kj_data(response_body)?;
    let Some(issue) = api68_issue_value(&data.pre_draw_issue) else {
        return Err(ApiError::Internal("KJAPI 最新期号为空".to_string()));
    };
    if issue != expected_issue {
        return Err(ApiError::NotFound(format!(
            "API 开奖源未找到期号 `{expected_issue}` 的开奖号码，当前返回期号 `{issue}`"
        )));
    }

    let draw_code = data.pre_draw_code.trim();
    if draw_code.is_empty() {
        return Err(ApiError::Internal("API 开奖源开奖号码为空".to_string()));
    }

    Ok(draw_code.to_string())
}

/// 判断第三方开奖源错误是否只是当前期号尚未开放开奖号码。
fn is_pending_api_draw_number(error: &ApiError) -> bool {
    matches!(error, ApiError::NotFound(message) if message.contains("暂未返回开奖号码"))
}

/// 解析 KJAPI 开奖响应，提取最近已开奖期号和下一期参考信息。
pub(crate) fn parse_kj_latest_issue(response_body: &str) -> ApiResult<ApiDrawSourceLatestIssue> {
    let data = parse_kj_data(response_body)?;
    let issue = api68_issue_value(&data.pre_draw_issue)
        .ok_or_else(|| ApiError::Internal("KJAPI 最新期号为空".to_string()))?;
    let draw_time = data
        .pre_draw_time
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let next_issue = data
        .draw_issue
        .as_ref()
        .and_then(api68_issue_value)
        .filter(|value| !value.trim().is_empty());
    let next_draw_time = data
        .draw_time
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Ok(ApiDrawSourceLatestIssue {
        issue,
        draw_time,
        next_issue,
        next_draw_time,
    })
}

/// 解析 BB 开奖响应，按指定河内5分彩期号提取开奖号码。
pub(crate) fn parse_bb_draw_number(response_body: &str, expected_issue: &str) -> ApiResult<String> {
    let expected_issue = expected_issue.trim();
    if expected_issue.is_empty() {
        return Err(ApiError::BadRequest("issue is required".to_string()));
    }

    let envelope = parse_bb_envelope(response_body)?;
    let returned_issue = envelope
        .value
        .first()
        .and_then(|item| item.last.as_ref())
        .map(|draw| draw.numero.trim().to_string())
        .filter(|issue| !issue.is_empty());

    let mut matched_pending_issue = false;
    for item in envelope.value {
        for draw in item.draws() {
            if draw.numero.trim() != expected_issue {
                continue;
            }
            let Some(draw_number) = draw.open_number.as_deref().map(str::trim) else {
                matched_pending_issue = true;
                continue;
            };
            if draw_number.is_empty() {
                matched_pending_issue = true;
                continue;
            }
            return Ok(draw_number.to_string());
        }
    }

    let detail = returned_issue
        .map(|issue| format!("，当前返回期号 `{issue}`"))
        .unwrap_or_default();
    if matched_pending_issue {
        return Err(ApiError::NotFound(format!(
            "API 开奖源期号 `{expected_issue}` 暂未返回开奖号码{detail}"
        )));
    }

    Err(ApiError::NotFound(format!(
        "API 开奖源未找到期号 `{expected_issue}` 的开奖号码{detail}"
    )))
}

/// 解析 BB 开奖响应，提取最近已开奖期号和下一期参考信息。
pub(crate) fn parse_bb_latest_issue(response_body: &str) -> ApiResult<ApiDrawSourceLatestIssue> {
    let envelope = parse_bb_envelope(response_body)?;
    let Some(item) = envelope.value.into_iter().find(|item| {
        item.last
            .as_ref()
            .is_some_and(|draw| !draw.numero.trim().is_empty())
    }) else {
        return Err(ApiError::Internal("BB 开奖源最新期号为空".to_string()));
    };
    let last = item
        .last
        .ok_or_else(|| ApiError::Internal("BB 开奖源最新期号为空".to_string()))?;
    let issue = last.numero.trim().to_string();
    let draw_time = last
        .actual_open_time
        .as_deref()
        .or(last.plan_open_time.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let next_issue = item
        .newest
        .as_ref()
        .or(item.next.as_ref())
        .map(|draw| draw.numero.trim().to_string())
        .filter(|value| !value.is_empty());
    let next_draw_time = item
        .newest
        .as_ref()
        .or(item.next.as_ref())
        .and_then(|draw| draw.plan_open_time.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    Ok(ApiDrawSourceLatestIssue {
        issue,
        draw_time,
        next_issue,
        next_draw_time,
    })
}

/// 解析印尼开奖响应，按归一化期号提取开奖号码。
pub(crate) fn parse_indonesia_draw_number(
    response_body: &str,
    expected_issue: &str,
) -> ApiResult<String> {
    let expected_issue = expected_issue.trim();
    if expected_issue.is_empty() {
        return Err(ApiError::BadRequest("issue is required".to_string()));
    }
    let expected_issue = normalize_indonesia_issue(expected_issue)
        .ok_or_else(|| ApiError::BadRequest("issue is invalid".to_string()))?;
    let data = parse_indonesia_data(response_body)?;

    if normalize_indonesia_issue(&data.latest_origin).as_deref() == Some(expected_issue.as_str()) {
        let latest_num = data.latest_num.trim();
        if !latest_num.is_empty() {
            return Ok(latest_num.to_string());
        }
    }

    for history in data.history {
        if normalize_indonesia_issue(&history.num).as_deref() != Some(expected_issue.as_str()) {
            continue;
        }
        let result = history.result.trim();
        if !result.is_empty() {
            return Ok(result.to_string());
        }
    }

    let detail = normalize_indonesia_issue(&data.latest_origin)
        .map(|issue| format!("，当前返回期号 `{issue}`"))
        .unwrap_or_default();
    Err(ApiError::NotFound(format!(
        "API 开奖源未找到期号 `{expected_issue}` 的开奖号码{detail}"
    )))
}

/// 解析印尼开奖响应，提取最近已开奖期号和下一期参考信息。
pub(crate) fn parse_indonesia_latest_issue(
    response_body: &str,
) -> ApiResult<ApiDrawSourceLatestIssue> {
    let data = parse_indonesia_data(response_body)?;
    let issue = normalize_indonesia_issue(&data.latest_origin)
        .or_else(|| normalize_indonesia_issue(&data.latest))
        .ok_or_else(|| ApiError::Internal("印尼开奖源最新期号为空".to_string()))?;
    let next_issue = normalize_indonesia_issue(&data.next_num);
    let next_draw_time = data
        .next_time
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    Ok(ApiDrawSourceLatestIssue {
        issue,
        draw_time: None,
        next_issue,
        next_draw_time,
    })
}

/// 解析输入并返回结构化值。
fn parse_api68_result(response_body: &str) -> ApiResult<Api68Result> {
    let envelope = serde_json::from_str::<Api68Envelope>(response_body)
        .map_err(|_| ApiError::Internal("API 开奖源响应无法解析".to_string()))?;

    if envelope.error_code != 0 {
        return Err(ApiError::Internal(format!(
            "API 开奖源返回错误码 {}",
            envelope.error_code
        )));
    }

    let result = envelope
        .result
        .ok_or_else(|| ApiError::Internal("API 开奖源 result 为空".to_string()))?;

    if result.business_code != 0 {
        return Err(ApiError::Internal(format!(
            "API 开奖源返回业务码 {}",
            result.business_code
        )));
    }

    Ok(result)
}

/// 解析输入并返回结构化值。
fn parse_kj_data(response_body: &str) -> ApiResult<KjData> {
    let envelope = serde_json::from_str::<KjEnvelope>(response_body)
        .map_err(|_| ApiError::Internal("API 开奖源响应无法解析".to_string()))?;

    if envelope.error_code != 0 {
        return Err(ApiError::Internal(format!(
            "API 开奖源返回错误码 {}",
            envelope.error_code
        )));
    }

    envelope
        .result
        .and_then(|result| result.data)
        .ok_or_else(|| ApiError::Internal("KJAPI result.data 为空".to_string()))
}

/// 解析 BB 开奖响应并校验业务成功状态。
fn parse_bb_envelope(response_body: &str) -> ApiResult<BbEnvelope> {
    let envelope = serde_json::from_str::<BbEnvelope>(response_body)
        .map_err(|_| ApiError::Internal("API 开奖源响应无法解析".to_string()))?;

    if !envelope.success {
        return Err(ApiError::Internal("BB 开奖源返回失败状态".to_string()));
    }

    Ok(envelope)
}

/// 解析印尼开奖响应并返回当前快照。
fn parse_indonesia_data(response_body: &str) -> ApiResult<IndonesiaDrawData> {
    serde_json::from_str::<IndonesiaDrawData>(response_body)
        .map_err(|_| ApiError::Internal("API 开奖源响应无法解析".to_string()))
}

/// 将印尼接口的日期-序号期号转换为系统可递增数字期号。
fn normalize_indonesia_issue(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Some(value.to_string());
    }
    let (date, sequence) = value.split_once('-')?;
    let date = date.trim();
    let sequence = sequence.trim();
    if date.len() != 8
        || !date.bytes().all(|byte| byte.is_ascii_digit())
        || sequence.is_empty()
        || !sequence.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let sequence_number = sequence.parse::<u32>().ok()?;
    Some(format!("{date}{sequence_number:03}"))
}

/// 从 API68 响应中提取期号字段。
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
    #[serde(default, deserialize_with = "deserialize_api68_data")]
    data: Vec<Api68Draw>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Api68Draw {
    pre_draw_issue: Value,
    pre_draw_code: String,
    #[serde(default)]
    pre_draw_time: Option<String>,
    #[serde(default)]
    draw_issue: Option<Value>,
    #[serde(default)]
    draw_time: Option<String>,
}

/// 兼容 API68 不同彩种接口返回数组或单对象的 data 字段。
fn deserialize_api68_data<'de, D>(deserializer: D) -> Result<Vec<Api68Draw>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Array(items) => items
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<Vec<_>, _>>()
            .map_err(de::Error::custom),
        Value::Object(_) => serde_json::from_value(value)
            .map(|draw| vec![draw])
            .map_err(de::Error::custom),
        Value::Null => Ok(Vec::new()),
        _ => Err(de::Error::custom("API68 data must be object or array")),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KjEnvelope {
    error_code: i64,
    #[serde(default)]
    result: Option<KjResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KjResult {
    #[serde(default)]
    data: Option<KjData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KjData {
    pre_draw_issue: Value,
    pre_draw_code: String,
    #[serde(default)]
    pre_draw_time: Option<String>,
    #[serde(default)]
    draw_issue: Option<Value>,
    #[serde(default)]
    draw_time: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BbEnvelope {
    success: bool,
    #[serde(default)]
    value: Vec<BbLotteryResult>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BbLotteryResult {
    #[serde(default)]
    newest: Option<BbDrawSnapshot>,
    #[serde(default)]
    last: Option<BbDrawSnapshot>,
    #[serde(default)]
    next: Option<BbDrawSnapshot>,
}

impl BbLotteryResult {
    /// 返回响应中可能携带开奖号码的快照，供按期号查找时统一遍历。
    fn draws(&self) -> Vec<&BbDrawSnapshot> {
        [&self.last, &self.newest, &self.next]
            .into_iter()
            .filter_map(|draw| draw.as_ref())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BbDrawSnapshot {
    numero: String,
    #[serde(default)]
    plan_open_time: Option<String>,
    #[serde(default)]
    actual_open_time: Option<String>,
    #[serde(default)]
    open_number: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IndonesiaDrawData {
    next_num: String,
    latest_num: String,
    latest_origin: String,
    #[serde(default)]
    history: Vec<IndonesiaHistoryItem>,
    latest: String,
    #[serde(default)]
    next_time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IndonesiaHistoryItem {
    num: String,
    result: String,
}

#[cfg(test)]
mod tests {
    use super::{
        parse_api68_draw_number, parse_api68_latest_issue, parse_bb_draw_number,
        parse_bb_latest_issue, parse_indonesia_draw_number, parse_indonesia_latest_issue,
        parse_kj_draw_number, parse_kj_latest_issue, ApiDrawSourceRepository, API68_AU5_SOURCE_ID,
        API68_FC3D_SOURCE_ID, API68_PL3_SOURCE_ID, API68_PL5_SOURCE_ID, BB_HN5_SOURCE_ID,
        IDN5_SOURCE_ID, KJ_TXFFC_SOURCE_ID,
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
    const API68_OBJECT_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "操作成功",
        "result": {
            "businessCode": 0,
            "message": "操作成功",
            "data": {
                "preDrawIssue": "20260604001",
                "preDrawCode": "01,06,02,04,03,05,07,09,10,08",
                "preDrawTime": "2026-06-04 12:00:00",
                "drawIssue": "20260604002",
                "drawTime": "2026-06-04 12:10:00"
            }
        }
    }"#;
    const KJ_SAMPLE: &str = r#"{
        "errorCode": 0,
        "message": "",
        "result": {
            "businessCode": "202606031178",
            "message": "",
            "data": {
                "lotKey": "txffc",
                "lotName": "腾讯分分彩",
                "preDrawIssue": "202606031178",
                "preDrawCode": "9,9,8,7,2",
                "preDrawTime": "2026-06-03 19:38:01",
                "drawIssue": 202606031179,
                "drawTime": "2026-06-03 19:39:00"
            }
        }
    }"#;
    const BB_SAMPLE: &str = r#"{
        "success": true,
        "value": [
            {
                "newest": {
                    "groupCode": "SSC",
                    "gameCode": "VIFFC5",
                    "numero": "20260617027",
                    "prettyNumero": "20260617-027",
                    "planOpenTime": "2026-06-17 02:15:00",
                    "actualOpenTime": null,
                    "openStatus": "SELLING",
                    "openNumber": null
                },
                "last": {
                    "groupCode": "SSC",
                    "gameCode": "VIFFC5",
                    "numero": "20260617026",
                    "prettyNumero": "20260617-026",
                    "planOpenTime": "2026-06-17 02:10:00",
                    "actualOpenTime": "2026-06-17 02:10:04",
                    "openStatus": "SETTLE",
                    "openNumber": "5,8,2,6,2"
                },
                "next": {
                    "groupCode": "SSC",
                    "gameCode": "VIFFC5",
                    "numero": "20260617028",
                    "prettyNumero": "20260617-028",
                    "planOpenTime": "2026-06-17 02:20:00",
                    "actualOpenTime": null,
                    "openStatus": "NONE",
                    "openNumber": null
                },
                "frequency": 300
            }
        ]
    }"#;
    const INDONESIA_SAMPLE: &str = r#"{
        "next_num": "20260617-43",
        "latest_num": "8,7,3,5,8",
        "latest_origin": "20260617-042",
        "history": [
            { "num": "20260617-42", "result": "8,7,3,5,8" },
            { "num": "20260617-41", "result": "7,4,4,3,1" }
        ],
        "latest": "20260617-42",
        "next_time": "2026-06-17 03:35:00",
        "next_second": 235
    }"#;

    #[test]
    /// 解析输入并返回结构化值。
    fn parse_api68_draw_number_matches_numeric_issue() {
        let draw_number =
            parse_api68_draw_number(API68_SAMPLE, "2026143").expect("draw number can be parsed");

        assert_eq!(draw_number, "3,7,6");
    }

    #[test]
    /// 解析输入并返回结构化值。
    fn parse_api68_draw_number_matches_string_issue() {
        let draw_number =
            parse_api68_draw_number(API68_SAMPLE, "2026142").expect("draw number can be parsed");

        assert_eq!(draw_number, "8,9,4");
    }

    #[test]
    /// 解析输入并返回结构化值。
    fn parse_api68_draw_number_rejects_missing_issue() {
        let error = parse_api68_draw_number(API68_SAMPLE, "2099999")
            .expect_err("missing issue is rejected");

        assert!(error.to_string().contains("未找到"));
    }

    #[test]
    /// 解析输入并返回结构化值。
    fn parse_api68_draw_number_rejects_business_failure() {
        let error = parse_api68_draw_number(
            r#"{"errorCode":0,"result":{"businessCode":1,"data":[]}}"#,
            "2026143",
        )
        .expect_err("business failure is rejected");

        assert!(error.to_string().contains("业务码 1"));
    }

    #[test]
    /// 解析输入并返回结构化值。
    fn parse_api68_latest_issue_uses_first_result_issue() {
        let latest = parse_api68_latest_issue(API68_SAMPLE).expect("latest issue can be parsed");

        assert_eq!(latest.issue, "2026143");
        assert_eq!(latest.draw_time.as_deref(), Some("2026-06-02 21:15:00"));
    }

    #[test]
    /// 解析 API68 单对象 data 响应并返回结构化值。
    fn parse_api68_object_data_response() {
        let draw_number = parse_api68_draw_number(API68_OBJECT_SAMPLE, "20260604001")
            .expect("draw number can be parsed");
        let latest =
            parse_api68_latest_issue(API68_OBJECT_SAMPLE).expect("latest issue can be parsed");

        assert_eq!(draw_number, "01,06,02,04,03,05,07,09,10,08");
        assert_eq!(latest.issue, "20260604001");
        assert_eq!(latest.draw_time.as_deref(), Some("2026-06-04 12:00:00"));
        assert_eq!(latest.next_issue.as_deref(), Some("20260604002"));
        assert_eq!(
            latest.next_draw_time.as_deref(),
            Some("2026-06-04 12:10:00")
        );
    }

    #[test]
    /// 解析输入并返回结构化值。
    fn parse_kj_draw_number_matches_current_issue() {
        let draw_number =
            parse_kj_draw_number(KJ_SAMPLE, "202606031178").expect("draw number can be parsed");

        assert_eq!(draw_number, "9,9,8,7,2");
    }

    #[test]
    /// 解析输入并返回结构化值。
    fn parse_kj_latest_issue_uses_current_and_next_issue() {
        let latest = parse_kj_latest_issue(KJ_SAMPLE).expect("latest issue can be parsed");

        assert_eq!(latest.issue, "202606031178");
        assert_eq!(latest.draw_time.as_deref(), Some("2026-06-03 19:38:01"));
        assert_eq!(latest.next_issue.as_deref(), Some("202606031179"));
        assert_eq!(
            latest.next_draw_time.as_deref(),
            Some("2026-06-03 19:39:00")
        );
    }

    #[test]
    /// 解析 BB 开奖河内 5 分彩响应并返回最近已开奖期号。
    fn parse_bb_draw_number_matches_last_settled_issue() {
        let draw_number =
            parse_bb_draw_number(BB_SAMPLE, "20260617026").expect("draw number can be parsed");

        assert_eq!(draw_number, "5,8,2,6,2");
    }

    #[test]
    /// BB 开奖源命中销售中期号但 openNumber 为空时，应提示等待而不是误报期号不存在。
    fn parse_bb_draw_number_reports_pending_open_number() {
        let error = parse_bb_draw_number(BB_SAMPLE, "20260617027")
            .expect_err("pending issue should wait for draw number");

        assert!(error.log_message().contains("暂未返回开奖号码"));
    }

    #[test]
    /// 解析 BB 开奖源最新期号时用 last 作为已开奖锚点、newest 作为下一期开奖。
    fn parse_bb_latest_issue_uses_last_and_newest_issue() {
        let latest = parse_bb_latest_issue(BB_SAMPLE).expect("latest issue can be parsed");

        assert_eq!(latest.issue, "20260617026");
        assert_eq!(latest.draw_time.as_deref(), Some("2026-06-17 02:10:04"));
        assert_eq!(latest.next_issue.as_deref(), Some("20260617027"));
        assert_eq!(
            latest.next_draw_time.as_deref(),
            Some("2026-06-17 02:15:00")
        );
    }

    #[test]
    /// 解析印尼 5 分彩响应时把 20260617-042 归一为数字期号并返回开奖号码。
    fn parse_indonesia_draw_number_matches_padded_issue() {
        let draw_number = parse_indonesia_draw_number(INDONESIA_SAMPLE, "20260617042")
            .expect("draw number can be parsed");

        assert_eq!(draw_number, "8,7,3,5,8");
    }

    #[test]
    /// 解析印尼 5 分彩最新期号时用 next_num 和 next_time 作为下一期锚点。
    fn parse_indonesia_latest_issue_uses_next_issue_and_time() {
        let latest =
            parse_indonesia_latest_issue(INDONESIA_SAMPLE).expect("latest issue can be parsed");

        assert_eq!(latest.issue, "20260617042");
        assert_eq!(latest.draw_time, None);
        assert_eq!(latest.next_issue.as_deref(), Some("20260617043"));
        assert_eq!(
            latest.next_draw_time.as_deref(),
            Some("2026-06-17 03:35:00")
        );
    }
    /// 验证静态 API68 响应可以为默认全国彩返回最新期号。
    #[tokio::test]
    async fn seeded_static_source_returns_latest_issue_for_default_api68_lotteries() {
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
        let pl5 = repository
            .latest_issue_for_lottery("pl5")
            .await
            .expect("latest issue can be fetched")
            .expect("pl5 has source");

        assert_eq!(fc3d.issue, "2026143");
        assert_eq!(pl3.issue, "2026143");
        assert_eq!(pl5.issue, "2026143");
    }
    /// 验证福彩3D和排列3使用独立的 API68 默认来源。
    #[tokio::test]
    async fn seeded_api68_source_splits_fc3d_and_pl3() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let sources = repository.list().await.expect("sources can be listed");
        let fc3d_source = sources
            .iter()
            .find(|source| source.id == API68_FC3D_SOURCE_ID)
            .expect("seeded source exists");
        let pl3_source = sources
            .iter()
            .find(|source| source.id == API68_PL3_SOURCE_ID)
            .expect("pl3 seeded source exists");

        assert_eq!(fc3d_source.lot_code.as_deref(), Some("10041"));
        assert_eq!(
            fc3d_source.reusable_for_lottery_ids,
            vec!["fc3d".to_string()]
        );
        assert_eq!(pl3_source.lot_code.as_deref(), Some("10043"));
        assert_eq!(
            pl3_source.endpoint.as_deref(),
            Some("https://api.api68.com/QuanGuoCai/getLotteryInfo1.do")
        );
        assert_eq!(pl3_source.reusable_for_lottery_ids, vec!["pl3".to_string()]);
    }
    /// 验证默认开奖源包含 API68 体彩排列5。
    #[tokio::test]
    async fn seeded_api68_source_includes_pl5() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let sources = repository.list().await.expect("sources can be listed");
        let source = sources
            .iter()
            .find(|source| source.id == API68_PL5_SOURCE_ID)
            .expect("pl5 seeded source exists");

        assert_eq!(source.lot_code.as_deref(), Some("10044"));
        assert_eq!(
            source.endpoint.as_deref(),
            Some("https://api.api68.com/QuanGuoCai/getLotteryInfo.do")
        );
        assert_eq!(source.reusable_for_lottery_ids, vec!["pl5".to_string()]);
    }
    /// 验证默认开奖源包含 API68 澳洲幸运5。
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
            Some("https://api.api68.com/CQShiCai/getBaseCQShiCai.do")
        );
        assert_eq!(source.reusable_for_lottery_ids, vec!["au5".to_string()]);
    }

    #[tokio::test]
    /// 默认开奖源包含用户新增的 API68 彩种。
    async fn seeded_sources_include_requested_api68_sources() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let sources = repository.list().await.expect("sources can be listed");

        for (source_id, lottery_id, lot_code, endpoint) in [
            (
                "api68-bjpk10",
                "bjpk10",
                "10001",
                "https://api.api68.com/pks/getLotteryPksInfo.do",
            ),
            (
                "api68-gd11x5",
                "gd11x5",
                "10006",
                "https://api.api68.com/ElevenFive/getElevenFiveInfo.do",
            ),
            (
                "api68-au20",
                "au20",
                "10013",
                "https://api.api68.com/LuckTwenty/getBaseLuckTewnty.do",
            ),
        ] {
            let source = sources
                .iter()
                .find(|source| source.id == source_id)
                .expect("seeded source exists");

            assert_eq!(source.lot_code.as_deref(), Some(lot_code));
            assert_eq!(source.endpoint.as_deref(), Some(endpoint));
            assert_eq!(
                source.reusable_for_lottery_ids,
                vec![lottery_id.to_string()]
            );
        }

        for removed_source_id in [
            "api68-jsk3",
            "api68-gxk3",
            "api68-jlk3",
            "api68-hebk3",
            "api68-nmgk3",
            "api68-ahk3",
            "api68-fjk3",
            "api68-hubk3",
            "api68-bjk3",
            "api68-bjkl8",
        ] {
            assert!(
                !sources.iter().any(|source| source.id == removed_source_id),
                "已停用的 API68 开奖源不应继续出现在默认来源中"
            );
        }
    }
    /// 验证默认开奖源包含 KJAPI 腾讯分彩。
    #[tokio::test]
    async fn seeded_sources_include_txffc_kjapi_source() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let sources = repository.list().await.expect("sources can be listed");
        let source = sources
            .iter()
            .find(|source| source.id == KJ_TXFFC_SOURCE_ID)
            .expect("txffc seeded source exists");

        assert_eq!(source.provider, Some(DrawSourceProvider::KjApi));
        assert_eq!(source.lot_code.as_deref(), Some("txffc"));
        assert_eq!(
            source.endpoint.as_deref(),
            Some("https://kjapi.net/hall/hallajax/getLotteryInfo")
        );
        assert_eq!(source.reusable_for_lottery_ids, vec!["txffc".to_string()]);
    }
    /// 验证默认开奖源包含 BB 河内5分彩。
    #[tokio::test]
    async fn seeded_sources_include_hn5_bb_source() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let sources = repository.list().await.expect("sources can be listed");
        let source = sources
            .iter()
            .find(|source| source.id == BB_HN5_SOURCE_ID)
            .expect("hn5 seeded source exists");

        assert_eq!(source.provider, Some(DrawSourceProvider::BbKaijiang));
        assert_eq!(source.lot_code.as_deref(), Some("VIFFC5"));
        assert_eq!(
            source.endpoint.as_deref(),
            Some("https://www.bbkaijiang.com/api/st-lottery-open/open-result/list-newest-result")
        );
        assert_eq!(source.reusable_for_lottery_ids, vec!["hn5".to_string()]);
    }
    /// 验证 BB 开奖源可以返回河内5分彩最新期号。
    #[tokio::test]
    async fn seeded_bb_source_returns_latest_issue_for_hn5() {
        let repository = ApiDrawSourceRepository::bb_seeded_with_static_response(BB_SAMPLE);

        let latest = repository
            .latest_issue_for_lottery("hn5")
            .await
            .expect("latest issue can be fetched")
            .expect("hn5 has source");

        assert_eq!(latest.issue, "20260617026");
        assert_eq!(latest.next_issue.as_deref(), Some("20260617027"));
    }
    /// 验证默认开奖源包含印尼5分彩来源。
    #[tokio::test]
    async fn seeded_sources_include_id5_indonesia_source() {
        let repository = ApiDrawSourceRepository::api68_seeded();
        let sources = repository.list().await.expect("sources can be listed");
        let source = sources
            .iter()
            .find(|source| source.id == IDN5_SOURCE_ID)
            .expect("id5 seeded source exists");

        assert_eq!(source.provider, Some(DrawSourceProvider::IndonesiaLottery));
        assert_eq!(source.lot_code.as_deref(), Some("IDFFC5"));
        assert_eq!(
            source.endpoint.as_deref(),
            Some("https://draw.indonesia-lottery.org/others/draw.php")
        );
        assert_eq!(source.reusable_for_lottery_ids, vec!["id5".to_string()]);
    }
    /// 验证印尼开奖源可以返回印尼5分彩最新期号。
    #[tokio::test]
    async fn seeded_indonesia_source_returns_latest_issue_for_id5() {
        let repository =
            ApiDrawSourceRepository::indonesia_seeded_with_static_response(INDONESIA_SAMPLE);

        let latest = repository
            .latest_issue_for_lottery("id5")
            .await
            .expect("latest issue can be fetched")
            .expect("id5 has source");

        assert_eq!(latest.issue, "20260617042");
        assert_eq!(latest.next_issue.as_deref(), Some("20260617043"));
    }
    /// 验证 KJAPI 可以返回腾讯分分彩最新期号。
    #[tokio::test]
    async fn seeded_kj_source_returns_latest_issue_for_txffc() {
        let repository = ApiDrawSourceRepository::kj_seeded_with_static_response(KJ_SAMPLE);

        let latest = repository
            .latest_issue_for_lottery("txffc")
            .await
            .expect("latest issue can be fetched")
            .expect("txffc has source");

        assert_eq!(latest.issue, "202606031178");
        assert_eq!(latest.next_issue.as_deref(), Some("202606031179"));
    }
    /// 验证来源创建拒绝重复彩种binding。
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
    /// 验证来源更新keeps默认pl3bindingindependent。
    #[tokio::test]
    async fn source_update_keeps_default_pl3_binding_independent() {
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
        let sources = repository.list().await.expect("sources can be listed");
        let pl3_source = sources
            .iter()
            .find(|source| source.id == API68_PL3_SOURCE_ID)
            .expect("pl3 source remains configured");

        assert_eq!(updated.reusable_for_lottery_ids, vec!["fc3d".to_string()]);
        assert_eq!(pl3_source.reusable_for_lottery_ids, vec!["pl3".to_string()]);
    }
    /// 验证来源保存拒绝非API彩种。
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

    /// 构造测试用开奖源保存请求。
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
