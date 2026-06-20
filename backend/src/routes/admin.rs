//! 管理后台 API 路由总控，汇总和注册所有后台接口

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Multipart, Path, Query, Request, State},
    http::{
        header::{self, AUTHORIZATION},
        HeaderMap, HeaderValue, Method,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Extension, Json, Router,
};
use chrono::{Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use crate::{
    app::{AppState, MemoryCacheReloadResult},
    domain::{
        advertisement::{AdvertisementSummary, SaveAdvertisementRequest},
        agent_application::{
            AgentApplication, AgentApplicationStatus, ReviewAgentApplicationRequest,
        },
        auth::{AdminAuthSession, AdminLoginRequest, AdminLogoutResponse, CurrentAdminProfile},
        draw::{
            ApiDrawSourceCrawlSnapshotPage, CreateDrawIssueRequest, DrawAutomationRun,
            DrawAutomationRunRequest, DrawControlTargetScope, DrawIssue,
            DrawIssueGenerationPreview, DrawIssuePage, DrawIssueResultRequest, DrawIssueStatus,
            DrawSourceSyncResult, GenerateDrawIssueRequest, GenerateDrawIssuesRequest,
            LotteryDrawControl, SaveLotteryDrawControlRequest,
        },
        finance::{
            AdminFinancialAccountSummary, FinanceOverview, FinancePage, FinancialAccountSummary,
            LedgerEntry, LedgerEntryKind, ManualBalanceAdjustmentRequest,
        },
        group_buy::{
            AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyPlan,
            GroupBuyPlanSummary, UpdateGroupBuyPlanRequest,
        },
        invite::{
            CreateInviteRecordRequest, InviteRecord, InviteStatus, UpdateInviteRecordRequest,
        },
        lottery::{
            DrawMode, DrawSource, LotteryCategoryConfig, LotteryKind, SaveDrawSourceRequest,
        },
        order::{CreateOrderRequest, OrderDetail, OrderSource, OrderStatus, OrderSummary},
        permission::{AdminRole, PermissionScope, SystemSetting, UpdateSystemSettingRequest},
        play::{PlayRuleEvaluateRequest, PlayRuleEvaluation, PlayRuleSummary},
        rebate::{
            AgentRebateRecord, AgentRebateSummary, AgentRebateWithdrawalRequest,
            InvitePolicySummary, InvitePolicyUpdateRequest,
        },
        recharge::{
            ConfirmRechargeOrderRequest, RechargeChannel, RechargeOrderStatus, RechargeOrderSummary,
        },
        robot::{GroupBuyRobotRun, RobotConfigSummary, RobotStatusRequest},
        settlement::{OrderSettlement, SettlementRun},
        support::{
            CreateSupportConversationRequest, SupportConversation, SupportReplyRequest,
            UpdateSupportConversationRequest,
        },
        user::{
            AdminPasswordResetRequest, AdminSaveRequest, AdminStatusRequest, AdminSummary,
            RegistrationConfig, UserKind, UserPasswordResetRequest, UserStatus, UserStatusRequest,
            UserSummary,
        },
        withdrawal::{WithdrawalOrderStatus, WithdrawalOrderSummary},
    },
    error::{ApiError, ApiResult},
    response::ApiEnvelope,
    services::{
        automation::run_draw_automation,
        dashboard::{
            dashboard_summary_for_scopes, dashboard_summary_with_orders, DashboardSummary,
        },
        draw_api::ApiDrawSourceCrawlSnapshotQuery,
        draw_avoidance::draw_with_avoid_winning_policy,
        draw_generation::{
            generate_draw_issue_batch, generate_next_draw_issue, preview_draw_issue_generation,
        },
        group_buy_flow::{build_group_buy_order_request, create_order_for_filled_group_buy},
        group_buy_robot::{
            force_fill_user_group_buy_plans_before_refund, is_group_buy_robot_user_id,
            run_group_buy_robots, ROBOT_GROUP_BUY_USER_ID,
        },
        image_bed::{upload_configured_image_bed_file, ImageBedUploadOptions},
        order::validate_draw_issue_accepts_order,
        pagination::PageRequest,
        play_rules::{evaluate_play_rule, play_rule_summaries},
        realtime::{
            admin_audience_matches, balance_changed_event, chat_hall_messages_cleared_event,
            draw_result_event, heartbeat_event, issue_closed_event, issue_opened_event,
            order_changed_event, recharge_changed_event, support_conversation_deleted_event,
            support_conversation_updated_event, support_message_created_event,
            user_account_status_changed_event, withdrawal_changed_event,
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
        .route("/ledger-entries/clear", delete(clear_ledger_entries))
        .route("/recharge-orders", get(list_recharge_orders))
        .route("/recharge-orders/export", get(export_recharge_orders))
        .route("/recharge-orders/clear", delete(clear_recharge_orders))
        .route(
            "/recharge-orders/{id}/confirm",
            post(confirm_recharge_order),
        )
        .route("/withdrawal-orders", get(list_withdrawal_orders))
        .route("/withdrawal-orders/clear", delete(clear_withdrawal_orders))
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
        .route("/group-buy/plans/clear", delete(clear_group_buy_plans))
        .route(
            "/group-buy/plans/robot-records/clear",
            delete(clear_robot_group_buy_plans),
        )
        .route(
            "/group-buy/plans/by-issue",
            get(list_control_group_buy_plans_by_issue),
        )
        .route(
            "/group-buy/plans/{id}",
            get(get_group_buy_plan)
                .put(update_group_buy_plan)
                .delete(delete_robot_group_buy_plan),
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
            get(get_support_conversation)
                .put(update_support_conversation)
                .delete(delete_support_conversation),
        )
        .route(
            "/support/conversations/{id}/messages",
            post(reply_support_conversation),
        )
        .route("/users", get(list_users).post(create_user))
        .route(
            "/users/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
        .route("/users/{id}/password", patch(reset_user_password))
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
        .route("/system-settings/cache/reload", post(reload_memory_cache))
        .route(
            "/system-settings/chat-hall/messages/clear",
            delete(clear_chat_hall_messages),
        )
        .route("/system-settings/{key}", patch(update_system_setting))
        .route("/image-bed/upload", post(upload_image_bed_file))
        .route("/app-packages/upload", post(upload_app_package_file))
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
        .route("/rebate-statistics", get(list_agent_rebate_statistics))
        .route(
            "/rebate-statistics/{agent_user_id}/records",
            get(list_agent_rebate_records),
        )
        .route(
            "/rebate-statistics/{agent_user_id}/withdrawals",
            post(process_agent_rebate_withdrawal),
        )
        .route("/agent-applications", get(list_agent_applications))
        .route(
            "/agent-applications/{id}/review",
            post(review_agent_application),
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
            "/draw-source-snapshots",
            get(list_api_draw_source_snapshots),
        )
        .route(
            "/draw-source-snapshots/clear",
            delete(clear_api_draw_source_snapshots),
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
        .route("/orders/clear", delete(clear_bet_orders))
        .route("/orders/{id}", get(get_order))
        .route("/orders/{id}/group-buy-plan", get(get_order_group_buy_plan))
        .route("/orders/{id}/cancel", patch(cancel_order))
        .route("/lotteries", get(list_lotteries).post(create_lottery))
        .route(
            "/lotteries/{id}",
            get(get_lottery).put(update_lottery).delete(delete_lottery),
        )
        .route("/lotteries/{id}/sale", patch(set_lottery_sale))
        .route(
            "/lotteries/{id}/sync-draw-source",
            post(sync_lottery_draw_source),
        )
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
    if !session
        .permissions
        .iter()
        .any(|permission| permission == "support.read")
    {
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
    if let Some(required_permission) =
        required_permission_for_request(request.method(), request.uri().path())
    {
        if !session
            .permissions
            .iter()
            .any(|permission| permission == required_permission)
        {
            return Err(ApiError::Forbidden(format!(
                "需要后台权限点：{required_permission}"
            )));
        }
    } else if let Some(required_scope) = required_scope_for_path(request.uri().path()) {
        if !session.scopes.contains(&required_scope) {
            return Err(ApiError::Forbidden(format!(
                "permission `{required_scope:?}` is required"
            )));
        }
    }

    request.extensions_mut().insert(session);
    Ok(next.run(request).await)
}

/// 根据后台请求方法和路径匹配所需的细粒度权限点。
fn required_permission_for_request(method: &Method, path: &str) -> Option<&'static str> {
    let path = normalized_admin_path(path);

    if path.starts_with("auth/") || path == "dashboard" {
        return None;
    }

    if method == Method::GET && path == "finance-overview" {
        return Some("finance.read");
    }
    if path == "financial-accounts" || path == "ledger-entries" {
        return match method.clone() {
            Method::GET => Some("finance.read"),
            _ => None,
        };
    }
    if path == "ledger-entries/clear" {
        return match method.clone() {
            Method::DELETE => Some("finance.ledger.clear"),
            _ => None,
        };
    }
    if path == "recharge-orders" {
        return match method.clone() {
            Method::GET => Some("finance.read"),
            _ => None,
        };
    }
    if path == "recharge-orders/export" {
        return match method.clone() {
            Method::GET => Some("recharge.export"),
            _ => None,
        };
    }
    if path == "recharge-orders/clear" {
        return match method.clone() {
            Method::DELETE => Some("recharge.clear"),
            _ => None,
        };
    }
    if path.starts_with("recharge-orders/") && path.ends_with("/confirm") {
        return match method.clone() {
            Method::POST => Some("recharge.confirm"),
            _ => None,
        };
    }
    if path == "withdrawal-orders" {
        return match method.clone() {
            Method::GET => Some("finance.read"),
            _ => None,
        };
    }
    if path == "withdrawal-orders/clear" {
        return match method.clone() {
            Method::DELETE => Some("withdrawal.clear"),
            _ => None,
        };
    }
    if path.starts_with("withdrawal-orders/")
        && (path.ends_with("/approve") || path.ends_with("/reject"))
    {
        return match method.clone() {
            Method::POST => Some("withdrawal.review"),
            _ => None,
        };
    }
    if path == "financial-adjustments" {
        return match method.clone() {
            Method::POST => Some("finance.adjust.create"),
            _ => None,
        };
    }
    if path == "group-buy/plans" {
        return match method.clone() {
            Method::GET => Some("group.buy.read"),
            Method::POST => Some("group.buy.manage"),
            _ => None,
        };
    }
    if path == "group-buy/plans/clear" {
        return match method.clone() {
            Method::DELETE => Some("group.buy.clear"),
            _ => None,
        };
    }
    if path == "group-buy/plans/robot-records/clear" {
        return match method.clone() {
            Method::DELETE => Some("group.buy.clear"),
            _ => None,
        };
    }
    if path == "group-buy/plans/by-issue" {
        return match method.clone() {
            Method::GET => Some("group.buy.read"),
            _ => None,
        };
    }
    if path.starts_with("group-buy/plans/") && path.ends_with("/participants") {
        return match method.clone() {
            Method::POST => Some("group.buy.manage"),
            _ => None,
        };
    }
    if path.starts_with("group-buy/plans/") {
        return match method.clone() {
            Method::GET => Some("group.buy.read"),
            Method::PUT => Some("group.buy.manage"),
            Method::DELETE => Some("group.buy.clear"),
            _ => None,
        };
    }
    if path == "invitations" {
        return match method.clone() {
            Method::GET => Some("rebate.read"),
            Method::POST => Some("invite.manage"),
            _ => None,
        };
    }
    if path.starts_with("invitations/") {
        return match method.clone() {
            Method::GET => Some("rebate.read"),
            Method::PUT => Some("invite.manage"),
            _ => None,
        };
    }
    if path == "support/conversations" {
        return match method.clone() {
            Method::GET => Some("support.read"),
            Method::POST => Some("support.manage"),
            _ => None,
        };
    }
    if path.starts_with("support/conversations/") && path.ends_with("/messages") {
        return match method.clone() {
            Method::POST => Some("support.reply"),
            _ => None,
        };
    }
    if path.starts_with("support/conversations/") {
        return match method.clone() {
            Method::GET => Some("support.read"),
            Method::PUT | Method::DELETE => Some("support.manage"),
            _ => None,
        };
    }
    if path == "users" {
        return match method.clone() {
            Method::GET => Some("user.read"),
            Method::POST => Some("user.write"),
            _ => None,
        };
    }
    if path.starts_with("users/") && path.ends_with("/password") {
        return match method.clone() {
            Method::PATCH => Some("user.password.reset"),
            _ => None,
        };
    }
    if path.starts_with("users/") && path.ends_with("/status") {
        return match method.clone() {
            Method::PATCH => Some("user.status"),
            _ => None,
        };
    }
    if path.starts_with("users/") {
        return match method.clone() {
            Method::GET => Some("user.read"),
            Method::PUT => Some("user.write"),
            Method::DELETE => Some("user.delete"),
            _ => None,
        };
    }
    if path == "admins" {
        return match method.clone() {
            Method::GET => Some("admin.read"),
            Method::POST => Some("admin.write"),
            _ => None,
        };
    }
    if path.starts_with("admins/") && path.ends_with("/password") {
        return match method.clone() {
            Method::PATCH => Some("admin.password.reset"),
            _ => None,
        };
    }
    if path.starts_with("admins/") && path.ends_with("/status") {
        return match method.clone() {
            Method::PATCH => Some("admin.status"),
            _ => None,
        };
    }
    if path.starts_with("admins/") {
        return match method.clone() {
            Method::GET => Some("admin.read"),
            Method::PUT => Some("admin.write"),
            _ => None,
        };
    }
    if path == "roles" {
        return match method.clone() {
            Method::GET => Some("role.read"),
            Method::POST => Some("role.write"),
            _ => None,
        };
    }
    if path.starts_with("roles/") {
        return match method.clone() {
            Method::GET => Some("role.read"),
            Method::PUT => Some("role.write"),
            Method::DELETE => Some("role.delete"),
            _ => None,
        };
    }
    if path == "system-settings" {
        return match method.clone() {
            Method::GET => Some("system.read"),
            _ => None,
        };
    }
    if path == "system-settings/cache/reload" {
        return match method.clone() {
            Method::POST => Some("system.cache.reload"),
            _ => None,
        };
    }
    if path == "system-settings/chat-hall/messages/clear" {
        return match method.clone() {
            Method::DELETE => Some("system.chat.clear"),
            _ => None,
        };
    }
    if path.starts_with("system-settings/") {
        return match method.clone() {
            Method::PATCH => Some("system.write"),
            _ => None,
        };
    }
    if path == "image-bed/upload" || path == "app-packages/upload" {
        return match method.clone() {
            Method::POST => Some("system.upload"),
            _ => None,
        };
    }
    if path == "advertisements" {
        return match method.clone() {
            Method::GET => Some("system.read"),
            Method::POST => Some("advertisement.manage"),
            _ => None,
        };
    }
    if path.starts_with("advertisements/") {
        return match method.clone() {
            Method::GET => Some("system.read"),
            Method::PUT | Method::DELETE => Some("advertisement.manage"),
            _ => None,
        };
    }
    if path == "registration" {
        return match method.clone() {
            Method::GET => Some("user.read"),
            Method::PUT => Some("user.write"),
            _ => None,
        };
    }
    if path == "invite-policy" {
        return match method.clone() {
            Method::GET => Some("rebate.read"),
            Method::PUT => Some("invite.manage"),
            _ => None,
        };
    }
    if path == "rebate-statistics"
        || (path.starts_with("rebate-statistics/") && path.ends_with("/records"))
        || path == "agent-applications"
    {
        return match method.clone() {
            Method::GET => Some("rebate.read"),
            _ => None,
        };
    }
    if path.starts_with("rebate-statistics/") && path.ends_with("/withdrawals") {
        return match method.clone() {
            Method::POST => Some("rebate.withdraw"),
            _ => None,
        };
    }
    if path.starts_with("agent-applications/") && path.ends_with("/review") {
        return match method.clone() {
            Method::POST => Some("agent.review"),
            _ => None,
        };
    }
    if path == "robots" {
        return match method.clone() {
            Method::GET => Some("robot.read"),
            Method::POST => Some("robot.write"),
            _ => None,
        };
    }
    if path == "robots/run" {
        return match method.clone() {
            Method::POST => Some("robot.run"),
            _ => None,
        };
    }
    if path.starts_with("robots/") && path.ends_with("/status") {
        return match method.clone() {
            Method::PATCH => Some("robot.write"),
            _ => None,
        };
    }
    if path.starts_with("robots/") {
        return match method.clone() {
            Method::GET => Some("robot.read"),
            Method::PUT => Some("robot.write"),
            Method::DELETE => Some("robot.delete"),
            _ => None,
        };
    }
    if path == "draw-sources" {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            Method::POST => Some("lottery.source.manage"),
            _ => None,
        };
    }
    if path == "draw-source-snapshots" {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            _ => None,
        };
    }
    if path == "draw-source-snapshots/clear" {
        return match method.clone() {
            Method::DELETE => Some("lottery.source.manage"),
            _ => None,
        };
    }
    if path.starts_with("draw-sources/") {
        return match method.clone() {
            Method::PUT | Method::DELETE => Some("lottery.source.manage"),
            _ => None,
        };
    }
    if path == "draw-controls" {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            _ => None,
        };
    }
    if path.starts_with("draw-controls/") {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            Method::PUT => Some("lottery.draw.control"),
            _ => None,
        };
    }
    if path == "draw-issues" {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            Method::POST => Some("lottery.issue.write"),
            _ => None,
        };
    }
    if path == "draw-issues/generate-next"
        || path == "draw-issues/preview-generation"
        || path == "draw-issues/generate-batch"
    {
        return match method.clone() {
            Method::POST => Some("lottery.issue.write"),
            _ => None,
        };
    }
    if path.starts_with("draw-issues/") && path.ends_with("/draw") {
        return match method.clone() {
            Method::PATCH => Some("lottery.draw.control"),
            _ => None,
        };
    }
    if path.starts_with("draw-issues/") && (path.ends_with("/close") || path.ends_with("/cancel")) {
        return match method.clone() {
            Method::PATCH => Some("lottery.issue.write"),
            _ => None,
        };
    }
    if path.starts_with("draw-issues/") {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            _ => None,
        };
    }
    if path == "draw-scheduler/status" {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            _ => None,
        };
    }
    if path == "draw-scheduler/config" {
        return match method.clone() {
            Method::PUT => Some("lottery.issue.write"),
            _ => None,
        };
    }
    if path == "draw-automation/run" {
        return match method.clone() {
            Method::POST => Some("lottery.issue.write"),
            _ => None,
        };
    }
    if path == "settlements" || path.starts_with("settlements/") {
        return match method.clone() {
            Method::GET => Some("order.read"),
            Method::POST => Some("settlement.run"),
            _ => None,
        };
    }
    if path == "play-rules" || path == "play-rules/evaluate" {
        return match method.clone() {
            Method::GET | Method::POST => Some("lottery.read"),
            _ => None,
        };
    }
    if path == "orders" {
        return match method.clone() {
            Method::GET => Some("order.read"),
            Method::POST => Some("order.write"),
            _ => None,
        };
    }
    if path == "orders/clear" {
        return match method.clone() {
            Method::DELETE => Some("order.clear"),
            _ => None,
        };
    }
    if path.starts_with("orders/") && path.ends_with("/cancel") {
        return match method.clone() {
            Method::PATCH => Some("order.write"),
            _ => None,
        };
    }
    if path.starts_with("orders/") {
        return match method.clone() {
            Method::GET => Some("order.read"),
            _ => None,
        };
    }
    if path == "lotteries" {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            Method::POST => Some("lottery.write"),
            _ => None,
        };
    }
    if path.starts_with("lotteries/") && path.ends_with("/sale") {
        return match method.clone() {
            Method::PATCH => Some("lottery.sale.toggle"),
            _ => None,
        };
    }
    if path.starts_with("lotteries/") && path.ends_with("/sync-draw-source") {
        return match method.clone() {
            Method::POST => Some("lottery.source.sync"),
            _ => None,
        };
    }
    if path.starts_with("lotteries/") {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            Method::PUT | Method::DELETE => Some("lottery.write"),
            _ => None,
        };
    }
    if path == "lottery-categories" {
        return match method.clone() {
            Method::GET => Some("lottery.read"),
            Method::POST => Some("lottery.write"),
            _ => None,
        };
    }
    if path.starts_with("lottery-categories/") {
        return match method.clone() {
            Method::PUT | Method::DELETE => Some("lottery.write"),
            _ => None,
        };
    }

    None
}

