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
    /// 业务唯一标识。
    pub id: String,
    /// 邀请人用户 ID。
    pub inviter_user_id: String,
    /// 邀请人用户名。
    pub inviter_username: String,
    /// 被邀请用户 ID。
    pub invitee_user_id: String,
    /// 被邀请用户名。
    pub invitee_username: String,
    /// 用户邀请码；只有代理邀请码具备邀请能力。
    pub invite_code: String,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: InviteStatus,
    /// 是否允许该邀请关系产生返利。
    pub rebate_enabled: bool,
    /// 后台备注或审核说明。
    pub note: String,
    /// 创建时间。
    pub created_at: String,
    /// 最后更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台创建邀请关系时提交的绑定参数。
pub struct CreateInviteRecordRequest {
    /// 业务唯一标识。
    pub id: String,
    /// 邀请人用户 ID。
    pub inviter_user_id: String,
    /// 被邀请用户 ID。
    pub invitee_user_id: String,
    /// 用户邀请码；只有代理邀请码具备邀请能力。
    pub invite_code: String,
    /// 是否允许该邀请关系产生返利。
    pub rebate_enabled: bool,
    /// 后台备注或审核说明。
    pub note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台更新邀请关系状态、返利开关和备注时提交的参数。
pub struct UpdateInviteRecordRequest {
    /// 业务状态，用于筛选、禁用或流转。
    pub status: InviteStatus,
    /// 是否允许该邀请关系产生返利。
    pub rebate_enabled: bool,
    /// 后台备注或审核说明。
    pub note: String,
}
