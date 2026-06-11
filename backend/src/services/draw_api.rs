//! 开奖源服务层，管理第三方开奖接口和开奖结果解析

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
    time::Duration,
};

use serde::{de, Deserialize, Deserializer, Serialize};
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
pub const API68_FC3D_SOURCE_NAME: &str = "API68 福彩 3D";
pub const API68_FC3D_LOTTERY_ID: &str = "fc3d";
pub const API68_PL3_SOURCE_ID: &str = "api68-pl3";
pub const API68_PL3_SOURCE_NAME: &str = "API68 体彩排列3";
pub const API68_PL3_LOTTERY_ID: &str = "pl3";
pub const API68_FC3D_LOT_CODE: &str = "10041";
pub const API68_PL3_LOT_CODE: &str = "10043";
pub const API68_PL5_SOURCE_ID: &str = "api68-pl5";
pub const API68_PL5_SOURCE_NAME: &str = "API68 体彩排列5";
pub const API68_PL5_LOTTERY_ID: &str = "pl5";
pub const API68_PL5_LOT_CODE: &str = "10044";
pub const API68_AU5_SOURCE_ID: &str = "api68-au5";
pub const API68_AU5_SOURCE_NAME: &str = "API68 澳洲幸运5";
pub const API68_AU5_LOTTERY_ID: &str = "au5";
pub const API68_AU5_LOT_CODE: &str = "10010";
pub const KJ_TXFFC_SOURCE_ID: &str = "kj-txffc";
pub const KJ_TXFFC_SOURCE_NAME: &str = "KJAPI 腾讯分分彩";
pub const KJ_TXFFC_LOTTERY_ID: &str = "txffc";
pub const KJ_TXFFC_LOT_KEY: &str = "txffc";
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
const API_DRAW_SOURCE_TIMEOUT_SECONDS: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 外部开奖源返回的最新期号和开奖时间摘要。
pub struct ApiDrawSourceLatestIssue {
    pub issue: String,
    pub draw_time: Option<String>,
    pub next_issue: Option<String>,
    pub next_draw_time: Option<String>,
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

    /// 创建并初始化新实例。
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

    /// 返回完整列表。
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
            DrawSourceProvider::Api68 => self.fetch_api68_draw_number(&source, &issue.issue).await,
            DrawSourceProvider::KjApi => self.fetch_kj_draw_number(&source, &issue.issue).await,
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
            DrawSourceProvider::Api68 => self.fetch_api68_latest_issue(&source).await,
            DrawSourceProvider::KjApi => self.fetch_kj_latest_issue(&source).await,
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
            .map_err(|_| ApiError::Internal("API 开奖源请求失败".to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Internal(format!(
                "API 开奖源返回 HTTP 状态 {status}"
            )));
        }

        let response_body = response
            .text()
            .await
            .map_err(|_| ApiError::Internal("API 开奖源响应读取失败".to_string()))?;

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
            .map_err(|_| ApiError::Internal("API 开奖源请求失败".to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Internal(format!(
                "API 开奖源返回 HTTP 状态 {status}"
            )));
        }

        let response_body = response
            .text()
            .await
            .map_err(|_| ApiError::Internal("API 开奖源响应读取失败".to_string()))?;

