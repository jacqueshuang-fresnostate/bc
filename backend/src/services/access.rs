//! 权限与账号服务，提供用户、管理员、角色和系统设置的状态管理

use std::{
    collections::{BTreeMap, HashSet},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{
    domain::{
        auth::{AdminAuthSession, AdminLoginRequest},
        permission::{AdminRole, PermissionScope, SystemSetting, UpdateSystemSettingRequest},
        user::{
            AdminPasswordResetRequest, AdminSaveRequest, AdminSummary, RegistrationConfig,
            UserStatus, UserSummary,
        },
    },
    error::{ApiError, ApiResult},
};

use super::business_database::{enum_from_string, enum_to_string, to_json, BusinessDatabase};

const DEFAULT_SEED_ADMIN_PASSWORD: &str = "admin123";
const MIN_ADMIN_PASSWORD_LEN: usize = 8;
const INVITE_CODE_LENGTH: usize = 8;
const INVITE_CODE_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DEMO_USER_INVITE_CODE: &str = "QWERTYPA";
const DEMO_AGENT_INVITE_CODE: &str = "KJHGFDSA";
const RISK_USER_INVITE_CODE: &str = "ZXCVBNML";

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
    persistence: Option<BusinessDatabase>,
}

impl AccessRepository {
    /// 创建一个只依赖种子数据的内存访问仓储，适配测试和本地开发场景。
    pub fn memory_seeded() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AccessStore::seeded())),
            persistence: None,
        }
    }

    /// 用数据库持久化初始化仓储，启动时会从 business database 回放所有用户与权限状态。
    pub async fn persistent(persistence: BusinessDatabase) -> ApiResult<Self> {
        let store = load_access_store(&persistence).await?;
        Ok(Self {
            inner: Arc::new(RwLock::new(store)),
            persistence: Some(persistence),
        })
    }

    /// 返回访问控制模块的聚合快照。
    pub async fn snapshot(&self) -> ApiResult<AccessSnapshot> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.snapshot())
    }

    /// 返回完整用户列表，供后台用户管理页和导出需求复用。
    pub async fn users(&self) -> ApiResult<Vec<UserSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.users())
    }

    /// 获取单个用户详情，找不到用户返回 NotFound。
    pub async fn get_user(&self, id: &str) -> ApiResult<UserSummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .get_user(id)
    }

    /// 新增用户：先在内存层校验并补全邀请码（空则自动生成），再落库并同步持久化。
    pub async fn create_user(&self, user: UserSummary) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.create_user(user)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 更新用户：检查路径 ID 与载荷 ID 一致、保留原邀请码，完成唯一性与持久化更新。
    pub async fn update_user(&self, id: &str, user: UserSummary) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.update_user(id, user)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 修改用户状态（启用/锁定/禁用），用于快速停用异常账户。
    pub async fn set_user_status(&self, id: &str, status: UserStatus) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.set_user_status(id, status)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 返回全部管理员列表。
    pub async fn admins(&self) -> ApiResult<Vec<AdminSummary>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.admins())
    }

    /// 按 ID 查询管理员详情。
    pub async fn get_admin(&self, id: &str) -> ApiResult<AdminSummary> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .get_admin(id)
    }

    /// 创建管理员并返回新记录。
    pub async fn create_admin(&self, admin: AdminSaveRequest) -> ApiResult<AdminSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.create_admin(admin)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 更新管理员信息。
    pub async fn update_admin(&self, id: &str, admin: AdminSaveRequest) -> ApiResult<AdminSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.update_admin(id, admin)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 变更管理员状态。
    pub async fn set_admin_status(&self, id: &str, status: UserStatus) -> ApiResult<AdminSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.set_admin_status(id, status)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 重置管理员密码。
    pub async fn reset_admin_password(
        &self,
        id: &str,
        payload: AdminPasswordResetRequest,
    ) -> ApiResult<AdminSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.reset_admin_password(id, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 返回全部角色列表。
    pub async fn roles(&self) -> ApiResult<Vec<AdminRole>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.roles())
    }

    /// 按 ID 查询角色。
    pub async fn get_role(&self, id: &str) -> ApiResult<AdminRole> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .get_role(id)
    }

    /// 新增角色。
    pub async fn create_role(&self, role: AdminRole) -> ApiResult<AdminRole> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.create_role(role)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 更新角色信息。
    pub async fn update_role(&self, id: &str, role: AdminRole) -> ApiResult<AdminRole> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.update_role(id, role)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 删除角色记录。
    pub async fn delete_role(&self, id: &str) -> ApiResult<AdminRole> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.delete_role(id)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 返回系统全部设置。
    pub async fn settings(&self) -> ApiResult<Vec<SystemSetting>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.settings())
    }

    /// 更新系统设置项。
    pub async fn update_setting(
        &self,
        key: &str,
        payload: UpdateSystemSettingRequest,
    ) -> ApiResult<SystemSetting> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.update_setting(key, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 返回注册策略。
    pub async fn registration(&self) -> ApiResult<RegistrationConfig> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| store.registration.clone())
    }

    /// 更新注册策略。
    pub async fn update_registration(
        &self,
        registration: RegistrationConfig,
    ) -> ApiResult<RegistrationConfig> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.update_registration(registration)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 执行管理员登录并返回会话。
    pub async fn login(&self, payload: AdminLoginRequest) -> ApiResult<AdminAuthSession> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.login(payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 根据 Token 还原管理员会话。
    pub async fn session_from_token(&self, token: &str) -> ApiResult<AdminAuthSession> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .session_from_token(token)
    }

    /// 退出登录并清理 Token。
    pub async fn logout(&self, token: &str) -> ApiResult<()> {
        let snapshot = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            store.logout(token)?;
            store.clone()
        };
        self.persist(&snapshot).await
    }

    async fn persist(&self, store: &AccessStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_access_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AccessStore {
    users: BTreeMap<String, UserSummary>,
    admins: BTreeMap<String, AdminSummary>,
    admin_password_hashes: BTreeMap<String, String>,
    roles: BTreeMap<String, AdminRole>,
    sessions: BTreeMap<String, String>,
    settings: BTreeMap<String, SystemSetting>,
    session_counter: u64,
    registration: RegistrationConfig,
}

async fn load_access_store(database: &BusinessDatabase) -> ApiResult<AccessStore> {
    let pool = database.pool();
    let mut users = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, username, email, kind, status, balance_minor, agent_id, invite_code
         FROM users
         ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?;
        users.insert(
            id.clone(),
            UserSummary {
                id,
                username: row
                    .try_get("username")
                    .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                email: row
                    .try_get("email")
                    .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                kind: enum_from_string(
                    row.try_get("kind")
                        .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                )?,
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                )?,
                balance_minor: row
                    .try_get("balance_minor")
                    .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                agent_id: row
                    .try_get("agent_id")
                    .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                invite_code: row
                    .try_get("invite_code")
                    .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
            },
        );
    }

    let mut roles = BTreeMap::new();
    for row in sqlx::query("SELECT id, name, scopes FROM admin_roles ORDER BY id ASC")
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::Internal("角色数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("角色数据读取失败".to_string()))?;
        roles.insert(
            id.clone(),
            AdminRole {
                id,
                name: row
                    .try_get("name")
                    .map_err(|_| ApiError::Internal("角色数据读取失败".to_string()))?,
                scopes: super::business_database::from_json(
                    row.try_get("scopes")
                        .map_err(|_| ApiError::Internal("角色数据读取失败".to_string()))?,
                )?,
            },
        );
    }

    let mut admins = BTreeMap::new();
    for row in
        sqlx::query("SELECT id, username, role_id, role_name, status FROM admins ORDER BY id ASC")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::Internal("管理员数据读取失败".to_string()))?
    {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("管理员数据读取失败".to_string()))?;
        admins.insert(
            id.clone(),
            AdminSummary {
                id,
                username: row
                    .try_get("username")
                    .map_err(|_| ApiError::Internal("管理员数据读取失败".to_string()))?,
                role_id: row
                    .try_get("role_id")
                    .map_err(|_| ApiError::Internal("管理员数据读取失败".to_string()))?,
                role_name: row
                    .try_get("role_name")
                    .map_err(|_| ApiError::Internal("管理员数据读取失败".to_string()))?,
                status: enum_from_string(
                    row.try_get("status")
                        .map_err(|_| ApiError::Internal("管理员数据读取失败".to_string()))?,
                )?,
            },
        );
    }

    let admin_password_hashes =
        sqlx::query("SELECT admin_id, password_hash FROM admin_password_hashes")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::Internal("管理员密码数据读取失败".to_string()))?
            .into_iter()
            .map(|row| {
                let admin_id = row
                    .try_get("admin_id")
                    .map_err(|_| ApiError::Internal("管理员密码数据读取失败".to_string()))?;
                let password_hash = row
                    .try_get("password_hash")
                    .map_err(|_| ApiError::Internal("管理员密码数据读取失败".to_string()))?;
                Ok((admin_id, password_hash))
            })
            .collect::<ApiResult<BTreeMap<String, String>>>()?;

    let sessions = sqlx::query("SELECT token, admin_id FROM admin_sessions")
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::Internal("管理员会话数据读取失败".to_string()))?
        .into_iter()
        .map(|row| {
            let token = row
                .try_get("token")
                .map_err(|_| ApiError::Internal("管理员会话数据读取失败".to_string()))?;
            let admin_id = row
                .try_get("admin_id")
                .map_err(|_| ApiError::Internal("管理员会话数据读取失败".to_string()))?;
            Ok((token, admin_id))
        })
        .collect::<ApiResult<BTreeMap<String, String>>>()?;

    let mut settings = BTreeMap::new();
    for row in sqlx::query("SELECT key, value, description FROM system_settings ORDER BY key ASC")
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::Internal("系统设置数据读取失败".to_string()))?
    {
        let key: String = row
            .try_get("key")
            .map_err(|_| ApiError::Internal("系统设置数据读取失败".to_string()))?;
        settings.insert(
            key.clone(),
            SystemSetting {
                key,
                value: row
                    .try_get("value")
                    .map_err(|_| ApiError::Internal("系统设置数据读取失败".to_string()))?,
                description: row
                    .try_get("description")
                    .map_err(|_| ApiError::Internal("系统设置数据读取失败".to_string()))?,
            },
        );
    }

    let registration = sqlx::query(
        "SELECT username_enabled, email_enabled, agent_invite_required
         FROM registration_config
         WHERE id = 'default'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("注册配置数据读取失败".to_string()))?
    .map(|row| {
        Ok(RegistrationConfig {
            username_enabled: row
                .try_get("username_enabled")
                .map_err(|_| ApiError::Internal("注册配置数据读取失败".to_string()))?,
            email_enabled: row
                .try_get("email_enabled")
                .map_err(|_| ApiError::Internal("注册配置数据读取失败".to_string()))?,
            agent_invite_required: row
                .try_get("agent_invite_required")
                .map_err(|_| ApiError::Internal("注册配置数据读取失败".to_string()))?,
        })
    })
    .transpose()?;

    let session_counter = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM access_runtime WHERE key = 'session_counter'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("用户权限运行数据读取失败".to_string()))?
    .unwrap_or_default();

    let Some(registration) = registration else {
        let seeded = AccessStore::seeded();
        save_access_store(database, &seeded).await?;
        return Ok(seeded);
    };

    if users.is_empty() && admins.is_empty() && roles.is_empty() && settings.is_empty() {
        let seeded = AccessStore::seeded();
        save_access_store(database, &seeded).await?;
        return Ok(seeded);
    }

    Ok(AccessStore {
        users,
        admins,
        admin_password_hashes,
        roles,
        sessions,
        settings,
        session_counter: u64::try_from(session_counter).unwrap_or_default(),
        registration,
    })
}