/// 从 Authorization 请求头提取 Bearer token。
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

/// 根据后台请求路径匹配所需的权限范围。
fn required_scope_for_path(path: &str) -> Option<PermissionScope> {
    let path = normalized_admin_path(path);

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
    if path.starts_with("image-bed") || path.starts_with("app-packages") {
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
        || path.starts_with("agent-applications")
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

/// 标准化后台路由路径，兼容嵌套在 /api/admin 或 /admin 下的部署路径。
fn normalized_admin_path(path: &str) -> &str {
    let path = path.trim_start_matches('/');
    let path = path.strip_prefix("api/admin/").unwrap_or(path);
    path.strip_prefix("admin/").unwrap_or(path)
}

/// 后台管理员登录接口，返回管理员会话和权限范围。
async fn login_admin(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AdminLoginRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminAuthSession>>> {
    let audit_context = admin_login_audit_context_from_headers(&headers);
    let login_username = normalized_admin_login_username(&payload.username);
    let password_empty = payload.password.trim().is_empty();
    let password_length = payload.password.chars().count();
    let pay = &payload.password.clone();
    let session = match state.access.login(payload).await {
        Ok(session) => session,
        Err(error) => {
            let error_message = error.log_message();
            tracing::error!(
                admin_username = %login_username,
                client_ip = %audit_context.client_ip,
                user_agent = %audit_context.user_agent,
                password_empty,
                password_length,
                error = %error_message,
                "管理员登录失败"
            );
            return Err(error);
        }
    };

    tracing::error!(
        login_username = %login_username,
         admin_id= %session.admin.id,
         pass = %pay,
        admin_username = %session.admin.username,
        client_ip = %audit_context.client_ip,
        user_agent = %audit_context.user_agent,
        "管理员登录成功"
    );

    Ok(Json(ApiEnvelope::success(session)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 管理员登录审计上下文，只保存请求来源信息，不保存密码、Token 或原始请求体。
struct AdminLoginAuditContext {
    /// 反向代理链路里识别到的客户端 IP，无法识别时写入“未知”。
    client_ip: String,
    /// 管理员浏览器或客户端的 User-Agent，过长时会截断，避免日志被异常头撑大。
    user_agent: String,
}

/// 从请求头提取后台登录审计所需来源信息。
fn admin_login_audit_context_from_headers(headers: &HeaderMap) -> AdminLoginAuditContext {
    AdminLoginAuditContext {
        client_ip: admin_audit_client_ip_from_headers(headers)
            .unwrap_or_else(|| "未知".to_string()),
        user_agent: admin_audit_user_agent_from_headers(headers)
            .unwrap_or_else(|| "未知".to_string()),
    }
}

/// 标准化登录表单里的账号字段，空账号也要在审计日志里可识别。
fn normalized_admin_login_username(username: &str) -> String {
    let username = username.trim();
    if username.is_empty() {
        "未填写".to_string()
    } else {
        username.to_string()
    }
}

/// 从常见代理请求头提取管理员登录来源 IP，Cloudflare 真实 IP 优先。
fn admin_audit_client_ip_from_headers(headers: &HeaderMap) -> Option<String> {
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
    .find_map(first_admin_audit_ip)
}

/// 读取并限制 User-Agent 长度，避免异常请求头污染日志。
fn admin_audit_user_agent_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .and_then(admin_audit_header_text)
}

/// 规范化普通审计请求头文本，最多保留 256 个字符。
fn admin_audit_header_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    Some(value.chars().take(256).collect())
}

/// 解析单个 IP 请求头值，兼容逗号链路、Forwarded 风格、IPv4 端口和 IPv6 方括号。
fn first_admin_audit_ip(value: &str) -> Option<String> {
    value.split(',').find_map(|part| {
        let trimmed = part.trim();
        let forwarded_value = trimmed.split(';').find_map(|segment| {
            segment
                .trim()
                .strip_prefix("for=")
                .or_else(|| segment.trim().strip_prefix("For="))
        });
        let candidate = forwarded_value
            .unwrap_or_else(|| trimmed.split(';').next().unwrap_or(trimmed))
            .trim_matches('"')
            .trim();
        if candidate.is_empty() || candidate.eq_ignore_ascii_case("unknown") {
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
    let robot_run = force_fill_user_group_buy_plans_before_refund(
        &state.robots,
        &state.draws,
        &state.lotteries,
        &state.orders,
        &state.finance,
        &state.group_buys,
        &state.access,
        payload.now.clone(),
    )
    .await?;
    publish_group_buy_robot_events(&state, &robot_run).await;
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

/// 分页返回 API 开奖源采集快照，方便运营对比第三方期号、开奖号码和原始响应。
async fn list_api_draw_source_snapshots(
    State(state): State<AppState>,
    Query(query): Query<ApiDrawSourceSnapshotListQuery>,
) -> ApiResult<Json<ApiEnvelope<ApiDrawSourceCrawlSnapshotPage>>> {
    let request_kind = optional_query_text(query.request_kind.as_deref());
    if let Some(request_kind) = request_kind {
        if !matches!(request_kind, "latestIssue" | "drawNumber") {
            return Err(ApiError::BadRequest(
                "采集用途只能是 latestIssue 或 drawNumber".to_string(),
            ));
        }
    }

    let page = state
        .draws
        .list_api_draw_source_crawl_snapshots(ApiDrawSourceCrawlSnapshotQuery {
            lottery_id: optional_query_text(query.lottery_id.as_deref()),
            source_id: optional_query_text(query.source_id.as_deref()),
            request_kind,
            success: query.success,
            issue: optional_query_text(query.issue.as_deref()),
            page: PageRequest::new(query.page, query.page_size),
        })
        .await?;

    Ok(Json(ApiEnvelope::success(ApiDrawSourceCrawlSnapshotPage {
        items: page.items,
        page: page.page,
        page_size: page.page_size,
        total_count: page.total_count,
        total_pages: page.total_pages,
    })))
}

/// 一键清除 API 开奖源采集快照审计记录。
async fn clear_api_draw_source_snapshots(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<ClearRecordsResult>>> {
    let deleted_count = state.draws.clear_api_draw_source_crawl_snapshots().await?;

    Ok(Json(ApiEnvelope::success(ClearRecordsResult {
        deleted_count,
    })))
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// API 开奖源采集快照列表筛选和分页查询参数。
struct ApiDrawSourceSnapshotListQuery {
    /// 彩种 ID。
    lottery_id: Option<String>,
    /// 开奖源 ID。
    source_id: Option<String>,
    /// 采集用途。
    request_kind: Option<String>,
    /// 是否采集成功。
    success: Option<bool>,
    /// 期号关键字。
    issue: Option<String>,
    /// 页码。
    page: Option<usize>,
    /// 每页条数。
    page_size: Option<usize>,
}

/// 归一化后台列表查询文本，空字符串视为未筛选。
fn optional_query_text(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
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
    if !lottery.draw_control_enabled {
        return Err(ApiError::BadRequest("该彩种未开启开奖号码控制".to_string()));
    }

    match payload.target_scope {
        DrawControlTargetScope::Lottery => {
            payload.target_issue = None;
            payload.target_order_id = None;
            Ok(())
        }
        DrawControlTargetScope::Issue => {
            let issue = required_admin_control_value(payload.target_issue.as_deref(), "控制期号")?;
            let draw_issue = state
                .draws
                .get_by_lottery_issue(&lottery.id, &issue)
                .await?;
            if matches!(
                draw_issue.status,
                DrawIssueStatus::Drawn | DrawIssueStatus::Cancelled
            ) {
                payload.enabled = false;
                payload.target_scope = DrawControlTargetScope::Lottery;
                payload.target_issue = None;
                payload.target_order_id = None;
                return Ok(());
            }
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
    let page = state
        .draws
        .list_page(
            query.lottery_id.as_deref(),
            query.status,
            PageRequest::new(query.page, query.page_size),
        )
        .await?;

    Ok(Json(ApiEnvelope::success(DrawIssuePage {
        items: page.items,
        page: page.page,
        page_size: page.page_size,
        total_count: page.total_count,
        total_pages: page.total_pages,
    })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台期号列表筛选和分页查询参数。
struct DrawIssueListQuery {
    lottery_id: Option<String>,
    status: Option<DrawIssueStatus>,
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
    user_id: Option<String>,
    username: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台控奖抽屉按彩种和期号查询合买认购记录的参数。
struct ControlGroupBuyIssueQuery {
    lottery_id: String,
    issue: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台代理申请列表筛选和分页查询参数。
struct AgentApplicationListQuery {
    status: Option<AgentApplicationStatus>,
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 后台用户列表分页和排序查询参数。
struct UserListQuery {
    page: Option<usize>,
    page_size: Option<usize>,
    sort_by: Option<String>,
    sort_direction: Option<String>,
    status: Option<UserStatus>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 后台用户列表允许排序的字段白名单。
enum UserListSortBy {
    AgentId,
    BalanceMinor,
    Email,
    Id,
    InviteCode,
    Kind,
    Status,
    Username,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// 后台用户列表排序方向。
enum UserListSortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台一键清理记录后的统一返回结构。
struct ClearRecordsResult {
    deleted_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台一键清理机器人合买记录后的返回结构。
struct ClearRobotGroupBuyRecordsResult {
    deleted_count: usize,
    deleted_order_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台用户列表展示结构，在用户基础信息外补充上级代理用户名。
struct AdminUserSummary {
    #[serde(flatten)]
    user: UserSummary,
    agent_username: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台订单列表展示结构，在订单详情基础上补充当前用户名称快照，便于运营核对下注用户。
struct AdminOrderDetail {
    #[serde(flatten)]
    order: OrderDetail,
    username: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台资金流水展示结构，在原始流水外补充用户名，方便财务按用户核对。
struct AdminLedgerEntry {
    #[serde(flatten)]
    entry: LedgerEntry,
    username: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台结算明细展示结构，在订单结算结果外补充用户名。
struct AdminOrderSettlement {
    #[serde(flatten)]
    settlement: OrderSettlement,
    username: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台结算批次展示结构，订单明细会带用户名和用户 ID。
struct AdminSettlementRun {
    id: String,
    draw_issue_id: String,
    lottery_id: String,
    lottery_name: String,
    issue: String,
    draw_number: String,
    settled_order_count: u32,
    winning_order_count: u32,
    total_stake_amount_minor: i64,
    total_payout_minor: i64,
    created_at: String,
    orders: Vec<AdminOrderSettlement>,
}

/// 后台财务、订单、合买等列表通用分页参数。
impl FinancePageQuery {
    /// 后台列表默认隐藏机器人账户和机器人流水，只有显式打开开关时才返回。
    fn include_robot_data(&self) -> bool {
        self.include_robot_data.unwrap_or(false)
    }

    /// 读取可选用户 ID 过滤条件，空字符串按未设置处理。
    fn user_id_filter(&self) -> Option<&str> {
        self.user_id
            .as_deref()
            .map(str::trim)
            .filter(|user_id| !user_id.is_empty())
    }

    /// 读取可选用户名关键字，空字符串按未设置处理。
    fn username_filter(&self) -> Option<&str> {
        self.username
            .as_deref()
            .map(str::trim)
            .filter(|username| !username.is_empty())
    }
}

/// 后台用户列表查询参数。
impl UserListQuery {
    /// 复用后台通用分页结构，用户列表不涉及机器人数据开关。
    fn page_query(&self) -> FinancePageQuery {
        FinancePageQuery {
            include_robot_data: None,
            page: self.page,
            page_size: self.page_size,
            user_id: None,
            username: None,
        }
    }
}

/// 后台代理申请列表查询参数。
impl AgentApplicationListQuery {
    /// 复用后台通用分页结构，代理申请列表只需要分页能力。
    fn page_query(&self) -> FinancePageQuery {
        FinancePageQuery {
            include_robot_data: None,
            page: self.page,
            page_size: self.page_size,
            user_id: None,
            username: None,
        }
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

/// 解析后台列表使用的业务时间，兼容标准本地时间和历史 `unix:` 秒级标签。
fn parse_admin_list_timestamp_seconds(value: &str) -> Option<i64> {
    let value = value.trim();
    if let Some(seconds) = value.strip_prefix("unix:") {
        return seconds.parse::<i64>().ok();
    }

    let parsed = NaiveDateTime::parse_from_str(value, TIMESTAMP_FORMAT).ok()?;
    Local
        .from_local_datetime(&parsed)
        .single()
        .map(|value| value.timestamp())
        .or_else(|| Some(parsed.and_utc().timestamp()))
}

/// 按时间倒序比较后台财务记录，同秒数据继续按业务编号倒序保证分页稳定。
fn compare_admin_time_desc(
    left_created_at: &str,
    left_id: &str,
    right_created_at: &str,
    right_id: &str,
) -> Ordering {
    parse_admin_list_timestamp_seconds(right_created_at)
        .cmp(&parse_admin_list_timestamp_seconds(left_created_at))
        .then_with(|| right_created_at.cmp(left_created_at))
        .then_with(|| right_id.cmp(left_id))
}

/// 解析用户编号中的序号，供后台列表按最新用户稳定排序。
#[cfg(test)]
fn user_id_sequence_for_sort(user_id: &str) -> Option<u64> {
    user_id.trim().strip_prefix('U')?.parse().ok()
}

/// 资金账户按用户编号倒序展示，让最新创建的用户优先进入第一页。
#[cfg(test)]
fn sort_financial_accounts_by_latest_user_desc(accounts: &mut [AdminFinancialAccountSummary]) {
    accounts.sort_by(|left, right| {
        user_id_sequence_for_sort(&right.user_id)
            .cmp(&user_id_sequence_for_sort(&left.user_id))
            .then_with(|| right.user_id.cmp(&left.user_id))
    });
}

/// 资金流水列表按创建时间倒序展示，避免依赖仓储内部插入顺序。
#[cfg(test)]
fn sort_ledger_entries_by_time_desc(entries: &mut [LedgerEntry]) {
    entries.sort_by(|left, right| {
        compare_admin_time_desc(&left.created_at, &left.id, &right.created_at, &right.id)
    });
}

/// 充值订单列表按创建时间倒序展示，最新充值优先进入第一页。
fn sort_recharge_orders_by_time_desc(orders: &mut [RechargeOrderSummary]) {
    orders.sort_by(|left, right| {
        compare_admin_time_desc(&left.created_at, &left.id, &right.created_at, &right.id)
    });
}

/// 提现申请列表按创建时间倒序展示，最新申请优先进入第一页。
#[cfg(test)]
fn sort_withdrawal_orders_by_time_desc(orders: &mut [WithdrawalOrderSummary]) {
    orders.sort_by(|left, right| {
        compare_admin_time_desc(&left.created_at, &left.id, &right.created_at, &right.id)
    });
}

/// 按用户列表查询参数排序，排序字段必须来自白名单。
fn sort_users(users: &mut [UserSummary], query: &UserListQuery) -> ApiResult<()> {
    let sort_by = user_sort_by(query.sort_by.as_deref())?;
    let direction = user_sort_direction(query.sort_direction.as_deref())?;
    users.sort_by(|left, right| {
        let ordering = compare_users(left, right, sort_by).then_with(|| left.id.cmp(&right.id));
        match direction {
            UserListSortDirection::Asc => ordering,
            UserListSortDirection::Desc => ordering.reverse(),
        }
    });

    Ok(())
}

/// 按用户状态过滤后台用户列表；未传状态时保留全部用户。
fn filter_users_by_status(users: &mut Vec<UserSummary>, status: Option<&UserStatus>) {
    let Some(status) = status else {
        return;
    };

    users.retain(|user| &user.status == status);
}

/// 解析用户列表排序字段，默认按用户 ID 排序。
fn user_sort_by(value: Option<&str>) -> ApiResult<UserListSortBy> {
    let value = value.unwrap_or("id").trim();
    if value.is_empty() {
        return Ok(UserListSortBy::Id);
    }

    match value {
        "agentId" => Ok(UserListSortBy::AgentId),
        "balance" | "balanceMinor" => Ok(UserListSortBy::BalanceMinor),
        "email" => Ok(UserListSortBy::Email),
        "id" | "userId" => Ok(UserListSortBy::Id),
        "inviteCode" => Ok(UserListSortBy::InviteCode),
        "kind" | "userKind" => Ok(UserListSortBy::Kind),
        "status" => Ok(UserListSortBy::Status),
        "username" => Ok(UserListSortBy::Username),
        _ => Err(ApiError::BadRequest(format!(
            "不支持的用户排序字段：{value}"
        ))),
    }
}

/// 解析用户列表排序方向，默认倒序，优先让最新或编号更靠后的用户显示在前。
fn user_sort_direction(value: Option<&str>) -> ApiResult<UserListSortDirection> {
    let value = value.unwrap_or("desc").trim();
    if value.is_empty() {
        return Ok(UserListSortDirection::Desc);
    }

    match value {
        "asc" | "ascending" => Ok(UserListSortDirection::Asc),
        "desc" | "descending" => Ok(UserListSortDirection::Desc),
        _ => Err(ApiError::BadRequest(format!(
            "不支持的用户排序方向：{value}"
        ))),
    }
}

/// 根据用户排序字段比较两条用户摘要。
fn compare_users(left: &UserSummary, right: &UserSummary, sort_by: UserListSortBy) -> Ordering {
    match sort_by {
        UserListSortBy::AgentId => {
            optional_text(left.agent_id.as_ref()).cmp(optional_text(right.agent_id.as_ref()))
        }
        UserListSortBy::BalanceMinor => left.balance_minor.cmp(&right.balance_minor),
        UserListSortBy::Email => {
            optional_text(left.email.as_ref()).cmp(optional_text(right.email.as_ref()))
        }
        UserListSortBy::Id => left.id.cmp(&right.id),
        UserListSortBy::InviteCode => left.invite_code.cmp(&right.invite_code),
        UserListSortBy::Kind => user_kind_order(&left.kind).cmp(&user_kind_order(&right.kind)),
        UserListSortBy::Status => {
            user_status_order(&left.status).cmp(&user_status_order(&right.status))
        }
        UserListSortBy::Username => left.username.cmp(&right.username),
    }
}

/// 空值排序时放在非空文本之前，保证查询结果稳定。
fn optional_text(value: Option<&String>) -> &str {
    value.map(String::as_str).unwrap_or("")
}

/// 用户类型排序顺序：普通用户在前，代理在后。
fn user_kind_order(kind: &crate::domain::user::UserKind) -> u8 {
    match kind {
        crate::domain::user::UserKind::Regular => 0,
        crate::domain::user::UserKind::Agent => 1,
    }
}

/// 用户状态排序顺序：启用、停用、锁定。
fn user_status_order(status: &crate::domain::user::UserStatus) -> u8 {
    match status {
        crate::domain::user::UserStatus::Active => 0,
        crate::domain::user::UserStatus::Suspended => 1,
        crate::domain::user::UserStatus::Locked => 2,
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
    let source_issue = state.draws.get(&id).await?;
    let lottery = state.lotteries.get(&source_issue.lottery_id).await?;
    let issue =
        draw_with_avoid_winning_policy(&state.draws, &state.orders, &lottery, &id, payload).await?;
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
) -> ApiResult<Json<ApiEnvelope<FinancePage<AdminSettlementRun>>>> {
    // 计奖派奖批次会持续增长，后台列表按统一分页信封返回。
    let usernames = admin_usernames(&state).await?;
    let settlements = state.orders.settlement_runs().await?;
    let settlements = settlements
        .into_iter()
        .map(|settlement| admin_settlement_run_with_usernames(settlement, &usernames))
        .collect::<Vec<_>>();

    Ok(Json(ApiEnvelope::success(page_items(settlements, query))))
}

/// 返回指定计奖派奖批次详情。
async fn get_settlement(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminSettlementRun>>> {
    let usernames = admin_usernames(&state).await?;
    let settlement = state.orders.get_settlement(&id).await?;
    let settlement = admin_settlement_run_with_usernames(settlement, &usernames);

    Ok(Json(ApiEnvelope::success(settlement)))
}

/// 后台触发指定已开奖期号的订单结算。
async fn settle_draw_issue_orders(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminSettlementRun>>> {
    let draw_issue = state.draws.get(&id).await?;
    let (settlement, entries) = state
        .orders
        .settle_with_payouts(&state.finance, &state.group_buys, &draw_issue)
        .await?;
    publish_settlement_balance_events(&state, &entries).await;
    let usernames = admin_usernames(&state).await?;
    let settlement = admin_settlement_run_with_usernames(settlement, &usernames);

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

/// 推送客服会话删除事件，让后台和用户端移除已删除的已解决会话。
fn publish_support_conversation_deleted(state: &AppState, conversation: &SupportConversation) {
    let event = support_conversation_deleted_event(conversation);
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
    let usernames = username_map_from_users(&access.users);
    let recent_orders = recent_orders
        .into_iter()
        .map(|order| order_summary_with_username(order, &usernames))
        .collect();
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
    let excluded_initiator_user_id = if query.include_robot_data() {
        None
    } else {
        Some(ROBOT_GROUP_BUY_USER_ID)
    };
    let page = state
        .group_buys
        .list_page(
            excluded_initiator_user_id,
            PageRequest::new(query.page, query.page_size),
        )
        .await?;

    Ok(Json(ApiEnvelope::success(page.into_finance_page())))
}

/// 一键清除已结束合买计划历史；未结算计划由仓储自动保留。
async fn clear_group_buy_plans(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<ClearRecordsResult>>> {
    let deleted_count = state.group_buys.clear_records().await?;

    Ok(Json(ApiEnvelope::success(ClearRecordsResult {
        deleted_count,
    })))
}

/// 一键清理纯机器人合买计划和关联机器人合买订单，包含未成单、待开奖和已结算记录。
async fn clear_robot_group_buy_plans(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<ClearRobotGroupBuyRecordsResult>>> {
    let cleanup = state
        .orders
        .remove_robot_group_buy_records(&state.group_buys, ROBOT_GROUP_BUY_USER_ID)
        .await?;
    tracing::info!(
        deleted_plan_count = cleanup.deleted_plan_count,
        deleted_order_count = cleanup.deleted_order_count,
        "后台已一键清理机器人合买记录"
    );

    Ok(Json(ApiEnvelope::success(
        ClearRobotGroupBuyRecordsResult {
            deleted_count: cleanup.deleted_plan_count,
            deleted_order_count: cleanup.deleted_order_count,
        },
    )))
}

/// 返回控奖抽屉当前彩种期号下的合买计划和认购记录，包含未满单、未成单计划。
async fn list_control_group_buy_plans_by_issue(
    State(state): State<AppState>,
    Query(query): Query<ControlGroupBuyIssueQuery>,
) -> ApiResult<Json<ApiEnvelope<Vec<GroupBuyPlan>>>> {
    let plans = state
        .group_buys
        .list_control_details_for_issue(&query.lottery_id, &query.issue)
        .await?;

    Ok(Json(ApiEnvelope::success(plans)))
}

/// 返回指定合买计划详情。
async fn get_group_buy_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let plan = state.group_buys.get(&id).await?;

    Ok(Json(ApiEnvelope::success(plan)))
}

/// 删除机器人发起且未混入真实用户认购的合买计划，主要用于清理机器人测试单据。
async fn delete_robot_group_buy_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let plan = state.group_buys.get(&id).await?;
    ensure_robot_group_buy_plan_can_be_deleted(&state, &plan).await?;

    let (removed_order, deleted) = state
        .orders
        .remove_group_buy_order_and_plan_records(&state.group_buys, plan.order_id.as_deref(), &id)
        .await?;
    if let Some(removed_order) = removed_order {
        publish_user_order_changed(&state, &removed_order, "deleted");
    }

    Ok(Json(ApiEnvelope::success(deleted)))
}

/// 校验机器人合买计划是否可以直接删除，避免真实用户资金和已结算订单失去审计链路。
async fn ensure_robot_group_buy_plan_can_be_deleted(
    state: &AppState,
    plan: &GroupBuyPlan,
) -> ApiResult<()> {
    if !is_group_buy_robot_user_id(&plan.initiator_user_id) {
        return Err(ApiError::BadRequest(
            "只能删除机器人发起的合买计划".to_string(),
        ));
    }
    if plan
        .participants
        .iter()
        .any(|participant| !is_group_buy_robot_user_id(&participant.user_id))
    {
        return Err(ApiError::BadRequest(
            "包含真实用户认购的合买计划不能直接删除，请先走取消退款或结算流程".to_string(),
        ));
    }
    if let Some(order_id) = plan.order_id.as_deref() {
        let order = state.orders.get(order_id).await?;
        if !is_group_buy_robot_user_id(&order.user_id)
            || order.order_source != OrderSource::GroupBuy
        {
            return Err(ApiError::BadRequest(
                "关联订单不是机器人合买订单，不能通过机器人清理入口删除".to_string(),
            ));
        }
        if order.status != OrderStatus::PendingDraw {
            return Err(ApiError::BadRequest(
                "已开奖或已取消的机器人合买订单不能直接删除".to_string(),
            ));
        }
    }

    Ok(())
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

/// 后台删除已解决的客服会话，处理中和等待用户的会话不能直接删除。
async fn delete_support_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let conversation = state.support.delete_resolved(&id).await?;
    publish_support_conversation_deleted(&state, &conversation);

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
    Query(query): Query<UserListQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<AdminUserSummary>>>> {
    let mut users = users_with_financial_balances(&state).await?;
    filter_users_by_status(&mut users, query.status.as_ref());
    sort_users(&mut users, &query)?;
    let usernames = username_map_from_users(&users);
    let users = users
        .into_iter()
        .map(|user| admin_user_summary_with_usernames(user, &usernames))
        .collect::<Vec<_>>();

    Ok(Json(ApiEnvelope::success(page_items(
        users,
        query.page_query(),
    ))))
}

/// 返回指定用户详情。
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminUserSummary>>> {
    let user = user_with_financial_balance(&state, &id).await?;
    let user = admin_user_summary(&state, user).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

/// 后台创建用户并初始化资金账户。
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<UserSummary>,
) -> ApiResult<Json<ApiEnvelope<AdminUserSummary>>> {
    let user = state.access.create_user(payload).await?;
    let account = state.finance.account_or_create(&user.id).await?;
    let user = user_with_account_balance(user, Some(&account));
    let user = admin_user_summary(&state, user).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

/// 后台更新用户基础资料，不直接修改余额和邀请码。
async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UserSummary>,
) -> ApiResult<Json<ApiEnvelope<AdminUserSummary>>> {
    let user = state.access.update_user(&id, payload).await?;
    let user = user_with_financial_balance_from_summary(&state, user).await?;
    let user = admin_user_summary(&state, user).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

/// 后台删除用户账号资料；历史资金、订单等业务记录继续保留用户 ID 作为审计线索。
async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminUserSummary>>> {
    ensure_user_can_be_deleted(&state, &id).await?;
    let user = state.access.delete_user(&id).await?;
    let user = admin_user_summary(&state, user).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

/// 后台切换用户状态。
async fn set_user_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UserStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminUserSummary>>> {
    let user = state.access.set_user_status(&id, payload.status).await?;
    publish_user_account_status_change(&state, &user);
    let user = user_with_financial_balance_from_summary(&state, user).await?;
    let user = admin_user_summary(&state, user).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

/// 用户被停用或锁定后立即通知本人在线端退出登录；启用状态不推送强退事件。
fn publish_user_account_status_change(state: &AppState, user: &UserSummary) {
    if user.status == UserStatus::Active {
        return;
    }

    let (status, reason) = match user.status {
        UserStatus::Suspended => ("suspended", "用户账号已停用"),
        UserStatus::Locked => ("locked", "用户账号已锁定"),
        UserStatus::Active => ("active", "用户账号已启用"),
    };
    state.realtime.publish_user(
        &user.id,
        user_account_status_changed_event(&user.id, status, reason),
    );
    tracing::info!(
        user_id = %user.id,
        username = %user.username,
        status = %status,
        "已推送用户账号状态变更强制下线事件"
    );
}

/// 后台重置普通用户登录密码。
async fn reset_user_password(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UserPasswordResetRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminUserSummary>>> {
    let user = state.access.reset_user_password(&id, payload).await?;
    let user = user_with_financial_balance_from_summary(&state, user).await?;
    let user = admin_user_summary(&state, user).await?;

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

/// 删除用户前校验资金账户是否已经处理完毕，避免把有余额或冻结资金的账号移出用户列表。
async fn ensure_user_can_be_deleted(state: &AppState, id: &str) -> ApiResult<()> {
    state.access.get_user(id).await?;
    let accounts = state.finance.accounts().await?;
    if let Some(account) = accounts.iter().find(|account| account.user_id == id) {
        if account.available_balance_minor != 0 || account.frozen_balance_minor != 0 {
            return Err(ApiError::Conflict(
                "用户资金账户仍有余额或冻结金额，请先通过财务处理后再删除用户".to_string(),
            ));
        }
    }

    Ok(())
}

/// 为后台用户展示项补充上级代理用户名。
async fn admin_user_summary(state: &AppState, user: UserSummary) -> ApiResult<AdminUserSummary> {
    let usernames = admin_usernames(state).await?;
    Ok(admin_user_summary_with_usernames(user, &usernames))
}

/// 使用已加载的用户名映射包装用户，避免列表页重复查询代理账号。
fn admin_user_summary_with_usernames(
    user: UserSummary,
    usernames: &BTreeMap<String, String>,
) -> AdminUserSummary {
    let agent_username = user
        .agent_id
        .as_ref()
        .and_then(|agent_id| usernames.get(agent_id).cloned());
    AdminUserSummary {
        user,
        agent_username,
    }
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

/// 手动从数据库刷新后端快照型内存缓存，供清表或直接改库后的维护使用。
async fn reload_memory_cache(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<MemoryCacheReloadResult>>> {
    let result = state.reload_memory_cache_from_database().await?;
    tracing::info!(
        refreshed_at = %result.refreshed_at,
        reloaded_module_count = result.reloaded_modules.len(),
        database_direct_module_count = result.database_direct_modules.len(),
        skipped_module_count = result.skipped_modules.len(),
        "后台手动刷新内存缓存完成"
    );

    Ok(Json(ApiEnvelope::success(result)))
}

/// 一键清除聊天大厅历史消息；只清除大厅展示记录，不回滚已产生的资金流水。
async fn clear_chat_hall_messages(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<ClearRecordsResult>>> {
    let deleted_count = state.chat_hall.clear_messages().await?;
    if deleted_count > 0 {
        state
            .realtime
            .publish_public(chat_hall_messages_cleared_event());
    }

    Ok(Json(ApiEnvelope::success(ClearRecordsResult {
        deleted_count,
    })))
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

/// 处理管理员 APP 安装包上传请求：复用图床配置透传 APK/IPA 文件并返回下载链接。
async fn upload_app_package_file(
    State(state): State<AppState>,
    payload: Multipart,
) -> ApiResult<Json<ApiEnvelope<Value>>> {
    let output = upload_configured_image_bed_file(
        &state.access,
        payload,
        ImageBedUploadOptions {
            image_only: false,
            missing_file_message: "未检测到 APP 安装包文件字段",
            default_file_name: "app-package.bin",
        },
    )
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

/// 返回代理邀请返利统计分页列表。
async fn list_agent_rebate_statistics(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<AgentRebateSummary>>>> {
    let summaries = agent_rebate_summaries(&state).await?;

    Ok(Json(ApiEnvelope::success(page_items(summaries, query))))
}

/// 返回指定代理的每一笔下级充值返利记录。
async fn list_agent_rebate_records(
    State(state): State<AppState>,
    Path(agent_user_id): Path<String>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<AgentRebateRecord>>>> {
    ensure_agent_user(&state, &agent_user_id).await?;
    let records = agent_rebate_record_page(
        &state,
        Some(&agent_user_id),
        PageRequest::new(query.page, query.page_size),
    )
    .await?;

    Ok(Json(ApiEnvelope::success(records)))
}

/// 后台处理代理返利提现，扣减代理可用余额并生成返利提现流水。
async fn process_agent_rebate_withdrawal(
    State(state): State<AppState>,
    Path(agent_user_id): Path<String>,
    Json(payload): Json<AgentRebateWithdrawalRequest>,
) -> ApiResult<Json<ApiEnvelope<LedgerEntry>>> {
    let agent = ensure_agent_user(&state, &agent_user_id).await?;
    let amount_minor = payload.amount_minor;
    if amount_minor <= 0 {
        return Err(ApiError::BadRequest("返利提现金额必须大于 0".to_string()));
    }
    let summary = agent_rebate_summary_for_agent(&state, &agent_user_id).await?;
    if amount_minor > summary.withdrawable_rebate_minor {
        return Err(ApiError::BadRequest("返利可提现金额不足".to_string()));
    }
    let description = payload.description.trim();
    let description = if description.is_empty() {
        format!("代理返利提现处理：{}", agent.username)
    } else {
        description.to_string()
    };
    let entry = state
        .finance
        .withdraw_agent_rebate(&agent_user_id, amount_minor, &description)
        .await?;
    publish_user_balance_changed(
        &state,
        &entry.user_id,
        "agent_rebate_withdrawal",
        entry.reference_id.as_deref(),
    )
    .await;

    Ok(Json(ApiEnvelope::success(entry)))
}

/// 返回后台代理申请分页列表，可按待审核、已通过或已驳回筛选。
async fn list_agent_applications(
    State(state): State<AppState>,
    Query(query): Query<AgentApplicationListQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<AgentApplication>>>> {
    let applications = state.agent_applications.list(query.status.clone()).await?;

    Ok(Json(ApiEnvelope::success(page_items(
        applications,
        query.page_query(),
    ))))
}

/// 后台审核代理申请；通过时会把用户类型升级为代理，之后邀请码才具备邀请功能。
async fn review_agent_application(
    State(state): State<AppState>,
    Extension(session): Extension<AdminAuthSession>,
    Path(id): Path<String>,
    Json(payload): Json<ReviewAgentApplicationRequest>,
) -> ApiResult<Json<ApiEnvelope<AgentApplication>>> {
    let current = state.agent_applications.get(&id).await?;
    if !matches!(&current.status, AgentApplicationStatus::Pending) {
        return Err(ApiError::BadRequest(
            "代理申请已经审核，不能重复处理".to_string(),
        ));
    }

    if payload.approved {
        let mut user = state.access.get_user(&current.user_id).await?;
        if !matches!(&user.status, UserStatus::Active) {
            return Err(ApiError::BadRequest(
                "申请用户状态异常，不能升级为代理".to_string(),
            ));
        }
        if !matches!(&user.kind, UserKind::Agent) {
            user.kind = UserKind::Agent;
            let user_id = user.id.clone();
            state.access.update_user(&user_id, user).await?;
        }
    }

    let application = state
        .agent_applications
        .review(&id, payload, &session.admin)
        .await?;

    Ok(Json(ApiEnvelope::success(application)))
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

/// 后台删除普通机器人配置；核心内置机器人由仓储层保护。
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
#[cfg(test)]
fn should_include_user_scoped_record(include_robot_data: bool, user_id: &str) -> bool {
    include_robot_data || !is_group_buy_robot_user_id(user_id)
}

/// 判断列表行是否命中指定用户筛选；未传用户 ID 时不过滤。
#[cfg(test)]
fn should_match_user_filter(query: &FinancePageQuery, user_id: &str) -> bool {
    query
        .user_id_filter()
        .map_or(true, |target_user_id| target_user_id == user_id)
}

/// 判断后台合买计划列表是否展示机器人发起的计划；机器人只作为参与人补单时不影响展示。
#[cfg(test)]
fn should_include_robot_initiated_group_buy_plan(
    include_robot_data: bool,
    initiator_user_id: &str,
) -> bool {
    include_robot_data || !is_group_buy_robot_user_id(initiator_user_id)
}

/// 生成充值订单 CSV 文本，带 UTF-8 BOM 方便表格软件直接识别中文。
fn recharge_orders_csv(orders: &[RechargeOrderSummary]) -> String {
    let mut csv = String::from(
        "\u{feff}订单ID,用户ID,用户名,充值渠道,支付方式,金额(元),状态,外部交易号,客服会话ID,创建时间,入账时间\n",
    );
    for order in orders {
        push_csv_row(
            &mut csv,
            &[
                order.id.clone(),
                order.user_id.clone(),
                order.username.clone(),
                recharge_channel_label(&order.channel).to_string(),
                order.pay_type.clone().unwrap_or_default(),
                minor_to_money_text(order.amount_minor),
                recharge_status_label(&order.status).to_string(),
                order.provider_trade_no.clone().unwrap_or_default(),
                order.support_conversation_id.clone().unwrap_or_default(),
                order.created_at.clone(),
                order.paid_at.clone().unwrap_or_default(),
            ],
        );
    }
    csv
}

/// 追加一行 CSV，并对逗号、引号和换行做标准转义。
fn push_csv_row(csv: &mut String, columns: &[String]) {
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            csv.push(',');
        }
        csv.push_str(&csv_escape(column));
    }
    csv.push('\n');
}

/// 转义单个 CSV 字段，保证用户昵称或交易号包含特殊字符时不会错列。
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// 把分转换为固定两位小数的元金额文本，避免导出时出现浮点误差。
fn minor_to_money_text(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let absolute = amount_minor.checked_abs().unwrap_or(i64::MAX);
    format!("{sign}{}.{:02}", absolute / 100, absolute % 100)
}

/// 返回充值渠道中文标签。
fn recharge_channel_label(channel: &RechargeChannel) -> &'static str {
    match channel {
        RechargeChannel::RainbowEpay => "彩虹易支付",
        RechargeChannel::CustomerService => "客服直充",
    }
}

/// 返回充值订单状态中文标签。
fn recharge_status_label(status: &RechargeOrderStatus) -> &'static str {
    match status {
        RechargeOrderStatus::Pending => "待支付",
        RechargeOrderStatus::WaitingCustomerService => "等待客服",
        RechargeOrderStatus::Paid => "已入账",
        RechargeOrderStatus::Cancelled => "已取消",
    }
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
    let excluded_user_id = if query.include_robot_data() {
        None
    } else {
        Some(ROBOT_GROUP_BUY_USER_ID)
    };
    let users = state.access.users().await?;
    let usernames: BTreeMap<String, String> = users
        .into_iter()
        .map(|user| (user.id, user.username))
        .collect();
    let page = state
        .finance
        .account_page(
            query.user_id_filter(),
            query.username_filter(),
            &usernames,
            excluded_user_id,
            PageRequest::new(query.page, query.page_size),
        )
        .await?;
    let accounts = page
        .items
        .into_iter()
        .map(|account| AdminFinancialAccountSummary {
            username: usernames.get(&account.user_id).cloned(),
            user_id: account.user_id,
            available_balance_minor: account.available_balance_minor,
            frozen_balance_minor: account.frozen_balance_minor,
        })
        .collect::<Vec<_>>();

    Ok(Json(ApiEnvelope::success(FinancePage {
        items: accounts,
        page: page.page,
        page_size: page.page_size,
        total_count: page.total_count,
        total_pages: page.total_pages,
    })))
}

/// 返回资金流水分页列表。
async fn list_ledger_entries(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<AdminLedgerEntry>>>> {
    let usernames = admin_usernames(&state).await?;
    let excluded_user_id = if query.include_robot_data() {
        None
    } else {
        Some(ROBOT_GROUP_BUY_USER_ID)
    };
    let page = state
        .finance
        .ledger_entry_page(
            query.user_id_filter(),
            excluded_user_id,
            PageRequest::new(query.page, query.page_size),
        )
        .await?;
    let entries = page
        .items
        .into_iter()
        .map(|entry| admin_ledger_entry_with_usernames(entry, &usernames))
        .collect::<Vec<_>>();

    Ok(Json(ApiEnvelope::success(FinancePage {
        items: entries,
        page: page.page,
        page_size: page.page_size,
        total_count: page.total_count,
        total_pages: page.total_pages,
    })))
}

/// 一键清除资金流水历史；不会回滚用户余额，也不会重置后续流水编号。
async fn clear_ledger_entries(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<ClearRecordsResult>>> {
    let deleted_count = state.finance.clear_ledger_entries().await?;

    Ok(Json(ApiEnvelope::success(ClearRecordsResult {
        deleted_count,
    })))
}

/// 返回充值订单分页列表。
async fn list_recharge_orders(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<RechargeOrderSummary>>>> {
    let page = state
        .recharges
        .list_page(PageRequest::new(query.page, query.page_size))
        .await?;

    Ok(Json(ApiEnvelope::success(page.into_finance_page())))
}

/// 导出全部充值订单为 CSV 文件，供后台财务留档或离线核对。
async fn export_recharge_orders(State(state): State<AppState>) -> ApiResult<Response> {
    let mut orders = state.recharges.list().await?;
    sort_recharge_orders_by_time_desc(&mut orders);
    let csv = recharge_orders_csv(&orders);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"recharge-orders.csv\""),
    );

    Ok((headers, csv).into_response())
}

/// 一键清除充值订单历史；不会回滚已入账余额和资金流水。
async fn clear_recharge_orders(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<ClearRecordsResult>>> {
    let deleted_count = state.recharges.clear_records().await?;

    Ok(Json(ApiEnvelope::success(ClearRecordsResult {
        deleted_count,
    })))
}

/// 返回提现申请分页列表。
async fn list_withdrawal_orders(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<WithdrawalOrderSummary>>>> {
    let page = state
        .withdrawals
        .list_page(PageRequest::new(query.page, query.page_size))
        .await?;

    Ok(Json(ApiEnvelope::success(page.into_finance_page())))
}

/// 一键清除提现申请历史；存在待审核申请时由仓储拒绝执行。
async fn clear_withdrawal_orders(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<ClearRecordsResult>>> {
    let deleted_count = state.withdrawals.clear_records().await?;

    Ok(Json(ApiEnvelope::success(ClearRecordsResult {
        deleted_count,
    })))
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
) -> ApiResult<Json<ApiEnvelope<FinancePage<AdminOrderDetail>>>> {
    let usernames = admin_usernames(&state).await?;
    let excluded_user_id = if query.include_robot_data() {
        None
    } else {
        Some(ROBOT_GROUP_BUY_USER_ID)
    };
    let page = state
        .orders
        .list_page(
            query.user_id_filter(),
            excluded_user_id,
            PageRequest::new(query.page, query.page_size),
        )
        .await?;
    let orders = page
        .items
        .into_iter()
        .map(|order| admin_order_detail_with_usernames(order, &usernames))
        .collect::<Vec<_>>();

    Ok(Json(ApiEnvelope::success(FinancePage {
        items: orders,
        page: page.page,
        page_size: page.page_size,
        total_count: page.total_count,
        total_pages: page.total_pages,
    })))
}

/// 一键清除投注订单和计奖派奖历史；存在待开奖订单时由仓储拒绝执行。
async fn clear_bet_orders(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<ClearRecordsResult>>> {
    let deleted_count = state.orders.clear_bet_records().await?;

    Ok(Json(ApiEnvelope::success(ClearRecordsResult {
        deleted_count,
    })))
}

/// 返回指定投注订单详情。
async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminOrderDetail>>> {
    let order = state.orders.get(&id).await?;
    let order = admin_order_detail(&state, order).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 按投注订单反查合买计划详情，供订单列表查看该合买订单的全部认购记录。
async fn get_order_group_buy_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let order = state.orders.get(&id).await?;
    if order.order_source != OrderSource::GroupBuy {
        return Err(ApiError::BadRequest("该订单不是合买下单".to_string()));
    }

    let plans = state
        .group_buys
        .plans_for_order_ids(std::slice::from_ref(&order.id))
        .await?;
    let plan = plans
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::NotFound("合买订单认购记录不存在".to_string()))?;

    Ok(Json(ApiEnvelope::success(plan)))
}

/// 后台代创建投注订单并扣款。
async fn create_order(
    State(state): State<AppState>,
    Json(payload): Json<CreateOrderRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminOrderDetail>>> {
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
    let order = admin_order_detail(&state, order).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 后台取消待开奖订单并退款。
async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminOrderDetail>>> {
    let order = state.orders.cancel_with_refund(&state.finance, &id).await?;
    publish_user_order_changed(&state, &order, "cancelled");
    publish_user_balance_changed(&state, &order.user_id, "order_refund", Some(&order.id)).await;
    let order = admin_order_detail(&state, order).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

/// 读取用户名称映射，后台表格只需要展示用户名，不把用户维护中的余额等其它字段混进业务领域。
async fn admin_usernames(state: &AppState) -> ApiResult<BTreeMap<String, String>> {
    Ok(state
        .access
        .users()
        .await?
        .into_iter()
        .map(|user| (user.id, user.username))
        .collect())
}

/// 从已有用户列表生成用户名称映射，供看板聚合接口避免重复读取用户仓储。
fn username_map_from_users(users: &[UserSummary]) -> BTreeMap<String, String> {
    users
        .iter()
        .map(|user| (user.id.clone(), user.username.clone()))
        .collect()
}

/// 为首页订单摘要补充用户名。
fn order_summary_with_username(
    mut order: OrderSummary,
    usernames: &BTreeMap<String, String>,
) -> OrderSummary {
    order.username = usernames.get(&order.user_id).cloned();
    order
}

/// 为单条后台订单补充用户名，供创建、取消和详情接口复用。
async fn admin_order_detail(state: &AppState, order: OrderDetail) -> ApiResult<AdminOrderDetail> {
    let usernames = admin_usernames(state).await?;
    Ok(admin_order_detail_with_usernames(order, &usernames))
}

/// 使用已加载的用户名映射包装订单，避免分页列表对每条订单重复查询用户。
fn admin_order_detail_with_usernames(
    order: OrderDetail,
    usernames: &BTreeMap<String, String>,
) -> AdminOrderDetail {
    let username = usernames.get(&order.user_id).cloned();
    AdminOrderDetail { order, username }
}

/// 为后台资金流水补充用户名。
fn admin_ledger_entry_with_usernames(
    entry: LedgerEntry,
    usernames: &BTreeMap<String, String>,
) -> AdminLedgerEntry {
    let username = usernames.get(&entry.user_id).cloned();
    AdminLedgerEntry { entry, username }
}

/// 校验并返回代理用户，后台返利提现只允许处理代理账户。
async fn ensure_agent_user(state: &AppState, agent_user_id: &str) -> ApiResult<UserSummary> {
    let agent = state.access.get_user(agent_user_id).await?;
    if !matches!(agent.kind, UserKind::Agent) {
        return Err(ApiError::BadRequest("只能处理代理用户的返利".to_string()));
    }

    Ok(agent)
}

/// 读取并构造全部代理返利统计列表。
async fn agent_rebate_summaries(state: &AppState) -> ApiResult<Vec<AgentRebateSummary>> {
    let users = state.access.users().await?;
    let accounts = state.finance.accounts().await?;
    let entries = state
        .finance
        .ledger_entries_by_kinds(&[
            LedgerEntryKind::RechargeRebateCredit,
            LedgerEntryKind::AgentRebateWithdrawal,
        ])
        .await?;
    let invite_records = state.invites.list().await?;
    let recharges = state.recharges.paid_orders().await?;
    let withdrawals = state.withdrawals.approved_orders().await?;
    let account_by_user_id = accounts
        .into_iter()
        .map(|account| (account.user_id.clone(), account))
        .collect::<BTreeMap<_, _>>();

    let mut summaries = users
        .iter()
        .filter(|user| matches!(user.kind, UserKind::Agent))
        .map(|agent| {
            agent_rebate_summary_from_data(
                agent,
                &users,
                &invite_records,
                &entries,
                &recharges,
                &withdrawals,
                &account_by_user_id,
            )
        })
        .collect::<ApiResult<Vec<_>>>()?;
    sort_agent_rebate_summaries(&mut summaries);
    Ok(summaries)
}

/// 返回单个代理的返利统计，没有返利记录也会返回零值统计。
async fn agent_rebate_summary_for_agent(
    state: &AppState,
    agent_user_id: &str,
) -> ApiResult<AgentRebateSummary> {
    let agent = ensure_agent_user(state, agent_user_id).await?;
    let users = state.access.users().await?;
    let accounts = state.finance.accounts().await?;
    let entries = state
        .finance
        .ledger_entry_kind_page(
            Some(agent_user_id),
            &[
                LedgerEntryKind::RechargeRebateCredit,
                LedgerEntryKind::AgentRebateWithdrawal,
            ],
            PageRequest::default(),
        )
        .await?
        .items;
    let invite_records = state.invites.list().await?;
    let recharges = state.recharges.paid_orders().await?;
    let withdrawals = state.withdrawals.approved_orders().await?;
    let account_by_user_id = accounts
        .into_iter()
        .map(|account| (account.user_id.clone(), account))
        .collect::<BTreeMap<_, _>>();

    agent_rebate_summary_from_data(
        &agent,
        &users,
        &invite_records,
        &entries,
        &recharges,
        &withdrawals,
        &account_by_user_id,
    )
}

/// 基于已加载数据构造单个代理返利统计，避免列表页重复读仓储。
fn agent_rebate_summary_from_data(
    agent: &UserSummary,
    users: &[UserSummary],
    invite_records: &[InviteRecord],
    entries: &[LedgerEntry],
    recharges: &[RechargeOrderSummary],
    withdrawals: &[WithdrawalOrderSummary],
    account_by_user_id: &BTreeMap<String, FinancialAccountSummary>,
) -> ApiResult<AgentRebateSummary> {
    let mut total_rebate_minor = 0_i64;
    let mut withdrawn_rebate_minor = 0_i64;
    let mut rebate_record_count = 0_usize;
    let mut last_rebate_at: Option<String> = None;

    for entry in entries.iter().filter(|entry| entry.user_id == agent.id) {
        match entry.kind {
            LedgerEntryKind::RechargeRebateCredit => {
                if entry.amount_minor > 0 {
                    total_rebate_minor = total_rebate_minor
                        .checked_add(entry.amount_minor)
                        .ok_or_else(|| ApiError::Internal("代理返利金额汇总溢出".to_string()))?;
                }
                rebate_record_count += 1;
                if admin_time_is_newer(&entry.created_at, last_rebate_at.as_deref()) {
                    last_rebate_at = Some(entry.created_at.clone());
                }
            }
            LedgerEntryKind::AgentRebateWithdrawal => {
                let amount = entry
                    .amount_minor
                    .checked_neg()
                    .ok_or_else(|| ApiError::Internal("代理返利提现金额汇总溢出".to_string()))?;
                if amount > 0 {
                    withdrawn_rebate_minor = withdrawn_rebate_minor
                        .checked_add(amount)
                        .ok_or_else(|| ApiError::Internal("代理返利提现汇总溢出".to_string()))?;
                }
            }
            _ => {}
        }
    }

    let pending_rebate_minor = total_rebate_minor
        .checked_sub(withdrawn_rebate_minor)
        .unwrap_or_default()
        .max(0);
    let account_available_balance_minor = account_by_user_id
        .get(&agent.id)
        .map(|account| account.available_balance_minor)
        .unwrap_or_default();
    let withdrawable_rebate_minor = pending_rebate_minor.min(account_available_balance_minor);

    let direct_invitee_ids = direct_invitee_ids(&agent.id, users, invite_records);
    let direct_invitee_recharge_minor =
        paid_recharge_total_for_users(recharges, &direct_invitee_ids);
    let direct_invitee_withdrawal_minor =
        approved_withdrawal_total_for_users(withdrawals, &direct_invitee_ids);

    Ok(AgentRebateSummary {
        account_available_balance_minor,
        agent_user_id: agent.id.clone(),
        agent_username: agent.username.clone(),
        direct_invitee_count: direct_invitee_ids.len(),
        direct_invitee_recharge_minor,
        direct_invitee_withdrawal_minor,
        invite_code: agent.invite_code.clone(),
        last_rebate_at,
        pending_rebate_minor,
        rebate_record_count,
        total_rebate_minor,
        withdrawable_rebate_minor,
        withdrawn_rebate_minor,
    })
}

/// 分页返回指定代理或全部代理的返利记录，记录来源以充值返利流水为准。
async fn agent_rebate_record_page(
    state: &AppState,
    agent_user_id: Option<&str>,
    page: PageRequest,
) -> ApiResult<FinancePage<AgentRebateRecord>> {
    let users = state.access.users().await?;
    let entries = state
        .finance
        .ledger_entry_kind_page(
            agent_user_id,
            &[LedgerEntryKind::RechargeRebateCredit],
            page,
        )
        .await?;
    let recharges = state.recharges.paid_orders().await?;
    let withdrawals = state.withdrawals.approved_orders().await?;
    let items = agent_rebate_records_from_data(
        agent_user_id,
        &users,
        &entries.items,
        &recharges,
        &withdrawals,
    );

    Ok(FinancePage {
        items,
        page: entries.page,
        page_size: entries.page_size,
        total_count: entries.total_count,
        total_pages: entries.total_pages,
    })
}

/// 基于资金流水和充值订单构造返利明细，充值订单被清理时仍保留流水本身。
fn agent_rebate_records_from_data(
    agent_user_id: Option<&str>,
    users: &[UserSummary],
    entries: &[LedgerEntry],
    recharges: &[RechargeOrderSummary],
    withdrawals: &[WithdrawalOrderSummary],
) -> Vec<AgentRebateRecord> {
    let users_by_id = users
        .iter()
        .map(|user| (user.id.as_str(), user))
        .collect::<BTreeMap<_, _>>();
    let recharges_by_id = recharges
        .iter()
        .map(|order| (order.id.as_str(), order))
        .collect::<BTreeMap<_, _>>();
    let invitee_recharge_totals = invitee_recharge_totals(recharges);
    let invitee_withdrawal_totals = invitee_withdrawal_totals(withdrawals);

    entries
        .iter()
        .filter(|entry| entry.kind == LedgerEntryKind::RechargeRebateCredit)
        .filter(|entry| agent_user_id.map_or(true, |agent_id| entry.user_id == agent_id))
        .map(|entry| {
            let recharge_order_id = rebate_recharge_order_id(entry);
            let recharge = recharge_order_id
                .as_deref()
                .and_then(|order_id| recharges_by_id.get(order_id).copied());
            let invitee_user_id = recharge
                .map(|order| order.user_id.clone())
                .or_else(|| invitee_user_id_from_rebate_description(&entry.description));
            let invitee_username = recharge.map(|order| order.username.clone()).or_else(|| {
                invitee_user_id
                    .as_deref()
                    .and_then(|user_id| users_by_id.get(user_id).map(|user| user.username.clone()))
            });
            let invitee_total_withdrawal_minor = invitee_user_id
                .as_deref()
                .and_then(|user_id| invitee_withdrawal_totals.get(user_id).copied())
                .unwrap_or_default();
            let invitee_total_recharge_minor = invitee_user_id
                .as_deref()
                .and_then(|user_id| invitee_recharge_totals.get(user_id).copied())
                .unwrap_or_default();
            let agent_username = users_by_id
                .get(entry.user_id.as_str())
                .map(|user| user.username.clone())
                .unwrap_or_else(|| "未知代理".to_string());

            AgentRebateRecord {
                agent_user_id: entry.user_id.clone(),
                agent_username,
                created_at: entry.created_at.clone(),
                invitee_user_id,
                invitee_username,
                invitee_total_recharge_minor,
                invitee_total_withdrawal_minor,
                ledger_entry_id: entry.id.clone(),
                rebate_amount_minor: entry.amount_minor,
                recharge_amount_minor: recharge.map(|order| order.amount_minor),
                recharge_order_id,
            }
        })
        .collect()
}

/// 按用户汇总已入账充值金额，用于代理返利详情展示下级实际充值总额。
fn invitee_recharge_totals(recharges: &[RechargeOrderSummary]) -> BTreeMap<String, i64> {
    let mut totals = BTreeMap::new();
    for order in recharges
        .iter()
        .filter(|order| order.status == RechargeOrderStatus::Paid)
    {
        let entry = totals.entry(order.user_id.clone()).or_insert(0_i64);
        *entry = entry.saturating_add(order.amount_minor.max(0));
    }
    totals
}

/// 按用户汇总已通过提现金额，用于代理返利详情展示下级实际提现总额。
fn invitee_withdrawal_totals(withdrawals: &[WithdrawalOrderSummary]) -> BTreeMap<String, i64> {
    let mut totals = BTreeMap::new();
    for order in withdrawals
        .iter()
        .filter(|order| order.status == WithdrawalOrderStatus::Approved)
    {
        let entry = totals.entry(order.user_id.clone()).or_insert(0_i64);
        *entry = entry.saturating_add(order.amount_minor.max(0));
    }
    totals
}

/// 汇总指定用户集合的已入账充值金额，用于代理详情顶部展示直属下级累计充值。
fn paid_recharge_total_for_users(
    recharges: &[RechargeOrderSummary],
    user_ids: &BTreeSet<String>,
) -> i64 {
    recharges
        .iter()
        .filter(|order| {
            order.status == RechargeOrderStatus::Paid && user_ids.contains(&order.user_id)
        })
        .fold(0_i64, |total, order| {
            total.saturating_add(order.amount_minor.max(0))
        })
}

/// 汇总指定用户集合的已通过提现金额，用于代理详情顶部展示直属下级累计提现。
fn approved_withdrawal_total_for_users(
    withdrawals: &[WithdrawalOrderSummary],
    user_ids: &BTreeSet<String>,
) -> i64 {
    withdrawals
        .iter()
        .filter(|order| {
            order.status == WithdrawalOrderStatus::Approved && user_ids.contains(&order.user_id)
        })
        .fold(0_i64, |total, order| {
            total.saturating_add(order.amount_minor.max(0))
        })
}

/// 收集代理的直属下级 ID，合并后台邀请关系和注册时绑定的上级代理。
fn direct_invitee_ids(
    agent_user_id: &str,
    users: &[UserSummary],
    invite_records: &[InviteRecord],
) -> BTreeSet<String> {
    let mut invitee_ids = BTreeSet::new();
    for record in invite_records.iter().filter(|record| {
        record.inviter_user_id == agent_user_id && matches!(record.status, InviteStatus::Active)
    }) {
        invitee_ids.insert(record.invitee_user_id.clone());
    }
    for user in users
        .iter()
        .filter(|user| user.agent_id.as_deref() == Some(agent_user_id))
    {
        invitee_ids.insert(user.id.clone());
    }
    invitee_ids
}

/// 从充值返利流水引用 ID 中恢复充值订单号。
fn rebate_recharge_order_id(entry: &LedgerEntry) -> Option<String> {
    entry
        .reference_id
        .as_deref()
        .and_then(|reference_id| reference_id.strip_prefix("recharge-rebate:"))
        .map(ToString::to_string)
}

/// 兼容充值订单历史被清理后的旧流水说明，尽量恢复下级用户 ID。
fn invitee_user_id_from_rebate_description(description: &str) -> Option<String> {
    description
        .rsplit_once("下级 ")
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// 判断候选时间是否比当前时间更新，兼容 `unix:` 历史标签。
fn admin_time_is_newer(candidate: &str, current: Option<&str>) -> bool {
    let Some(current) = current else {
        return true;
    };
    parse_admin_list_timestamp_seconds(candidate)
        .cmp(&parse_admin_list_timestamp_seconds(current))
        .then_with(|| candidate.cmp(current))
        .is_gt()
}

/// 代理返利统计按最近返利、待处理金额和代理 ID 倒序展示。
fn sort_agent_rebate_summaries(summaries: &mut [AgentRebateSummary]) {
    summaries.sort_by(|left, right| {
        parse_admin_list_timestamp_seconds(right.last_rebate_at.as_deref().unwrap_or(""))
            .cmp(&parse_admin_list_timestamp_seconds(
                left.last_rebate_at.as_deref().unwrap_or(""),
            ))
            .then_with(|| right.pending_rebate_minor.cmp(&left.pending_rebate_minor))
            .then_with(|| right.agent_user_id.cmp(&left.agent_user_id))
    });
}

/// 代理返利明细按返利流水创建时间倒序展示。
#[cfg(test)]
fn sort_agent_rebate_records_by_time_desc(records: &mut [AgentRebateRecord]) {
    records.sort_by(|left, right| {
        compare_admin_time_desc(
            &left.created_at,
            &left.ledger_entry_id,
            &right.created_at,
            &right.ledger_entry_id,
        )
    });
}

/// 为后台结算批次中的每条订单明细补充用户名。
fn admin_settlement_run_with_usernames(
    settlement: SettlementRun,
    usernames: &BTreeMap<String, String>,
) -> AdminSettlementRun {
    AdminSettlementRun {
        id: settlement.id,
        draw_issue_id: settlement.draw_issue_id,
        lottery_id: settlement.lottery_id,
        lottery_name: settlement.lottery_name,
        issue: settlement.issue,
        draw_number: settlement.draw_number,
        settled_order_count: settlement.settled_order_count,
        winning_order_count: settlement.winning_order_count,
        total_stake_amount_minor: settlement.total_stake_amount_minor,
        total_payout_minor: settlement.total_payout_minor,
        created_at: settlement.created_at,
        orders: settlement
            .orders
            .into_iter()
            .map(|order| {
                let username = usernames.get(&order.user_id).cloned();
                AdminOrderSettlement {
                    settlement: order,
                    username,
                }
            })
            .collect(),
    }
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

/// 手动按 API 开奖源校准指定彩种的下一期开奖期号。
async fn sync_lottery_draw_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawSourceSyncResult>>> {
    let lottery = state.lotteries.get(&id).await?;
    let now = Local::now()
        .naive_local()
        .format(TIMESTAMP_FORMAT)
        .to_string();
    let protected_issues = pending_order_issues_for_lottery(&state, &lottery.id).await?;
    let result = state
        .draws
        .sync_api_draw_source(
            &lottery,
            &now,
            lottery.sale_close_lead_seconds,
            &protected_issues,
        )
        .await?;

    state
        .realtime
        .publish_public(issue_opened_event(&result.target_issue));

    Ok(Json(ApiEnvelope::success(result)))
}

/// 读取有待开奖订单的期号，同步开奖源时不能静默取消这些期号。
async fn pending_order_issues_for_lottery(
    state: &AppState,
    lottery_id: &str,
) -> ApiResult<BTreeSet<String>> {
    Ok(state
        .orders
        .list()
        .await?
        .into_iter()
        .filter(|order| order.lottery_id == lottery_id && order.status == OrderStatus::PendingDraw)
        .map(|order| order.issue)
        .collect())
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
            sale_close_lead_seconds: Some(lottery.sale_close_lead_seconds),
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
        admin_login_audit_context_from_headers, admin_user_summary_with_usernames,
        agent_rebate_records_from_data, agent_rebate_summary_from_data,
        align_draw_issue_plan_after_sale_on, filter_users_by_status, finance_overview_for_query,
        first_admin_audit_ip, normalize_admin_draw_control_target, page_items,
        required_permission_for_request, required_scope_for_path,
        should_align_draw_issue_plan_after_sale_on, should_include_robot_initiated_group_buy_plan,
        should_include_user_scoped_record, should_match_user_filter,
        sort_agent_rebate_records_by_time_desc, sort_financial_accounts_by_latest_user_desc,
        sort_ledger_entries_by_time_desc, sort_recharge_orders_by_time_desc, sort_users,
        sort_withdrawal_orders_by_time_desc, username_map_from_users, FinancePageQuery,
        UserListQuery,
    };
    use crate::services::group_buy_robot::ROBOT_GROUP_BUY_USER_ID;
    use crate::{
        app::AppState,
        domain::{
            draw::{
                CreateDrawIssueRequest, DrawControlTargetScope, DrawIssueResultRequest,
                DrawIssueStatus, SaveLotteryDrawControlRequest,
            },
            finance::{
                AdminFinancialAccountSummary, FinancialAccountSummary, LedgerEntry, LedgerEntryKind,
            },
            invite::{InviteRecord, InviteStatus},
            lottery::DrawMode,
            order::CreateOrderRequest,
            permission::PermissionScope,
            play::{PlayRuleCode, PlaySelection},
            recharge::{RechargeChannel, RechargeOrderStatus, RechargeOrderSummary},
            user::{UserKind, UserStatus, UserSummary, WithdrawalMethodType},
            withdrawal::{WithdrawalOrderStatus, WithdrawalOrderSummary},
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
    use axum::http::{header, HeaderMap, Method};
    use std::collections::BTreeMap;

    #[test]
    /// 验证管理员登录审计优先使用 Cloudflare 真实 IP 并记录 User-Agent。
    fn admin_login_audit_context_prefers_cloudflare_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "198.51.100.8, 10.0.0.2".parse().unwrap());
        headers.insert("cf-connecting-ip", "203.0.113.9".parse().unwrap());
        headers.insert(header::USER_AGENT, "HongFu Admin Test".parse().unwrap());

        let context = admin_login_audit_context_from_headers(&headers);

        assert_eq!(context.client_ip, "203.0.113.9");
        assert_eq!(context.user_agent, "HongFu Admin Test");
    }

    #[test]
    /// 验证管理员登录审计能解析 Forwarded 请求头里的 IPv6 地址。
    fn admin_login_audit_ip_parser_handles_forwarded_ipv6() {
        let ip = first_admin_audit_ip("for=\"[2001:db8::1]:443\";proto=https");

        assert_eq!(ip.as_deref(), Some("2001:db8::1"));
    }

    #[test]
    /// 验证后台路由到权限范围的映射关系。
    fn required_scope_maps_admin_paths() {
        assert_eq!(required_scope_for_path("/dashboard"), None);
        assert_eq!(
            required_scope_for_path("/users"),
            Some(PermissionScope::Users)
        );
        assert_eq!(
            required_scope_for_path("/api/admin/users"),
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
            required_scope_for_path("/system-settings/cache/reload"),
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
        assert_eq!(
            required_scope_for_path("/rebate-statistics/U90001/records"),
            Some(PermissionScope::Rebates)
        );
    }

    #[test]
    /// 验证高风险后台接口会映射到细粒度操作权限点。
    fn required_permission_maps_sensitive_admin_paths() {
        assert_eq!(
            required_permission_for_request(&Method::DELETE, "/ledger-entries/clear"),
            Some("finance.ledger.clear")
        );
        assert_eq!(
            required_permission_for_request(&Method::POST, "/financial-adjustments"),
            Some("finance.adjust.create")
        );
        assert_eq!(
            required_permission_for_request(&Method::PATCH, "/users/U10001/password"),
            Some("user.password.reset")
        );
        assert_eq!(
            required_permission_for_request(&Method::PUT, "/draw-controls/au5"),
            Some("lottery.draw.control")
        );
        assert_eq!(
            required_permission_for_request(&Method::GET, "/api/admin/group-buy/plans"),
            Some("group.buy.read")
        );
        assert_eq!(
            required_permission_for_request(&Method::DELETE, "/api/admin/group-buy/plans/G-001"),
            Some("group.buy.clear")
        );
        assert_eq!(
            required_permission_for_request(
                &Method::DELETE,
                "/api/admin/system-settings/chat-hall/messages/clear",
            ),
            Some("system.chat.clear")
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
    /// 后台合买计划列表默认过滤机器人发起的计划，开关打开后才展示。
    fn group_buy_plan_filter_hides_robot_initiator_by_default() {
        assert!(!should_include_robot_initiated_group_buy_plan(
            false,
            ROBOT_GROUP_BUY_USER_ID
        ));
        assert!(should_include_robot_initiated_group_buy_plan(
            true,
            ROBOT_GROUP_BUY_USER_ID
        ));
        assert!(should_include_robot_initiated_group_buy_plan(
            false, "U10001"
        ));
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
                user_id: None,
                username: None,
            },
        );

        assert_eq!(page.items, vec!["G-003".to_string()]);
        assert_eq!(page.page, 2);
        assert_eq!(page.page_size, 2);
        assert_eq!(page.total_count, 3);
        assert_eq!(page.total_pages, 2);
    }

    #[test]
    /// 后台用户列表支持先按白名单字段排序，再返回请求页码对应的数据。
    fn user_list_sorting_runs_before_pagination() {
        let mut users = vec![
            test_user("U10001", "alice", 300),
            test_user("U10002", "bob", 100),
            test_user("U10003", "carol", 200),
        ];
        let query = UserListQuery {
            page: Some(2),
            page_size: Some(2),
            sort_by: Some("balanceMinor".to_string()),
            sort_direction: Some("desc".to_string()),
            status: None,
        };

        sort_users(&mut users, &query).expect("users can be sorted");
        let page = page_items(users, query.page_query());

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, "U10002");
        assert_eq!(page.page, 2);
        assert_eq!(page.page_size, 2);
        assert_eq!(page.total_count, 3);
    }

    #[test]
    /// 后台用户列表未传排序方向时默认按降序展示，符合运营优先看新用户的习惯。
    fn user_list_sort_direction_defaults_to_desc() {
        let mut users = vec![
            test_user("U10001", "alice", 300),
            test_user("U10002", "bob", 100),
            test_user("U10003", "carol", 200),
        ];
        let query = UserListQuery {
            page: Some(1),
            page_size: Some(2),
            sort_by: Some("id".to_string()),
            sort_direction: None,
            status: None,
        };

        sort_users(&mut users, &query).expect("users can be sorted with default direction");
        let page = page_items(users, query.page_query());

        assert_eq!(
            page.items
                .iter()
                .map(|user| user.id.as_str())
                .collect::<Vec<_>>(),
            vec!["U10003", "U10002"]
        );
    }

    #[test]
    /// 后台用户列表可以按锁定状态过滤，便于运营集中处理异常账号。
    fn user_list_status_filter_keeps_only_locked_users() {
        let mut users = vec![
            test_user("U10001", "alice", 300),
            test_user("U10002", "bob", 100),
            test_user("U10003", "carol", 200),
        ];
        users[1].status = UserStatus::Locked;

        filter_users_by_status(&mut users, Some(&UserStatus::Locked));

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].id, "U10002");
        assert_eq!(users[0].status, UserStatus::Locked);
    }

    #[test]
    /// 后台用户展示项会把上级代理 ID 解析成用户名，方便运营直接识别代理。
    fn admin_user_summary_includes_agent_username() {
        let agent = test_user("U90001", "agent_alpha", 0);
        let mut invitee = test_user("U10001", "demo_user", 0);
        invitee.agent_id = Some(agent.id.clone());
        let usernames = username_map_from_users(&[agent, invitee.clone()]);

        let summary = admin_user_summary_with_usernames(invitee, &usernames);

        assert_eq!(summary.user.agent_id.as_deref(), Some("U90001"));
        assert_eq!(summary.agent_username.as_deref(), Some("agent_alpha"));
    }

    #[test]
    /// 资金账户列表按用户编号倒序后再分页，最新用户优先展示。
    fn financial_accounts_sort_latest_user_before_pagination() {
        let mut accounts = vec![
            test_financial_account("U10001"),
            test_financial_account("U10003"),
            test_financial_account("U10002"),
        ];

        sort_financial_accounts_by_latest_user_desc(&mut accounts);
        let page = page_items(
            accounts,
            FinancePageQuery {
                include_robot_data: None,
                page: Some(1),
                page_size: Some(2),
                user_id: None,
                username: None,
            },
        );

        assert_eq!(
            page.items
                .iter()
                .map(|account| account.user_id.as_str())
                .collect::<Vec<_>>(),
            vec!["U10003", "U10002"]
        );
    }

    #[test]
    /// 财务列表按创建时间倒序后再分页，同一秒的数据按业务编号倒序保持稳定。
    fn finance_lists_sort_by_created_time_desc_before_pagination() {
        let mut ledger_entries = vec![
            test_ledger_entry("L000000000001", "2026-06-10 10:00:00"),
            test_ledger_entry("L000000000002", "2026-06-10 12:00:00"),
            test_ledger_entry("L000000000003", "2026-06-10 12:00:00"),
        ];
        sort_ledger_entries_by_time_desc(&mut ledger_entries);
        let ledger_page = page_items(
            ledger_entries,
            FinancePageQuery {
                include_robot_data: None,
                page: Some(1),
                page_size: Some(2),
                user_id: None,
                username: None,
            },
        );
        assert_eq!(
            ledger_page
                .items
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["L000000000003", "L000000000002"]
        );

        let user_filter = FinancePageQuery {
            include_robot_data: None,
            page: Some(1),
            page_size: Some(20),
            user_id: Some("U10001".to_string()),
            username: None,
        };
        assert!(should_match_user_filter(&user_filter, "U10001"));
        assert!(!should_match_user_filter(&user_filter, "U10002"));

        let mut recharge_orders = vec![
            test_recharge_order("R000000000001", "2026-06-10 09:00:00"),
            test_recharge_order("R000000000002", "2026-06-10 13:00:00"),
        ];
        sort_recharge_orders_by_time_desc(&mut recharge_orders);
        assert_eq!(recharge_orders[0].id, "R000000000002");

        let mut withdrawal_orders = vec![
            test_withdrawal_order("W000000000001", "2026-06-10 08:00:00"),
            test_withdrawal_order("W000000000002", "2026-06-10 14:00:00"),
        ];
        sort_withdrawal_orders_by_time_desc(&mut withdrawal_orders);
        assert_eq!(withdrawal_orders[0].id, "W000000000002");
    }

    #[test]
    /// 代理返利统计按返利入账和返利提现流水计算总额、待处理和可提现金额。
    fn agent_rebate_summary_counts_pending_and_withdrawable_amounts() {
        let mut agent = test_user("U90001", "agent_alpha", 0);
        agent.kind = UserKind::Agent;
        agent.invite_code = "ABCDEFGH".to_string();
        let mut invitee = test_user("U10001", "demo_user", 0);
        invitee.agent_id = Some(agent.id.clone());
        let users = vec![agent.clone(), invitee.clone()];
        let invite_records = vec![InviteRecord {
            created_at: "2026-06-12 10:00:00".to_string(),
            id: "INV-1".to_string(),
            invite_code: agent.invite_code.clone(),
            invitee_user_id: invitee.id.clone(),
            invitee_username: invitee.username.clone(),
            inviter_user_id: agent.id.clone(),
            inviter_username: agent.username.clone(),
            note: String::new(),
            rebate_enabled: true,
            status: InviteStatus::Active,
            updated_at: "2026-06-12 10:00:00".to_string(),
        }];
        let entries = vec![
            test_agent_rebate_entry(
                "L000000000010",
                LedgerEntryKind::RechargeRebateCredit,
                350,
                Some("recharge-rebate:R000000000001"),
                "2026-06-12 11:00:00",
            ),
            test_agent_rebate_entry(
                "L000000000011",
                LedgerEntryKind::AgentRebateWithdrawal,
                -120,
                None,
                "2026-06-12 12:00:00",
            ),
        ];
        let mut accounts = BTreeMap::new();
        accounts.insert(
            agent.id.clone(),
            FinancialAccountSummary {
                available_balance_minor: 200,
                frozen_balance_minor: 0,
                user_id: agent.id.clone(),
            },
        );
        let withdrawals = vec![test_withdrawal_order_for_user(
            "W000000000001",
            &invitee.id,
            &invitee.username,
            1_800,
            WithdrawalOrderStatus::Approved,
            "2026-06-12 12:30:00",
        )];
        let recharges = vec![
            test_recharge_order_for_user(
                "R000000000001",
                &invitee.id,
                &invitee.username,
                10_000,
                RechargeOrderStatus::Paid,
                "2026-06-12 10:55:00",
            ),
            test_recharge_order_for_user(
                "R000000000002",
                &invitee.id,
                &invitee.username,
                5_000,
                RechargeOrderStatus::Pending,
                "2026-06-12 10:58:00",
            ),
        ];

        let summary = agent_rebate_summary_from_data(
            &agent,
            &users,
            &invite_records,
            &entries,
            &recharges,
            &withdrawals,
            &accounts,
        )
        .expect("agent rebate summary can be calculated");

        assert_eq!(summary.direct_invitee_count, 1);
        assert_eq!(summary.direct_invitee_recharge_minor, 10_000);
        assert_eq!(summary.direct_invitee_withdrawal_minor, 1_800);
        assert_eq!(summary.total_rebate_minor, 350);
        assert_eq!(summary.withdrawn_rebate_minor, 120);
        assert_eq!(summary.pending_rebate_minor, 230);
        assert_eq!(summary.withdrawable_rebate_minor, 200);
        assert_eq!(
            summary.last_rebate_at.as_deref(),
            Some("2026-06-12 11:00:00")
        );
    }

    #[test]
    /// 代理返利明细可以从返利流水和充值订单中还原下级用户、充值单和充值金额。
    fn agent_rebate_records_include_invitee_and_recharge_order() {
        let mut agent = test_user("U90001", "agent_alpha", 0);
        agent.kind = UserKind::Agent;
        let invitee = test_user("U10001", "demo_user", 0);
        let users = vec![agent.clone(), invitee.clone()];
        let entries = vec![test_agent_rebate_entry(
            "L000000000010",
            LedgerEntryKind::RechargeRebateCredit,
            350,
            Some("recharge-rebate:R000000000001"),
            "2026-06-12 11:00:00",
        )];
        let recharges = vec![RechargeOrderSummary {
            amount_minor: 10_000,
            channel: RechargeChannel::CustomerService,
            created_at: "2026-06-12 10:59:00".to_string(),
            id: "R000000000001".to_string(),
            paid_at: Some("2026-06-12 11:00:00".to_string()),
            pay_type: None,
            payment_url: None,
            provider_trade_no: None,
            status: RechargeOrderStatus::Paid,
            support_conversation_id: None,
            user_id: invitee.id.clone(),
            username: invitee.username.clone(),
        }];
        let withdrawals = vec![test_withdrawal_order_for_user(
            "W000000000010",
            &invitee.id,
            &invitee.username,
            2_500,
            WithdrawalOrderStatus::Approved,
            "2026-06-12 12:10:00",
        )];
        let mut records = agent_rebate_records_from_data(
            Some(&agent.id),
            &users,
            &entries,
            &recharges,
            &withdrawals,
        );
        sort_agent_rebate_records_by_time_desc(&mut records);

        assert_eq!(records.len(), 1);
        assert_eq!(records[0].agent_username, "agent_alpha");
        assert_eq!(records[0].invitee_user_id.as_deref(), Some("U10001"));
        assert_eq!(
            records[0].recharge_order_id.as_deref(),
            Some("R000000000001")
        );
        assert_eq!(records[0].recharge_amount_minor, Some(10_000));
        assert_eq!(records[0].invitee_total_recharge_minor, 10_000);
        assert_eq!(records[0].rebate_amount_minor, 350);
        assert_eq!(records[0].invitee_total_withdrawal_minor, 2_500);
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

    #[tokio::test]
    /// 指定期号已经开奖时，后台保存控奖会自动关闭控制，避免控奖配置残留到已结束期号。
    async fn draw_control_issue_target_disables_when_issue_drawn() {
        let state = test_state();
        let lottery = state.lotteries.get("ssc60").await.expect("lottery exists");
        let issue = state
            .draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "202606052201".to_string(),
                    scheduled_at: "2026-06-05 22:01:00".to_string(),
                    sale_closed_at: "2026-06-05 22:00:30".to_string(),
                },
            )
            .await
            .expect("issue can be created");
        state
            .draws
            .draw(
                &issue.id,
                DrawIssueResultRequest {
                    draw_number: Some("1,2,3,4,5".to_string()),
                },
            )
            .await
            .expect("issue can be drawn");
        let mut payload = SaveLotteryDrawControlRequest {
            enabled: true,
            draw_number: Some("5,4,3,2,1".to_string()),
            target_scope: DrawControlTargetScope::Issue,
            target_issue: Some(issue.issue.clone()),
            target_order_id: None,
        };

        normalize_admin_draw_control_target(&state, &lottery, &mut payload)
            .await
            .expect("drawn issue target can be normalized");

        assert!(!payload.enabled);
        assert_eq!(payload.target_scope, DrawControlTargetScope::Lottery);
        assert_eq!(payload.target_issue, None);
        assert_eq!(payload.target_order_id, None);
    }

    #[tokio::test]
    /// 指定期号已经取消时，后台保存控奖也会自动关闭控制，避免控奖配置残留到无效期号。
    async fn draw_control_issue_target_disables_when_issue_cancelled() {
        let state = test_state();
        let lottery = state.lotteries.get("ssc60").await.expect("lottery exists");
        let issue = state
            .draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "202606052202".to_string(),
                    scheduled_at: "2026-06-05 22:02:00".to_string(),
                    sale_closed_at: "2026-06-05 22:01:30".to_string(),
                },
            )
            .await
            .expect("issue can be created");
        state
            .draws
            .cancel(&issue.id)
            .await
            .expect("issue can be cancelled");
        let mut payload = SaveLotteryDrawControlRequest {
            enabled: true,
            draw_number: Some("5,4,3,2,1".to_string()),
            target_scope: DrawControlTargetScope::Issue,
            target_issue: Some(issue.issue.clone()),
            target_order_id: None,
        };

        normalize_admin_draw_control_target(&state, &lottery, &mut payload)
            .await
            .expect("cancelled issue target can be normalized");

        assert!(!payload.enabled);
        assert_eq!(payload.target_scope, DrawControlTargetScope::Lottery);
        assert_eq!(payload.target_issue, None);
        assert_eq!(payload.target_order_id, None);
    }
    /// 构造路由测试所需的应用状态。
    fn test_state() -> AppState {
        AppState {
            access: AccessRepository::memory_seeded(),
            advertisements: AdvertisementRepository::memory(),
            agent_applications:
                crate::services::agent_application::AgentApplicationRepository::memory(),
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
                sale_close_lead_seconds: 1,
            }),
            support: SupportRepository::memory_seeded(),
            withdrawals: WithdrawalRepository::memory(),
        }
    }
    /// 构造测试用用户摘要。
    fn test_user(id: &str, username: &str, balance_minor: i64) -> UserSummary {
        UserSummary {
            agent_id: None,
            avatar_url: String::new(),
            balance_minor,
            contact_qq: String::new(),
            email: None,
            id: id.to_string(),
            invite_code: format!("{id}CODE"),
            kind: UserKind::Regular,
            registration_location: crate::domain::user::UserRegistrationLocation::default(),
            status: UserStatus::Active,
            username: username.to_string(),
            created_at: "2026-06-05 10:00:00".to_string(),
        }
    }
    /// 构造测试用资金流水。
    fn test_ledger_entry(id: &str, created_at: &str) -> LedgerEntry {
        LedgerEntry {
            amount_minor: 100,
            balance_after_minor: 1000,
            created_at: created_at.to_string(),
            description: "测试流水".to_string(),
            id: id.to_string(),
            kind: LedgerEntryKind::ManualAdjustment,
            reference_id: None,
            user_id: "U10001".to_string(),
        }
    }
    /// 构造测试用代理返利流水。
    fn test_agent_rebate_entry(
        id: &str,
        kind: LedgerEntryKind,
        amount_minor: i64,
        reference_id: Option<&str>,
        created_at: &str,
    ) -> LedgerEntry {
        LedgerEntry {
            amount_minor,
            balance_after_minor: 1000,
            created_at: created_at.to_string(),
            description: "下级用户充值返利：订单 R000000000001，下级 U10001".to_string(),
            id: id.to_string(),
            kind,
            reference_id: reference_id.map(ToString::to_string),
            user_id: "U90001".to_string(),
        }
    }
    /// 构造测试用资金账户摘要。
    fn test_financial_account(user_id: &str) -> AdminFinancialAccountSummary {
        AdminFinancialAccountSummary {
            available_balance_minor: 1000,
            frozen_balance_minor: 0,
            user_id: user_id.to_string(),
            username: Some(format!("user_{user_id}")),
        }
    }
    /// 构造测试用充值订单。
    fn test_recharge_order(id: &str, created_at: &str) -> RechargeOrderSummary {
        test_recharge_order_for_user(
            id,
            "U10001",
            "alice",
            1000,
            RechargeOrderStatus::Pending,
            created_at,
        )
    }
    /// 构造指定用户的测试充值订单。
    fn test_recharge_order_for_user(
        id: &str,
        user_id: &str,
        username: &str,
        amount_minor: i64,
        status: RechargeOrderStatus,
        created_at: &str,
    ) -> RechargeOrderSummary {
        RechargeOrderSummary {
            amount_minor,
            channel: RechargeChannel::CustomerService,
            created_at: created_at.to_string(),
            id: id.to_string(),
            paid_at: None,
            pay_type: None,
            payment_url: None,
            provider_trade_no: None,
            status,
            support_conversation_id: None,
            user_id: user_id.to_string(),
            username: username.to_string(),
        }
    }
    /// 构造测试用提现订单。
    fn test_withdrawal_order(id: &str, created_at: &str) -> WithdrawalOrderSummary {
        test_withdrawal_order_for_user(
            id,
            "U10001",
            "alice",
            1000,
            WithdrawalOrderStatus::Pending,
            created_at,
        )
    }
    /// 构造指定用户的测试提现订单。
    fn test_withdrawal_order_for_user(
        id: &str,
        user_id: &str,
        username: &str,
        amount_minor: i64,
        status: WithdrawalOrderStatus,
        created_at: &str,
    ) -> WithdrawalOrderSummary {
        WithdrawalOrderSummary {
            account_holder: username.to_string(),
            account_number: "13800000000".to_string(),
            amount_minor,
            bank_name: None,
            created_at: created_at.to_string(),
            id: id.to_string(),
            method_id: "WM10001".to_string(),
            method_type: WithdrawalMethodType::Alipay,
            reviewed_at: None,
            status,
            user_id: user_id.to_string(),
            username: username.to_string(),
        }
    }
}
