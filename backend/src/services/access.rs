//! 权限与账号服务，提供用户、管理员、角色和系统设置的状态管理

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    net::IpAddr,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Local;
use chrono::TimeZone;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgRow, Row};

use crate::{
    domain::{
        auth::{session_permissions_for_role, AdminAuthSession, AdminLoginRequest},
        permission::{
            admin_permission_definitions, is_known_permission_key, AdminRole, PermissionScope,
            SystemSetting, UpdateSystemSettingRequest,
        },
        user::{
            AdminPasswordResetRequest, AdminSaveRequest, AdminSummary, RegistrationConfig,
            UserAuthSession, UserAvatarRequest, UserBindEmailRequest, UserChangePasswordRequest,
            UserForgotPasswordRequest, UserForgotPasswordResponse, UserKind, UserLoginRequest,
            UserLogoutResponse, UserPasswordResetRequest, UserRegisterRequest,
            UserRegistrationLocation, UserResetPasswordRequest, UserResetPasswordResponse,
            UserStatus, UserSummary, WithdrawalMethod, WithdrawalMethodRequest,
            WithdrawalMethodType,
        },
    },
    error::{ApiError, ApiResult},
};

use super::{
    business_database::{enum_from_string, enum_to_string, to_json, BusinessDatabase},
    group_buy_robot::{is_group_buy_robot_user_id, ROBOT_GROUP_BUY_USER_IDS},
    pagination::{ListPage, PageRequest},
};

const DEFAULT_SEED_ADMIN_PASSWORD: &str = "admin123";
const MIN_ADMIN_PASSWORD_LEN: usize = 8;
const INVITE_CODE_LENGTH: usize = 8;
const INVITE_CODE_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const DEMO_USER_INVITE_CODE: &str = "QWER7YPA";
const DEMO_AGENT_INVITE_CODE: &str = "KJHG8DSA";
const RISK_USER_INVITE_CODE: &str = "ZXCV9NML";
const DEFAULT_SEED_USER_PASSWORD: &str = "12345678";
const MIN_USER_PASSWORD_LEN: usize = 8;
const USER_RESET_TOKEN_BYTES: usize = 24;
const USER_RESET_TOKEN_TTL_SECONDS: i64 = 15 * 60;
const WITHDRAWAL_METHOD_ID_BYTES: usize = 6;
const SESSION_TOKEN_RANDOM_BYTES: usize = 32;
const SESSION_TOKEN_PREFIX: &str = "bcst_";
const SESSION_TOKEN_HASH_PREFIX: &str = "sha256:";
const MAX_AVATAR_URL_LEN: usize = 500;
const MIN_CONTACT_QQ_LEN: usize = 5;
const MAX_CONTACT_QQ_LEN: usize = 12;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PasswordResetTokenRecord {
    /// 关联用户 ID。
    pub user_id: String,
    /// expiresatunix字段。
    pub expires_at_unix: i64,
}

#[derive(Debug, Clone)]
/// 用户权限模块的完整快照，用于后台仪表盘和跨仓储读取。
pub struct AccessSnapshot {
    /// 用户摘要列表。
    pub users: Vec<UserSummary>,
    /// 管理员摘要列表。
    pub admins: Vec<AdminSummary>,
    /// 管理员角色列表。
    pub roles: Vec<AdminRole>,
    /// 移动端或模块配置集合。
    pub settings: Vec<SystemSetting>,
    /// 用户注册开关和限制配置。
    pub registration: RegistrationConfig,
}

#[derive(Clone)]
/// 用户、管理员、角色、会话和系统设置仓储，负责该模块数据读取、业务变更和持久化协调。
pub struct AccessRepository {
    inner: Arc<RwLock<AccessStore>>,
    persistence: Option<BusinessDatabase>,
}

