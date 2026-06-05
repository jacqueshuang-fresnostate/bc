//! 手机端公开配置与首页聚合领域模型，供手机端应用读取基础展示信息。

use serde::{Deserialize, Serialize};

use crate::domain::{
    order::OrderDetail,
    play::{PlayRuleCode, PlaySelection},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileSiteConfig {
    pub platform_name: String,
    pub logo_image_url: Option<String>,
    pub intro: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryHomeSettings {
    pub banners_enabled: bool,
    pub ticker_enabled: bool,
    pub featured_enabled: bool,
    pub groups_enabled: bool,
    pub stats_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryTickerItem {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryTicker {
    pub enabled: bool,
    pub items: Vec<MobileLotteryTickerItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryLatestDraw {
    pub issue: String,
    pub result_numbers: Vec<String>,
    pub opened_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryCard {
    pub code: String,
    pub name: String,
    pub category: String,
    pub logo_url: Option<String>,
    pub issue: Option<String>,
    pub status: String,
    pub next_draw_time: Option<String>,
    pub sale_stop_time: Option<String>,
    pub draw_interval: Option<u32>,
    pub draw_time_text: String,
    pub schedule_text: String,
    pub latest_result: Vec<String>,
    pub result_style: String,
    pub result_count: usize,
    pub group_buy_enabled: bool,
    pub latest_draw: Option<MobileLotteryLatestDraw>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryGroup {
    pub code: String,
    pub name: String,
    pub lotteries: Vec<MobileLotteryCard>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryFeaturedSection {
    pub enabled: bool,
    pub title: String,
    pub lotteries: Vec<MobileLotteryCard>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryStats {
    pub today_winner_count: u64,
    pub total_payout_display: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileLotteryHomeResponse {
    pub server_time: String,
    pub settings: MobileLotteryHomeSettings,
    pub ticker: MobileLotteryTicker,
    pub featured_section: MobileLotteryFeaturedSection,
    pub groups: Vec<MobileLotteryGroup>,
    pub stats: MobileLotteryStats,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetLottery {
    pub code: String,
    pub name: String,
    pub category: String,
    pub draw_interval: u32,
    pub group_buy_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetGroupBuySettings {
    pub min_share_amount: String,
    pub initiator_min_buy_ratio: String,
    pub share_amount: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetRound {
    pub issue: String,
    pub status: String,
    pub scheduled_draw_at: Option<String>,
    pub sale_stop_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetLatestDraw {
    pub issue: String,
    pub result_numbers: Vec<String>,
    pub opened_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetPosition {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetOption {
    pub value: String,
    pub label: String,
    pub description: String,
    pub odds: Option<String>,
    pub disabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetOptionGroup {
    pub key: String,
    pub label: String,
    pub min_select_count: u32,
    pub max_select_count: u32,
    pub options: Vec<MobileBetOption>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetPlay {
    pub code: PlayRuleCode,
    pub name: String,
    pub full_name: String,
    pub rule_code: PlayRuleCode,
    pub input_mode: String,
    pub positions: Vec<MobileBetPosition>,
    pub digits: Vec<String>,
    pub number_grid_values: Vec<String>,
    pub option_value: Option<String>,
    pub min_select_count: u32,
    pub bet_number_count: u32,
    pub odds: String,
    pub unit_amount_fixed: bool,
    pub unit_amount: String,
    pub multiple_fixed: bool,
    pub multiple: u32,
    pub min_multiple: u32,
    pub max_multiple: Option<u32>,
    pub simple_description: String,
    pub detail_description: String,
    pub example_description: String,
    pub position_grid_kind: String,
    pub max_select_per_position: Option<u32>,
    pub option_groups: Vec<MobileBetOptionGroup>,
    pub option_groups_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileBetPageConfig {
    pub lottery: MobileBetLottery,
    pub group_buy_settings: MobileBetGroupBuySettings,
    pub round: MobileBetRound,
    pub latest_draw: Option<MobileBetLatestDraw>,
    pub plays: Vec<MobileBetPlay>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileCreateBetOrderRequest {
    pub lottery_id: String,
    pub issue: String,
    pub rule_code: PlayRuleCode,
    pub selection: PlaySelection,
    pub unit_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileCreateBetOrderBatchRequest {
    pub orders: Vec<MobileCreateBetOrderRequest>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileCreateBetOrderBatchResponse {
    pub orders: Vec<OrderDetail>,
}
