//! 彩种与开奖来源领域模型，定义开奖方式和销售与合买配置

use std::{collections::BTreeMap, error::Error, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::{
    domain::{
        lottery::{
            DrawMode, DrawSchedule, GroupBuyConfig, LotteryCategoryConfig, LotteryKind,
            LotteryNumberType, LotteryPlayConfig, LotteryPlayPositionSelectLimit, PlayCategory,
            DEFAULT_ISSUE_FORMAT_PATTERN,
        },
        play::PlayRuleCode,
    },
    error::{ApiError, ApiResult},
    services::{
        draw_generation::normalize_issue_format_pattern,
        play_rules::{
            number_type_for_rule, play_category_for_rule, play_position_select_limit_targets,
            play_rule_summaries,
        },
    },
};

#[derive(Clone)]
/// 彩种配置仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct LotteryRepository {
    inner: Arc<LotteryRepositoryKind>,
}

/// 彩种仓储运行模式，区分内存演示数据和 PostgreSQL 持久化。
enum LotteryRepositoryKind {
    Memory(std::sync::RwLock<LotteryStore>),
    Postgres(PostgresLotteryStore),
}

/// 彩种配置仓储，负责该模块数据读取、业务变更和持久化协调。
impl LotteryRepository {
    /// 返回带内置种子数据的内存仓储实例。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(LotteryRepositoryKind::Memory(std::sync::RwLock::new(
                LotteryStore::seeded(),
            ))),
        }
    }

    /// 基于连接字符串创建数据库连接池并执行迁移。
    pub async fn postgres(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        let store = PostgresLotteryStore { pool };
        store.seed_missing_defaults().await?;

        Ok(Self {
            inner: Arc::new(LotteryRepositoryKind::Postgres(store)),
        })
    }

    /// 返回完整列表。
    pub async fn list(&self) -> ApiResult<Vec<LotteryKind>> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .read()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))
                .map(|store| store.list()),
            LotteryRepositoryKind::Postgres(store) => store.list().await,
        }
    }

    /// 按 ID 查询单条记录。
    pub async fn get(&self, id: &str) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .read()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .get(id),
            LotteryRepositoryKind::Postgres(store) => store.get(id).await,
        }
    }

    /// 校验入参并创建一条新记录。
    pub async fn create(&self, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .create(lottery),
            LotteryRepositoryKind::Postgres(store) => store.create(lottery).await,
        }
    }

    /// 返回全部可用的彩种分类。
    pub async fn categories(&self) -> ApiResult<Vec<LotteryCategoryConfig>> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .read()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))
                .map(|store| store.categories()),
            LotteryRepositoryKind::Postgres(store) => store.list_categories().await,
        }
    }

    /// 新增一条彩种分类。
    pub async fn create_category(
        &self,
        category: LotteryCategoryConfig,
    ) -> ApiResult<LotteryCategoryConfig> {
        let category = normalize_category_config(category)?;

        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .create_category(category),
            LotteryRepositoryKind::Postgres(store) => store.create_category(category).await,
        }
    }

    /// 更新彩种分类名称。
    pub async fn update_category(
        &self,
        code: &str,
        category: LotteryCategoryConfig,
    ) -> ApiResult<LotteryCategoryConfig> {
        let category = normalize_category_config(category)?;

        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .update_category(code, category),
            LotteryRepositoryKind::Postgres(store) => store.update_category(code, category).await,
        }
    }

    /// 删除一条彩种分类。
    pub async fn delete_category(&self, code: &str) -> ApiResult<LotteryCategoryConfig> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .delete_category(code),
            LotteryRepositoryKind::Postgres(store) => store.delete_category(code).await,
        }
    }

    /// 更新现有记录并持久化变更。
    pub async fn update(&self, id: &str, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .update(id, lottery),
            LotteryRepositoryKind::Postgres(store) => store.update(id, lottery).await,
        }
    }

    /// 删除现有记录并返回被删对象。
    pub async fn delete(&self, id: &str) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .delete(id),
            LotteryRepositoryKind::Postgres(store) => store.delete(id).await,
        }
    }

    /// 更新彩种售卖开关。
    pub async fn set_sale_enabled(&self, id: &str, sale_enabled: bool) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .set_sale_enabled(id, sale_enabled),
            LotteryRepositoryKind::Postgres(store) => {
                store.set_sale_enabled(id, sale_enabled).await
            }
        }
    }

    /// 返回彩种仓储是否已经使用数据库直读模式，直读模式无需额外刷新内存快照。
    pub fn is_database_backed(&self) -> bool {
        matches!(self.inner.as_ref(), LotteryRepositoryKind::Postgres(_))
    }
}

#[derive(Debug, Clone)]
/// 彩种配置运行时数据快照，用于内存模式和数据库持久化前的业务校验。
pub struct LotteryStore {
    lotteries: BTreeMap<String, LotteryKind>,
    categories: BTreeMap<String, LotteryCategoryConfig>,
}

