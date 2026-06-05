//! 用户接口路由，提供注册、登录、会话、账户与提款方式能力

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Form, Path, Query, Request, State},
    http::header::AUTHORIZATION,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Extension, Json, Router,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::{
    app::AppState,
    domain::advertisement::MobileAdvertisement,
    domain::draw::DrawIssueStatus,
    domain::finance::{FinancialAccountSummary, LedgerEntry},
    domain::group_buy::{
        AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyCreateOptions,
        GroupBuyCreateSettings, GroupBuyParticipationSummary, GroupBuyPlan, GroupBuyPlanStatus,
        GroupBuySelectOption, UserCreateGroupBuyPlanRequest, UserGroupBuyActionResponse,
        UserGroupBuyPlan, UserGroupBuyPlanPage, UserJoinGroupBuyPlanRequest,
    },
    domain::lottery::LotteryKind,
    domain::mobile::{
        MobileBetPageConfig, MobileCreateBetOrderBatchRequest, MobileCreateBetOrderBatchResponse,
        MobileSiteConfig,
    },
    domain::order::{CreateOrderRequest, OrderDetail},
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
    domain::withdrawal::{CreateWithdrawalOrderRequest, WithdrawalOrderSummary},
    error::{ApiError, ApiResult},
    response::ApiEnvelope,
    services::recharge::{
        recharge_config_response, recharge_settings_from_system_settings,
        support_ticket_for_recharge,
    },
    services::{
        business_database::enum_to_string,
        mobile_bet::build_mobile_bet_page_config,
        order::validate_draw_issue_accepts_order,
        play_rules::play_rule_summaries,
        realtime::{
            audience_matches, balance_changed_event, heartbeat_event, order_changed_event,
            recharge_changed_event, withdrawal_changed_event,
        },
    },
};

const MAX_USER_BET_BATCH_SIZE: usize = 50;
const REALTIME_HEARTBEAT_SECONDS: u64 = 30;

/// 组装并返回当前用户模块对应的路由树。
pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        .route("/me", get(get_current_user))
        .route("/logout", post(logout_user))
        .route("/bind-email", post(bind_email))
        .route("/password/change", post(change_password))
        .route("/balance", get(get_balance))
        .route("/ledger-entries", get(list_ledger_entries))
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    let account = state.finance.account_or_create(&user.id).await?;
    let user = user_with_account_balance(user, Some(&account));

    Ok(Json(ApiEnvelope::success(user)))
}

