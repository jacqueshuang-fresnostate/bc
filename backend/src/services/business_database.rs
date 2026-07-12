//! 业务数据库封装，负责连接池与迁移执行

use std::{error::Error, io, time::Duration};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{
    migrate::{MigrateError, Migrator},
    postgres::PgPoolOptions,
    PgPool,
};

use crate::error::{ApiError, ApiResult};

const BUSINESS_TABLES_MIGRATION_VERSION: i64 = 20260603152000;
const COLUMN_COMMENTS_MIGRATION_VERSION: i64 = 20260603234000;
const WITHDRAWAL_TURNOVER_REPAIR_MIGRATION_VERSION: i64 = 20260704071000;
const WITHDRAWAL_TURNOVER_FULL_REPLAY_CHECKSUM: &str =
    "715752946e48d3ed8b4aba20cc68234575437028cc43c297e0b0cdd85a20f54b23d4b04cb89a6cc24ae34ba8ad730dcc";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistoricalMigrationValidation {
    AdminRoles,
    WithdrawalTurnover,
}

struct HistoricalMigrationRepair {
    version: i64,
    name: &'static str,
    reason: &'static str,
    allowed_legacy_checksums: &'static [&'static str],
    validation: HistoricalMigrationValidation,
}

impl HistoricalMigrationRepair {
    /// 只允许明确记录的旧 checksum；空白名单保留早期兼容项的既有行为。
    fn allows_database_checksum(&self, checksum: &[u8]) -> bool {
        self.allowed_legacy_checksums.is_empty()
            || self
                .allowed_legacy_checksums
                .contains(&checksum_hex(checksum).as_str())
    }
}

const HISTORICAL_MIGRATION_REPAIRS: [HistoricalMigrationRepair; 3] = [
    HistoricalMigrationRepair {
        version: BUSINESS_TABLES_MIGRATION_VERSION,
        name: "业务基础表",
        reason: "角色细粒度权限字段曾被写回已发布建表迁移",
        allowed_legacy_checksums: &[],
        validation: HistoricalMigrationValidation::AdminRoles,
    },
    HistoricalMigrationRepair {
        version: COLUMN_COMMENTS_MIGRATION_VERSION,
        name: "全量字段注释",
        reason: "早期字段注释迁移曾提前引用后续迁移才创建的字段",
        allowed_legacy_checksums: &[],
        validation: HistoricalMigrationValidation::AdminRoles,
    },
    HistoricalMigrationRepair {
        version: WITHDRAWAL_TURNOVER_REPAIR_MIGRATION_VERSION,
        name: "提现任务投注增量补偿",
        reason: "该迁移曾发布全量逐笔重放和轻量窗口补偿两个等价修复版本",
        allowed_legacy_checksums: &[WITHDRAWAL_TURNOVER_FULL_REPLAY_CHECKSUM],
        validation: HistoricalMigrationValidation::WithdrawalTurnover,
    },
];

#[derive(Clone)]
/// 业务数据库封装，保存 PostgreSQL 连接池并统一执行迁移。
pub struct BusinessDatabase {
    pool: PgPool,
}

/// 业务数据库连接池的构造和访问方法。
impl BusinessDatabase {
    /// 基于连接字符串创建数据库连接池并执行迁移。
    pub async fn postgres(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = configured_pg_pool_options().connect(database_url).await?;

        run_business_migrations(&pool).await?;

        Ok(Self { pool })
    }

    /// 返回数据库连接池引用。
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// 读取数据库连接池配置，避免并发请求被过小的默认连接池卡住。
pub(crate) fn configured_pg_pool_options() -> PgPoolOptions {
    let max_connections = env_u32("DATABASE_MAX_CONNECTIONS", 30).max(1);
    let min_connections = env_u32("DATABASE_MIN_CONNECTIONS", 2).min(max_connections);
    let acquire_timeout_seconds = env_u64("DATABASE_ACQUIRE_TIMEOUT_SECONDS", 10).max(1);

    PgPoolOptions::new()
        .max_connections(max_connections)
        .min_connections(min_connections)
        .acquire_timeout(Duration::from_secs(acquire_timeout_seconds))
}

/// 读取无符号整数环境变量；非法值回退默认值并输出中文告警。
fn env_u32(key: &str, default_value: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|value| match value.trim().parse::<u32>() {
            Ok(parsed) => Some(parsed),
            Err(error) => {
                tracing::warn!(
                    %error,
                    配置键 = key,
                    默认值 = default_value,
                    "数据库连接池整数配置无效，已使用默认值"
                );
                None
            }
        })
        .unwrap_or(default_value)
}

/// 读取无符号长整数环境变量；非法值回退默认值并输出中文告警。
fn env_u64(key: &str, default_value: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| match value.trim().parse::<u64>() {
            Ok(parsed) => Some(parsed),
            Err(error) => {
                tracing::warn!(
                    %error,
                    配置键 = key,
                    默认值 = default_value,
                    "数据库连接池时间配置无效，已使用默认值"
                );
                None
            }
        })
        .unwrap_or(default_value)
}

