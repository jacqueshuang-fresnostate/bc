//! 图床上传服务，统一读取系统图床配置并代理 multipart 文件上传。

use axum::extract::Multipart;
use serde_json::Value;

use crate::{
    error::{ApiError, ApiResult},
    services::access::AccessRepository,
};

/// 系统设置中保存图床上传接口地址的键名。
pub const IMAGE_BED_UPLOAD_URL_SETTING: &str = "image_bed_upload_url";
/// 系统设置中保存图床 Bearer Token 的键名。
pub const IMAGE_BED_AUTHORIZATION_TOKEN_SETTING: &str = "image_bed_authorization_token";
/// 系统设置中保存 multipart 文件字段名的键名。
pub const IMAGE_BED_UPLOAD_FIELD_SETTING: &str = "image_bed_upload_field";
/// 图床默认 multipart 文件字段名。
pub const IMAGE_BED_UPLOAD_FIELD_DEFAULT: &str = "file";
/// 系统设置中保存图床返回图片链接字段路径的键名。
pub const IMAGE_BED_RESULT_URL_FIELD_SETTING: &str = "image_bed_result_url_field";
/// 图床默认返回图片链接字段路径。
pub const IMAGE_BED_RESULT_URL_FIELD_DEFAULT: &str = "links.download";

#[derive(Debug, Clone, Copy)]
/// 图床上传行为选项，用于区分后台通用图片上传和用户头像上传。
pub struct ImageBedUploadOptions {
    /// 是否只允许图片 MIME 类型，头像上传需要开启。
    pub image_only: bool,
    /// multipart 中找不到文件字段时返回的中文错误。
    pub missing_file_message: &'static str,
    /// 上游未提供文件名时使用的默认文件名。
    pub default_file_name: &'static str,
}

impl Default for ImageBedUploadOptions {
    /// 返回默认值。
    fn default() -> Self {
        Self {
            image_only: false,
            missing_file_message: "未检测到图片文件字段",
            default_file_name: "upload.bin",
        }
    }
}

/// 按系统设置把 multipart 文件透传到图床，并按配置字段返回图片链接值。
pub async fn upload_configured_image_bed_file(
    access: &AccessRepository,
    mut payload: Multipart,
    options: ImageBedUploadOptions,
) -> ApiResult<Value> {
    let upload_url = access
        .setting_value(IMAGE_BED_UPLOAD_URL_SETTING)
        .await?
        .trim()
        .to_string();
    if upload_url.is_empty() {
        return Err(ApiError::BadRequest("图床上传接口地址未配置".to_string()));
    }

    let authorization_token = access
        .setting_value(IMAGE_BED_AUTHORIZATION_TOKEN_SETTING)
        .await?
        .trim()
        .to_string();
    if authorization_token.is_empty() {
        return Err(ApiError::BadRequest("图床上传 Token 未配置".to_string()));
    }

    let upload_field = image_bed_setting_or_default(
        access,
        IMAGE_BED_UPLOAD_FIELD_SETTING,
        IMAGE_BED_UPLOAD_FIELD_DEFAULT,
    )
    .await?;

    let mut upload_part = None;
    while let Some(field) = payload
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest("上传内容解析失败".to_string()))?
    {
        let field_name = field.name().unwrap_or_default().to_string();
        if field_name != upload_field && field_name != IMAGE_BED_UPLOAD_FIELD_DEFAULT {
            continue;
        }

        let file_name = field
            .file_name()
            .unwrap_or(options.default_file_name)
            .to_string();
        let content_type: Option<String> =
            field.content_type().map(std::string::ToString::to_string);
        if options.image_only {
            if let Some(content_type) = &content_type {
                if !content_type.starts_with("image/") {
                    return Err(ApiError::BadRequest("上传文件必须是图片类型".to_string()));
                }
            }
        }
        let bytes = field
            .bytes()
            .await
            .map_err(|_| ApiError::BadRequest("读取上传文件内容失败".to_string()))?
            .to_vec();

        let mut part = reqwest::multipart::Part::bytes(bytes).file_name(file_name);
        if let Some(content_type) = content_type {
            part = part
                .mime_str(&content_type)
                .map_err(|_| ApiError::BadRequest("文件类型格式异常".to_string()))?;
        }

        upload_part = Some(part);
        break;
    }

    let Some(part) = upload_part else {
        return Err(ApiError::BadRequest(
            options.missing_file_message.to_string(),
        ));
    };

    let form = reqwest::multipart::Form::new().part(upload_field, part);
    let response = reqwest::Client::new()
        .post(upload_url)
        .header("Authorization", format!("Bearer {authorization_token}"))
        .multipart(form)
        .send()
        .await
        .map_err(|_| ApiError::Internal("图床请求发送失败".to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let message = response
            .text()
            .await
            .map_err(|_| ApiError::Internal("图床响应读取失败".to_string()))?;
        return Err(ApiError::Internal(format!(
            "图床服务返回失败：HTTP {status}，响应内容 {message}"
        )));
    }

    let response_body = response
        .text()
        .await
        .map_err(|_| ApiError::Internal("图床响应读取失败".to_string()))?;
    let response_json = serde_json::from_str::<Value>(&response_body)
        .unwrap_or_else(|_| Value::String(response_body));
    let result_url_field = access
        .setting_value_optional(IMAGE_BED_RESULT_URL_FIELD_SETTING)
        .await?
        .unwrap_or_else(|| IMAGE_BED_RESULT_URL_FIELD_DEFAULT.to_string())
        .trim()
        .to_string();

    if result_url_field.is_empty() {
        Ok(response_json)
    } else {
        extract_image_bed_result_field(&response_json, &result_url_field)
    }
}

/// 把图床上传结果转换为可写入业务字段的图片链接文本。
pub fn image_bed_value_as_url(value: &Value, label: &str) -> ApiResult<String> {
    let Some(url) = value.as_str().map(str::trim).filter(|url| !url.is_empty()) else {
        return Err(ApiError::BadRequest(format!(
            "{label}不是有效图片链接文本：{value}"
        )));
    };
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(ApiError::BadRequest(format!("{label}格式不正确")));
    }

    Ok(url.to_string())
}

/// 读取图床配置值，空值会回退到默认值，避免系统设置误清空后上传字段不可用。
async fn image_bed_setting_or_default(
    access: &AccessRepository,
    key: &str,
    default_value: &str,
) -> ApiResult<String> {
    let value = access
        .setting_value_optional(key)
        .await?
        .unwrap_or_else(|| default_value.to_string())
        .trim()
        .to_string();

    Ok(if value.is_empty() {
        default_value.to_string()
    } else {
        value
    })
}

/// 按后台配置的字段路径从图床响应中提取图片链接。
fn extract_image_bed_result_field(response: &Value, field_path: &str) -> ApiResult<Value> {
    let Some(value) = resolve_json_path(response, field_path) else {
        return Err(ApiError::BadRequest(format!(
            "图床返回结构中未找到图片链接字段 `{field_path}`"
        )));
    };

    image_bed_value_as_url(value, &format!("图床返回字段 `{field_path}`")).map(Value::String)
}

/// 解析点号分隔的 JSON 字段路径。
fn resolve_json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        match current {
            Value::Object(map) => current = map.get(segment)?,
            _ => return None,
        }
    }
    Some(current)
}
