//! 管理后台 API 路由总控，汇总和注册所有后台接口

use axum::{
    extract::{Multipart, Path, Query, Request, State},
    http::header::AUTHORIZATION,
    middleware::{self, Next},
    response::Response,
    routing::{get, patch, post, put},
    Extension, Json, Router,
};
use chrono::Local;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

use crate::{
    app::AppState,
    domain::{
        advertisement::{AdvertisementSummary, SaveAdvertisementRequest},
        auth::{AdminAuthSession, AdminLoginRequest, AdminLogoutResponse, CurrentAdminProfile},
        draw::{
            CreateDrawIssueRequest, DrawAutomationRun, DrawAutomationRunRequest, DrawIssue,
            DrawIssueGenerationPreview, DrawIssuePage, DrawIssueResultRequest, DrawIssueStatus,
            GenerateDrawIssueRequest, GenerateDrawIssuesRequest, LotteryDrawControl,
            SaveLotteryDrawControlRequest,
        },
        finance::{
            AdminFinancialAccountSummary, FinanceOverview, FinancePage, FinancialAccountSummary,
            LedgerEntry, ManualBalanceAdjustmentRequest,
        },
        group_buy::{
            AddGroupBuyParticipantRequest, CreateGroupBuyPlanRequest, GroupBuyPlan,
            GroupBuyPlanSummary, UpdateGroupBuyPlanRequest,
        },
        invite::{CreateInviteRecordRequest, InviteRecord, UpdateInviteRecordRequest},
        lottery::{
            DrawMode, DrawSource, LotteryCategoryConfig, LotteryKind, SaveDrawSourceRequest,
        },
        order::{CreateOrderRequest, OrderDetail},
        permission::{AdminRole, PermissionScope, SystemSetting, UpdateSystemSettingRequest},
        play::{PlayRuleEvaluateRequest, PlayRuleEvaluation, PlayRuleSummary},
        rebate::{InvitePolicySummary, InvitePolicyUpdateRequest},
        recharge::{ConfirmRechargeOrderRequest, RechargeOrderSummary},
        robot::{RobotConfigSummary, RobotStatusRequest},
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
        order::validate_draw_issue_accepts_order,
        play_rules::{evaluate_play_rule, play_rule_summaries},
        scheduler::DrawSchedulerConfig,
        scheduler::DrawSchedulerStatus,
    },
};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const IMAGE_BED_UPLOAD_URL_SETTING: &str = "image_bed_upload_url";
const IMAGE_BED_AUTHORIZATION_TOKEN_SETTING: &str = "image_bed_authorization_token";
const IMAGE_BED_UPLOAD_FIELD_SETTING: &str = "image_bed_upload_field";
const IMAGE_BED_UPLOAD_FIELD_DEFAULT: &str = "file";
// 图床返回中用于提取可直接展示图片链接的 JSON 字段路径（支持 `a.b.c`）。
const IMAGE_BED_RESULT_URL_FIELD_SETTING: &str = "image_bed_result_url_field";
// 默认优先读取 `links.download`，与目前 `moonight` 图床响应保持一致。
const IMAGE_BED_RESULT_URL_FIELD_DEFAULT: &str = "links.download";

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
        .merge(protected_routes)
}

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

