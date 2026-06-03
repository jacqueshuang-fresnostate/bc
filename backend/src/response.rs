//! 统一 API 响应封装结构，确保前后端返回格式一致

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiEnvelope<T>
where
    T: Serialize,
{
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
}

impl<T> ApiEnvelope<T>
where
    T: Serialize,
{
    /// 构造统一成功响应体并返回成功结果。
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: "ok".to_string(),
        }
    }
}

impl ApiEnvelope<()> {
    /// 构造统一错误响应体并返回错误说明。
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: message.into(),
        }
    }
}
