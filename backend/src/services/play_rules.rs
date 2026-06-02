use crate::{
    domain::{
        lottery::LotteryNumberType,
        play::{
            BigSmallOddEvenPick, BigSmallOddEvenPosition, DigitAttribute, PlayRuleCode,
            PlayRuleEvaluateRequest, PlayRuleEvaluation, PlayRuleSummary, PlaySelection,
            ThreeDigitWindow,
        },
    },
    error::{ApiError, ApiResult},
};

pub fn play_rule_summaries() -> Vec<PlayRuleSummary> {
    use LotteryNumberType::{FiveDigit, ThreeDigit};
    use PlayRuleCode::*;
    use ThreeDigitWindow::{Back, Front, Full, Middle};

    vec![
        summary(
            ThreeDirect,
            "3 位直选",
            ThreeDigit,
            Full,
            "按百位、十位、个位顺序完全匹配",
        ),
        summary(
            ThreeGroupThree,
            "3 位组三复式",
            ThreeDigit,
            Full,
            "两个数字相同、一个数字不同，顺序不限",
        ),
        summary(
            ThreeGroupThreeBanker,
            "3 位组三胆拖",
            ThreeDigit,
            Full,
            "1 个胆码与拖码组成组三号码",
        ),
        summary(
            ThreeGroupSix,
            "3 位组六复式",
            ThreeDigit,
            Full,
            "三个数字全部不同，顺序不限",
        ),
        summary(
            ThreeGroupSixBanker,
            "3 位组六胆拖",
            ThreeDigit,
            Full,
            "1-2 个胆码与拖码补足 3 个不同数字",
        ),
        summary(
            FiveFrontDirect,
            "前 3 直选",
            FiveDigit,
            Front,
            "第 1-3 位按位完全匹配",
        ),
        summary(
            FiveMiddleDirect,
            "中 3 直选",
            FiveDigit,
            Middle,
            "第 2-4 位按位完全匹配",
        ),
        summary(
            FiveBackDirect,
            "后 3 直选",
            FiveDigit,
            Back,
            "第 3-5 位按位完全匹配",
        ),
        summary(
            FiveFrontDirectCombination,
            "前 3 直选组合",
            FiveDigit,
            Front,
            "从不重复数字生成前三直选排列",
        ),
        summary(
            FiveMiddleDirectCombination,
            "中 3 直选组合",
            FiveDigit,
            Middle,
            "从不重复数字生成中三直选排列",
        ),
        summary(
            FiveBackDirectCombination,
            "后 3 直选组合",
            FiveDigit,
            Back,
            "从不重复数字生成后三直选排列",
        ),
        summary(
            FiveFrontGroupThree,
            "前 3 组三复式",
            FiveDigit,
            Front,
            "前三为组三形态且数字命中",
        ),
        summary(
            FiveMiddleGroupThree,
            "中 3 组三复式",
            FiveDigit,
            Middle,
            "中三为组三形态且数字命中",
        ),
        summary(
            FiveBackGroupThree,
            "后 3 组三复式",
            FiveDigit,
            Back,
            "后三为组三形态且数字命中",
        ),
        summary(
            FiveFrontGroupThreeBanker,
            "前 3 组三胆拖",
            FiveDigit,
            Front,
            "胆码与拖码组成前三组三号码",
        ),
        summary(
            FiveMiddleGroupThreeBanker,
            "中 3 组三胆拖",
            FiveDigit,
            Middle,
            "胆码与拖码组成中三组三号码",
        ),
        summary(
            FiveBackGroupThreeBanker,
            "后 3 组三胆拖",
            FiveDigit,
            Back,
            "胆码与拖码组成后三组三号码",
        ),
        summary(
            FiveFrontGroupSix,
            "前 3 组六复式",
            FiveDigit,
            Front,
            "前三为组六形态且数字命中",
        ),
        summary(
            FiveMiddleGroupSix,
            "中 3 组六复式",
            FiveDigit,
            Middle,
            "中三为组六形态且数字命中",
        ),
        summary(
            FiveBackGroupSix,
            "后 3 组六复式",
            FiveDigit,
            Back,
            "后三为组六形态且数字命中",
        ),
        summary(
            FiveFrontGroupSixBanker,
            "前 3 组六胆拖",
            FiveDigit,
            Front,
            "胆码与拖码组成前三组六号码",
        ),
        summary(
            FiveMiddleGroupSixBanker,
            "中 3 组六胆拖",
            FiveDigit,
            Middle,
            "胆码与拖码组成中三组六号码",
        ),
        summary(
            FiveBackGroupSixBanker,
            "后 3 组六胆拖",
            FiveDigit,
            Back,
            "胆码与拖码组成后三组六号码",
        ),
        summary(
            FiveBigSmallOddEven,
            "大小单双",
            FiveDigit,
            Back,
            "默认按后两位判断大小和单双属性",
        ),
    ]
}

