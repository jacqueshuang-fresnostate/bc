//! 彩种与开奖来源领域模型，定义开奖方式和销售与合买配置

use serde::{Deserialize, Serialize};

use crate::domain::play::PlayRuleCode;

/// 彩种分类配置，允许按代码和展示名进行自定义。
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LotteryCategoryConfig {
    pub code: String,
    pub name: String,
}

/// 彩种分类标识，采用可扩展文本编码。
pub type LotteryCategory = String;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 彩种号码类型，决定开奖号码长度、玩法目录和号码格式校验。
pub enum LotteryNumberType {
    ThreeDigit,
    FiveDigit,
    Pk10,
    ElevenFive,
    FastThree,
    LuckTwenty,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 彩种开奖模式，区分平台生成、外部 API 采集和人工开奖。
pub enum DrawMode {
    Platform,
    Api,
    Manual,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
/// 彩种开奖排期配置，支持周期、每日固定时间和每周固定时间。
pub enum DrawSchedule {
    Periodic { interval_seconds: u32 },
    Daily { time: String },
    Weekly { weekdays: Vec<String>, time: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 彩种启用的玩法粗分类，用于后台筛选和按号码类型补齐玩法。
pub enum PlayCategory {
    Direct,
    GroupThree,
    GroupSix,
    DirectCombination,
    BigSmallOddEven,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 彩种合买配置，控制合买开关、单份金额和参与门槛。
pub struct GroupBuyConfig {
    pub enabled: bool,
    pub min_share_amount_minor: i64,
    pub initiator_min_percent: u8,
    pub participant_min_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个玩法位置选号数量上限；未配置的位置不限制。
pub struct LotteryPlayPositionSelectLimit {
    pub position_key: String,
    pub max_select_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 彩种单玩法赔率配置，覆盖玩法是否启用和赔率基点。
pub struct LotteryPlayConfig {
    pub rule_code: PlayRuleCode,
    pub enabled: bool,
    pub odds_basis_points: i64,
    #[serde(default)]
    pub position_select_limits: Vec<LotteryPlayPositionSelectLimit>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 开奖来源配置，描述外部接口、平台来源和复用彩种绑定。
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
/// 外部开奖源供应商枚举，用于选择不同接口解析器。
pub enum DrawSourceProvider {
    Api68,
    KjApi,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台创建或编辑 API 开奖源时提交的配置。
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
/// 彩种完整配置，供后台管理、手机端展示和开奖调度共同使用。
pub struct LotteryKind {
    pub id: String,
    pub name: String,
    pub category: LotteryCategory,
    /// 彩种 LOGO 地址，未上传时可为空。
    #[serde(default)]
    pub logo_url: String,
    pub number_type: LotteryNumberType,
    pub draw_mode: DrawMode,
    /// API 开奖源延迟秒数；只影响 API 模式到点后多久请求第三方开奖号码。
    #[serde(default)]
    pub api_draw_delay_seconds: u32,
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
