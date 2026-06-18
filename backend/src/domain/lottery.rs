//! 彩种与开奖来源领域模型，定义开奖方式和销售与合买配置

use serde::{Deserialize, Serialize};

use crate::domain::play::PlayRuleCode;

/// 平台开奖期号默认生成格式，按 `yyyyMMdd` 加 4 位每日递增序号生成。
pub const DEFAULT_ISSUE_FORMAT_PATTERN: &str = "{date}{seq4}";
/// 彩种默认封盘提前秒数，默认开奖前 1 秒停止销售。
pub const DEFAULT_SALE_CLOSE_LEAD_SECONDS: u32 = 1;

/// 彩种分类配置，允许按代码和展示名进行自定义。
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LotteryCategoryConfig {
    /// 彩种分类编码。
    pub code: String,
    /// 彩种分类名称。
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
/// 彩种开奖排期配置，支持相对周期、自然时间节点周期、每日固定时间和每周固定时间。
pub enum DrawSchedule {
    Periodic {
        interval_seconds: u32,
    },
    TimeNode {
        interval_seconds: u32,
        start_time: String,
    },
    Daily {
        time: String,
    },
    Weekly {
        weekdays: Vec<String>,
        time: String,
    },
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
    /// 功能开关。
    pub enabled: bool,
    /// 每份最低金额，单位为分。
    pub min_share_amount_minor: i64,
    /// 发起人最低自购比例。
    pub initiator_min_percent: u8,
    /// 参与人最低认购金额，单位为分。
    pub participant_min_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 单个玩法位置选号数量上限；未配置的位置不限制。
pub struct LotteryPlayPositionSelectLimit {
    /// 玩法位置键，例如第一位、第二位。
    pub position_key: String,
    /// 该位置最多可选择的号码数量。
    pub max_select_count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 彩种单玩法赔率配置，覆盖玩法是否启用和赔率基点。
pub struct LotteryPlayConfig {
    /// 玩法规则编码。
    pub rule_code: PlayRuleCode,
    /// 功能开关。
    pub enabled: bool,
    /// 赔率基点，10000 表示 1 倍。
    pub odds_basis_points: i64,
    /// 各选号位置的最多选择数量限制。
    #[serde(default)]
    pub position_select_limits: Vec<LotteryPlayPositionSelectLimit>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 开奖来源配置，描述外部接口、平台来源和复用彩种绑定。
pub struct DrawSource {
    /// 开奖源 ID。
    pub id: String,
    /// 开奖源展示名称。
    pub name: String,
    /// 该来源对应的开奖模式。
    pub mode: DrawMode,
    /// API 来源供应商；平台或人工来源为空。
    pub provider: Option<DrawSourceProvider>,
    /// 第三方接口彩种编码。
    pub lot_code: Option<String>,
    /// 第三方接口地址；为空时使用数据库默认值。
    pub endpoint: Option<String>,
    /// 后台是否允许编辑该配置。
    pub editable: bool,
    /// 允许复用该开奖源的彩种 ID 列表。
    pub reusable_for_lottery_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 外部开奖源供应商枚举，用于选择不同接口解析器。
pub enum DrawSourceProvider {
    Api68,
    KjApi,
    BbKaijiang,
    IndonesiaLottery,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台创建或编辑 API 开奖源时提交的配置。
pub struct SaveDrawSourceRequest {
    /// 开奖源 ID，编辑时用于定位原配置。
    pub id: String,
    /// 展示名称。
    pub name: String,
    /// 外部开奖源供应商。
    pub provider: DrawSourceProvider,
    /// 第三方开奖源彩种编码。
    pub lot_code: String,
    /// 第三方开奖源接口地址。
    #[serde(default)]
    pub endpoint: Option<String>,
    /// 允许复用该开奖源的彩种 ID 列表。
    pub reusable_for_lottery_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 彩种完整配置，供后台管理、手机端展示和开奖调度共同使用。
pub struct LotteryKind {
    /// 彩种 ID，作为投注、期号和开奖源绑定的主键。
    pub id: String,
    /// 彩种中文名称。
    pub name: String,
    /// 彩种分类编码。
    pub category: LotteryCategory,
    /// 彩种 LOGO 地址，未上传时可为空。
    #[serde(default)]
    pub logo_url: String,
    /// 号码类型，决定开奖号码长度和玩法目录。
    pub number_type: LotteryNumberType,
    /// 开奖模式。
    pub draw_mode: DrawMode,
    /// API 开奖源延迟秒数；只影响 API 模式到点后多久请求第三方开奖号码。
    #[serde(default)]
    pub api_draw_delay_seconds: u32,
    /// 是否允许后台控制开奖号码；关闭后管理端不展示控制入口，接口也不允许启用控制。
    #[serde(default = "default_draw_control_enabled")]
    pub draw_control_enabled: bool,
    /// 平台开奖期号生成格式；仅平台开奖模式按该模板生成期号。
    #[serde(default = "default_issue_format_pattern")]
    pub issue_format: String,
    /// 封盘提前秒数；生成新期号时按开奖时间减去该秒数计算封盘时间。
    #[serde(default = "default_sale_close_lead_seconds")]
    pub sale_close_lead_seconds: u32,
    /// 开奖排期配置。
    pub schedule: DrawSchedule,
    /// 彩种是否销售中。
    pub sale_enabled: bool,
    /// 彩种合买配置。
    pub group_buy: GroupBuyConfig,
    /// 彩种启用的玩法分类。
    pub play_categories: Vec<PlayCategory>,
    /// 彩种下各玩法的赔率与位置限制。
    #[serde(default)]
    pub play_configs: Vec<LotteryPlayConfig>,
}

/// 反序列化旧数据时补齐默认期号格式。
fn default_issue_format_pattern() -> String {
    DEFAULT_ISSUE_FORMAT_PATTERN.to_string()
}

/// 反序列化旧数据时补齐默认封盘提前秒数。
fn default_sale_close_lead_seconds() -> u32 {
    DEFAULT_SALE_CLOSE_LEAD_SECONDS
}

/// 兼容旧数据：历史彩种未保存该字段时默认仍允许控制开奖。
fn default_draw_control_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::DrawSchedule;

    #[test]
    /// 验证开奖排期枚举序列化时保持前端约定的驼峰字段名。
    fn draw_schedule_uses_camel_case_variant_fields() {
        let schedule = DrawSchedule::TimeNode {
            interval_seconds: 300,
            start_time: "00:00:00".to_string(),
        };

        let value = serde_json::to_value(schedule).expect("schedule can be serialized");

        assert_eq!(
            value,
            json!({ "timeNode": { "intervalSeconds": 300, "startTime": "00:00:00" } })
        );
    }

    #[test]
    /// 验证开奖排期枚举可以读取前端提交的驼峰字段名。
    fn draw_schedule_accepts_camel_case_variant_fields() {
        let schedule: DrawSchedule = serde_json::from_value(
            json!({ "timeNode": { "intervalSeconds": 300, "startTime": "00:00:00" } }),
        )
        .expect("schedule can be deserialized");

        assert_eq!(
            schedule,
            DrawSchedule::TimeNode {
                interval_seconds: 300,
                start_time: "00:00:00".to_string()
            }
        );
    }
}