async fn save_access_store(database: &BusinessDatabase, store: &AccessStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("用户权限事务开启失败".to_string()))?;

    for table in [
        "admin_sessions",
        "admin_password_hashes",
        "admins",
        "admin_roles",
        "system_settings",
        "registration_config",
        "access_runtime",
        "users",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("用户权限数据清理失败".to_string()))?;
    }

    for user in store.users.values() {
        sqlx::query(
            "INSERT INTO users (id, username, email, kind, status, balance_minor, agent_id, invite_code)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(enum_to_string(&user.kind)?)
        .bind(enum_to_string(&user.status)?)
        .bind(user.balance_minor)
        .bind(&user.agent_id)
        .bind(&user.invite_code)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("用户数据保存失败".to_string()))?;
    }

    for role in store.roles.values() {
        sqlx::query("INSERT INTO admin_roles (id, name, scopes) VALUES ($1, $2, $3)")
            .bind(&role.id)
            .bind(&role.name)
            .bind(to_json(&role.scopes)?)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("角色数据保存失败".to_string()))?;
    }

    for admin in store.admins.values() {
        sqlx::query(
            "INSERT INTO admins (id, username, role_id, role_name, status)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&admin.id)
        .bind(&admin.username)
        .bind(&admin.role_id)
        .bind(&admin.role_name)
        .bind(enum_to_string(&admin.status)?)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("管理员数据保存失败".to_string()))?;
    }

    for (admin_id, password_hash) in &store.admin_password_hashes {
        sqlx::query("INSERT INTO admin_password_hashes (admin_id, password_hash) VALUES ($1, $2)")
            .bind(admin_id)
            .bind(password_hash)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("管理员密码数据保存失败".to_string()))?;
    }

    for (token, admin_id) in &store.sessions {
        sqlx::query("INSERT INTO admin_sessions (token, admin_id) VALUES ($1, $2)")
            .bind(token)
            .bind(admin_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("管理员会话数据保存失败".to_string()))?;
    }

    for setting in store.settings.values() {
        sqlx::query("INSERT INTO system_settings (key, value, description) VALUES ($1, $2, $3)")
            .bind(&setting.key)
            .bind(&setting.value)
            .bind(&setting.description)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("系统设置数据保存失败".to_string()))?;
    }

    sqlx::query(
        "INSERT INTO registration_config
         (id, username_enabled, email_enabled, agent_invite_required)
         VALUES ('default', $1, $2, $3)",
    )
    .bind(store.registration.username_enabled)
    .bind(store.registration.email_enabled)
    .bind(store.registration.agent_invite_required)
    .execute(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("注册配置数据保存失败".to_string()))?;

    let session_counter = i64::try_from(store.session_counter)
        .map_err(|_| ApiError::Internal("管理员会话序号过大".to_string()))?;
    sqlx::query("INSERT INTO access_runtime (key, value) VALUES ('session_counter', $1)")
        .bind(session_counter)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("用户权限运行数据保存失败".to_string()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("用户权限事务提交失败".to_string()))
}

