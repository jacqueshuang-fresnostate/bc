//! 开奖赔付风险池服务，使用 Redis ZSET 为避奖策略提供最低赔付候选号码索引。

use std::collections::BTreeMap;

use crate::{
    domain::{
        draw::DrawIssue,
        lottery::LotteryNumberType,
        order::{OrderDetail, OrderStatus},
        play::PlayRuleEvaluateRequest,
    },
    error::{ApiError, ApiResult},
    services::{
        draw::{draw_number_spec, format_draw_number},
        order::payout_amount_minor,
        play_rules::evaluate_play_rule,
        redis_runtime::RedisRuntime,
    },
};

const MAX_RISK_POOL_CANDIDATES: u64 = 200_000;
const RISK_POOL_TTL_SECONDS: usize = 20 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Redis 风险池返回的最低赔付候选号码。
pub(crate) struct DrawRiskCandidate {
    /// 候选开奖号码，使用英文逗号分隔。
    pub(crate) draw_number: String,
    /// 当前候选号码对应的预计派奖金额，单位为分。
    pub(crate) score_minor: i64,
}

/// 创建期号后初始化当前期号的 Redis 赔付风险池，所有合法候选号码初始分为 0。
pub(crate) async fn initialize_risk_pool(
    redis: &RedisRuntime,
    issue: &DrawIssue,
    avoid_winning_enabled: bool,
) -> ApiResult<()> {
    if !avoid_winning_enabled || !redis.is_enabled() {
        return Ok(());
    }

    let candidates = enumerable_draw_numbers(&issue.number_type)?;
    let key = risk_pool_key(&issue.lottery_id, &issue.issue);
    redis.zadd_nx_zero_members(&key, &candidates).await?;
    redis
        .expire_key_seconds(&key, RISK_POOL_TTL_SECONDS)
        .await?;
    tracing::info!(
        lottery_id = %issue.lottery_id,
        issue = %issue.issue,
        candidate_count = candidates.len(),
        "开奖赔付风险池已初始化"
    );
    Ok(())
}

/// 把待开奖订单的潜在派奖金额累加进当前期号风险池。
pub(crate) async fn add_order_risk(redis: &RedisRuntime, order: &OrderDetail) -> ApiResult<()> {
    apply_order_risk_delta(redis, order, 1).await
}

/// 从当前期号风险池中扣回待开奖订单的潜在派奖金额，用于订单取消。
pub(crate) async fn remove_order_risk(redis: &RedisRuntime, order: &OrderDetail) -> ApiResult<()> {
    apply_order_risk_delta(redis, order, -1).await
}

/// 读取当前期号最低赔付候选号码；Redis 未启用或风险池不存在时返回 None。
pub(crate) async fn lowest_risk_candidate(
    redis: &RedisRuntime,
    issue: &DrawIssue,
) -> ApiResult<Option<DrawRiskCandidate>> {
    if !redis.is_enabled() {
        return Ok(None);
    }
    let key = risk_pool_key(&issue.lottery_id, &issue.issue);
    let Some((draw_number, score_minor)) = redis.lowest_zset_member_random(&key).await? else {
        return Ok(None);
    };
    Ok(Some(DrawRiskCandidate {
        draw_number,
        score_minor,
    }))
}

/// 按彩种和期号构造风险池键名，保持所有链路使用同一格式。
fn risk_pool_key(lottery_id: &str, issue: &str) -> String {
    format!("draw:risk:{lottery_id}:{issue}")
}

/// 根据方向把订单风险写入或扣回 Redis，风险池不存在时跳过重计算。
async fn apply_order_risk_delta(
    redis: &RedisRuntime,
    order: &OrderDetail,
    direction: i64,
) -> ApiResult<()> {
    if !redis.is_enabled() || order.status != OrderStatus::PendingDraw {
        return Ok(());
    }
    let key = risk_pool_key(&order.lottery_id, &order.issue);
    if !redis.key_exists(&key).await? {
        return Ok(());
    }

    let risks = order_risk_increments(order)?
        .into_iter()
        .map(|(draw_number, score_minor)| (draw_number, score_minor * direction))
        .collect::<Vec<_>>();
    if direction < 0 {
        redis.zincrby_many_floor_zero(&key, &risks).await?;
    } else {
        redis.zincrby_many(&key, &risks).await?;
    }
    Ok(())
}

