//! 手机端彩票公开接口路由，提供首页彩种分组与开奖摘要。

use std::collections::{BTreeSet, HashMap};

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::{
    app::AppState,
    domain::{
        draw::{DrawIssue, DrawIssueStatus},
        lottery::{DrawSchedule, LotteryCategoryConfig, LotteryKind},
        mobile::MobileLotteryHomeResponse,
    },
    error::ApiResult,
    response::ApiEnvelope,
    services::mobile_home::{build_mobile_lottery_home, mobile_featured_config_from_settings},
};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const DEFAULT_HISTORY_PAGE_SIZE: usize = 50;
const MAX_HISTORY_PAGE_SIZE: usize = 100;

/// 组装手机端彩票公开接口路由。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/home", get(get_lottery_home))
        .route("/groups", get(list_lottery_groups))
        .route("/history/latest", get(list_latest_draw_history))
        .route("/history", get(list_draw_history))
}

/// 返回手机端首页所需的销售中彩种、分类分组和最近开奖号码。
async fn get_lottery_home(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<MobileLotteryHomeResponse>>> {
    let lotteries = state.lotteries.list().await?;
    let categories = state.lotteries.categories().await?;
    let issues = state.draws.list().await?;
    let settings = state.access.settings().await?;
    let featured_config = mobile_featured_config_from_settings(&settings);
    let home = build_mobile_lottery_home(lotteries, categories, issues, featured_config);

    Ok(Json(ApiEnvelope::success(home)))
}

/// 返回手机端可筛选的销售中彩种分组。
async fn list_lottery_groups(
    State(state): State<AppState>,
) -> ApiResult<Json<ApiEnvelope<Vec<MobileLotteryHistoryGroup>>>> {
    let lotteries = state.lotteries.list().await?;
    let categories = state.lotteries.categories().await?;
    let groups = lottery_history_groups(&lotteries, &categories);

    Ok(Json(ApiEnvelope::success(groups)))
}

/// 返回每个彩种最近一期已开奖数据，支持按分组或彩种筛选。
async fn list_latest_draw_history(
    State(state): State<AppState>,
    Query(query): Query<LotteryHistoryQuery>,
) -> ApiResult<Json<ApiEnvelope<MobileLotteryHistoryPage>>> {
    let lotteries = state.lotteries.list().await?;
    let issues = state.draws.list().await?;
    let items = latest_history_items(&lotteries, &issues, &query);

    Ok(Json(ApiEnvelope::success(
        MobileLotteryHistoryPage::from_items(items, 1, None),
    )))
}

/// 返回单彩种或筛选范围内的已开奖历史。
async fn list_draw_history(
    State(state): State<AppState>,
    Query(query): Query<LotteryHistoryQuery>,
) -> ApiResult<Json<ApiEnvelope<MobileLotteryHistoryPage>>> {
    let lotteries = state.lotteries.list().await?;
    let issues = state.draws.list().await?;
    let items = draw_history_items(&lotteries, &issues, &query);
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_HISTORY_PAGE_SIZE)
        .clamp(1, MAX_HISTORY_PAGE_SIZE);

    Ok(Json(ApiEnvelope::success(
        MobileLotteryHistoryPage::from_items(items, page, Some(page_size)),
    )))
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// 手机端开奖历史按分类分组的数据结构。
struct MobileLotteryHistoryGroup {
    code: String,
    name: String,
    lotteries: Vec<MobileLotteryGroupLottery>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// 开奖历史分类下的彩种筛选项。
struct MobileLotteryGroupLottery {
    code: String,
    name: String,
    category: String,
    logo_url: Option<String>,
    draw_interval: Option<u32>,
    daily_draw_time: Option<String>,
    group_sort_order: usize,
    is_recommended: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// 手机端开奖历史单条记录。
struct MobileLotteryHistoryItem {
    id: String,
    lottery_code: String,
    lottery_name: String,
    category: String,
    logo_url: Option<String>,
    issue: String,
    result: String,
    result_numbers: Vec<String>,
    opened_at: Option<String>,
    status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
/// 手机端开奖历史分页响应。
struct MobileLotteryHistoryPage {
    items: Vec<MobileLotteryHistoryItem>,
    total_count: usize,
    page: usize,
    page_size: usize,
    total_pages: usize,
}

/// 手机端开奖历史分页响应。
impl MobileLotteryHistoryPage {
    /// 根据完整历史记录和分页参数生成开奖历史分页。
    fn from_items(
        items: Vec<MobileLotteryHistoryItem>,
        page: usize,
        page_size: Option<usize>,
    ) -> Self {
        let total_count = items.len();
        let page_size = page_size.unwrap_or_else(|| total_count.max(1));
        let total_pages = if total_count == 0 {
            0
        } else {
            total_count.div_ceil(page_size)
        };
        let page = if total_pages == 0 {
            1
        } else {
            page.min(total_pages)
        };
        let start = (page - 1).saturating_mul(page_size).min(total_count);
        let end = (start + page_size).min(total_count);

        Self {
            items: items[start..end].to_vec(),
            total_count,
            page,
            page_size,
            total_pages,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
/// 手机端开奖历史查询条件。
struct LotteryHistoryQuery {
    lottery_code: Option<String>,
    group_code: Option<String>,
    page: Option<usize>,
    page_size: Option<usize>,
}

/// 生成手机端开奖历史分类和彩种筛选项。
fn lottery_history_groups(
    lotteries: &[LotteryKind],
    categories: &[LotteryCategoryConfig],
) -> Vec<MobileLotteryHistoryGroup> {
    let selling_lotteries = lotteries
        .iter()
        .filter(|lottery| lottery.sale_enabled)
        .collect::<Vec<_>>();
    let selling_categories = selling_lotteries
        .iter()
        .map(|lottery| lottery.category.clone())
        .collect::<BTreeSet<_>>();
    let configured_codes = categories
        .iter()
        .map(|category| category.code.clone())
        .collect::<BTreeSet<_>>();
    let mut groups = categories
        .iter()
        .filter(|category| selling_categories.contains(&category.code))
        .map(|category| MobileLotteryHistoryGroup {
            code: category.code.clone(),
            name: category.name.clone(),
            lotteries: group_lotteries(&selling_lotteries, &category.code),
        })
        .collect::<Vec<_>>();

    groups.extend(
        selling_categories
            .into_iter()
            .filter(|code| !configured_codes.contains(code))
            .map(|code| MobileLotteryHistoryGroup {
                name: code.clone(),
                lotteries: group_lotteries(&selling_lotteries, &code),
                code,
            }),
    );

    groups
}

/// 把同一分类下的彩种转换为手机端筛选项。
fn group_lotteries(lotteries: &[&LotteryKind], category: &str) -> Vec<MobileLotteryGroupLottery> {
    lotteries
        .iter()
        .filter(|lottery| lottery.category == category)
        .enumerate()
        .map(|(index, lottery)| MobileLotteryGroupLottery {
            code: lottery.id.clone(),
            name: lottery.name.clone(),
            category: lottery.category.clone(),
            logo_url: optional_text(&lottery.logo_url),
            draw_interval: draw_interval_seconds(&lottery.schedule),
            daily_draw_time: daily_draw_time(&lottery.schedule),
            group_sort_order: index,
            is_recommended: draw_interval_seconds(&lottery.schedule)
                .is_some_and(|interval| interval <= 300),
        })
        .collect()
}

/// 从彩种排期中提取周期开奖秒数。
fn draw_interval_seconds(schedule: &DrawSchedule) -> Option<u32> {
    match schedule {
        DrawSchedule::Periodic { interval_seconds }
        | DrawSchedule::TimeNode {
            interval_seconds, ..
        } => Some(*interval_seconds),
        DrawSchedule::Daily { .. } | DrawSchedule::Weekly { .. } => None,
    }
}

/// 从彩种排期中提取每日开奖时间。
fn daily_draw_time(schedule: &DrawSchedule) -> Option<String> {
    match schedule {
        DrawSchedule::Daily { time } => Some(time.clone()),
        DrawSchedule::Periodic { .. }
        | DrawSchedule::TimeNode { .. }
        | DrawSchedule::Weekly { .. } => None,
    }
}

/// 为每个销售中彩种取最近一条开奖记录。
fn latest_history_items(
    lotteries: &[LotteryKind],
    issues: &[DrawIssue],
    query: &LotteryHistoryQuery,
) -> Vec<MobileLotteryHistoryItem> {
    let mut seen_lotteries = BTreeSet::new();
    sorted_history_candidates(lotteries, issues, query)
        .into_iter()
        .filter_map(|(issue, lottery)| {
            if !seen_lotteries.insert(lottery.id.clone()) {
                return None;
            }
            Some(history_item(issue, lottery))
        })
        .collect()
}

/// 按查询条件生成开奖历史分页记录。
fn draw_history_items(
    lotteries: &[LotteryKind],
    issues: &[DrawIssue],
    query: &LotteryHistoryQuery,
) -> Vec<MobileLotteryHistoryItem> {
    sorted_history_candidates(lotteries, issues, query)
        .into_iter()
        .map(|(issue, lottery)| history_item(issue, lottery))
        .collect()
}

/// 按开奖时间倒序整理可展示的历史期号候选。
fn sorted_history_candidates<'a>(
    lotteries: &'a [LotteryKind],
    issues: &'a [DrawIssue],
    query: &LotteryHistoryQuery,
) -> Vec<(&'a DrawIssue, &'a LotteryKind)> {
    let lotteries_by_id = lotteries
        .iter()
        .map(|lottery| (lottery.id.as_str(), lottery))
        .collect::<HashMap<_, _>>();
    let mut candidates = issues
        .iter()
        .filter(|issue| issue.status == DrawIssueStatus::Drawn)
        .filter(|issue| {
            issue
                .draw_number
                .as_deref()
                .is_some_and(|draw_number| !draw_number.trim().is_empty())
        })
        .filter_map(|issue| {
            let lottery = lotteries_by_id.get(issue.lottery_id.as_str()).copied()?;
            if lottery_matches_query(lottery, query) {
                Some((issue, lottery))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|(left, left_lottery), (right, right_lottery)| {
        history_time_value(right)
            .cmp(&history_time_value(left))
            .then_with(|| right.issue.cmp(&left.issue))
            .then_with(|| left_lottery.name.cmp(&right_lottery.name))
    });
    candidates
}

/// 判断彩种是否匹配开奖历史查询条件。
fn lottery_matches_query(lottery: &LotteryKind, query: &LotteryHistoryQuery) -> bool {
    if !lottery.sale_enabled {
        return false;
    }

    if let Some(lottery_code) = normalized_query_value(&query.lottery_code) {
        return lottery.id == lottery_code;
    }

    if let Some(group_code) = normalized_query_value(&query.group_code) {
        return group_code == "all" || lottery.category == group_code;
    }

    true
}

/// 规范化可选查询字符串。
fn normalized_query_value(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// 把开奖期号转换为手机端开奖历史记录。
fn history_item(issue: &DrawIssue, lottery: &LotteryKind) -> MobileLotteryHistoryItem {
    let result = issue.draw_number.clone().unwrap_or_default();

    MobileLotteryHistoryItem {
        id: issue.id.clone(),
        lottery_code: lottery.id.clone(),
        lottery_name: lottery.name.clone(),
        category: lottery.category.clone(),
        logo_url: optional_text(&lottery.logo_url),
        issue: issue.issue.clone(),
        result_numbers: split_draw_number(&result),
        result,
        opened_at: issue
            .drawn_at
            .clone()
            .or_else(|| Some(issue.scheduled_at.clone())),
        status: "drawn".to_string(),
    }
}

/// 把逗号分隔或连续数字开奖号码拆成号码数组。
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

/// 把空字符串规范化为 None。
fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// 计算开奖历史排序使用的时间值。
fn history_time_value(issue: &DrawIssue) -> i64 {
    issue
        .drawn_at
        .as_deref()
        .and_then(parse_timestamp)
        .or_else(|| parse_timestamp(&issue.scheduled_at))
        .map(|value| value.and_utc().timestamp())
        .unwrap_or_default()
}

/// 解析系统内常用时间文本为时间对象。
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
    use super::*;
    use crate::domain::lottery::{
        DrawMode, DrawSchedule, GroupBuyConfig, LotteryNumberType, PlayCategory,
    };

    #[test]
    fn latest_history_returns_one_latest_draw_per_selling_lottery() {
        let lotteries = vec![
            test_lottery("fc3d", "福彩3D", "digit3", true),
            test_lottery("pl3", "排列3", "digit3", true),
            test_lottery("closed", "停售彩", "digit3", false),
        ];
        let issues = vec![
            drawn_issue(
                "old",
                "fc3d",
                "福彩3D",
                "20260605001",
                "1,2,3",
                "2026-06-05 10:00:00",
            ),
            drawn_issue(
                "new",
                "fc3d",
                "福彩3D",
                "20260605002",
                "4,5,6",
                "2026-06-05 11:00:00",
            ),
            drawn_issue(
                "other",
                "pl3",
                "排列3",
                "20260605001",
                "7,8,9",
                "2026-06-05 09:00:00",
            ),
            drawn_issue(
                "hidden",
                "closed",
                "停售彩",
                "20260605001",
                "0,0,0",
                "2026-06-05 12:00:00",
            ),
        ];

        let items = latest_history_items(&lotteries, &issues, &LotteryHistoryQuery::default());

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].lottery_code, "fc3d");
        assert_eq!(items[0].issue, "20260605002");
        assert_eq!(items[0].result_numbers, vec!["4", "5", "6"]);
        assert!(items.iter().all(|item| item.lottery_code != "closed"));
    }

    #[test]
    fn draw_history_can_filter_by_group_and_paginate() {
        let lotteries = vec![
            test_lottery("fc3d", "福彩3D", "digit3", true),
            test_lottery("pl3", "排列3", "digit3", true),
            test_lottery("pk10", "PK10", "pk10", true),
        ];
        let issues = vec![
            drawn_issue("a", "fc3d", "福彩3D", "001", "123", "2026-06-05 10:00:00"),
            drawn_issue("b", "pl3", "排列3", "001", "456", "2026-06-05 11:00:00"),
            drawn_issue(
                "c",
                "pk10",
                "PK10",
                "001",
                "01,02,03",
                "2026-06-05 12:00:00",
            ),
        ];
        let query = LotteryHistoryQuery {
            group_code: Some("digit3".to_string()),
            page: Some(2),
            page_size: Some(1),
            ..LotteryHistoryQuery::default()
        };

        let page = MobileLotteryHistoryPage::from_items(
            draw_history_items(&lotteries, &issues, &query),
            query.page.unwrap(),
            query.page_size,
        );

        assert_eq!(page.total_count, 2);
        assert_eq!(page.total_pages, 2);
        assert_eq!(page.items[0].lottery_code, "fc3d");
    }

    #[test]
    fn lottery_history_groups_only_include_selling_categories() {
        let lotteries = vec![
            test_lottery("fc3d", "福彩3D", "digit3", true),
            test_lottery("pk10", "PK10", "pk10", false),
        ];
        let categories = vec![
            LotteryCategoryConfig {
                code: "digit3".to_string(),
                name: "三位数".to_string(),
            },
            LotteryCategoryConfig {
                code: "pk10".to_string(),
                name: "PK10".to_string(),
            },
        ];

        let groups = lottery_history_groups(&lotteries, &categories);

        assert_eq!(
            groups,
            vec![MobileLotteryHistoryGroup {
                code: "digit3".to_string(),
                name: "三位数".to_string(),
                lotteries: vec![MobileLotteryGroupLottery {
                    code: "fc3d".to_string(),
                    name: "福彩3D".to_string(),
                    category: "digit3".to_string(),
                    logo_url: Some("https://example.test/fc3d.png".to_string()),
                    draw_interval: Some(60),
                    daily_draw_time: None,
                    group_sort_order: 0,
                    is_recommended: true,
                }],
            }]
        );
    }

    fn test_lottery(id: &str, name: &str, category: &str, sale_enabled: bool) -> LotteryKind {
        LotteryKind {
            id: id.to_string(),
            name: name.to_string(),
            category: category.to_string(),
            logo_url: format!("https://example.test/{id}.png"),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Platform,
            api_draw_delay_seconds: 0,
            draw_control_enabled: true,
            issue_format: crate::domain::lottery::DEFAULT_ISSUE_FORMAT_PATTERN.to_string(),
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
            play_categories: vec![PlayCategory::Direct],
            play_configs: Vec::new(),
        }
    }

    fn drawn_issue(
        id: &str,
        lottery_id: &str,
        lottery_name: &str,
        issue: &str,
        draw_number: &str,
        drawn_at: &str,
    ) -> DrawIssue {
        DrawIssue {
            id: id.to_string(),
            lottery_id: lottery_id.to_string(),
            lottery_name: lottery_name.to_string(),
            issue: issue.to_string(),
            number_type: LotteryNumberType::ThreeDigit,
            draw_mode: DrawMode::Platform,
            scheduled_at: drawn_at.to_string(),
            sale_closed_at: drawn_at.to_string(),
            status: DrawIssueStatus::Drawn,
            draw_number: Some(draw_number.to_string()),
            drawn_at: Some(drawn_at.to_string()),
            created_at: drawn_at.to_string(),
        }
    }
}
