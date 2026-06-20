//! 开奖避开中奖策略服务，负责在开奖落库前生成不命中当前待开奖注单的开奖号码。

use crate::{
    domain::{
        draw::{DrawIssue, DrawIssueResultRequest},
        lottery::{DrawMode, LotteryKind, LotteryNumberType},
        order::OrderDetail,
        play::PlayRuleEvaluateRequest,
    },
    error::{ApiError, ApiResult},
    services::{
        draw::{
            draw_number_digits, draw_number_spec, format_draw_number, generated_draw_number,
            normalize_draw_number, DrawRepository,
        },
        order::OrderRepository,
        play_rules::evaluate_play_rule,
    },
};

/// 按彩种避开中奖开关执行普通开奖，必要时把开奖号码替换为不会命中当前注单的候选号码。
pub async fn draw_with_avoid_winning_policy(
    draws: &DrawRepository,
    orders: &OrderRepository,
    lottery: &LotteryKind,
    id: &str,
    payload: DrawIssueResultRequest,
) -> ApiResult<DrawIssue> {
    let (payload, uses_control_number) = draws.resolve_draw_payload(id, payload).await?;
    let issue = draws.get(id).await?;
    let (payload, uses_control_number) =
        apply_avoid_winning_policy(orders, lottery, &issue, payload, uses_control_number).await?;
    draws
        .draw_resolved_payload(id, payload, uses_control_number)
        .await
}

/// 按彩种避开中奖开关执行预取 API 开奖，必要时把 API 号码替换为不会命中的候选号码。
pub async fn draw_prefetched_api_with_avoid_winning_policy(
    draws: &DrawRepository,
    orders: &OrderRepository,
    lottery: &LotteryKind,
    id: &str,
    api_draw_number: Option<String>,
    allow_control_number: bool,
) -> ApiResult<DrawIssue> {
    let (payload, uses_control_number) = draws
        .resolve_prefetched_api_draw_payload(id, api_draw_number, allow_control_number)
        .await?;
    let issue = draws.get(id).await?;
    let (payload, uses_control_number) =
        apply_avoid_winning_policy(orders, lottery, &issue, payload, uses_control_number).await?;
    draws
        .draw_resolved_payload(id, payload, uses_control_number)
        .await
}

/// 根据避开中奖开关和当前待开奖订单，决定是否调整最终开奖号码。
async fn apply_avoid_winning_policy(
    orders: &OrderRepository,
    lottery: &LotteryKind,
    issue: &DrawIssue,
    payload: DrawIssueResultRequest,
    uses_control_number: bool,
) -> ApiResult<(DrawIssueResultRequest, bool)> {
    if !lottery.avoid_winning_enabled {
        return Ok((payload, uses_control_number));
    }

    let pending_orders = orders
        .list_pending_for_issue(&issue.lottery_id, &issue.issue)
        .await?;
    if pending_orders.is_empty() {
        return Ok((payload, uses_control_number));
    }

    let current_draw_number = resolved_draw_number(issue, &payload, uses_control_number)?;
    if !candidate_hits_any_order(&pending_orders, &current_draw_number)? {
        return Ok((payload, uses_control_number));
    }

    let replacement =
        find_non_winning_draw_number(&issue.number_type, &current_draw_number, &pending_orders)?
            .ok_or_else(|| {
                ApiError::Conflict(
                    "当前期号投注已覆盖所有开奖号码，无法生成避开中奖号码".to_string(),
                )
            })?;
    let uses_replacement_for_platform =
        uses_control_number || issue.draw_mode == DrawMode::Platform;

    tracing::warn!(
        lottery_id = %issue.lottery_id,
        lottery_name = %issue.lottery_name,
        issue = %issue.issue,
        order_count = pending_orders.len(),
        original_draw_number = %current_draw_number,
        replacement_draw_number = %replacement,
        "彩种自动避开中奖已调整开奖号码"
    );

    Ok((
        DrawIssueResultRequest {
            draw_number: Some(replacement),
        },
        uses_replacement_for_platform,
    ))
}

/// 按开奖模式还原当前载荷真正会写入的开奖号码，并转换为标准逗号格式。
fn resolved_draw_number(
    issue: &DrawIssue,
    payload: &DrawIssueResultRequest,
    uses_control_number: bool,
) -> ApiResult<String> {
    let draw_number = match issue.draw_mode {
        DrawMode::Manual => payload
            .draw_number
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ApiError::BadRequest("手动开奖需要填写开奖号码".to_string()))?
            .to_string(),
        DrawMode::Platform if uses_control_number => payload
            .draw_number
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ApiError::BadRequest("控制开奖需要填写开奖号码".to_string()))?
            .to_string(),
        DrawMode::Platform => {
            generated_draw_number(&issue.number_type, &issue.lottery_id, &issue.issue)
        }
        DrawMode::Api => payload
            .draw_number
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or_else(|| {
                generated_draw_number(&issue.number_type, &issue.lottery_id, &issue.issue)
            }),
    };

    normalize_draw_number(&draw_number, &issue.number_type)
}