/// 彩种配置运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl LotteryStore {
    /// 返回内置种子数据。
    pub fn seeded() -> Self {
        Self::from_lotteries(seed_lotteries())
    }

    /// 处理 from_lotteries 的具体内部流程。
    fn from_lotteries(lotteries: Vec<LotteryKind>) -> Self {
        Self {
            lotteries: lotteries
                .into_iter()
                .map(|lottery| (lottery.id.clone(), lottery))
                .collect(),
            categories: lottery_categories()
                .into_iter()
                .map(|category| (category.code.clone(), category))
                .collect(),
        }
    }

    /// 返回可选分类。
    pub fn categories(&self) -> Vec<LotteryCategoryConfig> {
        self.categories.values().cloned().collect()
    }

    /// 返回完整列表。
    pub fn list(&self) -> Vec<LotteryKind> {
        self.lotteries.values().cloned().collect()
    }

    /// 按 ID 查询单条记录。
    pub fn get(&self, id: &str) -> ApiResult<LotteryKind> {
        self.lotteries
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

    /// 校验入参并创建一条新记录。
    pub fn create(&mut self, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        let lottery = normalize_lottery(lottery)?;

        if !self.categories.contains_key(&lottery.category) {
            return Err(ApiError::BadRequest(format!(
                "lottery category `{}` not found",
                lottery.category
            )));
        }

        if self.lotteries.contains_key(&lottery.id) {
            return Err(ApiError::Conflict(format!(
                "lottery `{}` already exists",
                lottery.id
            )));
        }

        self.lotteries.insert(lottery.id.clone(), lottery.clone());
        Ok(lottery)
    }

    /// 更新现有记录并持久化变更。
    pub fn update(&mut self, id: &str, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        let lottery = normalize_lottery(lottery)?;

        if !self.categories.contains_key(&lottery.category) {
            return Err(ApiError::BadRequest(format!(
                "lottery category `{}` not found",
                lottery.category
            )));
        }

        if id != lottery.id {
            return Err(ApiError::BadRequest(
                "path id must match lottery id".to_string(),
            ));
        }

        if !self.lotteries.contains_key(id) {
            return Err(ApiError::NotFound(format!("lottery `{id}` not found")));
        }

        self.lotteries.insert(id.to_string(), lottery.clone());
        Ok(lottery)
    }

    /// 删除现有记录并返回被删对象。
    pub fn delete(&mut self, id: &str) -> ApiResult<LotteryKind> {
        self.lotteries
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

    /// 更新彩种售卖开关。
    pub fn set_sale_enabled(&mut self, id: &str, sale_enabled: bool) -> ApiResult<LotteryKind> {
        let lottery = self
            .lotteries
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))?;

        lottery.sale_enabled = sale_enabled;
        Ok(lottery.clone())
    }

    /// 新增一条分类。
    pub fn create_category(
        &mut self,
        category: LotteryCategoryConfig,
    ) -> ApiResult<LotteryCategoryConfig> {
        if self.categories.contains_key(&category.code) {
            return Err(ApiError::Conflict(format!(
                "lottery category `{}` already exists",
                category.code
            )));
        }

        self.categories
            .insert(category.code.clone(), category.clone());
        Ok(category)
    }

    /// 更新分类名称。
    pub fn update_category(
        &mut self,
        code: &str,
        category: LotteryCategoryConfig,
    ) -> ApiResult<LotteryCategoryConfig> {
        let existing = self
            .categories
            .get_mut(code)
            .ok_or_else(|| ApiError::NotFound(format!("lottery category `{code}` not found")))?;

        existing.name = category.name;
        Ok(existing.clone())
    }

    /// 删除分类。
    pub fn delete_category(&mut self, code: &str) -> ApiResult<LotteryCategoryConfig> {
        let is_used = self
            .lotteries
            .values()
            .any(|lottery| lottery.category == code);
        if is_used {
            return Err(ApiError::BadRequest(
                "分类已被彩种引用，无法删除".to_string(),
            ));
        }

        self.categories
            .remove(code)
            .ok_or_else(|| ApiError::NotFound(format!("lottery category `{code}` not found")))
    }
}

/// 彩种配置运行时数据快照，用于内存模式和数据库持久化前的业务校验。
struct PostgresLotteryStore {
    pool: PgPool,
}

