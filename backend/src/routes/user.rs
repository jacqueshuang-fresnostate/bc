//! 用户接口路由，提供注册、登录、会话、账户与提款方式能力

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Form, Multipart, Path, Query, Request, State},
    http::{header::AUTHORIZATION, HeaderMap},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Extension, Json, Router,
};
use chrono::{NaiveDateTime, TimeZone};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::Duration;

use crate::{
    app::AppState,
    domain::advertisement::MobileAdvertisement,
    domain::agent_application::{
        AgentApplication, SubmitAgentApplicationRequest, UserAgentApplicationResponse,
    },
    domain::chat_hall::{
        ChatHallGroupBuyPlanPayload, ChatHallMessage, ChatHallRedPacketClaimsResponse,
        ChatHallSpeakingStatusResponse, ClaimChatHallRedPacketResponse,
        CreateChatHallMessageRequest, CreateChatHallRedPacketRequest,
        ShareChatHallGroupBuyPlanRequest,
    },
    domain::draw::{DrawIssue, DrawIssueStatus},
    domain::finance::{FinancialAccountSummary, LedgerEntry, LedgerEntryKind},
    domain::group_buy::{
        AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyCreateOptions,
        GroupBuyCreateSettings, GroupBuyParticipationSummary, GroupBuyPlan, GroupBuyPlanStatus,
        GroupBuySelectOption, UserCreateGroupBuyPlanRequest, UserGroupBuyActionResponse,
        UserGroupBuyParticipantSummary, UserGroupBuyPlan, UserGroupBuyPlanPage,
        UserJoinGroupBuyPlanRequest,
    },
    domain::invite::{InviteRecord, InviteStatus},
    domain::lottery::{LotteryKind, LotteryNumberType},
    domain::mobile::{
        MobileAppUpdateConfig, MobileBetPageConfig, MobileCreateBetOrderBatchRequest,
        MobileCreateBetOrderBatchResponse, MobileSiteConfig,
    },
    domain::order::{CreateOrderRequest, OrderDetail, OrderSource, OrderStatus},
    domain::permission::SystemSetting,
    domain::rebate::InvitePolicySummary,
    domain::recharge::{
        CreateRechargeOrderRequest, CreateRechargeOrderResponse, RechargeConfigResponse,
        RechargeOrderSummary,
    },
    domain::support::{SupportConversation, UserSupportReplyRequest},
    domain::user::WithdrawalMethod,
    domain::user::{
        RegistrationConfig, UserAuthSession, UserAvatarRequest, UserBalanceResponse,
        UserBindEmailRequest, UserChangePasswordRequest, UserForgotPasswordRequest,
        UserForgotPasswordResponse, UserInvitationBetLotterySummary, UserInvitationBetPlaySummary,
        UserInvitationDirectUser, UserInvitationLatestBet, UserInvitationSummaryResponse, UserKind,
        UserLoginRequest, UserLogoutResponse, UserProfileResponse, UserRegisterRequest,
        UserResetPasswordRequest, UserResetPasswordResponse, UserStatus, UserSummary,
        WithdrawalMethodRequest,
    },
    domain::withdrawal::{CreateWithdrawalOrderRequest, WithdrawalOrderSummary},
    error::{ApiError, ApiResult},
    response::ApiEnvelope,
    services::recharge::{
        recharge_config_response, recharge_settings_from_system_settings,
        support_ticket_for_recharge,
    },
    services::{
        business_database::enum_to_string,
        group_buy_flow::{
            build_group_buy_order_request, create_order_for_filled_group_buy,
            parse_group_buy_rule_code, parse_group_buy_selection,
        },
        group_buy_robot::is_group_buy_robot_user_id,
        image_bed::{
            image_bed_value_as_url, upload_configured_image_bed_file, ImageBedUploadOptions,
        },
        mobile_bet::build_mobile_bet_page_config,
        order::validate_draw_issue_accepts_order,
        pagination::PageRequest,
        play_rules::{expanded_bets_for_rule, play_rule_summaries},
        realtime::{
            audience_matches, balance_changed_event, chat_hall_message_created_event,
            heartbeat_event, order_changed_event, recharge_changed_event,
            support_conversation_updated_event, support_message_created_event,
            withdrawal_changed_event,
        },
        rebate::credit_recharge_rebate_for_order,
        support_notification::spawn_support_telegram_notification,
    },
};

