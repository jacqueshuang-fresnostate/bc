//! 权限与角色领域模型，承载系统设置、模块权限和细粒度操作权限定义

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
/// 后台权限范围，控制管理员可以访问的业务模块。
pub enum PermissionScope {
    Users,
    Orders,
    Finance,
    CustomerService,
    Admins,
    Roles,
    SystemSettings,
    Lotteries,
    Robots,
    Rebates,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 后台细粒度权限点定义，用于把“能看某模块”和“能执行高风险动作”拆开管理。
pub struct AdminPermissionDefinition {
    /// 权限点唯一键，前端和后端鉴权都使用这个稳定字符串。
    pub key: &'static str,
    /// 权限点在角色维护页面展示的中文名称。
    pub label: &'static str,
    /// 权限点所属的中文分组。
    pub group: &'static str,
    /// 兼容旧模块权限时对应的模块范围。
    pub scope: PermissionScope,
    /// 是否属于调账、删除、清空、审核等高风险动作。
    pub sensitive: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 管理员角色，绑定角色名称、兼容模块范围和细粒度权限点集合。
pub struct AdminRole {
    /// 业务唯一标识。
    pub id: String,
    /// 展示名称。
    pub name: String,
    /// 角色拥有的模块权限范围列表，继续用于兼容旧角色和控制左侧菜单。
    pub scopes: Vec<PermissionScope>,
    /// 角色显式配置的细粒度权限点；旧数据缺少该字段时默认使用空数组。
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// 返回后台全部细粒度权限点定义。
pub fn admin_permission_definitions() -> &'static [AdminPermissionDefinition] {
    ADMIN_PERMISSION_DEFINITIONS
}

/// 判断权限点是否属于当前系统已知定义，避免保存拼写错误或过期权限。
pub fn is_known_permission_key(key: &str) -> bool {
    ADMIN_PERMISSION_DEFINITIONS
        .iter()
        .any(|definition| definition.key == key)
}

/// 将旧模块权限和新权限点合并成会话可直接判断的有效权限集合。
pub fn effective_permission_keys(
    scopes: &[PermissionScope],
    permissions: &[String],
) -> Vec<String> {
    let mut explicit_keys = BTreeSet::new();
    for permission in permissions {
        if is_known_permission_key(permission) {
            explicit_keys.insert(permission.clone());
        }
    }
    if !explicit_keys.is_empty() {
        return explicit_keys.into_iter().collect();
    }

    let mut keys = BTreeSet::new();
    for scope in scopes {
        for definition in ADMIN_PERMISSION_DEFINITIONS
            .iter()
            .filter(|definition| &definition.scope == scope)
        {
            keys.insert(definition.key.to_string());
        }
    }

    keys.into_iter().collect()
}

const ADMIN_PERMISSION_DEFINITIONS: &[AdminPermissionDefinition] = &[
    AdminPermissionDefinition {
        key: "user.read",
        label: "查看用户",
        group: "用户管理",
        scope: PermissionScope::Users,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "user.write",
        label: "新增或编辑用户",
        group: "用户管理",
        scope: PermissionScope::Users,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "user.status",
        label: "启停或锁定用户",
        group: "用户管理",
        scope: PermissionScope::Users,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "user.password.reset",
        label: "重置用户密码",
        group: "用户管理",
        scope: PermissionScope::Users,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "user.delete",
        label: "删除用户",
        group: "用户管理",
        scope: PermissionScope::Users,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "admin.read",
        label: "查看管理员",
        group: "管理员管理",
        scope: PermissionScope::Admins,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "admin.write",
        label: "新增或编辑管理员",
        group: "管理员管理",
        scope: PermissionScope::Admins,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "admin.status",
        label: "启停或锁定管理员",
        group: "管理员管理",
        scope: PermissionScope::Admins,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "admin.password.reset",
        label: "重置管理员密码",
        group: "管理员管理",
        scope: PermissionScope::Admins,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "role.read",
        label: "查看角色",
        group: "角色权限",
        scope: PermissionScope::Roles,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "role.write",
        label: "新增或编辑角色",
        group: "角色权限",
        scope: PermissionScope::Roles,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "role.delete",
        label: "删除角色",
        group: "角色权限",
        scope: PermissionScope::Roles,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "finance.read",
        label: "查看财务",
        group: "财务管理",
        scope: PermissionScope::Finance,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "finance.adjust.create",
        label: "手动调账",
        group: "财务管理",
        scope: PermissionScope::Finance,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "finance.ledger.clear",
        label: "清除资金流水",
        group: "财务管理",
        scope: PermissionScope::Finance,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "recharge.confirm",
        label: "确认充值",
        group: "充值订单",
        scope: PermissionScope::Finance,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "recharge.export",
        label: "导出充值记录",
        group: "充值订单",
        scope: PermissionScope::Finance,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "recharge.clear",
        label: "清除充值记录",
        group: "充值订单",
        scope: PermissionScope::Finance,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "withdrawal.review",
        label: "审核提现",
        group: "提现管理",
        scope: PermissionScope::Finance,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "withdrawal.clear",
        label: "清除提现记录",
        group: "提现管理",
        scope: PermissionScope::Finance,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "order.read",
        label: "查看订单",
        group: "订单管理",
        scope: PermissionScope::Orders,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "order.write",
        label: "创建或处理订单",
        group: "订单管理",
        scope: PermissionScope::Orders,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "order.clear",
        label: "清除投注记录",
        group: "订单管理",
        scope: PermissionScope::Orders,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "settlement.run",
        label: "计奖派奖",
        group: "计奖派奖",
        scope: PermissionScope::Orders,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "support.read",
        label: "查看客服会话",
        group: "在线客服",
        scope: PermissionScope::CustomerService,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "support.reply",
        label: "回复客服消息",
        group: "在线客服",
        scope: PermissionScope::CustomerService,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "support.manage",
        label: "管理客服会话",
        group: "在线客服",
        scope: PermissionScope::CustomerService,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "lottery.read",
        label: "查看彩种",
        group: "彩种管理",
        scope: PermissionScope::Lotteries,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "lottery.write",
        label: "新增或编辑彩种",
        group: "彩种管理",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "lottery.sale.toggle",
        label: "切换销售状态",
        group: "彩种管理",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "lottery.draw.control",
        label: "控制开奖号码",
        group: "彩种控制台",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "lottery.issue.write",
        label: "维护期号",
        group: "期号管理",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "lottery.source.manage",
        label: "维护开奖源",
        group: "开奖源",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "lottery.source.sync",
        label: "同步开奖源",
        group: "开奖源",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "play.rule.manage",
        label: "玩法配置",
        group: "玩法规则",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "group.buy.read",
        label: "查看合买",
        group: "合买管理",
        scope: PermissionScope::Lotteries,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "group.buy.manage",
        label: "维护合买",
        group: "合买管理",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "group.buy.clear",
        label: "清除合买记录",
        group: "合买管理",
        scope: PermissionScope::Lotteries,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "robot.read",
        label: "查看机器人",
        group: "机器人配置",
        scope: PermissionScope::Robots,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "robot.write",
        label: "维护机器人",
        group: "机器人配置",
        scope: PermissionScope::Robots,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "robot.run",
        label: "执行机器人",
        group: "机器人配置",
        scope: PermissionScope::Robots,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "robot.delete",
        label: "删除机器人",
        group: "机器人配置",
        scope: PermissionScope::Robots,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "rebate.read",
        label: "查看邀请返利",
        group: "邀请返利",
        scope: PermissionScope::Rebates,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "rebate.withdraw",
        label: "处理返利提现",
        group: "邀请返利",
        scope: PermissionScope::Rebates,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "agent.review",
        label: "审核代理申请",
        group: "代理管理",
        scope: PermissionScope::Rebates,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "invite.manage",
        label: "维护邀请配置",
        group: "邀请管理",
        scope: PermissionScope::Rebates,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "system.read",
        label: "查看系统设置",
        group: "系统设置",
        scope: PermissionScope::SystemSettings,
        sensitive: false,
    },
    AdminPermissionDefinition {
        key: "system.write",
        label: "修改系统设置",
        group: "系统设置",
        scope: PermissionScope::SystemSettings,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "system.cache.reload",
        label: "刷新系统缓存",
        group: "系统设置",
        scope: PermissionScope::SystemSettings,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "system.chat.clear",
        label: "清空聊天大厅",
        group: "系统设置",
        scope: PermissionScope::SystemSettings,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "system.upload",
        label: "上传系统文件",
        group: "系统设置",
        scope: PermissionScope::SystemSettings,
        sensitive: true,
    },
    AdminPermissionDefinition {
        key: "advertisement.manage",
        label: "维护广告",
        group: "广告管理",
        scope: PermissionScope::SystemSettings,
        sensitive: true,
    },
];

#[cfg(test)]
mod tests {
    use super::{effective_permission_keys, PermissionScope};

    #[test]
    /// 旧角色没有显式权限点时，继续按模块权限展开，保证历史角色可用。
    fn effective_permissions_expand_legacy_scopes_when_explicit_permissions_empty() {
        let permissions = effective_permission_keys(&[PermissionScope::Finance], &[]);

        assert!(permissions.contains(&"finance.read".to_string()));
        assert!(permissions.contains(&"finance.adjust.create".to_string()));
    }

    #[test]
    /// 新角色配置了显式权限点后，只按权限点授权，避免模块权限继续放大为全部操作。
    fn effective_permissions_prefer_explicit_permissions() {
        let permissions =
            effective_permission_keys(&[PermissionScope::Finance], &["finance.read".to_string()]);

        assert_eq!(permissions, vec!["finance.read".to_string()]);
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 系统设置项，保存后台可维护的运行配置。
pub struct SystemSetting {
    /// 配置键或选项键。
    pub key: String,
    /// 配置值或选项值。
    pub value: String,
    /// 配置或记录的中文说明。
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 更新单个系统设置值时提交的请求。
pub struct UpdateSystemSettingRequest {
    /// 配置值或选项值。
    pub value: String,
}