/// 彩种配置运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl PostgresLotteryStore {
    /// 写入彩种配置默认种子数据，供空库初始化使用。
    async fn seed_missing_defaults(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        for lottery in seed_lotteries() {
            self.insert_seed_lottery(lottery).await?;
        }

        self.insert_seed_categories().await?;

        Ok(())
    }

    async fn list_categories(&self) -> ApiResult<Vec<LotteryCategoryConfig>> {
        let rows = sqlx::query(
            "SELECT code, name
             FROM lottery_categories
             ORDER BY sort_order ASC, code ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;

        let mut categories = Vec::new();
        for row in rows {
            categories.push(LotteryCategoryConfig {
                code: row.try_get("code").map_err(database_error)?,
                name: row.try_get("name").map_err(database_error)?,
            });
        }

        Ok(categories)
    }

    async fn create_category(
        &self,
        category: LotteryCategoryConfig,
    ) -> ApiResult<LotteryCategoryConfig> {
        sqlx::query(
            "INSERT INTO lottery_categories (code, name)
             VALUES ($1, $2)",
        )
        .bind(&category.code)
        .bind(&category.name)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            tracing::error!(error = error.to_string(), "创建彩种分类失败");
            database_error(error)
        })?;

        Ok(category)
    }

    async fn update_category(
        &self,
        code: &str,
        category: LotteryCategoryConfig,
    ) -> ApiResult<LotteryCategoryConfig> {
        if code != category.code {
            return Err(ApiError::BadRequest("分类代码不能修改".to_string()));
        }

        let updated = sqlx::query(
            "UPDATE lottery_categories
             SET name = $2,
                 updated_at = now()
             WHERE code = $1
             RETURNING code, name",
        )
        .bind(code)
        .bind(&category.name)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?
        .ok_or_else(|| ApiError::NotFound(format!("lottery category `{code}` not found")))?;

        Ok(LotteryCategoryConfig {
            code: updated.try_get("code").map_err(database_error)?,
            name: updated.try_get("name").map_err(database_error)?,
        })
    }

    async fn delete_category(&self, code: &str) -> ApiResult<LotteryCategoryConfig> {
        let in_use = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM lotteries WHERE category = $1)",
        )
        .bind(code)
        .fetch_one(&self.pool)
        .await
        .map_err(database_error)?;

        if in_use {
            return Err(ApiError::BadRequest(
                "分类已被彩种引用，无法删除".to_string(),
            ));
        }

        let deleted = sqlx::query(
            "DELETE FROM lottery_categories
             WHERE code = $1
             RETURNING code, name",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?
        .ok_or_else(|| ApiError::NotFound(format!("lottery category `{code}` not found")))?;

        Ok(LotteryCategoryConfig {
            code: deleted.try_get("code").map_err(database_error)?,
            name: deleted.try_get("name").map_err(database_error)?,
        })
    }

    async fn ensure_category_exists(&self, category: &str) -> ApiResult<()> {
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM lottery_categories WHERE code = $1)",
        )
        .bind(category)
        .fetch_one(&self.pool)
        .await
        .map_err(database_error)?;

        if !exists {
            return Err(ApiError::BadRequest(format!(
                "lottery category `{category}` not found",
            )));
        }

        Ok(())
    }

    async fn insert_seed_categories(&self) -> ApiResult<()> {
        for (sort_order, category) in lottery_categories().into_iter().enumerate() {
            sqlx::query(
                "INSERT INTO lottery_categories (code, name, sort_order)
                 VALUES ($1, $2, $3)
                 ON CONFLICT (code) DO NOTHING",
            )
            .bind(&category.code)
            .bind(&category.name)
            .bind(i32::try_from(sort_order).unwrap_or(0))
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        }

        Ok(())
    }

    async fn list(&self) -> ApiResult<Vec<LotteryKind>> {
        let rows = sqlx::query(
            "SELECT id, name, category, logo_url, number_type, draw_mode, api_draw_delay_seconds, issue_format, schedule, sale_enabled, group_buy, play_categories, play_configs
             FROM lotteries
             ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;

        rows.into_iter().map(lottery_from_row).collect()
    }

    async fn get(&self, id: &str) -> ApiResult<LotteryKind> {
        let row = sqlx::query(
            "SELECT id, name, category, logo_url, number_type, draw_mode, api_draw_delay_seconds, issue_format, schedule, sale_enabled, group_buy, play_categories, play_configs
             FROM lotteries
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?;

        row.map(lottery_from_row)
            .transpose()?
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

    async fn create(&self, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        let lottery = normalize_lottery(lottery)?;
        self.ensure_category_exists(&lottery.category).await?;

        let created = sqlx::query(
            "INSERT INTO lotteries (
                id, name, category, logo_url, number_type, draw_mode, api_draw_delay_seconds, issue_format, schedule, sale_enabled, group_buy, play_categories, play_configs
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (id) DO NOTHING
             RETURNING id, name, category, logo_url, number_type, draw_mode, api_draw_delay_seconds, issue_format, schedule, sale_enabled, group_buy, play_categories, play_configs",
        )
        .bind(&lottery.id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.category)?)
        .bind(lottery.logo_url.trim())
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
        .bind(lottery.api_draw_delay_seconds as i32)
        .bind(&lottery.issue_format)
        .bind(json_value(&lottery.schedule)?)
        .bind(lottery.sale_enabled)
        .bind(json_value(&lottery.group_buy)?)
        .bind(json_value(&lottery.play_categories)?)
        .bind(json_value(&lottery.play_configs)?)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?;

        created
            .map(lottery_from_row)
            .transpose()?
            .ok_or_else(|| ApiError::Conflict(format!("lottery `{}` already exists", lottery.id)))
    }

    async fn insert_seed_lottery(&self, lottery: LotteryKind) -> ApiResult<()> {
        let lottery = normalize_lottery(lottery)?;

        sqlx::query(
            "INSERT INTO lotteries (
                id, name, category, logo_url, number_type, draw_mode, api_draw_delay_seconds, issue_format, schedule, sale_enabled, group_buy, play_categories, play_configs
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&lottery.id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.category)?)
        .bind(lottery.logo_url.trim())
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
        .bind(lottery.api_draw_delay_seconds as i32)
        .bind(&lottery.issue_format)
        .bind(json_value(&lottery.schedule)?)
        .bind(lottery.sale_enabled)
        .bind(json_value(&lottery.group_buy)?)
        .bind(json_value(&lottery.play_categories)?)
        .bind(json_value(&lottery.play_configs)?)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;

        Ok(())
    }

    async fn update(&self, id: &str, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        let lottery = normalize_lottery(lottery)?;
        self.ensure_category_exists(&lottery.category).await?;

        if id != lottery.id {
            return Err(ApiError::BadRequest(
                "path id must match lottery id".to_string(),
            ));
        }

        let updated = sqlx::query(
            "UPDATE lotteries
             SET name = $2,
                 category = $3,
                 logo_url = $4,
                 number_type = $5,
                 draw_mode = $6,
                 api_draw_delay_seconds = $7,
                 issue_format = $8,
                 schedule = $9,
                 sale_enabled = $10,
                 group_buy = $11,
                 play_categories = $12,
                 play_configs = $13,
                 updated_at = now()
             WHERE id = $1
             RETURNING id, name, category, logo_url, number_type, draw_mode, api_draw_delay_seconds, issue_format, schedule, sale_enabled, group_buy, play_categories, play_configs",
        )
        .bind(id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.category)?)
        .bind(lottery.logo_url.trim())
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
        .bind(lottery.api_draw_delay_seconds as i32)
        .bind(&lottery.issue_format)
        .bind(json_value(&lottery.schedule)?)
        .bind(lottery.sale_enabled)
        .bind(json_value(&lottery.group_buy)?)
        .bind(json_value(&lottery.play_categories)?)
        .bind(json_value(&lottery.play_configs)?)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?;

        updated
            .map(lottery_from_row)
            .transpose()?
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

    async fn delete(&self, id: &str) -> ApiResult<LotteryKind> {
        let deleted = sqlx::query(
            "DELETE FROM lotteries
             WHERE id = $1
             RETURNING id, name, category, logo_url, number_type, draw_mode, api_draw_delay_seconds, issue_format, schedule, sale_enabled, group_buy, play_categories, play_configs",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?;

        deleted
            .map(lottery_from_row)
            .transpose()?
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

    async fn set_sale_enabled(&self, id: &str, sale_enabled: bool) -> ApiResult<LotteryKind> {
        let updated = sqlx::query(
            "UPDATE lotteries
             SET sale_enabled = $2,
                 updated_at = now()
             WHERE id = $1
             RETURNING id, name, category, logo_url, number_type, draw_mode, api_draw_delay_seconds, issue_format, schedule, sale_enabled, group_buy, play_categories, play_configs",
        )
        .bind(id)
        .bind(sale_enabled)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?;

        updated
            .map(lottery_from_row)
            .transpose()?
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }
}

/// 返回平台内置彩种默认数据。
pub fn seed_lotteries() -> Vec<LotteryKind> {
    let mut lotteries = vec![
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            category: "regional".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            api_draw_delay_seconds: 0,
            issue_format: DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: false,
            group_buy: group_buy_config(),
            play_categories: vec![
                PlayCategory::Direct,
                PlayCategory::GroupThree,
                PlayCategory::GroupSix,
            ],
            play_configs: play_configs_with_overrides(
                LotteryNumberType::ThreeDigit,
                &[
                    PlayCategory::Direct,
                    PlayCategory::GroupThree,
                    PlayCategory::GroupSix,
                ],
                &[
                    (PlayRuleCode::ThreeDirect, 104_000),
                    (PlayRuleCode::ThreeGroupThree, 52_000),
                    (PlayRuleCode::ThreeGroupThreeBanker, 52_000),
                    (PlayRuleCode::ThreeGroupSix, 50_000),
                    (PlayRuleCode::ThreeGroupSixBanker, 50_000),
                ],
            ),
        },
        LotteryKind {
            id: "pl3".to_string(),
            name: "排列 3".to_string(),
            category: "regional".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            api_draw_delay_seconds: 0,
            issue_format: DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: false,
            group_buy: group_buy_config(),
            play_categories: vec![
                PlayCategory::Direct,
                PlayCategory::GroupThree,
                PlayCategory::GroupSix,
            ],
            play_configs: play_configs_with_overrides(
                LotteryNumberType::ThreeDigit,
                &[
                    PlayCategory::Direct,
                    PlayCategory::GroupThree,
                    PlayCategory::GroupSix,
                ],
                &[
                    (PlayRuleCode::ThreeDirect, 98_000),
                    (PlayRuleCode::ThreeGroupThree, 49_000),
                    (PlayRuleCode::ThreeGroupThreeBanker, 49_000),
                    (PlayRuleCode::ThreeGroupSix, 48_000),
                    (PlayRuleCode::ThreeGroupSixBanker, 48_000),
                ],
            ),
        },
        LotteryKind {
            id: "au5".to_string(),
            name: "澳洲幸运5".to_string(),
            category: "overseas".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Api,
            api_draw_delay_seconds: 0,
            issue_format: DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            schedule: DrawSchedule::Periodic {
                interval_seconds: 300,
            },
            sale_enabled: false,
            group_buy: group_buy_config(),
            play_categories: vec![
                PlayCategory::Direct,
                PlayCategory::DirectCombination,
                PlayCategory::GroupThree,
                PlayCategory::GroupSix,
                PlayCategory::BigSmallOddEven,
            ],
            play_configs: play_configs_with_overrides(
                LotteryNumberType::FiveDigit,
                &[
                    PlayCategory::Direct,
                    PlayCategory::DirectCombination,
                    PlayCategory::GroupThree,
                    PlayCategory::GroupSix,
                    PlayCategory::BigSmallOddEven,
                ],
                &[
                    (PlayRuleCode::FiveFrontDirect, 95_000),
                    (PlayRuleCode::FiveMiddleDirect, 96_000),
                    (PlayRuleCode::FiveBackDirect, 97_000),
                    (PlayRuleCode::FiveBigSmallOddEven, 19_000),
                ],
            ),
        },
        LotteryKind {
            id: "txffc".to_string(),
            name: "腾讯分分彩".to_string(),
            category: "overseas".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Api,
            api_draw_delay_seconds: 0,
            issue_format: DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: false,
            group_buy: group_buy_config(),
            play_categories: vec![
                PlayCategory::Direct,
                PlayCategory::DirectCombination,
                PlayCategory::GroupThree,
                PlayCategory::GroupSix,
                PlayCategory::BigSmallOddEven,
            ],
            play_configs: play_configs_with_overrides(
                LotteryNumberType::FiveDigit,
                &[
                    PlayCategory::Direct,
                    PlayCategory::DirectCombination,
                    PlayCategory::GroupThree,
                    PlayCategory::GroupSix,
                    PlayCategory::BigSmallOddEven,
                ],
                &[
                    (PlayRuleCode::FiveFrontDirect, 95_000),
                    (PlayRuleCode::FiveMiddleDirect, 96_000),
                    (PlayRuleCode::FiveBackDirect, 97_000),
                    (PlayRuleCode::FiveBigSmallOddEven, 19_000),
                ],
            ),
        },
        LotteryKind {
            id: "ssc60".to_string(),
            name: "60 秒时时彩".to_string(),
            category: "overseas".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Platform,
            api_draw_delay_seconds: 0,
            issue_format: DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: false,
            group_buy: group_buy_config(),
            play_categories: vec![
                PlayCategory::Direct,
                PlayCategory::DirectCombination,
                PlayCategory::GroupThree,
                PlayCategory::GroupSix,
                PlayCategory::BigSmallOddEven,
            ],
            play_configs: play_configs_with_overrides(
                LotteryNumberType::FiveDigit,
                &[
                    PlayCategory::Direct,
                    PlayCategory::DirectCombination,
                    PlayCategory::GroupThree,
                    PlayCategory::GroupSix,
                    PlayCategory::BigSmallOddEven,
                ],
                &[
                    (PlayRuleCode::FiveFrontDirect, 95_000),
                    (PlayRuleCode::FiveMiddleDirect, 96_000),
                    (PlayRuleCode::FiveBackDirect, 97_000),
                    (PlayRuleCode::FiveBigSmallOddEven, 19_000),
                ],
            ),
        },
        LotteryKind {
            id: "manual-test".to_string(),
            name: "指定号码测试彩".to_string(),
            category: "other".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Manual,
            api_draw_delay_seconds: 0,
            issue_format: DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            schedule: DrawSchedule::Weekly {
                weekdays: vec!["Tuesday".to_string(), "Thursday".to_string()],
                time: "21:00:00".to_string(),
            },
            sale_enabled: false,
            group_buy: GroupBuyConfig {
                enabled: false,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1_000,
            },
            play_categories: vec![PlayCategory::Direct],
            play_configs: play_configs_with_overrides(
                LotteryNumberType::FiveDigit,
                &[PlayCategory::Direct],
                &[
                    (PlayRuleCode::FiveFrontDirect, 88_000),
                    (PlayRuleCode::FiveMiddleDirect, 88_000),
                    (PlayRuleCode::FiveBackDirect, 88_000),
                ],
            ),
        },
    ];
    lotteries.extend(extra_api68_lotteries());
    lotteries
}

/// 返回用户要求新增接入的 API68 彩种默认数据。
fn extra_api68_lotteries() -> Vec<LotteryKind> {
    vec![
        api_lottery(
            "bjpk10",
            "北京PK10",
            "regional",
            LotteryNumberType::Pk10,
            600,
        ),
        api_lottery(
            "tjssc",
            "天津时时彩",
            "regional",
            LotteryNumberType::FiveDigit,
            1200,
        ),
        api_lottery(
            "xjssc",
            "新疆时时彩",
            "regional",
            LotteryNumberType::FiveDigit,
            1200,
        ),
        api_lottery(
            "gd11x5",
            "广东11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "au10",
            "澳洲幸运10",
            "overseas",
            LotteryNumberType::Pk10,
            300,
        ),
        api_lottery(
            "au20",
            "澳洲幸运20",
            "overseas",
            LotteryNumberType::LuckTwenty,
            300,
        ),
        api_lottery(
            "jx11x5",
            "江西11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "js11x5",
            "江苏11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "ah11x5",
            "安徽11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "sh11x5",
            "上海11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "ln11x5",
            "辽宁11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "hb11x5",
            "湖北11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "gx11x5",
            "广西11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "jl11x5",
            "吉林11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "nmg11x5",
            "内蒙古11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
        api_lottery(
            "zj11x5",
            "浙江11选5",
            "regional",
            LotteryNumberType::ElevenFive,
            600,
        ),
    ]
}

/// 构造 API 彩种默认值，已接入玩法的号码类型会自动补玩法配置。
fn api_lottery(
    id: &str,
    name: &str,
    category: &str,
    number_type: LotteryNumberType,
    interval_seconds: u32,
) -> LotteryKind {
    let play_categories = default_play_categories_for_number_type(&number_type);
    let play_configs = play_configs_with_overrides(number_type.clone(), &play_categories, &[]);

    LotteryKind {
        id: id.to_string(),
        name: name.to_string(),
        category: category.to_string(),
        logo_url: String::new(),
        number_type,
        draw_mode: DrawMode::Api,
        api_draw_delay_seconds: 0,
        issue_format: DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
        schedule: DrawSchedule::Periodic { interval_seconds },
        sale_enabled: false,
        group_buy: group_buy_config(),
        play_categories,
        play_configs,
    }
}

/// 返回已接入投注玩法号码类型的默认玩法分类；其它采集型彩种暂不开放投注玩法。
fn default_play_categories_for_number_type(number_type: &LotteryNumberType) -> Vec<PlayCategory> {
    match number_type {
        LotteryNumberType::ThreeDigit => vec![
            PlayCategory::Direct,
            PlayCategory::GroupThree,
            PlayCategory::GroupSix,
        ],
        LotteryNumberType::FiveDigit => vec![
            PlayCategory::Direct,
            PlayCategory::DirectCombination,
            PlayCategory::GroupThree,
            PlayCategory::GroupSix,
            PlayCategory::BigSmallOddEven,
        ],
        LotteryNumberType::Pk10
        | LotteryNumberType::ElevenFive
        | LotteryNumberType::FastThree
        | LotteryNumberType::LuckTwenty => Vec::new(),
    }
}

/// 返回默认彩种分类配置。
fn lottery_categories() -> Vec<LotteryCategoryConfig> {
    vec![
        LotteryCategoryConfig {
            code: "regional".to_string(),
            name: "地方彩种".to_string(),
        },
        LotteryCategoryConfig {
            code: "overseas".to_string(),
            name: "海外彩种".to_string(),
        },
        LotteryCategoryConfig {
            code: "welfare".to_string(),
            name: "福利彩种".to_string(),
        },
        LotteryCategoryConfig {
            code: "other".to_string(),
            name: "其他".to_string(),
        },
    ]
}

fn normalize_category_config(category: LotteryCategoryConfig) -> ApiResult<LotteryCategoryConfig> {
    let code = category.code.trim().to_string();
    let name = category.name.trim().to_string();

    if code.is_empty() {
        return Err(ApiError::BadRequest(
            "category code is required".to_string(),
        ));
    }
    if name.is_empty() {
        return Err(ApiError::BadRequest(
            "category name is required".to_string(),
        ));
    }

    Ok(LotteryCategoryConfig { code, name })
}

/// 标准化输入并返回规范值。
fn normalize_lottery(mut lottery: LotteryKind) -> ApiResult<LotteryKind> {
    validate_lottery_base(&lottery)?;

    if lottery.draw_mode != DrawMode::Api {
        lottery.api_draw_delay_seconds = 0;
    }
    lottery.issue_format = normalize_issue_format_pattern(&lottery.issue_format)?;

    if !number_type_supports_play_rules(&lottery.number_type) {
        lottery.play_categories = Vec::new();
        lottery.play_configs = Vec::new();
        return Ok(lottery);
    }

    let play_configs = normalize_play_configs(
        &lottery.number_type,
        &lottery.play_categories,
        &lottery.play_configs,
    )?;
    let play_categories = enabled_play_categories(&play_configs);
    if play_categories.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one play config must be enabled".to_string(),
        ));
    }

    lottery.play_categories = play_categories;
    lottery.play_configs = play_configs;

    Ok(lottery)
}

