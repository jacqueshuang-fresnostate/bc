//! 代理申请领域模型，定义手机端申请代理和后台审核需要的数据结构。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 代理申请状态，覆盖待审核、已通过和已驳回三个审核阶段。
pub enum AgentApplicationStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 代理申请记录，保存申请用户、申请说明、审核结果和审核人快照。
pub struct AgentApplication {
    /// 业务唯一标识。
    pub id: String,
    /// 关联用户 ID。
    pub user_id: String,
    /// 用户展示名。
    pub username: String,
    /// 用户邀请码；只有代理邀请码具备邀请能力。
    pub invite_code: String,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: AgentApplicationStatus,
    /// 申请、审核或跳过原因。
    pub reason: String,
    /// 审核备注。
    pub review_note: Option<String>,
    /// 审核管理员 ID。
    pub reviewed_by_admin_id: Option<String>,
    /// 审核管理员用户名。
    pub reviewed_by_admin_username: Option<String>,
    /// 审核完成时间。
    pub reviewed_at: Option<String>,
    /// 创建时间。
    pub created_at: String,
    /// 最后更新时间。
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端提交代理申请时填写的申请说明。
pub struct SubmitAgentApplicationRequest {
    /// 申请、审核或跳过原因。
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台审核代理申请时提交的审核结果和备注。
pub struct ReviewAgentApplicationRequest {
    /// 审核是否通过。
    pub approved: bool,
    /// 后台备注或审核说明。
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端读取代理中心时返回的申请状态包装，避免无申请时直接返回空数据。
pub struct UserAgentApplicationResponse {
    /// 当前用户的代理申请；为空表示未申请。
    pub application: Option<AgentApplication>,
}
