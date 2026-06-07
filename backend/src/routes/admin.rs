//! 管理后台 API 路由总控，汇总和注册所有后台接口

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Multipart, Path, Query, Request, State},
    http::header::AUTHORIZATION,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post, put},
    Extension, Json, Router,
};
use chrono::Local;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::BTreeMap, time::Duration};

use crate::{
    app::AppState,
    domain::{
        advertisement::{AdvertisementSummary, SaveAdvertisementRequest},
        auth::{AdminAuthSession, AdminLoginRequest, AdminLogoutResponse, CurrentAdminProfile},
        draw::{
            CreateDrawIssueRequest, DrawAutomationRun, DrawAutomationRunRequest,
            DrawControlTargetScope, DrawIssue, DrawIssueGenerationPreview, DrawIssuePage,
            DrawIssueResultRequest, DrawIssueStatus, GenerateDrawIssueRequest,
            GenerateDrawIssuesRequest, LotteryDrawControl, SaveLotteryDrawControlRequest,
        },
        finance::{
            AdminFinancialAccountSummary, FinanceOverview, FinancePage, FinancialAccountSummary,
            LedgerEntry, LedgerEntryKind, ManualBalanceAdjustmentRequest,
        },
        group_buy::{
            AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyPlan,
            GroupBuyPlanSummary, UpdateGroupBuyPlanRequest,
        },
        invite::{CreateInviteRecordRequest, InviteRecord, UpdateInviteRecordRequest},
        lottery::{
            DrawMode, DrawSource, LotteryCategoryConfig, LotteryKind, SaveDrawSourceRequest,
        },
        order::{CreateOrderRequest, OrderDetail, OrderSource, OrderStatus},
        permission::{AdminRole, PermissionScope, SystemSetting, UpdateSystemSettingRequest},
        play::{PlayRuleEvaluateRequest, PlayRuleEvaluation, PlayRuleSummary},
        rebate::{InvitePolicySummary, InvitePolicyUpdateRequest},
        recharge::{ConfirmRechargeOrderRequest, RechargeOrderSummary},
        robot::{GroupBuyRobotRun, RobotConfigSummary, RobotStatusRequest},
        settlement::SettlementRun,
        support::{
            CreateSupportConversationRequest, SupportConversation, SupportReplyRequest,
            UpdateSupportConversationRequest,
        },
        user::{
            AdminPasswordResetRequest, AdminSaveRequest, AdminStatusRequest, AdminSummary,
            RegistrationConfig, UserStatusRequest, UserSummary,
        },
        withdrawal::WithdrawalOrderSummary,
    },
    error::{ApiError, ApiResult},
    response::ApiEnvelope,
    services::{
        automation::run_draw_automation,
        dashboard::{
            dashboard_summary_for_scopes, dashboard_summary_with_orders, DashboardSummary,
        },
        draw_generation::{
            generate_draw_issue_batch, generate_next_draw_issue, preview_draw_issue_generation,
        },
        group_buy_flow::{build_group_buy_order_request, create_order_for_filled_group_buy},
        group_buy_robot::{is_group_buy_robot_user_id, run_group_buy_robots},
        image_bed::{upload_configured_image_bed_file, ImageBedUploadOptions},
        order::validate_draw_issue_accepts_order,
        play_rules::{evaluate_play_rule, play_rule_summaries},
        realtime::{
            admin_audience_matches, balance_changed_event, draw_result_event, heartbeat_event,
            issue_closed_event, issue_opened_event, order_changed_event, recharge_changed_event,
            support_conversation_updated_event, support_message_created_event,
            withdrawal_changed_event,
        },
        rebate::credit_recharge_rebate_for_order,
        scheduler::DrawSchedulerConfig,
        scheduler::DrawSchedulerStatus,
    },
};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const ADMIN_REALTIME_HEARTBEAT_SECONDS: u64 = 30;

/// 组装并返回当前模块对应的路由树。
pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        .route("/dashboard", get(get_dashboard_summary))
        .route("/finance-overview", get(get_finance_overview))
        .route("/financial-accounts", get(list_financial_accounts))
        .route("/ledger-entries", get(list_ledger_entries))
        .route("/recharge-orders", get(list_recharge_orders))
        .route(
            "/recharge-orders/{id}/confirm",
            post(confirm_recharge_order),
        )
        .route("/withdrawal-orders", get(list_withdrawal_orders))
        .route(
            "/withdrawal-orders/{id}/approve",
            post(approve_withdrawal_order),
        )
        .route(
            "/withdrawal-orders/{id}/reject",
            post(reject_withdrawal_order),
        )
        .route("/financial-adjustments", post(manual_balance_adjustment))
        .route(
            "/group-buy/plans",
            get(list_group_buy_plans).post(create_group_buy_plan),
        )
        .route(
            "/group-buy/plans/{id}",
            get(get_group_buy_plan).put(update_group_buy_plan),
        )
        .route(
            "/group-buy/plans/{id}/participants",
            post(add_group_buy_participant),
        )
        .route(
            "/invitations",
            get(list_invitations).post(create_invitation),
        )
        .route(
            "/invitations/{id}",
            get(get_invitation).put(update_invitation),
        )
        .route(
            "/support/conversations",
            get(list_support_conversations).post(create_support_conversation),
        )
        .route(
            "/support/conversations/{id}",
            get(get_support_conversation).put(update_support_conversation),
        )
        .route(
            "/support/conversations/{id}/messages",
            post(reply_support_conversation),
        )
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", get(get_user).put(update_user))
        .route("/users/{id}/status", patch(set_user_status))
        .route("/admins", get(list_admins).post(create_admin))
        .route("/admins/{id}", get(get_admin).put(update_admin))
        .route("/admins/{id}/password", patch(reset_admin_password))
        .route("/admins/{id}/status", patch(set_admin_status))
        .route("/roles", get(list_roles).post(create_role))
        .route(
            "/roles/{id}",
            get(get_role).put(update_role).delete(delete_role),
        )
        .route("/system-settings", get(list_system_settings))
        .route("/system-settings/{key}", patch(update_system_setting))
        .route("/image-bed/upload", post(upload_image_bed_file))
        .route(
            "/advertisements",
            get(list_advertisements).post(create_advertisement),
        )
        .route(
            "/advertisements/{id}",
            get(get_advertisement)
                .put(update_advertisement)
                .delete(delete_advertisement),
        )
        .route(
            "/registration",
            get(get_registration_config).put(update_registration_config),
        )
        .route(
            "/invite-policy",
            get(get_invite_policy).put(update_invite_policy),
        )
        .route("/robots", get(list_robots).post(create_robot))
        .route("/robots/run", post(run_group_buy_robots_request))
        .route(
            "/robots/{id}",
            get(get_robot).put(update_robot).delete(delete_robot),
        )
        .route("/robots/{id}/status", patch(set_robot_status))
        .route(
            "/draw-sources",
            get(list_draw_sources).post(create_draw_source),
        )
        .route(
            "/draw-sources/{id}",
            put(update_draw_source).delete(delete_draw_source),
        )
        .route("/draw-controls", get(list_lottery_draw_controls))
        .route(
            "/draw-controls/{lottery_id}",
            get(get_lottery_draw_control).put(save_lottery_draw_control),
        )
        .route(
            "/draw-issues",
            get(list_draw_issues).post(create_draw_issue),
        )
        .route(
            "/draw-issues/generate-next",
            post(generate_next_draw_issue_request),
        )
        .route(
            "/draw-issues/preview-generation",
            post(preview_draw_issue_generation_request),
        )
        .route(
            "/draw-issues/generate-batch",
            post(generate_draw_issue_batch_request),
        )
        .route("/draw-issues/{id}", get(get_draw_issue))
        .route("/draw-issues/{id}/close", patch(close_draw_issue))
        .route("/draw-issues/{id}/draw", patch(draw_issue_result))
        .route("/draw-issues/{id}/cancel", patch(cancel_draw_issue))
        .route("/draw-scheduler/status", get(get_draw_scheduler_status))
        .route("/draw-scheduler/config", put(update_draw_scheduler_config))
        .route("/draw-automation/run", post(run_draw_automation_request))
        .route("/settlements", get(list_settlements))
        .route("/settlements/{id}", get(get_settlement))
        .route(
            "/settlements/draw-issues/{id}",
            post(settle_draw_issue_orders),
        )
        .route("/play-rules", get(list_play_rules))
        .route("/play-rules/evaluate", post(evaluate_play_rule_request))
        .route("/orders", get(list_orders).post(create_order))
        .route("/orders/{id}", get(get_order))
        .route("/orders/{id}/cancel", patch(cancel_order))
        .route("/lotteries", get(list_lotteries).post(create_lottery))
        .route(
            "/lotteries/{id}",
            get(get_lottery).put(update_lottery).delete(delete_lottery),
        )
        .route("/lotteries/{id}/sale", patch(set_lottery_sale))
        .route(
            "/lottery-categories",
            get(list_lottery_categories).post(create_lottery_category),
        )
        .route(
            "/lottery-categories/{code}",
            put(update_lottery_category).delete(delete_lottery_category),
        )
        .route("/auth/me", get(get_current_admin))
        .route("/auth/logout", post(logout_admin))
        .route_layer(middleware::from_fn_with_state(state, require_admin_auth));

    Router::new()
        .route("/auth/login", post(login_admin))
        .route("/realtime", get(open_admin_realtime_socket))
        .merge(protected_routes)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台实时连接查询参数，浏览器 WebSocket 通过 token 查询参数完成鉴权。
