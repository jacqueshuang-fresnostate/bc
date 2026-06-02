use serde::{Deserialize, Serialize};

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
