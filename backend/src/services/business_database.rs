//! 业务数据库封装，负责连接池与迁移执行

use std::error::Error;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::error::{ApiError, ApiResult};

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

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    /// 返回数据库连接池引用。
    pub fn pool(&self) -> &PgPool {
        &self.pool
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