struct AdminRealtimeQuery {
    token: Option<String>,
}

/// 建立后台实时事件连接；浏览器 WebSocket 不能设置 Authorization，所以通过查询参数校验管理员 token。
async fn open_admin_realtime_socket(
    State(state): State<AppState>,
    Query(query): Query<AdminRealtimeQuery>,
    ws: WebSocketUpgrade,
) -> ApiResult<Response> {
    let token = query
        .token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::Unauthorized("后台实时连接 token 不能为空".to_string()))?;
    let session = state.access.session_from_token(token).await?;
    if !session.scopes.contains(&PermissionScope::CustomerService) {
        return Err(ApiError::Forbidden("后台实时客服权限不足".to_string()));
    }

    let realtime = state.realtime.clone();
    Ok(ws
        .on_upgrade(move |socket| handle_admin_realtime_socket(socket, realtime, session.admin.id))
        .into_response())
}

/// 持续向单个后台连接发送实时事件和心跳。
async fn handle_admin_realtime_socket(
    mut socket: WebSocket,
    realtime: crate::services::realtime::RealtimeHub,
    admin_id: String,
) {
    let mut receiver = realtime.subscribe();
    let mut heartbeat =
        tokio::time::interval(Duration::from_secs(ADMIN_REALTIME_HEARTBEAT_SECONDS));

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
                        if admin_audience_matches(&message.audience)
                            && send_realtime_payload(&mut socket, message.payload).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped_count)) => {
                        tracing::warn!(
                            admin_id = %admin_id,
                            skipped_count,
                            "后台实时事件连接消费过慢，已跳过部分历史事件"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
}

/// 将实时事件 JSON 发送到后台 WebSocket 连接。
async fn send_realtime_payload(
    socket: &mut WebSocket,
    payload: serde_json::Value,
) -> Result<(), axum::Error> {
    socket.send(Message::Text(payload.to_string().into())).await
}

/// 校验后台接口的管理员登录态和权限范围。
async fn require_admin_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> ApiResult<Response> {
    let token = bearer_token(&request)?;
    let session = state.access.session_from_token(token).await?;
    if let Some(required_scope) = required_scope_for_path(request.uri().path()) {
        if !session.scopes.contains(&required_scope) {
            return Err(ApiError::Forbidden(format!(
                "permission `{required_scope:?}` is required"
            )));
        }
    }

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

/// 处理 required_scope_for_path 的具体内部流程。
fn required_scope_for_path(path: &str) -> Option<PermissionScope> {
    let path = path.trim_start_matches('/');
    let path = path.strip_prefix("admin/").unwrap_or(path);

    if path.starts_with("auth/") || path == "dashboard" {
        return None;
    }
    if path.starts_with("users") || path.starts_with("registration") {
        return Some(PermissionScope::Users);
    }
    if path.starts_with("admins") {
        return Some(PermissionScope::Admins);
    }
    if path.starts_with("roles") {
        return Some(PermissionScope::Roles);
    }
    if path.starts_with("system-settings") {
        return Some(PermissionScope::SystemSettings);
    }
    if path.starts_with("image-bed") {
        return Some(PermissionScope::SystemSettings);
    }
    if path.starts_with("advertisements") {
        return Some(PermissionScope::SystemSettings);
    }
    if path.starts_with("orders") || path.starts_with("settlements") {
        return Some(PermissionScope::Orders);
    }
    if path.starts_with("financial-")
        || path.starts_with("ledger-entries")
        || path.starts_with("recharge-orders")
        || path.starts_with("withdrawal-orders")
        || path.starts_with("finance")
    {
        return Some(PermissionScope::Finance);
    }
    if path.starts_with("support") {
        return Some(PermissionScope::CustomerService);
    }
    if path.starts_with("robots") {
        return Some(PermissionScope::Robots);
    }
    if path.starts_with("invitations")
        || path.starts_with("invite-policy")
        || path.starts_with("rebate")
    {
        return Some(PermissionScope::Rebates);
    }
    if path.starts_with("lottery-categories") {
        return Some(PermissionScope::Lotteries);
    }
    if path.starts_with("draw")
        || path.starts_with("lotteries")
        || path.starts_with("group-buy")
        || path.starts_with("play-rules")
    {
        return Some(PermissionScope::Lotteries);
    }

    None
}

/// 后台管理员登录接口，返回管理员会话和权限范围。
async fn login_admin(
    State(state): State<AppState>,
    Json(payload): Json<AdminLoginRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminAuthSession>>> {
    let session = state.access.login(payload).await?;

    Ok(Json(ApiEnvelope::success(session)))
}

/// 返回当前管理员资料，用于后台刷新登录态。
async fn get_current_admin(
    Extension(session): Extension<AdminAuthSession>,
) -> ApiResult<Json<ApiEnvelope<CurrentAdminProfile>>> {
    Ok(Json(ApiEnvelope::success(session.profile())))
}

/// 注销当前管理员会话。
async fn logout_admin(
    State(state): State<AppState>,
    Extension(session): Extension<AdminAuthSession>,
) -> ApiResult<Json<ApiEnvelope<AdminLogoutResponse>>> {
    state.access.logout(&session.token).await?;

    Ok(Json(ApiEnvelope::success(AdminLogoutResponse {
        logged_out: true,
    })))
}

/// 后台手动触发一轮开奖自动化。
async fn run_draw_automation_request(
    State(state): State<AppState>,
    Json(payload): Json<DrawAutomationRunRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawAutomationRun>>> {
    let run = run_draw_automation(
        &state.draws,
        &state.lotteries,
        &state.orders,
        &state.finance,
        &state.group_buys,
        payload,
    )
    .await?;
    publish_draw_automation_events(&state, &run).await;

    Ok(Json(ApiEnvelope::success(run)))
}

/// 返回开奖调度器当前配置、运行状态和最近执行记录。
async fn get_draw_scheduler_status(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<DrawSchedulerStatus>>> {
    let status = state.scheduler.status()?;

    Ok(Json(ApiEnvelope::success(status)))
}

/// 后台保存开奖调度配置，并按开关启动或停止调度器。
async fn update_draw_scheduler_config(
    State(state): State<AppState>,
    Json(payload): Json<DrawSchedulerConfig>,
) -> ApiResult<Json<ApiEnvelope<DrawSchedulerStatus>>> {
    let status = state.scheduler.update_config(payload).await?;

    Ok(Json(ApiEnvelope::success(status)))
}

/// 返回后台开奖源列表。
async fn list_draw_sources(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawSource>>>> {
    let sources = state.draws.draw_sources().await?;

    Ok(Json(ApiEnvelope::success(sources)))
}

/// 创建新的外部 API 开奖源。
async fn create_draw_source(
    State(state): State<AppState>,
    Json(payload): Json<SaveDrawSourceRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawSource>>> {
    let lotteries = state.lotteries.list().await?;
    let source = state.draws.create_draw_source(payload, &lotteries).await?;

    Ok(Json(ApiEnvelope::success(source)))
}

/// 更新指定开奖源配置。
async fn update_draw_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SaveDrawSourceRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawSource>>> {
    let lotteries = state.lotteries.list().await?;
    let source = state
        .draws
        .update_draw_source(&id, payload, &lotteries)
        .await?;

    Ok(Json(ApiEnvelope::success(source)))
}

/// 删除指定开奖源配置。
async fn delete_draw_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawSource>>> {
    let source = state.draws.delete_draw_source(&id).await?;

    Ok(Json(ApiEnvelope::success(source)))
}

/// 返回所有彩种开奖控制配置。
async fn list_lottery_draw_controls(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LotteryDrawControl>>>> {
    let lotteries = state.lotteries.list().await?;
    let controls = state.draws.list_draw_controls(&lotteries).await?;

    Ok(Json(ApiEnvelope::success(controls)))
}

/// 返回单个彩种的开奖控制配置。
async fn get_lottery_draw_control(
    State(state): State<AppState>,
    Path(lottery_id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryDrawControl>>> {
    let lottery = state.lotteries.get(&lottery_id).await?;
    let control = state.draws.get_draw_control(&lottery).await?;

    Ok(Json(ApiEnvelope::success(control)))
}

/// 保存彩种开奖控制号码和控制范围。
async fn save_lottery_draw_control(
    State(state): State<AppState>,
    Path(lottery_id): Path<String>,
    Json(mut payload): Json<SaveLotteryDrawControlRequest>,
) -> ApiResult<Json<ApiEnvelope<LotteryDrawControl>>> {
    let lottery = state.lotteries.get(&lottery_id).await?;
    normalize_admin_draw_control_target(&state, &lottery, &mut payload).await?;
    let control = state.draws.save_draw_control(&lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(control)))
}

/// 校验后台开奖控制目标，并在选择订单时把目标期号补齐到请求中。
async fn normalize_admin_draw_control_target(
    state: &AppState,
    lottery: &LotteryKind,
    payload: &mut SaveLotteryDrawControlRequest,
) -> ApiResult<()> {
    if !payload.enabled {
        payload.target_scope = DrawControlTargetScope::Lottery;
        payload.target_issue = None;
        payload.target_order_id = None;
        return Ok(());
    }

    match payload.target_scope {
        DrawControlTargetScope::Lottery => {
            payload.target_issue = None;
            payload.target_order_id = None;
            Ok(())
        }
        DrawControlTargetScope::Issue => {
            let issue = required_admin_control_value(payload.target_issue.as_deref(), "控制期号")?;
            state
                .draws
                .get_by_lottery_issue(&lottery.id, &issue)
                .await?;
            payload.target_issue = Some(issue);
            payload.target_order_id = None;
            Ok(())
        }
        DrawControlTargetScope::Order => {
            let order_id =
                required_admin_control_value(payload.target_order_id.as_deref(), "目标订单")?;
            let order = state.orders.get(&order_id).await?;
            if order.lottery_id != lottery.id {
                return Err(ApiError::BadRequest("目标订单不属于当前彩种".to_string()));
            }
            if order.status != OrderStatus::PendingDraw {
                return Err(ApiError::BadRequest("只能控制待开奖订单".to_string()));
            }
            if let Some(target_issue) = payload
                .target_issue
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                if target_issue != order.issue {
                    return Err(ApiError::BadRequest(
                        "目标订单期号与控制期号不一致".to_string(),
                    ));
                }
            }
            payload.target_issue = Some(order.issue);
            payload.target_order_id = Some(order.id);
            Ok(())
        }
    }
}

/// 读取后台开奖控制目标字段，启用对应范围时不能为空。
fn required_admin_control_value(value: Option<&str>, label: &str) -> ApiResult<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| ApiError::BadRequest(format!("{label}不能为空")))
}

/// 按筛选条件返回后台期号分页列表。
async fn list_draw_issues(
    State(state): State<AppState>,
    Query(query): Query<DrawIssueListQuery>,
) -> ApiResult<Json<ApiEnvelope<DrawIssuePage>>> {
    let issues = if let Some(lottery_id) = query.lottery_id {
        let lottery_id = lottery_id.trim().to_string();
        if lottery_id.is_empty() {
            state.draws.list().await?
        } else {
            state.draws.list_by_lottery_id(&lottery_id).await?
        }
    } else {
        state.draws.list().await?
    };

    let total_count = issues.len();
    let (page, page_size, start, end) = if query.page.is_none() && query.page_size.is_none() {
        (1usize, total_count.max(1), 0usize, total_count)
    } else {
        let page_size = query.page_size.unwrap_or(20).max(1);
        let max_page = if total_count == 0 {
            1
        } else {
            total_count.div_ceil(page_size)
        };
        let page = query.page.unwrap_or(1).max(1).min(max_page);
        let start = (page - 1).saturating_mul(page_size);
        let end = (start + page_size).min(total_count);
        (page, page_size, start, end)
    };
    let total_pages = if total_count == 0 {
        0
    } else {
        total_count.div_ceil(page_size)
    };
    let items = issues[start..end].to_vec();

    Ok(Json(ApiEnvelope::success(DrawIssuePage {
        items,
        page,
        page_size,
        total_count,
        total_pages,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台期号列表筛选和分页查询参数。
struct DrawIssueListQuery {
    lottery_id: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台财务、订单、合买等列表通用分页参数。
struct FinancePageQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    include_robot_data: Option<bool>,
}

/// 后台财务、订单、合买等列表通用分页参数。
impl FinancePageQuery {
    /// 后台列表默认隐藏机器人账户和机器人流水，只有显式打开开关时才返回。
    fn include_robot_data(&self) -> bool {
        self.include_robot_data.unwrap_or(false)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台列表是否包含机器人数据的查询参数。
struct RobotDataQuery {
    include_robot_data: Option<bool>,
}

/// 后台列表是否包含机器人数据的查询参数。
impl RobotDataQuery {
    /// 后台查询默认隐藏机器人数据，避免运营统计被系统机器人干扰。
    fn include_robot_data(&self) -> bool {
        self.include_robot_data.unwrap_or(false)
    }
}

/// 按通用分页参数裁剪列表并生成分页响应。
fn page_items<T>(items: Vec<T>, query: FinancePageQuery) -> FinancePage<T> {
    let total_count = items.len();
    let (page, page_size, start, end) = if query.page.is_none() && query.page_size.is_none() {
        (1usize, total_count.max(1), 0usize, total_count)
    } else {
        let page_size = query.page_size.unwrap_or(20).max(1);
        let max_page = if total_count == 0 {
            1
        } else {
            total_count.div_ceil(page_size)
        };
        let page = query.page.unwrap_or(1).max(1).min(max_page);
        let start = (page - 1).saturating_mul(page_size);
        let end = (start + page_size).min(total_count);
        (page, page_size, start, end)
    };
    let total_pages = if total_count == 0 {
        0
    } else {
        total_count.div_ceil(page_size)
    };

    FinancePage {
        items: items.into_iter().skip(start).take(end - start).collect(),
        page,
        page_size,
        total_count,
        total_pages,
    }
}

/// 返回指定开奖期号详情。
async fn get_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.get(&id).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

/// 后台手动创建开奖期号。
async fn create_draw_issue(
    State(state): State<AppState>,
    Json(payload): Json<CreateDrawIssueRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issue = state.draws.create(&lottery, payload).await?;
    state.realtime.publish_public(issue_opened_event(&issue));

    Ok(Json(ApiEnvelope::success(issue)))
}

/// 后台生成单个下一期开奖期号。
async fn generate_next_draw_issue_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssueRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issue = generate_next_draw_issue(&state.draws, &lottery, payload).await?;
    state.realtime.publish_public(issue_opened_event(&issue));

    Ok(Json(ApiEnvelope::success(issue)))
}

/// 预览即将生成的下一期开奖期号，不落库。
async fn preview_draw_issue_generation_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssuesRequest>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawIssueGenerationPreview>>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let plans = preview_draw_issue_generation(&state.draws, &lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(plans)))
}

/// 后台批量生成未来期开奖期号。
async fn generate_draw_issue_batch_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssuesRequest>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawIssue>>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issues = generate_draw_issue_batch(&state.draws, &lottery, payload).await?;
    for issue in &issues {
        state.realtime.publish_public(issue_opened_event(issue));
    }

    Ok(Json(ApiEnvelope::success(issues)))
}

/// 后台手动封盘指定期号。
async fn close_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.close(&id).await?;
    state.realtime.publish_public(issue_closed_event(&issue));

    Ok(Json(ApiEnvelope::success(issue)))
}

/// 后台手动录入或控制指定期号开奖结果。
async fn draw_issue_result(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DrawIssueResultRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.draw(&id, payload).await?;
    state.realtime.publish_public(draw_result_event(&issue));

    Ok(Json(ApiEnvelope::success(issue)))
}

/// 后台取消尚未结算的开奖期号。
async fn cancel_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.cancel(&id).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

/// 返回计奖派奖批次分页列表。
async fn list_settlements(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<SettlementRun>>>> {
    // 计奖派奖批次会持续增长，后台列表按统一分页信封返回。
    let settlements = state.orders.settlement_runs().await?;

    Ok(Json(ApiEnvelope::success(page_items(settlements, query))))
}

/// 返回指定计奖派奖批次详情。
async fn get_settlement(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SettlementRun>>> {
    let settlement = state.orders.get_settlement(&id).await?;

    Ok(Json(ApiEnvelope::success(settlement)))
}

/// 后台触发指定已开奖期号的订单结算。
async fn settle_draw_issue_orders(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SettlementRun>>> {
    let draw_issue = state.draws.get(&id).await?;
    let (settlement, entries) = state
        .orders
        .settle_with_payouts(&state.finance, &state.group_buys, &draw_issue)
        .await?;
    publish_settlement_balance_events(&state, &entries).await;

    Ok(Json(ApiEnvelope::success(settlement)))
}

/// 推送一次开奖自动化产生的公开事件和用户余额事件。
async fn publish_draw_automation_events(state: &AppState, run: &DrawAutomationRun) {
    for issue in &run.closed_issues {
        state.realtime.publish_public(issue_closed_event(issue));
    }
    for issue in &run.drawn_issues {
        state.realtime.publish_public(draw_result_event(issue));
    }
    publish_settlement_balance_events(state, &run.ledger_entries).await;
}

/// 推送结算产生的用户余额变化事件。
async fn publish_settlement_balance_events(state: &AppState, entries: &[LedgerEntry]) {
    for entry in entries {
        publish_user_balance_changed(
            state,
            &entry.user_id,
            "settlement",
            entry.reference_id.as_deref(),
        )
        .await;
    }
}

/// 推送用户余额变化事件，读取资金账户失败只记录日志，不影响管理员操作结果。
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
            "后台推送用户余额变化时读取资金账户失败"
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

/// 推送客服消息新增事件，后台客服页和用户客服页都会收到同一条消息通知。
fn publish_support_message_created(state: &AppState, conversation: &SupportConversation) {
    let Some(message) = conversation.messages.last() else {
        return;
    };
    let event = support_message_created_event(conversation, message);
    state
        .realtime
        .publish_user(&conversation.user_id, event.clone());
    state.realtime.publish_admin(event);
}

/// 推送客服会话更新事件，保证状态和分配客服变更能实时同步。
fn publish_support_conversation_updated(state: &AppState, conversation: &SupportConversation) {
    let event = support_conversation_updated_event(conversation);
    state
        .realtime
        .publish_user(&conversation.user_id, event.clone());
    state.realtime.publish_admin(event);
}

/// 推送用户提现订单变化事件，供手机端提现记录按需刷新。
fn publish_user_withdrawal_changed(state: &AppState, order: &WithdrawalOrderSummary) {
    state
        .realtime
        .publish_user(&order.user_id, withdrawal_changed_event(order));
}

/// 返回后台可配置的玩法规则目录。
async fn list_play_rules() -> ApiResult<Json<ApiEnvelope<Vec<PlayRuleSummary>>>> {
    Ok(Json(ApiEnvelope::success(play_rule_summaries())))
}

/// 后台验证玩法选号、注数和中奖匹配项。
async fn evaluate_play_rule_request(
    Json(payload): Json<PlayRuleEvaluateRequest>,
) -> ApiResult<Json<ApiEnvelope<PlayRuleEvaluation>>> {
    Ok(Json(ApiEnvelope::success(evaluate_play_rule(payload)?)))
}

/// 返回后台首页仪表盘汇总数据。
async fn get_dashboard_summary(
    State(state): State<AppState>,
    Extension(session): Extension<AdminAuthSession>,
) -> ApiResult<Json<ApiEnvelope<DashboardSummary>>> {
    let lotteries = state.lotteries.list().await?;
    let recent_orders = state.orders.recent_summaries(8).await?;
    let finance = state.finance.overview().await?;
    let financial_accounts = state.finance.accounts().await?;
    let access = state.access.snapshot().await?;
    let invite_policy = state.rebates.get().await?;
    let robots = state.robots.list().await?;
    let group_buy_plans = state.group_buys.list().await?;
    let draw_sources = state.draws.draw_sources().await?;

    let summary = dashboard_summary_with_orders(
        lotteries,
        draw_sources,
        recent_orders,
        group_buy_plans,
        finance,
        financial_accounts,
        access,
        invite_policy,
        robots,
    );
    let summary = dashboard_summary_for_scopes(summary, &session.scopes);

    Ok(Json(ApiEnvelope::success(summary)))
}

/// 返回后台合买计划分页列表。
async fn list_group_buy_plans(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<GroupBuyPlanSummary>>>> {
    let plans = state.group_buys.list().await?;

    Ok(Json(ApiEnvelope::success(page_items(plans, query))))
}

/// 返回指定合买计划详情。
async fn get_group_buy_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let plan = state.group_buys.get(&id).await?;

    Ok(Json(ApiEnvelope::success(plan)))
}

/// 后台创建合买计划。
async fn create_group_buy_plan(
    State(state): State<AppState>,
    Json(payload): Json<CreateGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    build_group_buy_order_request(
        &state.draws,
        &state.orders,
        &lottery,
        &payload.initiator_user_id,
        &payload.issue,
        &payload.rule_code,
        &payload.numbers,
        payload.total_amount_minor,
    )
    .await?;
    let access = state.access.snapshot().await?;
    let mut plan = state
        .group_buys
        .create(payload, std::slice::from_ref(&lottery), &access.users)
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
                    "后台合买满单成单失败后移除计划失败"
                );
            }
            return Err(error);
        }
    };
    if let Some((_, attached_plan)) = &created_order {
        plan = attached_plan.clone();
    }
    let participant_id = format!("{}-P001", plan.id);
    if let Err(error) = state
        .finance
        .debit_group_buy(
            &plan.initiator_user_id,
            plan.filled_amount_minor,
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
                    "后台创建合买扣款失败后移除满单订单失败"
                );
            }
        }
        if let Err(rollback_error) = state.group_buys.remove_unfunded_plan(&plan.id).await {
            tracing::error!(
                group_buy_plan_id = %plan.id,
                error = %rollback_error.log_message(),
                "后台创建合买扣款失败后移除计划失败"
            );
        }
        return Err(error);
    }
    if let Some((order, _)) = &created_order {
        publish_user_order_changed(&state, order, "created");
    }
    publish_user_balance_changed(
        &state,
        &plan.initiator_user_id,
        "group_buy_debit",
        Some(&participant_id),
    )
    .await;

    Ok(Json(ApiEnvelope::success(plan)))
}

/// 后台更新合买计划状态和备注。
async fn update_group_buy_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let existing = state.group_buys.get(&id).await?;
    if payload.status == crate::domain::group_buy::GroupBuyPlanStatus::Cancelled {
        if let Some(order_id) = existing.order_id.as_deref() {
            let order = state.orders.get(order_id).await?;
            if order.status != OrderStatus::PendingDraw {
                return Err(ApiError::BadRequest(
                    "已开奖或已取消的合买订单不能取消".to_string(),
                ));
            }
        }
    }
    let plan = state.group_buys.update(&id, payload).await?;
    if plan.status == crate::domain::group_buy::GroupBuyPlanStatus::Cancelled
        && existing.status != crate::domain::group_buy::GroupBuyPlanStatus::Cancelled
    {
        if let Some(order_id) = existing.order_id.or_else(|| plan.order_id.clone()) {
            let order = state.orders.cancel(&order_id).await?;
            publish_user_order_changed(&state, &order, "cancelled");
        }
        let entries = state
            .finance
            .refund_group_buy_plan(&plan, "后台取消合买")
            .await?;
        publish_settlement_balance_events(&state, &entries).await;
    }

    Ok(Json(ApiEnvelope::success(plan)))
}

/// 后台为合买计划添加参与人。
async fn add_group_buy_participant(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AddGroupBuyParticipantRequest>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let existing = state.group_buys.get(&id).await?;
    let lottery = state.lotteries.get(&existing.lottery_id).await?;
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
    let participant_id = payload.id.clone();
    let participant_user_id = payload.user_id.clone();
    let participant_amount_minor = payload.amount_minor;
    let access = state.access.snapshot().await?;
    let mut plan = state
        .group_buys
        .add_participant(&id, payload, &access.users)
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
            if let Err(rollback_error) = state
                .group_buys
                .remove_unfunded_participant(&id, &participant_id)
                .await
            {
                tracing::error!(
                    group_buy_plan_id = %id,
                    group_buy_participant_id = %participant_id,
                    error = %rollback_error.log_message(),
                    "后台合买满单成单失败后移除参与记录失败"
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
            &participant_user_id,
            participant_amount_minor,
            &participant_id,
            &id,
        )
        .await
    {
        if let Some((order, _)) = created_order.take() {
            if let Err(rollback_error) = state.orders.remove_unfunded(&order.id).await {
                tracing::error!(
                    order_id = %order.id,
                    error = %rollback_error.log_message(),
                    "后台合买参与扣款失败后移除满单订单失败"
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
                "后台合买参与扣款失败后移除参与记录失败"
            );
        }
        return Err(error);
    }
    if let Some((order, _)) = &created_order {
        publish_user_order_changed(&state, order, "created");
    }
    publish_user_balance_changed(
        &state,
        &participant_user_id,
        "group_buy_debit",
        Some(&participant_id),
    )
    .await;

    Ok(Json(ApiEnvelope::success(plan)))
}

/// 返回后台邀请关系列表。
async fn list_invitations(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<InviteRecord>>>> {
    let invitations = state.invites.list().await?;

    Ok(Json(ApiEnvelope::success(invitations)))
}

/// 返回指定邀请关系详情。
async fn get_invitation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<InviteRecord>>> {
    let invitation = state.invites.get(&id).await?;

    Ok(Json(ApiEnvelope::success(invitation)))
}

/// 后台创建邀请关系记录。
async fn create_invitation(
    State(state): State<AppState>,
    Json(payload): Json<CreateInviteRecordRequest>,
) -> ApiResult<Json<ApiEnvelope<InviteRecord>>> {
    let access = state.access.snapshot().await?;
    let policy = state.rebates.get().await?;
    let invitation = state
        .invites
        .create(payload, &access.users, &policy)
        .await?;

    Ok(Json(ApiEnvelope::success(invitation)))
}

/// 后台更新邀请关系状态和返利开关。
async fn update_invitation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateInviteRecordRequest>,
) -> ApiResult<Json<ApiEnvelope<InviteRecord>>> {
    let invitation = state.invites.update(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(invitation)))
}

/// 返回后台客服会话列表。
async fn list_support_conversations(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<SupportConversation>>>> {
    let conversations = state.support.list().await?;

    Ok(Json(ApiEnvelope::success(conversations)))
}

/// 返回指定客服会话详情。
async fn get_support_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let conversation = state.support.get(&id).await?;

    Ok(Json(ApiEnvelope::success(conversation)))
}

/// 后台为用户创建客服会话。
async fn create_support_conversation(
    State(state): State<AppState>,
    Json(payload): Json<CreateSupportConversationRequest>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let access = state.access.snapshot().await?;
    let conversation = state.support.create(payload, &access.users).await?;
    publish_support_message_created(&state, &conversation);

    Ok(Json(ApiEnvelope::success(conversation)))
}

/// 后台更新客服会话状态、优先级或分配客服。
async fn update_support_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateSupportConversationRequest>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let access = state.access.snapshot().await?;
    let conversation = state.support.update(&id, payload, &access.admins).await?;
    publish_support_conversation_updated(&state, &conversation);

    Ok(Json(ApiEnvelope::success(conversation)))
}

/// 后台客服回复指定会话。
async fn reply_support_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SupportReplyRequest>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let access = state.access.snapshot().await?;
    let conversation = state.support.reply(&id, payload, &access.admins).await?;
    publish_support_message_created(&state, &conversation);

    Ok(Json(ApiEnvelope::success(conversation)))
}

/// 返回后台用户列表。
async fn list_users(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<UserSummary>>>> {
    let users = users_with_financial_balances(&state).await?;

    Ok(Json(ApiEnvelope::success(users)))
}

/// 返回指定用户详情。
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = user_with_financial_balance(&state, &id).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

/// 后台创建用户并初始化资金账户。
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<UserSummary>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.create_user(payload).await?;
    let account = state.finance.account_or_create(&user.id).await?;
    let user = user_with_account_balance(user, Some(&account));

    Ok(Json(ApiEnvelope::success(user)))
}

/// 后台更新用户基础资料，不直接修改余额和邀请码。
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UserSummary>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.update_user(&id, payload).await?;
    let user = user_with_financial_balance_from_summary(&state, user).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

/// 后台切换用户状态。
async fn set_user_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UserStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.set_user_status(&id, payload.status).await?;
    let user = user_with_financial_balance_from_summary(&state, user).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

/// 返回用户列表时用财务账户可用余额覆盖用户基础资料中的历史余额字段。
async fn users_with_financial_balances(state: &AppState) -> ApiResult<Vec<UserSummary>> {
    let mut users = state.access.users().await?;
    let accounts = state.finance.accounts().await?;
    let accounts = accounts
        .iter()
        .map(|account| (account.user_id.as_str(), account))
        .collect::<BTreeMap<_, _>>();
    for user in &mut users {
        user.balance_minor = accounts
            .get(user.id.as_str())
            .map(|account| account.available_balance_minor)
            .unwrap_or(user.balance_minor);
    }
    Ok(users)
}

/// 返回单个用户时同步财务账户可用余额。
async fn user_with_financial_balance(state: &AppState, id: &str) -> ApiResult<UserSummary> {
    let user = state.access.get_user(id).await?;
    user_with_financial_balance_from_summary(state, user).await
}

/// 将已有用户摘要与财务账户合并，避免用户维护接口成为余额编辑入口。
async fn user_with_financial_balance_from_summary(
    state: &AppState,
    user: UserSummary,
) -> ApiResult<UserSummary> {
    let accounts = state.finance.accounts().await?;
    let account = accounts.iter().find(|account| account.user_id == user.id);
    Ok(user_with_account_balance(user, account))
}

/// 处理单个用户和资金账户的余额合并。
fn user_with_account_balance(
    mut user: UserSummary,
    account: Option<&FinancialAccountSummary>,
) -> UserSummary {
    if let Some(account) = account {
        user.balance_minor = account.available_balance_minor;
    }
    user
}

/// 返回后台管理员账号列表。
async fn list_admins(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<AdminSummary>>>> {
    let admins = state.access.admins().await?;

    Ok(Json(ApiEnvelope::success(admins)))
}

/// 返回指定管理员账号详情。
async fn get_admin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.get_admin(&id).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

/// 后台创建管理员账号。
async fn create_admin(
    State(state): State<AppState>,
    Json(payload): Json<AdminSaveRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.create_admin(payload).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

/// 后台更新管理员账号资料。
async fn update_admin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminSaveRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.update_admin(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

/// 后台重置管理员登录密码。
async fn reset_admin_password(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminPasswordResetRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.reset_admin_password(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

/// 后台切换管理员账号状态。
async fn set_admin_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.set_admin_status(&id, payload.status).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

/// 返回后台角色列表。
async fn list_roles(State(state): State<AppState>) -> ApiResult<Json<ApiEnvelope<Vec<AdminRole>>>> {
    let roles = state.access.roles().await?;

    Ok(Json(ApiEnvelope::success(roles)))
}

/// 返回指定角色详情。
async fn get_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.get_role(&id).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

/// 后台创建权限角色。
async fn create_role(
    State(state): State<AppState>,
    Json(payload): Json<AdminRole>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.create_role(payload).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

/// 后台更新权限角色。
async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminRole>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.update_role(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

/// 后台删除未被使用的权限角色。
async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.delete_role(&id).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

/// 返回系统设置列表。
async fn list_system_settings(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<SystemSetting>>>> {
    let settings = state.access.settings().await?;

    Ok(Json(ApiEnvelope::success(settings)))
}

/// 后台更新单个系统设置值。
async fn update_system_setting(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<UpdateSystemSettingRequest>,
) -> ApiResult<Json<ApiEnvelope<SystemSetting>>> {
    let setting = state.access.update_setting(&key, payload).await?;

    Ok(Json(ApiEnvelope::success(setting)))
}

/// 处理管理员图片上传请求：读取图床配置后透传 multipart 文件到第三方服务。
async fn upload_image_bed_file(
    State(state): State<AppState>,
    payload: Multipart,
) -> ApiResult<Json<ApiEnvelope<Value>>> {
    let output =
        upload_configured_image_bed_file(&state.access, payload, ImageBedUploadOptions::default())
            .await?;

    Ok(Json(ApiEnvelope::success(output)))
}

/// 返回后台广告配置列表。
async fn list_advertisements(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<AdvertisementSummary>>>> {
    let advertisements = state.advertisements.list().await?;

    Ok(Json(ApiEnvelope::success(advertisements)))
}

/// 返回指定广告配置详情。
async fn get_advertisement(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdvertisementSummary>>> {
    let advertisement = state.advertisements.get(&id).await?;

    Ok(Json(ApiEnvelope::success(advertisement)))
}

/// 后台创建广告配置。
async fn create_advertisement(
    State(state): State<AppState>,
    Json(payload): Json<SaveAdvertisementRequest>,
) -> ApiResult<Json<ApiEnvelope<AdvertisementSummary>>> {
    let advertisement = state.advertisements.create(payload).await?;

    Ok(Json(ApiEnvelope::success(advertisement)))
}

/// 后台更新广告配置。
async fn update_advertisement(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SaveAdvertisementRequest>,
) -> ApiResult<Json<ApiEnvelope<AdvertisementSummary>>> {
    let advertisement = state.advertisements.update(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(advertisement)))
}

/// 后台删除广告配置。
async fn delete_advertisement(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdvertisementSummary>>> {
    let advertisement = state.advertisements.delete(&id).await?;

    Ok(Json(ApiEnvelope::success(advertisement)))
}

/// 返回用户注册开关配置。
async fn get_registration_config(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<RegistrationConfig>>> {
    let registration = state.access.registration().await?;

    Ok(Json(ApiEnvelope::success(registration)))
}

/// 后台更新注册方式和邀请码规则。
async fn update_registration_config(
    State(state): State<AppState>,
    Json(payload): Json<RegistrationConfig>,
) -> ApiResult<Json<ApiEnvelope<RegistrationConfig>>> {
    let registration = state.access.update_registration(payload).await?;

    Ok(Json(ApiEnvelope::success(registration)))
}

/// 返回邀请和返利策略。
async fn get_invite_policy(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<InvitePolicySummary>>> {
    let policy = state.rebates.get().await?;

    Ok(Json(ApiEnvelope::success(policy)))
}

/// 后台更新邀请权限和默认返利比例。
async fn update_invite_policy(
    State(state): State<AppState>,
    Json(payload): Json<InvitePolicyUpdateRequest>,
) -> ApiResult<Json<ApiEnvelope<InvitePolicySummary>>> {
    let policy = state.rebates.update(payload).await?;

    Ok(Json(ApiEnvelope::success(policy)))
}

/// 返回后台机器人配置列表。
async fn list_robots(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<RobotConfigSummary>>>> {
    let robots = state.robots.list().await?;

    Ok(Json(ApiEnvelope::success(robots)))
}

/// 返回指定机器人配置详情。
async fn get_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.get(&id).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

/// 后台创建机器人配置。
async fn create_robot(
    State(state): State<AppState>,
    Json(payload): Json<RobotConfigSummary>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let lotteries = state.lotteries.list().await?;
    let robot = state.robots.create(payload, &lotteries).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

/// 后台更新机器人配置。
async fn update_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RobotConfigSummary>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let lotteries = state.lotteries.list().await?;
    let robot = state.robots.update(&id, payload, &lotteries).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

/// 后台删除机器人配置。
async fn delete_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.delete(&id).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

/// 后台切换机器人运行状态。
async fn set_robot_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RobotStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.set_status(&id, payload.status).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

/// 后台手动触发合买机器人执行。
async fn run_group_buy_robots_request(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyRobotRun>>> {
    let run = run_group_buy_robots(
        &state.robots,
        &state.draws,
        &state.lotteries,
        &state.orders,
        &state.finance,
        &state.group_buys,
        &state.access,
        current_timestamp(),
    )
    .await?;
    publish_group_buy_robot_events(&state, &run).await;

    Ok(Json(ApiEnvelope::success(run)))
}

/// 推送合买机器人执行产生的余额和订单变化事件。
async fn publish_group_buy_robot_events(state: &AppState, run: &GroupBuyRobotRun) {
    for entry in &run.ledger_entries {
        publish_user_balance_changed(
            state,
            &entry.user_id,
            "group_buy_robot",
            entry.reference_id.as_deref(),
        )
        .await;
    }
    for order in &run.created_orders {
        publish_user_order_changed(state, order, "created");
    }
}

/// 返回后台手动任务使用的本地时间字符串。
fn current_timestamp() -> String {
    Local::now()
        .naive_local()
        .format(TIMESTAMP_FORMAT)
        .to_string()
}

/// 根据后台机器人数据开关计算财务总览；默认剔除机器人账户，避免运营数据被自动补单干扰。
async fn finance_overview_for_query(
    state: &AppState,
    include_robot_data: bool,
) -> ApiResult<FinanceOverview> {
    if include_robot_data {
        return state.finance.overview().await;
    }

    let accounts = state
        .finance
        .accounts()
        .await?
        .into_iter()
        .filter(|account| !is_group_buy_robot_user_id(&account.user_id))
        .collect::<Vec<_>>();
    let ledger_entries = state
        .finance
        .ledger_entries()
        .await?
        .into_iter()
        .filter(|entry| !is_group_buy_robot_user_id(&entry.user_id))
        .collect::<Vec<_>>();
    finance_overview_from_items(&accounts, &ledger_entries)
}

/// 从已过滤的账户和流水重新汇总财务指标，保证总览数字与当前列表口径一致。
fn finance_overview_from_items(
    accounts: &[FinancialAccountSummary],
    ledger_entries: &[LedgerEntry],
) -> ApiResult<FinanceOverview> {
    let mut total_balance_minor = 0_i64;
    let mut pending_withdraw_minor = 0_i64;
    for account in accounts {
        total_balance_minor = total_balance_minor
            .checked_add(account.available_balance_minor)
            .and_then(|amount| amount.checked_add(account.frozen_balance_minor))
            .ok_or_else(|| ApiError::Internal("财务总览金额汇总溢出".to_string()))?;
        pending_withdraw_minor = pending_withdraw_minor
            .checked_add(account.frozen_balance_minor)
            .ok_or_else(|| ApiError::Internal("财务冻结金额汇总溢出".to_string()))?;
    }

    let today_recharge_minor = ledger_entries
        .iter()
        .filter(|entry| entry.kind == LedgerEntryKind::RechargeCredit)
        .try_fold(0_i64, |total, entry| total.checked_add(entry.amount_minor))
        .ok_or_else(|| ApiError::Internal("财务充值金额汇总溢出".to_string()))?;
    let today_payout_minor = ledger_entries
        .iter()
        .filter(|entry| entry.kind == LedgerEntryKind::PayoutCredit)
        .try_fold(0_i64, |total, entry| total.checked_add(entry.amount_minor))
        .ok_or_else(|| ApiError::Internal("财务派奖金额汇总溢出".to_string()))?;

    Ok(FinanceOverview {
        total_balance_minor,
        pending_withdraw_minor,
        today_recharge_minor,
        today_payout_minor,
    })
}

/// 判断后台用户维度记录是否应返回给页面；机器人数据只有在开关打开时展示。
fn should_include_user_scoped_record(include_robot_data: bool, user_id: &str) -> bool {
    include_robot_data || !is_group_buy_robot_user_id(user_id)
}

/// 返回后台财务总览。
async fn get_finance_overview(
    State(state): State<AppState>,
    Query(query): Query<RobotDataQuery>,
) -> ApiResult<Json<ApiEnvelope<FinanceOverview>>> {
    let overview = finance_overview_for_query(&state, query.include_robot_data()).await?;

    Ok(Json(ApiEnvelope::success(overview)))
}

/// 返回资金账户分页列表。
async fn list_financial_accounts(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<AdminFinancialAccountSummary>>>> {
    let accounts = state
        .finance
        .accounts()
        .await?
        .into_iter()
        .filter(|account| {
            should_include_user_scoped_record(query.include_robot_data(), &account.user_id)
        })
        .collect::<Vec<_>>();
    let users = state.access.users().await?;
    let usernames: BTreeMap<String, String> = users
        .into_iter()
        .map(|user| (user.id, user.username))
        .collect();
    let accounts = accounts
        .into_iter()
        .map(|account| AdminFinancialAccountSummary {
            username: usernames.get(&account.user_id).cloned(),
            user_id: account.user_id,
            available_balance_minor: account.available_balance_minor,
            frozen_balance_minor: account.frozen_balance_minor,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiEnvelope::success(page_items(accounts, query))))
}

/// 返回资金流水分页列表。
async fn list_ledger_entries(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<LedgerEntry>>>> {
    let entries = state
        .finance
        .ledger_entries()
        .await?
        .into_iter()
        .filter(|entry| {
            should_include_user_scoped_record(query.include_robot_data(), &entry.user_id)
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiEnvelope::success(page_items(entries, query))))
}

/// 返回充值订单分页列表。
async fn list_recharge_orders(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<RechargeOrderSummary>>>> {
    let orders = state.recharges.list().await?;

    Ok(Json(ApiEnvelope::success(page_items(orders, query))))
}

/// 返回提现申请分页列表。
async fn list_withdrawal_orders(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<WithdrawalOrderSummary>>>> {
    let orders = state.withdrawals.list().await?;

    Ok(Json(ApiEnvelope::success(page_items(orders, query))))
}

/// 后台确认客服直充或人工充值订单入账。
async fn confirm_recharge_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ConfirmRechargeOrderRequest>,
) -> ApiResult<Json<ApiEnvelope<RechargeOrderSummary>>> {
    let order = state
        .recharges
        .confirm_customer_service_order(&id, payload, &state.finance)
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

    Ok(Json(ApiEnvelope::success(order)))
}

/// 后台审核通过提现申请并扣减冻结余额。
async fn approve_withdrawal_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<WithdrawalOrderSummary>>> {
    let order = state.withdrawals.approve_order(&id, &state.finance).await?;
    publish_user_withdrawal_changed(&state, &order);
    publish_user_balance_changed(&state, &order.user_id, "withdrawal_payout", Some(&order.id))
        .await;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 后台驳回提现申请并解冻余额。
async fn reject_withdrawal_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<WithdrawalOrderSummary>>> {
    let order = state.withdrawals.reject_order(&id, &state.finance).await?;
    publish_user_withdrawal_changed(&state, &order);
    publish_user_balance_changed(&state, &order.user_id, "withdrawal_reject", Some(&order.id))
        .await;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 后台财务手动调账接口。
async fn manual_balance_adjustment(
    State(state): State<AppState>,
    Json(payload): Json<ManualBalanceAdjustmentRequest>,
) -> ApiResult<Json<ApiEnvelope<LedgerEntry>>> {
    let entry = state.finance.manual_adjust(payload).await?;
    publish_user_balance_changed(
        &state,
        &entry.user_id,
        "manual_adjustment",
        entry.reference_id.as_deref(),
    )
    .await;

    Ok(Json(ApiEnvelope::success(entry)))
}

/// 返回后台投注订单分页列表。
async fn list_orders(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<OrderDetail>>>> {
    // 订单管理按分页读取；彩种控制台不传分页参数时仍通过同一信封拿到完整订单页。
    let orders = state
        .orders
        .list()
        .await?
        .into_iter()
        .filter(|order| {
            should_include_user_scoped_record(query.include_robot_data(), &order.user_id)
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiEnvelope::success(page_items(orders, query))))
}

/// 返回指定投注订单详情。
async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<OrderDetail>>> {
    let order = state.orders.get(&id).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 后台代创建投注订单并扣款。
async fn create_order(
    State(state): State<AppState>,
    Json(payload): Json<CreateOrderRequest>,
) -> ApiResult<Json<ApiEnvelope<OrderDetail>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let draw_issue = state
        .draws
        .get_by_lottery_issue(&payload.lottery_id, &payload.issue)
        .await?;
    validate_draw_issue_accepts_order(&draw_issue, &lottery, &payload.issue)?;
    let order = state
        .orders
        .create_with_debit(&state.finance, &lottery, payload, OrderSource::Direct)
        .await?;
    publish_user_order_changed(&state, &order, "created");
    publish_user_balance_changed(&state, &order.user_id, "order_debit", Some(&order.id)).await;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 后台取消待开奖订单并退款。
async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<OrderDetail>>> {
    let order = state.orders.cancel_with_refund(&state.finance, &id).await?;
    publish_user_order_changed(&state, &order, "cancelled");
    publish_user_balance_changed(&state, &order.user_id, "order_refund", Some(&order.id)).await;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 返回后台彩种列表。
async fn list_lotteries(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LotteryKind>>>> {
    let lotteries = state.lotteries.list().await?;

    Ok(Json(ApiEnvelope::success(lotteries)))
}

/// 返回指定彩种详情。
async fn get_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.get(&id).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

/// 后台创建彩种配置。
async fn create_lottery(
    State(state): State<AppState>,
    Json(payload): Json<LotteryKind>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.create(payload).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

/// 后台更新彩种配置。
async fn update_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<LotteryKind>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.update(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

/// 后台删除彩种配置。
async fn delete_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.delete(&id).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

/// 后台切换彩种销售状态，并在开售时补齐期号。
async fn set_lottery_sale(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SaleStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let before = state.lotteries.get(&id).await?;
    let need_align = should_align_draw_issue_plan_after_sale_on(&before.draw_mode)
        && !before.sale_enabled
        && payload.sale_enabled;

    let lottery = state
        .lotteries
        .set_sale_enabled(&id, payload.sale_enabled)
        .await?;

    if need_align {
        match align_draw_issue_plan_after_sale_on(&state, &lottery).await {
            Ok(issues) => {
                for issue in &issues {
                    state.realtime.publish_public(issue_opened_event(issue));
                }
            }
            Err(error) => {
                tracing::warn!(
                    lottery_id = %lottery.id,
                    error = %error.log_message(),
                    "开售后补齐期号失败，已保留销售状态切换结果"
                );
            }
        }
    }

    Ok(Json(ApiEnvelope::success(lottery)))
}

/// 判断彩种开售后是否需要立即补齐未来期号。
fn should_align_draw_issue_plan_after_sale_on(draw_mode: &DrawMode) -> bool {
    matches!(draw_mode, DrawMode::Api | DrawMode::Platform)
}

/// 返回彩种分类列表。
async fn list_lottery_categories(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LotteryCategoryConfig>>>> {
    let categories = state.lotteries.categories().await?;

    Ok(Json(ApiEnvelope::success(categories)))
}

/// 后台创建彩种分类。
async fn create_lottery_category(
    State(state): State<AppState>,
    Json(payload): Json<LotteryCategoryConfig>,
) -> ApiResult<Json<ApiEnvelope<LotteryCategoryConfig>>> {
    let category = state.lotteries.create_category(payload).await?;

    Ok(Json(ApiEnvelope::success(category)))
}

/// 后台更新彩种分类名称。
async fn update_lottery_category(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Json(payload): Json<LotteryCategoryConfig>,
) -> ApiResult<Json<ApiEnvelope<LotteryCategoryConfig>>> {
    let payload = LotteryCategoryConfig {
        code: code.clone(),
        ..payload
    };
    let category = state.lotteries.update_category(&code, payload).await?;

    Ok(Json(ApiEnvelope::success(category)))
}

/// 后台删除彩种分类。
async fn delete_lottery_category(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryCategoryConfig>>> {
    let category = state.lotteries.delete_category(&code).await?;

    Ok(Json(ApiEnvelope::success(category)))
}

/// 彩种开售后按开奖模式补齐可销售期号。
async fn align_draw_issue_plan_after_sale_on(
    state: &AppState,
    lottery: &LotteryKind,
) -> ApiResult<Vec<DrawIssue>> {
    let config = state.scheduler.config()?;
    let now = Local::now()
        .naive_local()
        .format(TIMESTAMP_FORMAT)
        .to_string();
    let existing_issues = state.draws.list_by_lottery_id(&lottery.id).await?;
    let existing_future_count = existing_issues
        .into_iter()
        .filter(|issue| {
            issue.status == DrawIssueStatus::Open && issue.scheduled_at.as_str() > now.as_str()
        })
        .count() as u32;

    if existing_future_count >= config.future_issue_count {
        return Ok(Vec::new());
    }

    let count = config.future_issue_count - existing_future_count;
    generate_draw_issue_batch(
        &state.draws,
        lottery,
        GenerateDrawIssuesRequest {
            lottery_id: lottery.id.clone(),
            now,
            count,
            sale_close_lead_seconds: Some(config.sale_close_lead_seconds),
        },
    )
    .await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台切换彩种销售状态时提交的请求。
struct SaleStatusRequest {
    sale_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        align_draw_issue_plan_after_sale_on, finance_overview_for_query,
        normalize_admin_draw_control_target, page_items, required_scope_for_path,
        should_align_draw_issue_plan_after_sale_on, should_include_user_scoped_record,
        FinancePageQuery,
    };
    use crate::services::group_buy_robot::ROBOT_GROUP_BUY_USER_ID;
    use crate::{
        app::AppState,
        domain::{
            draw::{DrawControlTargetScope, DrawIssueStatus, SaveLotteryDrawControlRequest},
            lottery::DrawMode,
            order::CreateOrderRequest,
            permission::PermissionScope,
            play::{PlayRuleCode, PlaySelection},
        },
        services::{
            access::AccessRepository,
            advertisement::AdvertisementRepository,
            chat_hall::ChatHallRepository,
            draw::DrawRepository,
            finance::FinanceRepository,
            group_buy::GroupBuyRepository,
            invite::InviteRepository,
            lottery::LotteryRepository,
            order::OrderRepository,
            realtime::RealtimeHub,
            rebate::RebateRepository,
            recharge::RechargeRepository,
            robot::RobotRepository,
            scheduler::{DrawSchedulerConfig, DrawSchedulerRepository},
            support::SupportRepository,
            withdrawal::WithdrawalRepository,
        },
    };

    #[test]
    /// 处理 required_scope_maps_admin_paths 的具体内部流程。
    fn required_scope_maps_admin_paths() {
        assert_eq!(required_scope_for_path("/dashboard"), None);
        assert_eq!(
            required_scope_for_path("/users"),
            Some(PermissionScope::Users)
        );
        assert_eq!(
            required_scope_for_path("/admins/A10001"),
            Some(PermissionScope::Admins)
        );
        assert_eq!(
            required_scope_for_path("/roles"),
            Some(PermissionScope::Roles)
        );
        assert_eq!(
            required_scope_for_path("/draw-issues"),
            Some(PermissionScope::Lotteries)
        );
        assert_eq!(
            required_scope_for_path("/image-bed/upload"),
            Some(PermissionScope::SystemSettings)
        );
        assert_eq!(
            required_scope_for_path("/settlements"),
            Some(PermissionScope::Orders)
        );
        assert_eq!(
            required_scope_for_path("/support/conversations"),
            Some(PermissionScope::CustomerService)
        );
        assert_eq!(
            required_scope_for_path("/lottery-categories"),
            Some(PermissionScope::Lotteries)
        );
        assert_eq!(
            required_scope_for_path("/robots"),
            Some(PermissionScope::Robots)
        );
        assert_eq!(
            required_scope_for_path("/invite-policy"),
            Some(PermissionScope::Rebates)
        );
    }

    #[test]
    /// 后台订单、资金账户和资金流水默认过滤机器人账户，开关打开后才展示。
    fn user_scoped_record_filter_hides_robot_by_default() {
        assert!(!should_include_user_scoped_record(
            false,
            ROBOT_GROUP_BUY_USER_ID
        ));
        assert!(should_include_user_scoped_record(
            true,
            ROBOT_GROUP_BUY_USER_ID
        ));
        assert!(should_include_user_scoped_record(false, "U10001"));
    }

    #[test]
    /// 合买计划列表复用后台分页结构，按请求页码返回当前页切片和总数。
    fn page_items_returns_requested_group_buy_page() {
        let page = page_items(
            vec![
                "G-001".to_string(),
                "G-002".to_string(),
                "G-003".to_string(),
            ],
            FinancePageQuery {
                include_robot_data: None,
                page: Some(2),
                page_size: Some(2),
            },
        );

        assert_eq!(page.items, vec!["G-003".to_string()]);
        assert_eq!(page.page, 2);
        assert_eq!(page.page_size, 2);
        assert_eq!(page.total_count, 3);
        assert_eq!(page.total_pages, 2);
    }

    #[tokio::test]
    /// 财务总览默认剔除机器人账户余额，开关打开后才纳入机器人自动授信和扣款口径。
    async fn finance_overview_hides_robot_account_by_default() {
        let state = test_state();

        let hidden_robot_overview = finance_overview_for_query(&state, false)
            .await
            .expect("overview can hide robot account");
        let full_overview = finance_overview_for_query(&state, true)
            .await
            .expect("overview can include robot account");

        assert!(full_overview.total_balance_minor > hidden_robot_overview.total_balance_minor);
    }

    #[test]
    /// 开售即时补期只覆盖 API 和平台开奖彩种，手动开奖仍由运营维护期号。
    fn sale_on_alignment_covers_api_and_platform_draw_modes() {
        assert!(should_align_draw_issue_plan_after_sale_on(&DrawMode::Api));
        assert!(should_align_draw_issue_plan_after_sale_on(
            &DrawMode::Platform
        ));
        assert!(!should_align_draw_issue_plan_after_sale_on(
            &DrawMode::Manual
        ));
    }

    #[tokio::test]
    /// 平台开奖彩种开售后会立即补齐未来期号，避免必须等待调度下一轮才开盘。
    async fn sale_on_alignment_generates_future_issue_for_platform_lottery() {
        let state = test_state();
        let lottery = state
            .lotteries
            .set_sale_enabled("ssc60", true)
            .await
            .expect("platform lottery sale can be enabled");

        let issues = align_draw_issue_plan_after_sale_on(&state, &lottery)
            .await
            .expect("platform lottery can align future draw issues");

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].lottery_id, "ssc60");
        assert_eq!(issues[0].draw_mode, DrawMode::Platform);
        assert_eq!(issues[0].status, DrawIssueStatus::Open);
    }

    #[tokio::test]
    /// 目标订单控制会校验订单归属，并把控制期号绑定为该订单期号。
    async fn draw_control_order_target_binds_order_issue() {
        let state = test_state();
        let mut lottery = state.lotteries.get("ssc60").await.expect("lottery exists");
        lottery.sale_enabled = true;
        let order = state
            .orders
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: lottery.id.clone(),
                    issue: "202606052200".to_string(),
                    rule_code: PlayRuleCode::FiveFrontDirect,
                    selection: PlaySelection {
                        positions: vec![vec![1], vec![2], vec![3]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .await
            .expect("order can be created");
        let mut payload = SaveLotteryDrawControlRequest {
            enabled: true,
            draw_number: Some("1,2,3,4,5".to_string()),
            target_scope: DrawControlTargetScope::Order,
            target_issue: None,
            target_order_id: Some(order.id.clone()),
        };

        normalize_admin_draw_control_target(&state, &lottery, &mut payload)
            .await
            .expect("order target can be normalized");

        assert_eq!(payload.target_issue.as_deref(), Some("202606052200"));
        assert_eq!(payload.target_order_id.as_deref(), Some(order.id.as_str()));
    }

    fn test_state() -> AppState {
        AppState {
            access: AccessRepository::memory_seeded(),
            advertisements: AdvertisementRepository::memory(),
            chat_hall: ChatHallRepository::memory(),
            draws: DrawRepository::memory(),
            finance: FinanceRepository::memory_seeded(),
            group_buys: GroupBuyRepository::memory_seeded(),
            invites: InviteRepository::memory_seeded(),
            lotteries: LotteryRepository::memory_seeded(),
            orders: OrderRepository::memory(),
            rebates: RebateRepository::memory_seeded(),
            realtime: RealtimeHub::new(),
            recharges: RechargeRepository::memory(),
            robots: RobotRepository::memory_seeded(),
            scheduler: DrawSchedulerRepository::new(DrawSchedulerConfig {
                enabled: false,
                interval_seconds: 5,
                future_issue_count: 1,
                sale_close_lead_seconds: 30,
            }),
            support: SupportRepository::memory_seeded(),
            withdrawals: WithdrawalRepository::memory(),
        }
    }
}