pub fn evaluate_play_rule(request: PlayRuleEvaluateRequest) -> ApiResult<PlayRuleEvaluation> {
    validate_draw_number(&request.draw_number, &request.number_type)?;

    let rule_number_type = number_type_for_rule(&request.rule_code);
    if rule_number_type != request.number_type {
        return Err(ApiError::BadRequest(
            "rule code does not match number type".to_string(),
        ));
    }

    let draw_digits = digits_from_string(&request.draw_number)?;
    let window = draw_window(&draw_digits, window_for_rule(&request.rule_code))?;
    let expanded_bets = expand_bets(&request.rule_code, &request.selection)?;
    let matched_bets = match_bets(
        &request.rule_code,
        &request.selection,
        &expanded_bets,
        &window,
    )?;
    let stake_count = expanded_bets.len() as u32;

    Ok(PlayRuleEvaluation {
        rule_code: request.rule_code,
        stake_count,
        expanded_bets,
        is_winning: !matched_bets.is_empty(),
        matched_bets,
    })
}

fn summary(
    code: PlayRuleCode,
    label: &str,
    number_type: LotteryNumberType,
    window: ThreeDigitWindow,
    description: &str,
) -> PlayRuleSummary {
    PlayRuleSummary {
        code,
        label: label.to_string(),
        number_type,
        window,
        description: description.to_string(),
    }
}

fn expand_bets(rule_code: &PlayRuleCode, selection: &PlaySelection) -> ApiResult<Vec<String>> {
    use PlayRuleCode::*;

    match rule_code {
        ThreeDirect | FiveFrontDirect | FiveMiddleDirect | FiveBackDirect => {
            expand_direct(&selection.positions)
        }
        FiveFrontDirectCombination | FiveMiddleDirectCombination | FiveBackDirectCombination => {
            expand_direct_combination(&selection.numbers)
        }
        ThreeGroupThree | FiveFrontGroupThree | FiveMiddleGroupThree | FiveBackGroupThree => {
            expand_group_three(&selection.numbers)
        }
        ThreeGroupThreeBanker
        | FiveFrontGroupThreeBanker
        | FiveMiddleGroupThreeBanker
        | FiveBackGroupThreeBanker => {
            expand_group_three_banker(&selection.banker_numbers, &selection.drag_numbers)
        }
        ThreeGroupSix | FiveFrontGroupSix | FiveMiddleGroupSix | FiveBackGroupSix => {
            expand_group_six(&selection.numbers)
        }
        ThreeGroupSixBanker
        | FiveFrontGroupSixBanker
        | FiveMiddleGroupSixBanker
        | FiveBackGroupSixBanker => {
            expand_group_six_banker(&selection.banker_numbers, &selection.drag_numbers)
        }
        FiveBigSmallOddEven => expand_big_small_odd_even(&selection.big_small_odd_even),
    }
}

fn match_bets(
    rule_code: &PlayRuleCode,
    selection: &PlaySelection,
    expanded_bets: &[String],
    window: &[u8],
) -> ApiResult<Vec<String>> {
    use PlayRuleCode::*;

    match rule_code {
        ThreeDirect | FiveFrontDirect | FiveMiddleDirect | FiveBackDirect => {
            let draw = digits_to_string(window);
            Ok(expanded_bets
                .iter()
                .filter(|bet| *bet == &draw)
                .cloned()
                .collect())
        }
        FiveFrontDirectCombination | FiveMiddleDirectCombination | FiveBackDirectCombination => {
            let draw = digits_to_string(window);
            Ok(expanded_bets
                .iter()
                .filter(|bet| *bet == &draw)
                .cloned()
                .collect())
        }
        ThreeGroupThree | FiveFrontGroupThree | FiveMiddleGroupThree | FiveBackGroupThree => Ok(
            match_grouped_bets(expanded_bets, window, DigitShape::GroupThree),
        ),
        ThreeGroupThreeBanker
        | FiveFrontGroupThreeBanker
        | FiveMiddleGroupThreeBanker
        | FiveBackGroupThreeBanker => Ok(match_grouped_bets(
            expanded_bets,
            window,
            DigitShape::GroupThree,
        )),
        ThreeGroupSix | FiveFrontGroupSix | FiveMiddleGroupSix | FiveBackGroupSix => Ok(
            match_grouped_bets(expanded_bets, window, DigitShape::GroupSix),
        ),
        ThreeGroupSixBanker
        | FiveFrontGroupSixBanker
        | FiveMiddleGroupSixBanker
        | FiveBackGroupSixBanker => Ok(match_grouped_bets(
            expanded_bets,
            window,
            DigitShape::GroupSix,
        )),
        FiveBigSmallOddEven => Ok(match_big_small_odd_even(
            &selection.big_small_odd_even,
            window,
        )),
    }
}

