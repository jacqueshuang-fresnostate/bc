use serde::Serialize;

use crate::domain::{
    finance::{FinanceOverview, FinancialAccountSummary},
    lottery::{DrawMode, DrawSource, LotteryKind},
    order::{GroupBuyPlanSummary, OrderSummary},
    permission::{AdminRole, SystemSetting},
    rebate::{InvitePolicySummary, RebateMode},
    robot::{RobotConfigSummary, RobotKind, RobotStatus},
    user::{AdminSummary, RegistrationConfig, UserSummary},
};

use super::access::AccessSnapshot;

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

pub fn dashboard_summary_with_orders(
    lotteries: Vec<LotteryKind>,
    recent_orders: Vec<OrderSummary>,
    finance: FinanceOverview,
    financial_accounts: Vec<FinancialAccountSummary>,
    access: AccessSnapshot,
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
        draw_sources: draw_sources(),
        recent_orders,
        group_buy_plans: group_buy_plans(),
        finance,
        financial_accounts,
        robots: robots(),
        users: access.users,
        admins: access.admins,
        roles: access.roles,
        settings: access.settings,
        registration: access.registration,
        invite_policy: InvitePolicySummary {
            agents_can_invite: true,
            regular_users_can_invite: false,
            rebate_mode: RebateMode::Immediate,
            supported_rebate_modes: vec![RebateMode::Immediate, RebateMode::RechargeTiered],
            default_recharge_rebate_basis_points: 350,
        },
    }
}

fn money_label(amount_minor: i64) -> String {
    let sign = if amount_minor < 0 { "-" } else { "" };
    let abs_amount = amount_minor.checked_abs().unwrap_or(i64::MAX);
    format!("{sign}¥{}.{:02}", abs_amount / 100, abs_amount % 100)
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

pub fn draw_sources() -> Vec<DrawSource> {
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

#[cfg(test)]
mod tests {
    use super::dashboard_summary_with_orders;
    use crate::{
        domain::finance::{FinanceOverview, FinancialAccountSummary},
        services::access::AccessRepository,
        services::lottery::seed_lotteries,
    };

    #[tokio::test]
    async fn dashboard_includes_required_module_groups() {
        let access = AccessRepository::memory_seeded()
            .snapshot()
            .await
            .expect("access snapshot can load");
        let summary = dashboard_summary_with_orders(
            seed_lotteries(),
            Vec::new(),
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
        );
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
