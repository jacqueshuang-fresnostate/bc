//! 统一 API 错误类型与响应码映射，提供中文错误文案

use axum::{http::StatusCode, response::IntoResponse, Json};
use thiserror::Error;

use crate::response::ApiEnvelope;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Error)]
#[allow(dead_code)]
/// 统一 API 错误类型，所有路由和服务层错误都转换为该枚举。
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

/// API 错误的状态码和日志展示方法。
impl ApiError {
    /// 根据错误类型映射 HTTP 状态码。
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
            Self::BadRequest(message) => format!("请求错误：{message}"),
            Self::Unauthorized(message) => format!("未授权：{message}"),
            Self::Forbidden(message) => format!("权限不足：{message}"),
            Self::NotFound(message) => format!("资源不存在：{message}"),
            Self::Conflict(message) => format!("资源冲突：{message}"),
            Self::Internal(message) => format!("内部错误：{message}"),
        }
    }
}

/// 把 API 错误转换为统一响应信封。
impl IntoResponse for ApiError {
    /// 生成 Axum 响应，并在服务端错误时记录中文日志。
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

#[cfg(test)]
mod tests {
    use super::ApiError;

    #[test]
    /// 验证接口错误日志前缀保持中文，避免运维排障看到模板英文。
    fn api_error_log_message_uses_chinese_prefixes() {
        let message = ApiError::BadRequest("邀请码无效".to_string()).log_message();

        assert_eq!(message, "请求错误：邀请码无效");
    }

    #[test]
    /// 错误日志保留原始错误详情，便于定位第三方接口、数据库和运行时问题。
    fn api_error_log_message_keeps_original_error_detail() {
        let message =
            ApiError::Internal("api draw source request failed".to_string()).log_message();

        assert_eq!(message, "内部错误：api draw source request failed");
    }
}