/// 用户、管理员、角色、会话和系统设置仓储，负责该模块数据读取、业务变更和持久化协调。
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

    /// 按用户 ID 批量读取用户名，供后台分页表格只补当前页需要的用户名称。
    pub async fn usernames_for_ids(
        &self,
        user_ids: &[String],
    ) -> ApiResult<BTreeMap<String, String>> {
        let user_ids = normalized_user_ids(user_ids);
        if user_ids.is_empty() {
            return Ok(BTreeMap::new());
        }
        if let Some(persistence) = &self.persistence {
            let user_ids = user_ids.iter().cloned().collect::<Vec<_>>();
            return query_usernames_for_ids(persistence, &user_ids).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| {
                store
                    .users
                    .iter()
                    .filter(|(id, _)| user_ids.contains(id.as_str()))
                    .map(|(id, user)| (id.clone(), user.username.clone()))
                    .collect()
            })
    }

    /// 按用户 ID 批量读取用户摘要，供后台分页列表补充上级代理等当前页展示信息。
    pub async fn users_for_ids(&self, user_ids: &[String]) -> ApiResult<Vec<UserSummary>> {
        let user_ids = normalized_user_ids(user_ids);
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        if let Some(persistence) = &self.persistence {
            let user_ids = user_ids.iter().cloned().collect::<Vec<_>>();
            return query_users_for_ids(persistence, &user_ids).await;
        }

        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))
            .map(|store| {
                store
                    .users
                    .iter()
                    .filter(|(id, _)| user_ids.contains(id.as_str()))
                    .map(|(_, user)| user.clone())
                    .collect()
            })
    }

    /// 分页返回用户列表；数据库模式下将状态、用户名搜索、排序、余额联查和分页下推到 SQL。
    pub async fn user_page(
        &self,
        include_robot_data: bool,
        status: Option<UserStatus>,
        username: Option<&str>,
        agent_id: Option<&str>,
        sort_by: &str,
        sort_direction: &str,
        page: PageRequest,
    ) -> ApiResult<ListPage<UserSummary>> {
        let username = normalized_username_filter(username);
        let agent_id = normalized_agent_id_filter(agent_id);
        if let Some(persistence) = &self.persistence {
            return query_user_page(
                persistence,
                status,
                username.as_deref(),
                agent_id.as_deref(),
                sort_by,
                sort_direction,
                include_robot_data,
                page,
            )
            .await;
        }

        let mut users = self.users().await?;
        if !include_robot_data {
            users.retain(|user| !is_group_buy_robot_user_id(&user.id));
        }
        if let Some(status) = status.as_ref() {
            users.retain(|user| &user.status == status);
        }
        if let Some(username) = username.as_ref() {
            let username = username.to_ascii_lowercase();
            users.retain(|user| user.username.to_ascii_lowercase().contains(&username));
        }
        if let Some(agent_id) = agent_id.as_ref() {
            users.retain(|user| user.agent_id.as_deref() == Some(agent_id.as_str()));
        }
        sort_user_summaries(&mut users, sort_by, sort_direction)?;
        Ok(ListPage::from_all(users, page))
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

    /// 更新用户：检查路径 ID 与载荷 ID 一致、保留余额、邀请码和头像，完成唯一性与持久化更新。
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

    /// 删除用户账号资料，并同步清理该用户的登录凭据、会话、重置码和提现方式。
    pub async fn delete_user(&self, id: &str) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.delete_user(id)?;
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

    /// 注册用户：按当前注册策略校验输入、创建用户并保存独立密码哈希。
    pub async fn register_user(&self, payload: UserRegisterRequest) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.register_user(payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 用户登录：支持用户名和邮箱两种登录标识，返回用户会话。
    pub async fn login_user(&self, payload: UserLoginRequest) -> ApiResult<UserAuthSession> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.login_user(payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 通过用户 token 解析当前登录会话。
    pub async fn session_from_user_token(&self, token: &str) -> ApiResult<UserAuthSession> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .session_from_user_token(token)
    }

    /// 用户登出：清理 token 后返回登出结果。
    pub async fn logout_user(&self, token: &str) -> ApiResult<UserLogoutResponse> {
        let snapshot = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            store.logout_user(token);
            store.clone()
        };
        self.persist(&snapshot).await?;

        Ok(UserLogoutResponse { logged_out: true })
    }

    /// 绑定邮箱：后续登录可直接用邮箱作为登录凭证。
    pub async fn bind_email(
        &self,
        user_id: &str,
        payload: UserBindEmailRequest,
    ) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.bind_email(user_id, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 设置当前用户头像：只更新头像链接，不影响邀请码、余额等受控字段。
    pub async fn update_user_avatar(
        &self,
        user_id: &str,
        payload: UserAvatarRequest,
    ) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.update_user_avatar(user_id, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 修改密码：校验旧密码后写入新密码哈希。
    pub async fn change_password(
        &self,
        user_id: &str,
        payload: UserChangePasswordRequest,
    ) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.change_password(user_id, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 发起忘记密码流程：返回重置码和过期时间。
    pub async fn request_forgot_password(
        &self,
        payload: UserForgotPasswordRequest,
    ) -> ApiResult<UserForgotPasswordResponse> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.request_forgot_password(payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 使用重置码完成密码重置。
    pub async fn reset_password(
        &self,
        payload: UserResetPasswordRequest,
    ) -> ApiResult<UserResetPasswordResponse> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.reset_password(payload)?;
            let snapshot = store.clone();
            (result, snapshot)
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 后台重置普通用户登录密码，适用于用户忘记密码或账号异常后的人工维护。
    pub async fn reset_user_password(
        &self,
        id: &str,
        payload: UserPasswordResetRequest,
    ) -> ApiResult<UserSummary> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.reset_user_password(id, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 列出当前用户绑定的全部提现方式。
    pub async fn list_withdrawal_methods(&self, user_id: &str) -> ApiResult<Vec<WithdrawalMethod>> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .list_withdrawal_methods(user_id)
    }

    /// 新增提现方式：支持设置默认方式，默认方式会自动覆盖同用户历史默认。
    pub async fn create_withdrawal_method(
        &self,
        user_id: &str,
        payload: WithdrawalMethodRequest,
    ) -> ApiResult<WithdrawalMethod> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.create_withdrawal_method(user_id, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 更新提现方式：会重置同用户默认状态并校验归属关系。
    pub async fn update_withdrawal_method(
        &self,
        user_id: &str,
        method_id: &str,
        payload: WithdrawalMethodRequest,
    ) -> ApiResult<WithdrawalMethod> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            let result = store.update_withdrawal_method(user_id, method_id, payload)?;
            (result, store.clone())
        };
        self.persist(&snapshot).await?;
        Ok(result)
    }

    /// 删除提现方式：不影响其他提现方式的默认配置。
    pub async fn delete_withdrawal_method(&self, user_id: &str, method_id: &str) -> ApiResult<()> {
        let (result, snapshot) = {
            let mut store = self
                .inner
                .write()
                .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?;
            store.delete_withdrawal_method(user_id, method_id)?;
            ((), store.clone())
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

    /// 读取单个系统设置项，按 key 返回完整信息。
    pub async fn get_setting(&self, key: &str) -> ApiResult<SystemSetting> {
        self.inner
            .read()
            .map_err(|_| ApiError::Internal("access store lock poisoned".to_string()))?
            .setting(key)
    }

    /// 读取单个系统设置项的值文本。
    pub async fn setting_value(&self, key: &str) -> ApiResult<String> {
        Ok(self.get_setting(key).await?.value)
    }

    /// 读取单个系统设置项的值文本，不存在时返回 None。
    pub async fn setting_value_optional(&self, key: &str) -> ApiResult<Option<String>> {
        match self.get_setting(key).await {
            Ok(setting) => Ok(Some(setting.value)),
            Err(ApiError::NotFound(_)) => Ok(None),
            Err(error) => Err(error),
        }
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

    /// 从数据库重新加载用户、管理员、角色、会话和系统设置快照，供后台缓存维护使用。
    pub async fn reload_from_database(&self) -> ApiResult<bool> {
        let Some(persistence) = &self.persistence else {
            return Ok(false);
        };
        let store = load_access_store(persistence).await?;
        *self
            .inner
            .write()
            .map_err(|_| ApiError::Internal("访问控制缓存刷新失败".to_string()))? = store;
        Ok(true)
    }
    /// 把当前仓储快照同步保存到持久化存储。
    async fn persist(&self, store: &AccessStore) -> ApiResult<()> {
        if let Some(persistence) = &self.persistence {
            save_access_store(persistence, store).await?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// 用户、管理员、角色、会话和系统设置运行时数据快照，用于内存模式和数据库持久化前的业务校验。
struct AccessStore {
    users: BTreeMap<String, UserSummary>,
    admins: BTreeMap<String, AdminSummary>,
    admin_password_hashes: BTreeMap<String, String>,
    user_password_hashes: BTreeMap<String, String>,
    user_id_counter: u64,
    roles: BTreeMap<String, AdminRole>,
    sessions: BTreeMap<String, String>,
    user_sessions: BTreeMap<String, String>,
    user_password_reset_tokens: BTreeMap<String, PasswordResetTokenRecord>,
    user_withdrawal_methods: BTreeMap<String, WithdrawalMethod>,
    settings: BTreeMap<String, SystemSetting>,
    session_counter: u64,
    user_session_counter: u64,
    registration: RegistrationConfig,
}

/// 从数据库加载用户、管理员、角色、会话和系统设置运行时快照，空库时按模块规则初始化。
async fn load_access_store(database: &BusinessDatabase) -> ApiResult<AccessStore> {
    let pool = database.pool();
    let mut users = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, username, email, avatar_url, contact_qq, kind, status, balance_minor, agent_id, invite_code,
                COALESCE(registered_ip, '') AS registered_ip,
                COALESCE(register_country, '') AS register_country,
                COALESCE(register_region, '') AS register_region,
                COALESCE(register_city, '') AS register_city,
                COALESCE(register_geo_source, 'unknown') AS register_geo_source,
                to_char(created_at, 'YYYY-MM-DD HH24:MI:SS') AS created_at
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
                avatar_url: row
                    .try_get("avatar_url")
                    .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                contact_qq: row
                    .try_get("contact_qq")
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
                registration_location: UserRegistrationLocation {
                    registered_ip: row
                        .try_get("registered_ip")
                        .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                    country: row
                        .try_get("register_country")
                        .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                    region: row
                        .try_get("register_region")
                        .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                    city: row
                        .try_get("register_city")
                        .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                    source: row
                        .try_get("register_geo_source")
                        .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
                },
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
            },
        );
    }

    let mut roles = BTreeMap::new();
    for row in sqlx::query(
        "SELECT id, name, scopes, COALESCE(permissions, '[]'::jsonb) AS permissions
         FROM admin_roles
         ORDER BY id ASC",
    )
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
                permissions: super::business_database::from_json(
                    row.try_get("permissions")
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

    let user_password_hashes =
        sqlx::query("SELECT user_id, password_hash FROM user_password_hashes")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::Internal("用户密码数据读取失败".to_string()))?
            .into_iter()
            .map(|row| {
                let user_id = row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("用户密码数据读取失败".to_string()))?;
                let password_hash = row
                    .try_get("password_hash")
                    .map_err(|_| ApiError::Internal("用户密码数据读取失败".to_string()))?;
                Ok((user_id, password_hash))
            })
            .collect::<ApiResult<BTreeMap<String, String>>>()?;

    let user_sessions = sqlx::query("SELECT token, user_id FROM user_sessions")
        .fetch_all(pool)
        .await
        .map_err(|_| ApiError::Internal("用户会话数据读取失败".to_string()))?
        .into_iter()
        .map(|row| {
            let token = row
                .try_get("token")
                .map_err(|_| ApiError::Internal("用户会话数据读取失败".to_string()))?;
            let user_id = row
                .try_get("user_id")
                .map_err(|_| ApiError::Internal("用户会话数据读取失败".to_string()))?;
            Ok((token, user_id))
        })
        .collect::<ApiResult<BTreeMap<String, String>>>()?;

    let user_password_reset_tokens =
        sqlx::query("SELECT token, user_id, expires_at_unix FROM user_password_reset_tokens")
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::Internal("用户重置码数据读取失败".to_string()))?
            .into_iter()
            .map(|row| {
                let token = row
                    .try_get("token")
                    .map_err(|_| ApiError::Internal("用户重置码数据读取失败".to_string()))?;
                let user_id = row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("用户重置码数据读取失败".to_string()))?;
                let expires_at_unix = row
                    .try_get("expires_at_unix")
                    .map_err(|_| ApiError::Internal("用户重置码数据读取失败".to_string()))?;

                Ok((
                    token,
                    PasswordResetTokenRecord {
                        user_id,
                        expires_at_unix,
                    },
                ))
            })
            .collect::<ApiResult<BTreeMap<String, PasswordResetTokenRecord>>>()?;

    let user_withdrawal_methods = sqlx::query(
        "SELECT id, user_id, method_type, account_holder, account_number,
                    bank_name, is_default, created_at, updated_at
             FROM user_withdrawal_methods
             ORDER BY id ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?
    .into_iter()
    .map(|row| {
        let id: String = row
            .try_get("id")
            .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?;

        Ok((
            id.clone(),
            WithdrawalMethod {
                id,
                user_id: row
                    .try_get("user_id")
                    .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?,
                method_type: enum_from_string(
                    row.try_get("method_type")
                        .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?,
                )?,
                account_holder: row
                    .try_get("account_holder")
                    .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?,
                account_number: row
                    .try_get("account_number")
                    .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?,
                bank_name: row
                    .try_get("bank_name")
                    .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?,
                is_default: row
                    .try_get("is_default")
                    .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?,
                created_at: row
                    .try_get("created_at")
                    .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?,
                updated_at: row
                    .try_get("updated_at")
                    .map_err(|_| ApiError::Internal("提现方式数据读取失败".to_string()))?,
            },
        ))
    })
    .collect::<ApiResult<BTreeMap<String, WithdrawalMethod>>>()?;

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

    let _has_missing_settings = fill_missing_system_settings(&mut settings);

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

    let user_session_counter = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM access_runtime WHERE key = 'user_session_counter'",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::Internal("用户权限运行数据读取失败".to_string()))?
    .unwrap_or_default();

    let user_id_counter = sqlx::query_scalar::<_, i64>(
        "SELECT value FROM access_runtime WHERE key = 'user_id_counter'",
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

    let mut access_store = AccessStore {
        users: users.clone(),
        admins,
        admin_password_hashes,
        user_password_hashes,
        roles,
        sessions,
        user_sessions,
        user_password_reset_tokens,
        user_withdrawal_methods,
        settings,
        session_counter: u64::try_from(session_counter).unwrap_or_default(),
        user_session_counter: u64::try_from(user_session_counter).unwrap_or_default(),
        user_id_counter: if user_id_counter > 0 {
            u64::try_from(user_id_counter).unwrap_or_default()
        } else {
            next_user_id_from_users(&users)
        },
        registration,
    };

    // 确保机器人补单用户（X90002-X90010）在数据库中存在，
    // 避免合买机器人认购时因用户不存在而失败。
    let robot_fill_user_ids = [
        "X90002", "X90003", "X90004", "X90005", "X90006", "X90007", "X90008", "X90009", "X90010",
    ];
    let mut missing_robot_users = Vec::new();
    for (index, robot_id) in robot_fill_user_ids.iter().enumerate() {
        if !access_store.users.contains_key(*robot_id) {
            let num = index + 2;
            missing_robot_users.push(UserSummary {
                id: robot_id.to_string(),
                username: format!("robot_fill_{num:02}"),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: crate::domain::user::UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 520_000,
                agent_id: Some("U90001".to_string()),
                invite_code: format!("ROBOT-{robot_id}"),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-01 09:00:00".to_string(),
            });
        }
    }
    let needs_robot_user_persist = !missing_robot_users.is_empty();
    for user in &missing_robot_users {
        access_store.users.insert(user.id.clone(), user.clone());
    }

    if !_has_missing_settings && !needs_robot_user_persist {
        return Ok(access_store);
    }

    save_access_store(database, &access_store).await?;
    Ok(access_store)
}

/// 数据库模式下分页读取用户并联查资金账户余额。
async fn query_user_page(
    database: &BusinessDatabase,
    status: Option<UserStatus>,
    username: Option<&str>,
    agent_id: Option<&str>,
    sort_by: &str,
    sort_direction: &str,
    include_robot_data: bool,
    page: PageRequest,
) -> ApiResult<ListPage<UserSummary>> {
    let status = status.as_ref().map(enum_to_string).transpose()?;
    let order_clause = user_page_order_clause(sort_by, sort_direction)?;
    let robot_user_ids = ROBOT_GROUP_BUY_USER_IDS
        .iter()
        .map(|user_id| (*user_id).to_string())
        .collect::<Vec<_>>();
    let total_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM users u
         WHERE ($1::text IS NULL OR u.status = $1)
           AND ($2::text IS NULL OR strpos(lower(u.username), lower($2::text)) > 0)
           AND ($3::text IS NULL OR u.agent_id = $3)
           AND ($4::bool OR NOT (u.id = ANY($5::text[])))",
    )
    .bind(status.as_deref())
    .bind(username)
    .bind(agent_id)
    .bind(include_robot_data)
    .bind(&robot_user_ids)
    .fetch_one(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户分页总数读取失败".to_string()))?;
    let total_count = usize::try_from(total_count)
        .map_err(|_| ApiError::Internal("用户分页总数无效".to_string()))?;
    let resolved = page.resolve(total_count);
    let sql = format!(
        "SELECT u.id, u.username, u.email, u.avatar_url, u.contact_qq, u.kind, u.status,
                COALESCE(account.available_balance_minor, u.balance_minor) AS balance_minor,
                u.agent_id, u.invite_code,
                COALESCE(u.registered_ip, '') AS registered_ip,
                COALESCE(u.register_country, '') AS register_country,
                COALESCE(u.register_region, '') AS register_region,
                COALESCE(u.register_city, '') AS register_city,
                COALESCE(u.register_geo_source, 'unknown') AS register_geo_source,
                to_char(u.created_at, 'YYYY-MM-DD HH24:MI:SS') AS created_at
         FROM users u
         LEFT JOIN financial_accounts account ON account.user_id = u.id
         WHERE ($1::text IS NULL OR u.status = $1)
           AND ($2::text IS NULL OR strpos(lower(u.username), lower($2::text)) > 0)
           AND ($3::text IS NULL OR u.agent_id = $3)
           AND ($4::bool OR NOT (u.id = ANY($5::text[])))
         ORDER BY {order_clause}
         LIMIT $6 OFFSET $7"
    );
    let rows = sqlx::query(&sql)
        .bind(status.as_deref())
        .bind(username)
        .bind(agent_id)
        .bind(include_robot_data)
        .bind(&robot_user_ids)
        .bind(resolved.limit_i64()?)
        .bind(resolved.offset_i64()?)
        .fetch_all(database.pool())
        .await
        .map_err(|_| ApiError::Internal("用户分页数据读取失败".to_string()))?;
    let items = rows
        .into_iter()
        .map(user_summary_from_row)
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(ListPage::new(items, resolved))
}

/// 数据库模式下按用户 ID 批量读取用户名。
async fn query_usernames_for_ids(
    database: &BusinessDatabase,
    user_ids: &[String],
) -> ApiResult<BTreeMap<String, String>> {
    let rows = sqlx::query(
        "SELECT id, username
         FROM users
         WHERE id = ANY($1::text[])",
    )
    .bind(user_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户名映射数据读取失败".to_string()))?;

    rows.into_iter()
        .map(|row| {
            let id = row
                .try_get("id")
                .map_err(|_| ApiError::Internal("用户名映射数据读取失败".to_string()))?;
            let username = row
                .try_get("username")
                .map_err(|_| ApiError::Internal("用户名映射数据读取失败".to_string()))?;
            Ok((id, username))
        })
        .collect()
}

/// 数据库模式下按用户 ID 批量读取用户摘要，避免后台列表为了补代理信息全量扫描用户表。
async fn query_users_for_ids(
    database: &BusinessDatabase,
    user_ids: &[String],
) -> ApiResult<Vec<UserSummary>> {
    let rows = sqlx::query(
        "SELECT u.id, u.username, u.email, u.avatar_url, u.contact_qq, u.kind, u.status,
                COALESCE(account.available_balance_minor, u.balance_minor) AS balance_minor,
                u.agent_id, u.invite_code,
                COALESCE(u.registered_ip, '') AS registered_ip,
                COALESCE(u.register_country, '') AS register_country,
                COALESCE(u.register_region, '') AS register_region,
                COALESCE(u.register_city, '') AS register_city,
                COALESCE(u.register_geo_source, 'unknown') AS register_geo_source,
                to_char(u.created_at, 'YYYY-MM-DD HH24:MI:SS') AS created_at
         FROM users u
         LEFT JOIN financial_accounts account ON account.user_id = u.id
         WHERE u.id = ANY($1::text[])",
    )
    .bind(user_ids)
    .fetch_all(database.pool())
    .await
    .map_err(|_| ApiError::Internal("用户展示数据读取失败".to_string()))?;

    rows.into_iter()
        .map(user_summary_from_row)
        .collect::<ApiResult<Vec<_>>>()
}

/// 归一化用户 ID 集合，去重并移除空值。
fn normalized_user_ids(user_ids: &[String]) -> BTreeSet<String> {
    user_ids
        .iter()
        .map(|user_id| user_id.trim())
        .filter(|user_id| !user_id.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// 归一化后台用户列表用户名搜索词，空白输入不参与过滤。
fn normalized_username_filter(username: Option<&str>) -> Option<String> {
    username
        .map(str::trim)
        .filter(|username| !username.is_empty())
        .map(ToString::to_string)
}

/// 归一化后台用户列表的上级代理筛选，空白输入不参与过滤。
fn normalized_agent_id_filter(agent_id: Option<&str>) -> Option<String> {
    agent_id
        .map(str::trim)
        .filter(|agent_id| !agent_id.is_empty())
        .map(ToString::to_string)
}

/// 将数据库行转换为用户摘要，供启动加载和分页查询复用。
fn user_summary_from_row(row: PgRow) -> ApiResult<UserSummary> {
    Ok(UserSummary {
        id: row
            .try_get("id")
            .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
        username: row
            .try_get("username")
            .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
        email: row
            .try_get("email")
            .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
        avatar_url: row
            .try_get("avatar_url")
            .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
        contact_qq: row
            .try_get("contact_qq")
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
        registration_location: UserRegistrationLocation {
            registered_ip: row
                .try_get("registered_ip")
                .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
            country: row
                .try_get("register_country")
                .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
            region: row
                .try_get("register_region")
                .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
            city: row
                .try_get("register_city")
                .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
            source: row
                .try_get("register_geo_source")
                .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
        },
        created_at: row
            .try_get("created_at")
            .map_err(|_| ApiError::Internal("用户数据读取失败".to_string()))?,
    })
}

/// 根据后台白名单排序字段生成用户分页 `ORDER BY` 片段。
fn user_page_order_clause(sort_by: &str, sort_direction: &str) -> ApiResult<String> {
    let direction = match sort_direction.trim().to_ascii_lowercase().as_str() {
        "" | "desc" | "descending" => "DESC",
        "asc" | "ascending" => "ASC",
        _ => return Err(ApiError::BadRequest("用户列表排序方向不支持".to_string())),
    };
    let expression = match sort_by.trim() {
        "" | "id" | "userId" => {
            "CASE WHEN u.id ~ '^U[0-9]+$' THEN substring(u.id from 2)::bigint ELSE 0 END"
        }
        "agentId" => "COALESCE(u.agent_id, '')",
        "balance" | "balanceMinor" => "COALESCE(account.available_balance_minor, u.balance_minor)",
        "email" => "COALESCE(u.email, '')",
        "inviteCode" => "u.invite_code",
        "kind" | "userKind" => "CASE u.kind WHEN 'regular' THEN 0 WHEN 'agent' THEN 1 ELSE 9 END",
        "status" => {
            "CASE u.status WHEN 'active' THEN 0 WHEN 'suspended' THEN 1 WHEN 'locked' THEN 2 ELSE 9 END"
        }
        "username" => "u.username",
        _ => return Err(ApiError::BadRequest("用户列表排序字段不支持".to_string())),
    };

    Ok(format!("{expression} {direction}, u.id {direction}"))
}

/// 内存模式下按后台白名单排序用户摘要，保持无数据库开发体验和数据库查询语义一致。
fn sort_user_summaries(
    users: &mut [UserSummary],
    sort_by: &str,
    sort_direction: &str,
) -> ApiResult<()> {
    let descending = match sort_direction.trim().to_ascii_lowercase().as_str() {
        "" | "desc" | "descending" => true,
        "asc" | "ascending" => false,
        _ => return Err(ApiError::BadRequest("用户列表排序方向不支持".to_string())),
    };
    let sort_by = sort_by.trim();
    users.sort_by(|left, right| {
        let ordering = match sort_by {
            "" | "id" | "userId" => user_id_sequence(&left.id)
                .cmp(&user_id_sequence(&right.id))
                .then_with(|| left.id.cmp(&right.id)),
            "agentId" => left.agent_id.cmp(&right.agent_id),
            "balance" | "balanceMinor" => left.balance_minor.cmp(&right.balance_minor),
            "email" => left.email.cmp(&right.email),
            "inviteCode" => left.invite_code.cmp(&right.invite_code),
            "kind" | "userKind" => {
                user_kind_sort_value(&left.kind).cmp(&user_kind_sort_value(&right.kind))
            }
            "status" => {
                user_status_sort_value(&left.status).cmp(&user_status_sort_value(&right.status))
            }
            "username" => left.username.cmp(&right.username),
            _ => return std::cmp::Ordering::Equal,
        };
        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
    if !matches!(
        sort_by,
        "" | "id"
            | "userId"
            | "agentId"
            | "balance"
            | "balanceMinor"
            | "email"
            | "inviteCode"
            | "kind"
            | "userKind"
            | "status"
            | "username"
    ) {
        return Err(ApiError::BadRequest("用户列表排序字段不支持".to_string()));
    }
    Ok(())
}

/// 解析用户 ID 中的数字序号，用于默认最新用户优先排序。
fn user_id_sequence(user_id: &str) -> u64 {
    user_id
        .trim()
        .strip_prefix('U')
        .and_then(|value| value.parse().ok())
        .unwrap_or_default()
}

/// 用户类型排序权重。
fn user_kind_sort_value(kind: &UserKind) -> u8 {
    match kind {
        UserKind::Regular => 0,
        UserKind::Agent => 1,
    }
}

/// 用户状态排序权重。
fn user_status_sort_value(status: &UserStatus) -> u8 {
    match status {
        UserStatus::Active => 0,
        UserStatus::Suspended => 1,
        UserStatus::Locked => 2,
    }
}

/// 把用户、管理员、角色、会话和系统设置运行时快照保存到数据库。
async fn save_access_store(database: &BusinessDatabase, store: &AccessStore) -> ApiResult<()> {
    let mut tx = database
        .pool()
        .begin()
        .await
        .map_err(|_| ApiError::Internal("用户权限事务开启失败".to_string()))?;

    for table in [
        "admin_sessions",
        "admin_password_hashes",
        "user_sessions",
        "user_password_hashes",
        "user_password_reset_tokens",
        "user_withdrawal_methods",
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
            "INSERT INTO users (
                id, username, email, avatar_url, contact_qq, kind, status, balance_minor,
                agent_id, invite_code, registered_ip, register_country, register_region,
                register_city, register_geo_source, created_at
             )
             VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                $9, $10, $11, $12, $13, $14, $15, $16::timestamptz
             )",
        )
        .bind(&user.id)
        .bind(&user.username)
        .bind(&user.email)
        .bind(&user.avatar_url)
        .bind(&user.contact_qq)
        .bind(enum_to_string(&user.kind)?)
        .bind(enum_to_string(&user.status)?)
        .bind(user.balance_minor)
        .bind(&user.agent_id)
        .bind(&user.invite_code)
        .bind(&user.registration_location.registered_ip)
        .bind(&user.registration_location.country)
        .bind(&user.registration_location.region)
        .bind(&user.registration_location.city)
        .bind(&user.registration_location.source)
        .bind(&user.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("用户数据保存失败".to_string()))?;
    }

    for role in store.roles.values() {
        sqlx::query(
            "INSERT INTO admin_roles (id, name, scopes, permissions)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&role.id)
        .bind(&role.name)
        .bind(to_json(&role.scopes)?)
        .bind(to_json(&role.permissions)?)
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

    for (user_id, password_hash) in &store.user_password_hashes {
        sqlx::query("INSERT INTO user_password_hashes (user_id, password_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(password_hash)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("用户密码数据保存失败".to_string()))?;
    }

    for (token, admin_id) in &store.sessions {
        sqlx::query("INSERT INTO admin_sessions (token, admin_id) VALUES ($1, $2)")
            .bind(token)
            .bind(admin_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("管理员会话数据保存失败".to_string()))?;
    }

    for (token, user_id) in &store.user_sessions {
        sqlx::query("INSERT INTO user_sessions (token, user_id) VALUES ($1, $2)")
            .bind(token)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|_| ApiError::Internal("用户会话数据保存失败".to_string()))?;
    }

    for (token, reset_token) in &store.user_password_reset_tokens {
        sqlx::query(
            "INSERT INTO user_password_reset_tokens (token, user_id, expires_at_unix) VALUES ($1, $2, $3)",
        )
        .bind(token)
        .bind(&reset_token.user_id)
        .bind(reset_token.expires_at_unix)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("用户重置码数据保存失败".to_string()))?;
    }

    for method in store.user_withdrawal_methods.values() {
        sqlx::query(
            "INSERT INTO user_withdrawal_methods
             (id, user_id, method_type, account_holder, account_number, bank_name, is_default, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(&method.id)
        .bind(&method.user_id)
        .bind(enum_to_string(&method.method_type)?)
        .bind(&method.account_holder)
        .bind(&method.account_number)
        .bind(&method.bank_name)
        .bind(method.is_default)
        .bind(&method.created_at)
        .bind(&method.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("提现方式数据保存失败".to_string()))?;
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
    let user_session_counter = i64::try_from(store.user_session_counter)
        .map_err(|_| ApiError::Internal("用户会话序号过大".to_string()))?;
    let user_id_counter = i64::try_from(store.user_id_counter)
        .map_err(|_| ApiError::Internal("用户序号过大".to_string()))?;
    sqlx::query("INSERT INTO access_runtime (key, value) VALUES ('session_counter', $1)")
        .bind(session_counter)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("用户权限运行数据保存失败".to_string()))?;
    sqlx::query("INSERT INTO access_runtime (key, value) VALUES ('user_session_counter', $1)")
        .bind(user_session_counter)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("用户权限运行数据保存失败".to_string()))?;
    sqlx::query("INSERT INTO access_runtime (key, value) VALUES ('user_id_counter', $1)")
        .bind(user_id_counter)
        .execute(&mut *tx)
        .await
        .map_err(|_| ApiError::Internal("用户权限运行数据保存失败".to_string()))?;

    tx.commit()
        .await
        .map_err(|_| ApiError::Internal("用户权限事务提交失败".to_string()))
}

/// 用户、管理员、角色、会话和系统设置运行时数据快照，用于内存模式和数据库持久化前的业务校验。
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
        let user_password_hashes = seed_user_password_hashes(&users);
        let user_id_counter = next_user_id_from_users(&users);
        let settings = seed_settings()
            .into_iter()
            .map(|setting| (setting.key.clone(), setting))
            .collect();

        Self {
            users,
            admins,
            admin_password_hashes,
            user_password_hashes,
            user_id_counter,
            roles,
            sessions: BTreeMap::new(),
            user_sessions: BTreeMap::new(),
            user_password_reset_tokens: BTreeMap::new(),
            user_withdrawal_methods: BTreeMap::new(),
            settings,
            session_counter: 0,
            user_session_counter: 0,
            registration: RegistrationConfig {
                username_enabled: true,
                email_enabled: false,
                agent_invite_required: false,
            },
        }
    }

    /// 生成当前模块的只读业务快照。
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

    /// 创建用户并落入内存集合，空邀请码时自动生成随机字母数字码。
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

    /// 更新内存用户：用户名、余额、邀请码和头像分别归属专门链路，用户维护只允许改联系方式、类型、状态和上级代理。
    fn update_user(&mut self, id: &str, user: UserSummary) -> ApiResult<UserSummary> {
        let mut user = normalize_user(user)?;
        if id != user.id {
            return Err(ApiError::BadRequest(
                "path id must match user id".to_string(),
            ));
        }
        let existing = self
            .users
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))?;
        user.username = existing.username;
        user.balance_minor = existing.balance_minor;
        user.invite_code = existing.invite_code;
        user.avatar_url = existing.avatar_url;
        user.registration_location = existing.registration_location;
        user.created_at = existing.created_at;

        self.users.insert(id.to_string(), user.clone());
        Ok(user)
    }

    /// 删除用户资料，并清理访问控制仓储内与该用户直接绑定的数据。
    fn delete_user(&mut self, id: &str) -> ApiResult<UserSummary> {
        if self
            .users
            .values()
            .any(|user| user.agent_id.as_deref() == Some(id))
        {
            return Err(ApiError::Conflict(
                "该用户仍有下级用户，请先调整下级代理关系".to_string(),
            ));
        }

        let removed = self
            .users
            .remove(id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))?;

        self.user_password_hashes.remove(id);
        self.user_sessions.retain(|_, user_id| user_id != id);
        self.user_password_reset_tokens
            .retain(|_, record| record.user_id != id);
        self.user_withdrawal_methods
            .retain(|_, method| method.user_id != id);

        Ok(removed)
    }

    /// 处理用户注册：校验注册策略、邀请码和唯一性，并创建用户和密码记录。
    fn register_user(&mut self, payload: UserRegisterRequest) -> ApiResult<UserSummary> {
        let password = validate_user_password(&payload.password)?;
        let username_provided = payload.username.is_some();
        let email_provided = payload.email.is_some();
        let username = payload
            .username
            .map(|value| required_trimmed(value, "username"))
            .transpose()?
            .filter(|value| !value.is_empty())
            .or_else(|| {
                payload
                    .email
                    .as_ref()
                    .map(|value| required_trimmed(value.clone(), "email"))
                    .transpose()
                    .ok()
                    .flatten()
            })
            .filter(|value| !value.is_empty());

        let username = username.ok_or_else(|| {
            ApiError::BadRequest("username 或 email 至少填写一项用于注册".to_string())
        })?;

        if username_provided && !self.registration.username_enabled {
            return Err(ApiError::BadRequest("当前系统禁止用户名注册".to_string()));
        }

        let email = payload
            .email
            .map(|value| required_trimmed(value, "email"))
            .transpose()?
            .filter(|value| !value.is_empty() && value.contains('@'));
        let contact_qq = normalize_required_contact_qq(payload.contact_qq)?;

        if email_provided && email.is_none() {
            return Err(ApiError::BadRequest("邮箱格式不正确".to_string()));
        }

        if !username_provided && !self.registration.email_enabled {
            return Err(ApiError::BadRequest("当前系统禁止邮箱注册".to_string()));
        }

        if !username_provided
            && email_provided
            && self.registration.agent_invite_required
            && payload.invite_code.is_none()
        {
            return Err(ApiError::BadRequest("邀请码不能为空".to_string()));
        }

        let invite_code = payload.invite_code;
        let agent_id = match invite_code {
            Some(code) => {
                let code = required_trimmed(code, "invite code")?;
                let inviter = self
                    .users
                    .values()
                    .find(|user| user.invite_code == code)
                    .filter(|user| user.kind == UserKind::Agent)
                    .ok_or_else(|| ApiError::BadRequest("邀请码无效".to_string()))?;
                Some(inviter.id.clone())
            }
            None => {
                if self.registration.agent_invite_required {
                    return Err(ApiError::BadRequest("邀请码不能为空".to_string()));
                }
                None
            }
        };

        for other in self.users.values() {
            if other.username == username {
                return Err(ApiError::Conflict("用户名已存在".to_string()));
            }
            if let Some(email) = &email {
                if let Some(existing_email) = &other.email {
                    if existing_email == email {
                        return Err(ApiError::Conflict("该邮箱已被绑定".to_string()));
                    }
                }
            }
        }

        let user_id = next_user_id(&self.users, &mut self.user_id_counter)?;
        let invite_code = random_invite_code(&self.users)?;
        let registration_location =
            registration_location_from_request(payload.registration_location);
        let user = UserSummary {
            id: user_id,
            username,
            email,
            avatar_url: String::new(),
            contact_qq,
            kind: UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id,
            invite_code,
            registration_location,
            created_at: format_local_time(),
        };

        self.user_password_hashes
            .insert(user.id.clone(), hash_user_password(&password)?);
        self.create_user(user.clone())?;

        Ok(user)
    }

    /// 处理用户登录：支持用户名/邮箱两种标识，并签发用户会话。
    fn login_user(&mut self, payload: UserLoginRequest) -> ApiResult<UserAuthSession> {
        let login_key = required_trimmed(payload.login_key, "login key")?;
        let password = validate_user_password(&payload.password)?;
        let user = self
            .users
            .values()
            .find(|user| {
                user.username == login_key || user.email.as_deref() == Some(login_key.as_str())
            })
            .ok_or_else(|| ApiError::Unauthorized("用户名/密码错误".to_string()))?
            .clone();

        if user.status != UserStatus::Active {
            return Err(ApiError::Forbidden(
                inactive_user_status_message(&user.status).to_string(),
            ));
        }

        let password_hash = self
            .user_password_hashes
            .get(&user.id)
            .ok_or_else(|| ApiError::Internal("用户密码未配置".to_string()))?;
        if !verify_user_password(&password, password_hash)? {
            return Err(ApiError::Unauthorized("用户名/密码错误".to_string()));
        }

        let token = self.next_user_session_token()?;
        self.user_sessions
            .insert(session_token_hash(&token), user.id.clone());
        self.session_from_user_token(&token)
    }

    /// 通过 token 解析用户会话，不存在会返回未授权。
    fn session_from_user_token(&self, token: &str) -> ApiResult<UserAuthSession> {
        let token = token.trim();
        if token.is_empty() {
            return Err(ApiError::Unauthorized("登录令牌不能为空".to_string()));
        }

        let token_hash = session_token_hash(token);
        let user_id = self
            .user_sessions
            .get(&token_hash)
            .ok_or_else(|| ApiError::Unauthorized("登录已过期，请重新登录".to_string()))?;
        let user = self.get_user(user_id)?;
        if user.status != UserStatus::Active {
            return Err(ApiError::Forbidden(
                inactive_user_status_message(&user.status).to_string(),
            ));
        }

        Ok(UserAuthSession {
            token: token.to_string(),
            user,
        })
    }

    /// 注销用户会话。
    fn logout_user(&mut self, token: &str) {
        self.user_sessions.remove(&session_token_hash(token.trim()));
    }

    /// 绑定邮箱并保持原有用户标识。
    fn bind_email(
        &mut self,
        user_id: &str,
        payload: UserBindEmailRequest,
    ) -> ApiResult<UserSummary> {
        let email = required_trimmed(payload.email, "email")?;
        if let Some(existing) = self
            .users
            .values()
            .find(|user| user.id != user_id && user.email.as_deref() == Some(email.as_str()))
        {
            return Err(ApiError::Conflict(format!(
                "email `{}` has been used",
                existing.email.clone().unwrap_or_default()
            )));
        }

        let user = self
            .users
            .get_mut(user_id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{user_id}` not found")))?;
        user.email = Some(email);
        Ok(user.clone())
    }

    /// 更新用户头像链接，允许用户清空头像，但非空时必须是 http/https 图片地址。
    fn update_user_avatar(
        &mut self,
        user_id: &str,
        payload: UserAvatarRequest,
    ) -> ApiResult<UserSummary> {
        let avatar_url = normalize_avatar_url(payload.avatar_url)?;
        let user = self
            .users
            .get_mut(user_id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{user_id}` not found")))?;
        user.avatar_url = avatar_url;
        Ok(user.clone())
    }

    /// 修改用户密码并刷新密码哈希。
    fn change_password(
        &mut self,
        user_id: &str,
        payload: UserChangePasswordRequest,
    ) -> ApiResult<UserSummary> {
        let old_password = validate_user_password(&payload.old_password)?;
        let new_password = validate_user_password(&payload.new_password)?;

        let user = self
            .users
            .get(user_id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{user_id}` not found")))?;

        let password_hash = self
            .user_password_hashes
            .get(user_id)
            .ok_or_else(|| ApiError::Internal("用户密码未配置".to_string()))?;
        if !verify_user_password(&old_password, password_hash)? {
            return Err(ApiError::Unauthorized("旧密码不正确".to_string()));
        }

        self.user_password_hashes
            .insert(user_id.to_string(), hash_user_password(&new_password)?);

        Ok(user.clone())
    }

    /// 生成忘记密码重置令牌。
    fn request_forgot_password(
        &mut self,
        payload: UserForgotPasswordRequest,
    ) -> ApiResult<UserForgotPasswordResponse> {
        let login_key = required_trimmed(payload.login_key, "login key")?;
        let user = self
            .users
            .values()
            .find(|user| {
                user.username == login_key || user.email.as_deref() == Some(login_key.as_str())
            })
            .ok_or_else(|| ApiError::NotFound("用户不存在".to_string()))?;

        let mut expired_tokens = Vec::new();
        for (token, record) in &self.user_password_reset_tokens {
            if record.user_id == user.id && record.expires_at_unix < current_unix_timestamp() {
                expired_tokens.push(token.to_string());
            }
        }

        for token in expired_tokens {
            self.user_password_reset_tokens.remove(&token);
        }

        let token = random_alnum_string(USER_RESET_TOKEN_BYTES);
        let now = current_unix_timestamp();
        let expires_at_unix = now
            .checked_add(USER_RESET_TOKEN_TTL_SECONDS)
            .ok_or_else(|| ApiError::Internal("重置码过期时间计算异常".to_string()))?;

        self.user_password_reset_tokens.insert(
            token.clone(),
            PasswordResetTokenRecord {
                user_id: user.id.clone(),
                expires_at_unix,
            },
        );

        Ok(UserForgotPasswordResponse {
            reset_token: token,
            expires_at: format_unix_timestamp(expires_at_unix),
        })
    }

    /// 使用重置码更新密码并清理 token。
    fn reset_password(
        &mut self,
        payload: UserResetPasswordRequest,
    ) -> ApiResult<UserResetPasswordResponse> {
        let new_password = validate_user_password(&payload.new_password)?;
        let now = current_unix_timestamp();
        let token = required_trimmed(payload.reset_token, "reset token")?;

        let record = self
            .user_password_reset_tokens
            .remove(&token)
            .ok_or_else(|| ApiError::Unauthorized("重置码无效".to_string()))?;
        if record.expires_at_unix < now {
            return Err(ApiError::Unauthorized("重置码已过期".to_string()));
        }

        let user = self
            .users
            .get(&record.user_id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{}` not found", record.user_id)))?;
        self.user_password_hashes
            .insert(user.id.clone(), hash_user_password(&new_password)?);

        Ok(UserResetPasswordResponse { reset: true })
    }

    /// 后台人工重置指定用户密码，写入新的用户密码哈希但不修改用户基础资料。
    fn reset_user_password(
        &mut self,
        id: &str,
        payload: UserPasswordResetRequest,
    ) -> ApiResult<UserSummary> {
        let password = validate_user_password(&payload.password)?;
        let user = self.get_user(id)?;
        self.user_password_hashes
            .insert(user.id.clone(), hash_user_password(&password)?);

        Ok(user)
    }

    /// 列出指定用户的提现方式。
    fn list_withdrawal_methods(&self, user_id: &str) -> ApiResult<Vec<WithdrawalMethod>> {
        if !self.users.contains_key(user_id) {
            return Err(ApiError::NotFound(format!("user `{user_id}` not found")));
        }

        Ok(self
            .user_withdrawal_methods
            .values()
            .filter(|method| method.user_id == user_id)
            .cloned()
            .collect())
    }

    /// 新增提现方式。
    fn create_withdrawal_method(
        &mut self,
        user_id: &str,
        payload: WithdrawalMethodRequest,
    ) -> ApiResult<WithdrawalMethod> {
        if !self.users.contains_key(user_id) {
            return Err(ApiError::NotFound(format!("user `{user_id}` not found")));
        }

        let account_holder = required_trimmed(payload.account_holder, "account holder")?;
        let account_number = required_trimmed(payload.account_number, "account number")?;
        let bank_name = payload
            .bank_name
            .map(|value| required_trimmed(value, "bank name"))
            .transpose()?;
        if payload.method_type == WithdrawalMethodType::BankCard && bank_name.is_none() {
            return Err(ApiError::BadRequest("bank card 必填银行卡名称".to_string()));
        }

        if payload.is_default {
            for method in self
                .user_withdrawal_methods
                .values_mut()
                .filter(|method| method.user_id == user_id)
            {
                method.is_default = false;
                method.updated_at = format_local_time();
            }
        }

        let method_id =
            random_withdrawal_method_id(&self.user_withdrawal_methods, WITHDRAWAL_METHOD_ID_BYTES)?;
        let now = format_local_time();
        let method = WithdrawalMethod {
            id: method_id,
            user_id: user_id.to_string(),
            method_type: payload.method_type,
            account_holder,
            account_number,
            bank_name,
            is_default: payload.is_default,
            created_at: now.clone(),
            updated_at: now,
        };
        self.user_withdrawal_methods
            .insert(method.id.clone(), method.clone());
        Ok(method)
    }

    /// 更新提现方式。
    fn update_withdrawal_method(
        &mut self,
        user_id: &str,
        method_id: &str,
        payload: WithdrawalMethodRequest,
    ) -> ApiResult<WithdrawalMethod> {
        if !self.users.contains_key(user_id) {
            return Err(ApiError::NotFound(format!("user `{user_id}` not found")));
        }

        let account_holder = required_trimmed(payload.account_holder, "account holder")?;
        let account_number = required_trimmed(payload.account_number, "account number")?;
        let bank_name = payload
            .bank_name
            .map(|value| required_trimmed(value, "bank name"))
            .transpose()?;
        if payload.method_type == WithdrawalMethodType::BankCard && bank_name.is_none() {
            return Err(ApiError::BadRequest("bank card 必填银行卡名称".to_string()));
        }

        if payload.is_default {
            let now = format_local_time();
            for method in self
                .user_withdrawal_methods
                .values_mut()
                .filter(|method| method.user_id == user_id)
            {
                method.is_default = false;
                method.updated_at = now.clone();
            }
        }

        let method = self
            .user_withdrawal_methods
            .get_mut(method_id)
            .ok_or_else(|| {
                ApiError::NotFound(format!("withdrawal method `{method_id}` not found"))
            })?;

        if method.user_id != user_id {
            return Err(ApiError::Forbidden(
                "withdrawal method permission denied".to_string(),
            ));
        }

        method.method_type = payload.method_type;
        method.account_holder = account_holder;
        method.account_number = account_number;
        method.bank_name = bank_name;
        method.is_default = payload.is_default;
        method.updated_at = format_local_time();

        Ok(method.clone())
    }

    /// 删除提现方式。
    fn delete_withdrawal_method(&mut self, user_id: &str, method_id: &str) -> ApiResult<()> {
        let method = self.user_withdrawal_methods.get(method_id).ok_or_else(|| {
            ApiError::NotFound(format!("withdrawal method `{method_id}` not found"))
        })?;
        if method.user_id != user_id {
            return Err(ApiError::Forbidden(
                "withdrawal method permission denied".to_string(),
            ));
        }

        self.user_withdrawal_methods
            .remove(method_id)
            .map(|_| ())
            .ok_or_else(|| ApiError::NotFound(format!("withdrawal method `{method_id}` not found")))
    }

    /// 切换用户状态并返回最新用户快照。
    fn set_user_status(&mut self, id: &str, status: UserStatus) -> ApiResult<UserSummary> {
        let user = self
            .users
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("user `{id}` not found")))?;
        user.status = status;
        if user.status != UserStatus::Active {
            self.user_sessions.retain(|_, user_id| user_id != id);
        }
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

    /// 返回管理员账号列表。
    fn admins(&self) -> Vec<AdminSummary> {
        self.admins.values().cloned().collect()
    }

    /// 按管理员 ID 读取账号详情。
    fn get_admin(&self, id: &str) -> ApiResult<AdminSummary> {
        self.admins
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("admin `{id}` not found")))
    }

    /// 创建管理员账号并写入初始密码哈希。
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

    /// 更新管理员账号资料并同步角色名称。
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

    /// 修改管理员账号状态。
    fn set_admin_status(&mut self, id: &str, status: UserStatus) -> ApiResult<AdminSummary> {
        let admin = self
            .admins
            .get_mut(id)
            .ok_or_else(|| ApiError::NotFound(format!("admin `{id}` not found")))?;
        admin.status = status;
        Ok(admin.clone())
    }

    /// 重置管理员登录密码并替换密码哈希。
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

    /// 返回后台角色列表。
    fn roles(&self) -> Vec<AdminRole> {
        self.roles.values().cloned().collect()
    }

    /// 按角色 ID 读取角色详情。
    fn get_role(&self, id: &str) -> ApiResult<AdminRole> {
        self.roles
            .get(id)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("role `{id}` not found")))
    }

    /// 创建后台角色并校验名称唯一。
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

    /// 更新后台角色并同步管理员角色名称。
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

    /// 删除未被管理员占用的后台角色。
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

    /// 返回系统设置列表。
    fn settings(&self) -> Vec<SystemSetting> {
        self.settings.values().cloned().collect()
    }

    /// 按配置键读取单项系统设置。
    fn setting(&self, key: &str) -> ApiResult<SystemSetting> {
        self.settings
            .get(key)
            .cloned()
            .ok_or_else(|| ApiError::NotFound(format!("setting `{key}` not found")))
    }

    /// 更新系统设置值和中文说明。
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

    /// 更新注册开关和注册安全策略。
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

    /// 校验管理员账号密码并生成后台登录会话。
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

        let token = self.next_session_token()?;
        self.sessions
            .insert(session_token_hash(&token), admin.id.clone());
        self.session_from_token(&token)
    }

    /// 按后台 token 解析管理员会话。
    fn session_from_token(&self, token: &str) -> ApiResult<AdminAuthSession> {
        let token = token.trim();
        if token.is_empty() {
            return Err(ApiError::Unauthorized(
                "authorization token is required".to_string(),
            ));
        }

        let token_hash = session_token_hash(token);
        let admin_id = self
            .sessions
            .get(&token_hash)
            .ok_or_else(|| ApiError::Unauthorized("invalid admin session".to_string()))?;
        let admin = self.get_admin(admin_id)?;
        if admin.status != UserStatus::Active {
            return Err(ApiError::Forbidden(
                "admin account is not active".to_string(),
            ));
        }
        let role = self.get_role(&admin.role_id)?;
        let permissions = session_permissions_for_role(&role);

        Ok(AdminAuthSession {
            admin,
            permissions,
            scopes: role.scopes.clone(),
            role,
            token: token.to_string(),
        })
    }

    /// 删除后台登录 token。
    fn logout(&mut self, token: &str) -> ApiResult<()> {
        self.sessions.remove(&session_token_hash(token.trim()));
        Ok(())
    }

    /// 角色名称变更后同步到管理员摘要。
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

    /// 生成管理员会话原始 token，并确保摘要不与现有会话冲突。
    fn next_session_token(&self) -> ApiResult<String> {
        random_unique_session_token(&self.sessions)
    }

    /// 生成用户会话原始 token，并确保摘要不与现有会话冲突。
    fn next_user_session_token(&self) -> ApiResult<String> {
        random_unique_session_token(&self.user_sessions)
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
    user.avatar_url = normalize_avatar_url(user.avatar_url)?;
    user.contact_qq = normalize_contact_qq(user.contact_qq)?;
    user.agent_id = user
        .agent_id
        .map(|agent_id| agent_id.trim().to_string())
        .filter(|agent_id| !agent_id.is_empty());
    user.invite_code = user.invite_code.trim().to_string();
    user.registration_location = normalize_registration_location(user.registration_location);
    user.created_at = user.created_at.trim().to_string();
    if user.created_at.is_empty() {
        user.created_at = format_local_time();
    }

    if user.balance_minor < 0 {
        return Err(ApiError::BadRequest(
            "user balance must not be negative".to_string(),
        ));
    }

    Ok(user)
}

/// 从注册请求中整理注册地，保留服务端写入的请求 IP，并补充可读来源。
fn registration_location_from_request(
    location: Option<UserRegistrationLocation>,
) -> UserRegistrationLocation {
    normalize_registration_location(location.unwrap_or_default())
}

/// 统一清洗注册地字段，避免空白、异常来源和内网 IP 在列表中直接暴露为难读内容。
fn normalize_registration_location(
    mut location: UserRegistrationLocation,
) -> UserRegistrationLocation {
    location.registered_ip = location.registered_ip.trim().to_string();
    location.country = location.country.trim().to_string();
    location.region = location.region.trim().to_string();
    location.city = location.city.trim().to_string();
    location.source = normalize_registration_source(&location.source);

    if location.source == "gps" {
        if location.country.is_empty() && location.region.is_empty() && location.city.is_empty() {
            location.source = if location.registered_ip.is_empty() {
                "unknown".to_string()
            } else {
                "ip".to_string()
            };
        }
    } else {
        let has_server_ip_location = location.source == "ip" && !location.registered_ip.is_empty();
        if has_server_ip_location {
            location.country = location.country.trim().to_string();
            location.region = location.region.trim().to_string();
            location.city = location.city.trim().to_string();
        } else {
            location.country.clear();
            location.region.clear();
            location.city.clear();
        }
        location.source = if location.registered_ip.is_empty() {
            "unknown".to_string()
        } else {
            "ip".to_string()
        };
    }

    if is_local_or_private_ip(&location.registered_ip)
        && location.country.is_empty()
        && location.region.is_empty()
        && location.city.is_empty()
    {
        location.country = "内网".to_string();
        location.region = "本地网络".to_string();
        location.source = "ip".to_string();
    }

    location
}

/// 将客户端传入的定位来源收敛到后台可展示的固定枚举。
fn normalize_registration_source(source: &str) -> String {
    match source.trim().to_ascii_lowercase().as_str() {
        "gps" | "geo" | "geolocation" => "gps".to_string(),
        "ip" | "client_ip" => "ip".to_string(),
        "client" | "manual" => "client".to_string(),
        _ => "unknown".to_string(),
    }
}

/// 判断注册 IP 是否属于本机、内网或保留地址，用于给本地联调账号显示友好注册地。
fn is_local_or_private_ip(ip: &str) -> bool {
    ip.parse::<IpAddr>().map_or(false, |addr| match addr {
        IpAddr::V4(addr) => {
            addr.is_loopback()
                || addr.is_private()
                || addr.is_link_local()
                || addr.is_unspecified()
                || addr.octets() == [255, 255, 255, 255]
                || addr.octets()[0] == 100 && (64..=127).contains(&addr.octets()[1])
        }
        IpAddr::V6(addr) => addr.is_loopback() || addr.is_unspecified() || addr.is_unique_local(),
    })
}

/// 按用户状态返回登录拦截文案，让停用和锁定在用户端表现为不同原因。
fn inactive_user_status_message(status: &UserStatus) -> &'static str {
    match status {
        UserStatus::Suspended => "用户账号已停用",
        UserStatus::Locked => "用户账号已锁定",
        UserStatus::Active => "用户账号未激活",
    }
}