/// 计算一张订单对各候选开奖号码产生的潜在派奖风险。
fn order_risk_increments(order: &OrderDetail) -> ApiResult<BTreeMap<String, i64>> {
    let mut risks = BTreeMap::new();
    for draw_number in enumerable_draw_numbers(&order.number_type)? {
        let evaluation = evaluate_play_rule(PlayRuleEvaluateRequest {
            number_type: order.number_type.clone(),
            rule_code: order.rule_code.clone(),
            selection: order.selection.clone(),
            draw_number: draw_number.clone(),
        })?;
        if evaluation.matched_bets.is_empty() {
            continue;
        }
        let payout_minor = payout_amount_minor(
            evaluation.matched_bets.len(),
            order.odds_basis_points,
            order.unit_amount_minor,
        )?;
        if payout_minor > 0 {
            risks.insert(draw_number, payout_minor);
        }
    }

    Ok(risks)
}

/// 穷举当前号码类型的所有合法开奖号码；只支持有限且可接受的号码空间。
fn enumerable_draw_numbers(number_type: &LotteryNumberType) -> ApiResult<Vec<String>> {
    let spec = draw_number_spec(number_type);
    if spec.unique {
        return Err(ApiError::Conflict(
            "当前号码类型暂不支持开奖赔付风险池".to_string(),
        ));
    }

    let range = u64::from(spec.max - spec.min + 1);
    let total = range.pow(spec.len as u32);
    if total > MAX_RISK_POOL_CANDIDATES {
        return Err(ApiError::Conflict(
            "当前号码空间过大，暂不支持开奖赔付风险池".to_string(),
        ));
    }

    let mut candidates = Vec::with_capacity(total as usize);
    for index in 0..total {
        candidates.push(format_draw_number(&index_to_digits(
            index, spec.len, spec.min, range,
        )));
    }
    Ok(candidates)
}

/// 把穷举序号转换为开奖号码数字列表。
fn index_to_digits(mut index: u64, len: usize, min: u8, range: u64) -> Vec<u8> {
    let mut digits = vec![min; len];
    for position in (0..len).rev() {
        digits[position] = min + (index % range) as u8;
        index /= range;
    }
    digits
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::{
            lottery::LotteryNumberType,
            order::{OrderDetail, OrderSource},
            play::{PlayRuleCode, PlaySelection},
        },
        services::draw_risk::{enumerable_draw_numbers, order_risk_increments},
    };

    #[test]
    /// 验证避奖 Redis 风险池只保留 20 分钟，避免历史期号候选长期占用 Redis。
    fn risk_pool_ttl_is_twenty_minutes() {
        assert_eq!(super::RISK_POOL_TTL_SECONDS, 20 * 60);
    }

    #[test]
    /// 验证三位号码空间会完整初始化为 1000 个候选。
    fn enumerable_three_digit_draw_numbers_contains_full_space() {
        let candidates = enumerable_draw_numbers(&LotteryNumberType::ThreeDigit).unwrap();

        assert_eq!(candidates.len(), 1_000);
        assert_eq!(candidates.first().map(String::as_str), Some("0,0,0"));
        assert_eq!(candidates.last().map(String::as_str), Some("9,9,9"));
    }

    #[test]
    /// 验证直选订单只会给命中的候选开奖号码累加预计派奖金额。
    fn order_risk_increments_use_payout_amount_for_matching_candidates() {
        let order = OrderDetail {
            id: "O000000000001".to_string(),
            order_source: OrderSource::Direct,
            user_id: "U10001".to_string(),
            lottery_id: "fc3d".to_string(),
            lottery_name: "福彩3D".to_string(),
            issue: "20260626001".to_string(),
            rule_code: PlayRuleCode::ThreeDirect,
            number_type: LotteryNumberType::ThreeDigit,
            selection: PlaySelection {
                positions: vec![vec![1], vec![2], vec![3]],
                ..PlaySelection::default()
            },
            stake_count: 1,
            unit_amount_minor: 200,
            amount_minor: 200,
            odds_basis_points: 100_000,
            expanded_bets: vec!["1,2,3".to_string()],
            draw_number: None,
            matched_bets: Vec::new(),
            payout_minor: 0,
            status: crate::domain::order::OrderStatus::PendingDraw,
            settled_at: None,
            created_at: "unix:1".to_string(),
        };

        let risks = order_risk_increments(&order).unwrap();

        assert_eq!(risks.len(), 1);
        assert_eq!(risks.get("1,2,3"), Some(&1_000));
    }
}
