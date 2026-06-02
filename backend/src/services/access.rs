use std::{
    collections::{BTreeMap, HashSet},
    sync::{Arc, RwLock},
};

use crate::{
    domain::{
        permission::{AdminRole, PermissionScope, SystemSetting, UpdateSystemSettingRequest},
        user::{AdminSummary, RegistrationConfig, UserStatus, UserSummary},
    },
    error::{ApiError, ApiResult},
};

#[derive(Debug, Clone)]
pub struct AccessSnapshot {
    pub users: Vec<UserSummary>,
    pub admins: Vec<AdminSummary>,
    pub roles: Vec<AdminRole>,
    pub settings: Vec<SystemSetting>,
    pub registration: RegistrationConfig,
}

#[derive(Clone)]
pub struct AccessRepository {
    inner: Arc<RwLock<AccessStore>>,
}

impl AccessRepository {
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AccessStore::seeded())),
        }
    }

    pub async fn snapshot(&self) -> ApiResult<AccessSnapshot> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.snapshot())
    }

    pub async fn users(&self) -> ApiResult<Vec<UserSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.users())
    }

    pub async fn get_user(&self, id: &str) -> ApiResult<UserSummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .get_user(id)
    }

    pub async fn create_user(&self, user: UserSummary) -> ApiResult<UserSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .create_user(user)
    }

    pub async fn update_user(&self, id: &str, user: UserSummary) -> ApiResult<UserSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .update_user(id, user)
    }

    pub async fn set_user_status(&self, id: &str, status: UserStatus) -> ApiResult<UserSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .set_user_status(id, status)
    }

    pub async fn admins(&self) -> ApiResult<Vec<AdminSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.admins())
    }

    pub async fn get_admin(&self, id: &str) -> ApiResult<AdminSummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .get_admin(id)
    }

    pub async fn create_admin(&self, admin: AdminSummary) -> ApiResult<AdminSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .create_admin(admin)
    }

    pub async fn update_admin(&self, id: &str, admin: AdminSummary) -> ApiResult<AdminSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .update_admin(id, admin)
    }

    pub async fn set_admin_status(&self, id: &str, status: UserStatus) -> ApiResult<AdminSummary> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .set_admin_status(id, status)
    }

    pub async fn roles(&self) -> ApiResult<Vec<AdminRole>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.roles())
    }

    pub async fn get_role(&self, id: &str) -> ApiResult<AdminRole> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .get_role(id)
    }

    pub async fn create_role(&self, role: AdminRole) -> ApiResult<AdminRole> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .create_role(role)
    }

    pub async fn update_role(&self, id: &str, role: AdminRole) -> ApiResult<AdminRole> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .update_role(id, role)
    }

    pub async fn delete_role(&self, id: &str) -> ApiResult<AdminRole> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .delete_role(id)
    }

    pub async fn settings(&self) -> ApiResult<Vec<SystemSetting>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.settings())
    }

    pub async fn update_setting(
        &self,
        key: &str,
        payload: UpdateSystemSettingRequest,
    ) -> ApiResult<SystemSetting> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .update_setting(key, payload)
    }

    pub async fn registration(&self) -> ApiResult<RegistrationConfig> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.registration.clone())
    }

    pub async fn update_registration(
        &self,
        registration: RegistrationConfig,
    ) -> ApiResult<RegistrationConfig> {
        self.inner
            .write()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .update_registration(registration)
    }
}

#[derive(Debug)]
struct AccessStore {
    users: BTreeMap<String, UserSummary>,
    admins: BTreeMap<String, AdminSummary>,
    roles: BTreeMap<String, AdminRole>,
    settings: BTreeMap<String, SystemSetting>,
    registration: RegistrationConfig,
}