/// 校验输入参数并返回校验结果。
fn validate_lottery_base(lottery: &LotteryKind) -> ApiResult<()> {
    if lottery.id.trim().is_empty() {
        return Err(ApiError::BadRequest("lottery id is required".to_string()));
    }

    if lottery.name.trim().is_empty() {
        return Err(ApiError::BadRequest("lottery name is required".to_string()));
    }

    if lottery.api_draw_delay_seconds > i32::MAX as u32 {
        return Err(ApiError::BadRequest(
            "api draw delay seconds is too large".to_string(),
        ));
    }

    if number_type_supports_play_rules(&lottery.number_type) && lottery.play_categories.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one play category is required".to_string(),
        ));
    }

    match &lottery.schedule {
        DrawSchedule::Periodic { interval_seconds } if *interval_seconds == 0 => {
            return Err(ApiError::BadRequest(
                "periodic interval must be greater than zero".to_string(),
            ));
        }
        DrawSchedule::Daily { time } if time.trim().is_empty() => {
            return Err(ApiError::BadRequest(
                "daily draw time is required".to_string(),
            ));
        }
        DrawSchedule::Weekly { weekdays, time } => {
            if weekdays.is_empty() {
                return Err(ApiError::BadRequest(
                    "weekly draw weekdays are required".to_string(),
                ));
            }
            if time.trim().is_empty() {
                return Err(ApiError::BadRequest(
                    "weekly draw time is required".to_string(),
                ));
            }
        }
        _ => {}
    }

    if lottery.group_buy.min_share_amount_minor <= 0 {
        return Err(ApiError::BadRequest(
            "group buy min share amount must be greater than zero".to_string(),
        ));
    }

    if lottery.group_buy.initiator_min_percent > 100 {
        return Err(ApiError::BadRequest(
            "initiator min percent must be between 0 and 100".to_string(),
        ));
    }

    if lottery.group_buy.participant_min_amount_minor <= 0 {
        return Err(ApiError::BadRequest(
            "participant min amount must be greater than zero".to_string(),
        ));
    }

    Ok(())
}

