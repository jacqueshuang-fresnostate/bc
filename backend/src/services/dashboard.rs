use serde::Serialize;

use crate::domain::{
    finance::{FinanceOverview, FinancialAccountSummary},
    lottery::{DrawMode, DrawSource, LotteryKind},
    order::{GroupBuyPlanSummary, OrderStatus, OrderSummary},
    permission::{AdminRole, PermissionScope, SystemSetting},
    rebate::{InvitePolicySummary, RebateMode},
    robot::{RobotConfigSummary, RobotKind, RobotStatus},
    user::{AdminSummary, RegistrationConfig, UserKind, UserStatus, UserSummary},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub metrics: Vec<Metric>,
    pub module_groups: Vec<ModuleGroup>,
    pub lotteries: Vec<LotteryKind>,
    pub draw_sources: Vec<DrawSource>,
    pub recent_orders: Vec<OrderSummary>,
    pub group_buy_plans: Vec<GroupBuyPlanSummary>,
    pub finance: FinanceOverview,
    pub financial_accounts: Vec<FinancialAccountSummary>,
    pub robots: Vec<RobotConfigSummary>,
    pub users: Vec<UserSummary>,
    pub admins: Vec<AdminSummary>,
    pub roles: Vec<AdminRole>,
    pub settings: Vec<SystemSetting>,
    pub registration: RegistrationConfig,
    pub invite_policy: InvitePolicySummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metric {
    pub key: String,
    pub label: String,
    pub value: String,
    pub trend: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleGroup {
    pub key: String,
    pub title: String,
    pub description: String,
    pub modules: Vec<AdminModule>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminModule {
    pub key: String,
    pub name: String,
    pub description: String,
    pub status: ModuleStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModuleStatus {
    Scaffolded,
    Planned,
}

pub fn dashboard_summary(lotteries: Vec<LotteryKind>) -> DashboardSummary {
    let lottery_count = lotteries.len();

    DashboardSummary {
        metrics: vec![
            metric("users", "用户总数", "128", "+12 今日新增"),
            metric("orders", "今日订单", "2,418", "示例数据"),
            metric(
                "lotteries",
                "已配置彩种",
                lottery_count.to_string(),
                "3 位与 5 位玩法",
            ),
            metric("finance", "平台余额", "¥83,240.00", "等待接入数据库"),
        ],
        module_groups: module_groups(),
        lotteries,
        draw_sources: draw_sources(),
        recent_orders: recent_orders(),
        group_buy_plans: group_buy_plans(),
        finance: FinanceOverview {
            total_balance_minor: 8_324_000,
            pending_withdraw_minor: 240_000,
            today_recharge_minor: 1_980_000,
            today_payout_minor: 760_000,
        },
        financial_accounts: financial_accounts(),
        robots: robots(),
        users: users(),
        admins: admins(),
        roles: roles(),
        settings: settings(),
        registration: RegistrationConfig {
            username_enabled: true,
            email_enabled: false,
            agent_invite_required: false,
        },
        invite_policy: InvitePolicySummary {
            agents_can_invite: true,
            regular_users_can_invite: false,
            rebate_mode: RebateMode::Immediate,
            supported_rebate_modes: vec![RebateMode::Immediate, RebateMode::RechargeTiered],
            default_recharge_rebate_basis_points: 350,
        },
    }
}

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
                    ModuleStatus::Planned,
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
                module(
                    "settings",
                    "系统设置",
                    "注册、彩种、风控等配置入口",
                    ModuleStatus::Scaffolded,
                ),
            ],
        },
        ModuleGroup {
            key: "lottery".to_string(),
            title: "主要功能".to_string(),
            description: "彩种、开奖、玩法与合买配置".to_string(),
            modules: vec![
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
                    "合买配置",
                    "份额、起购、参与金额配置",
                    ModuleStatus::Scaffolded,
                ),
            ],
        },
        ModuleGroup {
            key: "automation".to_string(),
            title: "机器人".to_string(),
            description: "自动发起合买与模拟购彩".to_string(),
            modules: vec![
                module(
                    "group-buy-robot",
                    "合买机器人",
                    "发起合买和满单辅助",
                    ModuleStatus::Scaffolded,
                ),
                module(
                    "purchase-robot",
                    "购彩机器人",
                    "开盘期间模拟购彩",
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
                    "用户注册",
                    "用户名注册与邮箱注册开关",
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
                    "返利配置",
                    "立即返利与阶梯返利",
                    ModuleStatus::Scaffolded,
                ),
            ],
        },
    ]
}

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

fn draw_sources() -> Vec<DrawSource> {
    vec![
        DrawSource {
            id: "fc3d-api".to_string(),
            name: "福彩 3D API".to_string(),
            mode: DrawMode::Api,
            reusable_for_lottery_ids: vec!["fc3d".to_string(), "pl3".to_string()],
        },
        DrawSource {
            id: "platform-random-5d".to_string(),
            name: "平台 5 位随机生成器".to_string(),
            mode: DrawMode::Platform,
            reusable_for_lottery_ids: vec!["ssc60".to_string()],
        },
    ]
}

