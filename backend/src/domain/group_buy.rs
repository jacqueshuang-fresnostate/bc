//! 合买领域模型，定义合买计划、参与人和份额约束

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买计划状态，描述从草稿、开放认购到满单、取消和结算的生命周期。
pub enum GroupBuyPlanStatus {
    Draft,
    Open,
    Filled,
    Cancelled,
    Settled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买参与记录，保存每个用户认购金额、份数和参与备注。
pub struct GroupBuyParticipant {
    /// 合买参与记录 ID。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: String,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 份数。
    pub share_count: u32,
    /// 后台备注或审核说明。
    pub note: String,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买计划完整实体，包含发起人、投注内容、份额设置和参与人列表。
pub struct GroupBuyPlan {
    /// 合买计划 ID。
    pub id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 投注订单 ID。
    pub order_id: Option<String>,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: String,
    /// 展示标题。
    pub title: String,
    /// 组选或组合玩法号码列表。
    pub numbers: String,
    /// 合买发起人用户 ID。
    pub initiator_user_id: String,
    /// 合买发起人用户名。
    pub initiator_username: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 已认购金额，单位为分。
    pub filled_amount_minor: i64,
    /// 每份最低金额，单位为分。
    pub min_share_amount_minor: i64,
    /// 参与人最低认购金额，单位为分。
    pub participant_min_amount_minor: i64,
    /// 份数。
    pub share_count: u32,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: GroupBuyPlanStatus,
    /// 合买参与人列表。
    pub participants: Vec<GroupBuyParticipant>,
    /// 后台备注或审核说明。
    pub note: String,
    /// 创建时间。
    pub created_at: String,
    /// 最后更新时间。
    pub updated_at: String,
}

/// 合买计划的派生展示方法。
impl GroupBuyPlan {
    /// 生成当前实体对象的汇总信息。
    pub fn summary(&self) -> GroupBuyPlanSummary {
        GroupBuyPlanSummary {
            id: self.id.clone(),
            lottery_id: self.lottery_id.clone(),
            lottery_name: self.lottery_name.clone(),
            order_id: self.order_id.clone(),
            issue: self.issue.clone(),
            rule_code: self.rule_code.clone(),
            title: self.title.clone(),
            initiator_user_id: self.initiator_user_id.clone(),
            initiator_username: self.initiator_username.clone(),
            total_amount_minor: self.total_amount_minor,
            filled_amount_minor: self.filled_amount_minor,
            share_count: self.share_count,
            status: self.status.clone(),
            created_at: self.created_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台列表和关联展示使用的合买计划摘要。
pub struct GroupBuyPlanSummary {
    /// 合买计划 ID。
    pub id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 投注订单 ID。
    pub order_id: Option<String>,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: String,
    /// 展示标题。
    pub title: String,
    /// 合买发起人用户 ID。
    pub initiator_user_id: String,
    /// 合买发起人用户名。
    pub initiator_username: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 已认购金额，单位为分。
    pub filled_amount_minor: i64,
    /// 份数。
    pub share_count: u32,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: GroupBuyPlanStatus,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台创建合买计划时提交的原始参数。
pub struct CreateGroupBuyPlanRequest {
    /// 业务唯一标识。
    pub id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩票期号。
    #[serde(default)]
    pub issue: String,
    /// 玩法规则编码。
    #[serde(default)]
    pub rule_code: String,
    /// 展示标题。
    #[serde(default)]
    pub title: String,
    /// 组选或组合玩法号码列表。
    #[serde(default)]
    pub numbers: String,
    /// 合买发起人用户 ID。
    pub initiator_user_id: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 发起人自购金额，单位为分。
    pub initiator_amount_minor: i64,
    /// 后台备注或审核说明。
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台更新合买计划状态和备注时提交的参数。
pub struct UpdateGroupBuyPlanRequest {
    /// 业务状态，用于筛选、禁用或流转。
    pub status: GroupBuyPlanStatus,
    /// 后台备注或审核说明。
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台为合买计划添加参与人时提交的认购参数。
pub struct AddGroupBuyParticipantRequest {
    /// 业务唯一标识。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 后台备注或审核说明。
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端用户发起合买计划时提交的投注和自购信息。
pub struct UserCreateGroupBuyPlanRequest {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: String,
    /// 展示标题。
    pub title: String,
    /// 组选或组合玩法号码列表。
    pub numbers: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 当前用户自购金额，单位为分。
    pub self_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端用户参与合买计划时提交的认购金额。
pub struct UserJoinGroupBuyPlanRequest {
    /// 业务金额，单位为分。
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买创建页下拉选项，统一承载彩种、期号和玩法选项。
pub struct GroupBuySelectOption {
    /// 前端展示文案。
    pub label: String,
    /// 配置值或选项值。
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 发起合买时的金额、份额和发起人最低自购设置。
pub struct GroupBuyCreateSettings {
    /// 每份最低金额，单位为分。
    pub min_share_amount_minor: i64,
    /// 发起人最低自购比例。
    pub initiator_min_percent: u8,
    /// 参与人最低认购金额，单位为分。
    pub participant_min_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端发起合买页面所需的彩种、期号、玩法和默认设置。
pub struct GroupBuyCreateOptions {
    /// 可选彩种列表。
    pub lotteries: Vec<GroupBuySelectOption>,
    /// 可选期号列表。
    pub issues: Vec<GroupBuySelectOption>,
    /// 可选玩法列表。
    pub plays: Vec<GroupBuySelectOption>,
    /// 移动端或模块配置集合。
    pub settings: GroupBuyCreateSettings,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 当前用户在某个合买计划中的参与摘要。
pub struct GroupBuyParticipationSummary {
    /// 份数。
    pub share_count: u32,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户端展示的合买参与人摘要，隐藏真实用户名和用户 ID，只暴露核对认购所需信息。
pub struct UserGroupBuyParticipantSummary {
    /// 业务唯一标识。
    pub id: String,
    /// 脱敏后的用户展示名。
    pub display_name: String,
    /// 业务金额，单位为分。
    pub amount_minor: i64,
    /// 份数。
    pub share_count: u32,
    /// ismine字段。
    pub is_mine: bool,
    /// 创建时间。
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端合买大厅和我的合买列表使用的计划展示数据。
pub struct UserGroupBuyPlan {
    /// 业务唯一标识。
    pub id: String,
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩种名称。
    pub lottery_name: String,
    /// 投注订单 ID。
    pub order_id: Option<String>,
    /// 彩种分类编码。
    pub category: Option<String>,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: String,
    /// 玩法中文名称。
    pub play_name: String,
    /// 展示标题。
    pub title: String,
    /// 组选或组合玩法号码列表。
    pub numbers: String,
    /// 总金额，单位为分。
    pub total_amount_minor: i64,
    /// 份数。
    pub share_count: u32,
    /// 单份金额，单位为分。
    pub share_amount_minor: i64,
    /// 参与人最低认购金额，单位为分。
    pub participant_min_amount_minor: i64,
    /// 已认购金额，单位为分。
    pub filled_amount_minor: i64,
    /// 已售份数。
    pub sold_shares: u32,
    /// 剩余可认购份数。
    pub available_shares: u32,
    /// 合买进度百分比。
    pub progress_percent: u32,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: GroupBuyPlanStatus,
    /// participantcount字段。
    pub participant_count: usize,
    /// 合买参与人列表。
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<UserGroupBuyParticipantSummary>,
    /// initiatordisplay字段。
    pub initiator_display: String,
    /// initiatoravatarurl字段。
    pub initiator_avatar_url: String,
    /// myparticipation字段。
    pub my_participation: Option<GroupBuyParticipationSummary>,
    /// 创建时间。
    pub created_at: String,
    /// 最后更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端合买计划分页响应，目前承载当前页计划列表。
pub struct UserGroupBuyPlanPage {
    /// 分页数据列表。
    pub items: Vec<UserGroupBuyPlan>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端发起或参与合买后的响应，返回计划和最新可用余额。
pub struct UserGroupBuyActionResponse {
    /// plan字段。
    pub plan: UserGroupBuyPlan,
    /// 可用余额，单位为分。
    pub available_balance_minor: i64,
}
