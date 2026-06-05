//! 手机端首页聚合服务，负责把彩种、分类和开奖期号组合成首页可直接消费的数据。

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Local, NaiveDateTime};

use crate::domain::{
    draw::{DrawIssue, DrawIssueStatus},
    lottery::{DrawSchedule, LotteryCategoryConfig, LotteryKind, LotteryNumberType},
    mobile::{
        MobileLotteryCard, MobileLotteryFeaturedSection, MobileLotteryGroup,
        MobileLotteryHomeResponse, MobileLotteryHomeSettings, MobileLotteryLatestDraw,
        MobileLotteryStats, MobileLotteryTicker, MobileLotteryTickerItem,
    },
};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, Default)]
struct LotteryIssueSnapshot {
    current: Option<DrawIssue>,
    latest_drawn: Option<DrawIssue>,
}

/// 生成手机端首页彩种聚合数据，只返回当前销售中的彩种。
pub fn build_mobile_lottery_home(
    lotteries: Vec<LotteryKind>,
    categories: Vec<LotteryCategoryConfig>,
    issues: Vec<DrawIssue>,
) -> MobileLotteryHomeResponse {
    let now = Local::now().naive_local();
    let snapshots = issue_snapshots(&issues);
    let selling_cards = lotteries
        .into_iter()
        .filter(|lottery| lottery.sale_enabled)
        .map(|lottery| {
            let snapshot = snapshots.get(&lottery.id);
            card_for_lottery(lottery, snapshot, now)
        })
        .collect::<Vec<_>>();
    let groups = group_cards_by_category(&selling_cards, categories);
    let ticker_items = ticker_items_from_cards(&selling_cards);
    let featured_lotteries = featured_cards(&selling_cards);

    MobileLotteryHomeResponse {
        server_time: now.format(TIMESTAMP_FORMAT).to_string(),
        settings: MobileLotteryHomeSettings {
            banners_enabled: true,
            ticker_enabled: !ticker_items.is_empty(),
            featured_enabled: !featured_lotteries.is_empty(),
            groups_enabled: !groups.is_empty(),
            stats_enabled: false,
        },
        ticker: MobileLotteryTicker {
            enabled: !ticker_items.is_empty(),
            items: ticker_items,
        },
        featured_section: MobileLotteryFeaturedSection {
            enabled: !featured_lotteries.is_empty(),
            title: "高频极速".to_string(),
            lotteries: featured_lotteries,
        },
        groups,
        stats: MobileLotteryStats {
            today_winner_count: 0,
            total_payout_display: "¥0".to_string(),
        },
    }
}

/// 按彩种整理当前期号和最近期开奖，避免页面层重复扫描期号列表。
fn issue_snapshots(issues: &[DrawIssue]) -> BTreeMap<String, LotteryIssueSnapshot> {
    let mut grouped: BTreeMap<String, Vec<DrawIssue>> = BTreeMap::new();
    for issue in issues {
        grouped
            .entry(issue.lottery_id.clone())
            .or_default()
            .push(issue.clone());
    }

    grouped
        .into_iter()
        .map(|(lottery_id, issues)| {
            (
                lottery_id,
                LotteryIssueSnapshot {
                    current: current_issue(&issues),
                    latest_drawn: latest_drawn_issue(&issues),
                },
            )
        })
        .collect()
}

/// 选择首页当前期：优先展示可售期，其次展示已封盘待开奖期，再退回最近期开奖期。
fn current_issue(issues: &[DrawIssue]) -> Option<DrawIssue> {
    issues
        .iter()
        .filter(|issue| issue.status == DrawIssueStatus::Open)
        .min_by_key(|issue| scheduled_time_value(issue).unwrap_or(i64::MAX))
        .cloned()
        .or_else(|| {
            issues
                .iter()
                .filter(|issue| issue.status == DrawIssueStatus::Closed)
                .min_by_key(|issue| scheduled_time_value(issue).unwrap_or(i64::MAX))
                .cloned()
        })
        .or_else(|| latest_drawn_issue(issues))
}

/// 取最近一个有开奖结果的期号，作为首页“最新开奖号码”来源。
fn latest_drawn_issue(issues: &[DrawIssue]) -> Option<DrawIssue> {
    issues
        .iter()
        .filter(|issue| issue.status == DrawIssueStatus::Drawn)
        .max_by_key(|issue| drawn_time_value(issue).unwrap_or(0))
        .cloned()
}

