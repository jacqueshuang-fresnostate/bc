//! 彩种与开奖来源领域模型，定义开奖方式和销售与合买配置

use serde::{Deserialize, Serialize};

use crate::domain::play::PlayRuleCode;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LotteryNumberType {
    ThreeDigit,
    FiveDigit,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DrawMode {
    Platform,
    Api,
    Manual,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DrawSchedule {
    Periodic { interval_seconds: u32 },
    Daily { time: String },
    Weekly { weekdays: Vec<String>, time: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlayCategory {
    Direct,
    GroupThree,
    GroupSix,
    DirectCombination,
    BigSmallOddEven,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GroupBuyConfig {
    pub enabled: bool,
    pub min_share_amount_minor: i64,
    pub initiator_min_percent: u8,
    pub participant_min_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LotteryPlayConfig {
    pub rule_code: PlayRuleCode,
    pub enabled: bool,
    pub odds_basis_points: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DrawSource {
    pub id: String,
    pub name: String,
    pub mode: DrawMode,
    pub provider: Option<DrawSourceProvider>,
    pub lot_code: Option<String>,
    pub endpoint: Option<String>,
    pub editable: bool,
    pub reusable_for_lottery_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DrawSourceProvider {
    Api68,
    KjApi,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveDrawSourceRequest {
    pub id: String,
    pub name: String,
    pub provider: DrawSourceProvider,
    pub lot_code: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    pub reusable_for_lottery_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
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
    #[serde(default)]
    pub play_configs: Vec<LotteryPlayConfig>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::DrawSchedule;

    #[test]
    /// 处理 draw_schedule_uses_camel_case_variant_fields 的具体内部流程。
    fn draw_schedule_uses_camel_case_variant_fields() {
        let schedule = DrawSchedule::Periodic {
            interval_seconds: 60,
        };

        let value = serde_json::to_value(schedule).expect("schedule can be serialized");

        assert_eq!(value, json!({ "periodic": { "intervalSeconds": 60 } }));
    }

    #[test]
    /// 处理 draw_schedule_accepts_camel_case_variant_fields 的具体内部流程。
    fn draw_schedule_accepts_camel_case_variant_fields() {
        let schedule: DrawSchedule =
            serde_json::from_value(json!({ "periodic": { "intervalSeconds": 60 } }))
                .expect("schedule can be deserialized");

        assert_eq!(
            schedule,
            DrawSchedule::Periodic {
                interval_seconds: 60
            }
        );
    }
}