fn expand_direct(positions: &[Vec<u8>]) -> ApiResult<Vec<String>> {
    if positions.len() != 3 {
        return Err(ApiError::BadRequest(
            "direct play requires three position selections".to_string(),
        ));
    }

    let positions = positions
        .iter()
        .map(|digits| normalized_digits(digits))
        .collect::<ApiResult<Vec<_>>>()?;

    let mut bets = Vec::new();
    for first in &positions[0] {
        for second in &positions[1] {
            for third in &positions[2] {
                bets.push(format!("{first}{second}{third}"));
            }
        }
    }
    Ok(bets)
}

fn expand_direct_combination(numbers: &[u8]) -> ApiResult<Vec<String>> {
    let numbers = normalized_digits(numbers)?;
    if numbers.len() < 3 {
        return Err(ApiError::BadRequest(
            "direct combination requires at least three digits".to_string(),
        ));
    }

    let mut bets = Vec::new();
    for first in &numbers {
        for second in &numbers {
            for third in &numbers {
                if first != second && first != third && second != third {
                    bets.push(format!("{first}{second}{third}"));
                }
            }
        }
    }
    Ok(bets)
}

fn expand_group_three(numbers: &[u8]) -> ApiResult<Vec<String>> {
    let numbers = normalized_digits(numbers)?;
    if numbers.len() < 2 {
        return Err(ApiError::BadRequest(
            "group three requires at least two digits".to_string(),
        ));
    }

    let mut bets = Vec::new();
    for repeated in &numbers {
        for single in &numbers {
            if repeated != single {
                bets.push(format!("{repeated}{repeated}{single}"));
            }
        }
    }
    Ok(bets)
}

fn expand_group_three_banker(bankers: &[u8], drags: &[u8]) -> ApiResult<Vec<String>> {
    let bankers = normalized_digits(bankers)?;
    let drags = normalized_digits(drags)?;
    ensure_no_overlap(&bankers, &drags)?;
    if bankers.len() != 1 {
        return Err(ApiError::BadRequest(
            "group three banker play requires exactly one banker digit".to_string(),
        ));
    }
    if drags.is_empty() {
        return Err(ApiError::BadRequest(
            "group three banker play requires drag digits".to_string(),
        ));
    }

    let banker = bankers[0];
    let mut bets = Vec::new();
    for drag in &drags {
        bets.push(format!("{banker}{banker}{drag}"));
        bets.push(format!("{drag}{drag}{banker}"));
    }
    Ok(bets)
}

fn expand_group_six(numbers: &[u8]) -> ApiResult<Vec<String>> {
    let numbers = normalized_digits(numbers)?;
    if numbers.len() < 3 {
        return Err(ApiError::BadRequest(
            "group six requires at least three digits".to_string(),
        ));
    }
    Ok(combinations(&numbers, 3)
        .into_iter()
        .map(|digits| digits_to_string(&digits))
        .collect())
}

fn expand_group_six_banker(bankers: &[u8], drags: &[u8]) -> ApiResult<Vec<String>> {
    let bankers = normalized_digits(bankers)?;
    let drags = normalized_digits(drags)?;
    ensure_no_overlap(&bankers, &drags)?;
    if bankers.is_empty() || bankers.len() > 2 {
        return Err(ApiError::BadRequest(
            "group six banker play requires one or two banker digits".to_string(),
        ));
    }

    let drag_count = 3usize.saturating_sub(bankers.len());
    if drags.len() < drag_count {
        return Err(ApiError::BadRequest(
            "group six banker play does not have enough drag digits".to_string(),
        ));
    }

    Ok(combinations(&drags, drag_count)
        .into_iter()
        .map(|mut digits| {
            let mut combined = bankers.clone();
            combined.append(&mut digits);
            combined.sort_unstable();
            digits_to_string(&combined)
        })
        .collect())
}

