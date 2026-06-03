//! 权限与角色领域模型，承载系统设置与角色范围定义

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PermissionScope {
    Users,
    Orders,
    Finance,
    CustomerService,
    Admins,
    Roles,
    SystemSettings,
    Lotteries,
    Robots,
    Rebates,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminRole {
    pub id: String,
    pub name: String,
    pub scopes: Vec<PermissionScope>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SystemSetting {
    pub key: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSystemSettingRequest {
    pub value: String,
}
