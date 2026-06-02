use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InviteStatus {
    Pending,
    Active,
    Disabled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InviteRecord {
    pub id: String,
    pub inviter_user_id: String,
    pub inviter_username: String,
    pub invitee_user_id: String,
    pub invitee_username: String,
    pub invite_code: String,
    pub status: InviteStatus,
    pub rebate_enabled: bool,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateInviteRecordRequest {
    pub id: String,
    pub inviter_user_id: String,
    pub invitee_user_id: String,
    pub invite_code: String,
    pub rebate_enabled: bool,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInviteRecordRequest {
    pub status: InviteStatus,
    pub rebate_enabled: bool,
    pub note: String,
}
