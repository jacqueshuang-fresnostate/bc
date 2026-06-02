use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UserKind {
    Regular,
    Agent,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum UserStatus {
    Active,
    Suspended,
    Locked,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminSummary {
    pub id: String,
    pub username: String,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminSaveRequest {
    pub id: String,
    pub username: String,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl AdminSaveRequest {
    pub fn summary(&self) -> AdminSummary {
        AdminSummary {
            id: self.id.clone(),
            username: self.username.clone(),
            role_id: self.role_id.clone(),
            role_name: self.role_name.clone(),
            status: self.status.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationConfig {
    pub username_enabled: bool,
    pub email_enabled: bool,
    pub agent_invite_required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserStatusRequest {
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminStatusRequest {
    pub status: UserStatus,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AdminPasswordResetRequest {
    pub password: String,
}
