//! 手机端实时事件中心，负责统一封装、广播和按用户过滤业务事件

use chrono::Local;
use serde_json::{json, Value};
use tokio::sync::broadcast;

use crate::domain::{
    draw::DrawIssue, finance::FinancialAccountSummary, order::OrderDetail,
    recharge::RechargeOrderSummary, withdrawal::WithdrawalOrderSummary,
};

const REALTIME_CHANNEL_CAPACITY: usize = 512;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RealtimeAudience {
    Public,
    User(String),
}

#[derive(Debug, Clone)]
pub struct RealtimeMessage {
    pub audience: RealtimeAudience,
    pub payload: Value,
}

#[derive(Clone)]
pub struct RealtimeHub {
    sender: broadcast::Sender<RealtimeMessage>,
}

impl Default for RealtimeHub {
    /// 创建默认实时事件中心。
    fn default() -> Self {
        Self::new()
    }
}

impl RealtimeHub {
    /// 初始化实时事件中心，使用广播通道保存短时间内的业务事件。
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(REALTIME_CHANNEL_CAPACITY);
        Self { sender }
    }

    /// 创建一个新的事件订阅者，供 WebSocket 连接独立消费。
    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeMessage> {
        self.sender.subscribe()
    }

    /// 发布公开事件，所有在线手机端连接都可以收到。
    pub fn publish_public(&self, payload: Value) {
        self.publish(RealtimeAudience::Public, payload);
    }

    /// 发布用户私有事件，仅推给携带对应用户 token 建立的连接。
    pub fn publish_user(&self, user_id: &str, payload: Value) {
        self.publish(RealtimeAudience::User(user_id.to_string()), payload);
    }

    /// 写入广播通道；没有订阅者时忽略发送失败，避免影响主业务事务。
    fn publish(&self, audience: RealtimeAudience, payload: Value) {
        let _ = self.sender.send(RealtimeMessage { audience, payload });
    }
}

/// 判断当前连接是否可以收到目标事件。
pub fn audience_matches(audience: &RealtimeAudience, user_id: Option<&str>) -> bool {
    match audience {
        RealtimeAudience::Public => true,
        RealtimeAudience::User(target_user_id) => user_id.is_some_and(|id| id == target_user_id),
    }
}

/// 封装心跳事件，帮助客户端判断连接是否存活。
pub fn heartbeat_event() -> Value {
    realtime_envelope("system.heartbeat", "public", json!({}))
}

/// 封装开奖完成事件，供首页、开奖页和下注页刷新当前彩种状态。
pub fn draw_result_event(issue: &DrawIssue) -> Value {
    realtime_envelope(
        "lottery.draw_result",
        "public",
        json!({
            "lotteryId": issue.lottery_id,
            "lotteryName": issue.lottery_name,
            "issue": issue.issue,
            "numberType": issue.number_type.clone(),
            "drawMode": issue.draw_mode.clone(),
            "drawNumber": issue.draw_number.clone().unwrap_or_default(),
            "resultNumbers": draw_number_parts(issue.draw_number.as_deref()),
            "scheduledAt": issue.scheduled_at,
            "saleClosedAt": issue.sale_closed_at,
            "drawnAt": issue.drawn_at,
        }),
    )
}

/// 封装期号封盘事件，供投注页在封盘后停止提交本期订单。
pub fn issue_closed_event(issue: &DrawIssue) -> Value {
    realtime_envelope(
        "lottery.issue_closed",
        "public",
        json!({
            "lotteryId": issue.lottery_id,
            "lotteryName": issue.lottery_name,
            "issue": issue.issue,
            "scheduledAt": issue.scheduled_at,
            "saleClosedAt": issue.sale_closed_at,
            "status": issue.status.clone(),
        }),
    )
}

/// 封装新期号开盘事件，供手机端在当前期结束后刷新下一期。
pub fn issue_opened_event(issue: &DrawIssue) -> Value {
    realtime_envelope(
        "lottery.issue_opened",
        "public",
        json!({
            "lotteryId": issue.lottery_id,
            "lotteryName": issue.lottery_name,
            "issue": issue.issue,
            "scheduledAt": issue.scheduled_at,
            "saleClosedAt": issue.sale_closed_at,
            "status": issue.status.clone(),
        }),
    )
}

