//! 后台看板聚合服务，输出模块状态与指标摘要

use serde::Serialize;

use crate::domain::{
    finance::{FinanceOverview, FinancialAccountSummary},
    group_buy::GroupBuyPlanSummary,
    lottery::{DrawSource, LotteryKind},
    order::OrderSummary,
    permission::{AdminRole, PermissionScope, SystemSetting},
    rebate::{InvitePolicySummary, RebateMode},
    robot::RobotConfigSummary,
    user::{AdminSummary, RegistrationConfig, UserSummary},
};

use super::access::AccessSnapshot;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台首页仪表盘聚合数据。
pub struct DashboardSummary {
    /// metrics字段。
    pub metrics: Vec<Metric>,
    /// 模块分组字段。
    pub module_groups: Vec<ModuleGroup>,
    /// 可选彩种列表。
    pub lotteries: Vec<LotteryKind>,
    /// 开奖来源字段。
    pub draw_sources: Vec<DrawSource>,
    /// 最近订单字段。
    pub recent_orders: Vec<OrderSummary>,
    /// 合买合买plans字段。
    pub group_buy_plans: Vec<GroupBuyPlanSummary>,
    /// 资金字段。
    pub finance: FinanceOverview,
    /// financial账户字段。
    pub financial_accounts: Vec<FinancialAccountSummary>,
    /// robots字段。
    pub robots: Vec<RobotConfigSummary>,
    /// 用户摘要列表。
    pub users: Vec<UserSummary>,
    /// 管理员摘要列表。
    pub admins: Vec<AdminSummary>,
    /// 管理员角色列表。
    pub roles: Vec<AdminRole>,
    /// 移动端或模块配置集合。
    pub settings: Vec<SystemSetting>,
    /// 用户注册开关和限制配置。
    pub registration: RegistrationConfig,
    /// 邀请策略字段。
    pub invite_policy: InvitePolicySummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台首页统计指标卡片。
pub struct Metric {
    /// 配置键或选项键。
    pub key: String,
    /// 前端展示文案。
    pub label: String,
    /// 配置值或选项值。
    pub value: String,
    /// trend字段。
    pub trend: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台首页功能模块分组。
pub struct ModuleGroup {
    /// 配置键或选项键。
    pub key: String,
    /// 展示标题。
    pub title: String,
    /// 配置或记录的中文说明。
    pub description: String,
    /// modules字段。
    pub modules: Vec<AdminModule>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台首页单个功能模块入口。
pub struct AdminModule {
    /// 配置键或选项键。
    pub key: String,
    /// 展示名称。
    pub name: String,
    /// 配置或记录的中文说明。
    pub description: String,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: ModuleStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// 后台首页模块状态，用于提示已完成、开发中或待配置。
pub enum ModuleStatus {
    Scaffolded,
}

/// 组装完整看板摘要并返回核心指标与模块列表。
pub fn dashboard_summary_with_orders(
    lotteries: Vec<LotteryKind>,
    draw_sources: Vec<DrawSource>,
    recent_orders: Vec<OrderSummary>,
    group_buy_plans: Vec<GroupBuyPlanSummary>,
    finance: FinanceOverview,
    financial_accounts: Vec<FinancialAccountSummary>,
    access: AccessSnapshot,
    invite_policy: InvitePolicySummary,
    robots: Vec<RobotConfigSummary>,
) -> DashboardSummary {
    let lottery_count = lotteries.len();
    let order_count = recent_orders.len();
    let finance_metric = money_label(finance.total_balance_minor);
    let user_count = access.users.len();

    DashboardSummary {
        metrics: vec![
            metric("users", "用户总数", user_count.to_string(), "内存用户仓储"),
            metric(
                "orders",
                "今日订单",
                order_count.to_string(),
                "内存订单仓储",
            ),
            metric(
                "lotteries",
                "已配置彩种",
                lottery_count.to_string(),
                "3 位与 5 位玩法",
            ),
            metric("finance", "平台余额", finance_metric, "内存资金仓储"),
        ],
        module_groups: module_groups(),
        lotteries,
        draw_sources,
        recent_orders,
        group_buy_plans,
        finance,
        financial_accounts,
        robots,
        users: access.users,
        admins: access.admins,
        roles: access.roles,
        settings: access.settings,
        registration: access.registration,
        invite_policy,
    }
}

/// 按管理员权限范围过滤看板内容，移除无权限模块。
pub fn dashboard_summary_for_scopes(
    mut summary: DashboardSummary,
    scopes: &[PermissionScope],
) -> DashboardSummary {
    summary
        .metrics
        .retain(|metric| is_allowed(metric_scope(&metric.key), scopes));
    summary.module_groups = summary
        .module_groups
        .into_iter()
        .filter_map(|mut group| {
            group
                .modules
                .retain(|module| is_allowed(module_scope(&module.key), scopes));

            if group.modules.is_empty() {
                None
            } else {
                Some(group)
            }
        })
        .collect();

    if !has_scope(scopes, &PermissionScope::Users) {
        summary.users.clear();
        summary.registration = redacted_registration_config();
    }
    if !has_scope(scopes, &PermissionScope::Orders) {
        summary.recent_orders.clear();
    }
    if !has_scope(scopes, &PermissionScope::Finance) {
        summary.finance = redacted_finance_overview();
        summary.financial_accounts.clear();
    }
    if !has_scope(scopes, &PermissionScope::Admins) {
        summary.admins.clear();
    }
    if !has_scope(scopes, &PermissionScope::Roles) {
        summary.roles.clear();
    }
    if !has_scope(scopes, &PermissionScope::SystemSettings) {
        summary.settings.clear();
    }
    if !has_scope(scopes, &PermissionScope::Lotteries) {
        summary.lotteries.clear();
        summary.draw_sources.clear();
        summary.group_buy_plans.clear();
    }
    if !has_scope(scopes, &PermissionScope::Robots) {
        summary.robots.clear();
    }
    if !has_scope(scopes, &PermissionScope::Rebates) {
        summary.invite_policy = redacted_invite_policy();
    }

    summary
}

/// 判断并返回布尔结果。
fn is_allowed(scope: Option<PermissionScope>, scopes: &[PermissionScope]) -> bool {
    match scope {
        Some(scope) => has_scope(scopes, &scope),
        None => true,
    }
}

/// 检查是否存在目标条件。
fn has_scope(scopes: &[PermissionScope], scope: &PermissionScope) -> bool {
    scopes.contains(scope)
}

/// 把系统概览指标键映射为读取权限。
fn metric_scope(key: &str) -> Option<PermissionScope> {
    match key {
        "users" => Some(PermissionScope::Users),
        "orders" => Some(PermissionScope::Orders),
        "lotteries" => Some(PermissionScope::Lotteries),
        "finance" => Some(PermissionScope::Finance),
        _ => None,
    }
}

/// 把系统概览模块键映射为读取权限。
fn module_scope(key: &str) -> Option<PermissionScope> {
    match key {
        "users" | "registration" => Some(PermissionScope::Users),
        "orders" | "settlements" => Some(PermissionScope::Orders),
        "finance" => Some(PermissionScope::Finance),
        "support" => Some(PermissionScope::CustomerService),
        "admins" => Some(PermissionScope::Admins),
        "roles" => Some(PermissionScope::Roles),
        "settings" | "advertisements" => Some(PermissionScope::SystemSettings),
        "lottery-console" | "lotteries" | "draw-modes" | "schedules" | "group-buy"
        | "play-rules" => Some(PermissionScope::Lotteries),
        "group-buy-robot" | "purchase-robot" => Some(PermissionScope::Robots),
        "invite" | "rebate" => Some(PermissionScope::Rebates),
        _ => None,
    }
}

/// 返回无权限时展示的脱敏财务概览。
fn redacted_finance_overview() -> FinanceOverview {
    FinanceOverview {
        total_balance_minor: 0,
        pending_withdraw_minor: 0,
        today_recharge_minor: 0,
        today_payout_minor: 0,
    }
}

/// 返回无权限时展示的脱敏注册配置。
fn redacted_registration_config() -> RegistrationConfig {
    RegistrationConfig {
        username_enabled: false,
        email_enabled: false,
        agent_invite_required: false,
    }
}

/// 返回无权限时展示的脱敏邀请策略。
fn redacted_invite_policy() -> InvitePolicySummary {
    InvitePolicySummary {
        agents_can_invite: false,
        regular_users_can_invite: false,
        rebate_mode: RebateMode::Immediate,
        supported_rebate_modes: Vec::new(),
        default_recharge_rebate_basis_points: 0,
    }
}

/// 把分为单位的金额格式化为后台展示文案。
fn money_label(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let abs_amount = amount_minor.checked_abs().unwrap_or(i64::MAX);
    format!("{sign}¥{}.{:02}", abs_amount / 100, abs_amount % 100)
}

/// 组装系统概览指标卡片。
fn metric(
    key: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
    trend: impl Into<String>,
) -> Metric {
    Metric {
        key: key.into(),
        label: label.into(),
        value: value.into(),
        trend: trend.into(),
    }
}

/// 组装系统概览模块分组。
fn module_groups() -> Vec<ModuleGroup> {
    vec![
        ModuleGroup {
            key: "common".to_string(),
            title: "公共功能".to_string(),
            description: "支撑日常运营的后台基础模块".to_string(),
            modules: vec![
                module(
                    "users",
                    "用户管理",
                    "添加用户、维护资料、查看资金",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "orders",
                    "订单管理",
                    "查询投注订单与状态",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "finance",
                    "财务管理",
                    "充值、提现、余额和流水入口",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "support",
                    "在线客服",
                    "客服会话和工单入口",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "admins",
                    "管理员管理",
                    "后台账号维护",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "roles",
                    "角色权限",
                    "权限范围与角色绑定",
                    ModuleStatus::Scaffolded,
                ),
            ],
        },
        ModuleGroup {
            key: "settings".to_string(),
            title: "系统设置".to_string(),
            description: "平台配置、注册策略和基础参数管理入口".to_string(),
            modules: vec![
                module(
                    "settings",
                    "系统设置",
                    "注册、彩种、风控等配置入口",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "advertisements",
                    "广告管理",
                    "配置手机端轮播广告和跳转入口",
                    ModuleStatus::Scaffolded,
                ),
            ],
        },
        ModuleGroup {
            key: "lottery".to_string(),
            title: "主要功能".to_string(),
            description: "彩种、开奖、玩法与合买管理".to_string(),
            modules: vec![
                module(
                    "lottery-console",
                    "彩种控制台",
                    "实时查看每个彩种倒计时和开奖号码",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "lotteries",
                    "彩种管理",
                    "依据 3 位和 5 位玩法创建彩种",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "draw-modes",
                    "开奖模式",
                    "平台开奖、API 开奖、指定号码",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "schedules",
                    "开奖时间",
                    "周期、每日固定、周开奖",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "group-buy",
                    "合买管理",
                    "合买计划、认购进度和参与记录",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "play-rules",
                    "玩法配置",
                    "查看玩法规则、启停玩法、配置彩种赔率",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "settlements",
                    "计奖派奖",
                    "开奖后计奖、派奖结果和订单状态",
                    ModuleStatus::Scaffolded,
                ),
            ],
        },
        ModuleGroup {
            key: "automation".to_string(),
            title: "机器人".to_string(),
            description: "自动发起合买与补未满单".to_string(),
            modules: vec![
                module(
                    "group-buy-robot",
                    "合买机器人",
                    "只负责发起合买计划",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "purchase-robot",
                    "补单机器人",
                    "只负责补未满单合买",
                    ModuleStatus::Scaffolded,
                ),
            ],
        },
        ModuleGroup {
            key: "growth".to_string(),
            title: "邀请返利".to_string(),
            description: "代理邀请、充值返利和注册策略".to_string(),
            modules: vec![
                module(
                    "registration",
                    "系统配置",
                    "注册方式与邀请要求",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "invite",
                    "邀请管理",
                    "代理邀请关系入口",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "rebate",
                    "返利管理",
                    "返利统计、明细与策略",
                    ModuleStatus::Scaffolded,
                ),
            ],
        },
    ]
}

/// 组装单个系统概览模块。
fn module(
    key: impl Into<String>,
    name: impl Into<String>,
    description: impl Into<String>,
    status: ModuleStatus,
) -> AdminModule {
    AdminModule {
        key: key.into(),
        name: name.into(),
        description: description.into(),
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::{dashboard_summary_for_scopes, dashboard_summary_with_orders, DashboardSummary};
    use crate::{
        domain::finance::{FinanceOverview, FinancialAccountSummary},
        domain::group_buy::{GroupBuyPlanStatus, GroupBuyPlanSummary},
        domain::permission::PermissionScope,
        domain::rebate::{InvitePolicySummary, RebateMode},
        domain::robot::{RobotConfigSummary, RobotKind, RobotStatus},
        services::access::{AccessRepository, AccessSnapshot},
        services::lottery::seed_lotteries,
    };
    /// 验证系统概览包含required模块分组。
    #[tokio::test]
    async fn dashboard_includes_required_module_groups() {
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let summary = sample_summary(access);
        let keys = summary
            .module_groups
            .iter()
            .map(|group| group.key.as_str())
            .collect::<Vec<_>>();

        assert!(keys.contains(&"common"));
        assert!(keys.contains(&"settings"));
        assert!(keys.contains(&"lottery"));
        assert!(keys.contains(&"automation"));
        assert!(keys.contains(&"growth"));

        let lottery_modules = summary
            .module_groups
            .iter()
            .find(|group| group.key == "lottery")
            .expect("lottery module group exists")
            .modules
            .iter()
            .map(|module| module.key.as_str())
            .collect::<Vec<_>>();
        assert!(lottery_modules.contains(&"lottery-console"));
    }
    /// 验证系统概览筛选sensitivefields用于ops权限。
    #[tokio::test]
    async fn dashboard_filters_sensitive_fields_for_ops_scopes() {
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let summary = dashboard_summary_for_scopes(
            sample_summary(access),
            &[
                PermissionScope::Users,
                PermissionScope::Orders,
                PermissionScope::Lotteries,
            ],
        );

        let metric_keys = summary
            .metrics
            .iter()
            .map(|metric| metric.key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(metric_keys, vec!["users", "orders", "lotteries"]);

        let module_keys = summary
            .module_groups
            .iter()
            .flat_map(|group| group.modules.iter())
            .map(|module| module.key.as_str())
            .collect::<Vec<_>>();
        assert!(module_keys.contains(&"users"));
        assert!(module_keys.contains(&"orders"));
        assert!(module_keys.contains(&"registration"));
        assert!(module_keys.contains(&"lottery-console"));
        assert!(module_keys.contains(&"play-rules"));
        assert!(module_keys.contains(&"settlements"));
        assert!(!module_keys.contains(&"admins"));
        assert!(!module_keys.contains(&"roles"));
        assert!(!module_keys.contains(&"settings"));
        assert!(!module_keys.contains(&"finance"));
        assert!(!module_keys.contains(&"support"));
        assert!(!module_keys.contains(&"group-buy-robot"));
        assert!(!module_keys.contains(&"invite"));
        assert!(!module_keys.contains(&"rebate"));

        assert!(!summary.users.is_empty());
        assert!(!summary.lotteries.is_empty());
        assert!(!summary.group_buy_plans.is_empty());
        assert!(summary.admins.is_empty());
        assert!(summary.roles.is_empty());
        assert!(summary.settings.is_empty());
        assert_eq!(summary.finance.total_balance_minor, 0);
        assert!(summary.financial_accounts.is_empty());
        assert!(summary.robots.is_empty());
        assert!(summary.registration.username_enabled);
        assert!(!summary.invite_policy.agents_can_invite);
        assert_eq!(
            summary.invite_policy.default_recharge_rebate_basis_points,
            0
        );
    }
    /// 验证系统概览keeps完整摘要用于super权限。
    #[tokio::test]
    async fn dashboard_keeps_full_summary_for_super_scopes() {
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let scopes = access
            .roles
            .iter()
            .find(|role| role.id == "role-super")
            .expect("super role exists")
            .scopes
            .clone();
        let summary = dashboard_summary_for_scopes(sample_summary(access), &scopes);

        let module_keys = summary
            .module_groups
            .iter()
            .flat_map(|group| group.modules.iter())
            .map(|module| module.key.as_str())
            .collect::<Vec<_>>();
        assert!(module_keys.contains(&"admins"));
        assert!(module_keys.contains(&"roles"));
        assert!(module_keys.contains(&"finance"));
        assert!(module_keys.contains(&"group-buy-robot"));
        assert!(module_keys.contains(&"invite"));
        assert!(module_keys.contains(&"rebate"));
        assert!(!summary.admins.is_empty());
        assert!(!summary.roles.is_empty());
        assert!(!summary.settings.is_empty());
        assert_eq!(summary.finance.total_balance_minor, 684_000);
        assert!(!summary.financial_accounts.is_empty());
        assert!(!summary.robots.is_empty());
        assert!(summary.invite_policy.agents_can_invite);
    }

    /// 构造样例摘要视图。
    fn sample_summary(access: AccessSnapshot) -> DashboardSummary {
        dashboard_summary_with_orders(
            seed_lotteries(),
            Vec::new(),
            Vec::new(),
            vec![GroupBuyPlanSummary {
                id: "G202606020001".to_string(),
                lottery_id: "fc3d".to_string(),
                lottery_name: "福彩 3D".to_string(),
                order_id: None,
                order_status: None,
                order_draw_number: None,
                order_payout_minor: None,
                order_settled_at: None,
                issue: "20260602001".to_string(),
                rule_code: "threeDirect".to_string(),
                title: "福彩 3D 第20260602001期合买".to_string(),
                initiator_user_id: "U90001".to_string(),
                initiator_username: "agent_alpha".to_string(),
                total_amount_minor: 100_000,
                filled_amount_minor: 72_000,
                share_count: 1_000,
                status: GroupBuyPlanStatus::Open,
                created_at: "2026-06-02 09:00:00".to_string(),
            }],
            FinanceOverview {
                total_balance_minor: 684_000,
                pending_withdraw_minor: 0,
                today_recharge_minor: 0,
                today_payout_minor: 0,
            },
            vec![FinancialAccountSummary {
                user_id: "U10001".to_string(),
                available_balance_minor: 12_000,
                frozen_balance_minor: 2_000,
            }],
            access,
            InvitePolicySummary {
                agents_can_invite: true,
                regular_users_can_invite: false,
                rebate_mode: RebateMode::Immediate,
                supported_rebate_modes: vec![RebateMode::Immediate, RebateMode::RechargeTiered],
                default_recharge_rebate_basis_points: 350,
            },
            vec![RobotConfigSummary {
                id: "RB-1".to_string(),
                name: "合买发单机器人".to_string(),
                kind: RobotKind::GroupBuy,
                lottery_ids: vec!["fc3d".to_string()],
                status: RobotStatus::Enabled,
                description: "只负责发起合买计划".to_string(),
                group_buy_fill_strategy: crate::domain::robot::GroupBuyRobotFillStrategy::Rhythm,
                group_buy_fill_before_draw_seconds: 15,
                group_buy_rhythm_fill_max_percent: 20,
                group_buy_rhythm_stage_count: 3,
                deletable: true,
            }],
        )
    }
}
