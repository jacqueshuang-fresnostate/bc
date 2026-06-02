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
    pub fn summary(&self) -> GroupBuyPlanSummary {
        GroupBuyPlanSummary {
            id: self.id.clone(),
            lottery_id: self.lottery_id.clone(),
            lottery_name: self.lottery_name.clone(),
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
