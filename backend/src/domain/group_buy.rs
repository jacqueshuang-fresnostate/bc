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
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub amount_minor: i64,
    pub share_count: u32,
    pub note: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买计划完整实体，包含发起人、投注内容、份额设置和参与人列表。
pub struct GroupBuyPlan {
    pub id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub order_id: Option<String>,
    pub issue: String,
    pub rule_code: String,
    pub title: String,
    pub numbers: String,
    pub initiator_user_id: String,
    pub initiator_username: String,
    pub total_amount_minor: i64,
    pub filled_amount_minor: i64,
    pub min_share_amount_minor: i64,
    pub participant_min_amount_minor: i64,
    pub share_count: u32,
    pub status: GroupBuyPlanStatus,
    pub participants: Vec<GroupBuyParticipant>,
    pub note: String,
    pub created_at: String,
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
    pub id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub order_id: Option<String>,
    pub issue: String,
    pub rule_code: String,
    pub title: String,
    pub initiator_user_id: String,
    pub initiator_username: String,
    pub total_amount_minor: i64,
    pub filled_amount_minor: i64,
    pub share_count: u32,
    pub status: GroupBuyPlanStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台创建合买计划时提交的原始参数。
pub struct CreateGroupBuyPlanRequest {
    pub id: String,
    pub lottery_id: String,
    #[serde(default)]
    pub issue: String,
    #[serde(default)]
    pub rule_code: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub numbers: String,
    pub initiator_user_id: String,
    pub total_amount_minor: i64,
    pub initiator_amount_minor: i64,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台更新合买计划状态和备注时提交的参数。
pub struct UpdateGroupBuyPlanRequest {
    pub status: GroupBuyPlanStatus,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台为合买计划添加参与人时提交的认购参数。
pub struct AddGroupBuyParticipantRequest {
    pub id: String,
    pub user_id: String,
    pub amount_minor: i64,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端用户发起合买计划时提交的投注和自购信息。
pub struct UserCreateGroupBuyPlanRequest {
    pub lottery_id: String,
    pub issue: String,
    pub rule_code: String,
    pub title: String,
    pub numbers: String,
    pub total_amount_minor: i64,
    pub self_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端用户参与合买计划时提交的认购金额。
pub struct UserJoinGroupBuyPlanRequest {
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 合买创建页下拉选项，统一承载彩种、期号和玩法选项。
pub struct GroupBuySelectOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 发起合买时的金额、份额和发起人最低自购设置。
pub struct GroupBuyCreateSettings {
    pub min_share_amount_minor: i64,
    pub initiator_min_percent: u8,
    pub participant_min_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端发起合买页面所需的彩种、期号、玩法和默认设置。
pub struct GroupBuyCreateOptions {
    pub lotteries: Vec<GroupBuySelectOption>,
    pub issues: Vec<GroupBuySelectOption>,
    pub plays: Vec<GroupBuySelectOption>,
    pub settings: GroupBuyCreateSettings,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 当前用户在某个合买计划中的参与摘要。
pub struct GroupBuyParticipationSummary {
    pub share_count: u32,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端合买大厅和我的合买列表使用的计划展示数据。
pub struct UserGroupBuyPlan {
    pub id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub order_id: Option<String>,
    pub category: Option<String>,
    pub issue: String,
    pub rule_code: String,
    pub play_name: String,
    pub title: String,
    pub numbers: String,
    pub total_amount_minor: i64,
    pub share_count: u32,
    pub share_amount_minor: i64,
    pub participant_min_amount_minor: i64,
    pub filled_amount_minor: i64,
    pub sold_shares: u32,
    pub available_shares: u32,
    pub progress_percent: u32,
    pub status: GroupBuyPlanStatus,
    pub participant_count: usize,
    pub initiator_display: String,
    pub my_participation: Option<GroupBuyParticipationSummary>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端合买计划分页响应，目前承载当前页计划列表。
pub struct UserGroupBuyPlanPage {
    pub items: Vec<UserGroupBuyPlan>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端发起或参与合买后的响应，返回计划和最新可用余额。
pub struct UserGroupBuyActionResponse {
    pub plan: UserGroupBuyPlan,
    pub available_balance_minor: i64,
}
