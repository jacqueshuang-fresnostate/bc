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
    /// 业务唯一标识。
    pub id: String,
    /// 展示名称。
    pub name: String,
    /// 业务类型。
    pub kind: RobotKind,
    /// 机器人可操作的彩种 ID 列表。
    pub lottery_ids: Vec<String>,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: RobotStatus,
    /// 配置或记录的中文说明。
    pub description: String,
    /// 合买机器人补满策略。
    #[serde(default = "default_group_buy_fill_strategy")]
    pub group_buy_fill_strategy: GroupBuyRobotFillStrategy,
    /// 开奖前兜底补满提前秒数。
    #[serde(default = "default_group_buy_fill_before_draw_seconds")]
    pub group_buy_fill_before_draw_seconds: u32,
    /// 后台是否允许删除该配置。
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
    /// 业务状态，用于筛选、禁用或流转。
    pub status: RobotStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买机器人本轮跳过的彩种或期号及原因。
pub struct GroupBuyRobotSkippedItem {
    /// 机器人配置 ID。
    pub robot_id: String,
    /// 机器人名称。
    pub robot_name: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩票期号。
    pub issue: Option<String>,
    /// 申请、审核或跳过原因。
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买机器人单轮执行结果，汇总创建计划、认购订单和资金流水。
pub struct GroupBuyRobotRun {
    /// 当前业务时间字符串。
    pub now: String,
    /// 本轮创建的合买计划。
    pub created_plans: Vec<GroupBuyPlan>,
    /// 本轮补满的合买计划。
    pub filled_plans: Vec<GroupBuyPlan>,
    /// 本轮创建的机器人注单。
    pub created_orders: Vec<OrderDetail>,
    /// 本轮生成的资金流水。
    pub ledger_entries: Vec<LedgerEntry>,
    /// 本轮机器人跳过项及原因。
    pub skipped_items: Vec<GroupBuyRobotSkippedItem>,
}