impl AccessStore {
    fn seeded() -> Self {
        let roles = seed_roles()
            .into_iter()
            .map(|role| (role.id.clone(), role))
            .collect();
        let admins = seed_admins()
            .into_iter()
            .map(|admin| (admin.id.clone(), admin))
            .collect();
        let users = seed_users()
            .into_iter()
            .map(|user| (user.id.clone(), user))
            .collect();
        let settings = seed_settings()
            .into_iter()
            .map(|setting| (setting.key.clone(), setting))
            .collect();

        Self {
            users,
            admins,
            roles,
            settings,
            registration: RegistrationConfig {
                username_enabled: true,
                email_enabled: false,
                agent_invite_required: false,
            },
        }
    }

    fn snapshot(&self) -> AccessSnapshot {
        AccessSnapshot {
            users: self.users(),
            admins: self.admins(),
            roles: self.roles(),
            settings: self.settings(),
            registration: self.registration.clone(),
        }
    }

    fn users(&self) -> Vec<UserSummary> {
        self.users.values().cloned().collect()
    }

    fn get_user(&self, id: &str) -> ApiResult<UserSummary> {
        self.users
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))
    }

    fn create_user(&mut self, user: UserSummary) -> ApiResult<UserSummary> {
        let user = normalize_user(user)?;
        if self.users.contains_key(&user.id) {
            return Err(ApiError::Conflict(format!(
                "user `{}` already exists",
                user.id
            )));
        }

        self.users.insert(user.id.clone(), user.clone());
        Ok(user)
    }

    fn update_user(&mut self, id: &str, user: UserSummary) -> ApiResult<UserSummary> {
        let user = normalize_user(user)?;
        if id != user.id {
            return Err(ApiError::BadRequest(
                "path id must match user id".to_string(),
            ));
        }
        if !self.users.contains_key(id) {
            return Err(ApiError::NotFound(format!("user `{id}` not found")));
        }

        self.users.insert(id.to_string(), user.clone());
        Ok(user)
    }

    fn set_user_status(&mut self, id: &str, status: UserStatus) -> ApiResult<UserSummary> {
        let user = self
            .users
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))?;
        user.status = status;
        Ok(user.clone())
    }

    fn admins(&self) -> Vec<AdminSummary> {
        self.admins.values().cloned().collect()
    }

    fn get_admin(&self, id: &str) -> ApiResult<AdminSummary> {
        self.admins
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("admin `{id}` not found")))
    }

    fn create_admin(&mut self, admin: AdminSummary) -> ApiResult<AdminSummary> {
        let admin = normalize_admin(admin, &self.roles)?;
        if self.admins.contains_key(&admin.id) {
            return Err(ApiError::Conflict(format!(
                "admin `{}` already exists",
                admin.id
            )));
        }

        self.admins.insert(admin.id.clone(), admin.clone());
        Ok(admin)
    }

    fn update_admin(&mut self, id: &str, admin: AdminSummary) -> ApiResult<AdminSummary> {
        let admin = normalize_admin(admin, &self.roles)?;
        if id != admin.id {
            return Err(ApiError::BadRequest(
                "path id must match admin id".to_string(),
            ));
        }
        if !self.admins.contains_key(id) {
            return Err(ApiError::NotFound(format!("admin `{id}` not found")));
        }

        self.admins.insert(id.to_string(), admin.clone());
        Ok(admin)
    }

    fn set_admin_status(&mut self, id: &str, status: UserStatus) -> ApiResult<AdminSummary> {
        let admin = self
            .admins
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("admin `{id}` not found")))?;
        admin.status = status;
        Ok(admin.clone())
    }

    fn roles(&self) -> Vec<AdminRole> {
        self.roles.values().cloned().collect()
    }

    fn get_role(&self, id: &str) -> ApiResult<AdminRole> {
        self.roles
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("role `{id}` not found")))
    }

    fn create_role(&mut self, role: AdminRole) -> ApiResult<AdminRole> {
        let role = normalize_role(role)?;
        if self.roles.contains_key(&role.id) {
            return Err(ApiError::Conflict(format!(
                "role `{}` already exists",
                role.id
            )));
        }

        self.roles.insert(role.id.clone(), role.clone());
        Ok(role)
    }

    fn update_role(&mut self, id: &str, role: AdminRole) -> ApiResult<AdminRole> {
        let role = normalize_role(role)?;
        if id != role.id {
            return Err(ApiError::BadRequest(
                "path id must match role id".to_string(),
            ));
        }
        if !self.roles.contains_key(id) {
            return Err(ApiError::NotFound(format!("role `{id}` not found")));
        }

        self.roles.insert(id.to_string(), role.clone());
        self.sync_admin_role_names(id);
        Ok(role)
    }

    fn delete_role(&mut self, id: &str) -> ApiResult<AdminRole> {
        if self.admins.values().any(|admin| admin.role_id == id) {
            return Err(ApiError::Conflict(format!(
                "role `{id}` is assigned to an admin"
            )));
        }

        self.roles
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("role `{id}` not found")))
    }

    fn settings(&self) -> Vec<SystemSetting> {
        self.settings.values().cloned().collect()
    }

    fn update_setting(
        &mut self,
        key: &str,
        payload: UpdateSystemSettingRequest,
    ) -> ApiResult<SystemSetting> {
        let setting = self
            .settings
            .get_mut(key)
            .ok_or_else(|| ApiError::NotFound(format!("setting `{key}` not found")))?;
        let value = payload.value.trim();
        if value.is_empty() {
            return Err(ApiError::BadRequest(
                "setting value is required".to_string(),
            ));
        }

        setting.value = value.to_string();
        Ok(setting.clone())
    }

    fn update_registration(
        &mut self,
        registration: RegistrationConfig,
    ) -> ApiResult<RegistrationConfig> {
        if !registration.username_enabled && !registration.email_enabled {
            return Err(ApiError::BadRequest(
                "username or email registration must be enabled".to_string(),
            ));
        }

        self.registration = registration;
        Ok(self.registration.clone())
    }

    fn sync_admin_role_names(&mut self, role_id: &str) {
        let Some(role) = self.roles.get(role_id) else {
            return;
        };
        for admin in self.admins.values_mut() {
            if admin.role_id == role_id {
                admin.role_name = role.name.clone();
            }
        }
    }
}