fn expand_big_small_odd_even(picks: &[BigSmallOddEvenPick]) -> ApiResult<Vec<String>> {
    if picks.is_empty() {
        return Err(ApiError::BadRequest(
            "big small odd even requires at least one pick".to_string(),
        ));
    }

    let mut bets = Vec::new();
    for pick in picks {
        if pick.attributes.is_empty() {
            return Err(ApiError::BadRequest(
                "big small odd even pick requires attributes".to_string(),
            ));
        }
        for attribute in &pick.attributes {
            bets.push(format!(
                "{}:{}",
                big_small_position_code(&pick.position),
                digit_attribute_code(attribute)
            ));
        }
    }
    Ok(bets)
}

fn match_grouped_bets(
    expanded_bets: &[String],
    window: &[u8],
    expected_shape: DigitShape,
) -> Vec<String> {
    if digit_shape(window) != expected_shape {
        return Vec::new();
    }

    let draw_key = sorted_digits_key(window);
    expanded_bets
        .iter()
        .filter(|bet| {
            digits_from_string(bet)
                .map(|digits| sorted_digits_key(&digits) == draw_key)
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

fn match_big_small_odd_even(picks: &[BigSmallOddEvenPick], window: &[u8]) -> Vec<String> {
    let tens = window[1];
    let ones = window[2];
    let mut matches = Vec::new();

    for pick in picks {
        let digit = match pick.position {
            BigSmallOddEvenPosition::Tens => tens,
            BigSmallOddEvenPosition::Ones => ones,
        };
        for attribute in &pick.attributes {
            if digit_matches_attribute(digit, attribute) {
                matches.push(format!(
                    "{}:{}",
                    big_small_position_code(&pick.position),
                    digit_attribute_code(attribute)
                ));
            }
        }
    }

    matches
}

fn validate_draw_number(draw_number: &str, number_type: &LotteryNumberType) -> ApiResult<()> {
    let expected_len = match number_type {
        LotteryNumberType::ThreeDigit => 3,
        LotteryNumberType::FiveDigit => 5,
    };

    if draw_number.len() != expected_len {
        return Err(ApiError::BadRequest(format!(
            "draw number must be {expected_len} digits"
        )));
    }

    if !draw_number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "draw number must contain digits only".to_string(),
        ));
    }

    Ok(())
}

fn digits_from_string(value: &str) -> ApiResult<Vec<u8>> {
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(ApiError::BadRequest(
            "number value must contain digits only".to_string(),
        ));
    }
    Ok(value.bytes().map(|byte| byte - b'0').collect())
}

fn normalized_digits(digits: &[u8]) -> ApiResult<Vec<u8>> {
    if digits.is_empty() {
        return Err(ApiError::BadRequest(
            "digit selection cannot be empty".to_string(),
        ));
    }
    if digits.iter().any(|digit| *digit > 9) {
        return Err(ApiError::BadRequest(
            "digit selection must be between 0 and 9".to_string(),
        ));
    }

    let mut normalized = digits.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    Ok(normalized)
}

fn ensure_no_overlap(left: &[u8], right: &[u8]) -> ApiResult<()> {
    if left.iter().any(|digit| right.contains(digit)) {
        return Err(ApiError::BadRequest(
            "banker digits and drag digits cannot overlap".to_string(),
        ));
    }
    Ok(())
}

fn draw_window(draw_digits: &[u8], window: ThreeDigitWindow) -> ApiResult<Vec<u8>> {
    match window {
        ThreeDigitWindow::Full if draw_digits.len() == 3 => Ok(draw_digits.to_vec()),
        ThreeDigitWindow::Front if draw_digits.len() == 5 => Ok(draw_digits[0..3].to_vec()),
        ThreeDigitWindow::Middle if draw_digits.len() == 5 => Ok(draw_digits[1..4].to_vec()),
        ThreeDigitWindow::Back if draw_digits.len() == 5 => Ok(draw_digits[2..5].to_vec()),
        _ => Err(ApiError::BadRequest(
            "draw number length does not match play window".to_string(),
        )),
    }
}