/// 统一执行后端业务迁移，并对已知历史迁移校验冲突做受控修复。
pub(crate) async fn run_business_migrations(
    pool: &PgPool,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let migrator = sqlx::migrate!("./migrations");
    let mut repaired_versions = Vec::new();

    loop {
        match migrator.run(pool).await {
            Ok(()) => return Ok(()),
            Err(MigrateError::VersionMismatch(version)) => {
                let Some(repair) = historical_migration_repair(version) else {
                    return Err(Box::new(MigrateError::VersionMismatch(version)));
                };

                if repaired_versions.contains(&version) {
                    return Err(io::Error::new(
                        io::ErrorKind::Other,
                        "SQLx 迁移记录修复后仍然校验失败，已停止自动重试",
                    )
                    .into());
                }

                repair_historical_migration_checksum(pool, &migrator, repair).await?;
                repaired_versions.push(version);
            }
            Err(error) => return Err(Box::new(error)),
        }
    }
}

/// 查找当前版本是否属于允许自动修复的已知历史迁移。
fn historical_migration_repair(version: i64) -> Option<&'static HistoricalMigrationRepair> {
    HISTORICAL_MIGRATION_REPAIRS
        .iter()
        .find(|repair| repair.version == version)
}

/// 修复曾被误改的历史迁移 checksum，保证旧库和中间版本库都能继续前向迁移。
async fn repair_historical_migration_checksum(
    pool: &PgPool,
    migrator: &Migrator,
    repair: &'static HistoricalMigrationRepair,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let current_checksum = current_migration_checksum(migrator, repair.version)?;
    let Some(database_checksum) = applied_migration_checksum(pool, repair.version).await? else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "无法修复 SQLx 迁移记录：数据库中缺少目标迁移记录",
        )
        .into());
    };
    if !repair.allows_database_checksum(&database_checksum) {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "拒绝修复未知 SQLx 迁移校验值：版本={}，数据库校验={}",
                repair.version,
                short_checksum_hex(&database_checksum)
            ),
        )
        .into());
    }

    let validation_ready = match repair.validation {
        HistoricalMigrationValidation::AdminRoles => admin_roles_table_exists(pool).await?,
        HistoricalMigrationValidation::WithdrawalTurnover => {
            withdrawal_turnover_migration_state_exists(pool).await?
        }
    };
    if !validation_ready {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "无法修复 SQLx 迁移记录：版本={} 的业务结构校验未通过",
                repair.version
            ),
        )
        .into());
    }

    let validation_detail = match repair.validation {
        HistoricalMigrationValidation::AdminRoles => format!(
            "角色权限字段已存在={}",
            admin_roles_permissions_column_exists(pool).await?
        ),
        HistoricalMigrationValidation::WithdrawalTurnover => {
            "提现任务表、事件表和兼容函数均已存在".to_string()
        }
    };
    tracing::warn!(
        "检测到已知历史迁移校验不一致，准备修复 SQLx 迁移记录：版本={}，迁移名称={}，修复原因={}，数据库校验={}，当前校验={}，结构校验={}",
        repair.version,
        repair.name,
        repair.reason,
        short_checksum_hex(&database_checksum),
        short_checksum_hex(current_checksum),
        validation_detail
    );

    let updated = sqlx::query(
        r#"
        UPDATE _sqlx_migrations
        SET checksum = $1
        WHERE version = $2 AND success = true
        "#,
    )
    .bind(current_checksum)
    .bind(repair.version)
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
        "已修复 SQLx 历史迁移记录：版本={}，迁移名称={}，后续迁移将继续按顺序执行",
        repair.version,
        repair.name
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

/// 检查提现任务历史迁移的核心结构，避免只凭 checksum 改写未知数据库。
async fn withdrawal_turnover_migration_state_exists(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT to_regclass('user_withdrawal_turnovers') IS NOT NULL
           AND to_regclass('user_withdrawal_turnover_events') IS NOT NULL
           AND to_regprocedure('apply_user_withdrawal_turnover_from_ledger()') IS NOT NULL
        "#,
    )
    .fetch_one(pool)
    .await
}

/// 把 SQLx SHA-384 checksum 转成完整十六进制文本，用于已知历史版本白名单比对。
fn checksum_hex(checksum: &[u8]) -> String {
    checksum
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
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

#[cfg(test)]
mod tests {
    use super::{
        checksum_hex, historical_migration_repair, HistoricalMigrationValidation,
        WITHDRAWAL_TURNOVER_FULL_REPLAY_CHECKSUM, WITHDRAWAL_TURNOVER_REPAIR_MIGRATION_VERSION,
    };

    #[test]
    fn withdrawal_turnover_repair_only_accepts_known_full_replay_checksum() {
        let repair = historical_migration_repair(WITHDRAWAL_TURNOVER_REPAIR_MIGRATION_VERSION)
            .expect("withdrawal turnover repair must be registered");

        assert_eq!(
            repair.validation,
            HistoricalMigrationValidation::WithdrawalTurnover
        );
        assert_eq!(
            repair.allowed_legacy_checksums,
            &[WITHDRAWAL_TURNOVER_FULL_REPLAY_CHECKSUM]
        );
        assert!(repair
            .allows_database_checksum(&checksum_bytes(WITHDRAWAL_TURNOVER_FULL_REPLAY_CHECKSUM)));
        assert!(!repair.allows_database_checksum(&[0_u8; 48]));
    }

    #[test]
    fn checksum_hex_keeps_full_byte_sequence() {
        assert_eq!(checksum_hex(&[0x00, 0x1a, 0xff]), "001aff");
    }

    fn checksum_bytes(value: &str) -> Vec<u8> {
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("checksum hex must be utf-8");
                u8::from_str_radix(pair, 16).expect("checksum hex must contain valid bytes")
            })
            .collect()
    }
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
