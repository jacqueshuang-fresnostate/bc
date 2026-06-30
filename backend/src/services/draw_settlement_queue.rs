//! 开奖结算 Redis 队列，负责把已开奖期号异步交给派奖消费者

use crate::{domain::draw::DrawIssue, error::ApiResult, services::redis_runtime::RedisRuntime};

const DRAW_SETTLEMENT_QUEUE_KEY: &str = "draw:settlement:queue";
const DRAW_SETTLEMENT_DEDUPE_PREFIX: &str = "draw:settlement:queued:";
const DRAW_SETTLEMENT_DEDUPE_TTL_SECONDS: usize = 10 * 60;

/// 将已开奖期号加入待结算队列；Redis 未启用时返回 false，调用方应回退同步结算。
pub async fn enqueue(redis: &RedisRuntime, issue: &DrawIssue) -> ApiResult<bool> {
    redis
        .enqueue_unique_list_item(
            DRAW_SETTLEMENT_QUEUE_KEY,
            &dedupe_key(&issue.id),
            &issue.id,
            DRAW_SETTLEMENT_DEDUPE_TTL_SECONDS,
        )
        .await
}

/// 从待结算队列弹出一批期号 ID，并释放对应去重键，便于失败后重新入队。
pub async fn pop_batch(redis: &RedisRuntime, limit: usize) -> ApiResult<Vec<String>> {
    let issue_ids = redis
        .lpop_list_items(DRAW_SETTLEMENT_QUEUE_KEY, limit)
        .await?;
    if !issue_ids.is_empty() {
        let dedupe_keys = issue_ids
            .iter()
            .map(|issue_id| dedupe_key(issue_id))
            .collect::<Vec<_>>();
        redis.delete_keys(&dedupe_keys).await?;
    }
    Ok(issue_ids)
}

fn dedupe_key(issue_id: &str) -> String {
    format!("{DRAW_SETTLEMENT_DEDUPE_PREFIX}{issue_id}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    /// Redis 未启用时队列写入返回 false，调用方可以继续同步结算。
    async fn disabled_redis_queue_is_a_noop() {
        let redis = RedisRuntime::disabled();
        let issue = DrawIssue {
            id: "D202607010001".to_string(),
            lottery_id: "fc3d".to_string(),
            lottery_name: "福彩3D".to_string(),
            issue: "202607010001".to_string(),
            number_type: crate::domain::lottery::LotteryNumberType::ThreeDigit,
            draw_mode: crate::domain::lottery::DrawMode::Platform,
            scheduled_at: "2026-07-01 00:55:00".to_string(),
            sale_closed_at: "2026-07-01 00:54:59".to_string(),
            status: crate::domain::draw::DrawIssueStatus::Drawn,
            draw_number: Some("1,2,3".to_string()),
            drawn_at: Some("2026-07-01 00:55:00".to_string()),
            created_at: "2026-07-01 00:50:00".to_string(),
        };

        assert!(!enqueue(&redis, &issue).await.expect("disabled redis ok"));
        assert!(pop_batch(&redis, 10)
            .await
            .expect("disabled redis pop ok")
            .is_empty());
    }
}