fn recent_orders() -> Vec<OrderSummary> {
    vec![
        OrderSummary {
            id: "O202606020001".to_string(),
            user_id: "U10001".to_string(),
            lottery_id: "fc3d".to_string(),
            issue: "2026154".to_string(),
            amount_minor: 2_000,
            status: OrderStatus::PendingDraw,
        },
        OrderSummary {
            id: "O202606020002".to_string(),
            user_id: "U10002".to_string(),
            lottery_id: "ssc60".to_string(),
            issue: "20260602-812".to_string(),
            amount_minor: 5_000,
            status: OrderStatus::Won,
        },
        OrderSummary {
            id: "O202606020003".to_string(),
            user_id: "U10004".to_string(),
            lottery_id: "pl3".to_string(),
            issue: "2026154".to_string(),
            amount_minor: 4_000,
            status: OrderStatus::Lost,
        },
        OrderSummary {
            id: "O202606020004".to_string(),
            user_id: "U10005".to_string(),
            lottery_id: "manual-test".to_string(),
            issue: "20260602-test".to_string(),
            amount_minor: 1_000,
            status: OrderStatus::Cancelled,
        },
    ]
}

fn group_buy_plans() -> Vec<GroupBuyPlanSummary> {
    vec![GroupBuyPlanSummary {
        id: "G202606020001".to_string(),
        lottery_id: "fc3d".to_string(),
        initiator_user_id: "U10003".to_string(),
        total_amount_minor: 100_000,
        filled_amount_minor: 72_000,
        share_count: 1_000,
        status: "open".to_string(),
    }]
}

fn robots() -> Vec<RobotConfigSummary> {
    vec![
        RobotConfigSummary {
            id: "R-GROUP-001".to_string(),
            name: "合买补单机器人".to_string(),
            kind: RobotKind::GroupBuy,
            lottery_ids: vec!["fc3d".to_string(), "ssc60".to_string()],
            status: RobotStatus::Enabled,
            description: "开盘期间发起合买并辅助满单".to_string(),
        },
        RobotConfigSummary {
            id: "R-BUY-001".to_string(),
            name: "购彩模拟机器人".to_string(),
            kind: RobotKind::Purchase,
            lottery_ids: vec!["ssc60".to_string()],
            status: RobotStatus::Paused,
            description: "按彩种开盘时间模拟普通用户购彩".to_string(),
        },
        RobotConfigSummary {
            id: "R-BUY-002".to_string(),
            name: "指定号码测试机器人".to_string(),
            kind: RobotKind::Purchase,
            lottery_ids: vec!["manual-test".to_string()],
            status: RobotStatus::Disabled,
            description: "指定号码测试彩暂停机器人执行".to_string(),
        },
    ]
}

fn users() -> Vec<UserSummary> {
    vec![
        UserSummary {
            id: "U10001".to_string(),
            username: "demo_user".to_string(),
            email: Some("demo@example.com".to_string()),
            kind: UserKind::Regular,
            status: UserStatus::Active,
            balance_minor: 12_000,
            agent_id: Some("U90001".to_string()),
        },
        UserSummary {
            id: "U90001".to_string(),
            username: "agent_alpha".to_string(),
            email: None,
            kind: UserKind::Agent,
            status: UserStatus::Active,
            balance_minor: 520_000,
            agent_id: None,
        },
        UserSummary {
            id: "U10004".to_string(),
            username: "risk_watch".to_string(),
            email: None,
            kind: UserKind::Regular,
            status: UserStatus::Suspended,
            balance_minor: 0,
            agent_id: Some("U90001".to_string()),
        },
    ]
}

fn admins() -> Vec<AdminSummary> {
    vec![
        AdminSummary {
            id: "A10001".to_string(),
            username: "admin".to_string(),
            role_name: "超级管理员".to_string(),
            status: UserStatus::Active,
        },
        AdminSummary {
            id: "A10002".to_string(),
            username: "locked_admin".to_string(),
            role_name: "运营管理员".to_string(),
            status: UserStatus::Locked,
        },
    ]
}

fn financial_accounts() -> Vec<FinancialAccountSummary> {
    vec![
        FinancialAccountSummary {
            user_id: "U10001".to_string(),
            available_balance_minor: 12_000,
            frozen_balance_minor: 2_000,
        },
        FinancialAccountSummary {
            user_id: "U90001".to_string(),
            available_balance_minor: 520_000,
            frozen_balance_minor: 0,
        },
    ]
}

fn roles() -> Vec<AdminRole> {
    vec![
        AdminRole {
            id: "role-super".to_string(),
            name: "超级管理员".to_string(),
            scopes: vec![
                PermissionScope::Users,
                PermissionScope::Orders,
                PermissionScope::Finance,
                PermissionScope::CustomerService,
                PermissionScope::Admins,
                PermissionScope::Roles,
                PermissionScope::SystemSettings,
                PermissionScope::Lotteries,
                PermissionScope::Robots,
                PermissionScope::Rebates,
            ],
        },
        AdminRole {
            id: "role-ops".to_string(),
            name: "运营管理员".to_string(),
            scopes: vec![
                PermissionScope::Users,
                PermissionScope::Orders,
                PermissionScope::Lotteries,
            ],
        },
    ]
}

fn settings() -> Vec<SystemSetting> {
    vec![
        SystemSetting {
            key: "email_registration_enabled".to_string(),
            value: "false".to_string(),
            description: "是否开启邮箱注册".to_string(),
        },
        SystemSetting {
            key: "recharge_rebate_mode".to_string(),
            value: "immediate".to_string(),
            description: "代理充值返利模式".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::dashboard_summary;
    use crate::services::lottery::seed_lotteries;

    #[test]
    fn dashboard_includes_required_module_groups() {
        let summary = dashboard_summary(seed_lotteries());
        let keys = summary
            .module_groups
            .iter()
            .map(|group| group.key.as_str())
            .collect::<Vec<_>>();

        assert!(keys.contains(&"common"));
        assert!(keys.contains(&"lottery"));
        assert!(keys.contains(&"automation"));
        assert!(keys.contains(&"growth"));
    }
}
