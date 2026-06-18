//! 统一 API 响应封装结构，确保前后端返回格式一致

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 统一 API 响应信封，成功和失败都保持同一 JSON 外层结构。
pub struct ApiEnvelope<T>
where
    T: Serialize,
{
    /// 请求是否处理成功。
    pub success: bool,
    /// 成功时携带的业务数据；失败时为空。
    pub data: Option<T>,
    /// 给前端展示或调试使用的中文消息。
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

/// 错误响应信封构造方法。
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
