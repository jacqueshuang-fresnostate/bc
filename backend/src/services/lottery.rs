//! 彩种与开奖来源领域模型，定义开奖方式和销售与合买配置

use std::{collections::BTreeMap, error::Error, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::{
    domain::{
        lottery::{
            DrawMode, DrawSchedule, GroupBuyConfig, LotteryCategory, LotteryKind,
            LotteryNumberType, LotteryPlayConfig, PlayCategory,
        },
        play::PlayRuleCode,
    },
    error::{ApiError, ApiResult},
    services::play_rules::{number_type_for_rule, play_category_for_rule, play_rule_summaries},
};

#[derive(Clone)]
pub struct LotteryRepository {
    inner: Arc<LotteryRepositoryKind>,
}

enum LotteryRepositoryKind {
    Memory(std::sync::RwLock<LotteryStore>),
    Postgres(PostgresLotteryStore),
}

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
}

#[derive(Debug, Clone)]
pub struct LotteryStore {
    lotteries: BTreeMap<String, LotteryKind>,
}

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
        }
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
}

struct PostgresLotteryStore {
    pool: PgPool,
}

impl PostgresLotteryStore {
    async fn seed_missing_defaults(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        for lottery in seed_lotteries() {
            self.insert_seed_lottery(lottery).await?;
        }

        Ok(())
    }

    async fn list(&self) -> ApiResult<Vec<LotteryKind>> {
        let rows = sqlx::query(
            "SELECT id, name, category, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories, play_configs
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
            "SELECT id, name, category, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories, play_configs
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

        let created = sqlx::query(
            "INSERT INTO lotteries (
                id, name, category, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories, play_configs
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (id) DO NOTHING
             RETURNING id, name, category, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories, play_configs",
        )
        .bind(&lottery.id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.category)?)
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
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
                id, name, category, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories, play_configs
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&lottery.id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.category)?)
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
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

        if id != lottery.id {
            return Err(ApiError::BadRequest(
                "path id must match lottery id".to_string(),
            ));
        }

        let updated = sqlx::query(
            "UPDATE lotteries
             SET name = $2,
                 category = $3,
                 number_type = $4,
                 draw_mode = $5,
                 schedule = $6,
                 sale_enabled = $7,
                 group_buy = $8,
                 play_categories = $9,
                 play_configs = $10,
                 updated_at = now()
             WHERE id = $1
             RETURNING id, name, category, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories, play_configs",
        )
        .bind(id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.category)?)
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
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
             RETURNING id, name, category, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories, play_configs",
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
             RETURNING id, name, category, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories, play_configs",
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
    vec![
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
            category: LotteryCategory::Regional,
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: true,
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
            category: LotteryCategory::Regional,
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Api,
            schedule: DrawSchedule::Daily {
                time: "21:00:15".to_string(),
            },
            sale_enabled: true,
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
            name: "澳洲 5 分彩".to_string(),
            category: LotteryCategory::Overseas,
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Api,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 300,
            },
            sale_enabled: true,
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
            category: LotteryCategory::Overseas,
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Api,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: true,
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
            category: LotteryCategory::Overseas,
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Platform,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: true,
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
            category: LotteryCategory::Other,
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Manual,
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
    ]
}

/// 标准化输入并返回规范值。
fn normalize_lottery(mut lottery: LotteryKind) -> ApiResult<LotteryKind> {
    validate_lottery_base(&lottery)?;

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

    if lottery.play_categories.is_empty() {
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

        configs.push(LotteryPlayConfig {
            rule_code: summary.code,
            enabled,
            odds_basis_points,
        });
    }

    Ok(configs)
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
        enabled: true,
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
    let schedule = json_from_value(row.try_get("schedule").map_err(database_error)?)?;
    let group_buy = json_from_value(row.try_get("group_buy").map_err(database_error)?)?;
    let play_categories = json_from_value(row.try_get("play_categories").map_err(database_error)?)?;
    let play_configs = json_from_value(row.try_get("play_configs").map_err(database_error)?)?;

    normalize_lottery(LotteryKind {
        id: row.try_get("id").map_err(database_error)?,
        name: row.try_get("name").map_err(database_error)?,
        category,
        number_type,
        draw_mode,
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
    serde_json::from_value(Value::String(value)).map_err(|_| {
        tracing::error!(
            error = "错误详情已按中文日志规则隐藏",
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
    serde_json::from_value(value).map_err(|_| {
        tracing::error!(
            error = "错误详情已按中文日志规则隐藏",
            "数据库中的彩种 JSON 无效"
        );
        ApiError::Internal("数据库中的彩种 JSON 无效".to_string())
    })
}

/// 处理 serde_error 的具体内部流程。
fn serde_error(_: serde_json::Error) -> ApiError {
    tracing::error!(
        error = "错误详情已按中文日志规则隐藏",
        "彩种 JSON 序列化失败"
    );
    ApiError::Internal("彩种 JSON 序列化失败".to_string())
}

/// 处理 database_error 的具体内部流程。
fn database_error(_: sqlx::Error) -> ApiError {
    tracing::error!(error = "错误详情已按中文日志规则隐藏", "彩种数据库操作失败");
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

        assert_eq!(lottery.name, "澳洲 5 分彩");
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
            .set_sale_enabled("fc3d", false)
            .expect("sale status can be changed");

        assert!(!updated.sale_enabled);
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

        assert_eq!(lotteries.len(), 6);
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
            .set_sale_enabled(&created.id, false)
            .await
            .expect("sale status can be toggled");
        let deleted = repository
            .delete(&created.id)
            .await
            .expect("lottery can be deleted");

        assert_eq!(created.id, "integration-smoke-3d");
        assert!(!toggled.sale_enabled);
        assert_eq!(deleted.id, "integration-smoke-3d");
    }
}
