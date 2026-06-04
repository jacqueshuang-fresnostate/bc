//! 用户接口路由，提供注册、登录、会话、账户与提款方式能力

use axum::{
    extract::{Form, Path, Query, Request, State},
    http::header::AUTHORIZATION,
    middleware::{self, Next},
    response::Response,
    routing::{get, post, put},
    Extension, Json, Router,
};
use std::collections::HashMap;

use crate::{
    app::AppState,
    domain::advertisement::MobileAdvertisement,
    domain::finance::{FinancialAccountSummary, LedgerEntry},
    domain::mobile::MobileSiteConfig,
    domain::permission::SystemSetting,
    domain::recharge::{
        CreateRechargeOrderRequest, CreateRechargeOrderResponse, RechargeConfigResponse,
        RechargeOrderSummary,
    },
    domain::support::{SupportConversation, UserSupportReplyRequest},
    domain::user::WithdrawalMethod,
    domain::user::{
        RegistrationConfig, UserAuthSession, UserBalanceResponse, UserBindEmailRequest,
        UserChangePasswordRequest, UserForgotPasswordRequest, UserForgotPasswordResponse,
        UserLoginRequest, UserLogoutResponse, UserProfileResponse, UserRegisterRequest,
        UserResetPasswordRequest, UserResetPasswordResponse, UserSummary, WithdrawalMethodRequest,
    },
    error::{ApiError, ApiResult},
    response::ApiEnvelope,
    services::recharge::{
        recharge_config_response, recharge_settings_from_system_settings,
        support_ticket_for_recharge,
    },
};

/// 组装并返回当前用户模块对应的路由树。
pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        .route("/me", get(get_current_user))
        .route("/logout", post(logout_user))
        .route("/bind-email", post(bind_email))
        .route("/password/change", post(change_password))
        .route("/balance", get(get_balance))
        .route("/ledger-entries", get(list_ledger_entries))
        .route("/recharge/config", get(get_recharge_config))
        .route(
            "/recharge/orders",
            get(list_recharge_orders).post(create_recharge_order),
        )
        .route(
            "/support/conversations",
            get(list_user_support_conversations),
        )
        .route(
            "/support/conversations/{id}",
            get(get_user_support_conversation),
        )
        .route(
            "/support/conversations/{id}/messages",
            post(reply_user_support_conversation),
        )
        .route(
            "/withdrawal-methods",
            get(list_withdrawal_methods).post(create_withdrawal_method),
        )
        .route(
            "/withdrawal-methods/{method_id}",
            put(update_withdrawal_method).delete(delete_withdrawal_method),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_user_auth,
        ));

    Router::new()
        .route("/mobile/advertisements", get(list_mobile_advertisements))
        .route("/mobile/site-config", get(get_mobile_site_config))
        .route("/register-options", get(get_registration_options))
        .route(
            "/recharge/epay/notify",
            get(rainbow_epay_notify_query).post(rainbow_epay_notify_form),
        )
        .route("/recharge/epay/return", get(rainbow_epay_return_query))
        .route("/register", post(register_user))
        .route("/login", post(login_user))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        .merge(protected_routes)
}

async fn require_user_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> ApiResult<Response> {
    let token = bearer_token(&request)?;
    let session = state.access.session_from_user_token(token).await?;

    request.extensions_mut().insert(session);
    Ok(next.run(request).await)
}

/// 处理 bearer_token 的具体内部流程。
fn bearer_token(request: &Request) -> ApiResult<&str> {
    let header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("authorization token is required".to_string()))?;

    let Some(token) = header.strip_prefix("Bearer ") else {
        return Err(ApiError::Unauthorized(
            "authorization bearer token is required".to_string(),
        ));
    };

    Ok(token)
}

