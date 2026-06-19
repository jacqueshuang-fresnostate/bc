//! 管理员认证与会话领域模型，定义登录/登出/会话数据结构

use serde::{Deserialize, Serialize};

use super::{
    permission::{effective_permission_keys, AdminRole, PermissionScope},
    user::AdminSummary,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 管理员登录请求，携带账号和原始密码。
pub struct AdminLoginRequest {
    /// 用户展示名。
    pub username: String,
    /// 用户输入的登录密码明文，仅用于请求校验和哈希生成。
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 管理员登录成功后的会话信息，包含返回给前端的 token 和权限范围。
pub struct AdminAuthSession {
    /// 管理员登录会话 token。
    pub token: String,
    /// 管理员账号摘要。
    pub admin: AdminSummary,
    /// 管理员角色信息。
    pub role: AdminRole,
    /// 角色拥有的权限范围列表。
    pub scopes: Vec<PermissionScope>,
    /// 角色拥有的有效细粒度权限点，包含旧模块权限自动展开的权限。
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 当前管理员资料接口返回的数据，不重复返回登录 token。
pub struct CurrentAdminProfile {
    /// 管理员账号摘要。
    pub admin: AdminSummary,
    /// 管理员角色信息。
    pub role: AdminRole,
    /// 角色拥有的权限范围列表。
    pub scopes: Vec<PermissionScope>,
    /// 角色拥有的有效细粒度权限点，包含旧模块权限自动展开的权限。
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 管理员登出接口响应，标记当前登录态是否已经失效。
pub struct AdminLogoutResponse {
    /// 当前会话是否已经登出。
    pub logged_out: bool,
}

/// 管理员会话的派生视图方法。
impl AdminAuthSession {
    /// 从管理员会话数据中提取可对外返回的 profile 信息。
    pub fn profile(&self) -> CurrentAdminProfile {
        CurrentAdminProfile {
            admin: self.admin.clone(),
            permissions: self.permissions.clone(),
            role: self.role.clone(),
            scopes: self.scopes.clone(),
        }
    }
}

/// 根据角色生成管理员登录会话使用的有效权限点。
pub fn session_permissions_for_role(role: &AdminRole) -> Vec<String> {
    effective_permission_keys(&role.scopes, &role.permissions)
}