/// 封装用户余额变化事件，只推送给资产发生变化的用户本人。
pub fn balance_changed_event(
    account: &FinancialAccountSummary,
    reason: &str,
    reference_id: Option<&str>,
) -> Value {
    realtime_envelope(
        "user.balance_changed",
        "user",
        json!({
            "userId": account.user_id,
            "availableBalanceMinor": account.available_balance_minor,
            "frozenBalanceMinor": account.frozen_balance_minor,
            "reason": reason,
            "referenceId": reference_id,
        }),
    )
}

/// 封装用户订单变化事件，用于注单列表和详情刷新。
pub fn order_changed_event(order: &OrderDetail, action: &str) -> Value {
    realtime_envelope(
        "user.order_changed",
        "user",
        json!({
            "action": action,
            "order": order,
        }),
    )
}

/// 封装充值订单变化事件，用于充值记录刷新。
pub fn recharge_changed_event(order: &RechargeOrderSummary) -> Value {
    realtime_envelope(
        "user.recharge_changed",
        "user",
        json!({
            "order": order,
        }),
    )
}

/// 封装提现订单变化事件，用于提现记录刷新。
pub fn withdrawal_changed_event(order: &WithdrawalOrderSummary) -> Value {
    realtime_envelope(
        "user.withdrawal_changed",
        "user",
        json!({
            "order": order,
        }),
    )
}

/// 统一构造实时事件信封，保证客户端只解析一种结构。
fn realtime_envelope(event: &str, scope: &str, data: Value) -> Value {
    json!({
        "event": event,
        "scope": scope,
        "occurredAt": current_timestamp(),
        "data": data,
    })
}

/// 将逗号分隔的开奖号码转成数组；无逗号的纯数字兼容为逐位数字。
fn draw_number_parts(draw_number: Option<&str>) -> Vec<String> {
    let text = draw_number.unwrap_or_default().trim();
    if text.is_empty() {
        return Vec::new();
    }

    if text.contains(',') || text.contains('，') || text.contains(' ') {
        return text
            .split(|character| matches!(character, ',' | '，' | ' '))
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect();
    }

    if text.chars().all(|character| character.is_ascii_digit()) {
        return text
            .chars()
            .map(|character| character.to_string())
            .collect();
    }

    vec![text.to_string()]
}

/// 返回当前本地时间字符串，与现有业务时间字段格式保持一致。
fn current_timestamp() -> String {
    Local::now()
        .naive_local()
        .format(TIMESTAMP_FORMAT)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        draw::DrawIssueStatus,
        lottery::{DrawMode, LotteryNumberType},
    };

    #[test]
    /// 验证公开事件可以被匿名连接接收，用户私有事件只推送给目标用户。
    fn realtime_audience_filters_user_events() {
        assert!(audience_matches(&RealtimeAudience::Public, None));
        assert!(audience_matches(
            &RealtimeAudience::User("U10001".to_string()),
            Some("U10001")
        ));
        assert!(!audience_matches(
            &RealtimeAudience::User("U10001".to_string()),
            Some("U10002")
        ));
        assert!(!audience_matches(
            &RealtimeAudience::User("U10001".to_string()),
            None
        ));
    }

    #[test]
    /// 验证开奖号码优先按逗号分隔，同时兼容无分隔纯数字。
    fn draw_number_parts_supports_comma_and_digits() {
        assert_eq!(draw_number_parts(Some("1,2,3")), vec!["1", "2", "3"]);
        assert_eq!(draw_number_parts(Some("1，2 3")), vec!["1", "2", "3"]);
        assert_eq!(
            draw_number_parts(Some("12345")),
            vec!["1", "2", "3", "4", "5"]
        );
    }

    #[test]
    /// 验证开奖事件使用当前系统统一事件信封。
    fn draw_result_event_uses_current_realtime_envelope() {
        let issue = DrawIssue {
            id: "D1".to_string(),
            lottery_id: "txffc".to_string(),
            lottery_name: "腾讯分分彩".to_string(),
            issue: "20260605001".to_string(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Api,
            scheduled_at: "2026-06-05 12:00:00".to_string(),
            sale_closed_at: "2026-06-05 11:59:30".to_string(),
            status: DrawIssueStatus::Drawn,
            draw_number: Some("1,2,3,4,5".to_string()),
            drawn_at: Some("2026-06-05 12:00:01".to_string()),
            created_at: "2026-06-05 11:50:00".to_string(),
        };

        let event = draw_result_event(&issue);

        assert_eq!(event["event"], "lottery.draw_result");
        assert_eq!(event["scope"], "public");
        assert_eq!(event["data"]["lotteryId"], "txffc");
        assert_eq!(
            event["data"]["resultNumbers"],
            json!(["1", "2", "3", "4", "5"])
        );
    }
}
