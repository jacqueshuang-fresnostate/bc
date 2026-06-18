//! 机器人领域模型，定义机器人类型、状态与配置项

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
};

use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::{
        lottery::LotteryKind,
        robot::{
            default_group_buy_fill_before_draw_seconds, default_group_buy_fill_strategy,
            GroupBuyRobotFillStrategy, RobotConfigSummary, RobotKind, RobotStatus,
        },
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

const PROTECTED_ROBOT_IDS: &[&str] = &["R-GROUP-001", "R-BUY-001"];

#[derive(Clone)]
/// 机器人配置仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct RobotRepository {
    inner: Arc<RwLock<RobotStore>>,
    persistence: Option<BusinessDatabase>,
}

/// 机器人配置仓储，负责该模块数据读取、业务变更和持久化协调。
impl RobotRepository {
    /// 返回带内置种子数据的内存仓储实例。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RobotStore::seeded())),
            persistence: None,
        }
    }

    /// 从数据库加载历史数据并初始化持久化仓储。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_robot_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 按当前仓储快照返回全部机器人配置列表。
    pub async fn list(&self) -> ApiResult<Vec<RobotConfigSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 按业务标识读取单条记录，未命中时返回未找到错误。
    pub async fn get(&self, id: &str) -> ApiResult<RobotConfigSummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?
            .get(id)
    }

    /// 校验入参并创建一条新记录。
    pub async fn create(
        &self,
        robot: RobotConfigSummary,
        lotteries: &[LotteryKind],
    ) -> ApiResult<RobotConfigSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?;
            let result = store.create(robot, lotteries)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 更新现有记录并持久化变更。
    pub async fn update(
        &self,
        id: &str,
        robot: RobotConfigSummary,
        lotteries: &[LotteryKind],
    ) -> ApiResult<RobotConfigSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?;
            let result = store.update(id, robot, lotteries)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 更新机器人的运行状态。
    pub async fn set_status(&self, id: &str, status: RobotStatus) -> ApiResult<RobotConfigSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?;
            let result = store.set_status(id, status)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 删除普通机器人配置；核心内置机器人只能暂停或禁用，不能删除。
    pub async fn delete(&self, id: &str) -> ApiResult<RobotConfigSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?;
            let result = store.delete(id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }
    /// 把当前仓储快照同步保存到持久化存储。
    async fn persist(&self, store: &RobotStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_robot_store(persistence, store).await?;
        }

        Ok(())
    }

    /// 从数据库重新加载机器人配置快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_robot_store(persistence).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("机器人配置缓存刷新失败".to_string()))? = store;
        Ok(true)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// 机器人配置运行时数据快照，用于内存模式和数据库持久化前的业务校验。
struct RobotStore {
    robots: BTreeMap<String, RobotConfigSummary>,
}

/// 从数据库加载机器人配置运行时快照，空库时按模块规则初始化。
async fn load_robot_store(database: &BusinessDatabase) -> ApiResult<RobotStore> {
    let pool = database.pool();
    let mut lottery_bindings = BTreeMap::<String, Vec<String>>::new();
    for row in sqlx::query(
        "SELECT robot_id, lottery_id
         FROM robot_lottery_bindings
         ORDER BY robot_id ASC, lottery_id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("机器人彩种绑定数据读取失败".to_string()))?
    {
        let robot_id: String = row
            .try_get("robot_id")
            .map_err(|_| ApiError::Internal("机器人彩种绑定数据读取失败".to_string()))?;
        let lottery_id: String = row
            .try_get("lottery_id")
            .map_err(|_| ApiError::Internal("机器人彩种绑定数据读取失败".to_string()))?;
        lottery_bindings
            .entry(robot_id)
            .or_default()
            .push(lottery_id);
    }

    let mut robots = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id,
                name,
                kind,
                status,
                description,
                group_buy_fill_strategy,
                group_buy_fill_before_draw_seconds
         FROM robot_configs
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("机器人配置数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("机器人配置数据读取失败".to_string()))?;
        robots.insert(
            id.clone(),
            RobotConfigSummary {
                id: id.clone(),
                name: row
                    .try_get("name")
                    .map_err(|_| ApiError::Internal("机器人配置数据读取失败".to_string()))?,
                kind: enum_from_string(
                    row.try_get("kind")
                        .map_err(|_| ApiError::Internal("机器人配置数据读取失败".to_string()))?,
                )?,
                lottery_ids: lottery_bindings.remove(&id).unwrap_or_default(),
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("机器人配置数据读取失败".to_string()))?,
                )?,
                description: row
                    .try_get("description")
                    .map_err(|_| ApiError::Internal("机器人配置数据读取失败".to_string()))?,
                group_buy_fill_strategy: enum_from_string(
                    row.try_get("group_buy_fill_strategy")
                        .map_err(|_| ApiError::Internal("机器人配置数据读取失败".to_string()))?,
                )?,
                group_buy_fill_before_draw_seconds: row
                    .try_get::<i32, _>("group_buy_fill_before_draw_seconds")
                    .map_err(|_| ApiError::Internal("机器人配置数据读取失败".to_string()))?
                    .try_into()
                    .map_err(|_| ApiError::Internal("机器人补满秒数数据无效".to_string()))?,
                deletable: is_robot_deletable(&id),
            },
        );
    }

    if robots.is_empty() {
        let seeded = RobotStore::seeded();
        save_robot_store(database, &seeded).await?;
        return Ok(seeded);
    }

    Ok(RobotStore { robots })
}

