//! Redis 运行时工具，提供可选连接池、分布式锁和缓存失效能力

use std::{env::VarError, error::Error, io, time::Duration};

use rand_core::{OsRng, RngCore};
use redis::{aio::ConnectionManager, AsyncCommands};

use crate::error::{ApiError, ApiResult};

const DEFAULT_LOCK_TTL_MS: u64 = 15_000;

#[derive(Clone)]
/// Redis 运行时封装；未配置 Redis 时保持禁用，方便本地内存模式和测试继续运行。
pub struct RedisRuntime {
    manager: Option<ConnectionManager>,
    lock_ttl: Duration,
}

/// Redis 分布式锁持有对象，调用方完成关键区后需要主动释放。
pub struct RedisLockGuard {
    manager: ConnectionManager,
    key: String,
    token: String,
}

/// Redis 运行时构造和基础操作。
impl RedisRuntime {
    /// 构建禁用状态的 Redis 运行时，用于未配置 `REDIS_URL` 的环境。
    pub fn disabled() -> Self {
        Self {
            manager: None,
            lock_ttl: Duration::from_millis(DEFAULT_LOCK_TTL_MS),
        }
    }

    /// 从环境变量读取 Redis 配置；配置为空时禁用，配置错误或连接失败时阻止启动。
    pub async fn from_env() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let Some(redis_url) = redis_url_from_env()? else {
            tracing::info!("未配置 REDIS_URL，Redis 缓存和分布式锁保持禁用");
            return Ok(Self::disabled());
        };
        let client = redis::Client::open(redis_url.as_str())?;
        let manager = client.get_connection_manager().await?;
        let lock_ttl = Duration::from_millis(redis_lock_ttl_ms_from_env()?);
        tracing::info!(
            lock_ttl_ms = lock_ttl.as_millis(),
            "已连接 Redis，启用缓存和分布式锁能力"
        );
        Ok(Self {
            manager: Some(manager),
            lock_ttl,
        })
    }

    /// 尝试获取分布式锁；Redis 未启用时返回 `None`，让单进程本地环境保持原行为。
    pub async fn acquire_lock(&self, key: impl Into<String>) -> ApiResult<Option<RedisLockGuard>> {
        let Some(manager) = &self.manager else {
            return Ok(None);
        };
        let key = key.into();
        let token = random_lock_token();
        let mut connection = manager.clone();
        let ttl_ms = usize::try_from(self.lock_ttl.as_millis())
            .map_err(|_| ApiError::Internal("Redis 锁过期时间过大".to_string()))?;
        let result: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(&token)
            .arg("NX")
            .arg("PX")
            .arg(ttl_ms)
            .query_async(&mut connection)
            .await
            .map_err(|error| {
                tracing::error!(%error, lock_key = key.as_str(), "Redis 分布式锁获取失败");
                ApiError::Internal("Redis 分布式锁获取失败".to_string())
            })?;

        if result.as_deref() != Some("OK") {
            return Err(ApiError::Conflict(
                "当前账号有投注正在处理中，请稍后再试".to_string(),
            ));
        }

        Ok(Some(RedisLockGuard {
            manager: manager.clone(),
            key,
            token,
        }))
    }

    /// 删除一批缓存键；Redis 未启用时静默跳过，缓存删除失败不影响主事务提交结果。
    pub async fn delete_keys(&self, keys: &[String]) -> ApiResult<()> {
        let Some(manager) = &self.manager else {
            return Ok(());
        };
        if keys.is_empty() {
            return Ok(());
        }
        let mut connection = manager.clone();
        let _: i64 = connection.del(keys).await.map_err(|error| {
            tracing::error!(%error, "Redis 缓存删除失败");
            ApiError::Internal("Redis 缓存删除失败".to_string())
        })?;
        Ok(())
    }
}

/// Redis 锁释放逻辑，使用 Lua 保证只释放自己持有的锁。
impl RedisLockGuard {
    /// 主动释放分布式锁；如果锁已过期或被替换，返回 false。
    pub async fn release(self) -> ApiResult<bool> {
        let mut connection = self.manager.clone();
        let released: i64 = redis::Script::new(
            "if redis.call('get', KEYS[1]) == ARGV[1] then return redis.call('del', KEYS[1]) else return 0 end",
        )
        .key(&self.key)
        .arg(&self.token)
        .invoke_async(&mut connection)
        .await
        .map_err(|error| {
            tracing::error!(%error, lock_key = self.key.as_str(), "Redis 分布式锁释放失败");
            ApiError::Internal("Redis 分布式锁释放失败".to_string())
        })?;
        Ok(released > 0)
    }
}

/// 读取并校验 Redis 连接串。
fn redis_url_from_env() -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    match std::env::var("REDIS_URL") {
        Ok(value) => normalize_redis_url_value(&value)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            "REDIS_URL 配置无效：必须是有效 UTF-8 文本",
        ))),
    }
}

/// 读取 Redis 锁默认过期时间，避免锁持有方异常退出后永久阻塞投注。
fn redis_lock_ttl_ms_from_env() -> Result<u64, io::Error> {
    match std::env::var("REDIS_LOCK_TTL_MS") {
        Ok(value) => {
            let ttl = value.trim().parse::<u64>().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "REDIS_LOCK_TTL_MS 配置无效：必须是正整数毫秒",
                )
            })?;
            if ttl == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "REDIS_LOCK_TTL_MS 配置无效：必须大于 0",
                ));
            }
            Ok(ttl)
        }
        Err(VarError::NotPresent) => Ok(DEFAULT_LOCK_TTL_MS),
        Err(VarError::NotUnicode(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "REDIS_LOCK_TTL_MS 配置无效：必须是有效 UTF-8 文本",
        )),
    }
}

/// 校验 Redis 连接串格式，避免启动后才暴露难懂的连接错误。
fn normalize_redis_url_value(value: &str) -> Result<Option<String>, io::Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if !trimmed.starts_with("redis://") && !trimmed.starts_with("rediss://") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "REDIS_URL 配置无效：必须以 redis:// 或 rediss:// 开头",
        ));
    }
    Ok(Some(trimmed.to_string()))
}

/// 生成锁 token，释放时通过 token 防止误删其它请求重新获得的锁。
fn random_lock_token() -> String {
    let high = OsRng.next_u64();
    let low = OsRng.next_u64();
    format!("{high:016x}{low:016x}")
}

#[cfg(test)]
mod tests {
    use super::normalize_redis_url_value;

    #[test]
    /// 验证空 Redis 连接串等同于未配置。
    fn redis_url_allows_empty_value_as_unconfigured() {
        assert_eq!(normalize_redis_url_value("").unwrap(), None);
        assert_eq!(normalize_redis_url_value("  ").unwrap(), None);
    }

    #[test]
    /// 验证 Redis 连接串需要显式协议。
    fn redis_url_rejects_invalid_scheme() {
        let error = normalize_redis_url_value("127.0.0.1:6379").expect_err("缺少协议必须失败");
        assert!(error.to_string().contains("REDIS_URL 配置无效"));
    }
}