/// 标准化注册 QQ 联系方式：注册时必须填写 5-12 位数字。
fn normalize_required_contact_qq(value: Option<String>) -> ApiResult<String> {
    let value = normalize_contact_qq(value.unwrap_or_default())?;
    if value.is_empty() {
        return Err(ApiError::BadRequest("QQ 号码不能为空".to_string()));
    }
    Ok(value)
}

/// 标准化用户 QQ 联系方式：后台维护允许为空，填写时必须是 5-12 位数字。
fn normalize_contact_qq(value: String) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(value);
    }
    let len = value.chars().count();
    if len < MIN_CONTACT_QQ_LEN || len > MAX_CONTACT_QQ_LEN {
        return Err(ApiError::BadRequest(format!(
            "QQ 号码需要是 {MIN_CONTACT_QQ_LEN}-{MAX_CONTACT_QQ_LEN} 位数字"
        )));
    }
    if !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(ApiError::BadRequest("QQ 号码只能填写数字".to_string()));
    }

    Ok(value)
}

/// 标准化头像链接：空值表示未设置，非空值只接受 http/https 链接。
fn normalize_avatar_url(value: String) -> ApiResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(value);
    }
    if value.chars().count() > MAX_AVATAR_URL_LEN {
        return Err(ApiError::BadRequest(
            "头像链接不能超过 500 个字符".to_string(),
        ));
    }
    if !(value.starts_with("https://") || value.starts_with("http://")) {
        return Err(ApiError::BadRequest(
            "头像链接必须是 http 或 https 链接".to_string(),
        ));
    }

    Ok(value)
}