/// 把机器人配置运行时快照保存到数据库。
async fn save_robot_store(database: &BusinessDatabase, store: &RobotStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("机器人事务开启失败".to_string()))?;

    for table in ["robot_lottery_bindings", "robot_configs"] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("机器人数据清理失败".to_string()))?;
    }

    for robot in store.robots.values() {
        sqlx::query(
            "INSERT INTO robot_configs (
                 id,
                 name,
                 kind,
                 status,
                 description,
                 group_buy_fill_strategy,
                 group_buy_fill_before_draw_seconds
             )
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&robot.id)
        .bind(&robot.name)
        .bind(enum_to_string(&robot.kind)?)
        .bind(enum_to_string(&robot.status)?)
        .bind(&robot.description)
        .bind(enum_to_string(&robot.group_buy_fill_strategy)?)
        .bind(
            i32::try_from(robot.group_buy_fill_before_draw_seconds)
                .map_err(|_| ApiError::BadRequest("合买机器人开奖前补满秒数过大".to_string()))?,
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("机器人配置数据保存失败".to_string()))?;

        for lottery_id in &robot.lottery_ids {
            sqlx::query(
                "INSERT INTO robot_lottery_bindings (robot_id, lottery_id)
                 VALUES ($1, $2)",
            )
            .bind(&robot.id)
            .bind(lottery_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("机器人彩种绑定数据保存失败".to_string()))?;
        }
    }

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("机器人事务提交失败".to_string()))
}

/// 机器人配置运行时数据快照，用于内存模式和数据库持久化前的业务校验。
impl RobotStore {
    /// 构建并返回种子数据。
    fn seeded() -> Self {
        Self {
            robots: seed_robots()
                .into_iter()
                .map(|robot| (robot.id.clone(), robot))
                .collect(),
        }
    }

    /// 按当前仓储快照返回全部机器人配置列表。
    fn list(&self) -> Vec<RobotConfigSummary> {
        self.robots.values().cloned().collect()
    }

    /// 按标识查询并返回单条记录。
    fn get(&self, id: &str) -> ApiResult<RobotConfigSummary> {
        self.robots
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("robot `{id}` not found")))
    }

    /// 校验入参并创建新记录。
    fn create(
        &mut self,
        robot: RobotConfigSummary,
        lotteries: &[LotteryKind],
    ) -> ApiResult<RobotConfigSummary> {
        let robot = normalize_robot(robot, lotteries)?;
        if self.robots.contains_key(&robot.id) {
            return Err(ApiError::Conflict(format!(
                "robot `{}` already exists",
                robot.id
            )));
        }

        self.robots.insert(robot.id.clone(), robot.clone());
        Ok(robot)
    }

    /// 校验入参并更新指定记录。
    fn update(
        &mut self,
        id: &str,
        robot: RobotConfigSummary,
        lotteries: &[LotteryKind],
    ) -> ApiResult<RobotConfigSummary> {
        let robot = normalize_robot(robot, lotteries)?;
        if id != robot.id {
            return Err(ApiError::BadRequest(
                "path id must match robot id".to_string(),
            ));
        }
        if !self.robots.contains_key(id) {
            return Err(ApiError::NotFound(format!("robot `{id}` not found")));
        }

        self.robots.insert(id.to_string(), robot.clone());
        Ok(robot)
    }

    /// 更新并持久化状态。
    fn set_status(&mut self, id: &str, status: RobotStatus) -> ApiResult<RobotConfigSummary> {
        let robot = self
            .robots
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("robot `{id}` not found")))?;
        robot.status = status;
        Ok(robot.clone())
    }

    /// 删除普通机器人配置，内置核心机器人返回冲突错误。
    fn delete(&mut self, id: &str) -> ApiResult<RobotConfigSummary> {
        let id = required_trimmed(id.to_string(), "robot id")?;
        let robot = self
            .robots
            .get(&id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("robot `{id}` not found")))?;
        if !robot.deletable {
            return Err(ApiError::Conflict(
                "内置机器人配置不能删除，请改为暂停或禁用".to_string(),
            ));
        }

        self.robots.remove(&id);
        Ok(robot)
    }
}

