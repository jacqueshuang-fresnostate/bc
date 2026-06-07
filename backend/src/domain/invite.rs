//! 邀请领域模型，定义邀请码、邀请关系与状态变更参数

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 邀请关系状态，控制邀请码关系是否参与返利和统计。
pub enum InviteStatus {
    Pending,
    Active,
    Disabled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台维护的邀请关系记录，连接邀请人、被邀请人和邀请码。
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
/// 后台创建邀请关系时提交的绑定参数。
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
/// 后台更新邀请关系状态、返利开关和备注时提交的参数。
pub struct UpdateInviteRecordRequest {
    pub status: InviteStatus,
    pub rebate_enabled: bool,
    pub note: String,
}
