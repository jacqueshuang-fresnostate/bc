use std::error::Error;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct StateDocumentRepository {
    pool: PgPool,
}

impl StateDocumentRepository {
    pub async fn postgres(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn load_or_seed<T>(&self, namespace: &str, seed: T) -> ApiResult<T>
    where
        T: DeserializeOwned + Serialize + Clone,
    {
        let namespace = normalize_namespace(namespace)?;
        let payload: Option<Value> =
            sqlx::query_scalar("SELECT payload FROM state_documents WHERE namespace = $1")
                .bind(&namespace)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| ApiError::Internal("业务状态读取失败".to_string()))?;

        let Some(payload) = payload else {
            self.save(&namespace, &seed).await?;
            return Ok(seed);
        };

        serde_json::from_value(payload)
            .map_err(|_| ApiError::Internal("业务状态反序列化失败".to_string()))
    }

    pub async fn save<T>(&self, namespace: &str, payload: &T) -> ApiResult<()>
    where
        T: Serialize,
    {
        let namespace = normalize_namespace(namespace)?;
        let payload = serde_json::to_value(payload)
            .map_err(|_| ApiError::Internal("业务状态序列化失败".to_string()))?;

        sqlx::query(
            "INSERT INTO state_documents (namespace, payload)
             VALUES ($1, $2)
             ON CONFLICT (namespace)
             DO UPDATE SET payload = EXCLUDED.payload, updated_at = now()",
        )
        .bind(namespace)
        .bind(payload)
        .execute(&self.pool)
        .await
        .map_err(|_| ApiError::Internal("业务状态保存失败".to_string()))?;

        Ok(())
    }
}

fn normalize_namespace(namespace: &str) -> ApiResult<String> {
    let namespace = namespace.trim();
    if namespace.is_empty() {
        return Err(ApiError::BadRequest("业务状态命名空间不能为空".to_string()));
    }

    Ok(namespace.to_string())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct TestDocument {
        value: String,
        count: u32,
    }

    #[tokio::test]
    async fn state_document_repository_seeds_saves_and_restores() {
        let Ok(database_url) = std::env::var("BC_TEST_DATABASE_URL") else {
            return;
        };
        let repository = StateDocumentRepository::postgres(&database_url)
            .await
            .expect("测试数据库可以连接");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("系统时间有效")
            .as_nanos();
        let namespace = format!("test-state-document-{timestamp}");

        let seeded = repository
            .load_or_seed(
                &namespace,
                TestDocument {
                    value: "seed".to_string(),
                    count: 1,
                },
            )
            .await
            .expect("空状态可以写入种子");
        assert_eq!(seeded.value, "seed");

        repository
            .save(
                &namespace,
                &TestDocument {
                    value: "saved".to_string(),
                    count: 2,
                },
            )
            .await
            .expect("状态可以保存");

        let restored = repository
            .load_or_seed(
                &namespace,
                TestDocument {
                    value: "ignored".to_string(),
                    count: 99,
                },
            )
            .await
            .expect("已有状态可以恢复");
        assert_eq!(
            restored,
            TestDocument {
                value: "saved".to_string(),
                count: 2
            }
        );
    }
}