/// 依据当前用户集合随机生成 8 位大写字母数字邀请码，最多尝试 128 次防止极端碰撞。
fn random_invite_code(users: &BTreeMap<String, UserSummary>) -> ApiResult<String> {
    for _ in 0..128 {
        let code = random_invite_code_candidate();

        if invite_code_has_required_charset(&code)
            && !users.values().any(|user| user.invite_code == code)
        {
            return Ok(code);
        }
    }

    Err(ApiError::Internal("邀请码生成失败".to_string()))
}

/// 从允许字符集中生成一个候选邀请码，最终是否可用由格式和唯一性校验决定。
fn random_invite_code_candidate() -> String {
    let mut bytes = [0u8; INVITE_CODE_LENGTH];
    OsRng.fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|byte| {
            let index = usize::from(*byte % INVITE_CODE_ALPHABET.len() as u8);
            INVITE_CODE_ALPHABET[index] as char
        })
        .collect::<String>()
}

/// 校验自动生成的邀请码必须同时包含大写字母和数字，避免退化成纯字母或纯数字。
fn invite_code_has_required_charset(code: &str) -> bool {
    code.chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        && code.chars().any(|ch| ch.is_ascii_uppercase())
        && code.chars().any(|ch| ch.is_ascii_digit())
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

/// 基于前缀 U 生成下一个用户 ID，并保证与现有用户主键不冲突。
fn next_user_id(users: &BTreeMap<String, UserSummary>, counter: &mut u64) -> ApiResult<String> {
    if *counter == 0 {
        *counter = next_user_id_from_users(users);
    }

    for _ in 0..2048 {
        *counter = counter
            .checked_add(1)
            .ok_or_else(|| ApiError::Internal("用户 ID 序号溢出".to_string()))?;
        let user_id = format!("U{:0>5}", *counter);
        if !users.contains_key(&user_id) {
            return Ok(user_id);
        }
    }

    Err(ApiError::Internal("用户 ID 生成失败".to_string()))
}

/// 基于现有用户集合推导下一个用户 ID 自增序号。
fn next_user_id_from_users(users: &BTreeMap<String, UserSummary>) -> u64 {
    users
        .keys()
        .filter_map(|id| id.strip_prefix('U'))
        .filter_map(|suffix| suffix.parse::<u64>().ok())
        .max()
        .unwrap_or(10000)
}

/// 验证普通用户密码规则。
fn validate_user_password(password: &str) -> ApiResult<String> {
    let password = required_trimmed(password.to_string(), "password")?;
    if password.chars().count() < MIN_USER_PASSWORD_LEN {
        return Err(ApiError::BadRequest(format!(
            "用户密码至少 {MIN_USER_PASSWORD_LEN} 位字符"
        )));
    }

    Ok(password)
}

/// 对用户密码进行安全哈希。
fn hash_user_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::Internal("用户密码哈希失败".to_string()))
}

