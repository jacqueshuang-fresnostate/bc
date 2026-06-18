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
    /// 业务唯一标识。
    pub id: String,
    /// 展示名称。
    pub name: String,
    /// 角色拥有的权限范围列表。
    pub scopes: Vec<PermissionScope>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 系统设置项，保存后台可维护的运行配置。
pub struct SystemSetting {
    /// 配置键或选项键。
    pub key: String,
    /// 配置值或选项值。
    pub value: String,
    /// 配置或记录的中文说明。
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 更新单个系统设置值时提交的请求。
pub struct UpdateSystemSettingRequest {
    /// 配置值或选项值。
    pub value: String,
}