impl AccessStore {
    /// 生成初始内存状态：管理员、角色、设置和用户均使用固定种子值，服务可直接启动。
    fn seeded() -> Self {
        let roles = seed_roles()
            .into_iter()
            .map(|role| (role.id.clone(), role))
            .collect();
        let admins = seed_admins()
            .into_iter()
            .map(|admin| (admin.id.clone(), admin))
            .collect();
        let admin_password_hashes = seed_admin_password_hashes(&admins);
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
            admin_password_hashes,
            roles,
            sessions: BTreeMap::new(),
            settings,
            session_counter: 0,
            registration: RegistrationConfig {
                username_enabled: true,
                email_enabled: false,
                agent_invite_required: false,
            },
        }
    }

    /// 处理 snapshot 的具体内部流程。
    fn snapshot(&self) -> AccessSnapshot {
        AccessSnapshot {
            users: self.users(),
            admins: self.admins(),
            roles: self.roles(),
            settings: self.settings(),
            registration: self.registration.clone(),
        }
    }

    /// 查询所有用户，返回可序列化的向量副本给上层接口。
    fn users(&self) -> Vec<UserSummary> {
        self.users.values().cloned().collect()
    }

    /// 按 ID 查询用户，若不存在给出明确 404 错误。
    fn get_user(&self, id: &str) -> ApiResult<UserSummary> {
        self.users
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))
    }

    /// 创建用户并落入内存集合，空邀请码时自动生成随机字母码。
    fn create_user(&mut self, user: UserSummary) -> ApiResult<UserSummary> {
        let mut user = normalize_user(user)?;
        if user.invite_code.is_empty() {
            user.invite_code = random_invite_code(&self.users)?;
        }
        self.ensure_unique_invite_code(&user.id, &user.invite_code)?;
        if self.users.contains_key(&user.id) {
            return Err(ApiError::Conflict(format!(
                "user `{}` already exists",
                user.id
            )));
        }

        self.users.insert(user.id.clone(), user.clone());
        Ok(user)
    }

    /// 更新内存用户：空邀请码会沿用数据库已有值，不会被改成空值。
    fn update_user(&mut self, id: &str, user: UserSummary) -> ApiResult<UserSummary> {
        let mut user = normalize_user(user)?;
        if id != user.id {
            return Err(ApiError::BadRequest(
                "path id must match user id".to_string(),
            ));
        }
        if !self.users.contains_key(id) {
            return Err(ApiError::NotFound(format!("user `{id}` not found")));
        }
        if user.invite_code.is_empty() {
            user.invite_code = self
                .users
                .get(id)
                .map(|existing| existing.invite_code.clone())
                .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))?;
        }
        self.ensure_unique_invite_code(id, &user.invite_code)?;

        self.users.insert(id.to_string(), user.clone());
        Ok(user)
    }

    /// 切换用户状态并返回最新用户快照。
    fn set_user_status(&mut self, id: &str, status: UserStatus) -> ApiResult<UserSummary> {
        let user = self
            .users
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))?;
        user.status = status;
        Ok(user.clone())
    }

    /// 校验邀请码未被其他用户占用，避免出现重复码导致推荐关系错误。
    fn ensure_unique_invite_code(&self, user_id: &str, invite_code: &str) -> ApiResult<()> {
        if let Some(existing) = self
            .users
            .values()
            .find(|user| user.id != user_id && user.invite_code == invite_code)
        {
            return Err(ApiError::Conflict(format!(
                "invite code `{invite_code}` is already assigned to user `{}`",
                existing.id
            )));
        }

        Ok(())
    }

    /// 处理 admins 的具体内部流程。
    fn admins(&self) -> Vec<AdminSummary> {
        self.admins.values().cloned().collect()
    }

    /// 处理 get_admin 的具体内部流程。
    fn get_admin(&self, id: &str) -> ApiResult<AdminSummary> {
        self.admins
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("admin `{id}` not found")))
    }

    /// 处理 create_admin 的具体内部流程。
    fn create_admin(&mut self, request: AdminSaveRequest) -> ApiResult<AdminSummary> {
        let password = request
            .password
            .as_deref()
            .ok_or_else(|| ApiError::BadRequest("admin password is required".to_string()))
            .and_then(validate_admin_password)?;
        let admin = normalize_admin(request.summary(), &self.roles)?;
        if self.admins.contains_key(&admin.id) {
            return Err(ApiError::Conflict(format!(
                "admin `{}` already exists",
                admin.id
            )));
        }

        let password_hash = hash_admin_password(&password)?;
        self.admin_password_hashes
            .insert(admin.id.clone(), password_hash);
        self.admins.insert(admin.id.clone(), admin.clone());
        Ok(admin)
    }

    /// 处理 update_admin 的具体内部流程。
    fn update_admin(&mut self, id: &str, request: AdminSaveRequest) -> ApiResult<AdminSummary> {
        let password = match request.password.as_deref() {
            Some(password) => Some(validate_admin_password(password)?),
            None => None,
        };
        let admin = normalize_admin(request.summary(), &self.roles)?;
        if id != admin.id {
            return Err(ApiError::BadRequest(
                "path id must match admin id".to_string(),
            ));
        }
        if !self.admins.contains_key(id) {
            return Err(ApiError::NotFound(format!("admin `{id}` not found")));
        }

        if let Some(password) = password {
            let password_hash = hash_admin_password(&password)?;
            self.admin_password_hashes
                .insert(admin.id.clone(), password_hash);
        }
        self.admins.insert(id.to_string(), admin.clone());
        Ok(admin)
    }

    /// 处理 set_admin_status 的具体内部流程。
    fn set_admin_status(&mut self, id: &str, status: UserStatus) -> ApiResult<AdminSummary> {
        let admin = self
            .admins
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("admin `{id}` not found")))?;
        admin.status = status;
        Ok(admin.clone())
    }

    /// 处理 reset_admin_password 的具体内部流程。
    fn reset_admin_password(
        &mut self,
        id: &str,
        payload: AdminPasswordResetRequest,
    ) -> ApiResult<AdminSummary> {
        let password = validate_admin_password(&payload.password)?;
        let admin = self.get_admin(id)?;
        let password_hash = hash_admin_password(&password)?;
        self.admin_password_hashes
            .insert(admin.id.clone(), password_hash);
        Ok(admin)
    }

    /// 处理 roles 的具体内部流程。
    fn roles(&self) -> Vec<AdminRole> {
        self.roles.values().cloned().collect()
    }

    /// 处理 get_role 的具体内部流程。
    fn get_role(&self, id: &str) -> ApiResult<AdminRole> {
        self.roles
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("role `{id}` not found")))
    }

    /// 处理 create_role 的具体内部流程。
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

    /// 处理 update_role 的具体内部流程。
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

    /// 处理 delete_role 的具体内部流程。
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

    /// 处理 settings 的具体内部流程。
    fn settings(&self) -> Vec<SystemSetting> {
        self.settings.values().cloned().collect()
    }

    /// 处理 update_setting 的具体内部流程。
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

    /// 处理 update_registration 的具体内部流程。
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

    /// 处理 login 的具体内部流程。
    fn login(&mut self, payload: AdminLoginRequest) -> ApiResult<AdminAuthSession> {
        let username = required_trimmed(payload.username, "admin username")?;
        let password = required_trimmed(payload.password, "admin password")?;
        let admin = self
            .admins
            .values()
            .find(|admin| admin.username == username || admin.id == username)
            .cloned()
            .ok_or_else(|| ApiError::Unauthorized("invalid admin credentials".to_string()))?;

        let password_hash = self.admin_password_hashes.get(&admin.id).ok_or_else(|| {
            ApiError::Internal(format!("admin `{}` password hash missing", admin.id))
        })?;
        if !verify_admin_password(&password, password_hash)? {
            return Err(ApiError::Unauthorized(
                "invalid admin credentials".to_string(),
            ));
        }
        if admin.status != UserStatus::Active {
            return Err(ApiError::Forbidden(
                "admin account is not active".to_string(),
            ));
        }

        let token = self.next_session_token(&admin.id)?;
        self.sessions.insert(token.clone(), admin.id.clone());
        self.session_from_token(&token)
    }

    /// 处理 session_from_token 的具体内部流程。
    fn session_from_token(&self, token: &str) -> ApiResult<AdminAuthSession> {
        let token = token.trim();
        if token.is_empty() {
            return Err(ApiError::Unauthorized(
                "authorization token is required".to_string(),
            ));
        }

        let admin_id = self
            .sessions
            .get(token)
            .ok_or_else(|| ApiError::Unauthorized("invalid admin session".to_string()))?;
        let admin = self.get_admin(admin_id)?;
        if admin.status != UserStatus::Active {
            return Err(ApiError::Forbidden(
                "admin account is not active".to_string(),
            ));
        }
        let role = self.get_role(&admin.role_id)?;

        Ok(AdminAuthSession {
            admin,
            scopes: role.scopes.clone(),
            role,
            token: token.to_string(),
        })
    }

    /// 处理 logout 的具体内部流程。
    fn logout(&mut self, token: &str) -> ApiResult<()> {
        self.sessions.remove(token.trim());
        Ok(())
    }

    /// 处理 sync_admin_role_names 的具体内部流程。
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

    /// 处理 next_session_token 的具体内部流程。
    fn next_session_token(&mut self, admin_id: &str) -> ApiResult<String> {
        self.session_counter = self.session_counter.saturating_add(1);
        let unix_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ApiError::Internal("system time is before unix epoch".to_string()))?
            .as_millis();

        Ok(format!(
            "adm-{admin_id}-{unix_millis}-{}",
            self.session_counter
        ))
    }
}

