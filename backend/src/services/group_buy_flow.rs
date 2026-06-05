//! 合买成单编排工具，负责把合买投注文本转换为当前订单引擎可识别的标准选号。

use crate::{
    domain::{
        group_buy::{GroupBuyPlan, GroupBuyPlanStatus},
        lottery::LotteryKind,
        order::{CreateOrderRequest, OrderDetail},
        play::{
            BigSmallOddEvenPick, BigSmallOddEvenPosition, DigitAttribute, PlayRuleCode,
            PlaySelection,
        },
    },
    error::{ApiError, ApiResult},
    services::{
        business_database::enum_from_string,
        draw::DrawRepository,
        group_buy::GroupBuyRepository,
        order::{validate_draw_issue_accepts_order, OrderRepository},
        play_rules::play_category_for_rule,
    },
};

/// 根据合买计划生成无需重复扣款的真实投注订单请求。
pub async fn build_group_buy_order_request(
    draws: &DrawRepository,
    orders: &OrderRepository,
    lottery: &LotteryKind,
    user_id: &str,
    issue: &str,
    rule_code: &str,
    numbers: &str,
    total_amount_minor: i64,
) -> ApiResult<CreateOrderRequest> {
    let issue = required_text(issue, "请选择合买期号")?;
    let rule_code = parse_group_buy_rule_code(rule_code)?;
    let selection = parse_group_buy_selection(&rule_code, numbers)?;
    let draw_issue = draws.get_by_lottery_issue(&lottery.id, &issue).await?;
    validate_draw_issue_accepts_order(&draw_issue, lottery, &issue)?;

    let probe = CreateOrderRequest {
        user_id: user_id.trim().to_string(),
        lottery_id: lottery.id.clone(),
        issue: issue.clone(),
        rule_code: rule_code.clone(),
        selection: selection.clone(),
        unit_amount_minor: 1,
    };
    let quote = orders.quote(lottery, &probe).await?;
    let stake_count = i64::from(quote.stake_count);
    if stake_count <= 0 {
        return Err(ApiError::BadRequest("合买投注内容没有有效注数".to_string()));
    }
    if total_amount_minor <= 0 {
        return Err(ApiError::BadRequest("合买总金额必须大于 0".to_string()));
    }
    if total_amount_minor % stake_count != 0 {
        return Err(ApiError::BadRequest(format!(
            "合买总金额必须能按 {stake_count} 注平均分配"
        )));
    }

    let unit_amount_minor = total_amount_minor / stake_count;
    if unit_amount_minor <= 0 {
        return Err(ApiError::BadRequest("合买单注金额必须大于 0".to_string()));
    }

    Ok(CreateOrderRequest {
        unit_amount_minor,
        ..probe
    })
}

/// 根据已经保存的合买计划生成真实投注订单请求。
pub async fn build_group_buy_order_request_from_plan(
    draws: &DrawRepository,
    orders: &OrderRepository,
    lottery: &LotteryKind,
    plan: &GroupBuyPlan,
) -> ApiResult<CreateOrderRequest> {
    build_group_buy_order_request(
        draws,
        orders,
        lottery,
        &plan.initiator_user_id,
        &plan.issue,
        &plan.rule_code,
        &plan.numbers,
        plan.total_amount_minor,
    )
    .await
}

/// 满单后生成真实投注订单，并把订单号回写到合买计划。
pub async fn create_order_for_filled_group_buy(
    draws: &DrawRepository,
    orders: &OrderRepository,
    group_buys: &GroupBuyRepository,
    lottery: &LotteryKind,
    plan: &GroupBuyPlan,
) -> ApiResult<Option<(OrderDetail, GroupBuyPlan)>> {
    if plan.status != GroupBuyPlanStatus::Filled || plan.order_id.is_some() {
        return Ok(None);
    }

    let payload = build_group_buy_order_request_from_plan(draws, orders, lottery, plan).await?;
    let order = orders.create(lottery, payload).await?;
    match group_buys.attach_order(&plan.id, &order.id).await {
        Ok(attached_plan) => Ok(Some((order, attached_plan))),
        Err(error) => {
            if let Err(rollback_error) = orders.remove_unfunded(&order.id).await {
                tracing::error!(
                    order_id = %order.id,
                    error = %rollback_error.log_message(),
                    "合买满单关联订单失败后移除未入账订单失败"
                );
            }
            Err(error)
        }
    }
}