async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<UserLoginRequest>,
) -> ApiResult<Json<ApiEnvelope<UserAuthSession>>> {
    let mut session = state.access.login_user(payload).await?;
    session.user = user_with_financial_balance(&state, session.user).await?;

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
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserProfileResponse>>> {
    let user = user_with_financial_balance(&state, session.user).await?;
    Ok(Json(ApiEnvelope::success(UserProfileResponse { user })))
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
    let user = user_with_financial_balance(&state, user).await?;

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
    let user = user_with_financial_balance(&state, user).await?;

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

async fn list_ledger_entries(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<Vec<LedgerEntry>>>> {
    let entries = state.finance.user_ledger_entries(&session.user.id).await?;

    Ok(Json(ApiEnvelope::success(entries)))
}

async fn get_user_bet_page_config(
    State(state): State<AppState>,
    Path(lottery_id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<MobileBetPageConfig>>> {
    let lottery = state.lotteries.get(&lottery_id).await?;
    if !lottery.sale_enabled {
        return Err(ApiError::BadRequest("彩种已停售".to_string()));
    }
    let issues = state.draws.list_by_lottery_id(&lottery.id).await?;
    let config = build_mobile_bet_page_config(&lottery, issues);

    Ok(Json(ApiEnvelope::success(config)))
}

async fn list_user_bet_orders(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<Vec<OrderDetail>>>> {
    let orders = state
        .orders
        .list()
        .await?
        .into_iter()
        .filter(|order| order.user_id == session.user.id)
        .collect();

    Ok(Json(ApiEnvelope::success(orders)))
}

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
    let mut total_amount_minor = 0_i64;
    for item in payload.orders {
        let order_payload = CreateOrderRequest {
            user_id: session.user.id.clone(),
            lottery_id: item.lottery_id,
            issue: item.issue,
            rule_code: item.rule_code,
            selection: item.selection,
            unit_amount_minor: item.unit_amount_minor,
        };
        let lottery = state.lotteries.get(&order_payload.lottery_id).await?;
        let draw_issue = state
            .draws
            .get_by_lottery_issue(&order_payload.lottery_id, &order_payload.issue)
            .await?;
        validate_draw_issue_accepts_order(&draw_issue, &lottery, &order_payload.issue)?;
        let quote = state.orders.quote(&lottery, &order_payload).await?;
        total_amount_minor = total_amount_minor
            .checked_add(quote.amount_minor)
            .ok_or_else(|| ApiError::BadRequest("投注总金额过大".to_string()))?;
        checked_orders.push((lottery, order_payload));
    }

    state
        .finance
        .ensure_available(&session.user.id, total_amount_minor)
        .await?;

    let mut created_orders = Vec::with_capacity(checked_orders.len());
    for (lottery, order_payload) in checked_orders {
        let order = state.orders.create(&lottery, order_payload).await?;
        if let Err(error) = state.finance.debit_order(&order).await {
            if let Err(rollback_error) = state.orders.remove_unfunded(&order.id).await {
                tracing::error!(
                    order_id = %order.id,
                    error = %rollback_error.log_message(),
                    "扣款失败后移除用户下注订单失败"
                );
            }
            return Err(error);
        }
        publish_user_order_changed(&state, &order, "created");
        publish_user_balance_changed(&state, &order.user_id, "order_debit", Some(&order.id)).await;
        created_orders.push(order);
    }

    Ok(Json(ApiEnvelope::success(
        MobileCreateBetOrderBatchResponse {
            orders: created_orders,
        },
    )))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserGroupBuyListQuery {
    #[serde(default, alias = "lottery_code")]
    lottery_id: Option<String>,
    #[serde(default, alias = "group_code")]
    group_code: Option<String>,
}

async fn list_user_group_buy_plans(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Query(query): Query<UserGroupBuyListQuery>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyPlanPage>>> {
    let lotteries = state.lotteries.list().await?;
    let items = user_group_buy_plans(&state, &session.user.id, &lotteries, query).await?;

    Ok(Json(ApiEnvelope::success(UserGroupBuyPlanPage { items })))
}

async fn get_user_group_buy_plan(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyPlan>>> {
    let lotteries = state.lotteries.list().await?;
    let plan = state.group_buys.get(&id).await?;
    let plan = user_group_buy_plan(&plan, &lotteries, Some(&session.user.id))?;

    Ok(Json(ApiEnvelope::success(plan)))
}

async fn list_my_group_buy_plans(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyPlanPage>>> {
    let lotteries = state.lotteries.list().await?;
    let items = state
        .group_buys
        .list_details()
        .await?
        .into_iter()
        .filter(|plan| {
            plan.participants
                .iter()
                .any(|participant| participant.user_id == session.user.id)
        })
        .map(|plan| user_group_buy_plan(&plan, &lotteries, Some(&session.user.id)))
        .collect::<ApiResult<Vec<_>>>()?;

    Ok(Json(ApiEnvelope::success(UserGroupBuyPlanPage { items })))
}

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

async fn create_user_group_buy_plan(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Json(payload): Json<UserCreateGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyActionResponse>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    validate_lottery_accepts_group_buy(&lottery)?;
    validate_group_buy_issue_and_play(&state, &lottery, &payload.issue, &payload.rule_code).await?;
    validate_group_buy_numbers(&payload.numbers)?;
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
    let plan = state
        .group_buys
        .create(request, std::slice::from_ref(&lottery), &access.users)
        .await?;

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
        if let Err(rollback_error) = state.group_buys.remove_unfunded_plan(&plan.id).await {
            tracing::error!(
                group_buy_plan_id = %plan.id,
                error = %rollback_error.log_message(),
                "合买发起扣款失败后移除计划失败"
            );
        }
        return Err(error);
    }

    publish_user_balance_changed(
        &state,
        &session.user.id,
        "group_buy_debit",
        Some(&participant_id),
    )
    .await;
    let account = state.finance.account_or_create(&session.user.id).await?;
    let plan = user_group_buy_plan(&plan, &[lottery], Some(&session.user.id))?;

    Ok(Json(ApiEnvelope::success(UserGroupBuyActionResponse {
        plan,
        available_balance_minor: account.available_balance_minor,
    })))
}

async fn join_user_group_buy_plan(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
    Path(id): Path<String>,
    Json(payload): Json<UserJoinGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<UserGroupBuyActionResponse>>> {
    let existing = state.group_buys.get(&id).await?;
    let participant_id = next_group_buy_participant_id(&existing);
    state
        .finance
        .ensure_available(&session.user.id, payload.amount_minor)
        .await?;
    let access = state.access.snapshot().await?;
    let updated = state
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

    if let Err(error) = state
        .finance
        .debit_group_buy(&session.user.id, payload.amount_minor, &participant_id, &id)
        .await
    {
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

    publish_user_balance_changed(
        &state,
        &session.user.id,
        "group_buy_debit",
        Some(&participant_id),
    )
    .await;
    let account = state.finance.account_or_create(&session.user.id).await?;
    let lotteries = state.lotteries.list().await?;
    let plan = user_group_buy_plan(&updated, &lotteries, Some(&session.user.id))?;

    Ok(Json(ApiEnvelope::success(UserGroupBuyActionResponse {
        plan,
        available_balance_minor: account.available_balance_minor,
    })))
}

async fn user_group_buy_plans(
    state: &AppState,
    user_id: &str,
    lotteries: &[LotteryKind],
    query: UserGroupBuyListQuery,
) -> ApiResult<Vec<UserGroupBuyPlan>> {
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

    state
        .group_buys
        .list_details()
        .await?
        .into_iter()
        .filter(|plan| {
            if let Some(lottery_id) = lottery_id {
                if plan.lottery_id != lottery_id {
                    return false;
                }
            }
            if let Some(group_code) = group_code {
                let Some(lottery) = lotteries
                    .iter()
                    .find(|lottery| lottery.id == plan.lottery_id)
                else {
                    return false;
                };
                if lottery.category != group_code {
                    return false;
                }
            }
            matches!(
                plan.status,
                GroupBuyPlanStatus::Draft | GroupBuyPlanStatus::Open | GroupBuyPlanStatus::Filled
            )
        })
        .map(|plan| user_group_buy_plan(&plan, lotteries, Some(user_id)))
        .collect()
}

fn user_group_buy_plan(
    plan: &GroupBuyPlan,
    lotteries: &[LotteryKind],
    user_id: Option<&str>,
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
        category: lottery.map(|lottery| lottery.category.clone()),
        issue: plan.issue.clone(),
        rule_code: plan.rule_code.clone(),
        play_name,
        title: if plan.title.trim().is_empty() {
            format!("{} 第{}期合买", plan.lottery_name, plan.issue)
        } else {
            plan.title.clone()
        },
        numbers: plan.numbers.clone(),
        total_amount_minor: plan.total_amount_minor,
        share_count: plan.share_count,
        share_amount_minor: plan.min_share_amount_minor,
        filled_amount_minor: plan.filled_amount_minor,
        sold_shares,
        available_shares,
        progress_percent,
        status: plan.status.clone(),
        participant_count: plan.participants.len(),
        initiator_display: plan.initiator_username.clone(),
        my_participation,
        created_at: plan.created_at.clone(),
        updated_at: plan.updated_at.clone(),
    })
}

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

fn group_buy_enabled_lotteries(lotteries: &[LotteryKind]) -> Vec<LotteryKind> {
    lotteries
        .iter()
        .filter(|lottery| lottery.sale_enabled && lottery.group_buy.enabled)
        .cloned()
        .collect()
}

fn default_group_buy_create_settings() -> GroupBuyCreateSettings {
    GroupBuyCreateSettings {
        min_share_amount_minor: 100,
        initiator_min_percent: 10,
        participant_min_amount_minor: 100,
    }
}

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

fn validate_lottery_accepts_group_buy(lottery: &LotteryKind) -> ApiResult<()> {
    if !lottery.sale_enabled {
        return Err(ApiError::BadRequest("彩种已停售".to_string()));
    }
    if !lottery.group_buy.enabled {
        return Err(ApiError::BadRequest("彩种未开启合买".to_string()));
    }
    Ok(())
}

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

fn amount_to_share_count(amount_minor: i64, min_share_amount_minor: i64) -> ApiResult<u32> {
    if min_share_amount_minor <= 0 {
        return Ok(0);
    }
    u32::try_from(amount_minor / min_share_amount_minor)
        .map_err(|_| ApiError::BadRequest("合买份数过大".to_string()))
}

fn next_group_buy_plan_id() -> String {
    format!("G{}", chrono::Local::now().format("%Y%m%d%H%M%S%3f"))
}

fn next_group_buy_participant_id(plan: &GroupBuyPlan) -> String {
    format!(
        "{}-P{}",
        plan.id,
        chrono::Local::now().format("%Y%m%d%H%M%S%3f")
    )
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
    publish_user_recharge_changed(&state, &response.order);

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
    let order = state
        .recharges
        .confirm_rainbow_notify(params, &settings, &state.finance)
        .await?;
    publish_user_recharge_changed(&state, &order);
    publish_user_balance_changed(&state, &order.user_id, "recharge_credit", Some(&order.id)).await;

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

async fn list_withdrawal_orders(
    State(state): State<AppState>,
    Extension(session): Extension<UserAuthSession>,
) -> ApiResult<Json<ApiEnvelope<Vec<WithdrawalOrderSummary>>>> {
    let orders = state.withdrawals.list_for_user(&session.user.id).await?;

    Ok(Json(ApiEnvelope::success(orders)))
}

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

/// 推送用户提现订单变化事件，供手机端提现记录按需刷新。
fn publish_user_withdrawal_changed(state: &AppState, order: &WithdrawalOrderSummary) {
    state
        .realtime
        .publish_user(&order.user_id, withdrawal_changed_event(order));
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
