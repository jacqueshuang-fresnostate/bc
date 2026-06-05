//! 合买领域模型，定义合买计划、参与人和份额约束

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GroupBuyPlanStatus {
    Draft,
    Open,
    Filled,
    Cancelled,
    Settled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub struct GroupBuyPlan {
    pub id: String,
    pub lottery_id: String,
    pub lottery_name: String,
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

impl GroupBuyPlan {
    /// 生成当前实体对象的汇总信息。
    pub fn summary(&self) -> GroupBuyPlanSummary {
        GroupBuyPlanSummary {
            id: self.id.clone(),
            lottery_id: self.lottery_id.clone(),
            lottery_name: self.lottery_name.clone(),
            issue: self.issue.clone(),
            rule_code: self.rule_code.clone(),
            title: self.title.clone(),
            initiator_user_id: self.initiator_user_id.clone(),
            initiator_username: self.initiator_username.clone(),
            total_amount_minor: self.total_amount_minor,
            filled_amount_minor: self.filled_amount_minor,
            share_count: self.share_count,
            status: self.status.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyPlanSummary {
    pub id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub issue: String,
    pub rule_code: String,
    pub title: String,
    pub initiator_user_id: String,
    pub initiator_username: String,
    pub total_amount_minor: i64,
    pub filled_amount_minor: i64,
    pub share_count: u32,
    pub status: GroupBuyPlanStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub struct UpdateGroupBuyPlanRequest {
    pub status: GroupBuyPlanStatus,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AddGroupBuyParticipantRequest {
    pub id: String,
    pub user_id: String,
    pub amount_minor: i64,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub struct UserJoinGroupBuyPlanRequest {
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuySelectOption {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyCreateSettings {
    pub min_share_amount_minor: i64,
    pub initiator_min_percent: u8,
    pub participant_min_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyCreateOptions {
    pub lotteries: Vec<GroupBuySelectOption>,
    pub issues: Vec<GroupBuySelectOption>,
    pub plays: Vec<GroupBuySelectOption>,
    pub settings: GroupBuyCreateSettings,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyParticipationSummary {
    pub share_count: u32,
    pub amount_minor: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserGroupBuyPlan {
    pub id: String,
    pub lottery_id: String,
    pub lottery_name: String,
    pub category: Option<String>,
    pub issue: String,
    pub rule_code: String,
    pub play_name: String,
    pub title: String,
    pub numbers: String,
    pub total_amount_minor: i64,
    pub share_count: u32,
    pub share_amount_minor: i64,
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
pub struct UserGroupBuyPlanPage {
    pub items: Vec<UserGroupBuyPlan>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserGroupBuyActionResponse {
    pub plan: UserGroupBuyPlan,
    pub available_balance_minor: i64,
}
