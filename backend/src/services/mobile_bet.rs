//! 手机端下注页聚合服务，负责把彩种、期号和玩法赔率转换为前端投注配置。

use chrono::NaiveDateTime;

use crate::domain::{
    draw::{DrawIssue, DrawIssueStatus},
    lottery::{DrawSchedule, LotteryKind, PlayCategory},
    mobile::{
        MobileBetGroupBuySettings, MobileBetLatestDraw, MobileBetLottery, MobileBetOption,
        MobileBetOptionGroup, MobileBetPageConfig, MobileBetPlay, MobileBetPosition,
        MobileBetRound,
    },
    play::{PlayRuleCode, PlayRuleSummary, ThreeDigitWindow},
};

use super::play_rules::{play_category_for_rule, play_rule_summaries};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const DEFAULT_UNIT_AMOUNT_MINOR: i64 = 200;
const DEFAULT_MAX_MULTIPLE: u32 = 999;

/// 生成手机端下注页配置，只暴露当前彩种已启用的玩法和赔率。
pub fn build_mobile_bet_page_config(
    lottery: &LotteryKind,
    issues: Vec<DrawIssue>,
) -> MobileBetPageConfig {
    let current_open_issue = current_open_issue(&issues);
    let waiting_issue = waiting_issue(&issues);
    let latest_drawn_issue = latest_drawn_issue(&issues);
    let summaries = play_rule_summaries();
    let plays = lottery
        .play_configs
        .iter()
        .filter(|config| config.enabled)
        .filter_map(|config| {
            let summary = summaries
                .iter()
                .find(|summary| summary.code == config.rule_code)?;
            Some(play_from_config(summary, config.odds_basis_points))
        })
        .collect::<Vec<_>>();

    MobileBetPageConfig {
        lottery: MobileBetLottery {
            code: lottery.id.clone(),
            name: lottery.name.clone(),
            category: lottery.category.clone(),
            draw_interval: draw_interval_seconds(&lottery.schedule),
            group_buy_enabled: lottery.group_buy.enabled,
        },
        group_buy_settings: MobileBetGroupBuySettings {
            min_share_amount: minor_to_decimal(lottery.group_buy.min_share_amount_minor),
            initiator_min_buy_ratio: format!("{:.2}", lottery.group_buy.initiator_min_percent),
            share_amount: minor_to_decimal(lottery.group_buy.min_share_amount_minor),
        },
        round: round_from_issue(current_open_issue.or(waiting_issue)),
        latest_draw: latest_drawn_issue.map(latest_draw_from_issue),
        plays,
    }
}

/// 选择最早封盘的可售期号，作为下注页当前可投注期。
fn current_open_issue(issues: &[DrawIssue]) -> Option<&DrawIssue> {
    issues
        .iter()
        .filter(|issue| issue.status == DrawIssueStatus::Open)
        .min_by_key(|issue| timestamp_value(&issue.sale_closed_at).unwrap_or(i64::MAX))
}

/// 没有可售期时展示最近待开奖期，让页面进入“开奖中/开盘轮询”状态。
fn waiting_issue(issues: &[DrawIssue]) -> Option<&DrawIssue> {
    issues
        .iter()
        .filter(|issue| issue.status == DrawIssueStatus::Closed)
        .min_by_key(|issue| timestamp_value(&issue.scheduled_at).unwrap_or(i64::MAX))
}

/// 选择最近一个带开奖结果的已开奖期号，作为下注页顶部最近开奖。
fn latest_drawn_issue(issues: &[DrawIssue]) -> Option<&DrawIssue> {
    issues
        .iter()
        .filter(|issue| issue.status == DrawIssueStatus::Drawn && issue.draw_number.is_some())
        .max_by_key(|issue| {
            issue
                .drawn_at
                .as_deref()
                .and_then(timestamp_value)
                .or_else(|| timestamp_value(&issue.scheduled_at))
                .unwrap_or_default()
        })
}

