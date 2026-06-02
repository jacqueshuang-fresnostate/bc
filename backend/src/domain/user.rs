use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UserKind {
    Regular,
    Agent,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UserStatus {
    Active,
    Suspended,
    Locked,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSummary {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub kind: UserKind,
    pub status: UserStatus,
    pub balance_minor: i64,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminSummary {
    pub id: String,
    pub username: String,
    pub role_name: String,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationConfig {
    pub username_enabled: bool,
    pub email_enabled: bool,
    pub agent_invite_required: bool,
}