const MAX_USER_BET_BATCH_SIZE: usize = 50;
const REALTIME_HEARTBEAT_SECONDS: u64 = 30;
const CHAT_HALL_SPEAKING_MIN_RECHARGE_SETTING_KEY: &str = "chat_hall_speaking_min_recharge_minor";
const ROBOT_GROUP_BUY_PLAN_PREFIX: &str = "G-ROBOT-";
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const ROBOT_GROUP_BUY_NICKNAME_BASES: &[&str] = &[
    "南风", "晴岚", "星野", "北辰", "橙光", "江月", "云起", "青柠", "初夏", "山海", "一念", "雾岛",
    "星眠", "晚舟", "听雨", "拾光", "月白", "海盐", "半夏", "松间", "清欢", "明澈", "若晴", "知夏",
    "予安", "映川", "星落", "云栖", "澄意", "青禾", "晚晴", "朝露",
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 用户端客服图片上传响应，只返回可写入客服消息的图片链接。
struct SupportImageUploadResponse {
    image_url: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 用户端列表分页参数。
struct UserPageQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    view: Option<UserBetOrderView>,
}

impl UserPageQuery {
    /// 当分页参数为空时返回全量，不分页；有分页参数时按页切片。
    fn paginate<T>(&self, items: Vec<T>) -> Vec<T> {
        if self.page.is_none() && self.page_size.is_none() {
            return items;
        }

        let total_count = items.len();
        let page_size = self.page_size.unwrap_or(20).max(1);
        let max_page = if total_count == 0 {
            1
        } else {
            total_count.div_ceil(page_size)
        };
        let page = self.page.unwrap_or(1).max(1).min(max_page);
        let start = (page - 1).saturating_mul(page_size);
        let end = (start + page_size).min(total_count);

        items.into_iter().skip(start).take(end - start).collect()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 用户端注单列表视图，用于把真实注单和未成单合买认购分开分页。
enum UserBetOrderView {
    Orders,
    GroupBuy,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 手机端 APP 更新检查查询参数；平台为空时默认按 Android 处理。
struct MobileAppUpdateQuery {
    platform: Option<String>,
    current_version: Option<String>,
    current_build: Option<u32>,
}

/// 组装并返回当前用户模块对应的路由树。
pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        .route("/me", get(get_current_user))
        .route("/avatar", put(update_user_avatar))
        .route("/avatar/upload", post(upload_user_avatar))
        .route("/logout", post(logout_user))
        .route("/bind-email", post(bind_email))
        .route("/password/change", post(change_password))
        .route("/balance", get(get_balance))
        .route("/ledger-entries", get(list_ledger_entries))
        .route("/invitations/summary", get(get_user_invitation_summary))
        .route(
            "/agent/application",
            get(get_user_agent_application).post(submit_user_agent_application),
        )
        .route(
            "/bet/page-config/{lottery_id}",
            get(get_user_bet_page_config),
        )
        .route(
            "/bet/orders",
            get(list_user_bet_orders).post(create_user_bet_orders),
        )
        .route(
            "/group-buy/plans",
            get(list_user_group_buy_plans).post(create_user_group_buy_plan),
        )
        .route("/group-buy/plans/{id}", get(get_user_group_buy_plan))
        .route(
            "/group-buy/plans/{id}/participants",
            post(join_user_group_buy_plan),
        )
        .route("/group-buy/my", get(list_my_group_buy_plans))
        .route(
            "/group-buy/create-options",
            get(get_user_group_buy_create_options),
        )
        .route("/recharge/config", get(get_recharge_config))
        .route(
            "/recharge/orders",
            get(list_recharge_orders).post(create_recharge_order),
        )
        .route(
            "/chat-hall/messages",
            get(list_chat_hall_messages).post(send_chat_hall_message),
        )
        .route(
            "/chat-hall/speaking-status",
            get(get_chat_hall_speaking_status),
        )
        .route("/chat-hall/red-packets", post(send_chat_hall_red_packet))
        .route(
            "/chat-hall/red-packets/{id}/claims",
            get(get_chat_hall_red_packet_claims),
        )
        .route(
            "/chat-hall/red-packets/{id}/claim",
            post(claim_chat_hall_red_packet),
        )
        .route(
            "/chat-hall/group-buy-plans",
            post(share_chat_hall_group_buy_plan),
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
        .route("/support/images/upload", post(upload_user_support_image))
        .route(
            "/support/conversations/{id}/read",
            post(mark_user_support_conversation_read),
        )
        .route(
            "/withdrawal-methods",
            get(list_withdrawal_methods).post(create_withdrawal_method),
        )
        .route(
            "/withdrawal-methods/{method_id}",
            put(update_withdrawal_method).delete(delete_withdrawal_method),
        )
        .route(
            "/withdrawals",
            get(list_withdrawal_orders).post(create_withdrawal_order),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_user_auth,
        ));

    Router::new()
        .route("/mobile/advertisements", get(list_mobile_advertisements))
        .route("/mobile/site-config", get(get_mobile_site_config))
        .route("/mobile/app-update", get(get_mobile_app_update))
        .route("/realtime", get(open_user_realtime_socket))
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

/// 校验用户端接口的登录态，并返回当前用户会话。
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

/// 从 Authorization 请求头提取 Bearer token。
fn bearer_token(request: &Request) -> ApiResult<&str> {
    let header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("请先登录".to_string()))?;

    let Some(token) = header.strip_prefix("Bearer ") else {
        return Err(ApiError::Unauthorized("登录令牌格式错误".to_string()));
    };

    Ok(token)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 用户端实时连接查询参数，支持 WebSocket token 鉴权。
struct UserRealtimeQuery {
    token: Option<String>,
}

/// 建立手机端实时事件连接；匿名连接只接收公开彩种事件，带用户 token 时追加本人私有事件。
async fn open_user_realtime_socket(
    State(state): State<AppState>,
    Query(query): Query<UserRealtimeQuery>,
    ws: WebSocketUpgrade,
) -> ApiResult<Response> {
    let user_id = match query.token {
        Some(token) if !token.trim().is_empty() => Some(
            state
                .access
                .session_from_user_token(token.trim())
                .await?
                .user
                .id,
        ),
        _ => None,
    };
    let realtime = state.realtime.clone();

    Ok(ws
        .on_upgrade(move |socket| handle_user_realtime_socket(socket, realtime, user_id))
        .into_response())
}

/// 持续向单个手机端连接发送实时事件和心跳。
async fn handle_user_realtime_socket(
    mut socket: WebSocket,
    realtime: crate::services::realtime::RealtimeHub,
    user_id: Option<String>,
) {
    let mut receiver = realtime.subscribe();
    let mut heartbeat = tokio::time::interval(Duration::from_secs(REALTIME_HEARTBEAT_SECONDS));

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if send_realtime_payload(&mut socket, heartbeat_event()).await.is_err() {
                    break;
                }
            }
            message = receiver.recv() => {
                match message {
                    Ok(message) => {
                        if audience_matches(&message.audience, user_id.as_deref())
                            && send_realtime_payload(&mut socket, message.payload).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped_count)) => {
                        tracing::warn!(
                            skipped_count,
                            "手机端实时事件连接消费过慢，已跳过部分历史事件"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

/// 将实时事件 JSON 发送到 WebSocket 连接。
async fn send_realtime_payload(
    socket: &mut WebSocket,
    payload: serde_json::Value,
) -> Result<(), axum::Error> {
    socket.send(Message::Text(payload.to_string().into())).await
}

/// 返回手机端轮播广告列表。
async fn list_mobile_advertisements(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<MobileAdvertisement>>>> {
    let advertisements = state.advertisements.list_mobile_carousel().await?;

    Ok(Json(ApiEnvelope::success(advertisements)))
}

/// 返回手机端平台名称、Logo 和简介配置。
async fn get_mobile_site_config(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<MobileSiteConfig>>> {
    let settings = state.access.settings().await?;
    let config = mobile_site_config_from_settings(&settings);

    Ok(Json(ApiEnvelope::success(config)))
}

/// 返回手机端 APP 更新配置，客户端启动时据此展示强制或可选更新弹窗。
async fn get_mobile_app_update(
    State(state): State<AppState>,
    Query(query): Query<MobileAppUpdateQuery>,
) -> ApiResult<Json<ApiEnvelope<MobileAppUpdateConfig>>> {
    let settings = state.access.settings().await?;
    let config = mobile_app_update_from_settings(&settings, &query);

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

/// 从系统设置组装 APP 更新检查结果，并根据客户端当前版本判断是否需要更新。
fn mobile_app_update_from_settings(
    settings: &[SystemSetting],
    query: &MobileAppUpdateQuery,
) -> MobileAppUpdateConfig {
    let platform = normalize_mobile_app_platform(query.platform.as_deref());
    let key_prefix = format!("mobile_app_{platform}");
    let enabled = config_bool(settings, &format!("{key_prefix}_enabled"));
    let latest_version = config_value(settings, &format!("{key_prefix}_latest_version"))
        .unwrap_or_else(|| "0.1.0".to_string());
    let latest_build = config_u32(settings, &format!("{key_prefix}_latest_build")).unwrap_or(1);
    let download_url = optional_config_value(settings, &format!("{key_prefix}_package_url"))
        .filter(|url| url.starts_with("http://") || url.starts_with("https://"));
    let force_update = config_bool(settings, &format!("{key_prefix}_force_update"));
    let release_notes =
        config_value(settings, &format!("{key_prefix}_release_notes")).unwrap_or_default();
    let current_version = query.current_version.as_deref().unwrap_or("0.0.0");
    let current_build = query.current_build.unwrap_or(0);
    let update_available = enabled
        && download_url.is_some()
        && (latest_build > current_build || version_is_newer(&latest_version, current_version));

    MobileAppUpdateConfig {
        platform: platform.to_string(),
        enabled,
        latest_version,
        latest_build,
        download_url,
        force_update,
        release_notes,
        update_available,
    }
}

/// 标准化客户端平台参数，避免 ipa、iphone 等别名导致读取不到 iOS 配置。
fn normalize_mobile_app_platform(platform: Option<&str>) -> &'static str {
    match platform
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "ios" | "iphone" | "ipad" | "ipa" => "ios",
        _ => "android",
    }
}

/// 读取布尔配置，只有明确写入 true、1、yes、on 时才视为开启。
fn config_bool(settings: &[SystemSetting], key: &str) -> bool {
    config_value(settings, key)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "true" | "1" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

/// 读取无符号整数配置，解析失败时交给调用方使用默认值。
fn config_u32(settings: &[SystemSetting], key: &str) -> Option<u32> {
    config_value(settings, key).and_then(|value| value.parse::<u32>().ok())
}

/// 比较版本号的数字片段，支持 1.2.3、1.2.3-beta 这类常见格式。
fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest_parts = version_numeric_parts(latest);
    let current_parts = version_numeric_parts(current);
    let max_len = latest_parts.len().max(current_parts.len());
    for index in 0..max_len {
        let latest_part = latest_parts.get(index).copied().unwrap_or(0);
        let current_part = current_parts.get(index).copied().unwrap_or(0);
        match latest_part.cmp(&current_part) {
            Ordering::Greater => return true,
            Ordering::Less => return false,
            Ordering::Equal => {}
        }
    }
    false
}

/// 提取版本号中的数字片段，空版本统一按 0 处理。
fn version_numeric_parts(version: &str) -> Vec<u32> {
    let parts: Vec<u32> = version
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u32>().ok())
        .collect();
    if parts.is_empty() {
        vec![0]
    } else {
        parts
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

/// 用户注册接口，支持用户名或邮箱注册。
async fn register_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut payload): Json<UserRegisterRequest>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    attach_registration_client_info(
        &mut payload,
        registration_client_info_from_headers(&headers),
    );
    let user = state.access.register_user(payload).await?;
    let account = state.finance.account_or_create(&user.id).await?;
    let user = user_with_account_balance(user, Some(&account));

    Ok(Json(ApiEnvelope::success(user)))
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
/// 注册请求的服务端来源信息，只从请求头提取，不信任客户端请求体里的推断地区。
struct RegistrationClientInfo {
    /// 服务端从代理链路识别到的客户端 IP。
    registered_ip: Option<String>,
    /// Cloudflare IP Geolocation 提供的国家或地区。
    cloudflare_country: Option<String>,
}

/// 给注册请求补入服务端可见的请求 IP 和可信代理地区，客户端定位失败时也不阻断注册。
fn attach_registration_client_info(
    payload: &mut UserRegisterRequest,
    client_info: RegistrationClientInfo,
) {
    let mut location = payload.registration_location.take().unwrap_or_default();
    let has_gps_location = matches!(
        location.source.trim().to_ascii_lowercase().as_str(),
        "gps" | "geo" | "geolocation"
    );
    if let Some(ip) = client_info.registered_ip {
        location.registered_ip = ip;
        if !has_gps_location {
            if let Some(country) = client_info.cloudflare_country {
                location.country = country;
            } else {
                location.country.clear();
            }
            location.region.clear();
            location.city.clear();
            location.source = "ip".to_string();
        }
        if location.source.trim().is_empty() || location.source == "unknown" {
            location.source = "ip".to_string();
        }
    }
    if location.source.trim().is_empty() {
        location.source = "unknown".to_string();
    }
    payload.registration_location = Some(location);
}

/// 从常见反向代理请求头里提取客户端 IP 和可信地区，用于注册来源审计。
fn registration_client_info_from_headers(headers: &HeaderMap) -> RegistrationClientInfo {
    let registered_ip = registration_ip_from_headers(headers);
    let cloudflare_country = registered_ip
        .as_ref()
        .and_then(|_| registration_cloudflare_country_from_headers(headers));

    RegistrationClientInfo {
        registered_ip,
        cloudflare_country,
    }
}

/// 从常见反向代理请求头里提取第一个客户端 IP，Cloudflare 专用头优先级最高。
fn registration_ip_from_headers(headers: &HeaderMap) -> Option<String> {
    [
        "cf-connecting-ip",
        "true-client-ip",
        "x-forwarded-for",
        "forwarded",
        "x-real-ip",
        "x-client-ip",
    ]
    .iter()
    .filter_map(|name| headers.get(*name))
    .filter_map(|value| value.to_str().ok())
    .find_map(first_registration_ip)
}

/// 读取 Cloudflare IP Geolocation 的国家或地区头，过滤未知、匿名网络等占位值。
fn registration_cloudflare_country_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("cf-ipcountry")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_ascii_uppercase())
        .filter(|value| !value.is_empty())
        .filter(|value| value != "XX" && value != "T1")
        .map(|value| registration_country_label(&value))
}

/// 把常见国家或地区代码转为后台可读中文，未知代码保留原值便于排障。
fn registration_country_label(code: &str) -> String {
    match code {
        "CN" => "中国",
        "HK" => "中国香港",
        "MO" => "中国澳门",
        "TW" => "中国台湾",
        "SG" => "新加坡",
        "MY" => "马来西亚",
        "TH" => "泰国",
        "VN" => "越南",
        "ID" => "印度尼西亚",
        "PH" => "菲律宾",
        "JP" => "日本",
        "KR" => "韩国",
        "US" => "美国",
        "CA" => "加拿大",
        "AU" => "澳大利亚",
        _ => code,
    }
    .to_string()
}

/// 解析单个 IP 请求头值，兼容逗号链路、Forwarded 风格和 IPv4 端口。
fn first_registration_ip(value: &str) -> Option<String> {
    value.split(',').find_map(|part| {
        let trimmed = part.trim();
        let forwarded_value = trimmed
            .split(';')
            .find_map(|segment| segment.trim().strip_prefix("for="));
        let candidate = forwarded_value
            .unwrap_or_else(|| trimmed.split(';').next().unwrap_or(trimmed))
            .trim_matches('"')
            .trim();
        if candidate.is_empty() {
            return None;
        }
        if candidate.eq_ignore_ascii_case("unknown") {
            return None;
        }
        let candidate = candidate
            .strip_prefix('[')
            .and_then(|value| value.split(']').next())
            .unwrap_or(candidate);
        let candidate = if candidate.parse::<std::net::IpAddr>().is_ok() {
            candidate
        } else if candidate.matches('.').count() == 3 {
            candidate.split(':').next().unwrap_or(candidate)
        } else {
            candidate
        };
        candidate
            .parse::<std::net::IpAddr>()
            .ok()
            .map(|ip| ip.to_string())
    })
}

/// 用户登录接口，支持用户名或邮箱作为登录标识。
async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<UserLoginRequest>,
) -> ApiResult<Json<ApiEnvelope<UserAuthSession>>> {
    let mut session = state.access.login_user(payload).await?;
    session.user = user_with_financial_balance(&state, session.user).await?;

    Ok(Json(ApiEnvelope::success(session)))
}

/// 用户忘记密码接口，生成临时重置 token。
async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<UserForgotPasswordRequest>,
) -> ApiResult<Json<ApiEnvelope<UserForgotPasswordResponse>>> {
    let response = state.access.request_forgot_password(payload).await?;

    Ok(Json(ApiEnvelope::success(response)))
}

/// 用户通过重置 token 修改密码。
async fn reset_password(
    State(state): State<AppState>,
    Json(payload): Json<UserResetPasswordRequest>,
) -> ApiResult<Json<ApiEnvelope<UserResetPasswordResponse>>> {
    let response = state.access.reset_password(payload).await?;

    Ok(Json(ApiEnvelope::success(response)))
}

/// 返回当前登录用户资料。
async fn get_current_user(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserProfileResponse>>> {
    let user = user_with_financial_balance(&state, session.user).await?;
    Ok(Json(ApiEnvelope::success(UserProfileResponse { user })))
}

/// 当前用户直接设置头像链接，适用于外部已上传后只回写图片地址的场景。
async fn update_user_avatar(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<UserAvatarRequest>,
) -> ApiResult<Json<ApiEnvelope<UserProfileResponse>>> {
    let user = state
        .access
        .update_user_avatar(&session.user.id, payload)
        .await?;
    state
        .chat_hall
        .update_user_avatar(&user.id, &user.avatar_url)
        .await?;
    let user = user_with_financial_balance(&state, user).await?;

    Ok(Json(ApiEnvelope::success(UserProfileResponse { user })))
}

/// 当前用户上传头像文件：后端读取图床配置代理上传，并把返回图片链接写入用户资料。
async fn upload_user_avatar(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    payload: Multipart,
) -> ApiResult<Json<ApiEnvelope<UserProfileResponse>>> {
    let avatar_url = upload_avatar_to_image_bed(&state, payload).await?;
    let user = state
        .access
        .update_user_avatar(&session.user.id, UserAvatarRequest { avatar_url })
        .await?;
    state
        .chat_hall
        .update_user_avatar(&user.id, &user.avatar_url)
        .await?;
    let user = user_with_financial_balance(&state, user).await?;

    Ok(Json(ApiEnvelope::success(UserProfileResponse { user })))
}

/// 按系统设置把头像文件透传到图床，返回可保存到用户资料的图片链接。
async fn upload_avatar_to_image_bed(state: &AppState, payload: Multipart) -> ApiResult<String> {
    let output = upload_configured_image_bed_file(
        &state.access,
        payload,
        ImageBedUploadOptions {
            image_only: true,
            missing_file_message: "未检测到头像图片文件字段",
            default_file_name: "avatar.png",
        },
    )
    .await?;

    image_bed_value_as_url(&output, "图床返回的头像链接")
}

/// 当前用户上传客服图片：后端读取图床配置代理上传，并返回图片链接供消息发送使用。
async fn upload_user_support_image(
    State(state): State<AppState>,
    payload: Multipart,
) -> ApiResult<Json<ApiEnvelope<SupportImageUploadResponse>>> {
    let output = upload_configured_image_bed_file(
        &state.access,
        payload,
        ImageBedUploadOptions {
            image_only: true,
            missing_file_message: "未检测到客服图片文件字段",
            default_file_name: "support-image.png",
        },
    )
    .await?;
    let image_url = image_bed_value_as_url(&output, "图床返回的客服图片链接")?;

    Ok(Json(ApiEnvelope::success(SupportImageUploadResponse {
        image_url,
    })))
}

/// 注销当前用户登录会话。
async fn logout_user(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserLogoutResponse>>> {
    state.access.logout_user(&session.token).await?;

    Ok(Json(ApiEnvelope::success(UserLogoutResponse {
        logged_out: true,
    })))
}

/// 当前用户绑定或更新邮箱。
async fn bind_email(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<UserBindEmailRequest>,
) -> ApiResult<Json<ApiEnvelope<UserAuthSession>>> {
    let user = state.access.bind_email(&session.user.id, payload).await?;
    let user = user_with_financial_balance(&state, user).await?;

    Ok(Json(ApiEnvelope::success(UserAuthSession {
        token: session.token,
        user,
    })))
}

/// 当前用户修改登录密码。
async fn change_password(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<UserChangePasswordRequest>,
) -> ApiResult<Json<ApiEnvelope<UserAuthSession>>> {
    let user = state
        .access
        .change_password(&session.user.id, payload)
        .await?;
    let user = user_with_financial_balance(&state, user).await?;

    Ok(Json(ApiEnvelope::success(UserAuthSession {
        token: session.token,
        user,
    })))
}

/// 返回当前用户余额和资金账户。
async fn get_balance(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserBalanceResponse>>> {
    let account: FinancialAccountSummary =
        state.finance.account_or_create(&session.user.id).await?;
    let user = user_with_account_balance(session.user, Some(&account));

    Ok(Json(ApiEnvelope::success(UserBalanceResponse {
        user,
        account,
    })))
}

/// 用户端返回用户摘要时优先展示财务账户可用余额。
async fn user_with_financial_balance(
    state: &AppState,
    user: UserSummary,
) -> ApiResult<UserSummary> {
    let account = state.finance.account_or_create(&user.id).await?;
    Ok(user_with_account_balance(user, Some(&account)))
}

/// 合并用户基础资料和资金账户，避免资料表余额与财务账户余额不一致。
fn user_with_account_balance(
    mut user: UserSummary,
    account: Option<&FinancialAccountSummary>,
) -> UserSummary {
    if let Some(account) = account {
        user.balance_minor = account.available_balance_minor;
    }
    user
}

/// 返回当前用户自己的资金流水。
async fn list_ledger_entries(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Query(query): Query<UserPageQuery>,
) -> ApiResult<Json<ApiEnvelope<Vec<LedgerEntry>>>> {
    let page = state
        .finance
        .user_ledger_entry_page(
            &session.user.id,
            PageRequest::new(query.page, query.page_size),
        )
        .await?;

    Ok(Json(ApiEnvelope::success(page.items)))
}

/// 汇总当前用户的邀请中心信息，供手机端展示邀请码、直属用户和充值统计。
async fn get_user_invitation_summary(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserInvitationSummaryResponse>>> {
    let policy = state.rebates.get().await?;
    let can_invite = user_can_invite(&session.user, &policy);
    let direct_users = if matches!(session.user.kind, UserKind::Agent) {
        let access = state.access.snapshot().await?;
        let invite_records = state.invites.list().await?;
        let candidates = collect_direct_invitation_candidates(
            &session.user.id,
            &access.users,
            &invite_records,
            can_invite,
        );
        let direct_user_ids = candidates
            .iter()
            .map(|candidate| candidate.user.id.clone())
            .collect::<Vec<_>>();
        let balance_by_user_id = direct_user_available_balances(&state, &direct_user_ids).await?;
        let recharge_totals = direct_user_recharge_totals(&state, &direct_user_ids).await?;
        let withdrawal_totals = direct_user_withdrawal_totals(&state).await?;
        let mut bet_profiles = direct_user_bet_profiles(&state, &direct_user_ids).await?;
        let mut direct_users = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let available_balance_minor = balance_by_user_id
                .get(&candidate.user.id)
                .copied()
                .unwrap_or_default();
            let total_deposit_minor = recharge_totals
                .get(&candidate.user.id)
                .copied()
                .unwrap_or_default();
            let total_withdrawal_minor = withdrawal_totals
                .get(&candidate.user.id)
                .copied()
                .unwrap_or_default();
            let bet_profile = bet_profiles
                .remove(&candidate.user.id)
                .unwrap_or_else(DirectUserBetProfile::default);
            direct_users.push(UserInvitationDirectUser {
                id: candidate.user.id.clone(),
                username: candidate.user.username.clone(),
                status: candidate.user.status.clone(),
                invite_status: candidate.invite_status,
                rebate_enabled: candidate.rebate_enabled,
                available_balance_minor,
                total_deposit_minor,
                total_withdrawal_minor,
                total_bet_amount_minor: bet_profile.total_bet_amount_minor,
                bet_lottery_summaries: bet_profile.lottery_summary_items(),
                bet_play_summaries: bet_profile.play_summary_items(),
                latest_bet: bet_profile.latest_bet.clone(),
                registered_at: candidate.user.created_at.clone(),
                created_at: candidate.created_at,
            });
        }
        direct_users
    } else {
        Vec::new()
    };
    let total_direct_deposit_minor = sum_direct_deposit_minor(&direct_users)?;
    let active_direct_count = direct_users
        .iter()
        .filter(|user| {
            matches!(user.invite_status, InviteStatus::Active)
                && matches!(user.status, UserStatus::Active)
        })
        .count();

    Ok(Json(ApiEnvelope::success(UserInvitationSummaryResponse {
        can_invite,
        invitation_code: session.user.invite_code,
        direct_count: direct_users.len(),
        active_direct_count,
        total_direct_deposit_minor,
        total_paid_commission_minor: user_recharge_rebate_minor(&state, &session.user.id).await?,
        rebate_mode: policy.rebate_mode,
        default_recharge_rebate_basis_points: policy.default_recharge_rebate_basis_points,
        direct_users,
    })))
}

/// 返回当前用户最近一次代理申请状态，供手机端代理中心展示审核进度。
async fn get_user_agent_application(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserAgentApplicationResponse>>> {
    let application = state
        .agent_applications
        .latest_for_user(&session.user.id)
        .await?;

    Ok(Json(ApiEnvelope::success(UserAgentApplicationResponse {
        application,
    })))
}

/// 当前用户提交代理申请，审核通过后后台会把账号升级为代理。
async fn submit_user_agent_application(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<SubmitAgentApplicationRequest>,
) -> ApiResult<Json<ApiEnvelope<AgentApplication>>> {
    let user = state.access.get_user(&session.user.id).await?;
    let application = state.agent_applications.submit(&user, payload).await?;

    Ok(Json(ApiEnvelope::success(application)))
}

/// 邀请中心内部直属用户候选项，统一承载来源用户、关系状态和创建时间。
#[derive(Clone)]
struct DirectInvitationCandidate {
    user: UserSummary,
    invite_status: InviteStatus,
    rebate_enabled: bool,
    created_at: String,
}

/// 判断当前用户是否拥有可对外使用的邀请码权限。
fn user_can_invite(user: &UserSummary, policy: &InvitePolicySummary) -> bool {
    matches!(user.kind, UserKind::Agent) && policy.agents_can_invite
}

/// 合并后台邀请记录和注册时绑定的代理关系，形成手机端邀请中心直属用户列表。
fn collect_direct_invitation_candidates(
    current_user_id: &str,
    users: &[UserSummary],
    invite_records: &[InviteRecord],
    default_rebate_enabled: bool,
) -> Vec<DirectInvitationCandidate> {
    let users_by_id: HashMap<&str, &UserSummary> =
        users.iter().map(|user| (user.id.as_str(), user)).collect();
    let mut candidates: BTreeMap<String, DirectInvitationCandidate> = BTreeMap::new();

    for record in invite_records
        .iter()
        .filter(|record| record.inviter_user_id == current_user_id)
    {
        if let Some(user) = users_by_id.get(record.invitee_user_id.as_str()) {
            candidates.insert(
                user.id.clone(),
                DirectInvitationCandidate {
                    user: (*user).to_owned(),
                    invite_status: record.status.clone(),
                    rebate_enabled: record.rebate_enabled,
                    created_at: record.created_at.clone(),
                },
            );
        }
    }

    for user in users
        .iter()
        .filter(|user| user.agent_id.as_deref() == Some(current_user_id))
    {
        candidates
            .entry(user.id.clone())
            .or_insert_with(|| DirectInvitationCandidate {
                user: user.clone(),
                invite_status: InviteStatus::Active,
                rebate_enabled: default_rebate_enabled,
                created_at: String::new(),
            });
    }

    candidates.into_values().collect()
}

#[derive(Debug, Clone)]
/// 邀请中心下级单笔投注输入，统一承载普通注单和合买认购记录。
struct DirectUserBetInput {
    order_id: String,
    lottery_id: String,
    lottery_name: String,
    issue: String,
    rule_code: String,
    play_name: String,
    amount_minor: i64,
    created_at: String,
}

#[derive(Debug, Clone, Default)]
/// 邀请中心下级投注画像聚合结果，按用户累计总金额、彩种、玩法和最近投注。
struct DirectUserBetProfile {
    total_bet_amount_minor: i64,
    lottery_summaries: BTreeMap<String, DirectUserBetLotteryAccumulator>,
    play_summaries: BTreeMap<String, DirectUserBetPlayAccumulator>,
    latest_bet: Option<UserInvitationLatestBet>,
}

impl DirectUserBetProfile {
    /// 纳入一笔下级投注，正向金额才进入统计并同步刷新最近投注。
    fn add_bet(&mut self, bet: DirectUserBetInput) -> ApiResult<()> {
        if bet.amount_minor <= 0 {
            return Ok(());
        }

        self.total_bet_amount_minor = self
            .total_bet_amount_minor
            .checked_add(bet.amount_minor)
            .ok_or_else(|| ApiError::Internal("直属用户投注汇总金额溢出".to_string()))?;

        let lottery_entry = self
            .lottery_summaries
            .entry(bet.lottery_id.clone())
            .or_insert_with(|| DirectUserBetLotteryAccumulator {
                lottery_id: bet.lottery_id.clone(),
                lottery_name: bet.lottery_name.clone(),
                amount_minor: 0,
                order_count: 0,
            });
        lottery_entry.amount_minor = lottery_entry
            .amount_minor
            .checked_add(bet.amount_minor)
            .ok_or_else(|| ApiError::Internal("直属用户彩种投注金额溢出".to_string()))?;
        lottery_entry.order_count = lottery_entry.order_count.saturating_add(1);

        let play_key = format!("{}::{}", bet.lottery_id, bet.rule_code);
        let play_entry =
            self.play_summaries
                .entry(play_key)
                .or_insert_with(|| DirectUserBetPlayAccumulator {
                    lottery_id: bet.lottery_id.clone(),
                    lottery_name: bet.lottery_name.clone(),
                    rule_code: bet.rule_code.clone(),
                    play_name: bet.play_name.clone(),
                    amount_minor: 0,
                    order_count: 0,
                });
        play_entry.amount_minor = play_entry
            .amount_minor
            .checked_add(bet.amount_minor)
            .ok_or_else(|| ApiError::Internal("直属用户玩法投注金额溢出".to_string()))?;
        play_entry.order_count = play_entry.order_count.saturating_add(1);

        let latest_bet = UserInvitationLatestBet {
            order_id: bet.order_id,
            lottery_id: bet.lottery_id,
            lottery_name: bet.lottery_name,
            issue: bet.issue,
            rule_code: bet.rule_code,
            play_name: bet.play_name,
            amount_minor: bet.amount_minor,
            created_at: bet.created_at,
        };
        if self
            .latest_bet
            .as_ref()
            .map(|current| {
                latest_bet.created_at > current.created_at
                    || (latest_bet.created_at == current.created_at
                        && latest_bet.order_id > current.order_id)
            })
            .unwrap_or(true)
        {
            self.latest_bet = Some(latest_bet);
        }

        Ok(())
    }

    /// 转成按投注金额倒序的彩种汇总，便于手机端直接展示。
    fn lottery_summary_items(&self) -> Vec<UserInvitationBetLotterySummary> {
        let mut items = self
            .lottery_summaries
            .values()
            .map(DirectUserBetLotteryAccumulator::summary)
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .amount_minor
                .cmp(&left.amount_minor)
                .then_with(|| right.order_count.cmp(&left.order_count))
                .then_with(|| left.lottery_name.cmp(&right.lottery_name))
        });
        items
    }

    /// 转成按投注金额倒序的玩法汇总，便于手机端查看买了什么玩法。
    fn play_summary_items(&self) -> Vec<UserInvitationBetPlaySummary> {
        let mut items = self
            .play_summaries
            .values()
            .map(DirectUserBetPlayAccumulator::summary)
            .collect::<Vec<_>>();
        items.sort_by(|left, right| {
            right
                .amount_minor
                .cmp(&left.amount_minor)
                .then_with(|| right.order_count.cmp(&left.order_count))
                .then_with(|| left.lottery_name.cmp(&right.lottery_name))
                .then_with(|| left.play_name.cmp(&right.play_name))
        });
        items
    }
}

#[derive(Debug, Clone)]
/// 下级彩种投注金额累加器，内部使用后再转换为接口结构。
struct DirectUserBetLotteryAccumulator {
    lottery_id: String,
    lottery_name: String,
    amount_minor: i64,
    order_count: usize,
}

impl DirectUserBetLotteryAccumulator {
    /// 输出手机端接口使用的彩种投注汇总。
    fn summary(&self) -> UserInvitationBetLotterySummary {
        UserInvitationBetLotterySummary {
            lottery_id: self.lottery_id.clone(),
            lottery_name: self.lottery_name.clone(),
            amount_minor: self.amount_minor,
            order_count: self.order_count,
        }
    }
}

#[derive(Debug, Clone)]
/// 下级玩法投注金额累加器，内部使用后再转换为接口结构。
struct DirectUserBetPlayAccumulator {
    lottery_id: String,
    lottery_name: String,
    rule_code: String,
    play_name: String,
    amount_minor: i64,
    order_count: usize,
}

impl DirectUserBetPlayAccumulator {
    /// 输出手机端接口使用的玩法投注汇总。
    fn summary(&self) -> UserInvitationBetPlaySummary {
        UserInvitationBetPlaySummary {
            lottery_id: self.lottery_id.clone(),
            lottery_name: self.lottery_name.clone(),
            rule_code: self.rule_code.clone(),
            play_name: self.play_name.clone(),
            amount_minor: self.amount_minor,
            order_count: self.order_count,
        }
    }
}

/// 一次性汇总直属下级充值入账流水，避免代理中心读取无关用户资金流水。
async fn direct_user_recharge_totals(
    state: &AppState,
    direct_user_ids: &[String],
) -> ApiResult<BTreeMap<String, i64>> {
    state
        .finance
        .recharge_credit_totals_for_user_ids(direct_user_ids)
        .await
}

/// 批量读取直属下级可用余额，避免代理中心为每个下级单独查询资金账户。
async fn direct_user_available_balances(
    state: &AppState,
    direct_user_ids: &[String],
) -> ApiResult<BTreeMap<String, i64>> {
    let accounts = state.finance.accounts_for_user_ids(direct_user_ids).await?;
    Ok(accounts
        .into_iter()
        .map(|account| (account.user_id, account.available_balance_minor))
        .collect())
}

/// 汇总直属下级投注画像，普通下注按注单统计，合买按参与记录统计。
async fn direct_user_bet_profiles(
    state: &AppState,
    direct_user_ids: &[String],
) -> ApiResult<BTreeMap<String, DirectUserBetProfile>> {
    if direct_user_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let play_labels = play_rule_label_map()?;
    let mut profiles: BTreeMap<String, DirectUserBetProfile> = BTreeMap::new();
    let direct_user_id_set = direct_user_ids.iter().cloned().collect::<BTreeSet<_>>();

    for order in state
        .orders
        .list_direct_positive_for_user_ids(direct_user_ids)
        .await?
    {
        let rule_code = enum_to_string(&order.rule_code)?;
        let play_name = play_rule_label(&play_labels, &rule_code);
        profiles
            .entry(order.user_id.clone())
            .or_default()
            .add_bet(DirectUserBetInput {
                order_id: order.id,
                lottery_id: order.lottery_id,
                lottery_name: order.lottery_name,
                issue: order.issue,
                rule_code,
                play_name,
                amount_minor: order.amount_minor,
                created_at: order.created_at,
            })?;
    }

    for plan in state
        .group_buys
        .list_active_details_for_participant_user_ids(direct_user_ids)
        .await?
        .into_iter()
    {
        let plan_order_id = plan.order_id.clone().unwrap_or_else(|| plan.id.clone());
        let lottery_id = plan.lottery_id.clone();
        let lottery_name = plan.lottery_name.clone();
        let issue = plan.issue.clone();
        let rule_code = plan.rule_code.clone();
        let play_name = play_rule_label(&play_labels, &rule_code);
        for participant in plan.participants.into_iter().filter(|participant| {
            direct_user_id_set.contains(&participant.user_id) && participant.amount_minor > 0
        }) {
            profiles
                .entry(participant.user_id.clone())
                .or_default()
                .add_bet(DirectUserBetInput {
                    order_id: format!("{}:{}", plan_order_id, participant.id),
                    lottery_id: lottery_id.clone(),
                    lottery_name: lottery_name.clone(),
                    issue: issue.clone(),
                    rule_code: rule_code.clone(),
                    play_name: play_name.clone(),
                    amount_minor: participant.amount_minor,
                    created_at: participant.created_at,
                })?;
        }
    }

    Ok(profiles)
}

/// 构造玩法编码到中文玩法名的映射，供邀请中心投注画像复用。
fn play_rule_label_map() -> ApiResult<HashMap<String, String>> {
    let mut labels = HashMap::new();
    for summary in play_rule_summaries() {
        labels.insert(enum_to_string(&summary.code)?, summary.label);
    }
    Ok(labels)
}

/// 根据玩法编码返回中文名；未知编码保留原始编码便于排查配置问题。
fn play_rule_label(labels: &HashMap<String, String>, rule_code: &str) -> String {
    labels
        .get(rule_code)
        .cloned()
        .unwrap_or_else(|| rule_code.to_string())
}

/// 汇总全部已通过提现订单，供邀请中心展示直属下级提现金额。
async fn direct_user_withdrawal_totals(state: &AppState) -> ApiResult<BTreeMap<String, i64>> {
    let orders = state.withdrawals.approved_orders().await?;
    sum_approved_withdrawal_minor_by_user(&orders)
}

/// 按用户汇总已通过提现金额，忽略异常的非正数金额并保护溢出。
fn sum_approved_withdrawal_minor_by_user(
    orders: &[WithdrawalOrderSummary],
) -> ApiResult<BTreeMap<String, i64>> {
    let mut totals = BTreeMap::new();
    for order in orders.iter().filter(|order| order.amount_minor > 0) {
        let current = totals.entry(order.user_id.clone()).or_insert(0_i64);
        let next = current
            .checked_add(order.amount_minor)
            .ok_or_else(|| ApiError::Internal("直属用户提现汇总金额溢出".to_string()))?;
        *current = next;
    }
    Ok(totals)
}

/// 汇总充值入账流水，忽略非充值流水和异常的负数充值记录。
#[cfg(test)]
fn sum_recharge_credits_minor(entries: &[LedgerEntry]) -> ApiResult<i64> {
    entries
        .iter()
        .filter(|entry| {
            matches!(entry.kind, LedgerEntryKind::RechargeCredit) && entry.amount_minor > 0
        })
        .try_fold(0_i64, |total, entry| {
            total
                .checked_add(entry.amount_minor)
                .ok_or_else(|| ApiError::Internal("直属用户充值汇总金额溢出".to_string()))
        })
}

/// 统计当前代理已真实入账的充值返利流水。
async fn user_recharge_rebate_minor(state: &AppState, user_id: &str) -> ApiResult<i64> {
    let entries = state.finance.user_ledger_entries(user_id).await?;
    sum_recharge_rebate_credits_minor(&entries)
}

/// 汇总正向充值返利流水，作为邀请中心“已结算返利”来源。
fn sum_recharge_rebate_credits_minor(entries: &[LedgerEntry]) -> ApiResult<i64> {
    entries
        .iter()
        .filter(|entry| {
            matches!(entry.kind, LedgerEntryKind::RechargeRebateCredit) && entry.amount_minor > 0
        })
        .try_fold(0_i64, |total, entry| {
            total
                .checked_add(entry.amount_minor)
                .ok_or_else(|| ApiError::Internal("邀请返利汇总金额溢出".to_string()))
        })
}

/// 汇总邀请中心直属充值金额，避免金额字段溢出后继续返回错误数据。
fn sum_direct_deposit_minor(direct_users: &[UserInvitationDirectUser]) -> ApiResult<i64> {
    direct_users.iter().try_fold(0_i64, |total, user| {
        total
            .checked_add(user.total_deposit_minor)
            .ok_or_else(|| ApiError::Internal("邀请中心直属充值汇总金额溢出".to_string()))
    })
}

/// 返回手机端下注页所需的彩种、期号和玩法配置。
async fn get_user_bet_page_config(
    State(state): State<AppState>,
    Path(lottery_id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<MobileBetPageConfig>>> {
    let lottery = state.lotteries.get(&lottery_id).await?;
    if !lottery.sale_enabled {
        return Err(ApiError::BadRequest("彩种已停售".to_string()));
    }
    let issues = state
        .draws
        .list_page(
            Some(&lottery.id),
            None,
            PageRequest::new(Some(1), Some(120)),
        )
        .await?
        .items;
    let config = build_mobile_bet_page_config(&lottery, issues);

    Ok(Json(ApiEnvelope::success(config)))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 用户端注单列表响应，合买订单会额外带出当前用户的参与金额和个人派奖金额。
struct UserBetOrderDetailResponse {
    #[serde(flatten)]
    order: OrderDetail,
    #[serde(skip_serializing_if = "Option::is_none")]
    group_buy_plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    group_buy_plan_status: Option<GroupBuyPlanStatus>,
    #[serde(skip_serializing_if = "is_false")]
    group_buy_pending_plan: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    participation_amount_minor: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    participation_share_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    participation_payout_minor: Option<i64>,
}

/// 判断布尔值是否为 false，供 serde 跳过默认 false 字段。
fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone)]
/// 当前用户在一张合买真实订单里的个人份额，供注单列表展示使用。
struct UserGroupBuyOrderShare {
    plan_id: String,
    plan_status: GroupBuyPlanStatus,
    amount_minor: i64,
    share_count: u32,
    payout_minor: Option<i64>,
}

/// 返回当前用户可见的注单列表。
async fn list_user_bet_orders(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Query(query): Query<UserPageQuery>,
) -> ApiResult<Json<ApiEnvelope<Vec<UserBetOrderDetailResponse>>>> {
    if matches!(query.view, Some(UserBetOrderView::GroupBuy)) {
        let lotteries = state.lotteries.list().await?;
        let plans = state
            .group_buys
            .list_unformed_details_for_user_page(
                &session.user.id,
                PageRequest::new(query.page, query.page_size),
            )
            .await?
            .items;
        let mut orders = Vec::new();
        for plan in &plans {
            if let Some(order) =
                unformed_group_buy_order_response(&session.user.id, plan, &lotteries)?
            {
                orders.push(order);
            }
        }

        return Ok(Json(ApiEnvelope::success(orders)));
    }

    let group_buy_order_ids = state
        .group_buys
        .order_ids_for_user(&session.user.id)
        .await?;
    let orders = state
        .orders
        .list_user_visible_page(
            &session.user.id,
            &group_buy_order_ids,
            PageRequest::new(query.page, query.page_size),
        )
        .await?
        .items;
    let order_ids = orders
        .iter()
        .map(|order| order.id.clone())
        .collect::<Vec<_>>();
    let group_buy_plans = state.group_buys.plans_for_order_ids(&order_ids).await?;
    let lotteries = state.lotteries.list().await?;
    let ledger_entries = state
        .finance
        .ledger_entry_kind_page(
            Some(&session.user.id),
            &[LedgerEntryKind::PayoutCredit],
            PageRequest::default(),
        )
        .await?
        .items;
    let orders = user_visible_bet_orders(
        &session.user.id,
        orders,
        &group_buy_plans,
        &ledger_entries,
        &lotteries,
    )?;

    Ok(Json(ApiEnvelope::success(filter_user_bet_orders_by_view(
        orders, query.view,
    ))))
}

/// 合并本人独立下注订单、本人参与且已经成单的合买投注订单，以及尚未生成真实订单的合买认购。
fn user_visible_bet_orders(
    user_id: &str,
    orders: Vec<OrderDetail>,
    group_buy_plans: &[GroupBuyPlan],
    ledger_entries: &[LedgerEntry],
    lotteries: &[LotteryKind],
) -> ApiResult<Vec<UserBetOrderDetailResponse>> {
    let mut visible_orders = Vec::new();
    for order in orders {
        let group_buy_share = if order.order_source == OrderSource::GroupBuy {
            user_group_buy_order_share(user_id, &order, group_buy_plans, ledger_entries)?
        } else {
            None
        };
        if order.user_id == user_id || group_buy_share.is_some() {
            let group_buy_plan_id = group_buy_share.as_ref().map(|share| share.plan_id.clone());
            let group_buy_plan_status = group_buy_share
                .as_ref()
                .map(|share| share.plan_status.clone());
            let participation_amount_minor =
                group_buy_share.as_ref().map(|share| share.amount_minor);
            let participation_share_count = group_buy_share.as_ref().map(|share| share.share_count);
            let participation_payout_minor = group_buy_share.and_then(|share| share.payout_minor);
            visible_orders.push(UserBetOrderDetailResponse {
                order,
                group_buy_plan_id,
                group_buy_plan_status,
                group_buy_pending_plan: false,
                participation_amount_minor,
                participation_share_count,
                participation_payout_minor,
            });
        }
    }

    for plan in group_buy_plans
        .iter()
        .filter(|plan| plan.order_id.is_none())
    {
        if let Some(order) = unformed_group_buy_order_response(user_id, plan, lotteries)? {
            visible_orders.push(order);
        }
    }

    sort_user_bet_order_responses_by_created_at_desc(&mut visible_orders);

    Ok(visible_orders)
}

/// 将尚未满单成单的合买认购映射为用户端注单时间线里的特殊记录。
fn unformed_group_buy_order_response(
    user_id: &str,
    plan: &GroupBuyPlan,
    lotteries: &[LotteryKind],
) -> ApiResult<Option<UserBetOrderDetailResponse>> {
    if !plan
        .participants
        .iter()
        .any(|participant| participant.user_id == user_id)
    {
        return Ok(None);
    }

    let rule_code = parse_group_buy_rule_code(&plan.rule_code)?;
    let selection = parse_group_buy_selection(&rule_code, &plan.numbers)?;
    let expanded_bets = expanded_bets_for_rule(&rule_code, &selection)?;
    let stake_count = u32::try_from(expanded_bets.len())
        .map_err(|_| ApiError::Internal("合买注数超过手机端展示范围".to_string()))?;
    let participation_amount_minor = group_buy_user_participation_amount_minor(plan, user_id)?;
    let participation_share_count = group_buy_user_participation_share_count(plan, user_id)?;
    let created_at = group_buy_user_latest_participation_at(plan, user_id)
        .unwrap_or_else(|| plan.created_at.clone());
    let status = if plan.status == GroupBuyPlanStatus::Cancelled {
        OrderStatus::Cancelled
    } else {
        OrderStatus::PendingDraw
    };

    Ok(Some(UserBetOrderDetailResponse {
        order: OrderDetail {
            id: format!("GB-{}", plan.id),
            order_source: OrderSource::GroupBuy,
            user_id: user_id.to_string(),
            lottery_id: plan.lottery_id.clone(),
            lottery_name: plan.lottery_name.clone(),
            issue: plan.issue.clone(),
            rule_code,
            number_type: group_buy_plan_number_type(plan, lotteries),
            selection,
            stake_count,
            unit_amount_minor: plan.min_share_amount_minor,
            amount_minor: plan.total_amount_minor,
            odds_basis_points: 0,
            expanded_bets,
            draw_number: None,
            matched_bets: Vec::new(),
            payout_minor: 0,
            status,
            settled_at: None,
            created_at,
        },
        group_buy_plan_id: Some(plan.id.clone()),
        group_buy_plan_status: Some(plan.status.clone()),
        group_buy_pending_plan: plan.status != GroupBuyPlanStatus::Cancelled,
        participation_amount_minor: Some(participation_amount_minor),
        participation_share_count: Some(participation_share_count),
        participation_payout_minor: None,
    }))
}

/// 按创建时间倒序稳定排列用户端注单，保证未成单合买和真实订单处于同一时间线。
fn sort_user_bet_order_responses_by_created_at_desc(items: &mut [UserBetOrderDetailResponse]) {
    items.sort_by(|left, right| {
        right
            .order
            .created_at
            .cmp(&left.order.created_at)
            .then_with(|| right.order.id.cmp(&left.order.id))
    });
}

/// 根据手机端 Tab 过滤注单：真实已下单订单和未成单合买认购分别分页。
fn filter_user_bet_orders_by_view(
    items: Vec<UserBetOrderDetailResponse>,
    view: Option<UserBetOrderView>,
) -> Vec<UserBetOrderDetailResponse> {
    match view {
        Some(UserBetOrderView::Orders) => items
            .into_iter()
            .filter(|item| !is_unformed_group_buy_order(item))
            .collect(),
        Some(UserBetOrderView::GroupBuy) => items
            .into_iter()
            .filter(is_unformed_group_buy_order)
            .collect(),
        None => items,
    }
}

/// 判断是否是尚未形成真实投注订单的合买认购记录，取消的未成单计划也归入“我的合买”。
fn is_unformed_group_buy_order(item: &UserBetOrderDetailResponse) -> bool {
    item.order.order_source == OrderSource::GroupBuy
        && item.group_buy_plan_id.is_some()
        && (item.group_buy_pending_plan || item.order.id.starts_with("GB-"))
}

/// 获取合买计划对应彩种号码类型；彩种配置缺失时按玩法编码做保守推断。
fn group_buy_plan_number_type(plan: &GroupBuyPlan, lotteries: &[LotteryKind]) -> LotteryNumberType {
    lotteries
        .iter()
        .find(|lottery| lottery.id == plan.lottery_id)
        .map(|lottery| lottery.number_type.clone())
        .unwrap_or_else(|| {
            if plan.rule_code.starts_with("five") {
                LotteryNumberType::FiveDigit
            } else {
                LotteryNumberType::ThreeDigit
            }
        })
}

/// 计算当前用户在合买订单中的展示份额，优先使用真实资金流水里的派奖记录。
fn user_group_buy_order_share(
    user_id: &str,
    order: &OrderDetail,
    group_buy_plans: &[GroupBuyPlan],
    ledger_entries: &[LedgerEntry],
) -> ApiResult<Option<UserGroupBuyOrderShare>> {
    let Some(plan) = group_buy_plans
        .iter()
        .find(|plan| plan.order_id.as_deref() == Some(order.id.as_str()))
    else {
        return Ok(None);
    };
    if !plan
        .participants
        .iter()
        .any(|participant| participant.user_id == user_id)
    {
        return Ok(None);
    }

    let amount_minor = group_buy_user_participation_amount_minor(plan, user_id)?;
    let share_count = group_buy_user_participation_share_count(plan, user_id)?;
    let payout_minor = group_buy_user_participation_payout_minor(
        plan,
        user_id,
        &order.id,
        order.payout_minor,
        ledger_entries,
    )?;
    Ok(Some(UserGroupBuyOrderShare {
        plan_id: plan.id.clone(),
        plan_status: plan.status.clone(),
        amount_minor,
        share_count,
        payout_minor,
    }))
}

/// 汇总用户在同一合买计划里的多次认购金额。
fn group_buy_user_participation_amount_minor(plan: &GroupBuyPlan, user_id: &str) -> ApiResult<i64> {
    plan.participants
        .iter()
        .filter(|participant| participant.user_id == user_id)
        .try_fold(0_i64, |sum, participant| {
            sum.checked_add(participant.amount_minor)
                .ok_or_else(|| ApiError::Internal("合买参与金额汇总溢出".to_string()))
        })
}

/// 汇总用户在同一合买计划里的多次认购份数。
fn group_buy_user_participation_share_count(plan: &GroupBuyPlan, user_id: &str) -> ApiResult<u32> {
    plan.participants
        .iter()
        .filter(|participant| participant.user_id == user_id)
        .try_fold(0_u32, |sum, participant| {
            sum.checked_add(participant.share_count)
                .ok_or_else(|| ApiError::Internal("合买参与份数汇总溢出".to_string()))
        })
}

/// 返回用户在某个合买计划里最后一次认购时间，用于我的注单时间线排序。
fn group_buy_user_latest_participation_at(plan: &GroupBuyPlan, user_id: &str) -> Option<String> {
    plan.participants
        .iter()
        .filter(|participant| participant.user_id == user_id)
        .map(|participant| participant.created_at.clone())
        .max()
}

/// 计算用户个人派奖金额，避免手机端把整张合买订单的奖金展示给每个参与人。
fn group_buy_user_participation_payout_minor(
    plan: &GroupBuyPlan,
    user_id: &str,
    order_id: &str,
    order_payout_minor: i64,
    ledger_entries: &[LedgerEntry],
) -> ApiResult<Option<i64>> {
    if order_payout_minor <= 0 {
        return Ok(None);
    }
    if let Some(ledger_payout_minor) =
        group_buy_user_payout_from_ledger(plan, user_id, order_id, ledger_entries)?
    {
        return Ok(Some(ledger_payout_minor));
    }

    Ok(Some(calculated_group_buy_user_payout_minor(
        plan,
        user_id,
        order_payout_minor,
    )?))
}

/// 从真实派奖流水里读取用户个人分账金额，保证展示金额与账户实际入账一致。
fn group_buy_user_payout_from_ledger(
    plan: &GroupBuyPlan,
    user_id: &str,
    order_id: &str,
    ledger_entries: &[LedgerEntry],
) -> ApiResult<Option<i64>> {
    let participant_ids = plan
        .participants
        .iter()
        .filter(|participant| participant.user_id == user_id)
        .map(|participant| participant.id.as_str())
        .collect::<Vec<_>>();
    let payout_minor = ledger_entries
        .iter()
        .filter(|entry| entry.kind == LedgerEntryKind::PayoutCredit)
        .filter(|entry| entry.user_id == user_id)
        .filter(|entry| {
            participant_ids.iter().any(|participant_id| {
                entry
                    .reference_id
                    .as_deref()
                    .map(|reference_id| {
                        group_buy_payout_reference_matches(reference_id, order_id, participant_id)
                    })
                    .unwrap_or(false)
            })
        })
        .try_fold(0_i64, |sum, entry| {
            sum.checked_add(entry.amount_minor)
                .ok_or_else(|| ApiError::Internal("合买派奖金额汇总溢出".to_string()))
        })?;

    if payout_minor > 0 {
        Ok(Some(payout_minor))
    } else {
        Ok(None)
    }
}

/// 识别合买派奖流水引用，格式为“结算批次:订单号:参与编号”。
fn group_buy_payout_reference_matches(
    reference_id: &str,
    order_id: &str,
    participant_id: &str,
) -> bool {
    let mut parts = reference_id.split(':');
    let _settlement_id = parts.next();
    let reference_order_id = parts.next();
    let reference_participant_id = parts.next();
    reference_order_id == Some(order_id)
        && reference_participant_id == Some(participant_id)
        && parts.next().is_none()
}

/// 没有历史派奖流水时，按财务服务相同的比例和余数规则计算个人展示金额。
fn calculated_group_buy_user_payout_minor(
    plan: &GroupBuyPlan,
    user_id: &str,
    order_payout_minor: i64,
) -> ApiResult<i64> {
    if plan.total_amount_minor <= 0 {
        return Err(ApiError::BadRequest("合买总金额无效".to_string()));
    }
    let participants = plan
        .participants
        .iter()
        .filter(|participant| participant.amount_minor > 0)
        .collect::<Vec<_>>();
    if participants.is_empty() {
        return Err(ApiError::BadRequest("合买计划没有可派奖参与人".to_string()));
    }

    let mut remaining_payout = order_payout_minor;
    let mut user_payout = 0_i64;
    let participant_count = participants.len();
    for (index, participant) in participants.into_iter().enumerate() {
        let payout_minor = if index + 1 == participant_count {
            remaining_payout
        } else {
            proportional_minor(
                order_payout_minor,
                participant.amount_minor,
                plan.total_amount_minor,
            )?
        };
        remaining_payout = remaining_payout
            .checked_sub(payout_minor)
            .ok_or_else(|| ApiError::BadRequest("合买派奖金额过大".to_string()))?;
        if participant.user_id == user_id {
            user_payout = user_payout
                .checked_add(payout_minor)
                .ok_or_else(|| ApiError::Internal("合买个人派奖金额汇总溢出".to_string()))?;
        }
    }

    Ok(user_payout)
}

/// 按比例计算最小货币单位金额，和财务分账逻辑保持一致。
fn proportional_minor(total_minor: i64, part_minor: i64, base_minor: i64) -> ApiResult<i64> {
    if total_minor < 0 || part_minor < 0 || base_minor <= 0 {
        return Err(ApiError::BadRequest("合买派奖比例金额无效".to_string()));
    }
    total_minor
        .checked_mul(part_minor)
        .map(|amount| amount / base_minor)
        .ok_or_else(|| ApiError::BadRequest("合买派奖金额过大".to_string()))
}

/// 用户端批量提交购彩篮订单并扣款。
async fn create_user_bet_orders(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<MobileCreateBetOrderBatchRequest>,
) -> ApiResult<Json<ApiEnvelope<MobileCreateBetOrderBatchResponse>>> {
    if payload.orders.is_empty() {
        return Err(ApiError::BadRequest("请先选择投注内容".to_string()));
    }
    if payload.orders.len() > MAX_USER_BET_BATCH_SIZE {
        return Err(ApiError::BadRequest(format!(
            "一次最多提交 {MAX_USER_BET_BATCH_SIZE} 笔投注"
        )));
    }

    let mut checked_orders = Vec::with_capacity(payload.orders.len());
    let mut lottery_cache: BTreeMap<String, LotteryKind> = BTreeMap::new();
    let mut draw_issue_cache: BTreeMap<(String, String), DrawIssue> = BTreeMap::new();
    for item in payload.orders {
        let order_payload = CreateOrderRequest {
            user_id: session.user.id.clone(),
            lottery_id: item.lottery_id,
            issue: item.issue,
            rule_code: item.rule_code,
            selection: item.selection,
            unit_amount_minor: item.unit_amount_minor,
        };
        let lottery = match lottery_cache.get(&order_payload.lottery_id) {
            Some(lottery) => lottery.clone(),
            None => {
                let lottery = state.lotteries.get(&order_payload.lottery_id).await?;
                lottery_cache.insert(order_payload.lottery_id.clone(), lottery.clone());
                lottery
            }
        };
        let draw_issue_key = (
            order_payload.lottery_id.clone(),
            order_payload.issue.clone(),
        );
        let draw_issue = match draw_issue_cache.get(&draw_issue_key) {
            Some(draw_issue) => draw_issue.clone(),
            None => {
                let draw_issue = state
                    .draws
                    .get_by_lottery_issue(&order_payload.lottery_id, &order_payload.issue)
                    .await?;
                draw_issue_cache.insert(draw_issue_key, draw_issue.clone());
                draw_issue
            }
        };
        validate_draw_issue_accepts_order(&draw_issue, &lottery, &order_payload.issue)?;
        let quote = state.orders.quote(&lottery, &order_payload).await?;
        if quote.amount_minor <= 0 {
            return Err(ApiError::BadRequest("投注金额必须大于 0".to_string()));
        }
        checked_orders.push((lottery, order_payload));
    }

    let redis_lock = state
        .redis
        .acquire_lock(format!("lock:user:{}:bet", session.user.id))
        .await?;
    let created_orders_result = state
        .orders
        .create_many_with_debit(&state.finance, checked_orders, OrderSource::Direct)
        .await;
    if let Some(lock) = redis_lock {
        if let Err(error) = lock.release().await {
            tracing::warn!(
                error = %error.log_message(),
                user_id = session.user.id.as_str(),
                "投注用户 Redis 锁释放失败"
            );
        }
    }
    let created_orders = created_orders_result?;
    if let Err(error) = state
        .redis
        .delete_keys(&[format!("user:{}:balance", session.user.id)])
        .await
    {
        tracing::warn!(
            error = %error.log_message(),
            user_id = session.user.id.as_str(),
            "用户余额缓存失效失败"
        );
    }
    for order in &created_orders {
        publish_user_order_changed(&state, &order, "created");
        publish_user_balance_changed(&state, &order.user_id, "order_debit", Some(&order.id)).await;
    }

    Ok(Json(ApiEnvelope::success(
        MobileCreateBetOrderBatchResponse {
            orders: created_orders,
        },
    )))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 手机端合买列表筛选参数。
struct UserGroupBuyListQuery {
    #[serde(default, alias = "lottery_code")]
    lottery_id: Option<String>,
    #[serde(default, alias = "group_code")]
    group_code: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

/// 返回手机端合买大厅计划列表。
async fn list_user_group_buy_plans(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Query(query): Query<UserGroupBuyListQuery>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyPlanPage>>> {
    let lotteries = state.lotteries.list().await?;
    let items = user_group_buy_plans(&state, &session.user.id, &lotteries, &query).await?;

    Ok(Json(ApiEnvelope::success(UserGroupBuyPlanPage { items })))
}

/// 返回手机端合买计划详情。
async fn get_user_group_buy_plan(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyPlan>>> {
    let lotteries = state.lotteries.list().await?;
    let access = state.access.snapshot().await?;
    let plan = state.group_buys.get(&id).await?;
    let plan = user_group_buy_plan(
        &plan,
        &lotteries,
        Some(&session.user.id),
        &access.users,
        true,
    )?;

    Ok(Json(ApiEnvelope::success(plan)))
}

/// 返回当前用户发起或参与的合买计划。
async fn list_my_group_buy_plans(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Query(query): Query<UserPageQuery>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyPlanPage>>> {
    let lotteries = state.lotteries.list().await?;
    let access = state.access.snapshot().await?;
    let mut items = state
        .group_buys
        .list_details_for_user(&session.user.id)
        .await?
        .into_iter()
        .map(|plan| {
            user_group_buy_plan(
                &plan,
                &lotteries,
                Some(&session.user.id),
                &access.users,
                false,
            )
        })
        .collect::<ApiResult<Vec<_>>>()?;
    sort_group_buy_plans_by_time_desc(&mut items);
    let items = query.paginate(items);

    Ok(Json(ApiEnvelope::success(UserGroupBuyPlanPage { items })))
}

/// 返回手机端发起合买所需的彩种、期号和玩法选项。
async fn get_user_group_buy_create_options(
    State(state): State<AppState>,
    Query(query): Query<UserGroupBuyListQuery>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyCreateOptions>>> {
    let lotteries = group_buy_enabled_lotteries(&state.lotteries.list().await?);
    let selected_lottery_id = query
        .lottery_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            lotteries
                .first()
                .map(|lottery| lottery.id.as_str())
                .unwrap_or("")
        });
    let selected_lottery = lotteries
        .iter()
        .find(|lottery| lottery.id == selected_lottery_id)
        .or_else(|| lotteries.first());
    let Some(selected_lottery) = selected_lottery else {
        return Ok(Json(ApiEnvelope::success(GroupBuyCreateOptions {
            lotteries: Vec::new(),
            issues: Vec::new(),
            plays: Vec::new(),
            settings: default_group_buy_create_settings(),
        })));
    };

    let issues = state
        .draws
        .list_by_lottery_id(&selected_lottery.id)
        .await?
        .into_iter()
        .filter(|issue| issue.status == DrawIssueStatus::Open)
        .map(|issue| GroupBuySelectOption {
            label: format!("第{}期", issue.issue),
            value: issue.issue,
        })
        .collect();
    let plays = enabled_group_buy_play_options(selected_lottery)?;

    Ok(Json(ApiEnvelope::success(GroupBuyCreateOptions {
        lotteries: lotteries
            .iter()
            .map(|lottery| GroupBuySelectOption {
                label: lottery.name.clone(),
                value: lottery.id.clone(),
            })
            .collect(),
        issues,
        plays,
        settings: GroupBuyCreateSettings {
            min_share_amount_minor: selected_lottery.group_buy.min_share_amount_minor,
            initiator_min_percent: selected_lottery.group_buy.initiator_min_percent,
            participant_min_amount_minor: selected_lottery.group_buy.participant_min_amount_minor,
        },
    })))
}

/// 手机端用户发起合买计划并完成自购扣款。
async fn create_user_group_buy_plan(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<UserCreateGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyActionResponse>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    validate_lottery_accepts_group_buy(&lottery)?;
    validate_group_buy_issue_and_play(&state, &lottery, &payload.issue, &payload.rule_code).await?;
    validate_group_buy_numbers(&payload.numbers)?;
    build_group_buy_order_request(
        &state.draws,
        &state.orders,
        &lottery,
        &session.user.id,
        &payload.issue,
        &payload.rule_code,
        &payload.numbers,
        payload.total_amount_minor,
    )
    .await?;
    state
        .finance
        .ensure_available(&session.user.id, payload.self_amount_minor)
        .await?;

    let plan_id = next_group_buy_plan_id();
    let participant_id = format!("{plan_id}-P001");
    let access = state.access.snapshot().await?;
    let request = CreateGroupBuyPlanRequest {
        id: plan_id.clone(),
        lottery_id: lottery.id.clone(),
        issue: payload.issue.trim().to_string(),
        rule_code: payload.rule_code.trim().to_string(),
        title: payload.title.trim().to_string(),
        numbers: payload.numbers.trim().to_string(),
        initiator_user_id: session.user.id.clone(),
        total_amount_minor: payload.total_amount_minor,
        initiator_amount_minor: payload.self_amount_minor,
        note: "用户发起合买".to_string(),
    };
    let mut plan = state
        .group_buys
        .create(request, std::slice::from_ref(&lottery), &access.users)
        .await?;
    let mut created_order = match create_order_for_filled_group_buy(
        &state.draws,
        &state.orders,
        &state.group_buys,
        &lottery,
        &plan,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            if let Err(rollback_error) = state.group_buys.remove_unfunded_plan(&plan.id).await {
                tracing::error!(
                    group_buy_plan_id = %plan.id,
                    error = %rollback_error.log_message(),
                    "合买满单成单失败后移除计划失败"
                );
            }
            return Err(error);
        }
    };
    if let Some((_, attached_plan)) = &created_order {
        plan = attached_plan.clone();
    }

    if let Err(error) = state
        .finance
        .debit_group_buy(
            &session.user.id,
            payload.self_amount_minor,
            &participant_id,
            &plan.id,
        )
        .await
    {
        if let Some((order, _)) = created_order.take() {
            if let Err(rollback_error) = state.orders.remove_unfunded(&order.id).await {
                tracing::error!(
                    order_id = %order.id,
                    error = %rollback_error.log_message(),
                    "合买发起扣款失败后移除满单订单失败"
                );
            }
        }
        if let Err(rollback_error) = state.group_buys.remove_unfunded_plan(&plan.id).await {
            tracing::error!(
                group_buy_plan_id = %plan.id,
                error = %rollback_error.log_message(),
                "合买发起扣款失败后移除计划失败"
            );
        }
        return Err(error);
    }
    if let Some((order, _)) = &created_order {
        publish_user_order_changed(&state, order, "created");
    }

    publish_user_balance_changed(
        &state,
        &session.user.id,
        "group_buy_debit",
        Some(&participant_id),
    )
    .await;
    let account = state.finance.account_or_create(&session.user.id).await?;
    let plan = user_group_buy_plan(
        &plan,
        &[lottery],
        Some(&session.user.id),
        &access.users,
        true,
    )?;

    Ok(Json(ApiEnvelope::success(UserGroupBuyActionResponse {
        plan,
        available_balance_minor: account.available_balance_minor,
    })))
}

/// 手机端用户认购合买计划并扣款。
async fn join_user_group_buy_plan(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
    Json(payload): Json<UserJoinGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyActionResponse>>> {
    let existing = state.group_buys.get(&id).await?;
    let lottery = state.lotteries.get(&existing.lottery_id).await?;
    validate_lottery_accepts_group_buy(&lottery)?;
    build_group_buy_order_request(
        &state.draws,
        &state.orders,
        &lottery,
        &existing.initiator_user_id,
        &existing.issue,
        &existing.rule_code,
        &existing.numbers,
        existing.total_amount_minor,
    )
    .await?;
    let participant_id = next_group_buy_participant_id(&existing);
    state
        .finance
        .ensure_available(&session.user.id, payload.amount_minor)
        .await?;
    let access = state.access.snapshot().await?;
    let mut updated = state
        .group_buys
        .add_participant(
            &id,
            AddGroupBuyParticipantRequest {
                id: participant_id.clone(),
                user_id: session.user.id.clone(),
                amount_minor: payload.amount_minor,
                note: "用户参与合买".to_string(),
            },
            &access.users,
        )
        .await?;
    let mut created_order = match create_order_for_filled_group_buy(
        &state.draws,
        &state.orders,
        &state.group_buys,
        &lottery,
        &updated,
    )
    .await
    {
        Ok(result) => result,
        Err(error) => {
            if let Err(rollback_error) = state
                .group_buys
                .remove_unfunded_participant(&id, &participant_id)
                .await
            {
                tracing::error!(
                    group_buy_plan_id = %id,
                    group_buy_participant_id = %participant_id,
                    error = %rollback_error.log_message(),
                    "合买满单成单失败后移除参与记录失败"
                );
            }
            return Err(error);
        }
    };
    if let Some((_, attached_plan)) = &created_order {
        updated = attached_plan.clone();
    }

    if let Err(error) = state
        .finance
        .debit_group_buy(&session.user.id, payload.amount_minor, &participant_id, &id)
        .await
    {
        if let Some((order, _)) = created_order.take() {
            if let Err(rollback_error) = state.orders.remove_unfunded(&order.id).await {
                tracing::error!(
                    order_id = %order.id,
                    error = %rollback_error.log_message(),
                    "合买参与扣款失败后移除满单订单失败"
                );
            }
        }
        if let Err(rollback_error) = state
            .group_buys
            .remove_unfunded_participant(&id, &participant_id)
            .await
        {
            tracing::error!(
                group_buy_plan_id = %id,
                group_buy_participant_id = %participant_id,
                error = %rollback_error.log_message(),
                "合买参与扣款失败后移除参与记录失败"
            );
        }
        return Err(error);
    }
    if let Some((order, _)) = &created_order {
        publish_user_order_changed(&state, order, "created");
    }

    publish_user_balance_changed(
        &state,
        &session.user.id,
        "group_buy_debit",
        Some(&participant_id),
    )
    .await;
    let account = state.finance.account_or_create(&session.user.id).await?;
    let lotteries = state.lotteries.list().await?;
    let plan = user_group_buy_plan(
        &updated,
        &lotteries,
        Some(&session.user.id),
        &access.users,
        true,
    )?;

    Ok(Json(ApiEnvelope::success(UserGroupBuyActionResponse {
        plan,
        available_balance_minor: account.available_balance_minor,
    })))
}

/// 把合买计划集合转换为手机端展示列表。
async fn user_group_buy_plans(
    state: &AppState,
    user_id: &str,
    lotteries: &[LotteryKind],
    query: &UserGroupBuyListQuery,
) -> ApiResult<Vec<UserGroupBuyPlan>> {
    let access = state.access.snapshot().await?;
    let lottery_id = query
        .lottery_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let group_code = query
        .group_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let lottery_ids = if let Some(lottery_id) = lottery_id {
        vec![lottery_id.to_string()]
    } else if let Some(group_code) = group_code {
        lotteries
            .iter()
            .filter(|lottery| lottery.category == group_code)
            .map(|lottery| lottery.id.clone())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let items = state
        .group_buys
        .list_active_details_page(&lottery_ids, PageRequest::new(query.page, query.page_size))
        .await?
        .items
        .into_iter()
        .map(|plan| user_group_buy_plan(&plan, lotteries, Some(user_id), &access.users, false))
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(items)
}

/// 把单个合买计划转换为手机端展示详情。
fn user_group_buy_plan(
    plan: &GroupBuyPlan,
    lotteries: &[LotteryKind],
    user_id: Option<&str>,
    users: &[UserSummary],
    include_participants: bool,
) -> ApiResult<UserGroupBuyPlan> {
    let lottery = lotteries
        .iter()
        .find(|lottery| lottery.id == plan.lottery_id);
    let sold_shares = amount_to_share_count(plan.filled_amount_minor, plan.min_share_amount_minor)?;
    let available_shares = plan.share_count.saturating_sub(sold_shares);
    let progress_percent = if plan.total_amount_minor <= 0 {
        0
    } else {
        ((plan.filled_amount_minor.saturating_mul(100)) / plan.total_amount_minor).clamp(0, 100)
            as u32
    };
    let my_participation = user_id.and_then(|user_id| user_group_buy_participation(plan, user_id));
    let play_name = play_rule_summaries()
        .into_iter()
        .find(|summary| enum_to_string(&summary.code).ok().as_deref() == Some(&plan.rule_code))
        .map(|summary| summary.label)
        .unwrap_or_else(|| plan.rule_code.clone());

    Ok(UserGroupBuyPlan {
        id: plan.id.clone(),
        lottery_id: plan.lottery_id.clone(),
        lottery_name: plan.lottery_name.clone(),
        order_id: plan.order_id.clone(),
        category: lottery.map(|lottery| lottery.category.clone()),
        issue: plan.issue.clone(),
        rule_code: plan.rule_code.clone(),
        play_name,
        title: user_group_buy_title(plan),
        numbers: plan.numbers.clone(),
        total_amount_minor: plan.total_amount_minor,
        share_count: plan.share_count,
        share_amount_minor: plan.min_share_amount_minor,
        participant_min_amount_minor: plan.participant_min_amount_minor,
        filled_amount_minor: plan.filled_amount_minor,
        sold_shares,
        available_shares,
        progress_percent,
        status: plan.status.clone(),
        participant_count: plan.participants.len(),
        participants: if include_participants {
            user_group_buy_participants(plan, user_id)
        } else {
            Vec::new()
        },
        initiator_display: user_group_buy_initiator_display(plan),
        initiator_avatar_url: user_group_buy_initiator_avatar_url(plan, users),
        my_participation,
        created_at: plan.created_at.clone(),
        updated_at: plan.updated_at.clone(),
    })
}

/// 生成手机端合买参与人展示列表，名称统一脱敏，当前用户标记为“我”。
fn user_group_buy_participants(
    plan: &GroupBuyPlan,
    user_id: Option<&str>,
) -> Vec<UserGroupBuyParticipantSummary> {
    plan.participants
        .iter()
        .map(|participant| {
            let is_mine = user_id
                .map(|user_id| participant.user_id == user_id)
                .unwrap_or(false);
            UserGroupBuyParticipantSummary {
                id: participant.id.clone(),
                display_name: user_group_buy_participant_display(participant, is_mine),
                amount_minor: participant.amount_minor,
                share_count: participant.share_count,
                is_mine,
                created_at: participant.created_at.clone(),
            }
        })
        .collect()
}

/// 生成单个合买参与人的用户端展示名，避免暴露机器人和完整用户名。
fn user_group_buy_participant_display(
    participant: &crate::domain::group_buy::GroupBuyParticipant,
    is_mine: bool,
) -> String {
    if is_mine {
        return "我".to_string();
    }
    if is_group_buy_robot_user_id(&participant.user_id) {
        let base_hash = stable_group_buy_display_hash(&participant.id);
        let base = ROBOT_GROUP_BUY_NICKNAME_BASES
            .get(base_hash as usize % ROBOT_GROUP_BUY_NICKNAME_BASES.len())
            .copied()
            .unwrap_or("南风");
        let suffix = base_hash % 9_000 + 1_000;
        return mask_group_buy_initiator_display(&format!("{base}{suffix}"));
    }

    mask_group_buy_initiator_display(&participant.username)
}

/// 生成手机端合买计划标题。
fn user_group_buy_title(plan: &GroupBuyPlan) -> String {
    if is_robot_group_buy_plan(plan) || plan.title.trim().is_empty() {
        format!("{} 第{}期合买", plan.lottery_name, plan.issue)
    } else {
        plan.title.clone()
    }
}

/// 生成合买发起人展示名，用户端统一只展示脱敏名称。
fn user_group_buy_initiator_display(plan: &GroupBuyPlan) -> String {
    let display_name = if is_robot_group_buy_plan(plan) {
        robot_group_buy_initiator_display(plan)
    } else {
        plan.initiator_username.clone()
    };

    mask_group_buy_initiator_display(&display_name)
}

/// 查询普通合买发起人的头像地址，机器人计划不返回真实机器人头像。
fn user_group_buy_initiator_avatar_url(plan: &GroupBuyPlan, users: &[UserSummary]) -> String {
    if is_robot_group_buy_plan(plan) {
        return String::new();
    }

    users
        .iter()
        .find(|user| user.id == plan.initiator_user_id)
        .map(|user| user.avatar_url.trim().to_string())
        .filter(|avatar_url| !avatar_url.is_empty())
        .unwrap_or_default()
}

/// 对合买大厅发起人名称做隐私脱敏，保留前四个字符并用星号替代剩余内容。
fn mask_group_buy_initiator_display(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return "会员".to_string();
    }

    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 4 {
        return chars.into_iter().collect();
    }

    let visible_prefix = chars.iter().take(4).collect::<String>();
    format!("{}{}", visible_prefix, "*".repeat(chars.len() - 4))
}

/// 判断合买计划是否由机器人账户发起。
fn is_robot_group_buy_plan(plan: &GroupBuyPlan) -> bool {
    plan.id.starts_with(ROBOT_GROUP_BUY_PLAN_PREFIX)
}

/// 为机器人合买计划生成稳定但不暴露机器人的展示名。
fn robot_group_buy_initiator_display(plan: &GroupBuyPlan) -> String {
    let base_hash = stable_group_buy_display_hash(&plan.id);
    let base = ROBOT_GROUP_BUY_NICKNAME_BASES
        .get(base_hash as usize % ROBOT_GROUP_BUY_NICKNAME_BASES.len())
        .copied()
        .unwrap_or("南风");
    let suffix_hash = stable_group_buy_display_hash(&format!("{}:{}", plan.id, plan.issue));
    let suffix = suffix_hash % 9_000 + 1_000;

    format!("{base}{suffix}")
}

/// 根据计划编号生成稳定哈希，保证匿名展示名可重复。
fn stable_group_buy_display_hash(value: &str) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

/// 汇总当前用户在合买计划中的参与金额和份数。
fn user_group_buy_participation(
    plan: &GroupBuyPlan,
    user_id: &str,
) -> Option<GroupBuyParticipationSummary> {
    let mut amount_minor = 0_i64;
    let mut share_count = 0_u32;
    for participant in plan
        .participants
        .iter()
        .filter(|participant| participant.user_id == user_id)
    {
        amount_minor = amount_minor.saturating_add(participant.amount_minor);
        share_count = share_count.saturating_add(participant.share_count);
    }
    (amount_minor > 0).then_some(GroupBuyParticipationSummary {
        amount_minor,
        share_count,
    })
}

/// 筛选已开售且允许合买的彩种。
fn group_buy_enabled_lotteries(lotteries: &[LotteryKind]) -> Vec<LotteryKind> {
    lotteries
        .iter()
        .filter(|lottery| lottery.sale_enabled && lottery.group_buy.enabled)
        .cloned()
        .collect()
}

/// 返回手机端发起合买的默认份额设置。
fn default_group_buy_create_settings() -> GroupBuyCreateSettings {
    GroupBuyCreateSettings {
        min_share_amount_minor: 100,
        initiator_min_percent: 10,
        participant_min_amount_minor: 100,
    }
}

/// 返回指定彩种已启用的合买玩法选项。
fn enabled_group_buy_play_options(lottery: &LotteryKind) -> ApiResult<Vec<GroupBuySelectOption>> {
    let summaries = play_rule_summaries()
        .into_iter()
        .map(|summary| {
            let code = enum_to_string(&summary.code)?;
            Ok((code, summary))
        })
        .collect::<ApiResult<HashMap<_, _>>>()?;
    lottery
        .play_configs
        .iter()
        .filter(|config| config.enabled)
        .map(|config| {
            let value = enum_to_string(&config.rule_code)?;
            let label = summaries
                .get(&value)
                .map(|summary| summary.label.clone())
                .unwrap_or_else(|| value.clone());
            Ok(GroupBuySelectOption { label, value })
        })
        .collect()
}

/// 校验彩种是否允许发起合买。
fn validate_lottery_accepts_group_buy(lottery: &LotteryKind) -> ApiResult<()> {
    if !lottery.sale_enabled {
        return Err(ApiError::BadRequest("彩种已停售".to_string()));
    }
    if !lottery.group_buy.enabled {
        return Err(ApiError::BadRequest("彩种未开启合买".to_string()));
    }
    Ok(())
}

/// 校验合买期号和玩法是否可用。
async fn validate_group_buy_issue_and_play(
    state: &AppState,
    lottery: &LotteryKind,
    issue: &str,
    rule_code: &str,
) -> ApiResult<()> {
    let issue = issue.trim();
    if issue.is_empty() {
        return Err(ApiError::BadRequest("请选择合买期号".to_string()));
    }
    let draw_issue = state.draws.get_by_lottery_issue(&lottery.id, issue).await?;
    if draw_issue.status != DrawIssueStatus::Open {
        return Err(ApiError::BadRequest("合买期号已停止销售".to_string()));
    }

    let rule_code = rule_code.trim();
    if rule_code.is_empty() {
        return Err(ApiError::BadRequest("请选择合买玩法".to_string()));
    }
    let play_enabled = lottery
        .play_configs
        .iter()
        .filter(|config| config.enabled)
        .any(|config| enum_to_string(&config.rule_code).ok().as_deref() == Some(rule_code));
    if !play_enabled {
        return Err(ApiError::BadRequest("合买玩法未开启".to_string()));
    }

    Ok(())
}

/// 校验合买投注内容是否为空或超长。
fn validate_group_buy_numbers(numbers: &str) -> ApiResult<()> {
    let numbers = numbers.trim();
    if numbers.is_empty() {
        return Err(ApiError::BadRequest("请输入合买投注内容".to_string()));
    }
    if numbers.chars().count() > 500 {
        return Err(ApiError::BadRequest("合买投注内容过长".to_string()));
    }
    Ok(())
}

/// 按金额和每份金额换算合买份数。
fn amount_to_share_count(amount_minor: i64, min_share_amount_minor: i64) -> ApiResult<u32> {
    if min_share_amount_minor <= 0 {
        return Ok(0);
    }
    u32::try_from(amount_minor / min_share_amount_minor)
        .map_err(|_| ApiError::BadRequest("合买份数过大".to_string()))
}

/// 生成手机端合买计划编号。
fn next_group_buy_plan_id() -> String {
    format!("G{}", chrono::Local::now().format("%Y%m%d%H%M%S%3f"))
}

/// 生成合买参与记录编号。
fn next_group_buy_participant_id(plan: &GroupBuyPlan) -> String {
    let next_index = plan.participants.len().saturating_add(1);
    let mut random_bytes = [0_u8; 8];
    OsRng.fill_bytes(&mut random_bytes);
    let random_suffix = u64::from_be_bytes(random_bytes);
    format!(
        "{}-P{}-{:03}-{:016X}",
        plan.id,
        chrono::Local::now().format("%Y%m%d%H%M%S%f"),
        next_index,
        random_suffix,
    )
}

/// 返回用户端充值配置。
async fn get_recharge_config(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<RechargeConfigResponse>>> {
    let settings = state.access.settings().await?;
    let settings = recharge_settings_from_system_settings(&settings);

    Ok(Json(ApiEnvelope::success(recharge_config_response(
        &settings,
    ))))
}

/// 返回当前用户充值订单列表。
async fn list_recharge_orders(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Query(query): Query<UserPageQuery>,
) -> ApiResult<Json<ApiEnvelope<Vec<RechargeOrderSummary>>>> {
    let page = state
        .recharges
        .list_for_user_page(
            &session.user.id,
            PageRequest::new(query.page, query.page_size),
        )
        .await?;

    Ok(Json(ApiEnvelope::success(page.items)))
}

/// 用户创建充值订单。
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
        response.support_conversation_id = Some(conversation.id.clone());
        response.order = order;
        publish_support_message_created(&state, &conversation);
    }
    publish_user_recharge_changed(&state, &response.order);

    Ok(Json(ApiEnvelope::success(response)))
}

/// 处理彩虹易支付 GET 回调。
async fn rainbow_epay_notify_query(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<String> {
    confirm_rainbow_notify(state, params).await
}

/// 处理彩虹易支付表单回调。
async fn rainbow_epay_notify_form(
    State(state): State<AppState>,
    Form(params): Form<HashMap<String, String>>,
) -> ApiResult<String> {
    confirm_rainbow_notify(state, params).await
}

/// 统一校验并确认彩虹易支付回调。
async fn confirm_rainbow_notify(
    state: AppState,
    params: HashMap<String, String>,
) -> ApiResult<String> {
    let settings = state.access.settings().await?;
    let settings = recharge_settings_from_system_settings(&settings);
    let order = state
        .recharges
        .confirm_rainbow_notify(params, &settings, &state.finance)
        .await?;
    let rebate_entry = credit_recharge_rebate_for_order(
        &state.access,
        &state.invites,
        &state.rebates,
        &state.finance,
        &order,
    )
    .await?;
    publish_user_recharge_changed(&state, &order);
    publish_user_balance_changed(&state, &order.user_id, "recharge_credit", Some(&order.id)).await;
    if let Some(entry) = rebate_entry {
        publish_user_balance_changed(
            &state,
            &entry.user_id,
            "recharge_rebate_credit",
            entry.reference_id.as_deref(),
        )
        .await;
    }

    Ok("success".to_string())
}

/// 处理彩虹易支付前端返回页查询参数。
async fn rainbow_epay_return_query(
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<ApiEnvelope<HashMap<String, String>>>> {
    Ok(Json(ApiEnvelope::success(params)))
}

/// 返回公共聊天大厅最近消息，所有登录用户都可查看。
async fn list_chat_hall_messages(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<ChatHallMessage>>>> {
    let messages = state.chat_hall.list().await?;
    let messages = public_chat_hall_messages(messages);

    Ok(Json(ApiEnvelope::success(messages)))
}

/// 返回当前用户聊天大厅发言资格，供手机端提前禁用输入栏并展示充值门槛。
async fn get_chat_hall_speaking_status(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<ChatHallSpeakingStatusResponse>>> {
    let status = chat_hall_speaking_status(&state, &session.user.id).await?;

    Ok(Json(ApiEnvelope::success(status)))
}

/// 当前用户发送一条大厅消息，保存成功后推送给所有在线手机端连接。
async fn send_chat_hall_message(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<CreateChatHallMessageRequest>,
) -> ApiResult<Json<ApiEnvelope<ChatHallMessage>>> {
    ensure_chat_hall_speaking_allowed(&state, &session.user.id).await?;
    let message = state.chat_hall.send(&session.user, payload).await?;
    let message = public_chat_hall_message(message);
    state
        .realtime
        .publish_public(chat_hall_message_created_event(&message));

    Ok(Json(ApiEnvelope::success(message)))
}

/// 当前用户发送聊天大厅红包，扣款成功后把红包卡片广播给所有在线用户。
async fn send_chat_hall_red_packet(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<CreateChatHallRedPacketRequest>,
) -> ApiResult<Json<ApiEnvelope<ChatHallMessage>>> {
    ensure_chat_hall_speaking_allowed(&state, &session.user.id).await?;
    let message = state
        .chat_hall
        .send_red_packet(&state.finance, &session.user, payload)
        .await?;
    let message = public_chat_hall_message(message);
    state
        .realtime
        .publish_public(chat_hall_message_created_event(&message));

    let reference_id = chat_hall_message_payload_string(&message, "redPacketId")
        .unwrap_or_else(|| message.id.clone());
    publish_user_balance_changed(
        &state,
        &session.user.id,
        "red_packet_debit",
        Some(&reference_id),
    )
    .await;

    Ok(Json(ApiEnvelope::success(message)))
}

/// 当前用户领取聊天大厅红包，入账成功后广播更新后的红包卡片。
async fn claim_chat_hall_red_packet(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<ClaimChatHallRedPacketResponse>>> {
    let response = state
        .chat_hall
        .claim_red_packet(&state.finance, &session.user, &id)
        .await?;
    let response = public_chat_hall_claim_response(response);
    state
        .realtime
        .publish_public(chat_hall_message_created_event(&response.message));

    publish_user_balance_changed(
        &state,
        &session.user.id,
        "red_packet_credit",
        Some(&response.claim.id),
    )
    .await;

    Ok(Json(ApiEnvelope::success(response)))
}

/// 当前用户查看指定聊天大厅红包的领取记录，用于展示谁抢到了红包。
async fn get_chat_hall_red_packet_claims(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<ChatHallRedPacketClaimsResponse>>> {
    let response = state.chat_hall.red_packet_claims(&id).await?;
    let response = public_chat_hall_red_packet_claims_response(response);

    Ok(Json(ApiEnvelope::success(response)))
}

/// 当前用户把自己发起或参与过的合买计划分享到聊天大厅。
async fn share_chat_hall_group_buy_plan(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<ShareChatHallGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<ChatHallMessage>>> {
    ensure_chat_hall_speaking_allowed(&state, &session.user.id).await?;
    let plan_id = payload.plan_id.trim();
    if plan_id.is_empty() {
        return Err(ApiError::BadRequest("请选择要分享的合买计划".to_string()));
    }
    let plan = state.group_buys.get(plan_id).await?;
    let belongs_to_user = plan
        .participants
        .iter()
        .any(|participant| participant.user_id == session.user.id);
    if !belongs_to_user {
        return Err(ApiError::BadRequest("只能分享自己的合买计划".to_string()));
    }
    let lotteries = state.lotteries.list().await?;
    let access = state.access.snapshot().await?;
    let plan_summary = user_group_buy_plan(
        &plan,
        &lotteries,
        Some(&session.user.id),
        &access.users,
        false,
    )?;
    let message = state
        .chat_hall
        .share_group_buy_plan(
            &session.user,
            ChatHallGroupBuyPlanPayload {
                plan_id: plan_summary.id,
                lottery_id: plan_summary.lottery_id,
                lottery_name: plan_summary.lottery_name,
                issue: plan_summary.issue,
                play_name: plan_summary.play_name,
                title: plan_summary.title,
                total_amount_minor: plan_summary.total_amount_minor,
                share_amount_minor: plan_summary.share_amount_minor,
                sold_shares: plan_summary.sold_shares,
                available_shares: plan_summary.available_shares,
                progress_percent: plan_summary.progress_percent,
                status: enum_to_string(&plan_summary.status)?,
            },
        )
        .await?;
    let message = public_chat_hall_message(message);
    state
        .realtime
        .publish_public(chat_hall_message_created_event(&message));

    Ok(Json(ApiEnvelope::success(message)))
}

/// 统一校验聊天大厅发言权限，避免前端绕过禁用态直接调用发送接口。
async fn ensure_chat_hall_speaking_allowed(state: &AppState, user_id: &str) -> ApiResult<()> {
    let status = chat_hall_speaking_status(state, user_id).await?;
    if status.can_speak {
        return Ok(());
    }

    Err(ApiError::Forbidden(status.message))
}

/// 按系统设置和用户累计充值金额组装聊天大厅发言状态。
async fn chat_hall_speaking_status(
    state: &AppState,
    user_id: &str,
) -> ApiResult<ChatHallSpeakingStatusResponse> {
    let settings = state.access.settings().await?;
    let required_recharge_minor = chat_hall_required_recharge_minor(&settings);
    if required_recharge_minor <= 0 {
        return Ok(chat_hall_speaking_status_from_amounts(0, 0));
    }

    let current_recharge_minor = state.finance.total_recharge_credit_minor(user_id).await?;
    Ok(chat_hall_speaking_status_from_amounts(
        required_recharge_minor,
        current_recharge_minor,
    ))
}

/// 从系统配置读取聊天大厅发言最低累计充值金额，非法值按不限制处理。
fn chat_hall_required_recharge_minor(settings: &[SystemSetting]) -> i64 {
    config_value(settings, CHAT_HALL_SPEAKING_MIN_RECHARGE_SETTING_KEY)
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|amount| *amount > 0)
        .unwrap_or(0)
}

/// 构造聊天大厅发言状态，集中处理金额兜底和用户提示文案。
fn chat_hall_speaking_status_from_amounts(
    required_recharge_minor: i64,
    current_recharge_minor: i64,
) -> ChatHallSpeakingStatusResponse {
    let required_recharge_minor = required_recharge_minor.max(0);
    let current_recharge_minor = current_recharge_minor.max(0);
    let missing_recharge_minor = required_recharge_minor.saturating_sub(current_recharge_minor);
    let can_speak = required_recharge_minor == 0 || missing_recharge_minor == 0;
    let message = if can_speak {
        String::new()
    } else {
        format!(
            "抱歉，暂无发言权限，充值 {} 元即可参与群聊",
            format_chat_hall_yuan(required_recharge_minor)
        )
    };

    ChatHallSpeakingStatusResponse {
        can_speak,
        required_recharge_minor,
        current_recharge_minor,
        missing_recharge_minor,
        message,
    }
}

/// 把后台配置的分金额转成用户提示里的人民币展示文本。
fn format_chat_hall_yuan(amount_minor: i64) -> String {
    let amount_minor = amount_minor.max(0);
    let yuan = amount_minor / 100;
    let cents = amount_minor % 100;
    if cents == 0 {
        format!("¥{yuan}")
    } else {
        format!("¥{yuan}.{cents:02}")
    }
}

/// 从聊天大厅消息 payload 中读取指定字符串字段。
fn chat_hall_message_payload_string(message: &ChatHallMessage, key: &str) -> Option<String> {
    message
        .payload
        .as_ref()
        .and_then(|payload| payload.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

/// 将聊天大厅历史消息转换成用户端公开数据，避免公开暴露完整用户名。
fn public_chat_hall_messages(messages: Vec<ChatHallMessage>) -> Vec<ChatHallMessage> {
    messages.into_iter().map(public_chat_hall_message).collect()
}

/// 将聊天大厅消息中的用户名做公开展示脱敏，保留数据库中的原始用户名不变。
fn public_chat_hall_message(mut message: ChatHallMessage) -> ChatHallMessage {
    message.username = mask_public_chat_hall_username(&message.username);
    message
}

/// 将红包领取响应转换成用户端公开数据，消息和领取人都统一脱敏。
fn public_chat_hall_claim_response(
    mut response: ClaimChatHallRedPacketResponse,
) -> ClaimChatHallRedPacketResponse {
    response.message = public_chat_hall_message(response.message);
    response.claim.username = mask_public_chat_hall_username(&response.claim.username);
    response
}

/// 将红包领取记录列表转换成用户端公开数据，领取人名称只展示前四个字符。
fn public_chat_hall_red_packet_claims_response(
    mut response: ChatHallRedPacketClaimsResponse,
) -> ChatHallRedPacketClaimsResponse {
    response
        .claims
        .iter_mut()
        .for_each(|claim| claim.username = mask_public_chat_hall_username(&claim.username));
    response
}

/// 对聊天大厅公开用户名做隐私脱敏，保留前四个字符，后续字符统一替换为星号。
fn mask_public_chat_hall_username(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return "会员".to_string();
    }

    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 4 {
        return chars.into_iter().collect();
    }

    let visible_prefix = chars.iter().take(4).collect::<String>();
    format!("{}{}", visible_prefix, "*".repeat(chars.len() - 4))
}

/// 返回当前用户客服会话列表。
async fn list_user_support_conversations(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<Vec<SupportConversation>>>> {
    let conversations = state.support.list_for_user(&session.user.id).await?;

    Ok(Json(ApiEnvelope::success(conversations)))
}

/// 返回当前用户指定客服会话详情。
async fn get_user_support_conversation(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let conversation = state.support.get_for_user(&id, &session.user.id).await?;

    Ok(Json(ApiEnvelope::success(conversation)))
}

/// 当前用户回复自己的客服会话。
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
    publish_support_message_created(&state, &conversation);

    Ok(Json(ApiEnvelope::success(conversation)))
}

/// 当前用户打开客服会话后标记为已读，用于清理手机端在线客服红点。
async fn mark_user_support_conversation_read(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let conversation = state.support.mark_user_read(&id, &session.user.id).await?;
    publish_user_support_conversation_updated(&state, &conversation);

    Ok(Json(ApiEnvelope::success(conversation)))
}

/// 返回当前用户提现方式列表。
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

/// 当前用户新增提现方式。
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

/// 当前用户更新提现方式。
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

/// 当前用户删除提现方式。
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

/// 返回当前用户提现申请列表。
async fn list_withdrawal_orders(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Query(query): Query<UserPageQuery>,
) -> ApiResult<Json<ApiEnvelope<Vec<WithdrawalOrderSummary>>>> {
    let page = state
        .withdrawals
        .list_for_user_page(
            &session.user.id,
            PageRequest::new(query.page, query.page_size),
        )
        .await?;

    Ok(Json(ApiEnvelope::success(page.items)))
}

/// 解析用户列表展示中的时间字符串，兼容 unix 秒字段。
fn parse_user_timestamp_seconds(value: &str) -> Option<i64> {
    let value = value.trim();
    if let Some(seconds) = value.strip_prefix("unix:") {
        return seconds.parse::<i64>().ok();
    }

    let parsed = NaiveDateTime::parse_from_str(value, TIMESTAMP_FORMAT).ok()?;
    chrono::Local
        .from_local_datetime(&parsed)
        .single()
        .map(|value| value.timestamp())
        .or_else(|| Some(parsed.and_utc().timestamp()))
}

/// 按创建时间倒序比较，处理同秒时按业务流水/订单编号倒序保证分页稳定。
fn compare_created_time_desc(
    left_created_at: &str,
    left_id: &str,
    right_created_at: &str,
    right_id: &str,
) -> Ordering {
    parse_user_timestamp_seconds(right_created_at)
        .cmp(&parse_user_timestamp_seconds(left_created_at))
        .then_with(|| right_created_at.cmp(left_created_at))
        .then_with(|| right_id.cmp(left_id))
}

/// 合买计划列表按创建时间倒序展示。
fn sort_group_buy_plans_by_time_desc(plans: &mut [UserGroupBuyPlan]) {
    plans.sort_by(|left, right| {
        compare_created_time_desc(&left.created_at, &left.id, &right.created_at, &right.id)
    });
}

/// 当前用户提交提现申请并冻结余额。
async fn create_withdrawal_order(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<CreateWithdrawalOrderRequest>,
) -> ApiResult<Json<ApiEnvelope<WithdrawalOrderSummary>>> {
    let method_id = payload.method_id.trim().to_string();
    let method = state
        .access
        .list_withdrawal_methods(&session.user.id)
        .await?
        .into_iter()
        .find(|method| method.id == method_id)
        .ok_or_else(|| ApiError::NotFound("提现方式不存在".to_string()))?;
    let order = state
        .withdrawals
        .create_order(&session.user, &method, payload, &state.finance)
        .await?;
    publish_user_withdrawal_changed(&state, &order);
    publish_user_balance_changed(&state, &order.user_id, "withdrawal_freeze", Some(&order.id))
        .await;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 推送用户余额变化事件，读取资金账户失败只记录日志，不影响主业务结果。
async fn publish_user_balance_changed(
    state: &AppState,
    user_id: &str,
    reason: &str,
    reference_id: Option<&str>,
) {
    match state.finance.account_or_create(user_id).await {
        Ok(account) => state.realtime.publish_user(
            user_id,
            balance_changed_event(&account, reason, reference_id),
        ),
        Err(error) => tracing::warn!(
            user_id,
            error = %error.log_message(),
            "推送用户余额变化时读取资金账户失败"
        ),
    }
}

/// 推送用户注单变化事件，供手机端注单列表按需刷新。
fn publish_user_order_changed(state: &AppState, order: &OrderDetail, action: &str) {
    state
        .realtime
        .publish_user(&order.user_id, order_changed_event(order, action));
}

/// 推送用户充值订单变化事件，供手机端充值记录按需刷新。
fn publish_user_recharge_changed(state: &AppState, order: &RechargeOrderSummary) {
    state
        .realtime
        .publish_user(&order.user_id, recharge_changed_event(order));
}

/// 推送客服消息新增事件，保证客服直充聊天在用户端和后台之间实时同步。
fn publish_support_message_created(state: &AppState, conversation: &SupportConversation) {
    let Some(message) = conversation.messages.last() else {
        return;
    };
    let event = support_message_created_event(conversation, message);
    state
        .realtime
        .publish_user(&conversation.user_id, event.clone());
    state.realtime.publish_admin(event);
    spawn_support_telegram_notification(state.access.clone(), conversation);
}

/// 只向会话所属用户推送客服会话已读变化，避免后台客服列表因用户读消息产生无意义刷新。
fn publish_user_support_conversation_updated(state: &AppState, conversation: &SupportConversation) {
    state.realtime.publish_user(
        &conversation.user_id,
        support_conversation_updated_event(conversation),
    );
}

/// 推送用户提现订单变化事件，供手机端提现记录按需刷新。
fn publish_user_withdrawal_changed(state: &AppState, order: &WithdrawalOrderSummary) {
    state
        .realtime
        .publish_user(&order.user_id, withdrawal_changed_event(order));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        group_buy::{GroupBuyParticipant, GroupBuyPlanStatus},
        lottery::{DrawMode, DrawSchedule, GroupBuyConfig, LotteryNumberType, PlayCategory},
        order::{OrderSource, OrderStatus},
        play::{PlayRuleCode, PlaySelection},
        rebate::RebateMode,
    };
    use std::collections::HashSet;

    #[test]
    /// 验证注册 IP 解析兼容常见反向代理请求头形态。
    fn registration_ip_parser_handles_proxy_headers() {
        assert_eq!(
            first_registration_ip("for=\"203.0.113.5\";proto=https"),
            Some("203.0.113.5".to_string())
        );
        assert_eq!(
            first_registration_ip("198.51.100.8:443, 10.0.0.2"),
            Some("198.51.100.8".to_string())
        );
        assert_eq!(
            first_registration_ip("for=\"[2001:db8::1]\";proto=https"),
            Some("2001:db8::1".to_string())
        );
    }

    #[test]
    /// 验证 Cloudflare 请求头优先于普通代理链路，并保留 IPv6 真实客户端地址。
    fn registration_ip_parser_prefers_cloudflare_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "198.51.100.8, 10.0.0.2".parse().unwrap());
        headers.insert(
            "cf-connecting-ip",
            "2409:8950:5353:80:c46d:c9ff:fec7:4f38".parse().unwrap(),
        );
        headers.insert("cf-ipcountry", "CN".parse().unwrap());

        let client_info = registration_client_info_from_headers(&headers);

        assert_eq!(
            client_info.registered_ip,
            Some("2409:8950:5353:80:c46d:c9ff:fec7:4f38".to_string())
        );
        assert_eq!(client_info.cloudflare_country, Some("中国".to_string()));
    }

    #[test]
    /// 验证代理头写入 IP 时会清掉客户端伪造地区，只保留服务端可信地区。
    fn registration_client_info_discards_client_location_without_cloudflare_country() {
        let mut payload = UserRegisterRequest {
            username: Some("proxy_user".to_string()),
            email: None,
            contact_qq: None,
            password: "password123".to_string(),
            invite_code: None,
            registration_location: Some(crate::domain::user::UserRegistrationLocation {
                registered_ip: "1.1.1.1".to_string(),
                country: "客户端国家".to_string(),
                region: "客户端区域".to_string(),
                city: "客户端城市".to_string(),
                source: "client".to_string(),
            }),
        };

        attach_registration_client_info(
            &mut payload,
            RegistrationClientInfo {
                registered_ip: Some("203.0.113.20".to_string()),
                cloudflare_country: None,
            },
        );

        let location = payload.registration_location.expect("location attached");
        assert_eq!(location.registered_ip, "203.0.113.20");
        assert_eq!(location.country, "");
        assert_eq!(location.region, "");
        assert_eq!(location.city, "");
        assert_eq!(location.source, "ip");
    }

    #[test]
    /// 验证真实 GPS 来源不会被 Cloudflare 国家头覆盖。
    fn registration_client_info_keeps_gps_location_when_cloudflare_country_exists() {
        let mut payload = UserRegisterRequest {
            username: Some("gps_user".to_string()),
            email: None,
            contact_qq: None,
            password: "password123".to_string(),
            invite_code: None,
            registration_location: Some(crate::domain::user::UserRegistrationLocation {
                registered_ip: String::new(),
                country: "中国".to_string(),
                region: "广东".to_string(),
                city: "深圳".to_string(),
                source: "gps".to_string(),
            }),
        };

        attach_registration_client_info(
            &mut payload,
            RegistrationClientInfo {
                registered_ip: Some("203.0.113.30".to_string()),
                cloudflare_country: Some("美国".to_string()),
            },
        );

        let location = payload.registration_location.expect("location attached");
        assert_eq!(location.registered_ip, "203.0.113.30");
        assert_eq!(location.country, "中国");
        assert_eq!(location.region, "广东");
        assert_eq!(location.city, "深圳");
        assert_eq!(location.source, "gps");
    }

    #[test]
    /// 验证用户端邀请中心只允许代理在策略开启时使用邀请码。
    fn user_invitation_permission_requires_agent_and_enabled_policy() {
        let policy = test_invite_policy(true);
        let regular = test_invitation_user("U90010", "regular", UserKind::Regular, None);
        let agent = test_invitation_user("U90011", "agent", UserKind::Agent, None);

        assert!(!user_can_invite(&regular, &policy));
        assert!(user_can_invite(&agent, &policy));
        assert!(!user_can_invite(&agent, &test_invite_policy(false)));
    }

    #[test]
    /// 验证邀请中心下级投注画像会累计玩法金额，并选出最近一笔投注。
    fn user_invitation_bet_profile_summarizes_play_and_latest_bet() {
        let mut profile = DirectUserBetProfile::default();

        profile
            .add_bet(DirectUserBetInput {
                order_id: "O-1".to_string(),
                lottery_id: "txffc".to_string(),
                lottery_name: "腾讯分分彩".to_string(),
                issue: "202606170001".to_string(),
                rule_code: "fiveFrontDirect".to_string(),
                play_name: "前 3 直选".to_string(),
                amount_minor: 1_000,
                created_at: "2026-06-17 10:00:00".to_string(),
            })
            .expect("首笔投注应可汇总");
        profile
            .add_bet(DirectUserBetInput {
                order_id: "O-2".to_string(),
                lottery_id: "txffc".to_string(),
                lottery_name: "腾讯分分彩".to_string(),
                issue: "202606170002".to_string(),
                rule_code: "fiveBackDirect".to_string(),
                play_name: "后 3 直选".to_string(),
                amount_minor: 2_500,
                created_at: "2026-06-17 10:02:00".to_string(),
            })
            .expect("第二笔投注应可汇总");

        assert_eq!(profile.total_bet_amount_minor, 3_500);
        let play_summaries = profile.play_summary_items();
        assert_eq!(play_summaries[0].play_name, "后 3 直选");
        assert_eq!(play_summaries[0].amount_minor, 2_500);
        let latest_bet = profile.latest_bet.expect("应存在最近投注");
        assert_eq!(latest_bet.order_id, "O-2");
        assert_eq!(latest_bet.issue, "202606170002");
    }

    #[test]
    /// 验证邀请中心会合并后台邀请记录和注册时绑定的代理关系。
    fn user_invitation_candidates_merge_records_and_agent_links() {
        let agent = test_invitation_user("U90011", "agent", UserKind::Agent, None);
        let record_user = test_invitation_user(
            "U90012",
            "record_user",
            UserKind::Regular,
            Some(agent.id.clone()),
        );
        let linked_user = test_invitation_user(
            "U90013",
            "linked_user",
            UserKind::Regular,
            Some(agent.id.clone()),
        );
        let users = vec![agent.clone(), record_user.clone(), linked_user.clone()];
        let records = vec![InviteRecord {
            id: "INV-90012".to_string(),
            inviter_user_id: agent.id.clone(),
            inviter_username: agent.username.clone(),
            invitee_user_id: record_user.id.clone(),
            invitee_username: record_user.username.clone(),
            invite_code: agent.invite_code.clone(),
            status: InviteStatus::Pending,
            rebate_enabled: false,
            note: String::new(),
            created_at: "2026-06-05 19:00:00".to_string(),
            updated_at: "2026-06-05 19:00:00".to_string(),
        }];

        let candidates = collect_direct_invitation_candidates(&agent.id, &users, &records, true);

        assert_eq!(candidates.len(), 2);
        let record_candidate = candidates
            .iter()
            .find(|candidate| candidate.user.id == record_user.id)
            .expect("后台邀请记录用户应进入直属列表");
        assert!(matches!(
            record_candidate.invite_status,
            InviteStatus::Pending
        ));
        assert!(!record_candidate.rebate_enabled);
        assert_eq!(record_candidate.created_at, "2026-06-05 19:00:00");
        assert_eq!(record_candidate.user.created_at, "2026-06-05 18:00:00");
        let linked_candidate = candidates
            .iter()
            .find(|candidate| candidate.user.id == linked_user.id)
            .expect("注册绑定代理用户应进入直属列表");
        assert!(matches!(
            linked_candidate.invite_status,
            InviteStatus::Active
        ));
        assert!(linked_candidate.rebate_enabled);
        assert!(linked_candidate.created_at.is_empty());
        assert_eq!(linked_candidate.user.created_at, "2026-06-05 18:00:00");
    }

    #[test]
    /// 验证邀请中心直属充值汇总只统计正向充值入账流水。
    fn user_invitation_recharge_sum_counts_only_positive_recharge_credit() {
        let entries = vec![
            test_ledger_entry("L-001", LedgerEntryKind::RechargeCredit, 10_000),
            test_ledger_entry("L-002", LedgerEntryKind::OrderDebit, -2_000),
            test_ledger_entry("L-003", LedgerEntryKind::RechargeCredit, -1_000),
            test_ledger_entry("L-004", LedgerEntryKind::RechargeCredit, 5_000),
        ];

        let amount = sum_recharge_credits_minor(&entries).expect("充值汇总不应失败");

        assert_eq!(amount, 15_000);
    }

    #[test]
    /// 验证聊天大厅发言门槛不足时返回可直接展示给手机端的中文提示。
    fn chat_hall_speaking_status_blocks_when_recharge_below_threshold() {
        let status = chat_hall_speaking_status_from_amounts(30_000, 12_000);

        assert!(!status.can_speak);
        assert_eq!(status.required_recharge_minor, 30_000);
        assert_eq!(status.current_recharge_minor, 12_000);
        assert_eq!(status.missing_recharge_minor, 18_000);
        assert_eq!(
            status.message,
            "抱歉，暂无发言权限，充值 ¥300 元即可参与群聊"
        );
    }

    #[test]
    /// 验证聊天大厅发言门槛配置为 0 时表示不限制，达到门槛也允许发言。
    fn chat_hall_speaking_status_allows_zero_or_reached_threshold() {
        let disabled_settings = vec![test_setting(
            CHAT_HALL_SPEAKING_MIN_RECHARGE_SETTING_KEY,
            "0",
        )];
        assert_eq!(chat_hall_required_recharge_minor(&disabled_settings), 0);
        assert!(chat_hall_speaking_status_from_amounts(0, 0).can_speak);
        assert!(chat_hall_speaking_status_from_amounts(30_000, 30_000).can_speak);
    }

    #[test]
    /// 验证聊天大厅用户端公开名称只保留前四个字符，避免完整用户名泄露到手机端。
    fn public_chat_hall_username_is_masked() {
        assert_eq!(mask_public_chat_hall_username("爱情819281"), "爱情81****");
        assert_eq!(mask_public_chat_hall_username("测试用户1"), "测试用户*");
        assert_eq!(mask_public_chat_hall_username("张三"), "张三");
        assert_eq!(mask_public_chat_hall_username(""), "会员");
    }

    #[test]
    /// 验证聊天大厅消息和红包领取记录在返回手机端前会统一脱敏。
    fn public_chat_hall_response_masks_message_and_claim_usernames() {
        use crate::domain::chat_hall::{
            ChatHallMessageType, ChatHallRedPacketClaim, ChatHallRedPacketClaimsResponse,
        };

        let message = public_chat_hall_message(ChatHallMessage {
            id: "CHM-000000000001".to_string(),
            user_id: "U10001".to_string(),
            username: "爱情819281".to_string(),
            avatar_url: String::new(),
            content: "大家好".to_string(),
            message_type: ChatHallMessageType::Text,
            payload: None,
            created_at: "2026-06-21 18:00:00".to_string(),
        });
        assert_eq!(message.username, "爱情81****");

        let claims = public_chat_hall_red_packet_claims_response(ChatHallRedPacketClaimsResponse {
            red_packet_id: "CHRP-000000000001".to_string(),
            greeting: "恭喜发财".to_string(),
            total_amount_minor: 1_000,
            remaining_amount_minor: 500,
            claim_count: 2,
            claimed_count: 1,
            claims: vec![ChatHallRedPacketClaim {
                id: "CHRPC-000000000001".to_string(),
                red_packet_id: "CHRP-000000000001".to_string(),
                user_id: "U10002".to_string(),
                username: "明月清风888".to_string(),
                amount_minor: 500,
                created_at: "2026-06-21 18:01:00".to_string(),
            }],
        });

        assert_eq!(claims.claims[0].username, "明月清风***");
    }

    #[test]
    /// 验证邀请中心已结算返利只统计真实充值返利入账流水。
    fn user_invitation_paid_commission_counts_recharge_rebate_credit() {
        let entries = vec![
            test_ledger_entry("L-001", LedgerEntryKind::RechargeRebateCredit, 350),
            test_ledger_entry("L-002", LedgerEntryKind::RechargeCredit, 10_000),
            test_ledger_entry("L-003", LedgerEntryKind::RechargeRebateCredit, -100),
            test_ledger_entry("L-004", LedgerEntryKind::PayoutCredit, 500),
        ];

        let amount = sum_recharge_rebate_credits_minor(&entries).expect("返利汇总不应失败");

        assert_eq!(amount, 350);
    }

    #[test]
    /// 验证邀请中心直属提现汇总按用户累计已通过的正向提现金额。
    fn user_invitation_withdrawal_sum_counts_positive_approved_orders() {
        let orders = vec![
            test_withdrawal_order_for_user("W-001", "U90012", 10_000),
            test_withdrawal_order_for_user("W-002", "U90012", 5_000),
            test_withdrawal_order_for_user("W-003", "U90013", 12_000),
            test_withdrawal_order_for_user("W-004", "U90014", -1_000),
        ];

        let totals = sum_approved_withdrawal_minor_by_user(&orders).expect("提现汇总不应失败");

        assert_eq!(totals.get("U90012"), Some(&15_000));
        assert_eq!(totals.get("U90013"), Some(&12_000));
        assert!(!totals.contains_key("U90014"));
    }

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

    #[test]
    /// 验证 APP 更新配置在未配置下载地址时不会误提示客户端更新。
    fn mobile_app_update_ignores_unconfigured_download_url() {
        let settings = vec![
            test_setting("mobile_app_android_enabled", "true"),
            test_setting("mobile_app_android_latest_version", "2.0.0"),
            test_setting("mobile_app_android_latest_build", "20"),
            test_setting("mobile_app_android_package_url", "未配置"),
        ];
        let query = MobileAppUpdateQuery {
            platform: Some("android".to_string()),
            current_version: Some("1.0.0".to_string()),
            current_build: Some(1),
        };

        let config = mobile_app_update_from_settings(&settings, &query);

        assert_eq!(config.platform, "android");
        assert!(config.enabled);
        assert!(!config.update_available);
        assert_eq!(config.download_url, None);
    }

    #[test]
    /// 验证构建号落后时会返回需要更新，并携带强制更新配置。
    fn mobile_app_update_detects_newer_build() {
        let settings = vec![
            test_setting("mobile_app_android_enabled", "true"),
            test_setting("mobile_app_android_latest_version", "1.1.0"),
            test_setting("mobile_app_android_latest_build", "12"),
            test_setting(
                "mobile_app_android_package_url",
                "https://example.com/app.apk",
            ),
            test_setting("mobile_app_android_force_update", "true"),
            test_setting("mobile_app_android_release_notes", "修复启动异常"),
        ];
        let query = MobileAppUpdateQuery {
            platform: Some("apk".to_string()),
            current_version: Some("1.1.0".to_string()),
            current_build: Some(11),
        };

        let config = mobile_app_update_from_settings(&settings, &query);

        assert!(config.update_available);
        assert!(config.force_update);
        assert_eq!(
            config.download_url,
            Some("https://example.com/app.apk".to_string())
        );
        assert_eq!(config.release_notes, "修复启动异常");
    }

    #[test]
    /// 验证 iOS 平台别名会读取 iOS 配置，并能通过版本号判断更新。
    fn mobile_app_update_supports_ios_alias_and_version_compare() {
        let settings = vec![
            test_setting("mobile_app_ios_enabled", "true"),
            test_setting("mobile_app_ios_latest_version", "1.2.0"),
            test_setting("mobile_app_ios_latest_build", "1"),
            test_setting("mobile_app_ios_package_url", "https://example.com/app.ipa"),
        ];
        let query = MobileAppUpdateQuery {
            platform: Some("iphone".to_string()),
            current_version: Some("1.1.9".to_string()),
            current_build: Some(1),
        };

        let config = mobile_app_update_from_settings(&settings, &query);

        assert_eq!(config.platform, "ios");
        assert_eq!(config.latest_version, "1.2.0");
        assert!(config.update_available);
    }

    #[test]
    /// 验证常见版本号数字片段比较不会把相同版本判定为更新。
    fn version_compare_handles_equal_and_newer_versions() {
        assert!(version_is_newer("1.2.10", "1.2.9"));
        assert!(version_is_newer("2.0.0-beta.1", "1.9.9"));
        assert!(!version_is_newer("1.2.3", "1.2.3"));
        assert!(!version_is_newer("1.2.3", "1.2.4"));
    }

    #[test]
    /// 验证用户端展示机器人合买时会隐藏真实机器人账号和机器人标题。
    fn user_group_buy_plan_masks_robot_initiator_display() {
        let lotteries = vec![test_group_buy_lottery()];
        let first_plan = test_group_buy_plan(
            "G-ROBOT-R-BUY-001-SSC60-20260605200000",
            "20260605200000",
            "agent_alpha",
            "合买机器人 20260605200000",
        );
        let second_plan = test_group_buy_plan(
            "G-ROBOT-R-BUY-001-SSC60-20260605200100",
            "20260605200100",
            "agent_alpha",
            "合买机器人 20260605200100",
        );

        let first_view = user_group_buy_plan(&first_plan, &lotteries, None, &[], false)
            .expect("robot plan can map");
        let second_view = user_group_buy_plan(&second_plan, &lotteries, None, &[], false)
            .expect("robot plan can map");

        assert_ne!(first_view.initiator_display, "agent_alpha");
        assert_eq!(first_view.initiator_avatar_url, "");
        assert!(first_view.initiator_display.contains('*'));
        assert!(!first_view.initiator_display.contains("机器人"));
        assert!(!first_view.initiator_display.contains("agent"));
        assert!(!first_view.initiator_display.contains("会员"));
        assert_eq!(first_view.title, "测试彩 第20260605200000期合买");
        assert_ne!(first_view.initiator_display, second_view.initiator_display);
        assert_eq!(second_view.title, "测试彩 第20260605200100期合买");
    }

    #[test]
    /// 验证普通用户合买也只展示脱敏发起人和自定义标题。
    fn user_group_buy_plan_masks_normal_initiator_display() {
        let lotteries = vec![test_group_buy_lottery()];
        let plan = test_group_buy_plan(
            "G-USER-001",
            "20260605200000",
            "regular_user",
            "用户发起合买",
        );

        let view =
            user_group_buy_plan(&plan, &lotteries, None, &[], false).expect("normal plan can map");

        assert_eq!(view.initiator_display, "regu********");
        assert_eq!(view.title, "用户发起合买");
    }

    #[test]
    /// 验证普通用户合买会透出发起人头像，机器人计划不会透出真实头像。
    fn user_group_buy_plan_returns_normal_initiator_avatar_only() {
        let lotteries = vec![test_group_buy_lottery()];
        let mut user = test_invitation_user("U90001", "regular_user", UserKind::Regular, None);
        user.avatar_url = " https://cdn.example.com/avatar.png ".to_string();
        let plan = test_group_buy_plan(
            "G-USER-AVATAR-001",
            "20260605200000",
            "regular_user",
            "用户发起合买",
        );
        let robot_plan = test_group_buy_plan(
            "G-ROBOT-R-BUY-001-SSC60-20260605200000",
            "20260605200000",
            "agent_alpha",
            "合买机器人 20260605200000",
        );

        let view = user_group_buy_plan(&plan, &lotteries, None, &[user.clone()], false)
            .expect("normal plan can map");
        let robot_view = user_group_buy_plan(&robot_plan, &lotteries, None, &[user], false)
            .expect("robot plan can map");

        assert_eq!(
            view.initiator_avatar_url,
            "https://cdn.example.com/avatar.png"
        );
        assert_eq!(robot_view.initiator_avatar_url, "");
    }

    #[test]
    /// 验证用户端合买详情参与人列表只展示脱敏信息，并标记当前用户。
    fn user_group_buy_plan_returns_masked_participants() {
        let lotteries = vec![test_group_buy_lottery()];
        let mut plan = test_group_buy_plan(
            "G-USER-PARTICIPANTS-001",
            "20260605200000",
            "regular_user",
            "用户发起合买",
        );
        plan.initiator_user_id = "U10001".to_string();
        plan.participants = vec![
            GroupBuyParticipant {
                id: "G-USER-PARTICIPANTS-001-P001".to_string(),
                user_id: "U10001".to_string(),
                username: "爱情819281".to_string(),
                amount_minor: 1_000,
                share_count: 1,
                note: "发起人认购".to_string(),
                created_at: "2026-06-05 20:00:00".to_string(),
            },
            GroupBuyParticipant {
                id: "G-USER-PARTICIPANTS-001-P002".to_string(),
                user_id: "U20002".to_string(),
                username: "current_user".to_string(),
                amount_minor: 2_000,
                share_count: 2,
                note: "用户认购".to_string(),
                created_at: "2026-06-05 20:01:00".to_string(),
            },
            GroupBuyParticipant {
                id: "G-USER-PARTICIPANTS-001-P003".to_string(),
                user_id: "U90001".to_string(),
                username: "agent_alpha".to_string(),
                amount_minor: 2_000,
                share_count: 2,
                note: "机器人补单".to_string(),
                created_at: "2026-06-05 20:02:00".to_string(),
            },
        ];

        let view = user_group_buy_plan(&plan, &lotteries, Some("U20002"), &[], true)
            .expect("合买计划可以转换为用户端详情");

        assert_eq!(view.participant_count, 3);
        assert_eq!(view.participants[0].display_name, "爱情81****");
        assert_eq!(view.participants[0].amount_minor, 1_000);
        assert!(!view.participants[0].is_mine);
        assert_eq!(view.participants[1].display_name, "我");
        assert!(view.participants[1].is_mine);
        assert_eq!(view.participants[1].share_count, 2);
        assert!(!view.participants[2].display_name.contains("agent"));
        assert!(!view.participants[2].display_name.contains("机器人"));
        assert!(!view.participants[2].display_name.contains("会员"));
        assert!(view.participants[2].display_name.contains('*'));
    }

    #[test]
    /// 验证用户端合买列表转换默认不携带参与人明细，避免列表接口变重。
    fn user_group_buy_plan_omits_participants_for_list_response() {
        let lotteries = vec![test_group_buy_lottery()];
        let plan = test_group_buy_plan(
            "G-USER-LIST-001",
            "20260605200000",
            "regular_user",
            "用户发起合买",
        );

        let view = user_group_buy_plan(&plan, &lotteries, Some("U90001"), &[], false)
            .expect("合买列表可以转换");

        assert_eq!(view.participant_count, 1);
        assert!(view.participants.is_empty());
    }

    #[test]
    /// 验证合买发起人脱敏支持中文、短昵称和空昵称。
    fn mask_group_buy_initiator_display_handles_edge_cases() {
        assert_eq!(
            mask_group_buy_initiator_display(" 爱情819281 "),
            "爱情81****"
        );
        assert_eq!(mask_group_buy_initiator_display("张三"), "张三");
        assert_eq!(mask_group_buy_initiator_display("A9"), "A9");
        assert_eq!(mask_group_buy_initiator_display("测试用户"), "测试用户");
        assert_eq!(mask_group_buy_initiator_display("测试用户1"), "测试用户*");
        assert_eq!(mask_group_buy_initiator_display("单"), "单");
        assert_eq!(mask_group_buy_initiator_display(""), "会员");
    }

    #[test]
    /// 验证合买参与编号包含计划前缀和随机段，连续生成时不应重复。
    fn group_buy_participant_id_uses_random_suffix() {
        let plan = test_group_buy_plan(
            "G-USER-UNIQUE-001",
            "20260605200000",
            "regular_user",
            "用户发起合买",
        );
        let mut generated_ids = HashSet::new();

        for _ in 0..32 {
            let participant_id = next_group_buy_participant_id(&plan);
            let random_suffix = participant_id
                .rsplit('-')
                .next()
                .expect("参与编号应包含随机后缀");

            assert!(participant_id.starts_with("G-USER-UNIQUE-001-P"));
            assert_eq!(random_suffix.len(), 16);
            assert!(random_suffix.chars().all(|ch| ch.is_ascii_hexdigit()));
            assert!(generated_ids.insert(participant_id));
        }
    }

    #[test]
    /// 验证用户端注单列表会包含本人参与且已经满单生成的合买订单。
    fn user_visible_bet_orders_include_participated_group_buy_order() {
        let direct_order = test_order("O000000000001", "U20002", OrderSource::Direct);
        let participated_group_order = test_order("O000000000002", "U10001", OrderSource::GroupBuy);
        let unrelated_group_order = test_order("O000000000003", "U10001", OrderSource::GroupBuy);
        let unrelated_direct_order = test_order("O000000000004", "U10001", OrderSource::Direct);
        let plans = vec![
            test_group_buy_plan_with_order(
                "G-USER-ORDER-001",
                "O000000000002",
                vec![
                    test_group_buy_participant("G-USER-ORDER-001-P001", "U10001"),
                    test_group_buy_participant("G-USER-ORDER-001-P002", "U20002"),
                    test_group_buy_participant_with_amount(
                        "G-USER-ORDER-001-P003",
                        "U20002",
                        2_000,
                    ),
                ],
            ),
            test_group_buy_plan_with_order(
                "G-USER-ORDER-002",
                "O000000000003",
                vec![test_group_buy_participant(
                    "G-USER-ORDER-002-P001",
                    "U10003",
                )],
            ),
        ];

        let visible = user_visible_bet_orders(
            "U20002",
            vec![
                unrelated_direct_order,
                unrelated_group_order,
                participated_group_order,
                direct_order,
            ],
            &plans,
            &[],
            &[test_group_buy_lottery()],
        )
        .expect("用户注单列表可以合并合买参与金额");
        let visible_ids = visible
            .iter()
            .map(|item| item.order.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(visible_ids, vec!["O000000000002", "O000000000001"]);
        let group_buy_item = visible
            .iter()
            .find(|item| item.order.order_source == OrderSource::GroupBuy)
            .expect("参与的合买订单应进入注单列表");
        assert_eq!(group_buy_item.participation_amount_minor, Some(3_000));
        assert_eq!(group_buy_item.participation_payout_minor, None);
        assert_eq!(
            group_buy_item.group_buy_plan_id.as_deref(),
            Some("G-USER-ORDER-001")
        );
        let direct_item = visible
            .iter()
            .find(|item| item.order.order_source == OrderSource::Direct)
            .expect("独立订单应进入注单列表");
        assert_eq!(direct_item.group_buy_plan_id, None);
        assert_eq!(direct_item.participation_amount_minor, None);
        assert_eq!(direct_item.participation_payout_minor, None);
    }

    #[test]
    /// 验证用户端注单列表会包含本人已经认购但尚未满单生成真实订单的合买计划。
    fn user_visible_bet_orders_include_unformed_group_buy_participation() {
        let direct_order = test_order("O000000000007", "U20002", OrderSource::Direct);
        let mut pending_plan = test_group_buy_plan(
            "G-USER-PENDING-001",
            "20260605200100",
            "regular_user",
            "未满单合买",
        );
        pending_plan.filled_amount_minor = 3_000;
        pending_plan.participants = vec![
            test_group_buy_participant("G-USER-PENDING-001-P001", "U10001"),
            test_group_buy_participant_with_amount("G-USER-PENDING-001-P002", "U20002", 2_000),
        ];
        pending_plan.participants[1].created_at = "2026-06-05 20:02:00".to_string();

        let visible = user_visible_bet_orders(
            "U20002",
            vec![direct_order],
            &[pending_plan],
            &[],
            &[test_group_buy_lottery()],
        )
        .expect("用户注单列表可以合并未成单合买认购");

        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].order.id, "GB-G-USER-PENDING-001");
        assert_eq!(
            visible[0].group_buy_plan_id.as_deref(),
            Some("G-USER-PENDING-001")
        );
        assert_eq!(
            visible[0].group_buy_plan_status,
            Some(GroupBuyPlanStatus::Open)
        );
        assert!(visible[0].group_buy_pending_plan);
        assert_eq!(visible[0].participation_amount_minor, Some(2_000));
        assert_eq!(visible[0].participation_share_count, Some(2));
        assert_eq!(visible[0].order.order_source, OrderSource::GroupBuy);
        assert_eq!(visible[0].order.status, OrderStatus::PendingDraw);
        assert_eq!(visible[0].order.number_type, LotteryNumberType::FiveDigit);
        assert_eq!(visible[0].order.stake_count, 1);
        assert_eq!(visible[0].order.unit_amount_minor, 1_000);
        assert_eq!(visible[0].order.expanded_bets, vec!["123".to_string()]);
        assert_eq!(
            visible[0].order.selection.positions,
            vec![vec![1], vec![2], vec![3]]
        );
        assert_eq!(visible[1].order.id, "O000000000007");
    }

    #[test]
    /// 验证用户端注单视图可以把真实已下单订单和未成单合买认购分开分页。
    fn user_bet_order_view_filter_splits_orders_and_group_buy_participations() {
        let direct_order = test_order("O000000000007", "U20002", OrderSource::Direct);
        let mut pending_plan = test_group_buy_plan(
            "G-USER-PENDING-001",
            "20260605200100",
            "regular_user",
            "未满单合买",
        );
        pending_plan.participants = vec![
            test_group_buy_participant("G-USER-PENDING-001-P001", "U10001"),
            test_group_buy_participant_with_amount("G-USER-PENDING-001-P002", "U20002", 2_000),
        ];

        let visible = user_visible_bet_orders(
            "U20002",
            vec![direct_order],
            &[pending_plan],
            &[],
            &[test_group_buy_lottery()],
        )
        .expect("用户注单列表可以合并未成单合买认购");

        let order_view =
            filter_user_bet_orders_by_view(visible.clone(), Some(UserBetOrderView::Orders));
        let group_buy_view =
            filter_user_bet_orders_by_view(visible, Some(UserBetOrderView::GroupBuy));

        assert_eq!(order_view.len(), 1);
        assert_eq!(order_view[0].order.id, "O000000000007");
        assert_eq!(group_buy_view.len(), 1);
        assert_eq!(group_buy_view[0].order.id, "GB-G-USER-PENDING-001");
    }

    #[test]
    /// 验证合买中奖注单会按当前用户认购比例展示个人派奖金额。
    fn user_visible_bet_orders_calculates_group_buy_participation_payout_by_share() {
        let group_order =
            test_won_order_with_payout("O000000000005", "U10001", OrderSource::GroupBuy, 1_900);
        let mut plan = test_group_buy_plan_with_order(
            "G-USER-ORDER-003",
            "O000000000005",
            vec![
                test_group_buy_participant_with_amount("G-USER-ORDER-003-P001", "U10001", 38_600),
                test_group_buy_participant_with_amount("G-USER-ORDER-003-P002", "U20002", 30_000),
            ],
        );
        plan.total_amount_minor = 68_600;
        plan.filled_amount_minor = 68_600;

        let visible = user_visible_bet_orders(
            "U20002",
            vec![group_order],
            &[plan],
            &[],
            &[test_group_buy_lottery()],
        )
        .expect("用户注单列表可以计算合买个人派奖金额");

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].participation_amount_minor, Some(30_000));
        assert_eq!(visible[0].participation_payout_minor, Some(831));
        assert_eq!(visible[0].order.payout_minor, 1_900);
    }

    #[test]
    /// 验证已有真实派奖流水时，用户注单优先展示流水里的个人入账金额。
    fn user_visible_bet_orders_uses_ledger_group_buy_participation_payout() {
        let group_order =
            test_won_order_with_payout("O000000000006", "U10001", OrderSource::GroupBuy, 1_900);
        let mut plan = test_group_buy_plan_with_order(
            "G-USER-ORDER-004",
            "O000000000006",
            vec![
                test_group_buy_participant_with_amount("G-USER-ORDER-004-P001", "U10001", 38_600),
                test_group_buy_participant_with_amount("G-USER-ORDER-004-P002", "U20002", 30_000),
            ],
        );
        plan.total_amount_minor = 68_600;
        plan.filled_amount_minor = 68_600;
        let ledger_entries = vec![test_ledger_entry_for_user(
            "L-GROUP-PAYOUT-001",
            "U20002",
            LedgerEntryKind::PayoutCredit,
            832,
            Some("S000001:O000000000006:G-USER-ORDER-004-P002"),
        )];

        let visible = user_visible_bet_orders(
            "U20002",
            vec![group_order],
            &[plan],
            &ledger_entries,
            &[test_group_buy_lottery()],
        )
        .expect("用户注单列表可以读取合买个人派奖流水");

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].participation_payout_minor, Some(832));
    }
    /// 构造合买接口测试使用的彩种。
    fn test_group_buy_lottery() -> LotteryKind {
        LotteryKind {
            id: "ssc60".to_string(),
            name: "测试彩".to_string(),
            category: "high-frequency".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Platform,
            api_draw_delay_seconds: 0,
            draw_control_enabled: true,
            avoid_winning_enabled: false,
            issue_format: crate::domain::lottery::DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            sale_close_lead_seconds: crate::domain::lottery::DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: true,
                min_share_amount_minor: 1_000,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1_000,
            },
            play_categories: vec![PlayCategory::Direct],
            play_configs: Vec::new(),
        }
    }
    /// 构造合买接口测试使用的计划。
    fn test_group_buy_plan(
        id: &str,
        issue: &str,
        initiator_username: &str,
        title: &str,
    ) -> GroupBuyPlan {
        GroupBuyPlan {
            id: id.to_string(),
            lottery_id: "ssc60".to_string(),
            lottery_name: "测试彩".to_string(),
            order_id: None,
            issue: issue.to_string(),
            rule_code: "fiveFrontDirect".to_string(),
            title: title.to_string(),
            numbers: "1|2|3".to_string(),
            initiator_user_id: "U90001".to_string(),
            initiator_username: initiator_username.to_string(),
            total_amount_minor: 5_000,
            filled_amount_minor: 1_000,
            min_share_amount_minor: 1_000,
            participant_min_amount_minor: 1_000,
            share_count: 5,
            status: GroupBuyPlanStatus::Open,
            participants: vec![GroupBuyParticipant {
                id: format!("{id}-P001"),
                user_id: "U90001".to_string(),
                username: initiator_username.to_string(),
                amount_minor: 1_000,
                share_count: 1,
                note: "发起人认购".to_string(),
                created_at: "2026-06-05 20:00:00".to_string(),
            }],
            note: String::new(),
            created_at: "2026-06-05 20:00:00".to_string(),
            updated_at: "2026-06-05 20:00:00".to_string(),
        }
    }
    /// 构造已成单的合买测试计划。
    fn test_group_buy_plan_with_order(
        id: &str,
        order_id: &str,
        participants: Vec<GroupBuyParticipant>,
    ) -> GroupBuyPlan {
        let mut plan = test_group_buy_plan(id, "20260605200000", "regular_user", "用户发起合买");
        plan.order_id = Some(order_id.to_string());
        plan.status = GroupBuyPlanStatus::Filled;
        plan.filled_amount_minor = plan.total_amount_minor;
        plan.participants = participants;
        plan
    }
    /// 构造合买测试参与人。
    fn test_group_buy_participant(id: &str, user_id: &str) -> GroupBuyParticipant {
        test_group_buy_participant_with_amount(id, user_id, 1_000)
    }
    /// 按金额构造合买测试参与人。
    fn test_group_buy_participant_with_amount(
        id: &str,
        user_id: &str,
        amount_minor: i64,
    ) -> GroupBuyParticipant {
        GroupBuyParticipant {
            id: id.to_string(),
            user_id: user_id.to_string(),
            username: format!("{user_id}_name"),
            amount_minor,
            share_count: u32::try_from(amount_minor / 1_000).unwrap_or(1),
            note: "测试认购".to_string(),
            created_at: "2026-06-05 20:00:00".to_string(),
        }
    }
    /// 构造用户端测试订单。
    fn test_order(id: &str, user_id: &str, order_source: OrderSource) -> OrderDetail {
        OrderDetail {
            id: id.to_string(),
            order_source,
            user_id: user_id.to_string(),
            lottery_id: "ssc60".to_string(),
            lottery_name: "测试彩".to_string(),
            issue: "20260605200000".to_string(),
            rule_code: PlayRuleCode::FiveFrontDirect,
            number_type: LotteryNumberType::FiveDigit,
            selection: PlaySelection {
                positions: vec![vec![1], vec![2], vec![3]],
                ..PlaySelection::default()
            },
            stake_count: 1,
            unit_amount_minor: 200,
            amount_minor: 200,
            odds_basis_points: 95_000,
            expanded_bets: vec!["123".to_string()],
            draw_number: None,
            matched_bets: Vec::new(),
            payout_minor: 0,
            status: OrderStatus::PendingDraw,
            settled_at: None,
            created_at: "2026-06-05 20:00:00".to_string(),
        }
    }
    /// 构造带派奖金额的中奖测试订单。
    fn test_won_order_with_payout(
        id: &str,
        user_id: &str,
        order_source: OrderSource,
        payout_minor: i64,
    ) -> OrderDetail {
        let mut order = test_order(id, user_id, order_source);
        order.payout_minor = payout_minor;
        order.status = OrderStatus::Won;
        order.settled_at = Some("2026-06-05 20:05:00".to_string());
        order
    }
    /// 构造邀请中心测试用户。
    fn test_invitation_user(
        id: &str,
        username: &str,
        kind: UserKind,
        agent_id: Option<String>,
    ) -> UserSummary {
        UserSummary {
            id: id.to_string(),
            username: username.to_string(),
            email: None,
            avatar_url: String::new(),
            contact_qq: String::new(),
            kind,
            status: UserStatus::Active,
            balance_minor: 0,
            agent_id,
            invite_code: "ABCDEFGH".to_string(),
            registration_location: crate::domain::user::UserRegistrationLocation::default(),
            created_at: "2026-06-05 18:00:00".to_string(),
        }
    }
    /// 构造邀请策略测试数据。
    fn test_invite_policy(agents_can_invite: bool) -> InvitePolicySummary {
        InvitePolicySummary {
            agents_can_invite,
            regular_users_can_invite: false,
            rebate_mode: RebateMode::Immediate,
            supported_rebate_modes: vec![RebateMode::Immediate, RebateMode::RechargeTiered],
            default_recharge_rebate_basis_points: 300,
        }
    }
    /// 构造测试用资金流水。
    fn test_ledger_entry(id: &str, kind: LedgerEntryKind, amount_minor: i64) -> LedgerEntry {
        test_ledger_entry_for_user(id, "U90012", kind, amount_minor, None)
    }
    /// 构造指定用户的测试提现订单。
    fn test_withdrawal_order_for_user(
        id: &str,
        user_id: &str,
        amount_minor: i64,
    ) -> WithdrawalOrderSummary {
        WithdrawalOrderSummary {
            id: id.to_string(),
            user_id: user_id.to_string(),
            username: format!("{user_id}_name"),
            method_id: "WM-TEST".to_string(),
            method_type: crate::domain::user::WithdrawalMethodType::BankCard,
            account_holder: "测试用户".to_string(),
            account_number: "6222000000000000".to_string(),
            bank_name: Some("测试银行".to_string()),
            amount_minor,
            status: crate::domain::withdrawal::WithdrawalOrderStatus::Approved,
            created_at: "2026-06-05 19:00:00".to_string(),
            reviewed_at: Some("2026-06-05 19:05:00".to_string()),
        }
    }
    /// 构造系统设置测试项。
    fn test_setting(key: &str, value: &str) -> SystemSetting {
        SystemSetting {
            key: key.to_string(),
            value: value.to_string(),
            description: "测试系统设置".to_string(),
        }
    }
    /// 构造指定用户的资金流水测试项。
    fn test_ledger_entry_for_user(
        id: &str,
        user_id: &str,
        kind: LedgerEntryKind,
        amount_minor: i64,
        reference_id: Option<&str>,
    ) -> LedgerEntry {
        LedgerEntry {
            id: id.to_string(),
            user_id: user_id.to_string(),
            kind,
            amount_minor,
            balance_after_minor: amount_minor.max(0),
            reference_id: reference_id.map(str::to_string),
            description: "测试流水".to_string(),
            created_at: "2026-06-05 19:00:00".to_string(),
        }
    }
}