/// 根据当前期号生成前端状态；非可售期统一返回 opening，触发手机端轮询下一期。
fn round_from_issue(issue: Option<&DrawIssue>) -> MobileBetRound {
    let Some(issue) = issue else {
        return MobileBetRound {
            issue: String::new(),
            status: "opening".to_string(),
            scheduled_draw_at: None,
            sale_stop_at: None,
        };
    };

    MobileBetRound {
        issue: issue.issue.clone(),
        status: if issue.status == DrawIssueStatus::Open {
            "selling".to_string()
        } else {
            "opening".to_string()
        },
        scheduled_draw_at: Some(issue.scheduled_at.clone()),
        sale_stop_at: Some(issue.sale_closed_at.clone()),
    }
}

/// 将期开奖数据转换为手机端最近开奖结构，开奖号码按逗号或逐字符拆分。
fn latest_draw_from_issue(issue: &DrawIssue) -> MobileBetLatestDraw {
    MobileBetLatestDraw {
        issue: issue.issue.clone(),
        result_numbers: split_draw_number(issue.draw_number.as_deref().unwrap_or_default()),
        opened_at: issue
            .drawn_at
            .clone()
            .or_else(|| Some(issue.scheduled_at.clone())),
    }
}

/// 根据玩法摘要和赔率配置生成一个动态玩法入口。
fn play_from_config(summary: &PlayRuleSummary, odds_basis_points: i64) -> MobileBetPlay {
    let rule_code = summary.code.clone();
    let position_grid_kind = position_grid_kind_for_rule(&rule_code);
    let option_groups = option_groups_for_rule(&rule_code, odds_basis_points);

    MobileBetPlay {
        code: rule_code.clone(),
        name: summary.label.clone(),
        full_name: summary.label.clone(),
        rule_code,
        input_mode: "position-grid".to_string(),
        positions: positions_for_rule(summary),
        digits: digit_values(),
        number_grid_values: digit_values(),
        option_value: None,
        min_select_count: min_select_count_for_rule(&summary.code),
        bet_number_count: 3,
        odds: odds_text(odds_basis_points),
        unit_amount_fixed: true,
        unit_amount: minor_to_decimal(DEFAULT_UNIT_AMOUNT_MINOR),
        multiple_fixed: false,
        multiple: 1,
        min_multiple: 1,
        max_multiple: Some(DEFAULT_MAX_MULTIPLE),
        simple_description: summary.description.clone(),
        detail_description: summary.description.clone(),
        example_description: example_for_rule(&summary.code),
        position_grid_kind,
        max_select_per_position: None,
        option_groups,
        option_groups_error: None,
    }
}

/// 返回玩法需要展示的位置行；单行复式和胆拖玩法由 position_grid_kind 控制交互。
fn positions_for_rule(summary: &PlayRuleSummary) -> Vec<MobileBetPosition> {
    match play_category_for_rule(&summary.code) {
        PlayCategory::Direct => direct_positions(summary.window.clone()),
        PlayCategory::DirectCombination | PlayCategory::GroupThree | PlayCategory::GroupSix => {
            if is_banker_rule(&summary.code) {
                vec![position("banker", "胆码"), position("drag", "拖码")]
            } else {
                vec![position("numbers", "号码")]
            }
        }
        PlayCategory::BigSmallOddEven => Vec::new(),
    }
}

/// 根据直选窗口展示对应三位位置。
fn direct_positions(window: ThreeDigitWindow) -> Vec<MobileBetPosition> {
    match window {
        ThreeDigitWindow::Full => vec![
            position("hundreds", "百位"),
            position("tens", "十位"),
            position("ones", "个位"),
        ],
        ThreeDigitWindow::Front => vec![
            position("first", "第 1 位"),
            position("second", "第 2 位"),
            position("third", "第 3 位"),
        ],
        ThreeDigitWindow::Middle => vec![
            position("second", "第 2 位"),
            position("third", "第 3 位"),
            position("fourth", "第 4 位"),
        ],
        ThreeDigitWindow::Back => vec![
            position("third", "第 3 位"),
            position("fourth", "第 4 位"),
            position("fifth", "第 5 位"),
        ],
    }
}

/// 创建一个前端位置行描述。
fn position(key: &str, label: &str) -> MobileBetPosition {
    MobileBetPosition {
        key: key.to_string(),
        label: label.to_string(),
    }
}