/// 判断当前号码类型是否已经接入投注玩法和赔率配置。
fn number_type_supports_play_rules(number_type: &LotteryNumberType) -> bool {
    matches!(
        number_type,
        LotteryNumberType::ThreeDigit | LotteryNumberType::FiveDigit
    )
}

/// 标准化输入并返回规范值。
fn normalize_play_configs(
    number_type: &LotteryNumberType,
    play_categories: &[PlayCategory],
    submitted_configs: &[LotteryPlayConfig],
) -> ApiResult<Vec<LotteryPlayConfig>> {
    let mut configs = Vec::new();
    let summaries = play_rule_summaries()
        .into_iter()
        .filter(|summary| summary.number_type == *number_type)
        .collect::<Vec<_>>();

    for submitted in submitted_configs {
        if number_type_for_rule(&submitted.rule_code) != *number_type {
            return Err(ApiError::BadRequest(
                "play config rule does not match lottery number type".to_string(),
            ));
        }
        if submitted.odds_basis_points <= 0 {
            return Err(ApiError::BadRequest(
                "play odds basis points must be greater than zero".to_string(),
            ));
        }
    }

    for summary in summaries {
        let submitted = submitted_configs
            .iter()
            .find(|config| config.rule_code == summary.code);
        let category_enabled = play_categories.contains(&summary.category);
        let odds_basis_points = submitted
            .map(|config| config.odds_basis_points)
            .unwrap_or_else(|| default_odds_basis_points_for_rule(&summary.code));
        let enabled = submitted
            .map(|config| config.enabled && category_enabled)
            .unwrap_or(category_enabled);
        let position_select_limits = submitted
            .map(|config| {
                normalize_position_select_limits(&summary.code, &config.position_select_limits)
            })
            .transpose()?
            .unwrap_or_default();

        configs.push(LotteryPlayConfig {
            rule_code: summary.code,
            enabled,
            odds_basis_points,
            position_select_limits,
        });
    }

    Ok(configs)
}