fn normalize_user(mut user: UserSummary) -> ApiResult<UserSummary> {
    user.id = required_trimmed(user.id, "user id")?;
    user.username = required_trimmed(user.username, "username")?;
    user.email = user
        .email
        .map(|email| email.trim().to_string())
        .filter(|email| !email.is_empty());
    user.agent_id = user
        .agent_id
        .map(|agent_id| agent_id.trim().to_string())
        .filter(|agent_id| !agent_id.is_empty());

    if user.balance_minor < 0 {
        return Err(ApiError::BadRequest(
            "user balance must not be negative".to_string(),
        ));
    }

    Ok(user)
}

fn normalize_admin(
    mut admin: AdminSummary,
    roles: &BTreeMap<String, AdminRole>,
) -> ApiResult<AdminSummary> {
    admin.id = required_trimmed(admin.id, "admin id")?;
    admin.username = required_trimmed(admin.username, "admin username")?;
    admin.role_id = required_trimmed(admin.role_id, "admin role id")?;
    let role = roles
        .get(&admin.role_id)
        .ok_or_else(|| ApiError::NotFound(format!("role `{}` not found", admin.role_id)))?;
    admin.role_name = role.name.clone();

    Ok(admin)
}

fn normalize_role(mut role: AdminRole) -> ApiResult<AdminRole> {
    role.id = required_trimmed(role.id, "role id")?;
    role.name = required_trimmed(role.name, "role name")?;
    if role.scopes.is_empty() {
        return Err(ApiError::BadRequest(
            "at least one permission scope is required".to_string(),
        ));
    }

    let mut seen = HashSet::new();
    role.scopes.retain(|scope| seen.insert(scope.clone()));
    Ok(role)
}

fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