/// 解析合买玩法编码为当前后端玩法枚举。
pub fn parse_group_buy_rule_code(rule_code: &str) -> ApiResult<PlayRuleCode> {
    enum_from_string(required_text(rule_code, "请选择合买玩法")?)
        .map_err(|_| ApiError::BadRequest("合买玩法编码无效".to_string()))
}

/// 将合买投注内容文本转换为标准选号结构。
pub fn parse_group_buy_selection(
    rule_code: &PlayRuleCode,
    numbers: &str,
) -> ApiResult<PlaySelection> {
    required_text(numbers, "请输入合买投注内容")?;
    match play_category_for_rule(rule_code) {
        crate::domain::lottery::PlayCategory::Direct => Ok(PlaySelection {
            positions: parse_direct_positions(numbers)?,
            ..PlaySelection::default()
        }),
        crate::domain::lottery::PlayCategory::DirectCombination
        | crate::domain::lottery::PlayCategory::GroupThree
        | crate::domain::lottery::PlayCategory::GroupSix
            if is_banker_rule(rule_code) =>
        {
            let segments = parse_position_segments(numbers);
            if segments.len() != 2 {
                return Err(ApiError::BadRequest(
                    "胆拖玩法请使用 胆码|拖码 的格式".to_string(),
                ));
            }
            Ok(PlaySelection {
                banker_numbers: parse_digit_list(&segments[0])?,
                drag_numbers: parse_digit_list(&segments[1])?,
                ..PlaySelection::default()
            })
        }
        crate::domain::lottery::PlayCategory::DirectCombination
        | crate::domain::lottery::PlayCategory::GroupThree
        | crate::domain::lottery::PlayCategory::GroupSix => Ok(PlaySelection {
            numbers: parse_digit_list(first_segment_or_all(numbers))?,
            ..PlaySelection::default()
        }),
        crate::domain::lottery::PlayCategory::BigSmallOddEven => Ok(PlaySelection {
            big_small_odd_even: parse_big_small_odd_even(numbers)?,
            ..PlaySelection::default()
        }),
    }
}

/// 解析直选位置选号，兼容 `1|2|3` 和单注 `1,2,3`。
fn parse_direct_positions(numbers: &str) -> ApiResult<Vec<Vec<u8>>> {
    let segments = parse_position_segments(numbers);
    if segments.len() == 3 {
        return segments
            .iter()
            .map(|segment| parse_digit_list(segment))
            .collect();
    }

    let digits = parse_digit_list(numbers)?;
    if digits.len() != 3 {
        return Err(ApiError::BadRequest(
            "直选玩法请使用 1|2|3 或 1,2,3 格式".to_string(),
        ));
    }
    Ok(digits.into_iter().map(|digit| vec![digit]).collect())
}

/// 解析大小单双选号，兼容 `big|odd` 与 `tens:big|ones:odd`。
fn parse_big_small_odd_even(numbers: &str) -> ApiResult<Vec<BigSmallOddEvenPick>> {
    let segments = parse_position_segments(numbers);
    if segments.is_empty() || segments.len() > 2 {
        return Err(ApiError::BadRequest(
            "大小单双请按 十位|个位 的格式选择".to_string(),
        ));
    }

    let mut picks = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let (position, values) = parse_big_small_segment(index, segment)?;
        picks.push(BigSmallOddEvenPick {
            position,
            attributes: values,
        });
    }
    Ok(picks)
}

/// 解析大小单双单个位置片段。
fn parse_big_small_segment(
    index: usize,
    segment: &str,
) -> ApiResult<(BigSmallOddEvenPosition, Vec<DigitAttribute>)> {
    let (position, value_text) = if let Some((position_text, values)) = segment.split_once(':') {
        (parse_big_small_position(position_text, index)?, values)
    } else if let Some((position_text, values)) = segment.split_once('：') {
        (parse_big_small_position(position_text, index)?, values)
    } else {
        (default_big_small_position(index)?, segment)
    };

    let attributes = split_tokens(value_text)
        .into_iter()
        .map(parse_digit_attribute)
        .collect::<ApiResult<Vec<_>>>()?;
    if attributes.is_empty() {
        return Err(ApiError::BadRequest("大小单双属性不能为空".to_string()));
    }
    Ok((position, attributes))
}

