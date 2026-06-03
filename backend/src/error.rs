//! 统一 API 错误类型与响应码映射，提供中文错误文案

use axum::{http::StatusCode, response::IntoResponse, Json};
use thiserror::Error;

use crate::response::ApiEnvelope;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ApiError {
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("unauthorized: {0}")]
    Unauthorized(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl ApiError {
    /// 处理 status_code 的具体内部流程。
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// 返回当前错误对象的中文日志消息文本，用于日志展示和错误提示。
    pub fn log_message(&self) -> String {
        match self {
            Self::BadRequest(message) => format!("请求错误：{}", log_detail(message)),
            Self::Unauthorized(message) => format!("未授权：{}", log_detail(message)),
            Self::Forbidden(message) => format!("权限不足：{}", log_detail(message)),
            Self::NotFound(message) => format!("资源不存在：{}", log_detail(message)),
            Self::Conflict(message) => format!("资源冲突：{}", log_detail(message)),
            Self::Internal(message) => format!("内部错误：{}", log_detail(message)),
        }
    }
}

impl IntoResponse for ApiError {
    /// 处理 into_response 的具体内部流程。
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let log_message = self.log_message();
        let message = self.to_string();

        if status.is_server_error() {
            tracing::error!(message = %log_message, "接口错误");
        }

        (status, Json(ApiEnvelope::error(message))).into_response()
    }
}

/// 处理 log_detail 的具体内部流程。
fn log_detail(message: &str) -> &str {
    if message
        .chars()
        .any(|character| character.is_ascii_alphabetic())
    {
        "错误详情已按中文日志规则隐藏"
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use super::ApiError;

    #[test]
    /// 处理 api_error_log_message_uses_chinese_prefixes 的具体内部流程。
    fn api_error_log_message_uses_chinese_prefixes() {
        let message = ApiError::BadRequest("邀请码无效".to_string()).log_message();

        assert_eq!(message, "请求错误：邀请码无效");
    }

    #[test]
    /// 处理 api_error_log_message_hides_english_internal_details 的具体内部流程。
    fn api_error_log_message_hides_english_internal_details() {
        let message =
            ApiError::Internal("api draw source request failed".to_string()).log_message();

        assert_eq!(message, "内部错误：错误详情已按中文日志规则隐藏");
        assert!(!message.contains("api draw source request failed"));
    }
}