/// 把单个彩种和期号快照转换为手机端首页卡片。
fn card_for_lottery(
    lottery: LotteryKind,
    snapshot: Option<&LotteryIssueSnapshot>,
    now: NaiveDateTime,
) -> MobileLotteryCard {
    let current = snapshot.and_then(|snapshot| snapshot.current.as_ref());
    let latest_drawn = snapshot.and_then(|snapshot| snapshot.latest_drawn.as_ref());
    let latest_result = latest_drawn
        .and_then(|issue| issue.draw_number.as_deref())
        .map(split_draw_number)
        .unwrap_or_default();
    let latest_draw = latest_drawn.map(|issue| MobileLotteryLatestDraw {
        issue: issue.issue.clone(),
        result_numbers: latest_result.clone(),
        opened_at: issue
            .drawn_at
            .clone()
            .or_else(|| Some(issue.scheduled_at.clone())),
    });

    MobileLotteryCard {
        code: lottery.id.clone(),
        name: lottery.name.clone(),
        category: lottery.category.clone(),
        logo_url: optional_text(&lottery.logo_url),
        issue: current
            .map(|issue| issue.issue.clone())
            .or_else(|| latest_drawn.map(|issue| issue.issue.clone())),
        status: current
            .map(|issue| homepage_status(issue, now))
            .unwrap_or_else(|| "waiting".to_string()),
        next_draw_time: current.map(|issue| issue.scheduled_at.clone()),
        sale_stop_time: current.map(|issue| issue.sale_closed_at.clone()),
        draw_interval: draw_interval_seconds(&lottery.schedule),
        draw_time_text: schedule_text(&lottery.schedule),
        schedule_text: schedule_text(&lottery.schedule),
        latest_result,
        result_style: "balls".to_string(),
        result_count: result_count(&lottery.number_type),
        group_buy_enabled: lottery.group_buy.enabled,
        latest_draw,
    }
}

/// 依据销售状态和封盘时间返回首页卡片业务状态。
fn homepage_status(issue: &DrawIssue, now: NaiveDateTime) -> String {
    match issue.status {
        DrawIssueStatus::Open => parse_timestamp(&issue.sale_closed_at)
            .filter(|sale_closed_at| *sale_closed_at <= now)
            .map(|_| "sealed".to_string())
            .unwrap_or_else(|| "selling".to_string()),
        DrawIssueStatus::Closed => "sealed".to_string(),
        DrawIssueStatus::Drawn => "drawn".to_string(),
        DrawIssueStatus::Cancelled => "closed".to_string(),
    }
}

/// 按后台彩种分类配置顺序生成首页分组，未配置分类的彩种放到对应代码兜底组。
fn group_cards_by_category(
    cards: &[MobileLotteryCard],
    categories: Vec<LotteryCategoryConfig>,
) -> Vec<MobileLotteryGroup> {
    let mut grouped: BTreeMap<String, Vec<MobileLotteryCard>> = BTreeMap::new();
    for card in cards {
        grouped
            .entry(card.category.clone())
            .or_default()
            .push(card.clone());
    }

    let mut used_categories = BTreeSet::new();
    let mut groups = Vec::new();
    for category in categories {
        used_categories.insert(category.code.clone());
        if let Some(lotteries) = grouped
            .remove(&category.code)
            .filter(|items| !items.is_empty())
        {
            groups.push(MobileLotteryGroup {
                code: category.code,
                name: category.name,
                lotteries,
            });
        }
    }

    for (code, lotteries) in grouped {
        if lotteries.is_empty() || used_categories.contains(&code) {
            continue;
        }
        groups.push(MobileLotteryGroup {
            name: code.clone(),
            code,
            lotteries,
        });
    }

    groups
}

/// 首页推荐区优先选高频彩种；没有高频彩种时展示前 3 个销售中彩种。
fn featured_cards(cards: &[MobileLotteryCard]) -> Vec<MobileLotteryCard> {
    let high_frequency = cards
        .iter()
        .filter(|card| card.draw_interval.is_some_and(|interval| interval <= 300))
        .take(3)
        .cloned()
        .collect::<Vec<_>>();

    if high_frequency.is_empty() {
        cards.iter().take(3).cloned().collect()
    } else {
        high_frequency
    }
}

