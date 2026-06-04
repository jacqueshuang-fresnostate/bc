//! 广告服务层，负责后台广告配置持久化和手机端轮播筛选。

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
};

use chrono::{Local, NaiveDateTime};
use sqlx::Row;

use crate::{
    domain::advertisement::{
        AdvertisementPlacement, AdvertisementStatus, AdvertisementSummary, MobileAdvertisement,
        SaveAdvertisementRequest,
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Clone)]
pub struct AdvertisementRepository {
    inner: Arc<RwLock<AdvertisementStore>>,
    persistence: Option<BusinessDatabase>,
}

impl AdvertisementRepository {
    /// 返回空的内存广告仓储，适合无数据库本地预览。
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AdvertisementStore::default())),
            persistence: None,
        }
    }

    /// 从数据库加载广告配置并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_advertisement_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回后台广告列表，按展示顺序排序。
    pub async fn list(&self) -> ApiResult<Vec<AdvertisementSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("广告仓储锁读取失败".to_string()))
            .map(|store| store.list())
    }

    /// 返回当前可展示给手机端的轮播广告。
    pub async fn list_mobile_carousel(&self) -> ApiResult<Vec<MobileAdvertisement>> {
        let now = Local::now().naive_local();
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("广告仓储锁读取失败".to_string()))
            .map(|store| store.list_mobile_carousel(now))
    }

    /// 按 ID 查询后台广告详情。
    pub async fn get(&self, id: &str) -> ApiResult<AdvertisementSummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("广告仓储锁读取失败".to_string()))?
            .get(id)
    }

    /// 创建广告配置并持久化。
    pub async fn create(
        &self,
        payload: SaveAdvertisementRequest,
    ) -> ApiResult<AdvertisementSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("广告仓储锁写入失败".to_string()))?;
            let result = store.create(payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 更新广告配置并持久化。
    pub async fn update(
        &self,
        id: &str,
        payload: SaveAdvertisementRequest,
    ) -> ApiResult<AdvertisementSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("广告仓储锁写入失败".to_string()))?;
            let result = store.update(id, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 删除广告配置并持久化。
    pub async fn delete(&self, id: &str) -> ApiResult<AdvertisementSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("广告仓储锁写入失败".to_string()))?;
            let result = store.delete(id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 在数据库模式下保存广告仓储快照。
    async fn persist(&self, store: &AdvertisementStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_advertisement_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
struct AdvertisementStore {
    advertisements: BTreeMap<String, AdvertisementSummary>,
}

impl AdvertisementStore {
    /// 返回按排序号和 ID 排列的后台广告列表。
    fn list(&self) -> Vec<AdvertisementSummary> {
        let mut advertisements = self.advertisements.values().cloned().collect::<Vec<_>>();
        advertisements.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then_with(|| left.id.cmp(&right.id))
        });
        advertisements
    }

    /// 返回当前启用且在有效期内的手机端轮播广告。
    fn list_mobile_carousel(&self, now: NaiveDateTime) -> Vec<MobileAdvertisement> {
        self.list()
            .into_iter()
            .filter(|advertisement| advertisement.is_active_mobile(now))
            .map(|advertisement| advertisement.public_mobile())
            .collect()
    }

    /// 按 ID 查询广告配置。
    fn get(&self, id: &str) -> ApiResult<AdvertisementSummary> {
        self.advertisements
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("advertisement `{id}` not found")))
    }

    /// 创建广告配置，空 ID 会自动生成。
    fn create(&mut self, payload: SaveAdvertisementRequest) -> ApiResult<AdvertisementSummary> {
        let now = now_string();
        let advertisement = normalize_advertisement(
            payload,
            None,
            &self.used_ids(),
            self.next_advertisement_id(),
            &now,
        )?;
        if self.advertisements.contains_key(&advertisement.id) {
            return Err(ApiError::Conflict(format!(
                "advertisement `{}` already exists",
                advertisement.id
            )));
        }

        self.advertisements
            .insert(advertisement.id.clone(), advertisement.clone());
        Ok(advertisement)
    }

    /// 更新广告配置，路径 ID 和请求 ID 必须一致。
    fn update(
        &mut self,
        id: &str,
        payload: SaveAdvertisementRequest,
    ) -> ApiResult<AdvertisementSummary> {
        let current = self.get(id)?;
        let now = now_string();
        let advertisement = normalize_advertisement(
            payload,
            Some(&current),
            &self.used_ids(),
            id.to_string(),
            &now,
        )?;
        if advertisement.id != id {
            return Err(ApiError::BadRequest(
                "path id must match advertisement id".to_string(),
            ));
        }

        self.advertisements
            .insert(id.to_string(), advertisement.clone());
        Ok(advertisement)
    }

    /// 删除广告配置。
    fn delete(&mut self, id: &str) -> ApiResult<AdvertisementSummary> {
        self.advertisements
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("advertisement `{id}` not found")))
    }

    /// 返回当前已使用广告 ID 集合。
    fn used_ids(&self) -> BTreeSet<String> {
        self.advertisements.keys().cloned().collect()
    }

    /// 按现有 ID 自动生成下一条广告 ID。
    fn next_advertisement_id(&self) -> String {
        let max_number = self
            .advertisements
            .keys()
            .filter_map(|id| id.strip_prefix("AD"))
            .filter_map(|number| number.parse::<u32>().ok())
            .max()
            .unwrap_or(0);
        format!("AD{:06}", max_number.saturating_add(1))
    }
}