/// 判断候选开奖号码是否会让任意待开奖订单中奖。
fn candidate_hits_any_order(orders: &[OrderDetail], draw_number: &str) -> ApiResult<bool> {
    for order in orders {
        let evaluation = evaluate_play_rule(PlayRuleEvaluateRequest {
            number_type: order.number_type.clone(),
            rule_code: order.rule_code.clone(),
            selection: order.selection.clone(),
            draw_number: draw_number.to_string(),
        })?;
        if !evaluation.matched_bets.is_empty() {
            return Ok(true);
        }
    }

    Ok(false)
}

/// 穷举当前号码类型的候选号码，返回第一组不会命中任何订单的开奖号码。
fn find_non_winning_draw_number(
    number_type: &LotteryNumberType,
    current_draw_number: &str,
    orders: &[OrderDetail],
) -> ApiResult<Option<String>> {
    let spec = draw_number_spec(number_type);
    if spec.unique {
        return Err(ApiError::Conflict(
            "当前号码类型暂不支持自动避开中奖".to_string(),
        ));
    }

    let range = u64::from(spec.max - spec.min + 1);
    let total = range.pow(spec.len as u32);
    if total > 200_000 {
        return Err(ApiError::Conflict(
            "当前号码空间过大，暂不支持自动避开中奖".to_string(),
        ));
    }

    let current_digits = draw_number_digits(current_draw_number, number_type)?;
    let start_index = digits_to_index(&current_digits, spec.min, range)?;
    for offset in 1..=total {
        let index = (start_index + offset) % total;
        let candidate = format_draw_number(&index_to_digits(index, spec.len, spec.min, range));
        if !candidate_hits_any_order(orders, &candidate)? {
            return Ok(Some(candidate));
        }
    }

    Ok(None)
}

/// 把当前开奖号码转换为穷举起点，方便从原始号码附近继续查找候选。
fn digits_to_index(digits: &[u8], min: u8, range: u64) -> ApiResult<u64> {
    let mut index = 0u64;
    for digit in digits {
        let value = digit
            .checked_sub(min)
            .ok_or_else(|| ApiError::BadRequest("开奖号码数字范围无效".to_string()))?;
        index = index
            .checked_mul(range)
            .and_then(|base| base.checked_add(u64::from(value)))
            .ok_or_else(|| ApiError::BadRequest("开奖号码空间过大".to_string()))?;
    }

    Ok(index)
}

/// 把穷举序号转换回开奖号码数字列表。
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
            draw::{CreateDrawIssueRequest, DrawIssueResultRequest},
            lottery::{
                DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType,
                LotteryPlayConfig, PlayCategory,
            },
            order::CreateOrderRequest,
            play::{PlayRuleCode, PlaySelection},
        },
        services::{
            draw::{generated_draw_number, DrawRepository},
            draw_avoidance::draw_with_avoid_winning_policy,
            order::OrderRepository,
        },
    };

    #[tokio::test]
    /// 验证开启避开中奖后，平台开奖会避开当前期号的中奖投注。
    async fn avoid_winning_policy_replaces_platform_number_when_order_would_win() {
        let mut lottery = lottery_fixture();
        lottery.avoid_winning_enabled = true;
        let draws = DrawRepository::memory();
        let orders = OrderRepository::memory();
        let issue = draws
            .create(
                &lottery,
                CreateDrawIssueRequest {
                    lottery_id: lottery.id.clone(),
                    issue: "202606200001".to_string(),
                    scheduled_at: "2026-06-20 10:00:00".to_string(),
                    sale_closed_at: "2026-06-20 09:59:59".to_string(),
                },
            )
            .await
            .expect("可以创建开奖期");
        let generated = generated_draw_number(&lottery.number_type, &lottery.id, &issue.issue);
        let digits = crate::services::draw::draw_number_digits(&generated, &lottery.number_type)
            .expect("平台号码可以解析");

        orders
            .create(
                &lottery,
                CreateOrderRequest {
                    user_id: "U10001".to_string(),
                    lottery_id: lottery.id.clone(),
                    issue: issue.issue.clone(),
                    rule_code: PlayRuleCode::ThreeDirect,
                    selection: PlaySelection {
                        positions: vec![vec![digits[0]], vec![digits[1]], vec![digits[2]]],
                        ..PlaySelection::default()
                    },
                    unit_amount_minor: 200,
                },
            )
            .await
            .expect("可以创建待开奖订单");

        let drawn = draw_with_avoid_winning_policy(
            &draws,
            &orders,
            &lottery,
            &issue.id,
            DrawIssueResultRequest::default(),
        )
        .await
        .expect("可以执行避奖开奖");

        assert_ne!(drawn.draw_number.as_deref(), Some(generated.as_str()));
    }

    /// 构造测试彩种，启用三位直选玩法。
    fn lottery_fixture() -> LotteryKind {
        LotteryKind {
            id: "avoid-test".to_string(),
            name: "避奖测试彩".to_string(),
            category: "test".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Platform,
            api_draw_delay_seconds: 0,
            draw_control_enabled: true,
            avoid_winning_enabled: false,
            issue_format: crate::domain::lottery::DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
            sale_close_lead_seconds: crate::domain::lottery::DEFAULT_SALE_CLOSE_LEAD_SECONDS,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: false,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 100,
            },
            play_categories: vec![PlayCategory::Direct],
            play_configs: vec![LotteryPlayConfig {
                rule_code: PlayRuleCode::ThreeDirect,
                enabled: true,
                odds_basis_points: 100_000,
                position_select_limits: Vec::new(),
            }],
        }
    }
}