fn seed_users() -> Vec<UserSummary> {
    vec![
        UserSummary {
            id: "U10001".to_string(),
            username: "demo_user".to_string(),
            email: Some("demo@example.com".to_string()),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 12_000,
            agent_id: Some("U90001".to_string()),
        },
        UserSummary {
            id: "U90001".to_string(),
            username: "agent_alpha".to_string(),
            email: None,
            kind: crate::domain::user::UserKind::Agent,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: None,
        },
        UserSummary {
            id: "U10004".to_string(),
            username: "risk_watch".to_string(),
            email: None,
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Suspended,
            balance_minor: 0,
            agent_id: Some("U90001".to_string()),
        },
    ]
}

fn seed_admins() -> Vec<AdminSummary> {
    vec![
        AdminSummary {
            id: "A10001".to_string(),
            username: "admin".to_string(),
            role_id: "role-super".to_string(),
            role_name: "超级管理员".to_string(),
            status: UserStatus::Active,
        },
        AdminSummary {
            id: "A10002".to_string(),
            username: "locked_admin".to_string(),
            role_id: "role-ops".to_string(),
            role_name: "运营管理员".to_string(),
            status: UserStatus::Locked,
        },
    ]
}

fn seed_roles() -> Vec<AdminRole> {
    vec![
        AdminRole {
            id: "role-super".to_string(),
            name: "超级管理员".to_string(),
            scopes: vec![
                PermissionScope::Users,
                PermissionScope::Orders,
                PermissionScope::Finance,
                PermissionScope::CustomerService,
                PermissionScope::Admins,
                PermissionScope::Roles,
                PermissionScope::SystemSettings,
                PermissionScope::Lotteries,
                PermissionScope::Robots,
                PermissionScope::Rebates,
            ],
        },
        AdminRole {
            id: "role-ops".to_string(),
            name: "运营管理员".to_string(),
            scopes: vec![
                PermissionScope::Users,
                PermissionScope::Orders,
                PermissionScope::Lotteries,
            ],
        },
    ]
}

fn seed_settings() -> Vec<SystemSetting> {
    vec![
        SystemSetting {
            key: "email_registration_enabled".to_string(),
            value: "false".to_string(),
            description: "是否开启邮箱注册".to_string(),
        },
        SystemSetting {
            key: "recharge_rebate_mode".to_string(),
            value: "immediate".to_string(),
            description: "代理充值返利模式".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::UserKind;

    #[tokio::test]
    async fn access_repository_creates_and_updates_user() {
        let access = AccessRepository::memory_seeded();
        let created = access
            .create_user(UserSummary {
                id: " U20001 ".to_string(),
                username: "new_user".to_string(),
                email: Some("new@example.com".to_string()),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 1000,
                agent_id: None,
            })
            .await
            .expect("user can be created");

        assert_eq!(created.id, "U20001");

        let updated = access
            .set_user_status("U20001", UserStatus::Locked)
            .await
            .expect("status can be updated");
        assert_eq!(updated.status, UserStatus::Locked);
    }

    #[tokio::test]
    async fn access_repository_rejects_empty_role_scopes() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .create_role(AdminRole {
                id: "role-empty".to_string(),
                name: "空角色".to_string(),
                scopes: Vec::new(),
            })
            .await
            .expect_err("empty scopes must be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn access_repository_prevents_deleting_assigned_role() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .delete_role("role-super")
            .await
            .expect_err("assigned role cannot be deleted");

        assert!(matches!(error, ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn access_repository_syncs_admin_role_name_after_role_update() {
        let access = AccessRepository::memory_seeded();
        access
            .update_role(
                "role-ops",
                AdminRole {
                    id: "role-ops".to_string(),
                    name: "运营主管".to_string(),
                    scopes: vec![PermissionScope::Users],
                },
            )
            .await
            .expect("role can be updated");

        let admin = access
            .get_admin("A10002")
            .await
            .expect("admin can be fetched");
        assert_eq!(admin.role_name, "运营主管");
    }

    #[tokio::test]
    async fn access_repository_rejects_closed_registration() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .update_registration(RegistrationConfig {
                username_enabled: false,
                email_enabled: false,
                agent_invite_required: false,
            })
            .await
            .expect_err("all registration methods cannot be disabled");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }
}