/// 标准化输入并返回规范值。
fn normalize_robot(
    mut robot: RobotConfigSummary,
    lotteries: &[LotteryKind],
) -> ApiResult<RobotConfigSummary> {
    robot.id = required_trimmed(robot.id, "robot id")?;
    robot.name = required_trimmed(robot.name, "robot name")?;
    robot.description = required_trimmed(robot.description, "robot description")?;
    robot.lottery_ids = robot
        .lottery_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();

    if robot.lottery_ids.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one robot lottery is required".to_string(),
        ));
    }

    let known_lottery_ids = lotteries
        .iter()
        .map(|lottery| lottery.id.as_str())
        .collect::<BTreeSet<_>>();
    for lottery_id in &robot.lottery_ids {
        if !known_lottery_ids.contains(lottery_id.as_str()) {
            return Err(ApiError::NotFound(format!(
                "lottery `{lottery_id}` not found for robot"
            )));
        }
    }

    robot.lottery_ids = robot
        .lottery_ids
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    normalize_group_buy_fill_config(&mut robot)?;
    robot.deletable = is_robot_deletable(&robot.id);

    Ok(robot)
}

/// 规范合买机器人补满策略，避免无效秒数导致调度无法判断触发窗口。
fn normalize_group_buy_fill_config(robot: &mut RobotConfigSummary) -> ApiResult<()> {
    if robot.kind != RobotKind::GroupBuy {
        robot.group_buy_fill_strategy = default_group_buy_fill_strategy();
        robot.group_buy_fill_before_draw_seconds = default_group_buy_fill_before_draw_seconds();
        return Ok(());
    }

    if robot.group_buy_fill_before_draw_seconds == 0 {
        return Err(ApiError::BadRequest(
            "合买机器人开奖前补满秒数必须大于 0".to_string(),
        ));
    }
    if robot.group_buy_fill_before_draw_seconds > 86_400 {
        return Err(ApiError::BadRequest(
            "合买机器人开奖前补满秒数不能超过 86400".to_string(),
        ));
    }

    match robot.group_buy_fill_strategy {
        GroupBuyRobotFillStrategy::Rhythm | GroupBuyRobotFillStrategy::BeforeDraw => Ok(()),
    }
}

/// 根据机器人 ID 判断是否允许后台删除。
fn is_robot_deletable(id: &str) -> bool {
    !PROTECTED_ROBOT_IDS.contains(&id)
}

/// 去除空白并校验必填字段。
fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