async fn login_admin(
    State(state): State<AppState>,
    Json(payload): Json<AdminLoginRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminAuthSession>>> {
    let session = state.access.login(payload).await?;

    Ok(Json(ApiEnvelope::success(session)))
}

async fn get_current_admin(
    Extension(session): Extension<AdminAuthSession>,
) -> ApiResult<Json<ApiEnvelope<CurrentAdminProfile>>> {
    Ok(Json(ApiEnvelope::success(session.profile())))
}

async fn logout_admin(
    State(state): State<AppState>,
    Extension(session): Extension<AdminAuthSession>,
) -> ApiResult<Json<ApiEnvelope<AdminLogoutResponse>>> {
    state.access.logout(&session.token).await?;

    Ok(Json(ApiEnvelope::success(AdminLogoutResponse {
        logged_out: true,
    })))
}

async fn run_draw_automation_request(
    State(state): State<AppState>,
    Json(payload): Json<DrawAutomationRunRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawAutomationRun>>> {
    let run = run_draw_automation(
        &state.draws,
        &state.lotteries,
        &state.orders,
        &state.finance,
        payload,
    )
    .await?;

    Ok(Json(ApiEnvelope::success(run)))
}

async fn get_draw_scheduler_status(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<DrawSchedulerStatus>>> {
    let status = state.scheduler.status()?;

    Ok(Json(ApiEnvelope::success(status)))
}

async fn update_draw_scheduler_config(
    State(state): State<AppState>,
    Json(payload): Json<DrawSchedulerConfig>,
) -> ApiResult<Json<ApiEnvelope<DrawSchedulerStatus>>> {
    let status = state.scheduler.update_config(payload).await?;

    Ok(Json(ApiEnvelope::success(status)))
}

async fn list_draw_sources(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawSource>>>> {
    let sources = state.draws.draw_sources().await?;

    Ok(Json(ApiEnvelope::success(sources)))
}

async fn create_draw_source(
    State(state): State<AppState>,
    Json(payload): Json<SaveDrawSourceRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawSource>>> {
    let lotteries = state.lotteries.list().await?;
    let source = state.draws.create_draw_source(payload, &lotteries).await?;

    Ok(Json(ApiEnvelope::success(source)))
}

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

async fn delete_draw_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawSource>>> {
    let source = state.draws.delete_draw_source(&id).await?;

    Ok(Json(ApiEnvelope::success(source)))
}

async fn list_lottery_draw_controls(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LotteryDrawControl>>>> {
    let lotteries = state.lotteries.list().await?;
    let controls = state.draws.list_draw_controls(&lotteries).await?;

    Ok(Json(ApiEnvelope::success(controls)))
}

async fn get_lottery_draw_control(
    State(state): State<AppState>,
    Path(lottery_id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryDrawControl>>> {
    let lottery = state.lotteries.get(&lottery_id).await?;
    let control = state.draws.get_draw_control(&lottery).await?;

    Ok(Json(ApiEnvelope::success(control)))
}

async fn save_lottery_draw_control(
    State(state): State<AppState>,
    Path(lottery_id): Path<String>,
    Json(payload): Json<SaveLotteryDrawControlRequest>,
) -> ApiResult<Json<ApiEnvelope<LotteryDrawControl>>> {
    let lottery = state.lotteries.get(&lottery_id).await?;
    let control = state.draws.save_draw_control(&lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(control)))
}

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
struct DrawIssueListQuery {
    lottery_id: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FinancePageQuery {
    page: Option<usize>,
    page_size: Option<usize>,
}

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

async fn get_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.get(&id).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn create_draw_issue(
    State(state): State<AppState>,
    Json(payload): Json<CreateDrawIssueRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issue = state.draws.create(&lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn generate_next_draw_issue_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssueRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issue = generate_next_draw_issue(&state.draws, &lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn preview_draw_issue_generation_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssuesRequest>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawIssueGenerationPreview>>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let plans = preview_draw_issue_generation(&state.draws, &lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(plans)))
}

async fn generate_draw_issue_batch_request(
    State(state): State<AppState>,
    Json(payload): Json<GenerateDrawIssuesRequest>,
) -> ApiResult<Json<ApiEnvelope<Vec<DrawIssue>>>> {
    let lottery = state.lotteries.get(&payload.lottery_id).await?;
    let issues = generate_draw_issue_batch(&state.draws, &lottery, payload).await?;

    Ok(Json(ApiEnvelope::success(issues)))
}

async fn close_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.close(&id).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn draw_issue_result(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DrawIssueResultRequest>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.draw(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn cancel_draw_issue(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<DrawIssue>>> {
    let issue = state.draws.cancel(&id).await?;

    Ok(Json(ApiEnvelope::success(issue)))
}

async fn list_settlements(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<SettlementRun>>>> {
    let settlements = state.orders.settlement_runs().await?;

    Ok(Json(ApiEnvelope::success(settlements)))
}

async fn get_settlement(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SettlementRun>>> {
    let settlement = state.orders.get_settlement(&id).await?;

    Ok(Json(ApiEnvelope::success(settlement)))
}

async fn settle_draw_issue_orders(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SettlementRun>>> {
    let draw_issue = state.draws.get(&id).await?;
    let settlement = state.orders.settle_draw_issue(&draw_issue).await?;
    state.finance.credit_settlement(&settlement).await?;

    Ok(Json(ApiEnvelope::success(settlement)))
}

async fn list_play_rules() -> ApiResult<Json<ApiEnvelope<Vec<PlayRuleSummary>>>> {
    Ok(Json(ApiEnvelope::success(play_rule_summaries())))
}

async fn evaluate_play_rule_request(
    Json(payload): Json<PlayRuleEvaluateRequest>,
) -> ApiResult<Json<ApiEnvelope<PlayRuleEvaluation>>> {
    Ok(Json(ApiEnvelope::success(evaluate_play_rule(payload)?)))
}

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

async fn list_group_buy_plans(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<GroupBuyPlanSummary>>>> {
    let plans = state.group_buys.list().await?;

    Ok(Json(ApiEnvelope::success(plans)))
}

async fn get_group_buy_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let plan = state.group_buys.get(&id).await?;

    Ok(Json(ApiEnvelope::success(plan)))
}

async fn create_group_buy_plan(
    State(state): State<AppState>,
    Json(payload): Json<CreateGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let lotteries = state.lotteries.list().await?;
    let access = state.access.snapshot().await?;
    let plan = state
        .group_buys
        .create(payload, &lotteries, &access.users)
        .await?;

    Ok(Json(ApiEnvelope::success(plan)))
}

async fn update_group_buy_plan(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateGroupBuyPlanRequest>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let plan = state.group_buys.update(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(plan)))
}

async fn add_group_buy_participant(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AddGroupBuyParticipantRequest>,
) -> ApiResult<Json<ApiEnvelope<GroupBuyPlan>>> {
    let access = state.access.snapshot().await?;
    let plan = state
        .group_buys
        .add_participant(&id, payload, &access.users)
        .await?;

    Ok(Json(ApiEnvelope::success(plan)))
}

async fn list_invitations(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<InviteRecord>>>> {
    let invitations = state.invites.list().await?;

    Ok(Json(ApiEnvelope::success(invitations)))
}

async fn get_invitation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<InviteRecord>>> {
    let invitation = state.invites.get(&id).await?;

    Ok(Json(ApiEnvelope::success(invitation)))
}

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

async fn update_invitation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateInviteRecordRequest>,
) -> ApiResult<Json<ApiEnvelope<InviteRecord>>> {
    let invitation = state.invites.update(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(invitation)))
}

async fn list_support_conversations(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<SupportConversation>>>> {
    let conversations = state.support.list().await?;

    Ok(Json(ApiEnvelope::success(conversations)))
}

async fn get_support_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let conversation = state.support.get(&id).await?;

    Ok(Json(ApiEnvelope::success(conversation)))
}

async fn create_support_conversation(
    State(state): State<AppState>,
    Json(payload): Json<CreateSupportConversationRequest>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let access = state.access.snapshot().await?;
    let conversation = state.support.create(payload, &access.users).await?;

    Ok(Json(ApiEnvelope::success(conversation)))
}

async fn update_support_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateSupportConversationRequest>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let access = state.access.snapshot().await?;
    let conversation = state.support.update(&id, payload, &access.admins).await?;

    Ok(Json(ApiEnvelope::success(conversation)))
}

async fn reply_support_conversation(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SupportReplyRequest>,
) -> ApiResult<Json<ApiEnvelope<SupportConversation>>> {
    let access = state.access.snapshot().await?;
    let conversation = state.support.reply(&id, payload, &access.admins).await?;

    Ok(Json(ApiEnvelope::success(conversation)))
}

async fn list_users(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<UserSummary>>>> {
    let users = users_with_financial_balances(&state).await?;

    Ok(Json(ApiEnvelope::success(users)))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = user_with_financial_balance(&state, &id).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<UserSummary>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.create_user(payload).await?;
    let account = state.finance.account_or_create(&user.id).await?;
    let user = user_with_account_balance(user, Some(&account));

    Ok(Json(ApiEnvelope::success(user)))
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UserSummary>,
) -> ApiResult<Json<ApiEnvelope<UserSummary>>> {
    let user = state.access.update_user(&id, payload).await?;
    let user = user_with_financial_balance_from_summary(&state, user).await?;

    Ok(Json(ApiEnvelope::success(user)))
}

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

async fn list_admins(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<AdminSummary>>>> {
    let admins = state.access.admins().await?;

    Ok(Json(ApiEnvelope::success(admins)))
}

async fn get_admin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.get_admin(&id).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn create_admin(
    State(state): State<AppState>,
    Json(payload): Json<AdminSaveRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.create_admin(payload).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn update_admin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminSaveRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.update_admin(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn reset_admin_password(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminPasswordResetRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.reset_admin_password(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn set_admin_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<AdminSummary>>> {
    let admin = state.access.set_admin_status(&id, payload.status).await?;

    Ok(Json(ApiEnvelope::success(admin)))
}

async fn list_roles(State(state): State<AppState>) -> ApiResult<Json<ApiEnvelope<Vec<AdminRole>>>> {
    let roles = state.access.roles().await?;

    Ok(Json(ApiEnvelope::success(roles)))
}

async fn get_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.get_role(&id).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

async fn create_role(
    State(state): State<AppState>,
    Json(payload): Json<AdminRole>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.create_role(payload).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

async fn update_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AdminRole>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.update_role(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

async fn delete_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdminRole>>> {
    let role = state.access.delete_role(&id).await?;

    Ok(Json(ApiEnvelope::success(role)))
}

async fn list_system_settings(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<SystemSetting>>>> {
    let settings = state.access.settings().await?;

    Ok(Json(ApiEnvelope::success(settings)))
}

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
    mut payload: Multipart,
) -> ApiResult<Json<ApiEnvelope<Value>>> {
    let upload_url = state
        .access
        .setting_value(IMAGE_BED_UPLOAD_URL_SETTING)
        .await?
        .trim()
        .to_string();
    if upload_url.is_empty() {
        return Err(ApiError::BadRequest("图床上传接口地址未配置".to_string()));
    }

    let authorization_token = state
        .access
        .setting_value(IMAGE_BED_AUTHORIZATION_TOKEN_SETTING)
        .await?
        .trim()
        .to_string();
    if authorization_token.is_empty() {
        return Err(ApiError::BadRequest("图床上传 Token 未配置".to_string()));
    }

    let upload_field = state
        .access
        .setting_value_optional(IMAGE_BED_UPLOAD_FIELD_SETTING)
        .await?
        .unwrap_or_else(|| IMAGE_BED_UPLOAD_FIELD_DEFAULT.to_string())
        .trim()
        .to_string();

    let mut upload_part = None;
    while let Some(field) = payload
        .next_field()
        .await
        .map_err(|_| ApiError::BadRequest("上传内容解析失败".to_string()))?
    {
        if field.name() == Some(upload_field.as_str()) {
            let file_name = field.file_name().unwrap_or("upload.bin").to_string();
            let content_type: Option<String> =
                field.content_type().map(std::string::ToString::to_string);
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
    }

    let Some(part) = upload_part else {
        return Err(ApiError::BadRequest("未检测到图片文件字段".to_string()));
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
    // 将图床响应解析为 JSON，支持原始文本回退，避免解析失败直接失败。
    let response_json = serde_json::from_str::<Value>(&response_body)
        .unwrap_or_else(|_| Value::String(response_body));
    // 按配置读取返回字段路径；为空时兼容返回原始对象，适配未来图床返回结构变更。
    let result_url_field = state
        .access
        .setting_value_optional(IMAGE_BED_RESULT_URL_FIELD_SETTING)
        .await?
        .unwrap_or_else(|| IMAGE_BED_RESULT_URL_FIELD_DEFAULT.to_string())
        .trim()
        .to_string();

    let output = if result_url_field.is_empty() {
        response_json
    } else {
        extract_image_bed_result_field(&response_json, &result_url_field)?
    };

    Ok(Json(ApiEnvelope::success(output)))
}

async fn list_advertisements(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<AdvertisementSummary>>>> {
    let advertisements = state.advertisements.list().await?;

    Ok(Json(ApiEnvelope::success(advertisements)))
}

async fn get_advertisement(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdvertisementSummary>>> {
    let advertisement = state.advertisements.get(&id).await?;

    Ok(Json(ApiEnvelope::success(advertisement)))
}

async fn create_advertisement(
    State(state): State<AppState>,
    Json(payload): Json<SaveAdvertisementRequest>,
) -> ApiResult<Json<ApiEnvelope<AdvertisementSummary>>> {
    let advertisement = state.advertisements.create(payload).await?;

    Ok(Json(ApiEnvelope::success(advertisement)))
}

async fn update_advertisement(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SaveAdvertisementRequest>,
) -> ApiResult<Json<ApiEnvelope<AdvertisementSummary>>> {
    let advertisement = state.advertisements.update(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(advertisement)))
}

async fn delete_advertisement(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<AdvertisementSummary>>> {
    let advertisement = state.advertisements.delete(&id).await?;

    Ok(Json(ApiEnvelope::success(advertisement)))
}

fn extract_image_bed_result_field(response: &Value, field_path: &str) -> ApiResult<Value> {
    // 根据配置路径从上游响应里找图片链接，返回缺失时给出可读错误。
    let Some(value) = resolve_json_path(response, field_path) else {
        return Err(ApiError::BadRequest(format!(
            "图床返回结构中未找到图片链接字段 `{field_path}`"
        )));
    };

    if let Some(url) = value.as_str().filter(|item| !item.trim().is_empty()) {
        Ok(Value::String(url.to_string()))
    } else {
        Err(ApiError::BadRequest(format!(
            "图床返回字段 `{field_path}` 不是有效图片链接文本：{value}"
        )))
    }
}

fn resolve_json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    // 支持 `.` 分隔的嵌套对象路径，如 `links.download`。
    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            return None;
        }
        match current {
            Value::Object(map) => {
                current = map.get(segment)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

async fn get_registration_config(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<RegistrationConfig>>> {
    let registration = state.access.registration().await?;

    Ok(Json(ApiEnvelope::success(registration)))
}

async fn update_registration_config(
    State(state): State<AppState>,
    Json(payload): Json<RegistrationConfig>,
) -> ApiResult<Json<ApiEnvelope<RegistrationConfig>>> {
    let registration = state.access.update_registration(payload).await?;

    Ok(Json(ApiEnvelope::success(registration)))
}

async fn get_invite_policy(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<InvitePolicySummary>>> {
    let policy = state.rebates.get().await?;

    Ok(Json(ApiEnvelope::success(policy)))
}

async fn update_invite_policy(
    State(state): State<AppState>,
    Json(payload): Json<InvitePolicyUpdateRequest>,
) -> ApiResult<Json<ApiEnvelope<InvitePolicySummary>>> {
    let policy = state.rebates.update(payload).await?;

    Ok(Json(ApiEnvelope::success(policy)))
}

async fn list_robots(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<RobotConfigSummary>>>> {
    let robots = state.robots.list().await?;

    Ok(Json(ApiEnvelope::success(robots)))
}

async fn get_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.get(&id).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn create_robot(
    State(state): State<AppState>,
    Json(payload): Json<RobotConfigSummary>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let lotteries = state.lotteries.list().await?;
    let robot = state.robots.create(payload, &lotteries).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn update_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RobotConfigSummary>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let lotteries = state.lotteries.list().await?;
    let robot = state.robots.update(&id, payload, &lotteries).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn delete_robot(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.delete(&id).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn set_robot_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<RobotStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<RobotConfigSummary>>> {
    let robot = state.robots.set_status(&id, payload.status).await?;

    Ok(Json(ApiEnvelope::success(robot)))
}

async fn get_finance_overview(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<FinanceOverview>>> {
    let overview = state.finance.overview().await?;

    Ok(Json(ApiEnvelope::success(overview)))
}

async fn list_financial_accounts(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<AdminFinancialAccountSummary>>>> {
    let accounts = state.finance.accounts().await?;
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

async fn list_ledger_entries(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<LedgerEntry>>>> {
    let entries = state.finance.ledger_entries().await?;

    Ok(Json(ApiEnvelope::success(page_items(entries, query))))
}

async fn list_recharge_orders(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<RechargeOrderSummary>>>> {
    let orders = state.recharges.list().await?;

    Ok(Json(ApiEnvelope::success(page_items(orders, query))))
}

async fn list_withdrawal_orders(
    State(state): State<AppState>,
    Query(query): Query<FinancePageQuery>,
) -> ApiResult<Json<ApiEnvelope<FinancePage<WithdrawalOrderSummary>>>> {
    let orders = state.withdrawals.list().await?;

    Ok(Json(ApiEnvelope::success(page_items(orders, query))))
}

async fn confirm_recharge_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ConfirmRechargeOrderRequest>,
) -> ApiResult<Json<ApiEnvelope<RechargeOrderSummary>>> {
    let order = state
        .recharges
        .confirm_customer_service_order(&id, payload, &state.finance)
        .await?;

    Ok(Json(ApiEnvelope::success(order)))
}

async fn approve_withdrawal_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<WithdrawalOrderSummary>>> {
    let order = state.withdrawals.approve_order(&id, &state.finance).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

async fn reject_withdrawal_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<WithdrawalOrderSummary>>> {
    let order = state.withdrawals.reject_order(&id, &state.finance).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

async fn manual_balance_adjustment(
    State(state): State<AppState>,
    Json(payload): Json<ManualBalanceAdjustmentRequest>,
) -> ApiResult<Json<ApiEnvelope<LedgerEntry>>> {
    let entry = state.finance.manual_adjust(payload).await?;

    Ok(Json(ApiEnvelope::success(entry)))
}

async fn list_orders(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<OrderDetail>>>> {
    let orders = state.orders.list().await?;

    Ok(Json(ApiEnvelope::success(orders)))
}

async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<OrderDetail>>> {
    let order = state.orders.get(&id).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

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
    let quote = state.orders.quote(&lottery, &payload).await?;
    state
        .finance
        .ensure_available(&payload.user_id, quote.amount_minor)
        .await?;
    let order = state.orders.create(&lottery, payload).await?;
    if let Err(error) = state.finance.debit_order(&order).await {
        if let Err(rollback_error) = state.orders.remove_unfunded(&order.id).await {
            tracing::error!(
                order_id = %order.id,
                error = %rollback_error.log_message(),
                "扣款失败后移除未入账订单失败"
            );
        }
        return Err(error);
    }

    Ok(Json(ApiEnvelope::success(order)))
}

async fn cancel_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<OrderDetail>>> {
    let existing = state.orders.get(&id).await?;
    state.finance.ensure_order_can_refund(&existing).await?;
    let order = state.orders.cancel(&id).await?;
    state.finance.refund_order(&order).await?;

    Ok(Json(ApiEnvelope::success(order)))
}

async fn list_lotteries(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LotteryKind>>>> {
    let lotteries = state.lotteries.list().await?;

    Ok(Json(ApiEnvelope::success(lotteries)))
}

async fn get_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.get(&id).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn create_lottery(
    State(state): State<AppState>,
    Json(payload): Json<LotteryKind>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.create(payload).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn update_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<LotteryKind>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.update(&id, payload).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn delete_lottery(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let lottery = state.lotteries.delete(&id).await?;

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn set_lottery_sale(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<SaleStatusRequest>,
) -> ApiResult<Json<ApiEnvelope<LotteryKind>>> {
    let before = state.lotteries.get(&id).await?;
    let need_align =
        before.draw_mode == DrawMode::Api && !before.sale_enabled && payload.sale_enabled;

    let lottery = state
        .lotteries
        .set_sale_enabled(&id, payload.sale_enabled)
        .await?;

    if need_align {
        if let Err(error) = align_api_draw_issue_plan_after_sale_on(&state, &lottery).await {
            tracing::warn!(
                lottery_id = %lottery.id,
                error = %error.log_message(),
                "开售后补齐期号失败，已保留销售状态切换结果"
            );
        }
    }

    Ok(Json(ApiEnvelope::success(lottery)))
}

async fn list_lottery_categories(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<LotteryCategoryConfig>>>> {
    let categories = state.lotteries.categories().await?;

    Ok(Json(ApiEnvelope::success(categories)))
}

async fn create_lottery_category(
    State(state): State<AppState>,
    Json(payload): Json<LotteryCategoryConfig>,
) -> ApiResult<Json<ApiEnvelope<LotteryCategoryConfig>>> {
    let category = state.lotteries.create_category(payload).await?;

    Ok(Json(ApiEnvelope::success(category)))
}

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

async fn delete_lottery_category(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> ApiResult<Json<ApiEnvelope<LotteryCategoryConfig>>> {
    let category = state.lotteries.delete_category(&code).await?;

    Ok(Json(ApiEnvelope::success(category)))
}

async fn align_api_draw_issue_plan_after_sale_on(
    state: &AppState,
    lottery: &LotteryKind,
) -> ApiResult<()> {
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
        return Ok(());
    }

    let count = config.future_issue_count - existing_future_count;
    let _ = generate_draw_issue_batch(
        &state.draws,
        lottery,
        GenerateDrawIssuesRequest {
            lottery_id: lottery.id.clone(),
            now,
            count,
            sale_close_lead_seconds: Some(config.sale_close_lead_seconds),
        },
    )
    .await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaleStatusRequest {
    sale_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::required_scope_for_path;
    use crate::domain::permission::PermissionScope;

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
}