/// 从最近期开奖中生成首页跑马灯内容。
fn ticker_items_from_cards(cards: &[MobileLotteryCard]) -> Vec<MobileLotteryTickerItem> {
    cards
        .iter()
        .filter_map(|card| {
            let latest_draw = card.latest_draw.as_ref()?;
            if latest_draw.result_numbers.is_empty() {
                return None;
            }
            Some(MobileLotteryTickerItem {
                id: format!("{}-{}", card.code, latest_draw.issue),
                text: format!(
                    "{} 第{}期 开奖号码 {}",
                    card.name,
                    latest_draw.issue,
                    latest_draw.result_numbers.join(",")
                ),
            })
        })
        .take(8)
        .collect()
}

/// 返回周期开奖周期秒数，非周期彩种返回空。
fn draw_interval_seconds(schedule: &DrawSchedule) -> Option<u32> {
    match schedule {
        DrawSchedule::Periodic { interval_seconds } => Some(*interval_seconds),
        DrawSchedule::Daily { .. } | DrawSchedule::Weekly { .. } => None,
    }
}

/// 生成首页展示用开奖时间文案。
fn schedule_text(schedule: &DrawSchedule) -> String {
    match schedule {
        DrawSchedule::Periodic { interval_seconds } => interval_text(*interval_seconds),
        DrawSchedule::Daily { time } => format!("每日 {time}"),
        DrawSchedule::Weekly { weekdays, time } => {
            let weekdays = weekdays
                .iter()
                .map(|weekday| weekday_text(weekday))
                .collect::<Vec<_>>()
                .join("、");
            format!("每周{weekdays} {time}")
        }
    }
}

/// 把秒数周期转换为中文文案。
fn interval_text(interval_seconds: u32) -> String {
    if interval_seconds < 60 {
        return format!("{interval_seconds}秒开奖");
    }
    if interval_seconds % 60 == 0 {
        return format!("{}分钟开奖", interval_seconds / 60);
    }
    format!("{interval_seconds}秒开奖")
}

/// 把英文星期转换为中文展示。
fn weekday_text(value: &str) -> String {
    match value {
        "Monday" => "一".to_string(),
        "Tuesday" => "二".to_string(),
        "Wednesday" => "三".to_string(),
        "Thursday" => "四".to_string(),
        "Friday" => "五".to_string(),
        "Saturday" => "六".to_string(),
        "Sunday" => "日".to_string(),
        other => other.to_string(),
    }
}

/// 返回不同号码类型在首页应展示的号码个数。
fn result_count(number_type: &LotteryNumberType) -> usize {
    match number_type {
        LotteryNumberType::ThreeDigit | LotteryNumberType::FastThree => 3,
        LotteryNumberType::FiveDigit | LotteryNumberType::ElevenFive => 5,
        LotteryNumberType::Pk10 => 10,
        LotteryNumberType::LuckTwenty => 20,
    }
}