/// 校验用户密码是否匹配。
fn verify_user_password(password: &str, password_hash: &str) -> ApiResult<bool> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|_| ApiError::Internal("用户密码哈希格式无效".to_string()))?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// 返回当前系统时间 Unix 秒级时间戳。
fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// 将 Unix 秒级时间戳转换为本地可读时间。
fn format_unix_timestamp(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .map(|datetime| datetime.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| format!("{timestamp}"))
}

/// 格式化当前本地时间。
fn format_local_time() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 生成随机数字字母组合。
fn random_alnum_string(length: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    let mut bytes = vec![0u8; length];
    OsRng.fill_bytes(&mut bytes);
    let max = ALPHABET.len() as u8;

    bytes
        .into_iter()
        .map(|byte| {
            let index = usize::from(byte % max);
            ALPHABET[index] as char
        })
        .collect()
}

/// 生成不包含账号信息的强随机会话 token。
fn random_session_token() -> String {
    let mut bytes = vec![0u8; SESSION_TOKEN_RANDOM_BYTES];
    OsRng.fill_bytes(&mut bytes);
    format!("{SESSION_TOKEN_PREFIX}{}", hex_encode(&bytes))
}

/// 生成不与现有会话摘要冲突的强随机会话 token。
fn random_unique_session_token(sessions: &BTreeMap<String, String>) -> ApiResult<String> {
    for _ in 0..256 {
        let token = random_session_token();
        if !sessions.contains_key(&session_token_hash(&token)) {
            return Ok(token);
        }
    }

    Err(ApiError::Internal("会话 token 生成失败".to_string()))
}

/// 计算会话 token 摘要，数据库和内存会话索引只保存该摘要。
fn session_token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.trim().as_bytes());
    format!("{SESSION_TOKEN_HASH_PREFIX}{}", hex_encode(&digest))
}

/// 把二进制数据编码为小写十六进制字符串。
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    output
}

