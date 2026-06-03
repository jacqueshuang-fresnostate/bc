//! 业务数据库封装，负责连接池与迁移执行

use std::error::Error;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct BusinessDatabase {
    pool: PgPool,
}

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

pub(crate) fn to_json<T>(value: &T) -> ApiResult<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(|_| ApiError::Internal("业务数据序列化失败".to_string()))
}

pub(crate) fn from_json<T>(value: Value) -> ApiResult<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(value)
        .map_err(|_| ApiError::Internal("业务数据反序列化失败".to_string()))
}

pub(crate) fn enum_to_string<T>(value: &T) -> ApiResult<String>
where
    T: Serialize,
{
    match to_json(value)? {
        Value::String(value) => Ok(value),
        _ => Err(ApiError::Internal("业务枚举序列化失败".to_string())),
    }
}

pub(crate) fn enum_from_string<T>(value: String) -> ApiResult<T>
where
    T: DeserializeOwned,
{
    from_json(Value::String(value))
}