/// 将位置文本解析为大小单双支持的十位或个位。
fn parse_big_small_position(
    value: &str,
    fallback_index: usize,
) -> ApiResult<BigSmallOddEvenPosition> {
    match value.trim().to_ascii_lowercase().as_str() {
        "tens" | "ten" | "十位" => Ok(BigSmallOddEvenPosition::Tens),
        "ones" | "one" | "个位" => Ok(BigSmallOddEvenPosition::Ones),
        "" => default_big_small_position(fallback_index),
        _ => Err(ApiError::BadRequest(
            "大小单双位置只支持十位或个位".to_string(),
        )),
    }
}

/// 根据片段顺序返回大小单双默认位置。
fn default_big_small_position(index: usize) -> ApiResult<BigSmallOddEvenPosition> {
    match index {
        0 => Ok(BigSmallOddEvenPosition::Tens),
        1 => Ok(BigSmallOddEvenPosition::Ones),
        _ => Err(ApiError::BadRequest(
            "大小单双最多支持十位和个位".to_string(),
        )),
    }
}

/// 将属性文本解析为大小单双枚举。
fn parse_digit_attribute(value: &str) -> ApiResult<DigitAttribute> {
    match value.trim().to_ascii_lowercase().as_str() {
        "big" | "大" => Ok(DigitAttribute::Big),
        "small" | "小" => Ok(DigitAttribute::Small),
        "odd" | "single" | "单" => Ok(DigitAttribute::Odd),
        "even" | "双" => Ok(DigitAttribute::Even),
        _ => Err(ApiError::BadRequest(
            "大小单双属性只支持大、小、单、双".to_string(),
        )),
    }
}

/// 判断玩法是否是胆拖形态。
fn is_banker_rule(rule_code: &PlayRuleCode) -> bool {
    use PlayRuleCode::*;
    matches!(
        rule_code,
        ThreeGroupThreeBanker
            | ThreeGroupSixBanker
            | FiveFrontGroupThreeBanker
            | FiveMiddleGroupThreeBanker
            | FiveBackGroupThreeBanker
            | FiveFrontGroupSixBanker
            | FiveMiddleGroupSixBanker
            | FiveBackGroupSixBanker
    )
}

/// 读取第一个位置片段；没有位置分隔符时使用完整文本。
fn first_segment_or_all(numbers: &str) -> &str {
    numbers.split('|').next().unwrap_or(numbers)
}

/// 按位置分隔符拆分投注内容。
fn parse_position_segments(numbers: &str) -> Vec<String> {
    numbers
        .split('|')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// 解析数字列表，支持英文逗号、中文逗号和空白分隔。
fn parse_digit_list(value: &str) -> ApiResult<Vec<u8>> {
    let mut digits = Vec::new();
    for token in split_tokens(value) {
        if token.len() != 1 || !token.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ApiError::BadRequest(
                "投注号码必须是 0-9，并使用逗号或空格分隔".to_string(),
            ));
        }
        digits.push(token.as_bytes()[0] - b'0');
    }
    if digits.is_empty() {
        return Err(ApiError::BadRequest("投注号码不能为空".to_string()));
    }
    Ok(digits)
}

/// 按逗号或空白拆分输入 token。
fn split_tokens(value: &str) -> Vec<&str> {
    value
        .split(|character: char| character == ',' || character == '，' || character.is_whitespace())
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect()
}

/// 修剪并校验必填文本。
fn required_text(value: &str, message: &str) -> ApiResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ApiError::BadRequest(message.to_string()));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_direct_single_stake_with_commas() {
        let selection = parse_group_buy_selection(&PlayRuleCode::ThreeDirect, "1,2,3")
            .expect("direct selection can parse");

        assert_eq!(selection.positions, vec![vec![1], vec![2], vec![3]]);
    }

    #[test]
    fn parses_banker_drag_numbers() {
        let selection = parse_group_buy_selection(&PlayRuleCode::ThreeGroupSixBanker, "1|2,3,4")
            .expect("banker selection can parse");

        assert_eq!(selection.banker_numbers, vec![1]);
        assert_eq!(selection.drag_numbers, vec![2, 3, 4]);
    }

    #[test]
    fn parses_big_small_odd_even_numbers() {
        let selection =
            parse_group_buy_selection(&PlayRuleCode::FiveBigSmallOddEven, "tens:big|ones:odd")
                .expect("big small odd even selection can parse");

        assert_eq!(selection.big_small_odd_even.len(), 2);
        assert_eq!(
            selection.big_small_odd_even[0].position,
            BigSmallOddEvenPosition::Tens
        );
        assert_eq!(
            selection.big_small_odd_even[0].attributes,
            vec![DigitAttribute::Big]
        );
    }
}
