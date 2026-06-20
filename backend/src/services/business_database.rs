//! 业务数据库封装，负责连接池与迁移执行

use std::{error::Error, io};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{
    migrate::{MigrateError, Migrator},
    postgres::PgPoolOptions,
    PgPool,
};

use crate::error::{ApiError, ApiResult};

const BUSINESS_TABLES_MIGRATION_VERSION: i64 = 20260603152000;

#[derive(Clone)]
/// 业务数据库封装，保存 PostgreSQL 连接池并统一执行迁移。
pub struct BusinessDatabase {
    pool: PgPool,
}

/// 业务数据库连接池的构造和访问方法。
impl BusinessDatabase {
    /// 基于连接字符串创建数据库连接池并执行迁移。
    pub async fn postgres(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        run_business_migrations(&pool).await?;

        Ok(Self { pool })
    }

    /// 返回数据库连接池引用。
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// 统一执行后端业务迁移，并对已知历史迁移校验冲突做受控修复。
pub(crate) async fn run_business_migrations(
    pool: &PgPool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let migrator = sqlx::migrate!("./migrations");
    match migrator.run(pool).await {
        Ok(()) => Ok(()),
        Err(MigrateError::VersionMismatch(version))
            if version == BUSINESS_TABLES_MIGRATION_VERSION =>
        {
            repair_business_tables_migration_checksum(pool, &migrator).await?;
            migrator.run(pool).await?;
            Ok(())
        }
        Err(error) => Err(Box::new(error)),
    }
}

/// 修复曾被误改的业务基础表迁移 checksum，保证旧库和中间版本库都能继续前向迁移。
async fn repair_business_tables_migration_checksum(
    pool: &PgPool,
    migrator: &Migrator,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let current_checksum = current_migration_checksum(migrator, BUSINESS_TABLES_MIGRATION_VERSION)?;
    let Some(database_checksum) =
        applied_migration_checksum(pool, BUSINESS_TABLES_MIGRATION_VERSION).await?
    else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "无法修复 SQLx 迁移记录：数据库中缺少业务基础表迁移记录",
        )
        .into());
    };

    let admin_roles_exists = admin_roles_table_exists(pool).await?;
    if !admin_roles_exists {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "无法修复 SQLx 迁移记录：admin_roles 表不存在",
        )
        .into());
    }

    let permissions_exists = admin_roles_permissions_column_exists(pool).await?;
    tracing::warn!(
        "检测到历史迁移校验不一致，准备修复 SQLx 迁移记录：版本={}，数据库校验={}，当前校验={}，角色权限字段已存在={}",
        BUSINESS_TABLES_MIGRATION_VERSION,
        short_checksum_hex(&database_checksum),
        short_checksum_hex(current_checksum),
        permissions_exists
    );

    let updated = sqlx::query(
        r#"
        UPDATE _sqlx_migrations
        SET checksum = $1
        WHERE version = $2 AND success = true
        "#,
    )
    .bind(current_checksum)
    .bind(BUSINESS_TABLES_MIGRATION_VERSION)
    .execute(pool)
    .await?
    .rows_affected();

    if updated != 1 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "SQLx 迁移记录修复失败：更新行数不符合预期",
        )
        .into());
    }

    tracing::warn!(
        "已修复 SQLx 历史迁移记录：版本={}，后续权限字段仍由前向迁移创建或确认存在",
        BUSINESS_TABLES_MIGRATION_VERSION
    );
    Ok(())
}

/// 从当前代码内嵌的 migration 集合中读取指定版本的 checksum。
fn current_migration_checksum<'a>(
    migrator: &'a Migrator,
    version: i64,
) -> Result<&'a [u8], Box<dyn Error + Send + Sync>> {
    migrator
        .iter()
        .find(|migration| migration.version == version)
        .map(|migration| migration.checksum.as_ref())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "无法修复 SQLx 迁移记录：当前代码缺少目标迁移版本",
            )
            .into()
        })
}

/// 读取数据库里已经成功应用的指定 migration checksum。
async fn applied_migration_checksum(
    pool: &PgPool,
    version: i64,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT checksum
        FROM _sqlx_migrations
        WHERE version = $1 AND success = true
        "#,
    )
    .bind(version)
    .fetch_optional(pool)
    .await
}

/// 确认基础业务角色表存在，避免在未知数据库结构上改写迁移记录。
async fn admin_roles_table_exists(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = current_schema()
              AND table_name = 'admin_roles'
        )
        "#,
    )
    .fetch_one(pool)
    .await
}

/// 检查细粒度权限字段是否已经存在，用于中文启动日志说明当前库处于哪种历史状态。
async fn admin_roles_permissions_column_exists(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = current_schema()
              AND table_name = 'admin_roles'
              AND column_name = 'permissions'
        )
        "#,
    )
    .fetch_one(pool)
    .await
}

/// 把较长的 checksum 缩短成中文日志里便于人工比对的十六进制前缀。
fn short_checksum_hex(checksum: &[u8]) -> String {
    checksum
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

/// 把业务结构序列化为 JSON 值，供 JSONB 字段保存。
pub(crate) fn to_json<T>(value: &T) -> ApiResult<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|_| ApiError::Internal("业务数据序列化失败".to_string()))
}

/// 把 JSONB 字段反序列化为业务结构。
pub(crate) fn from_json<T>(value: Value) -> ApiResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|_| ApiError::Internal("业务数据反序列化失败".to_string()))
}

/// 把 serde 枚举值转换为数据库保存的 camelCase 字符串。
pub(crate) fn enum_to_string<T>(value: &T) -> ApiResult<String>
where
    T: Serialize,
{
    match to_json(value)? {
        Value::String(value) => Ok(value),
        _ => Err(ApiError::Internal("业务枚举序列化失败".to_string())),
    }
}

/// 把数据库中的 camelCase 字符串恢复为 serde 枚举值。
pub(crate) fn enum_from_string<T>(value: String) -> ApiResult<T>
where
    T: DeserializeOwned,
{
    from_json(Value::String(value))
}