        parse_api68_latest_issue(&response_body)
    }

    async fn fetch_kj_draw_number(
        &self,
        source: &ApiDrawSourceConfig,
        issue: &str,
    ) -> ApiResult<String> {
        if let Some(response_body) = self.static_responses.get(&source.id) {
            return parse_kj_draw_number(response_body, issue);
        }

        let response = self
            .client
            .get(source.url())
            .timeout(Duration::from_secs(API_DRAW_SOURCE_TIMEOUT_SECONDS))
            .send()
            .await
            .map_err(|_| ApiError::Internal("API 开奖源请求失败".to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Internal(format!(
                "API 开奖源返回 HTTP 状态 {status}"
            )));
        }

        let response_body = response
            .text()
            .await
            .map_err(|_| ApiError::Internal("API 开奖源响应读取失败".to_string()))?;

        parse_kj_draw_number(&response_body, issue)
    }

    async fn fetch_kj_latest_issue(
        &self,
        source: &ApiDrawSourceConfig,
    ) -> ApiResult<ApiDrawSourceLatestIssue> {
        if let Some(response_body) = self.static_responses.get(&source.id) {
            return parse_kj_latest_issue(response_body);
        }

        let response = self
            .client
            .get(source.url())
            .timeout(Duration::from_secs(API_DRAW_SOURCE_TIMEOUT_SECONDS))
            .send()
            .await
            .map_err(|_| ApiError::Internal("API 开奖源请求失败".to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(ApiError::Internal(format!(
                "API 开奖源返回 HTTP 状态 {status}"
            )));
        }

        let response_body = response
            .text()
            .await
            .map_err(|_| ApiError::Internal("API 开奖源响应读取失败".to_string()))?;

        parse_kj_latest_issue(&response_body)
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
    /// 创建并初始化新实例。
    fn new(sources: Vec<ApiDrawSourceConfig>) -> Self {
        Self {
            sources: sources
                .into_iter()
                .map(|source| (source.id.clone(), source))
                .collect(),
        }
    }

    /// 返回完整数据列表。
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

    /// 处理 source_from_request 的具体内部流程。
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

    /// 校验输入参数并返回校验结果。
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
    /// 处理 api68_fc3d 的具体内部流程。
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

    /// 处理 api68_au5 的具体内部流程。
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

    /// 处理 kj_txffc 的具体内部流程。
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

    /// 处理 url 的具体内部流程。
    fn url(&self) -> String {
        match self.provider {
            DrawSourceProvider::Api68 => source_url(&self.endpoint, "lotCode", &self.lot_code),
            DrawSourceProvider::KjApi => source_url(&self.endpoint, "lotKey", &self.lot_code),
        }
    }

    /// 聚合并返回摘要结果。
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

/// 处理 default_api68_quanguocai_endpoint 的具体内部流程。
fn default_api68_quanguocai_endpoint() -> String {
    DEFAULT_API68_QUANGUOCAI_ENDPOINT.to_string()
}

/// 处理 default_api68_cqshicai_single_endpoint 的具体内部流程。
fn default_api68_cqshicai_single_endpoint() -> String {
    DEFAULT_API68_CQSHICAI_SINGLE_ENDPOINT.to_string()
}

/// 处理 default_kj_endpoint 的具体内部流程。
fn default_kj_endpoint() -> String {
    DEFAULT_KJ_ENDPOINT.to_string()
}

/// 处理 normalized_endpoint 的具体内部流程。
fn normalized_endpoint(endpoint: Option<&str>, provider: &DrawSourceProvider) -> String {
    endpoint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| match provider {
            DrawSourceProvider::Api68 => default_api68_quanguocai_endpoint(),
            DrawSourceProvider::KjApi => default_kj_endpoint(),
        })
}

/// 处理 default_api_draw_sources 的具体内部流程。
fn default_api_draw_sources() -> Vec<ApiDrawSourceConfig> {
    let mut sources = vec![
        ApiDrawSourceConfig::api68_fc3d(),
        ApiDrawSourceConfig::api68_pl3(),
        ApiDrawSourceConfig::api68_pl5(),
        ApiDrawSourceConfig::api68_au5(),
        ApiDrawSourceConfig::kj_txffc(),
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

/// 处理 source_url 的具体内部流程。
fn source_url(endpoint: &str, query_key: &str, lot_code: &str) -> String {
    if endpoint.contains(&format!("{query_key}=")) {
        return endpoint.to_string();
    }
    let separator = if endpoint.contains('?') { '&' } else { '?' };
    format!("{endpoint}{separator}{query_key}={lot_code}")
}

/// 处理 reusable_lottery_ids 的具体内部流程。
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

/// 校验输入参数并返回校验结果。
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

/// 处理 api68_issue_value 的具体内部流程。
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

#[cfg(test)]
mod tests {
    use super::{
        parse_api68_draw_number, parse_api68_latest_issue, parse_kj_draw_number,
        parse_kj_latest_issue, ApiDrawSourceRepository, API68_AU5_SOURCE_ID, API68_FC3D_SOURCE_ID,
        API68_PL3_SOURCE_ID, API68_PL5_SOURCE_ID, KJ_TXFFC_SOURCE_ID,
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

    /// 处理 draw_source_payload 的具体内部流程。
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