/// 统一归一化用户提交字段：去空格、检查必填项并校验余额非负数。
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
    user.invite_code = user.invite_code.trim().to_string();

    if user.balance_minor < 0 {
        return Err(ApiError::BadRequest(
            "user balance must not be negative".to_string(),
        ));
    }

    Ok(user)
}

/// 依据当前用户集合随机生成 8 位大写字母邀请码，最多尝试 128 次防止极端碰撞。
fn random_invite_code(users: &BTreeMap<String, UserSummary>) -> ApiResult<String> {
    for _ in 0..128 {
        let mut bytes = [0u8; INVITE_CODE_LENGTH];
        OsRng.fill_bytes(&mut bytes);
        let code = bytes
            .iter()
            .map(|byte| {
                let index = usize::from(*byte % INVITE_CODE_ALPHABET.len() as u8);
                INVITE_CODE_ALPHABET[index] as char
            })
            .collect::<String>();

        if !users.values().any(|user| user.invite_code == code) {
            return Ok(code);
        }
    }

    Err(ApiError::Internal("邀请码生成失败".to_string()))
}

/// 标准化输入并返回规范值。
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

/// 校验输入参数并返回校验结果。
fn validate_admin_password(password: &str) -> ApiResult<String> {
    let password = required_trimmed(password.to_string(), "admin password")?;
    if password.chars().count() < MIN_ADMIN_PASSWORD_LEN {
        return Err(ApiError::BadRequest(format!(
            "admin password must be at least {MIN_ADMIN_PASSWORD_LEN} characters"
        )));
    }

    Ok(password)
}