/// 拆分开奖号码，兼容英文逗号、中文逗号、空白和紧凑数字字符串。
fn split_draw_number(value: &str) -> Vec<String> {
    let text = value.trim();
    if text.is_empty() {
        return Vec::new();
    }
    if text.contains(',') || text.contains('，') || text.contains(' ') {
        return text
            .split([',', '，', ' '])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    if text.bytes().all(|byte| byte.is_ascii_digit()) {
        return text.chars().map(|item| item.to_string()).collect();
    }
    vec![text.to_string()]
}

/// 把空字符串转为 None，便于前端判断是否展示图片。
fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// 返回期号开奖时间排序值。
fn scheduled_time_value(issue: &DrawIssue) -> Option<i64> {
    parse_timestamp(&issue.scheduled_at).map(|value| value.and_utc().timestamp())
}

/// 返回已开奖期号排序值，优先使用实际开奖时间。
fn drawn_time_value(issue: &DrawIssue) -> Option<i64> {
    issue
        .drawn_at
        .as_deref()
        .and_then(parse_timestamp)
        .or_else(|| parse_timestamp(&issue.scheduled_at))
        .map(|value| value.and_utc().timestamp())
}

/// 解析业务时间字符串，兼容标准时间和 unix: 秒级标签。
fn parse_timestamp(value: &str) -> Option<NaiveDateTime> {
    let value = value.trim();
    if let Some(seconds) = value.strip_prefix("unix:") {
        let seconds = seconds.parse::<i64>().ok()?;
        return DateTime::from_timestamp(seconds, 0).map(|value| value.naive_utc());
    }

    NaiveDateTime::parse_from_str(value, TIMESTAMP_FORMAT).ok()
}

#[cfg(test)]
mod tests {
    use crate::domain::{
        draw::{DrawIssue, DrawIssueStatus},
        lottery::{
            DrawMode, DrawSchedule, GroupBuyConfig, LotteryCategoryConfig, LotteryKind,
            LotteryNumberType,
        },
    };

    use super::build_mobile_lottery_home;

    #[test]
    /// 首页只返回销售中的彩种，并按分类带出最近开奖号码。
    fn mobile_home_groups_selling_lotteries_with_latest_draws() {
        let response = build_mobile_lottery_home(
            vec![
                sample_lottery("fc3d", "福彩 3D", "welfare", true),
                sample_lottery("au5", "澳洲 5 分彩", "overseas", true),
                sample_lottery("closed", "停售彩种", "regional", false),
            ],
            vec![
                LotteryCategoryConfig {
                    code: "welfare".to_string(),
                    name: "福利彩种".to_string(),
                },
                LotteryCategoryConfig {
                    code: "overseas".to_string(),
                    name: "海外彩种".to_string(),
                },
                LotteryCategoryConfig {
                    code: "regional".to_string(),
                    name: "地方彩种".to_string(),
                },
            ],
            vec![
                sample_issue(
                    "D000000000001",
                    "fc3d",
                    "福彩 3D",
                    "20260605001",
                    DrawIssueStatus::Drawn,
                    Some("1,2,3"),
                    "2026-06-05 20:00:00",
                ),
                sample_issue(
                    "D000000000002",
                    "fc3d",
                    "福彩 3D",
                    "20260605002",
                    DrawIssueStatus::Open,
                    None,
                    "2026-06-05 20:01:00",
                ),
                sample_issue(
                    "D000000000003",
                    "closed",
                    "停售彩种",
                    "20260605001",
                    DrawIssueStatus::Drawn,
                    Some("9,9,9"),
                    "2026-06-05 20:00:00",
                ),
            ],
        );

        let all_codes = response
            .groups
            .iter()
            .flat_map(|group| group.lotteries.iter().map(|lottery| lottery.code.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(all_codes, vec!["fc3d", "au5"]);
        assert!(!response.settings.stats_enabled);

        let fc3d = response.groups[0].lotteries[0].clone();
        assert_eq!(fc3d.issue.as_deref(), Some("20260605002"));
        assert_eq!(fc3d.latest_result, vec!["1", "2", "3"]);
        assert_eq!(
            fc3d.latest_draw.as_ref().map(|draw| draw.issue.as_str()),
            Some("20260605001")
        );
    }

    fn sample_lottery(id: &str, name: &str, category: &str, sale_enabled: bool) -> LotteryKind {
        LotteryKind {
            id: id.to_string(),
            name: name.to_string(),
            category: category.to_string(),
            logo_url: String::new(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Platform,
            schedule: DrawSchedule::Periodic {
                interval_seconds: 60,
            },
            sale_enabled,
            group_buy: GroupBuyConfig {
                enabled: false,
                min_share_amount_minor: 100,
                initiator_min_percent: 10,
                participant_min_amount_minor: 100,
            },
            play_categories: Vec::new(),
            play_configs: Vec::new(),
        }
    }

    fn sample_issue(
        id: &str,
        lottery_id: &str,
        lottery_name: &str,
        issue: &str,
        status: DrawIssueStatus,
        draw_number: Option<&str>,
        scheduled_at: &str,
    ) -> DrawIssue {
        DrawIssue {
            id: id.to_string(),
            lottery_id: lottery_id.to_string(),
            lottery_name: lottery_name.to_string(),
            issue: issue.to_string(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Platform,
            scheduled_at: scheduled_at.to_string(),
            sale_closed_at: "2099-01-01 00:00:00".to_string(),
            status,
            draw_number: draw_number.map(ToString::to_string),
            drawn_at: draw_number.map(|_| scheduled_at.to_string()),
            created_at: scheduled_at.to_string(),
        }
    }
}
