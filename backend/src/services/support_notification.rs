//! 客服消息外部通知服务，负责把用户新消息提醒发送到 Telegram。

use std::time::Duration;

use serde::Serialize;

use crate::{
    domain::{
        permission::SystemSetting,
        support::{SupportConversation, SupportMessage, SupportMessageAuthor},
    },
    error::{ApiError, ApiResult},
    services::access::AccessRepository,
};

/// 系统设置中保存客服 Telegram 提醒开关的键名。
pub const SUPPORT_TELEGRAM_ENABLED_SETTING: &str = "support_telegram_notification_enabled";
/// 系统设置中保存 Telegram Bot Token 的键名。
pub const SUPPORT_TELEGRAM_BOT_TOKEN_SETTING: &str = "support_telegram_bot_token";
/// 系统设置中保存 Telegram 接收会话或频道 ID 的键名。
pub const SUPPORT_TELEGRAM_CHAT_ID_SETTING: &str = "support_telegram_chat_id";

const TELEGRAM_SEND_MESSAGE_TIMEOUT_SECONDS: u64 = 8;
const SUPPORT_MESSAGE_PREVIEW_MAX_CHARS: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 客服 Telegram 提醒配置，来源于后台系统设置。
pub struct SupportTelegramNotificationSettings {
    pub enabled: bool,
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Serialize)]
/// Telegram sendMessage 请求体，只使用文本提醒能力。
struct TelegramSendMessageRequest {
    chat_id: String,
    text: String,
    disable_web_page_preview: bool,
}

/// 用户客服消息落库后异步尝试发送 Telegram 提醒，失败只写日志不影响主业务。
pub fn spawn_support_telegram_notification(
    access: AccessRepository,
    conversation: &SupportConversation,
) {
    let Some(message) = conversation.messages.last().cloned() else {
        return;
    };
    if !support_message_requires_telegram_notification(&message) {
        return;
    }

    let conversation = conversation.clone();
    tokio::spawn(async move {
        if let Err(error) =
            notify_support_user_message_to_telegram(&access, &conversation, &message).await
        {
            tracing::warn!(
                conversation_id = %conversation.id,
                user_id = %conversation.user_id,
                message_id = %message.id,
                error = %error.log_message(),
                "客服消息 Telegram 提醒发送失败"
            );
        }
    });
}

/// 按当前系统设置把用户客服消息发送到 Telegram。
async fn notify_support_user_message_to_telegram(
    access: &AccessRepository,
    conversation: &SupportConversation,
    message: &SupportMessage,
) -> ApiResult<()> {
    let settings = access.settings().await?;
    let settings = support_telegram_settings_from_system_settings(&settings);
    if !settings.enabled {
        return Ok(());
    }
    validate_support_telegram_settings(&settings)?;

    let text = build_support_telegram_message(conversation, message);
    send_telegram_message(&settings, text).await
}

/// 从系统设置解析客服 Telegram 提醒配置，默认关闭。
pub fn support_telegram_settings_from_system_settings(
    settings: &[SystemSetting],
) -> SupportTelegramNotificationSettings {
    SupportTelegramNotificationSettings {
        enabled: setting_value(settings, SUPPORT_TELEGRAM_ENABLED_SETTING)
            .map(|value| bool_setting(value, false))
            .unwrap_or(false),
        bot_token: setting_value(settings, SUPPORT_TELEGRAM_BOT_TOKEN_SETTING)
            .map(unconfigured_to_empty)
            .unwrap_or_default(),
        chat_id: setting_value(settings, SUPPORT_TELEGRAM_CHAT_ID_SETTING)
            .map(unconfigured_to_empty)
            .unwrap_or_default(),
    }
}

/// 只提醒用户发来的客服消息，后台客服回复和系统消息不触发 Telegram。
pub fn support_message_requires_telegram_notification(message: &SupportMessage) -> bool {
    matches!(message.author, SupportMessageAuthor::User)
}

/// 构造 Telegram 文本提醒，控制消息长度并保留排查所需的会话和用户标识。
pub fn build_support_telegram_message(
    conversation: &SupportConversation,
    message: &SupportMessage,
) -> String {
    let content =
        truncate_for_telegram_preview(&message.content, SUPPORT_MESSAGE_PREVIEW_MAX_CHARS);
    format!(
        "新的客服消息提醒\n会话：{}\n用户：{}（{}）\n主题：{}\n时间：{}\n内容：{}",
        conversation.id,
        conversation.username,
        conversation.user_id,
        conversation.subject,
        message.created_at,
        content
    )
}

/// 校验 Telegram 提醒必要配置，避免请求第三方接口时才暴露空值。
fn validate_support_telegram_settings(
    settings: &SupportTelegramNotificationSettings,
) -> ApiResult<()> {
    if settings.bot_token.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "Telegram Bot Token 未配置".to_string(),
        ));
    }
    if settings.chat_id.trim().is_empty() {
        return Err(ApiError::BadRequest("Telegram Chat ID 未配置".to_string()));
    }

    Ok(())
}

