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
    pub fn profile(&self) -> CurrentAdminProfile {
        CurrentAdminProfile {
            admin: self.admin.clone(),
            role: self.role.clone(),
            scopes: self.scopes.clone(),
        }
    }
}
