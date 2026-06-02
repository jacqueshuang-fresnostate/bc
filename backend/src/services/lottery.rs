use std::{collections::BTreeMap, error::Error, sync::Arc};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::{
    domain::lottery::{
        DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType, PlayCategory,
    },
    error::{ApiError, ApiResult},
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
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(LotteryRepositoryKind::Memory(std::sync::RwLock::new(
                LotteryStore::seeded(),
            ))),
        }
    }

    pub async fn postgres(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        let store = PostgresLotteryStore { pool };
        store.seed_if_empty().await?;

        Ok(Self {
            inner: Arc::new(LotteryRepositoryKind::Postgres(store)),
        })
    }

    pub async fn list(&self) -> ApiResult<Vec<LotteryKind>> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .read()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))
                .map(|store| store.list()),
            LotteryRepositoryKind::Postgres(store) => store.list().await,
        }
    }

    pub async fn get(&self, id: &str) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .read()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .get(id),
            LotteryRepositoryKind::Postgres(store) => store.get(id).await,
        }
    }

    pub async fn create(&self, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .create(lottery),
            LotteryRepositoryKind::Postgres(store) => store.create(lottery).await,
        }
    }

    pub async fn update(&self, id: &str, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .update(id, lottery),
            LotteryRepositoryKind::Postgres(store) => store.update(id, lottery).await,
        }
    }

    pub async fn delete(&self, id: &str) -> ApiResult<LotteryKind> {
        match self.inner.as_ref() {
            LotteryRepositoryKind::Memory(store) => store
                .write()
                .map_err(|_| ApiError::Internal("lottery store lock poisoned".to_string()))?
                .delete(id),
            LotteryRepositoryKind::Postgres(store) => store.delete(id).await,
        }
    }

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
    pub fn seeded() -> Self {
        Self::from_lotteries(seed_lotteries())
    }

    fn from_lotteries(lotteries: Vec<LotteryKind>) -> Self {
        Self {
            lotteries: lotteries
                .into_iter()
                .map(|lottery| (lottery.id.clone(), lottery))
                .collect(),
        }
    }

    pub fn list(&self) -> Vec<LotteryKind> {
        self.lotteries.values().cloned().collect()
    }

    pub fn get(&self, id: &str) -> ApiResult<LotteryKind> {
        self.lotteries
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

    pub fn create(&mut self, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        validate_lottery(&lottery)?;

        if self.lotteries.contains_key(&lottery.id) {
            return Err(ApiError::Conflict(format!(
                "lottery `{}` already exists",
                lottery.id
            )));
        }

        self.lotteries.insert(lottery.id.clone(), lottery.clone());
        Ok(lottery)
    }

    pub fn update(&mut self, id: &str, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        validate_lottery(&lottery)?;

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

    pub fn delete(&mut self, id: &str) -> ApiResult<LotteryKind> {
        self.lotteries
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("lottery `{id}` not found")))
    }

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
    async fn seed_if_empty(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM lotteries")
            .fetch_one(&self.pool)
            .await?;

        if count > 0 {
            return Ok(());
        }

        for lottery in seed_lotteries() {
            self.insert_lottery(lottery).await?;
        }

        Ok(())
    }

    async fn list(&self) -> ApiResult<Vec<LotteryKind>> {
        let rows = sqlx::query(
            "SELECT id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories
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
            "SELECT id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories
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
        validate_lottery(&lottery)?;

        let created = sqlx::query(
            "INSERT INTO lotteries (
                id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO NOTHING
             RETURNING id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories",
        )
        .bind(&lottery.id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
        .bind(json_value(&lottery.schedule)?)
        .bind(lottery.sale_enabled)
        .bind(json_value(&lottery.group_buy)?)
        .bind(json_value(&lottery.play_categories)?)
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?;

        created
            .map(lottery_from_row)
            .transpose()?
            .ok_or_else(|| ApiError::Conflict(format!("lottery `{}` already exists", lottery.id)))
    }

    async fn insert_lottery(&self, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        let row = sqlx::query(
            "INSERT INTO lotteries (
                id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories",
        )
        .bind(&lottery.id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
        .bind(json_value(&lottery.schedule)?)
        .bind(lottery.sale_enabled)
        .bind(json_value(&lottery.group_buy)?)
        .bind(json_value(&lottery.play_categories)?)
        .fetch_one(&self.pool)
        .await
        .map_err(database_error)?;

        lottery_from_row(row)
    }

    async fn update(&self, id: &str, lottery: LotteryKind) -> ApiResult<LotteryKind> {
        validate_lottery(&lottery)?;

        if id != lottery.id {
            return Err(ApiError::BadRequest(
                "path id must match lottery id".to_string(),
            ));
        }

        let updated = sqlx::query(
            "UPDATE lotteries
             SET name = $2,
                 number_type = $3,
                 draw_mode = $4,
                 schedule = $5,
                 sale_enabled = $6,
                 group_buy = $7,
                 play_categories = $8,
                 updated_at = now()
             WHERE id = $1
             RETURNING id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories",
        )
        .bind(id)
        .bind(&lottery.name)
        .bind(enum_value(&lottery.number_type)?)
        .bind(enum_value(&lottery.draw_mode)?)
        .bind(json_value(&lottery.schedule)?)
        .bind(lottery.sale_enabled)
        .bind(json_value(&lottery.group_buy)?)
        .bind(json_value(&lottery.play_categories)?)
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
             RETURNING id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories",
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
             RETURNING id, name, number_type, draw_mode, schedule, sale_enabled, group_buy, play_categories",
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

pub fn seed_lotteries() -> Vec<LotteryKind> {
    vec![
        LotteryKind {
            id: "fc3d".to_string(),
            name: "福彩 3D".to_string(),
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
        },
        LotteryKind {
            id: "pl3".to_string(),
            name: "排列 3".to_string(),
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
        },
        LotteryKind {
            id: "ssc60".to_string(),
            name: "60 秒时时彩".to_string(),
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
        },
        LotteryKind {
            id: "manual-test".to_string(),
            name: "指定号码测试彩".to_string(),
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
        },
    ]
}

fn validate_lottery(lottery: &LotteryKind) -> ApiResult<()> {
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

fn group_buy_config() -> GroupBuyConfig {
    GroupBuyConfig {
        enabled: true,
        min_share_amount_minor: 100,
        initiator_min_percent: 10,
        participant_min_amount_minor: 1_000,
    }
}

fn lottery_from_row(row: sqlx::postgres::PgRow) -> ApiResult<LotteryKind> {
    let number_type = enum_from_string(row.try_get("number_type").map_err(database_error)?)?;
    let draw_mode = enum_from_string(row.try_get("draw_mode").map_err(database_error)?)?;
    let schedule = json_from_value(row.try_get("schedule").map_err(database_error)?)?;
    let group_buy = json_from_value(row.try_get("group_buy").map_err(database_error)?)?;
    let play_categories = json_from_value(row.try_get("play_categories").map_err(database_error)?)?;

    Ok(LotteryKind {
        id: row.try_get("id").map_err(database_error)?,
        name: row.try_get("name").map_err(database_error)?,
        number_type,
        draw_mode,
        schedule,
        sale_enabled: row.try_get("sale_enabled").map_err(database_error)?,
        group_buy,
        play_categories,
    })
}

fn enum_value<T: Serialize>(value: &T) -> ApiResult<String> {
    let value = serde_json::to_value(value).map_err(serde_error)?;

    value.as_str().map(ToString::to_string).ok_or_else(|| {
        tracing::error!("lottery enum did not serialize to string");
        ApiError::Internal("lottery enum serialization failed".to_string())
    })
}

fn enum_from_string<T: DeserializeOwned>(value: String) -> ApiResult<T> {
    serde_json::from_value(Value::String(value)).map_err(|error| {
        tracing::error!(%error, "invalid lottery enum in database");
        ApiError::Internal("invalid lottery data in database".to_string())
    })
}

fn json_value<T: Serialize>(value: &T) -> ApiResult<Value> {
    serde_json::to_value(value).map_err(serde_error)
}

fn json_from_value<T: DeserializeOwned>(value: Value) -> ApiResult<T> {
    serde_json::from_value(value).map_err(|error| {
        tracing::error!(%error, "invalid lottery json in database");
        ApiError::Internal("invalid lottery data in database".to_string())
    })
}

fn serde_error(error: serde_json::Error) -> ApiError {
    tracing::error!(%error, "lottery json serialization failed");
    ApiError::Internal("lottery data serialization failed".to_string())
}

fn database_error(error: sqlx::Error) -> ApiError {
    tracing::error!(%error, "lottery database operation failed");
    ApiError::Internal("lottery database operation failed".to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        enum_from_string, enum_value, json_from_value, json_value, seed_lotteries,
        LotteryRepository, LotteryStore,
    };
    use crate::domain::lottery::{DrawMode, DrawSchedule, LotteryKind};

    #[test]
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
    fn store_rejects_duplicate_id() {
        let mut store = LotteryStore::seeded();
        let lottery = store.get("fc3d").expect("seed lottery exists");

        let error = store.create(lottery).expect_err("duplicate id rejected");

        assert!(error.to_string().contains("already exists"));
    }

    #[test]
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
    fn store_toggles_sale_status() {
        let mut store = LotteryStore::seeded();

        let updated = store
            .set_sale_enabled("fc3d", false)
            .expect("sale status can be changed");

        assert!(!updated.sale_enabled);
    }

    #[test]
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

        assert_eq!(lotteries.len(), 4);
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