/// 处理 hash_admin_password 的具体内部流程。
fn hash_admin_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::Internal("admin password hash failed".to_string()))
}

/// 处理 verify_admin_password 的具体内部流程。
fn verify_admin_password(password: &str, password_hash: &str) -> ApiResult<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| ApiError::Internal("admin password hash is invalid".to_string()))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// 标准化输入并返回规范值。
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

/// 去除空白并校验必填字段。
fn required_trimmed(value: String, label: &str) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(ApiError::BadRequest(format!("{label} is required")));
    }
    Ok(value)
}

/// 加载系统内置用户种子，给每个演示用户分配固定的邀请码。
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
            invite_code: DEMO_USER_INVITE_CODE.to_string(),
        },
        UserSummary {
            id: "U90001".to_string(),
            username: "agent_alpha".to_string(),
            email: None,
            kind: crate::domain::user::UserKind::Agent,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: None,
            invite_code: DEMO_AGENT_INVITE_CODE.to_string(),
        },
        UserSummary {
            id: "U10004".to_string(),
            username: "risk_watch".to_string(),
            email: None,
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Suspended,
            balance_minor: 0,
            agent_id: Some("U90001".to_string()),
            invite_code: RISK_USER_INVITE_CODE.to_string(),
        },
    ]
}

/// 返回内置种子或测试数据。
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

