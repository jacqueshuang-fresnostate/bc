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
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub invite_code: String,
    pub status: AgentApplicationStatus,
    pub reason: String,
    pub review_note: Option<String>,
    pub reviewed_by_admin_id: Option<String>,
    pub reviewed_by_admin_username: Option<String>,
    pub reviewed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端提交代理申请时填写的申请说明。
pub struct SubmitAgentApplicationRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台审核代理申请时提交的审核结果和备注。
pub struct ReviewAgentApplicationRequest {
    pub approved: bool,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端读取代理中心时返回的申请状态包装，避免无申请时直接返回空数据。
pub struct UserAgentApplicationResponse {
    pub application: Option<AgentApplication>,
}