impl AdvertisementSummary {
    /// 判断当前广告是否应出现在手机端轮播中。
    fn is_active_mobile(&self, now: NaiveDateTime) -> bool {
        if self.placement != AdvertisementPlacement::MobileCarousel
            || self.status != AdvertisementStatus::Enabled
        {
            return false;
        }
        if self
            .start_at
            .as_deref()
            .and_then(parse_time)
            .is_some_and(|start_at| start_at > now)
        {
            return false;
        }
        if self
            .end_at
            .as_deref()
            .and_then(parse_time)
            .is_some_and(|end_at| end_at < now)
        {
            return false;
        }

        true
    }
}

/// 从数据库加载广告配置。
async fn load_advertisement_store(database: &BusinessDatabase) -> ApiResult<AdvertisementStore> {
    let mut advertisements = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, title, image_url, link_url, placement, status, sort_order, start_at, end_at, created_at, updated_at
         FROM advertisements
         ORDER BY sort_order ASC, id ASC",
    )
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?;
        advertisements.insert(
            id.clone(),
            AdvertisementSummary {
                id,
                title: row
                    .try_get("title")
                    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                image_url: row
                    .try_get("image_url")
                    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                link_url: row
                    .try_get("link_url")
                    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                placement: enum_from_string(
                    row.try_get("placement")
                        .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                )?,
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                )?,
                sort_order: row
                    .try_get("sort_order")
                    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                start_at: row
                    .try_get("start_at")
                    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                end_at: row
                    .try_get("end_at")
                    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|_| ApiError::Internal("广告配置数据读取失败".to_string()))?,
            },
        );
    }

    Ok(AdvertisementStore { advertisements })
}