/// 将玩法分类映射成手机端位置宫格算法。
fn position_grid_kind_for_rule(rule_code: &PlayRuleCode) -> String {
    use PlayRuleCode::*;
    match rule_code {
        FiveFrontDirectCombination | FiveMiddleDirectCombination | FiveBackDirectCombination => {
            "direct_combination"
        }
        ThreeGroupThree | FiveFrontGroupThree | FiveMiddleGroupThree | FiveBackGroupThree => {
            "group3_compound"
        }
        ThreeGroupSix | FiveFrontGroupSix | FiveMiddleGroupSix | FiveBackGroupSix => {
            "group6_compound"
        }
        ThreeGroupThreeBanker
        | FiveFrontGroupThreeBanker
        | FiveMiddleGroupThreeBanker
        | FiveBackGroupThreeBanker => "group3_dantuo",
        ThreeGroupSixBanker
        | FiveFrontGroupSixBanker
        | FiveMiddleGroupSixBanker
        | FiveBackGroupSixBanker => "group6_dantuo",
        _ => "direct",
    }
    .to_string()
}

/// 大小单双玩法用配置化选项组展示，下注时映射到 bigSmallOddEven selection。
fn option_groups_for_rule(
    rule_code: &PlayRuleCode,
    odds_basis_points: i64,
) -> Vec<MobileBetOptionGroup> {
    if !matches!(rule_code, PlayRuleCode::FiveBigSmallOddEven) {
        return Vec::new();
    }

    let options = vec![
        option("big", "大", "5-9", odds_basis_points),
        option("small", "小", "0-4", odds_basis_points),
        option("odd", "单", "奇数", odds_basis_points),
        option("even", "双", "偶数", odds_basis_points),
    ];

    vec![
        MobileBetOptionGroup {
            key: "tens".to_string(),
            label: "十位".to_string(),
            min_select_count: 1,
            max_select_count: 1,
            options: options.clone(),
        },
        MobileBetOptionGroup {
            key: "ones".to_string(),
            label: "个位".to_string(),
            min_select_count: 1,
            max_select_count: 1,
            options,
        },
    ]
}

/// 创建一个大小单双选项。
fn option(value: &str, label: &str, description: &str, odds_basis_points: i64) -> MobileBetOption {
    MobileBetOption {
        value: value.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        odds: Some(odds_text(odds_basis_points)),
        disabled: false,
    }
}

/// 判断玩法是否属于胆拖形态。
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

/// 返回玩法单行最少选择数量，供前端展示和轻量校验。
fn min_select_count_for_rule(rule_code: &PlayRuleCode) -> u32 {
    match position_grid_kind_for_rule(rule_code).as_str() {
        "direct_combination" => 3,
        "group3_compound" => 2,
        "group6_compound" => 3,
        _ => 1,
    }
}

/// 给不同玩法补一条简短示例，便于玩法弹层展示。
fn example_for_rule(rule_code: &PlayRuleCode) -> String {
    match play_category_for_rule(rule_code) {
        PlayCategory::Direct => "例如选择 1、2、3，开奖号码对应三位为 1,2,3 即命中。".to_string(),
        PlayCategory::DirectCombination => {
            "例如选择 1、2、3、4，系统会展开不同数字的三位排列。".to_string()
        }
        PlayCategory::GroupThree => "例如选择 1、2、3，组三形态且数字命中即中奖。".to_string(),
        PlayCategory::GroupSix => "例如选择 1、2、3、4，系统会展开三不同数字组合。".to_string(),
        PlayCategory::BigSmallOddEven => {
            "选择后两位的大小或单双属性，按开奖号码后两位判断。".to_string()
        }
    }
}

/// 返回 0-9 的数字池。
fn digit_values() -> Vec<String> {
    (0..=9).map(|digit| digit.to_string()).collect()
}

/// 读取周期秒数；非周期彩种返回 0，前端只作为展示兜底。
fn draw_interval_seconds(schedule: &DrawSchedule) -> u32 {
    match schedule {
        DrawSchedule::Periodic { interval_seconds } => *interval_seconds,
        DrawSchedule::Daily { .. } | DrawSchedule::Weekly { .. } => 0,
    }
}

/// 分转元，固定保留两位小数。
fn minor_to_decimal(value: i64) -> String {
    format!("{:.2}", value as f64 / 100.0)
}

/// 基点赔率转展示赔率，固定保留两位小数。
fn odds_text(value: i64) -> String {
    format!("{:.2}", value as f64 / 10_000.0)
}