async fn list_mobile_advertisements(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<MobileAdvertisement>>>> {
    let advertisements = state.advertisements.list_mobile_carousel().await?;

    Ok(Json(ApiEnvelope::success(advertisements)))
}

async fn get_mobile_site_config(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<MobileSiteConfig>>> {
    let settings = state.access.settings().await?;
    let config = mobile_site_config_from_settings(&settings);

    Ok(Json(ApiEnvelope::success(config)))
}

/// 返回手机端注册入口需要的公开注册策略。
async fn get_registration_options(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<RegistrationConfig>>> {
    let registration = state.access.registration().await?;

    Ok(Json(ApiEnvelope::success(registration)))
}

/// 从系统设置中提取手机端公开展示配置，隐藏未配置占位值。
fn mobile_site_config_from_settings(settings: &[SystemSetting]) -> MobileSiteConfig {
    MobileSiteConfig {
        platform_name: optional_config_value(settings, "mobile_platform_name")
            .unwrap_or_else(|| "彩票管理系统".to_string()),
        logo_image_url: optional_config_value(settings, "mobile_logo_image_url"),
        intro: config_value(settings, "mobile_site_intro")
            .unwrap_or_else(|| "欢迎使用彩票管理系统，祝您理性购彩、好运常伴。".to_string()),
    }
}

/// 读取可公开配置值，自动忽略空字符串和“未配置”占位。
fn optional_config_value(settings: &[SystemSetting], key: &str) -> Option<String> {
    config_value(settings, key).filter(|value| value != "未配置")
}

/// 按配置键读取系统设置值，统一修剪首尾空白。
fn config_value(settings: &[SystemSetting], key: &str) -> Option<String> {
    settings
        .iter()
        .find(|setting| setting.key == key)
        .map(|setting| setting.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn register_user(
    State(state): State<AppState>,
    Json(payload): Json<UserRegisterRequest>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.register_user(payload).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<UserLoginRequest>,
) -> ApiResult<Json<ApiEnvelope<UserAuthSession>>> {
    let session = state.access.login_user(payload).await?;

    Ok(Json(ApiEnvelope::success(session)))
}

async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<UserForgotPasswordRequest>,
) -> ApiResult<Json<ApiEnvelope<UserForgotPasswordResponse>>> {
    let response = state.access.request_forgot_password(payload).await?;

    Ok(Json(ApiEnvelope::success(response)))
}

async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<UserResetPasswordRequest>,
) -> ApiResult<Json<ApiEnvelope<UserResetPasswordResponse>>> {
    let response = state.access.reset_password(payload).await?;

    Ok(Json(ApiEnvelope::success(response)))
}

async fn get_current_user(
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserProfileResponse>>> {
    Ok(Json(ApiEnvelope::success(UserProfileResponse {
        user: session.user,
    })))
}

async fn logout_user(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserLogoutResponse>>> {
    state.access.logout_user(&session.token).await?;

    Ok(Json(ApiEnvelope::success(UserLogoutResponse {
        logged_out: true,
    })))
}

async fn bind_email(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<UserBindEmailRequest>,
) -> ApiResult<Json<ApiEnvelope<UserAuthSession>>> {
    let user = state.access.bind_email(&session.user.id, payload).await?;

    Ok(Json(ApiEnvelope::success(UserAuthSession {
        token: session.token,
        user,
    })))
}

async fn change_password(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<UserChangePasswordRequest>,
) -> ApiResult<Json<ApiEnvelope<UserAuthSession>>> {
    let user = state
        .access
        .change_password(&session.user.id, payload)
        .await?;

    Ok(Json(ApiEnvelope::success(UserAuthSession {
        token: session.token,
        user,
    })))
}

async fn get_balance(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserBalanceResponse>>> {
    let account: FinancialAccountSummary =
        state.finance.account_or_create(&session.user.id).await?;

    Ok(Json(ApiEnvelope::success(UserBalanceResponse {
        user: session.user,
        account,
    })))
}

async fn list_ledger_entries(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<Vec<LedgerEntry>>>> {
    let entries = state.finance.user_ledger_entries(&session.user.id).await?;

    Ok(Json(ApiEnvelope::success(entries)))
}

async fn get_recharge_config(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<RechargeConfigResponse>>> {
    let settings = state.access.settings().await?;
    let settings = recharge_settings_from_system_settings(&settings);

    Ok(Json(ApiEnvelope::success(recharge_config_response(
        &settings,
    ))))
}

async fn list_recharge_orders(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<Vec<RechargeOrderSummary>>>> {
    let orders = state.recharges.list_for_user(&session.user.id).await?;

    Ok(Json(ApiEnvelope::success(orders)))
}

async fn create_recharge_order(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<CreateRechargeOrderRequest>,
) -> ApiResult<Json<ApiEnvelope<CreateRechargeOrderResponse>>> {
    let settings = state.access.settings().await?;
    let settings = recharge_settings_from_system_settings(&settings);
    let mut response = state
        .recharges
        .create_order(&session.user, payload, &settings)
        .await?;

    if let Some(ticket) = support_ticket_for_recharge(&response.order) {
        let users = state.access.users().await?;
        let conversation = state
            .support
            .create(
                crate::domain::support::CreateSupportConversationRequest {
                    id: ticket.conversation_id,
                    user_id: session.user.id.clone(),
                    subject: ticket.subject,
                    priority: crate::domain::support::SupportPriority::Normal,
                    content: ticket.content,
                },
                &users,
            )
            .await?;
        let order = state
            .recharges
            .attach_support_conversation(&response.order.id, &conversation.id)
            .await?;
        response.support_conversation_id = Some(conversation.id);
        response.order = order;
    }

    Ok(Json(ApiEnvelope::success(response)))
}

async fn rainbow_epay_notify_query(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<String> {
    confirm_rainbow_notify(state, params).await
}

async fn rainbow_epay_notify_form(
    State(state): State<AppState>,
    Form(params): Form<HashMap<String, String>>,
) -> ApiResult<String> {
    confirm_rainbow_notify(state, params).await
}

async fn confirm_rainbow_notify(
    state: AppState,
    params: HashMap<String, String>,
) -> ApiResult<String> {
    let settings = state.access.settings().await?;
    let settings = recharge_settings_from_system_settings(&settings);
    state
        .recharges
        .confirm_rainbow_notify(params, &settings, &state.finance)
        .await?;

    Ok("success".to_string())
}

async fn rainbow_epay_return_query(
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<ApiEnvelope<HashMap<String, String>>>> {
    Ok(Json(ApiEnvelope::success(params)))
}

async fn list_user_support_conversations(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<Vec<SupportConversation>>>> {
    let conversations = state.support.list_for_user(&session.user.id).await?;

    Ok(Json(ApiEnvelope::success(conversations)))
}

async fn get_user_support_conversation(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let conversation = state.support.get_for_user(&id, &session.user.id).await?;

    Ok(Json(ApiEnvelope::success(conversation)))
}

async fn reply_user_support_conversation(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
    Json(payload): Json<UserSupportReplyRequest>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let conversation = state
        .support
        .user_reply(&id, &session.user, payload)
        .await?;

    Ok(Json(ApiEnvelope::success(conversation)))
}

async fn list_withdrawal_methods(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<Vec<WithdrawalMethod>>>> {
    let methods = state
        .access
        .list_withdrawal_methods(&session.user.id)
        .await?;

    Ok(Json(ApiEnvelope::success(methods)))
}

async fn create_withdrawal_method(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<WithdrawalMethodRequest>,
) -> ApiResult<Json<ApiEnvelope<WithdrawalMethod>>> {
    let method = state
        .access
        .create_withdrawal_method(&session.user.id, payload)
        .await?;

    Ok(Json(ApiEnvelope::success(method)))
}

async fn update_withdrawal_method(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(method_id): Path<String>,
    Json(payload): Json<WithdrawalMethodRequest>,
) -> ApiResult<Json<ApiEnvelope<WithdrawalMethod>>> {
    let method = state
        .access
        .update_withdrawal_method(&session.user.id, &method_id, payload)
        .await?;

    Ok(Json(ApiEnvelope::success(method)))
}

async fn delete_withdrawal_method(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(method_id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<()>>> {
    state
        .access
        .delete_withdrawal_method(&session.user.id, &method_id)
        .await?;

    Ok(Json(ApiEnvelope::success(())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// 验证手机端公开配置会隐藏未配置占位，并保留站点介绍。
    fn mobile_site_config_hides_unconfigured_logo() {
        let settings = vec![
            SystemSetting {
                key: "mobile_platform_name".to_string(),
                value: "测试平台".to_string(),
                description: "手机端展示的平台名称".to_string(),
            },
            SystemSetting {
                key: "mobile_logo_image_url".to_string(),
                value: "未配置".to_string(),
                description: "手机端站点 Logo 图片链接".to_string(),
            },
            SystemSetting {
                key: "mobile_site_intro".to_string(),
                value: "欢迎语".to_string(),
                description: "手机端站点介绍".to_string(),
            },
        ];

        let config = mobile_site_config_from_settings(&settings);

        assert_eq!(config.platform_name, "测试平台");
        assert_eq!(config.logo_image_url, None);
        assert_eq!(config.intro, "欢迎语");
    }

    #[test]
    /// 验证手机端公开配置能返回真实 Logo 图片链接。
    fn mobile_site_config_returns_logo_url() {
        let settings = vec![SystemSetting {
            key: "mobile_logo_image_url".to_string(),
            value: "https://example.com/logo.png".to_string(),
            description: "手机端站点 Logo 图片链接".to_string(),
        }];

        let config = mobile_site_config_from_settings(&settings);

        assert_eq!(config.platform_name, "彩票管理系统");
        assert_eq!(
            config.logo_image_url,
            Some("https://example.com/logo.png".to_string())
        );
        assert!(!config.intro.is_empty());
    }
}