/// 标准化单玩法各位置选号上限，未配置的位置不限制。
fn normalize_position_select_limits(
    rule_code: &PlayRuleCode,
    limits: &[LotteryPlayPositionSelectLimit],
) -> ApiResult<Vec<LotteryPlayPositionSelectLimit>> {
    let targets = play_position_select_limit_targets(rule_code);
    let mut normalized = Vec::new();

    for (position_key, _) in targets {
        let matching_limits = limits
            .iter()
            .filter(|limit| limit.position_key.trim() == position_key)
            .collect::<Vec<_>>();
        if matching_limits.len() > 1 {
            return Err(ApiError::BadRequest(
                "play position select limit key is duplicated".to_string(),
            ));
        }
        let Some(limit) = matching_limits.first() else {
            continue;
        };
        if limit.max_select_count == 0 {
            return Err(ApiError::BadRequest(
                "play position select limit must be greater than zero".to_string(),
            ));
        }
        normalized.push(LotteryPlayPositionSelectLimit {
            position_key: position_key.to_string(),
            max_select_count: limit.max_select_count,
        });
    }

    for limit in limits {
        let position_key = limit.position_key.trim();
        if position_key.is_empty() {
            return Err(ApiError::BadRequest(
                "play position select limit key is required".to_string(),
            ));
        }
        if !play_position_select_limit_targets(rule_code)
            .iter()
            .any(|(key, _)| *key == position_key)
        {
            return Err(ApiError::BadRequest(
                "play position select limit key is not allowed for this rule".to_string(),
            ));
        }
    }

    Ok(normalized)
}

/// 处理 enabled_play_categories 的具体内部流程。
fn enabled_play_categories(play_configs: &[LotteryPlayConfig]) -> Vec<PlayCategory> {
    let mut categories = Vec::new();
    for config in play_configs.iter().filter(|config| config.enabled) {
        let category = play_category_for_rule(&config.rule_code);
        if !categories.contains(&category) {
            categories.push(category);
        }
    }
    categories
}

/// 处理 play_configs_with_overrides 的具体内部流程。
fn play_configs_with_overrides(
    number_type: LotteryNumberType,
    play_categories: &[PlayCategory],
    overrides: &[(PlayRuleCode, i64)],
) -> Vec<LotteryPlayConfig> {
    play_rule_summaries()
        .into_iter()
        .filter(|summary| summary.number_type == number_type)
        .map(|summary| {
            let odds_basis_points = overrides
                .iter()
                .find(|(rule_code, _)| *rule_code == summary.code)
                .map(|(_, odds)| *odds)
                .unwrap_or_else(|| default_odds_basis_points_for_rule(&summary.code));

            LotteryPlayConfig {
                rule_code: summary.code,
                enabled: play_categories.contains(&summary.category),
                odds_basis_points,
                position_select_limits: Vec::new(),
            }
        })
        .collect()
}

/// 处理 default_odds_basis_points_for_rule 的具体内部流程。
fn default_odds_basis_points_for_rule(rule_code: &PlayRuleCode) -> i64 {
    match play_category_for_rule(rule_code) {
        PlayCategory::Direct | PlayCategory::DirectCombination => 100_000,
        PlayCategory::GroupThree | PlayCategory::GroupSix => 50_000,
        PlayCategory::BigSmallOddEven => 20_000,
    }
}