/// 保存广告仓储快照到数据库。
async fn save_advertisement_store(
    database: &BusinessDatabase,
    store: &AdvertisementStore,
) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("广告事务开启失败".to_string()))?;

    sqlx::query("DELETE FROM advertisements")
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("广告配置数据清理失败".to_string()))?;

    for advertisement in store.list() {
        sqlx::query(
            "INSERT INTO advertisements (
                id, title, image_url, link_url, placement, status, sort_order, start_at, end_at, created_at, updated_at
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(&advertisement.id)
        .bind(&advertisement.title)
        .bind(&advertisement.image_url)
        .bind(&advertisement.link_url)
        .bind(enum_to_string(&advertisement.placement)?)
        .bind(enum_to_string(&advertisement.status)?)
        .bind(advertisement.sort_order)
        .bind(&advertisement.start_at)
        .bind(&advertisement.end_at)
        .bind(&advertisement.created_at)
        .bind(&advertisement.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("广告配置数据保存失败".to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("广告事务提交失败".to_string()))
}

/// 标准化广告保存请求并返回完整广告配置。
fn normalize_advertisement(
    payload: SaveAdvertisementRequest,
    current: Option<&AdvertisementSummary>,
    used_ids: &BTreeSet<String>,
    fallback_id: String,
    now: &str,
) -> ApiResult<AdvertisementSummary> {
    let id = optional_trimmed(payload.id).unwrap_or(fallback_id);
    if id.is_empty() {
        return Err(ApiError::BadRequest(
            "advertisement id is required".to_string(),
        ));
    }
    if current.is_none() && used_ids.contains(&id) {
        return Err(ApiError::Conflict(format!(
            "advertisement `{id}` already exists"
        )));
    }
    if payload.sort_order < 0 {
        return Err(ApiError::BadRequest(
            "advertisement sort order must be greater than or equal to zero".to_string(),
        ));
    }

    let start_at = normalize_optional_time(payload.start_at, "advertisement start time")?;
    let end_at = normalize_optional_time(payload.end_at, "advertisement end time")?;
    if let (Some(start_at_value), Some(end_at_value)) = (&start_at, &end_at) {
        let start_time = parse_time(start_at_value).ok_or_else(|| {
            ApiError::BadRequest("advertisement start time format is invalid".to_string())
        })?;
        let end_time = parse_time(end_at_value).ok_or_else(|| {
            ApiError::BadRequest("advertisement end time format is invalid".to_string())
        })?;
        if end_time < start_time {
            return Err(ApiError::BadRequest(
                "advertisement end time must be after start time".to_string(),
            ));
        }
    }

    Ok(AdvertisementSummary {
        id,
        title: required_trimmed(payload.title, "advertisement title")?,
        image_url: required_trimmed(payload.image_url, "advertisement image url")?,
        link_url: optional_trimmed(payload.link_url),
        placement: payload.placement,
        status: payload.status,
        sort_order: payload.sort_order,
        start_at,
        end_at,
        created_at: current
            .map(|advertisement| advertisement.created_at.clone())
            .unwrap_or_else(|| now.to_string()),
        updated_at: now.to_string(),
    })
}

/// 校验并规范化可选时间字符串。
fn normalize_optional_time(value: Option<String>, label: &str) -> ApiResult<Option<String>> {
    let Some(value) = optional_trimmed(value) else {
        return Ok(None);
    };
    parse_time(&value).ok_or_else(|| ApiError::BadRequest(format!("{label} format is invalid")))?;
    Ok(Some(value))
}

/// 解析后台约定的时间字符串。
fn parse_time(value: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(value, TIMESTAMP_FORMAT).ok()
}

/// 去除空白并校验必填字段。
fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

/// 去除可选字符串空白，空字符串视为未配置。
fn optional_trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// 返回当前本地时间字符串。
fn now_string() -> String {
    Local::now().format(TIMESTAMP_FORMAT).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save_payload(id: Option<&str>, title: &str, sort_order: i32) -> SaveAdvertisementRequest {
        SaveAdvertisementRequest {
            id: id.map(str::to_string),
            title: title.to_string(),
            image_url: format!("https://example.test/{title}.png"),
            link_url: Some("https://example.test/activity".to_string()),
            placement: AdvertisementPlacement::MobileCarousel,
            status: AdvertisementStatus::Enabled,
            sort_order,
            start_at: None,
            end_at: None,
        }
    }

    #[tokio::test]
    async fn advertisement_repository_creates_updates_and_deletes_advertisement() {
        let repository = AdvertisementRepository::memory();
        let created = repository
            .create(save_payload(None, "首屏轮播", 10))
            .await
            .expect("advertisement can be created");

        assert_eq!(created.id, "AD000001");
        assert_eq!(created.title, "首屏轮播");

        let mut update_payload = save_payload(Some(&created.id), "活动轮播", 5);
        update_payload.status = AdvertisementStatus::Disabled;
        let updated = repository
            .update(&created.id, update_payload)
            .await
            .expect("advertisement can be updated");

        assert_eq!(updated.title, "活动轮播");
        assert_eq!(updated.status, AdvertisementStatus::Disabled);

        let deleted = repository
            .delete(&created.id)
            .await
            .expect("advertisement can be deleted");
        assert_eq!(deleted.id, created.id);
        assert!(repository
            .list()
            .await
            .expect("list can be loaded")
            .is_empty());
    }

    #[tokio::test]
    async fn mobile_carousel_only_returns_enabled_and_active_items() {
        let repository = AdvertisementRepository::memory();
        let active = repository
            .create(save_payload(Some("AD000010"), "启用", 20))
            .await
            .expect("active advertisement can be created");
        let mut disabled_payload = save_payload(Some("AD000011"), "停用", 10);
        disabled_payload.status = AdvertisementStatus::Disabled;
        repository
            .create(disabled_payload)
            .await
            .expect("disabled advertisement can be created");
        let mut future_payload = save_payload(Some("AD000012"), "未开始", 30);
        future_payload.start_at = Some("2999-01-01 00:00:00".to_string());
        repository
            .create(future_payload)
            .await
            .expect("future advertisement can be created");

        let advertisements = repository
            .list_mobile_carousel()
            .await
            .expect("mobile advertisements can be listed");

        assert_eq!(advertisements.len(), 1);
        assert_eq!(advertisements[0].id, active.id);
    }

    #[tokio::test]
    async fn advertisement_repository_rejects_invalid_time_window() {
        let repository = AdvertisementRepository::memory();
        let mut payload = save_payload(Some("AD000020"), "时间错误", 1);
        payload.start_at = Some("2026-06-04 10:00:00".to_string());
        payload.end_at = Some("2026-06-04 09:00:00".to_string());

        let error = repository
            .create(payload)
            .await
            .expect_err("invalid time window must be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }
}