/// 调用 Telegram Bot API 发送文本消息。
async fn send_telegram_message(
    settings: &SupportTelegramNotificationSettings,
    text: String,
) -> ApiResult<()> {
    let url = format!(
        "https://api.telegram.org/bot{}/sendMessage",
        settings.bot_token
    );
    let response = reqwest::Client::new()
        .post(url)
        .timeout(Duration::from_secs(TELEGRAM_SEND_MESSAGE_TIMEOUT_SECONDS))
        .json(&TelegramSendMessageRequest {
            chat_id: settings.chat_id.clone(),
            text,
            disable_web_page_preview: true,
        })
        .send()
        .await
        .map_err(|error| {
            ApiError::Internal(format!("Telegram 请求发送失败：{}", error.without_url()))
        })?;

    if response.status().is_success() {
        return Ok(());
    }

    let status = response.status();
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "响应内容读取失败".to_string());
    Err(ApiError::Internal(format!(
        "Telegram 返回失败：HTTP {status}，响应内容 {response_text}"
    )))
}

/// 按键名读取系统设置原始值。
fn setting_value<'a>(settings: &'a [SystemSetting], key: &str) -> Option<&'a str> {
    settings
        .iter()
        .find(|setting| setting.key == key)
        .map(|setting| setting.value.trim())
}

/// 解析布尔配置，兼容常见开启值。
fn bool_setting(value: &str, fallback: bool) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return fallback;
    }
    matches!(value, "true" | "1" | "yes" | "on")
}

/// 把“未配置”等占位值转成空值，避免误当成真实密钥或 Chat ID。
fn unconfigured_to_empty(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() || matches!(value, "未配置" | "请配置" | "please-configure") {
        String::new()
    } else {
        value.to_string()
    }
}

/// 截断 Telegram 预览文本，避免一条长消息刷屏。
fn truncate_for_telegram_preview(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut preview = value.chars().take(max_chars).collect::<String>();
    preview.push('…');
    preview
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::support::{SupportConversationStatus, SupportMessageType, SupportPriority};

    #[test]
    /// 验证客服 Telegram 配置默认关闭，并把占位配置视为空值。
    fn support_telegram_settings_default_to_disabled() {
        let settings = support_telegram_settings_from_system_settings(&[
            setting(SUPPORT_TELEGRAM_ENABLED_SETTING, "false"),
            setting(SUPPORT_TELEGRAM_BOT_TOKEN_SETTING, "未配置"),
            setting(SUPPORT_TELEGRAM_CHAT_ID_SETTING, "请配置"),
        ]);

        assert!(!settings.enabled);
        assert_eq!(settings.bot_token, "");
        assert_eq!(settings.chat_id, "");
    }

    #[test]
    /// 验证只有用户消息会触发 Telegram 提醒。
    fn support_telegram_notification_only_targets_user_messages() {
        let mut message = test_message(SupportMessageAuthor::User);
        assert!(support_message_requires_telegram_notification(&message));

        message.author = SupportMessageAuthor::Admin;
        assert!(!support_message_requires_telegram_notification(&message));
        message.author = SupportMessageAuthor::System;
        assert!(!support_message_requires_telegram_notification(&message));
    }

    #[test]
    /// 验证 Telegram 提醒文本包含客服会话排查所需的用户、主题和消息内容。
    fn support_telegram_message_contains_conversation_context() {
        let conversation = test_conversation();
        let message = conversation.messages.last().expect("测试会话有消息");
        let text = build_support_telegram_message(&conversation, message);

        assert!(text.contains("新的客服消息提醒"));
        assert!(text.contains("会话：CS-TEST"));
        assert!(text.contains("用户：tester（U10001）"));
        assert!(text.contains("主题：充值咨询"));
        assert!(text.contains("内容：充值凭证已经上传，请帮我确认。"));
    }

    fn setting(key: &str, value: &str) -> SystemSetting {
        SystemSetting {
            key: key.to_string(),
            value: value.to_string(),
            description: "测试配置".to_string(),
        }
    }

    fn test_conversation() -> SupportConversation {
        SupportConversation {
            id: "CS-TEST".to_string(),
            user_id: "U10001".to_string(),
            username: "tester".to_string(),
            subject: "充值咨询".to_string(),
            status: SupportConversationStatus::Open,
            priority: SupportPriority::Normal,
            assigned_admin_id: None,
            assigned_admin_name: None,
            unread_count: 1,
            user_unread_count: 0,
            created_at: "2026-06-09 18:00:00".to_string(),
            updated_at: "2026-06-09 18:00:00".to_string(),
            messages: vec![test_message(SupportMessageAuthor::User)],
        }
    }

    fn test_message(author: SupportMessageAuthor) -> SupportMessage {
        SupportMessage {
            id: "CS-TEST-M001".to_string(),
            author,
            author_id: "U10001".to_string(),
            author_name: "tester".to_string(),
            message_type: SupportMessageType::Text,
            content: "充值凭证已经上传，请帮我确认。".to_string(),
            image_url: None,
            created_at: "2026-06-09 18:00:00".to_string(),
        }
    }
}