/// 解析中文时间文本为可比较数值。
fn timestamp_value(value: &str) -> Option<i64> {
    NaiveDateTime::parse_from_str(value.trim(), TIMESTAMP_FORMAT)
        .ok()
        .map(|value| value.and_utc().timestamp())
}

/// 开奖号码兼容逗号分隔和紧凑数字串。
fn split_draw_number(value: &str) -> Vec<String> {
    let text = value.trim();
    if text.is_empty() {
        return Vec::new();
    }
    if text.contains(',') || text.contains('，') {
        return text
            .split(|character| character == ',' || character == '，')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect();
    }

    text.chars()
        .map(|character| character.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        draw::{DrawIssue, DrawIssueStatus},
        lottery::{
            DrawMode, DrawSchedule, GroupBuyConfig, LotteryKind, LotteryNumberType,
            LotteryPlayConfig, PlayCategory,
        },
        play::PlayRuleCode,
    };

    use super::build_mobile_bet_page_config;

    #[test]
    /// 验证下注页配置只暴露可售期、最近开奖和已启用玩法。
    fn mobile_bet_page_config_uses_current_round_and_enabled_plays() {
        let lottery = lottery_fixture();
        let issues = vec![
            issue_fixture("I001", DrawIssueStatus::Drawn, Some("1,2,3,4,5")),
            issue_fixture("I002", DrawIssueStatus::Open, None),
        ];

        let config = build_mobile_bet_page_config(&lottery, issues);

        assert_eq!(config.lottery.code, "txffc");
        assert_eq!(config.round.issue, "I002");
        assert_eq!(config.round.status, "selling");
        assert_eq!(
            config.latest_draw.expect("应返回最近开奖").result_numbers,
            vec!["1", "2", "3", "4", "5"]
        );
        assert_eq!(config.plays.len(), 1);
        assert_eq!(config.plays[0].code, PlayRuleCode::FiveFrontDirect);
    }

    #[test]
    /// 验证直选组合会以可多选的位置宫格返回，前端才能正确计算排列注数。
    fn direct_combination_uses_position_grid_kind() {
        let mut lottery = lottery_fixture();
        lottery.play_configs = vec![LotteryPlayConfig {
            rule_code: PlayRuleCode::FiveFrontDirectCombination,
            enabled: true,
            odds_basis_points: 100_000,
        }];

        let config = build_mobile_bet_page_config(&lottery, Vec::new());

        assert_eq!(config.plays[0].position_grid_kind, "direct_combination");
        assert_eq!(config.plays[0].positions[0].key, "numbers");
        assert_eq!(config.plays[0].min_select_count, 3);
    }

    fn lottery_fixture() -> LotteryKind {
        LotteryKind {
            id: "txffc".to_string(),
            name: "腾讯分分彩".to_string(),
            category: "overseas".to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Api,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled: true,
            group_buy: GroupBuyConfig {
                enabled: false,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 1_000,
            },
            play_categories: vec![PlayCategory::Direct],
            play_configs: vec![
                LotteryPlayConfig {
                    rule_code: PlayRuleCode::FiveFrontDirect,
                    enabled: true,
                    odds_basis_points: 95_000,
                },
                LotteryPlayConfig {
                    rule_code: PlayRuleCode::FiveBackDirect,
                    enabled: false,
                    odds_basis_points: 95_000,
                },
            ],
        }
    }

    fn issue_fixture(issue: &str, status: DrawIssueStatus, draw_number: Option<&str>) -> DrawIssue {
        let minute = match issue {
            "I001" => "01",
            "I002" => "02",
            _ => "03",
        };
        DrawIssue {
            id: format!("draw-{issue}"),
            lottery_id: "txffc".to_string(),
            lottery_name: "腾讯分分彩".to_string(),
            issue: issue.to_string(),
            number_type: LotteryNumberType::FiveDigit,
            draw_mode: DrawMode::Api,
            scheduled_at: format!("2026-06-05 12:{minute}:00"),
            sale_closed_at: format!("2026-06-05 12:{minute}:30"),
            status,
            draw_number: draw_number.map(ToString::to_string),
            drawn_at: Some(format!("2026-06-05 12:{minute}:02")),
            created_at: "2026-06-05 12:00:00".to_string(),
        }
    }
}
