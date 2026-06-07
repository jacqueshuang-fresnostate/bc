//! 权限与角色领域模型，承载系统设置与角色范围定义

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
/// 后台权限范围，控制管理员可以访问的业务模块。
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
/// 管理员角色，绑定角色名称和权限范围集合。
pub struct AdminRole {
    pub id: String,
    pub name: String,
    pub scopes: Vec<PermissionScope>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 系统设置项，保存后台可维护的运行配置。
pub struct SystemSetting {
    pub key: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 更新单个系统设置值时提交的请求。
pub struct UpdateSystemSettingRequest {
    pub value: String,
}
