use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, RwLock},
};

use crate::{
    domain::{
        lottery::LotteryKind,
        robot::{RobotConfigSummary, RobotKind, RobotStatus},
    },
    error::{ApiError, ApiResult},
};

#[derive(Clone)]
pub struct RobotRepository {
    inner: Arc<RwLock<RobotStore>>,
}

impl RobotRepository {
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RobotStore::seeded())),
        }
    }

    pub async fn list(&self) -> ApiResult<Vec<RobotConfigSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))
            .map(|store| store.list())
    }

    pub async fn get(&self, id: &str) -> ApiResult<RobotConfigSummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?
            .get(id)
    }

    pub async fn create(
        &self,
        robot: RobotConfigSummary,
        lotteries: &[LotteryKind],
    ) -> ApiResult<RobotConfigSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?
            .create(robot, lotteries)
    }

    pub async fn update(
        &self,
        id: &str,
        robot: RobotConfigSummary,
        lotteries: &[LotteryKind],
    ) -> ApiResult<RobotConfigSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?
            .update(id, robot, lotteries)
    }

    pub async fn delete(&self, id: &str) -> ApiResult<RobotConfigSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?
            .delete(id)
    }

    pub async fn set_status(&self, id: &str, status: RobotStatus) -> ApiResult<RobotConfigSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("robot store lock poisoned".to_string()))?
            .set_status(id, status)
    }
}

#[derive(Debug)]
struct RobotStore {
    robots: BTreeMap<String, RobotConfigSummary>,
}

impl RobotStore {
    fn seeded() -> Self {
        Self {
            robots: seed_robots()
                .into_iter()
                .map(|robot| (robot.id.clone(), robot))
                .collect(),
        }
    }

    fn list(&self) -> Vec<RobotConfigSummary> {
        self.robots.values().cloned().collect()
    }

    fn get(&self, id: &str) -> ApiResult<RobotConfigSummary> {
        self.robots
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("robot `{id}` not found")))
    }

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

    fn delete(&mut self, id: &str) -> ApiResult<RobotConfigSummary> {
        self.robots
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("robot `{id}` not found")))
    }

    fn set_status(&mut self, id: &str, status: RobotStatus) -> ApiResult<RobotConfigSummary> {
        let robot = self
            .robots
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("robot `{id}` not found")))?;
        robot.status = status;
        Ok(robot.clone())
    }
}

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

fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

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
