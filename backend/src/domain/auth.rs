//! 管理员认证与会话领域模型，定义登录/登出/会话数据结构

use serde::{Deserialize, Serialize};

use super::{
    permission::{AdminRole, PermissionScope},
    user::AdminSummary,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 管理员登录请求，携带账号和原始密码。
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 管理员登录成功后的会话信息，包含返回给前端的 token 和权限范围。
pub struct AdminAuthSession {
    pub token: String,
    pub admin: AdminSummary,
    pub role: AdminRole,
    pub scopes: Vec<PermissionScope>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 当前管理员资料接口返回的数据，不重复返回登录 token。
pub struct CurrentAdminProfile {
    pub admin: AdminSummary,
    pub role: AdminRole,
    pub scopes: Vec<PermissionScope>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 管理员登出接口响应，标记当前登录态是否已经失效。
pub struct AdminLogoutResponse {
    pub logged_out: bool,
}

/// 管理员会话的派生视图方法。
impl AdminAuthSession {
    /// 从管理员会话数据中提取可对外返回的 profile 信息。
    pub fn profile(&self) -> CurrentAdminProfile {
        CurrentAdminProfile {
            admin: self.admin.clone(),
            role: self.role.clone(),
            scopes: self.scopes.clone(),
        }
    }
}
