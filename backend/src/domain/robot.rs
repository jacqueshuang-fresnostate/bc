//! 机器人领域模型，定义机器人类型、状态、配置项与执行结果

use serde::{Deserialize, Serialize};

use crate::domain::{finance::LedgerEntry, group_buy::GroupBuyPlan, order::OrderDetail};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 机器人类型，当前主要用于合买自动发起和自动认购。
pub enum RobotKind {
    GroupBuy,
    Purchase,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 机器人运行状态，控制调度器是否执行机器人任务。
pub enum RobotStatus {
    Enabled,
    Paused,
    Disabled,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买机器人补满策略，控制机器人按阶段补单或在开奖前一次性补满。
pub enum GroupBuyRobotFillStrategy {
    Rhythm,
    BeforeDraw,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台机器人配置摘要，包含绑定彩种和运行状态。
pub struct RobotConfigSummary {
    pub id: String,
    pub name: String,
    pub kind: RobotKind,
    pub lottery_ids: Vec<String>,
    pub status: RobotStatus,
    pub description: String,
    #[serde(default = "default_group_buy_fill_strategy")]
    pub group_buy_fill_strategy: GroupBuyRobotFillStrategy,
    #[serde(default = "default_group_buy_fill_before_draw_seconds")]
    pub group_buy_fill_before_draw_seconds: u32,
    #[serde(default)]
    pub deletable: bool,
}

/// 返回合买机器人默认补满策略，兼容历史配置和旧接口请求。
pub fn default_group_buy_fill_strategy() -> GroupBuyRobotFillStrategy {
    GroupBuyRobotFillStrategy::Rhythm
}

/// 返回合买机器人默认开奖前补满秒数，只有策略为开奖前补满时生效。
pub fn default_group_buy_fill_before_draw_seconds() -> u32 {
    15
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台切换机器人状态时提交的请求。
pub struct RobotStatusRequest {
    pub status: RobotStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买机器人本轮跳过的彩种或期号及原因。
pub struct GroupBuyRobotSkippedItem {
    pub robot_id: String,
    pub robot_name: String,
    pub lottery_id: String,
    pub issue: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买机器人单轮执行结果，汇总创建计划、认购订单和资金流水。
pub struct GroupBuyRobotRun {
    pub now: String,
    pub created_plans: Vec<GroupBuyPlan>,
    pub filled_plans: Vec<GroupBuyPlan>,
    pub created_orders: Vec<OrderDetail>,
    pub ledger_entries: Vec<LedgerEntry>,
    pub skipped_items: Vec<GroupBuyRobotSkippedItem>,
}