/// 返回内置种子或测试数据。
fn seed_admin_password_hashes(admins: &BTreeMap<String, AdminSummary>) -> BTreeMap<String, String> {
    admins
        .keys()
        .map(|admin_id| {
            let hash = hash_admin_password(DEFAULT_SEED_ADMIN_PASSWORD)
                .unwrap_or_else(|_| panic!("种子管理员密码哈希失败"));
            (admin_id.clone(), hash)
        })
        .collect()
}

/// 返回内置种子或测试数据。
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

/// 返回内置种子或测试数据。
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
                invite_code: String::new(),
            })
            .await
            .expect("user can be created");

        assert_eq!(created.id, "U20001");
        assert_eq!(created.invite_code.len(), INVITE_CODE_LENGTH);
        assert!(created
            .invite_code
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() && ch.is_ascii_uppercase()));

        let updated = access
            .set_user_status("U20001", UserStatus::Locked)
            .await
            .expect("status can be updated");
        assert_eq!(updated.status, UserStatus::Locked);
    }

    #[tokio::test]
    async fn access_repository_rejects_duplicate_invite_code() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .create_user(UserSummary {
                id: "U20002".to_string(),
                username: "duplicate_invite_code".to_string(),
                email: None,
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: DEMO_AGENT_INVITE_CODE.to_string(),
            })
            .await
            .expect_err("duplicate invite code must be rejected");

        assert!(matches!(error, ApiError::Conflict(_)));
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

    #[tokio::test]
    async fn access_repository_logs_in_active_admin() {
        let access = AccessRepository::memory_seeded();
        let session = access
            .login(AdminLoginRequest {
                username: "admin".to_string(),
                password: "admin123".to_string(),
            })
            .await
            .expect("active admin can login");

        assert_eq!(session.admin.id, "A10001");
        assert!(session.scopes.contains(&PermissionScope::Admins));

        let current = access
            .session_from_token(&session.token)
            .await
            .expect("session token can be resolved");
        assert_eq!(current.admin.username, "admin");
    }

    #[tokio::test]
    async fn access_repository_rejects_locked_admin_login() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .login(AdminLoginRequest {
                username: "locked_admin".to_string(),
                password: "admin123".to_string(),
            })
            .await
            .expect_err("locked admin must be rejected");

        assert!(matches!(error, ApiError::Forbidden(_)));
    }

    #[tokio::test]
    async fn access_repository_rejects_wrong_admin_password() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .login(AdminLoginRequest {
                username: "admin".to_string(),
                password: "wrong-password".to_string(),
            })
            .await
            .expect_err("wrong password must be rejected");

        assert!(matches!(error, ApiError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn access_repository_creates_admin_with_individual_password() {
        let access = AccessRepository::memory_seeded();
        let created = access
            .create_admin(AdminSaveRequest {
                id: "A20001".to_string(),
                username: "ops_admin".to_string(),
                role_id: "role-ops".to_string(),
                role_name: String::new(),
                status: UserStatus::Active,
                password: Some("opsSecret123".to_string()),
            })
            .await
            .expect("admin can be created");

        assert_eq!(created.role_name, "运营管理员");

        let session = access
            .login(AdminLoginRequest {
                username: "ops_admin".to_string(),
                password: "opsSecret123".to_string(),
            })
            .await
            .expect("new admin can login with individual password");
        assert_eq!(session.admin.id, "A20001");
    }

    #[tokio::test]
    async fn access_repository_resets_admin_password() {
        let access = AccessRepository::memory_seeded();
        access
            .create_admin(AdminSaveRequest {
                id: "A20002".to_string(),
                username: "reset_admin".to_string(),
                role_id: "role-ops".to_string(),
                role_name: String::new(),
                status: UserStatus::Active,
                password: Some("beforeReset123".to_string()),
            })
            .await
            .expect("admin can be created");

        access
            .reset_admin_password(
                "A20002",
                AdminPasswordResetRequest {
                    password: "afterReset123".to_string(),
                },
            )
            .await
            .expect("password can be reset");

        let old_error = access
            .login(AdminLoginRequest {
                username: "reset_admin".to_string(),
                password: "beforeReset123".to_string(),
            })
            .await
            .expect_err("old password must be rejected");
        assert!(matches!(old_error, ApiError::Unauthorized(_)));

        let session = access
            .login(AdminLoginRequest {
                username: "reset_admin".to_string(),
                password: "afterReset123".to_string(),
            })
            .await
            .expect("new password can login");
        assert_eq!(session.admin.id, "A20002");
    }

    #[tokio::test]
    async fn access_repository_rejects_short_admin_password() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .create_admin(AdminSaveRequest {
                id: "A20003".to_string(),
                username: "short_password_admin".to_string(),
                role_id: "role-ops".to_string(),
                role_name: String::new(),
                status: UserStatus::Active,
                password: Some("short".to_string()),
            })
            .await
            .expect_err("short password must be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn access_repository_requires_password_for_new_admin() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .create_admin(AdminSaveRequest {
                id: "A20004".to_string(),
                username: "missing_password_admin".to_string(),
                role_id: "role-ops".to_string(),
                role_name: String::new(),
                status: UserStatus::Active,
                password: None,
            })
            .await
            .expect_err("new admin must include an initial password");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn access_repository_invalidates_logout_token() {
        let access = AccessRepository::memory_seeded();
        let session = access
            .login(AdminLoginRequest {
                username: "admin".to_string(),
                password: "admin123".to_string(),
            })
            .await
            .expect("active admin can login");

        access
            .logout(&session.token)
            .await
            .expect("logout should succeed");
        let error = access
            .session_from_token(&session.token)
            .await
            .expect_err("logged out token must be invalid");

        assert!(matches!(error, ApiError::Unauthorized(_)));
    }
}