/// 生成不重复的提现方式 ID。
fn random_withdrawal_method_id(
    methods: &BTreeMap<String, WithdrawalMethod>,
    byte_len: usize,
) -> ApiResult<String> {
    for _ in 0..256 {
        let random = random_alnum_string(byte_len);
        let id = format!("WM-{random}");
        if !methods.contains_key(&id) {
            return Ok(id);
        }
    }

    Err(ApiError::Internal("提现方式 ID 生成失败".to_string()))
}

/// 按现有用户集合生成用户密码哈希映射。
fn seed_user_password_hashes(users: &BTreeMap<String, UserSummary>) -> BTreeMap<String, String> {
    users
        .keys()
        .map(|user_id| {
            let hash = hash_user_password(DEFAULT_SEED_USER_PASSWORD)
                .unwrap_or_else(|_| panic!("种子用户密码哈希失败"));
            (user_id.clone(), hash)
        })
        .collect()
}

/// 校验管理员密码长度和空白字符规则。
fn validate_admin_password(password: &str) -> ApiResult<String> {
    let password = required_trimmed(password.to_string(), "admin password")?;
    if password.chars().count() < MIN_ADMIN_PASSWORD_LEN {
        return Err(ApiError::BadRequest(format!(
            "admin password must be at least {MIN_ADMIN_PASSWORD_LEN} characters"
        )));
    }

    Ok(password)
}

/// 使用 Argon2 生成管理员密码哈希。
fn hash_admin_password(password: &str) -> ApiResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::Internal("admin password hash failed".to_string()))
}

/// 校验管理员输入密码是否匹配已保存哈希。
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
    if role.scopes.is_empty() && role.permissions.is_empty() {
        return Err(ApiError::BadRequest(
            "至少需要配置一个模块权限或操作权限".to_string(),
        ));
    }

    let mut seen = HashSet::new();
    role.scopes.retain(|scope| seen.insert(scope.clone()));
    let mut seen_permissions = BTreeSet::new();
    let mut permissions = Vec::new();
    for permission in role.permissions {
        let permission = permission.trim().to_string();
        if permission.is_empty() {
            continue;
        }
        if !is_known_permission_key(&permission) {
            return Err(ApiError::BadRequest(format!(
                "未知的后台权限点：{permission}"
            )));
        }
        if seen_permissions.insert(permission.clone()) {
            permissions.push(permission);
        }
    }
    role.permissions = permissions;
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
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 12_000,
            agent_id: Some("U90001".to_string()),
            invite_code: DEMO_USER_INVITE_CODE.to_string(),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 10:00:00".to_string(),
        },
        UserSummary {
            id: "U90001".to_string(),
            username: "agent_alpha".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Agent,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: None,
            invite_code: DEMO_AGENT_INVITE_CODE.to_string(),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90002".to_string(),
            username: "robot_fill_02".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90002"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90003".to_string(),
            username: "robot_fill_03".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90003"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90004".to_string(),
            username: "robot_fill_04".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90004"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90005".to_string(),
            username: "robot_fill_05".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90005"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90006".to_string(),
            username: "robot_fill_06".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90006"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90007".to_string(),
            username: "robot_fill_07".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90007"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90008".to_string(),
            username: "robot_fill_08".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90008"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90009".to_string(),
            username: "robot_fill_09".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90009"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "X90010".to_string(),
            username: "robot_fill_10".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: Some("U90001".to_string()),
            invite_code: format!("ROBOT-X90010"),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 09:00:00".to_string(),
        },
        UserSummary {
            id: "U10004".to_string(),
            username: "risk_watch".to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind: crate::domain::user::UserKind::Regular,
            status: UserStatus::Suspended,
            balance_minor: 0,
            agent_id: Some("U90001".to_string()),
            invite_code: RISK_USER_INVITE_CODE.to_string(),
            registration_location: UserRegistrationLocation::default(),
            created_at: "2026-06-01 11:00:00".to_string(),
        },
    ]
}

/// 返回初始化内置管理员账号。
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

/// 为内置管理员生成默认密码哈希。
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

/// 返回初始化内置角色和权限范围。
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
            permissions: admin_permission_definitions()
                .iter()
                .map(|definition| definition.key.to_string())
                .collect(),
        },
        AdminRole {
            id: "role-ops".to_string(),
            name: "运营管理员".to_string(),
            scopes: vec![
                PermissionScope::Users,
                PermissionScope::Orders,
                PermissionScope::Lotteries,
            ],
            permissions: Vec::new(),
        },
    ]
}

