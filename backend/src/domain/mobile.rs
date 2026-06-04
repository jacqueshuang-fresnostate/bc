//! 手机端公开配置与首页聚合领域模型，供手机端应用读取基础展示信息。

use serde::{Deserialize, Serialize};

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