/// 返回内置机器人配置，系统机器人不允许删除。
fn seed_robots() -> Vec<RobotConfigSummary> {
    vec![
        RobotConfigSummary {
            id: "R-GROUP-001".to_string(),
            name: "合买补单机器人".to_string(),
            kind: RobotKind::GroupBuy,
            lottery_ids: vec!["fc3d".to_string(), "ssc60".to_string()],
            status: RobotStatus::Enabled,
            description: "开盘期间发起合买并辅助满单".to_string(),
            group_buy_fill_strategy: default_group_buy_fill_strategy(),
            group_buy_fill_before_draw_seconds: default_group_buy_fill_before_draw_seconds(),
            deletable: false,
        },
        RobotConfigSummary {
            id: "R-BUY-001".to_string(),
            name: "购彩模拟机器人".to_string(),
            kind: RobotKind::Purchase,
            lottery_ids: vec!["ssc60".to_string()],
            status: RobotStatus::Paused,
            description: "按彩种开盘时间模拟普通用户购彩".to_string(),
            group_buy_fill_strategy: default_group_buy_fill_strategy(),
            group_buy_fill_before_draw_seconds: default_group_buy_fill_before_draw_seconds(),
            deletable: false,
        },
        RobotConfigSummary {
            id: "R-BUY-002".to_string(),
            name: "指定号码测试机器人".to_string(),
            kind: RobotKind::Purchase,
            lottery_ids: vec!["manual-test".to_string()],
            status: RobotStatus::Disabled,
            description: "指定号码测试彩暂停机器人执行".to_string(),
            group_buy_fill_strategy: default_group_buy_fill_strategy(),
            group_buy_fill_before_draw_seconds: default_group_buy_fill_before_draw_seconds(),
            deletable: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::lottery::seed_lotteries;
    /// 验证机器人仓储创建和更新机器人。
    #[tokio::test]
    async fn robot_repository_creates_and_updates_robot() {
        let robots = RobotRepository::memory_seeded();
        let lotteries = seed_lotteries();
        let created = robots
            .create(
                RobotConfigSummary {
                    id: " R-NEW ".to_string(),
                    name: "新购彩机器人".to_string(),
                    kind: RobotKind::Purchase,
                    lottery_ids: vec!["fc3d".to_string(), "fc3d".to_string()],
                    status: RobotStatus::Paused,
                    description: "测试机器人".to_string(),
                    group_buy_fill_strategy: default_group_buy_fill_strategy(),
                    group_buy_fill_before_draw_seconds: default_group_buy_fill_before_draw_seconds(
                    ),
                    deletable: true,
                },
                &lotteries,
            )
            .await
            .expect("robot can be created");

        assert_eq!(created.id, "R-NEW");
        assert_eq!(created.lottery_ids, vec!["fc3d".to_string()]);
        assert!(created.deletable);

        let updated = robots
            .set_status("R-NEW", RobotStatus::Enabled)
            .await
            .expect("status can be updated");
        assert_eq!(updated.status, RobotStatus::Enabled);
    }
    /// 验证机器人仓储拒绝空彩种ids。
    #[tokio::test]
    async fn robot_repository_rejects_empty_lottery_ids() {
        let robots = RobotRepository::memory_seeded();
        let lotteries = seed_lotteries();
        let error = robots
            .create(
                RobotConfigSummary {
                    id: "R-EMPTY".to_string(),
                    name: "空彩种机器人".to_string(),
                    kind: RobotKind::GroupBuy,
                    lottery_ids: Vec::new(),
                    status: RobotStatus::Paused,
                    description: "测试机器人".to_string(),
                    group_buy_fill_strategy: default_group_buy_fill_strategy(),
                    group_buy_fill_before_draw_seconds: default_group_buy_fill_before_draw_seconds(
                    ),
                    deletable: true,
                },
                &lotteries,
            )
            .await
            .expect_err("empty lotteries must be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }
    /// 验证机器人仓储拒绝unknown彩种id。
    #[tokio::test]
    async fn robot_repository_rejects_unknown_lottery_id() {
        let robots = RobotRepository::memory_seeded();
        let lotteries = seed_lotteries();
        let error = robots
            .create(
                RobotConfigSummary {
                    id: "R-MISSING".to_string(),
                    name: "未知彩种机器人".to_string(),
                    kind: RobotKind::Purchase,
                    lottery_ids: vec!["missing".to_string()],
                    status: RobotStatus::Paused,
                    description: "测试机器人".to_string(),
                    group_buy_fill_strategy: default_group_buy_fill_strategy(),
                    group_buy_fill_before_draw_seconds: default_group_buy_fill_before_draw_seconds(
                    ),
                    deletable: true,
                },
                &lotteries,
            )
            .await
            .expect_err("unknown lottery must be rejected");

        assert!(matches!(error, ApiError::NotFound(_)));
    }
    /// 验证机器人仓储删除仅deletablerobots。
    #[tokio::test]
    async fn robot_repository_deletes_only_deletable_robots() {
        let robots = RobotRepository::memory_seeded();
        let lotteries = seed_lotteries();
        robots
            .create(
                RobotConfigSummary {
                    id: "R-DELETE-ME".to_string(),
                    name: "可删除机器人".to_string(),
                    kind: RobotKind::Purchase,
                    lottery_ids: vec!["fc3d".to_string()],
                    status: RobotStatus::Paused,
                    description: "测试删除".to_string(),
                    group_buy_fill_strategy: default_group_buy_fill_strategy(),
                    group_buy_fill_before_draw_seconds: default_group_buy_fill_before_draw_seconds(
                    ),
                    deletable: true,
                },
                &lotteries,
            )
            .await
            .expect("robot can be created");

        let deleted = robots
            .delete("R-DELETE-ME")
            .await
            .expect("deletable robot can be deleted");
        assert_eq!(deleted.id, "R-DELETE-ME");
        assert!(robots.get("R-DELETE-ME").await.is_err());
    }
    /// 验证机器人仓储拒绝deletingprotected机器人。
    #[tokio::test]
    async fn robot_repository_rejects_deleting_protected_robot() {
        let robots = RobotRepository::memory_seeded();
        let error = robots
            .delete("R-GROUP-001")
            .await
            .expect_err("protected robot cannot be deleted");

        assert!(matches!(error, ApiError::Conflict(_)));
    }
}