/// 处理 group_buy_config 的具体内部流程。
fn group_buy_config() -> GroupBuyConfig {
    GroupBuyConfig {
        enabled: false,
        min_share_amount_minor: 100,
        initiator_min_percent: 10,
        participant_min_amount_minor: 1_000,
    }
}

/// 处理 lottery_from_row 的具体内部流程。
fn lottery_from_row(row: sqlx::postgres::PgRow) -> ApiResult<LotteryKind> {
    let number_type = enum_from_string(row.try_get("number_type").map_err(database_error)?)?;
    let draw_mode = enum_from_string(row.try_get("draw_mode").map_err(database_error)?)?;
    let category = enum_from_string(row.try_get("category").map_err(database_error)?)?;
    let logo_url: String = row.try_get("logo_url").map_err(database_error)?;
    let schedule = json_from_value(row.try_get("schedule").map_err(database_error)?)?;
    let group_buy = json_from_value(row.try_get("group_buy").map_err(database_error)?)?;
    let play_categories = json_from_value(row.try_get("play_categories").map_err(database_error)?)?;
    let play_configs = json_from_value(row.try_get("play_configs").map_err(database_error)?)?;

    normalize_lottery(LotteryKind {
        id: row.try_get("id").map_err(database_error)?,
        name: row.try_get("name").map_err(database_error)?,
        category,
        logo_url: logo_url.trim().to_string(),
        number_type,
        draw_mode,
        api_draw_delay_seconds: row
            .try_get::<i32, _>("api_draw_delay_seconds")
            .map_err(database_error)? as u32,
        issue_format: row
            .try_get::<String, _>("issue_format")
            .map_err(database_error)?,
        schedule,
        sale_enabled: row.try_get("sale_enabled").map_err(database_error)?,
        group_buy,
        play_categories,
        play_configs,
    })
}

/// 处理 enum_value 的具体内部流程。
fn enum_value<T: Serialize>(value: &T) -> ApiResult<String> {
    let value = serde_json::to_value(value).map_err(serde_error)?;

    value.as_str().map(ToString::to_string).ok_or_else(|| {
        tracing::error!("彩种枚举没有序列化为字符串");
        ApiError::Internal("彩种枚举序列化失败".to_string())
    })
}

/// 处理 enum_from_string 的具体内部流程。
fn enum_from_string<T: DeserializeOwned>(value: String) -> ApiResult<T> {
    serde_json::from_value(Value::String(value)).map_err(|error| {
        tracing::error!(
            error = %error,
            "数据库中的彩种枚举无效"
        );
        ApiError::Internal("数据库中的彩种枚举无效".to_string())
    })
}

/// 处理 json_value 的具体内部流程。
fn json_value<T: Serialize>(value: &T) -> ApiResult<Value> {
    serde_json::to_value(value).map_err(serde_error)
}

/// 处理 json_from_value 的具体内部流程。
fn json_from_value<T: DeserializeOwned>(value: Value) -> ApiResult<T> {
    serde_json::from_value(value).map_err(|error| {
        tracing::error!(
            error = %error,
            "数据库中的彩种 JSON 无效"
        );
        ApiError::Internal("数据库中的彩种 JSON 无效".to_string())
    })
}

/// 处理 serde_error 的具体内部流程。
fn serde_error(error: serde_json::Error) -> ApiError {
    tracing::error!(error = %error, "彩种 JSON 序列化失败");
    ApiError::Internal("彩种 JSON 序列化失败".to_string())
}

