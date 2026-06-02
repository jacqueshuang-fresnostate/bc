use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRole {
    pub id: String,
    pub name: String,
    pub scopes: Vec<PermissionScope>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSetting {
    pub key: String,
    pub value: String,
    pub description: String,
}
