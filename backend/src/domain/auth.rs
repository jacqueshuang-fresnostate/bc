//! 管理员认证与会话领域模型，定义登录/登出/会话数据结构

use serde::{Deserialize, Serialize};

use super::{
    permission::{AdminRole, PermissionScope},
    user::AdminSummary,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminLoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminAuthSession {
    pub token: String,
    pub admin: AdminSummary,
    pub role: AdminRole,
    pub scopes: Vec<PermissionScope>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CurrentAdminProfile {
    pub admin: AdminSummary,
    pub role: AdminRole,
    pub scopes: Vec<PermissionScope>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminLogoutResponse {
    pub logged_out: bool,
}

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