/// 返回系统设置的初始化默认值。
fn seed_settings() -> Vec<SystemSetting> {
    vec![
        SystemSetting {
            key: "email_registration_enabled".to_string(),
            value: "false".to_string(),
            description: "是否开启邮箱注册".to_string(),
        },
        SystemSetting {
            key: "image_bed_upload_url".to_string(),
            value: "https://oss.moonight.cc.cd/api/v1/upload".to_string(),
            description: "图床上传接口地址".to_string(),
        },
        SystemSetting {
            key: "image_bed_authorization_token".to_string(),
            value: String::new(),
            description: "图床请求 Authorization Token（不含 Bearer 前缀，必须在后台手动配置）"
                .to_string(),
        },
        SystemSetting {
            key: "image_bed_upload_field".to_string(),
            value: "file".to_string(),
            description: "图床上传字段名".to_string(),
        },
        SystemSetting {
            key: "image_bed_result_url_field".to_string(),
            value: "links.download".to_string(),
            description: "图床返回中的图片链接字段（支持点号路径）".to_string(),
        },
        SystemSetting {
            key: "mobile_platform_name".to_string(),
            value: "彩票管理系统".to_string(),
            description: "手机端展示的平台名称".to_string(),
        },
        SystemSetting {
            key: "mobile_logo_image_url".to_string(),
            value: "未配置".to_string(),
            description: "手机端站点 Logo 图片链接".to_string(),
        },
        SystemSetting {
            key: "mobile_site_intro".to_string(),
            value: "欢迎使用彩票管理系统，祝您理性购彩、好运常伴。".to_string(),
            description: "手机端首页或关于页面展示的站点介绍".to_string(),
        },
        SystemSetting {
            key: "mobile_home_featured_enabled".to_string(),
            value: "false".to_string(),
            description: "手机端首页高频极速模块开关，默认关闭".to_string(),
        },
        SystemSetting {
            key: "mobile_home_featured_title".to_string(),
            value: "高频极速".to_string(),
            description: "手机端首页高频极速模块标题".to_string(),
        },
        SystemSetting {
            key: "mobile_home_featured_lottery_codes".to_string(),
            value: String::new(),
            description: "手机端首页高频极速展示彩种 ID，多个用英文逗号分隔".to_string(),
        },
        SystemSetting {
            key: "mobile_app_android_enabled".to_string(),
            value: "false".to_string(),
            description: "Android APP 更新检查开关".to_string(),
        },
        SystemSetting {
            key: "mobile_app_android_latest_version".to_string(),
            value: "0.1.0".to_string(),
            description: "Android APP 最新版本号".to_string(),
        },
        SystemSetting {
            key: "mobile_app_android_latest_build".to_string(),
            value: "1".to_string(),
            description: "Android APP 最新构建号，数字越大版本越新".to_string(),
        },
        SystemSetting {
            key: "mobile_app_android_package_url".to_string(),
            value: "未配置".to_string(),
            description: "Android APK 安装包下载链接".to_string(),
        },
        SystemSetting {
            key: "mobile_app_android_force_update".to_string(),
            value: "false".to_string(),
            description: "Android APP 是否强制更新".to_string(),
        },
        SystemSetting {
            key: "mobile_app_android_release_notes".to_string(),
            value: String::new(),
            description: "Android APP 更新说明".to_string(),
        },
        SystemSetting {
            key: "mobile_app_ios_enabled".to_string(),
            value: "false".to_string(),
            description: "iOS APP 更新检查开关".to_string(),
        },
        SystemSetting {
            key: "mobile_app_ios_latest_version".to_string(),
            value: "0.1.0".to_string(),
            description: "iOS APP 最新版本号".to_string(),
        },
        SystemSetting {
            key: "mobile_app_ios_latest_build".to_string(),
            value: "1".to_string(),
            description: "iOS APP 最新构建号，数字越大版本越新".to_string(),
        },
        SystemSetting {
            key: "mobile_app_ios_package_url".to_string(),
            value: "未配置".to_string(),
            description: "iOS IPA 安装包下载链接".to_string(),
        },
        SystemSetting {
            key: "mobile_app_ios_force_update".to_string(),
            value: "false".to_string(),
            description: "iOS APP 是否强制更新".to_string(),
        },
        SystemSetting {
            key: "mobile_app_ios_release_notes".to_string(),
            value: String::new(),
            description: "iOS APP 更新说明".to_string(),
        },
        SystemSetting {
            key: "recharge_rebate_mode".to_string(),
            value: "immediate".to_string(),
            description: "代理充值返利模式".to_string(),
        },
        SystemSetting {
            key: "recharge_min_amount_minor".to_string(),
            value: "100".to_string(),
            description: "用户单笔充值最小金额（元）".to_string(),
        },
        SystemSetting {
            key: "recharge_max_amount_minor".to_string(),
            value: "10000000".to_string(),
            description: "用户单笔充值最大金额（元）".to_string(),
        },
        SystemSetting {
            key: "recharge_rainbow_epay_enabled".to_string(),
            value: "false".to_string(),
            description: "是否开启彩虹易支付在线充值".to_string(),
        },
        SystemSetting {
            key: "recharge_rainbow_epay_gateway_url".to_string(),
            value: "https://pay.example.com".to_string(),
            description: "彩虹易支付网关域名，不需要填写 submit.php".to_string(),
        },
        SystemSetting {
            key: "recharge_rainbow_epay_pid".to_string(),
            value: "未配置".to_string(),
            description: "彩虹易支付商户号".to_string(),
        },
        SystemSetting {
            key: "recharge_rainbow_epay_key".to_string(),
            value: "未配置".to_string(),
            description: "彩虹易支付商户密钥".to_string(),
        },
        SystemSetting {
            key: "recharge_rainbow_epay_notify_url".to_string(),
            value: "/api/user/recharge/epay/notify".to_string(),
            description: "彩虹易支付异步通知地址，生产环境建议填写完整外网 URL".to_string(),
        },
        SystemSetting {
            key: "recharge_rainbow_epay_return_url".to_string(),
            value: "/api/user/recharge/epay/return".to_string(),
            description: "彩虹易支付同步返回地址，生产环境建议填写完整外网 URL".to_string(),
        },
        SystemSetting {
            key: "recharge_rainbow_epay_pay_types".to_string(),
            value: "alipay,wxpay".to_string(),
            description: "彩虹易支付允许的支付方式，多个值用英文逗号分隔".to_string(),
        },
        SystemSetting {
            key: "recharge_customer_service_enabled".to_string(),
            value: "true".to_string(),
            description: "是否开启客服直充".to_string(),
        },
        SystemSetting {
            key: "recharge_customer_service_message".to_string(),
            value: "客服已收到您的直充申请，请在会话中确认付款方式和到账信息。".to_string(),
            description: "客服直充创建订单后返回给用户的提示文案".to_string(),
        },
        SystemSetting {
            key: "recharge_bonus_enabled".to_string(),
            value: "false".to_string(),
            description: "是否开启用户充值赠送活动".to_string(),
        },
        SystemSetting {
            key: "recharge_bonus_rules".to_string(),
            value: "[]".to_string(),
            description: "用户充值赠送活动档位，支持固定金额或百分比赠送".to_string(),
        },
        SystemSetting {
            key: "chat_hall_speaking_min_recharge_minor".to_string(),
            value: "0".to_string(),
            description: "聊天大厅发言最低累计充值金额（元），0 表示不限制".to_string(),
        },
        SystemSetting {
            key: "withdrawal_turnover_enabled".to_string(),
            value: "false".to_string(),
            description: "是否开启提现前充值等额有效投注要求".to_string(),
        },
        SystemSetting {
            key: "support_telegram_notification_enabled".to_string(),
            value: "false".to_string(),
            description: "是否开启新客服消息 Telegram 提醒".to_string(),
        },
        SystemSetting {
            key: "support_telegram_bot_token".to_string(),
            value: "未配置".to_string(),
            description: "Telegram Bot Token，仅用于客服新消息提醒".to_string(),
        },
        SystemSetting {
            key: "support_telegram_chat_id".to_string(),
            value: "未配置".to_string(),
            description: "Telegram 接收提醒的 Chat ID、群组 ID 或频道用户名".to_string(),
        },
    ]
}
/// 补齐缺失的系统设置默认项，避免旧库升级后读取失败。
fn fill_missing_system_settings(settings: &mut BTreeMap<String, SystemSetting>) -> bool {
    let mut changed = false;

    for setting in seed_settings() {
        if !settings.contains_key(&setting.key) {
            settings.insert(setting.key.clone(), setting);
            changed = true;
        }
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::UserKind;
    use crate::domain::user::WithdrawalMethodType;
    use crate::domain::user::{
        UserChangePasswordRequest, UserForgotPasswordRequest, UserLoginRequest,
        UserPasswordResetRequest, UserRegisterRequest, UserResetPasswordRequest,
        WithdrawalMethodRequest,
    };
    /// 断言邀请code格式满足测试要求。
    fn assert_invite_code_format(code: &str) {
        assert_eq!(code.len(), INVITE_CODE_LENGTH);
        assert!(code
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit()));
        assert!(code.chars().any(|ch| ch.is_ascii_uppercase()));
        assert!(code.chars().any(|ch| ch.is_ascii_digit()));
    }
    /// 断言会话令牌isopaque满足测试要求。
    fn assert_session_token_is_opaque(token: &str) {
        assert_eq!(
            token.len(),
            SESSION_TOKEN_PREFIX.len() + SESSION_TOKEN_RANDOM_BYTES * 2
        );
        assert!(token.starts_with(SESSION_TOKEN_PREFIX));
        assert!(token[SESSION_TOKEN_PREFIX.len()..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
    }
    /// 验证内置用户邀请码使用随机字母数字格式。
    #[test]
    fn seed_invite_codes_use_alnum_format() {
        assert_invite_code_format(DEMO_USER_INVITE_CODE);
        assert_invite_code_format(DEMO_AGENT_INVITE_CODE);
        assert_invite_code_format(RISK_USER_INVITE_CODE);
    }
    /// 验证访问控制仓储可以创建并更新用户。
    #[tokio::test]
    async fn access_repository_creates_and_updates_user() {
        let access = AccessRepository::memory_seeded();
        let created = access
            .create_user(UserSummary {
                id: " U20001 ".to_string(),
                username: "new_user".to_string(),
                email: Some("new@example.com".to_string()),
                avatar_url: String::new(),
                contact_qq: "123456".to_string(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 1000,
                agent_id: None,
                invite_code: String::new(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            })
            .await
            .expect("user can be created");

        assert_eq!(created.id, "U20001");
        assert_invite_code_format(&created.invite_code);

        let updated = access
            .set_user_status("U20001", UserStatus::Locked)
            .await
            .expect("status can be updated");
        assert_eq!(updated.status, UserStatus::Locked);
    }

    /// 验证后台用户分页入口支持按用户名关键字搜索，且大小写不敏感。
    #[tokio::test]
    async fn access_repository_user_page_filters_by_username() {
        let access = AccessRepository::memory_seeded();
        access
            .create_user(UserSummary {
                id: "U20010".to_string(),
                username: "Search_Target".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: String::new(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-21 14:20:00".to_string(),
            })
            .await
            .expect("search target user can be created");

        let page = access
            .user_page(
                false,
                None,
                Some("target"),
                None,
                "id",
                "desc",
                PageRequest::new(Some(1), Some(10)),
            )
            .await
            .expect("user page can be filtered by username");

        assert_eq!(page.total_count, 1);
        assert_eq!(page.items[0].username, "Search_Target");
    }

    /// 验证后台用户分页可以按上级代理筛选直属下级，并且筛选发生在分页前。
    #[tokio::test]
    async fn access_repository_user_page_filters_by_agent_before_pagination() {
        let access = AccessRepository::memory_seeded();
        for user in [
            UserSummary {
                id: "U21001".to_string(),
                username: "agent_filter_alpha".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Agent,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: "AF21001".to_string(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-28 10:00:00".to_string(),
            },
            UserSummary {
                id: "U21002".to_string(),
                username: "agent_filter_beta".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Agent,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: "AF21002".to_string(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-28 10:01:00".to_string(),
            },
            UserSummary {
                id: "U21003".to_string(),
                username: "alpha_direct_one".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: Some("U21001".to_string()),
                invite_code: String::new(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-28 10:02:00".to_string(),
            },
            UserSummary {
                id: "U21004".to_string(),
                username: "alpha_direct_two".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: Some("U21001".to_string()),
                invite_code: String::new(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-28 10:03:00".to_string(),
            },
            UserSummary {
                id: "U21005".to_string(),
                username: "beta_direct_one".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: Some("U21002".to_string()),
                invite_code: String::new(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-28 10:04:00".to_string(),
            },
        ] {
            access
                .create_user(user)
                .await
                .expect("agent filter fixture user can be created");
        }

        let page = access
            .user_page(
                false,
                None,
                None,
                Some("U21001"),
                "id",
                "desc",
                PageRequest::new(Some(1), Some(1)),
            )
            .await
            .expect("user page can be filtered by agent");

        assert_eq!(page.total_count, 2);
        assert_eq!(page.items.len(), 1);
        assert!(page
            .items
            .iter()
            .all(|user| user.agent_id.as_deref() == Some("U21001")));
    }

    /// 验证后台用户分页默认隐藏机器人账号，显式打开开关后才返回。
    #[tokio::test]
    async fn access_repository_user_page_hides_robot_users_by_default() {
        let access = AccessRepository::memory_seeded();

        let default_page = access
            .user_page(
                false,
                None,
                Some("agent_alpha"),
                None,
                "id",
                "desc",
                PageRequest::new(Some(1), Some(10)),
            )
            .await
            .expect("user page can filter robot users by default");
        let visible_page = access
            .user_page(
                true,
                None,
                Some("agent_alpha"),
                None,
                "id",
                "desc",
                PageRequest::new(Some(1), Some(10)),
            )
            .await
            .expect("user page can include robot users");

        assert_eq!(default_page.total_count, 0);
        assert_eq!(visible_page.total_count, 1);
        assert_eq!(visible_page.items[0].id, "U90001");
    }

    /// 验证存在直属邀请下级时不允许删除用户。
    #[tokio::test]
    async fn access_repository_rejects_delete_user_with_direct_invitees() {
        let access = AccessRepository::memory_seeded();

        let error = access
            .delete_user("U90001")
            .await
            .expect_err("agent with direct invitees cannot be deleted");

        assert!(
            matches!(error, ApiError::Conflict(message) if message == "该用户仍有下级用户，请先调整下级代理关系")
        );
    }
    /// 验证删除用户时同步清理会话、密码和提现方式等访问数据。
    #[tokio::test]
    async fn access_repository_deletes_user_and_access_artifacts() {
        let access = AccessRepository::memory_seeded();
        let session = access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "12345678".to_string(),
            })
            .await
            .expect("active user can login");
        let reset = access
            .request_forgot_password(UserForgotPasswordRequest {
                login_key: "demo_user".to_string(),
            })
            .await
            .expect("reset token can be created");
        let method = access
            .create_withdrawal_method(
                "U10001",
                WithdrawalMethodRequest {
                    method_type: WithdrawalMethodType::Alipay,
                    account_holder: "测试用户".to_string(),
                    account_number: "demo@example.com".to_string(),
                    bank_name: None,
                    is_default: true,
                },
            )
            .await
            .expect("withdrawal method can be created");

        let deleted = access
            .delete_user("U10001")
            .await
            .expect("regular user can be deleted");

        assert_eq!(deleted.id, "U10001");
        assert!(matches!(
            access.get_user("U10001").await,
            Err(ApiError::NotFound(_))
        ));
        assert!(matches!(
            access.session_from_user_token(&session.token).await,
            Err(ApiError::Unauthorized(_))
        ));
        assert!(matches!(
            access.list_withdrawal_methods("U10001").await,
            Err(ApiError::NotFound(_))
        ));

        let store = access.inner.read().expect("access store can be read");
        assert!(!store.user_password_hashes.contains_key("U10001"));
        assert!(!store
            .user_password_reset_tokens
            .contains_key(&reset.reset_token));
        assert!(!store.user_withdrawal_methods.contains_key(&method.id));
    }
    /// 验证访问控制仓储生成的邀请码保持唯一。
    #[tokio::test]
    async fn access_repository_generates_unique_invite_codes() {
        let access = AccessRepository::memory_seeded();
        let first = access
            .create_user(UserSummary {
                id: "U20003".to_string(),
                username: "random_invite_a".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: String::new(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            })
            .await
            .expect("first user can be created");
        let second = access
            .create_user(UserSummary {
                id: "U20004".to_string(),
                username: "random_invite_b".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: String::new(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:01:00".to_string(),
            })
            .await
            .expect("second user can be created");

        assert_invite_code_format(&first.invite_code);
        assert_invite_code_format(&second.invite_code);
        assert_ne!(first.invite_code, second.invite_code);
    }
    /// 验证后台更新用户资料时保留用户名、余额和邀请码。
    #[tokio::test]
    async fn access_repository_update_preserves_username_balance_and_invite_code() {
        let access = AccessRepository::memory_seeded();
        let original = access.get_user("U10001").await.expect("seed user exists");

        let updated = access
            .update_user(
                "U10001",
                UserSummary {
                    id: "U10001".to_string(),
                    username: "renamed_demo".to_string(),
                    email: original.email.clone(),
                    avatar_url: "https://example.com/should-not-override.png".to_string(),
                    contact_qq: "234567".to_string(),
                    kind: original.kind.clone(),
                    status: original.status.clone(),
                    balance_minor: 999_999,
                    agent_id: original.agent_id.clone(),
                    invite_code: "ZZZZ9999".to_string(),
                    registration_location: UserRegistrationLocation {
                        registered_ip: "8.8.8.8".to_string(),
                        country: "测试国家".to_string(),
                        region: String::new(),
                        city: String::new(),
                        source: "client".to_string(),
                    },
                    created_at: "2026-06-05 12:00:00".to_string(),
                },
            )
            .await
            .expect("user can be updated");

        assert_eq!(updated.username, original.username);
        assert_eq!(updated.balance_minor, original.balance_minor);
        assert_eq!(updated.invite_code, original.invite_code);
        assert_eq!(updated.avatar_url, original.avatar_url);
        assert_eq!(
            updated.registration_location,
            original.registration_location
        );
        assert_eq!(updated.contact_qq, "234567");
    }
    /// 验证用户头像可以更新并持久化。
    #[tokio::test]
    async fn access_repository_updates_user_avatar() {
        let access = AccessRepository::memory_seeded();
        let updated = access
            .update_user_avatar(
                "U10001",
                UserAvatarRequest {
                    avatar_url: " https://cdn.example.com/avatar.png ".to_string(),
                },
            )
            .await
            .expect("avatar can be updated");

        assert_eq!(updated.avatar_url, "https://cdn.example.com/avatar.png");
        assert_eq!(
            access
                .get_user("U10001")
                .await
                .expect("user can be loaded")
                .avatar_url,
            "https://cdn.example.com/avatar.png"
        );
    }
    /// 验证非法头像地址会被拒绝。
    #[tokio::test]
    async fn access_repository_rejects_invalid_user_avatar_url() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .update_user_avatar(
                "U10001",
                UserAvatarRequest {
                    avatar_url: "ftp://cdn.example.com/avatar.png".to_string(),
                },
            )
            .await
            .expect_err("invalid avatar url should be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }
    /// 验证重复邀请码不会被写入。
    #[tokio::test]
    async fn access_repository_rejects_duplicate_invite_code() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .create_user(UserSummary {
                id: "U20002".to_string(),
                username: "duplicate_invite_code".to_string(),
                email: None,
                avatar_url: String::new(),
                contact_qq: String::new(),
                kind: UserKind::Regular,
                status: UserStatus::Active,
                balance_minor: 0,
                agent_id: None,
                invite_code: DEMO_AGENT_INVITE_CODE.to_string(),
                registration_location: UserRegistrationLocation::default(),
                created_at: "2026-06-05 10:00:00".to_string(),
            })
            .await
            .expect_err("duplicate invite code must be rejected");

        assert!(matches!(error, ApiError::Conflict(_)));
    }
    /// 验证角色权限范围不能为空。
    #[tokio::test]
    async fn access_repository_rejects_empty_role_scopes() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .create_role(AdminRole {
                id: "role-empty".to_string(),
                name: "空角色".to_string(),
                scopes: Vec::new(),
                permissions: Vec::new(),
            })
            .await
            .expect_err("empty scopes must be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }
    /// 验证已分配给管理员的角色不能删除。
    #[tokio::test]
    async fn access_repository_prevents_deleting_assigned_role() {
        let access = AccessRepository::memory_seeded();
        let error = access
            .delete_role("role-super")
            .await
            .expect_err("assigned role cannot be deleted");

        assert!(matches!(error, ApiError::Conflict(_)));
    }
    /// 验证角色名称更新后同步到管理员摘要。
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
                    permissions: Vec::new(),
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
    /// 验证关闭注册时用户注册会被拒绝。
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
    /// 验证启用状态管理员可以登录。
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
    /// 验证管理员会话 token 落库前会哈希。
    #[tokio::test]
    async fn access_repository_hashes_admin_session_token_at_rest() {
        let access = AccessRepository::memory_seeded();
        let session = access
            .login(AdminLoginRequest {
                username: "admin".to_string(),
                password: "admin123".to_string(),
            })
            .await
            .expect("active admin can login");

        assert_session_token_is_opaque(&session.token);
        assert!(!session.token.contains("A10001"));
        assert!(!session.token.starts_with("adm-"));

        let store = access.inner.read().expect("access store can be read");
        assert!(!store.sessions.contains_key(&session.token));
        assert!(store
            .sessions
            .contains_key(&session_token_hash(&session.token)));
        assert!(store
            .sessions
            .keys()
            .all(|token| token.starts_with(SESSION_TOKEN_HASH_PREFIX)));
    }
    /// 验证锁定管理员不能登录。
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
    /// 验证管理员密码错误时拒绝登录。
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
    /// 验证新增管理员使用独立密码哈希。
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
    /// 验证用户可按用户名或邮箱注册。
    #[tokio::test]
    async fn access_repository_registers_user_by_username_or_email() {
        let access = AccessRepository::memory_seeded();

        let username_user = access
            .register_user(UserRegisterRequest {
                username: Some("new_member".to_string()),
                email: None,
                contact_qq: Some("1234567".to_string()),
                password: "newPassword123".to_string(),
                invite_code: None,
                registration_location: Some(UserRegistrationLocation {
                    registered_ip: "192.168.2.10".to_string(),
                    country: String::new(),
                    region: String::new(),
                    city: String::new(),
                    source: "ip".to_string(),
                }),
            })
            .await
            .expect("username register should succeed");

        assert_eq!(username_user.username, "new_member");
        assert_eq!(username_user.contact_qq, "1234567");
        assert_eq!(
            username_user.registration_location.registered_ip,
            "192.168.2.10"
        );
        assert_eq!(username_user.registration_location.country, "内网");
        assert_invite_code_format(&username_user.invite_code);

        let _ = access
            .update_registration(RegistrationConfig {
                username_enabled: false,
                email_enabled: true,
                agent_invite_required: false,
            })
            .await
            .expect("register policy can be updated");

        let email_user = access
            .register_user(UserRegisterRequest {
                username: None,
                email: Some("mail_reg@example.com".to_string()),
                contact_qq: Some("7654321".to_string()),
                password: "emailPassword123".to_string(),
                invite_code: None,
                registration_location: None,
            })
            .await
            .expect("email register should succeed");

        assert_eq!(email_user.username, "mail_reg@example.com");
        assert_eq!(email_user.contact_qq, "7654321");
        assert!(access
            .login_user(UserLoginRequest {
                login_key: "mail_reg@example.com".to_string(),
                password: "emailPassword123".to_string(),
            })
            .await
            .is_ok());

        assert_eq!(
            access
                .registration()
                .await
                .expect("registration can be loaded")
                .email_enabled,
            true
        );
        assert_invite_code_format(&email_user.invite_code);

        assert_ne!(username_user.username, email_user.username);
    }

    /// 验证用户注册必须填写 QQ 联系方式，避免手机端必填规则被绕过。
    #[tokio::test]
    async fn access_repository_rejects_register_without_contact_qq() {
        let access = AccessRepository::memory_seeded();

        let err = access
            .register_user(UserRegisterRequest {
                username: Some("missing_qq".to_string()),
                email: None,
                contact_qq: None,
                password: "newPassword123".to_string(),
                invite_code: None,
                registration_location: None,
            })
            .await
            .expect_err("register without QQ should be rejected");

        assert!(matches!(err, ApiError::BadRequest(message) if message == "QQ 号码不能为空"));
    }

    /// 验证客户端推断的注册地不会覆盖服务端来源。
    #[tokio::test]
    async fn access_repository_discards_client_inferred_registration_location() {
        let access = AccessRepository::memory_seeded();

        let user = access
            .register_user(UserRegisterRequest {
                username: Some("client_location_user".to_string()),
                email: None,
                contact_qq: Some("2233445".to_string()),
                password: "locationPass123".to_string(),
                invite_code: None,
                registration_location: Some(UserRegistrationLocation {
                    registered_ip: "8.8.8.8".to_string(),
                    country: "US".to_string(),
                    region: "America/Los_Angeles".to_string(),
                    city: "Los Angeles".to_string(),
                    source: "client".to_string(),
                }),
            })
            .await
            .expect("user can be registered");

        assert_eq!(user.registration_location.registered_ip, "8.8.8.8");
        assert_eq!(user.registration_location.country, "");
        assert_eq!(user.registration_location.region, "");
        assert_eq!(user.registration_location.city, "");
        assert_eq!(user.registration_location.source, "ip");
    }
    /// 验证服务端从可信代理头写入的 IP 国家、省份和城市字段会保留。
    #[tokio::test]
    async fn access_repository_keeps_server_ip_country_registration_location() {
        let access = AccessRepository::memory_seeded();

        let user = access
            .register_user(UserRegisterRequest {
                username: Some("server_ip_country_user".to_string()),
                email: None,
                contact_qq: Some("3344556".to_string()),
                password: "locationPass123".to_string(),
                invite_code: None,
                registration_location: Some(UserRegistrationLocation {
                    registered_ip: "2409:8950:5353:80:c46d:c9ff:fec7:4f38".to_string(),
                    country: "中国".to_string(),
                    region: "广东".to_string(),
                    city: "深圳".to_string(),
                    source: "ip".to_string(),
                }),
            })
            .await
            .expect("user can be registered");

        assert_eq!(
            user.registration_location.registered_ip,
            "2409:8950:5353:80:c46d:c9ff:fec7:4f38"
        );
        assert_eq!(user.registration_location.country, "中国");
        assert_eq!(user.registration_location.region, "广东");
        assert_eq!(user.registration_location.city, "深圳");
        assert_eq!(user.registration_location.source, "ip");
    }
    /// 验证明确 GPS 来源的注册地会被保留。
    #[tokio::test]
    async fn access_repository_keeps_gps_registration_location() {
        let access = AccessRepository::memory_seeded();

        let user = access
            .register_user(UserRegisterRequest {
                username: Some("gps_location_user".to_string()),
                email: None,
                contact_qq: Some("4455667".to_string()),
                password: "locationPass123".to_string(),
                invite_code: None,
                registration_location: Some(UserRegistrationLocation {
                    registered_ip: "8.8.4.4".to_string(),
                    country: "中国".to_string(),
                    region: "广东".to_string(),
                    city: "深圳".to_string(),
                    source: "gps".to_string(),
                }),
            })
            .await
            .expect("user can be registered");

        assert_eq!(user.registration_location.registered_ip, "8.8.4.4");
        assert_eq!(user.registration_location.country, "中国");
        assert_eq!(user.registration_location.region, "广东");
        assert_eq!(user.registration_location.city, "深圳");
        assert_eq!(user.registration_location.source, "gps");
    }
    /// 验证用户会话 token 落库前会哈希。
    #[tokio::test]
    async fn access_repository_hashes_user_session_token_at_rest() {
        let access = AccessRepository::memory_seeded();
        let session = access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "12345678".to_string(),
            })
            .await
            .expect("active user can login");

        assert_session_token_is_opaque(&session.token);
        assert!(!session.token.contains("U10001"));
        assert!(!session.token.starts_with("user-"));

        let current = access
            .session_from_user_token(&session.token)
            .await
            .expect("user session can be resolved");
        assert_eq!(current.user.username, "demo_user");

        let store = access.inner.read().expect("access store can be read");
        assert!(!store.user_sessions.contains_key(&session.token));
        assert!(store
            .user_sessions
            .contains_key(&session_token_hash(&session.token)));
        assert!(store
            .user_sessions
            .keys()
            .all(|token| token.starts_with(SESSION_TOKEN_HASH_PREFIX)));
    }
    /// 验证停用和锁定用户登录错误文案可区分。
    #[tokio::test]
    async fn access_repository_distinguishes_suspended_and_locked_user_login_errors() {
        let access = AccessRepository::memory_seeded();

        access
            .set_user_status("U10001", UserStatus::Suspended)
            .await
            .expect("user can be suspended");
        let suspended_error = access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "12345678".to_string(),
            })
            .await
            .expect_err("suspended user cannot login");
        assert!(matches!(
            suspended_error,
            ApiError::Forbidden(message) if message == "用户账号已停用"
        ));

        access
            .set_user_status("U10001", UserStatus::Active)
            .await
            .expect("user can be enabled");
        let session = access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "12345678".to_string(),
            })
            .await
            .expect("active user can login");
        access
            .set_user_status("U10001", UserStatus::Locked)
            .await
            .expect("user can be locked");

        let locked_login_error = access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "12345678".to_string(),
            })
            .await
            .expect_err("locked user cannot login");
        assert!(matches!(
            locked_login_error,
            ApiError::Forbidden(message) if message == "用户账号已锁定"
        ));

        let locked_session_error = access
            .session_from_user_token(&session.token)
            .await
            .expect_err("locked user session cannot be restored");
        assert!(matches!(
            locked_session_error,
            ApiError::Unauthorized(message) if message == "登录已过期，请重新登录"
        ));
    }
    /// 验证只允许邮箱注册时非法邮箱会被拒绝。
    #[tokio::test]
    async fn access_repository_rejects_invalid_user_email_register_when_username_only_disabled() {
        let access = AccessRepository::memory_seeded();

        let _ = access
            .update_registration(RegistrationConfig {
                username_enabled: false,
                email_enabled: true,
                agent_invite_required: false,
            })
            .await
            .expect("register policy can be updated");

        let error = access
            .register_user(UserRegisterRequest {
                username: Some("still_forbidden".to_string()),
                email: None,
                contact_qq: Some("5566778".to_string()),
                password: "forbidPassword123".to_string(),
                invite_code: None,
                registration_location: None,
            })
            .await
            .expect_err("username register should be rejected");

        assert!(matches!(error, ApiError::BadRequest(_)));
    }
    /// 验证用户可以通过旧密码修改密码。
    #[tokio::test]
    async fn access_repository_supports_user_password_change() {
        let access = AccessRepository::memory_seeded();

        access
            .change_password(
                "U10001",
                UserChangePasswordRequest {
                    old_password: "12345678".to_string(),
                    new_password: "newpass123".to_string(),
                },
            )
            .await
            .expect("password can be updated");

        assert!(access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "newpass123".to_string(),
            })
            .await
            .is_ok());
    }
    /// 验证管理员可以重置用户密码。
    #[tokio::test]
    async fn access_repository_supports_admin_reset_user_password() {
        let access = AccessRepository::memory_seeded();

        access
            .reset_user_password(
                "U10001",
                UserPasswordResetRequest {
                    password: "manualPass123".to_string(),
                },
            )
            .await
            .expect("admin can reset user password");

        assert!(access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "manualPass123".to_string(),
            })
            .await
            .is_ok());
        assert!(access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "12345678".to_string(),
            })
            .await
            .is_err());
    }
    /// 验证忘记密码和重置密码完整流程。
    #[tokio::test]
    async fn access_repository_forget_and_reset_password_flow() {
        let access = AccessRepository::memory_seeded();

        let forgot = access
            .request_forgot_password(UserForgotPasswordRequest {
                login_key: "demo_user".to_string(),
            })
            .await
            .expect("forgot password can be requested");

        assert!(!forgot.reset_token.trim().is_empty());

        let reset = access
            .reset_password(UserResetPasswordRequest {
                reset_token: forgot.reset_token.clone(),
                new_password: "resetPass123".to_string(),
            })
            .await
            .expect("reset token should be accepted");

        assert!(reset.reset);

        assert!(access
            .login_user(UserLoginRequest {
                login_key: "demo_user".to_string(),
                password: "resetPass123".to_string(),
            })
            .await
            .is_ok());
    }
    /// 验证用户提现方式的新增、默认和删除流程。
    #[tokio::test]
    async fn access_repository_manage_user_withdrawal_methods() {
        let access = AccessRepository::memory_seeded();

        assert_eq!(
            access
                .list_withdrawal_methods("U10001")
                .await
                .expect("can read user withdrawal methods")
                .len(),
            0
        );

        let wechat = access
            .create_withdrawal_method(
                "U10001",
                WithdrawalMethodRequest {
                    method_type: WithdrawalMethodType::Wechat,
                    account_holder: "收款人".to_string(),
                    account_number: "wechat_id_01".to_string(),
                    bank_name: None,
                    is_default: true,
                },
            )
            .await
            .expect("wechat withdrawal method created");

        let alipay = access
            .create_withdrawal_method(
                "U10001",
                WithdrawalMethodRequest {
                    method_type: WithdrawalMethodType::Alipay,
                    account_holder: "收款人".to_string(),
                    account_number: "alipay_01".to_string(),
                    bank_name: None,
                    is_default: false,
                },
            )
            .await
            .expect("alipay withdrawal method created");

        let methods = access
            .list_withdrawal_methods("U10001")
            .await
            .expect("can list withdrawal methods");
        assert_eq!(methods.len(), 2);
        assert!(methods
            .iter()
            .find(|method| method.id == wechat.id)
            .is_some_and(|m| m.is_default));

        let updated = access
            .update_withdrawal_method(
                "U10001",
                &alipay.id,
                WithdrawalMethodRequest {
                    method_type: WithdrawalMethodType::BankCard,
                    account_holder: "收款人".to_string(),
                    account_number: "6227001234567890".to_string(),
                    bank_name: Some("招商银行".to_string()),
                    is_default: true,
                },
            )
            .await
            .expect("can set bank card as default method");

        assert!(updated.is_default);
        assert_eq!(updated.bank_name.as_deref(), Some("招商银行"));

        let methods = access
            .list_withdrawal_methods("U10001")
            .await
            .expect("list updated methods");
        let default_count = methods.iter().filter(|method| method.is_default).count();
        assert_eq!(default_count, 1);

        access
            .delete_withdrawal_method("U10001", &wechat.id)
            .await
            .expect("withdrawal method can be deleted");

        let methods = access
            .list_withdrawal_methods("U10001")
            .await
            .expect("final methods list");
        assert_eq!(methods.len(), 1);
    }
    /// 验证管理员密码可以被重置。
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
    /// 验证过短管理员密码会被拒绝。
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
    /// 验证新增管理员必须提供初始密码。
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
    /// 验证登出后 token 立即失效。
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