/// 处理 database_error 的具体内部流程。
fn database_error(error: sqlx::Error) -> ApiError {
    tracing::error!(error = %error, "彩种数据库操作失败");
    ApiError::Internal("彩种数据库操作失败".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        enum_from_string, enum_value, json_from_value, json_value, seed_lotteries,
        LotteryRepository, LotteryStore,
    };
    use crate::domain::lottery::{DrawMode, DrawSchedule, LotteryKind, LotteryNumberType};

    #[test]
    /// 处理 store_creates_and_lists_lottery 的具体内部流程。
    fn store_creates_and_lists_lottery() {
        let mut store = LotteryStore::seeded();
        let mut lottery = seed_lotteries()
            .into_iter()
            .find(|item| item.id == "fc3d")
            .expect("seed lottery exists");
        lottery.id = "new3d".to_string();
        lottery.name = "新 3D".to_string();

        store.create(lottery).expect("lottery can be created");

        assert!(store.list().iter().any(|item| item.id == "new3d"));
    }

    #[test]
    /// 处理 seeded_lotteries_include_au5_api_lottery 的具体内部流程。
    fn seeded_lotteries_include_au5_api_lottery() {
        let lottery = seed_lotteries()
            .into_iter()
            .find(|item| item.id == "au5")
            .expect("au5 seed lottery exists");

        assert_eq!(lottery.name, "澳洲幸运5");
        assert_eq!(lottery.number_type, LotteryNumberType::FiveDigit);
        assert_eq!(lottery.draw_mode, DrawMode::Api);
        assert_eq!(
            lottery.schedule,
            DrawSchedule::Periodic {
                interval_seconds: 300
            }
        );
    }

    #[test]
    /// 内置彩种包含用户要求新增的 API68 彩种和正确号码类型。
    fn seeded_lotteries_include_requested_api68_lotteries() {
        let lotteries = seed_lotteries();

        for (id, number_type, has_play_rules) in [
            ("bjpk10", LotteryNumberType::Pk10, false),
            ("tjssc", LotteryNumberType::FiveDigit, true),
            ("gd11x5", LotteryNumberType::ElevenFive, false),
            ("au10", LotteryNumberType::Pk10, false),
            ("au20", LotteryNumberType::LuckTwenty, false),
            ("zj11x5", LotteryNumberType::ElevenFive, false),
        ] {
            let lottery = lotteries
                .iter()
                .find(|item| item.id == id)
                .expect("seed lottery exists");

            assert_eq!(lottery.number_type, number_type);
            assert_eq!(lottery.draw_mode, DrawMode::Api);
            assert_eq!(lottery.play_configs.is_empty(), !has_play_rules);
        }

        for removed_id in [
            "jsk3", "gxk3", "jlk3", "hebk3", "nmgk3", "ahk3", "fjk3", "hubk3", "bjk3", "bjkl8",
        ] {
            assert!(
                !lotteries.iter().any(|item| item.id == removed_id),
                "已停用的 API68 彩种不应继续出现在默认种子中"
            );
        }
    }

    #[test]
    /// 处理 seeded_lotteries_include_txffc_api_lottery 的具体内部流程。
    fn seeded_lotteries_include_txffc_api_lottery() {
        let lottery = seed_lotteries()
            .into_iter()
            .find(|item| item.id == "txffc")
            .expect("txffc seed lottery exists");

        assert_eq!(lottery.name, "腾讯分分彩");
        assert_eq!(lottery.number_type, LotteryNumberType::FiveDigit);
        assert_eq!(lottery.draw_mode, DrawMode::Api);
        assert_eq!(
            lottery.schedule,
            DrawSchedule::Periodic {
                interval_seconds: 60
            }
        );
    }

    #[test]
    /// 内置彩种默认停售，并默认关闭合买。
    fn seeded_lotteries_default_to_closed_sale_and_group_buy() {
        let lotteries = seed_lotteries();

        assert!(lotteries.iter().all(|lottery| !lottery.sale_enabled));
        assert!(lotteries.iter().all(|lottery| !lottery.group_buy.enabled));
    }

    #[test]
    /// 处理 store_rejects_duplicate_id 的具体内部流程。
    fn store_rejects_duplicate_id() {
        let mut store = LotteryStore::seeded();
        let lottery = store.get("fc3d").expect("seed lottery exists");

        let error = store.create(lottery).expect_err("duplicate id rejected");

        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    /// 处理 store_rejects_invalid_periodic_schedule 的具体内部流程。
    fn store_rejects_invalid_periodic_schedule() {
        let mut store = LotteryStore::seeded();
        let mut lottery: LotteryKind = store.get("ssc60").expect("seed lottery exists");
        lottery.id = "bad-period".to_string();
        lottery.schedule = DrawSchedule::Periodic {
            interval_seconds: 0,
        };

        let error = store
            .create(lottery)
            .expect_err("zero interval should be rejected");

        assert!(error.to_string().contains("greater than zero"));
    }

    #[test]
    /// 处理 store_toggles_sale_status 的具体内部流程。
    fn store_toggles_sale_status() {
        let mut store = LotteryStore::seeded();

        let updated = store
            .set_sale_enabled("fc3d", true)
            .expect("sale status can be changed");

        assert!(updated.sale_enabled);
    }

    #[test]
    /// 处理 lottery_database_values_use_frontend_contract_names 的具体内部流程。
    fn lottery_database_values_use_frontend_contract_names() {
        let lottery = seed_lotteries()
            .into_iter()
            .find(|item| item.id == "ssc60")
            .expect("seed lottery exists");

        assert_eq!(enum_value(&lottery.number_type).unwrap(), "fiveDigit");
        assert_eq!(enum_value(&lottery.draw_mode).unwrap(), "platform");

        let schedule = json_value(&lottery.schedule).unwrap();
        assert_eq!(schedule["periodic"]["intervalSeconds"], 60);
    }

    #[test]
    /// 新增外部彩种号码类型需要继续使用前端契约名称落库。
    fn lottery_number_type_database_values_include_external_lottery_types() {
        assert_eq!(enum_value(&LotteryNumberType::Pk10).unwrap(), "pk10");
        assert_eq!(
            enum_value(&LotteryNumberType::ElevenFive).unwrap(),
            "elevenFive"
        );
        assert_eq!(
            enum_value(&LotteryNumberType::FastThree).unwrap(),
            "fastThree"
        );
        assert_eq!(
            enum_value(&LotteryNumberType::LuckTwenty).unwrap(),
            "luckTwenty"
        );

        let number_type: LotteryNumberType = enum_from_string("luckTwenty".to_string()).unwrap();
        assert_eq!(number_type, LotteryNumberType::LuckTwenty);
    }

    #[test]
    /// 处理 lottery_database_values_round_trip_to_domain_types 的具体内部流程。
    fn lottery_database_values_round_trip_to_domain_types() {
        let draw_mode: DrawMode = enum_from_string("manual".to_string()).unwrap();
        let schedule: DrawSchedule = json_from_value(serde_json::json!({
            "weekly": {
                "weekdays": ["Tuesday", "Thursday"],
                "time": "21:00:00"
            }
        }))
        .unwrap();

        assert_eq!(draw_mode, DrawMode::Manual);
        assert_eq!(
            schedule,
            DrawSchedule::Weekly {
                weekdays: vec!["Tuesday".to_string(), "Thursday".to_string()],
                time: "21:00:00".to_string()
            }
        );
    }

    #[tokio::test]
    async fn repository_uses_seeded_memory_lotteries() {
        let repository = LotteryRepository::memory_seeded();

        let lotteries = repository.list().await.expect("lotteries can be listed");

        assert_eq!(lotteries.len(), 22);
    }

    #[tokio::test]
    async fn postgres_repository_smoke_when_test_database_is_configured() {
        let Ok(database_url) = std::env::var("BC_TEST_DATABASE_URL") else {
            return;
        };

        let repository = LotteryRepository::postgres(&database_url)
            .await
            .expect("postgres repository can connect");
        let mut lottery = seed_lotteries()
            .into_iter()
            .find(|item| item.id == "fc3d")
            .expect("seed lottery exists");
        lottery.id = "integration-smoke-3d".to_string();
        lottery.name = "集成测试 3D".to_string();

        let _ = repository.delete(&lottery.id).await;
        let created = repository
            .create(lottery.clone())
            .await
            .expect("lottery can be created");
        let toggled = repository
            .set_sale_enabled(&created.id, true)
            .await
            .expect("sale status can be toggled");
        let deleted = repository
            .delete(&created.id)
            .await
            .expect("lottery can be deleted");

        assert_eq!(created.id, "integration-smoke-3d");
        assert!(toggled.sale_enabled);
        assert_eq!(deleted.id, "integration-smoke-3d");
    }
}