fn window_for_rule(rule_code: &PlayRuleCode) -> ThreeDigitWindow {
    use PlayRuleCode::*;
    match rule_code {
        ThreeDirect
        | ThreeGroupThree
        | ThreeGroupThreeBanker
        | ThreeGroupSix
        | ThreeGroupSixBanker => ThreeDigitWindow::Full,
        FiveFrontDirect
        | FiveFrontDirectCombination
        | FiveFrontGroupThree
        | FiveFrontGroupThreeBanker
        | FiveFrontGroupSix
        | FiveFrontGroupSixBanker => ThreeDigitWindow::Front,
        FiveMiddleDirect
        | FiveMiddleDirectCombination
        | FiveMiddleGroupThree
        | FiveMiddleGroupThreeBanker
        | FiveMiddleGroupSix
        | FiveMiddleGroupSixBanker => ThreeDigitWindow::Middle,
        FiveBackDirect
        | FiveBackDirectCombination
        | FiveBackGroupThree
        | FiveBackGroupThreeBanker
        | FiveBackGroupSix
        | FiveBackGroupSixBanker
        | FiveBigSmallOddEven => ThreeDigitWindow::Back,
    }
}

fn number_type_for_rule(rule_code: &PlayRuleCode) -> LotteryNumberType {
    match rule_code {
        PlayRuleCode::ThreeDirect
        | PlayRuleCode::ThreeGroupThree
        | PlayRuleCode::ThreeGroupThreeBanker
        | PlayRuleCode::ThreeGroupSix
        | PlayRuleCode::ThreeGroupSixBanker => LotteryNumberType::ThreeDigit,
        _ => LotteryNumberType::FiveDigit,
    }
}

fn digits_to_string(digits: &[u8]) -> String {
    digits
        .iter()
        .map(|digit| char::from(b'0' + *digit))
        .collect()
}

fn combinations(digits: &[u8], count: usize) -> Vec<Vec<u8>> {
    fn walk(
        digits: &[u8],
        count: usize,
        start: usize,
        current: &mut Vec<u8>,
        output: &mut Vec<Vec<u8>>,
    ) {
        if current.len() == count {
            output.push(current.clone());
            return;
        }

        for index in start..digits.len() {
            current.push(digits[index]);
            walk(digits, count, index + 1, current, output);
            current.pop();
        }
    }

    let mut output = Vec::new();
    walk(digits, count, 0, &mut Vec::new(), &mut output);
    output
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DigitShape {
    Triple,
    GroupThree,
    GroupSix,
}

fn digit_shape(digits: &[u8]) -> DigitShape {
    let mut sorted = digits.to_vec();
    sorted.sort_unstable();
    if sorted[0] == sorted[2] {
        DigitShape::Triple
    } else if sorted[0] == sorted[1] || sorted[1] == sorted[2] {
        DigitShape::GroupThree
    } else {
        DigitShape::GroupSix
    }
}

fn sorted_digits_key(digits: &[u8]) -> String {
    let mut sorted = digits.to_vec();
    sorted.sort_unstable();
    digits_to_string(&sorted)
}

fn digit_matches_attribute(digit: u8, attribute: &DigitAttribute) -> bool {
    match attribute {
        DigitAttribute::Big => digit >= 5,
        DigitAttribute::Small => digit <= 4,
        DigitAttribute::Odd => digit % 2 == 1,
        DigitAttribute::Even => digit % 2 == 0,
    }
}

fn big_small_position_code(position: &BigSmallOddEvenPosition) -> &'static str {
    match position {
        BigSmallOddEvenPosition::Tens => "tens",
        BigSmallOddEvenPosition::Ones => "ones",
    }
}

