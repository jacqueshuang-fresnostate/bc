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
        robot::{RobotConfigSummary, RobotKind, RobotStatus},
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, BusinessDatabase};

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

    /// 返回完整列表。
    pub async fn list(&self) -> ApiResult<Vec<RobotConfigSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    /// 按 ID 查询单条记录。
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

    async fn persist(&self, store: &RobotStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_robot_store(persistence, store).await?;
        }

        Ok(())
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
        "SELECT id, name, kind, status, description
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
            "INSERT INTO robot_configs (id, name, kind, status, description)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&robot.id)
        .bind(&robot.name)
        .bind(enum_to_string(&robot.kind)?)
        .bind(enum_to_string(&robot.status)?)
        .bind(&robot.description)
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

    /// 返回完整数据列表。
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

    Ok(robot)
}

/// 去除空白并校验必填字段。
fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

/// 返回内置种子或测试数据。
fn seed_robots() -> Vec<RobotConfigSummary> {
    vec![
        RobotConfigSummary {
            id: "R-GROUP-001".to_string(),
            name: "合买补单机器人".to_string(),
            kind: RobotKind::GroupBuy,
            lottery_ids: vec!["fc3d".to_string(), "ssc60".to_string()],
            status: RobotStatus::Enabled,
            description: "开盘期间发起合买并辅助满单".to_string(),
        },
        RobotConfigSummary {
            id: "R-BUY-001".to_string(),
            name: "购彩模拟机器人".to_string(),
            kind: RobotKind::Purchase,
            lottery_ids: vec!["ssc60".to_string()],
            status: RobotStatus::Paused,
            description: "按彩种开盘时间模拟普通用户购彩".to_string(),
        },
        RobotConfigSummary {
            id: "R-BUY-002".to_string(),
            name: "指定号码测试机器人".to_string(),
            kind: RobotKind::Purchase,
            lottery_ids: vec!["manual-test".to_string()],
            status: RobotStatus::Disabled,
            description: "指定号码测试彩暂停机器人执行".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::lottery::seed_lotteries;

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
                },
                &lotteries,
            )
            .await
            .expect("robot can be created");

        assert_eq!(created.id, "R-NEW");
        assert_eq!(created.lottery_ids, vec!["fc3d".to_string()]);

        let updated = robots
            .set_status("R-NEW", RobotStatus::Enabled)
            .await
            .expect("status can be updated");
        assert_eq!(updated.status, RobotStatus::Enabled);
    }

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
                },
                &lotteries,
            )
            .await
            .expect_err("empty lotteries must be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

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
                },
                &lotteries,
            )
            .await
            .expect_err("unknown lottery must be rejected");

        assert!(matches!(error, ApiError::NotFound(_)));
    }
}
