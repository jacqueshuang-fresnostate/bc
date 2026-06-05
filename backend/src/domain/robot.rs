//! 机器人领域模型，定义机器人类型、状态、配置项与执行结果

use serde::{Deserialize, Serialize};

use crate::domain::{finance::LedgerEntry, group_buy::GroupBuyPlan, order::OrderDetail};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RobotKind {
    GroupBuy,
    Purchase,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RobotStatus {
    Enabled,
    Paused,
    Disabled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RobotConfigSummary {
    pub id: String,
    pub name: String,
    pub kind: RobotKind,
    pub lottery_ids: Vec<String>,
    pub status: RobotStatus,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RobotStatusRequest {
    pub status: RobotStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyRobotSkippedItem {
    pub robot_id: String,
    pub robot_name: String,
    pub lottery_id: String,
    pub issue: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyRobotRun {
    pub now: String,
    pub created_plans: Vec<GroupBuyPlan>,
    pub filled_plans: Vec<GroupBuyPlan>,
    pub created_orders: Vec<OrderDetail>,
    pub ledger_entries: Vec<LedgerEntry>,
    pub skipped_items: Vec<GroupBuyRobotSkippedItem>,
}