fn digit_attribute_code(attribute: &DigitAttribute) -> &'static str {
    match attribute {
        DigitAttribute::Big => "big",
        DigitAttribute::Small => "small",
        DigitAttribute::Odd => "odd",
        DigitAttribute::Even => "even",
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        lottery::LotteryNumberType,
        play::{
            BigSmallOddEvenPick, BigSmallOddEvenPosition, DigitAttribute, PlayRuleCode,
            PlayRuleEvaluateRequest, PlaySelection,
        },
    };

    use super::evaluate_play_rule;

    #[test]
    fn three_direct_matches_exact_order() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::ThreeDigit,
            rule_code: PlayRuleCode::ThreeDirect,
            selection: PlaySelection {
                positions: vec![vec![2], vec![4], vec![7]],
                ..PlaySelection::default()
            },
            draw_number: "247".to_string(),
        });

        assert_eq!(evaluation.stake_count, 1);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["247"]);
    }

    #[test]
    fn three_group_three_counts_and_matches_permutations() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::ThreeDigit,
            rule_code: PlayRuleCode::ThreeGroupThree,
            selection: PlaySelection {
                numbers: vec![2, 4, 7],
                ..PlaySelection::default()
            },
            draw_number: "422".to_string(),
        });

        assert_eq!(evaluation.stake_count, 6);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["224"]);
    }

    #[test]
    fn three_group_three_banker_counts_banker_drag_shapes() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::ThreeDigit,
            rule_code: PlayRuleCode::ThreeGroupThreeBanker,
            selection: PlaySelection {
                banker_numbers: vec![2],
                drag_numbers: vec![4, 7],
                ..PlaySelection::default()
            },
            draw_number: "772".to_string(),
        });

        assert_eq!(evaluation.stake_count, 4);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["772"]);
    }

    #[test]
    fn three_group_six_counts_combinations() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::ThreeDigit,
            rule_code: PlayRuleCode::ThreeGroupSix,
            selection: PlaySelection {
                numbers: vec![1, 2, 4, 7],
                ..PlaySelection::default()
            },
            draw_number: "724".to_string(),
        });

        assert_eq!(evaluation.stake_count, 4);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["247"]);
    }

    #[test]
    fn three_group_six_banker_counts_drag_combinations() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::ThreeDigit,
            rule_code: PlayRuleCode::ThreeGroupSixBanker,
            selection: PlaySelection {
                banker_numbers: vec![2, 4],
                drag_numbers: vec![1, 7, 9],
                ..PlaySelection::default()
            },
            draw_number: "942".to_string(),
        });

        assert_eq!(evaluation.stake_count, 3);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["249"]);
    }

    #[test]
    fn five_middle_direct_uses_middle_window() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::FiveDigit,
            rule_code: PlayRuleCode::FiveMiddleDirect,
            selection: PlaySelection {
                positions: vec![vec![8], vec![9], vec![4]],
                ..PlaySelection::default()
            },
            draw_number: "78942".to_string(),
        });

        assert_eq!(evaluation.stake_count, 1);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["894"]);
    }

    #[test]
    fn five_front_direct_combination_counts_permutations() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::FiveDigit,
            rule_code: PlayRuleCode::FiveFrontDirectCombination,
            selection: PlaySelection {
                numbers: vec![1, 2, 3, 4],
                ..PlaySelection::default()
            },
            draw_number: "23142".to_string(),
        });

        assert_eq!(evaluation.stake_count, 24);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["231"]);
    }

    #[test]
    fn five_back_group_six_uses_back_window() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::FiveDigit,
            rule_code: PlayRuleCode::FiveBackGroupSix,
            selection: PlaySelection {
                numbers: vec![2, 4, 7, 9],
                ..PlaySelection::default()
            },
            draw_number: "78942".to_string(),
        });

        assert_eq!(evaluation.stake_count, 4);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["249"]);
    }

    #[test]
    fn five_big_small_odd_even_uses_tail_two_digits() {
        let evaluation = evaluate(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::FiveDigit,
            rule_code: PlayRuleCode::FiveBigSmallOddEven,
            selection: PlaySelection {
                big_small_odd_even: vec![
                    BigSmallOddEvenPick {
                        position: BigSmallOddEvenPosition::Tens,
                        attributes: vec![DigitAttribute::Small],
                    },
                    BigSmallOddEvenPick {
                        position: BigSmallOddEvenPosition::Ones,
                        attributes: vec![DigitAttribute::Even],
                    },
                ],
                ..PlaySelection::default()
            },
            draw_number: "78942".to_string(),
        });

        assert_eq!(evaluation.stake_count, 2);
        assert!(evaluation.is_winning);
        assert_eq!(evaluation.matched_bets, vec!["tens:small", "ones:even"]);
    }

    #[test]
    fn rejects_overlapping_banker_and_drag_digits() {
        let error = evaluate_play_rule(PlayRuleEvaluateRequest {
            number_type: LotteryNumberType::ThreeDigit,
            rule_code: PlayRuleCode::ThreeGroupSixBanker,
            selection: PlaySelection {
                banker_numbers: vec![2],
                drag_numbers: vec![2, 4, 7],
                ..PlaySelection::default()
            },
            draw_number: "247".to_string(),
        })
        .expect_err("overlap should be rejected");

        assert!(error.to_string().contains("cannot overlap"));
    }

    fn evaluate(request: PlayRuleEvaluateRequest) -> super::PlayRuleEvaluation {
        evaluate_play_rule(request).expect("play rule can be evaluated")
    }
}
