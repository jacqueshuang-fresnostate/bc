use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LotteryNumberType {
    ThreeDigit,
    FiveDigit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawMode {
    Platform,
    Api,
    Manual,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DrawSchedule {
    Periodic { interval_seconds: u32 },
    Daily { time: String },
    Weekly { weekdays: Vec<String>, time: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlayCategory {
    Direct,
    GroupThree,
    GroupSix,
    DirectCombination,
    BigSmallOddEven,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyConfig {
    pub enabled: bool,
    pub min_share_amount_minor: i64,
    pub initiator_min_percent: u8,
    pub participant_min_amount_minor: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawSource {
    pub id: String,
    pub name: String,
    pub mode: DrawMode,
    pub reusable_for_lottery_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LotteryKind {
    pub id: String,
    pub name: String,
    pub number_type: LotteryNumberType,
    pub draw_mode: DrawMode,
    pub schedule: DrawSchedule,
    pub sale_enabled: bool,
    pub group_buy: GroupBuyConfig,
    pub play_categories: Vec<PlayCategory>,
}
