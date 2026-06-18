//! 手机端公开配置与首页聚合领域模型，供手机端应用读取基础展示信息。

use serde::{Deserialize, Serialize};

use crate::domain::{
    lottery::LotteryPlayPositionSelectLimit,
    order::OrderDetail,
    play::{PlayRuleCode, PlaySelection},
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端站点基础配置，包含平台名称、Logo 和简介。
pub struct MobileSiteConfig {
    /// 手机端展示的平台名称。
    pub platform_name: String,
    /// 手机端平台 Logo 图片地址。
    pub logo_image_url: Option<String>,
    /// intro字段。
    pub intro: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端 APP 更新检查结果，供 Android/iOS 客户端启动时判断是否需要提示更新。
pub struct MobileAppUpdateConfig {
    /// 应用平台，例如 android 或 ios。
    pub platform: String,
    /// 功能开关。
    pub enabled: bool,
    /// 最新应用版本号。
    pub latest_version: String,
    /// 最新应用构建号。
    pub latest_build: u32,
    /// 安装包下载地址。
    pub download_url: Option<String>,
    /// 是否强制更新。
    pub force_update: bool,
    /// 版本更新说明。
    pub release_notes: String,
    /// 当前客户端是否需要更新。
    pub update_available: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页各模块开关，决定轮播、公告、高频和统计是否展示。
pub struct MobileLotteryHomeSettings {
    /// 首页轮播是否启用。
    pub banners_enabled: bool,
    /// 首页滚动公告是否启用。
    pub ticker_enabled: bool,
    /// 首页高频极速模块是否启用。
    pub featured_enabled: bool,
    /// 首页彩种分组是否启用。
    pub groups_enabled: bool,
    /// 首页统计卡片是否启用。
    pub stats_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页公告或滚动提示的单条内容。
pub struct MobileLotteryTickerItem {
    /// 业务唯一标识。
    pub id: String,
    /// 公告或提示文本。
    pub text: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页公告模块数据。
pub struct MobileLotteryTicker {
    /// 功能开关。
    pub enabled: bool,
    /// 分页数据列表。
    pub items: Vec<MobileLotteryTickerItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页彩种最近一期开奖记录。
pub struct MobileLotteryLatestDraw {
    /// 彩票期号。
    pub issue: String,
    /// 已拆分的开奖号码列表。
    pub result_numbers: Vec<String>,
    /// 最近期开奖时间。
    pub opened_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页彩种卡片，聚合销售状态、倒计时和最近开奖号。
pub struct MobileLotteryCard {
    /// 业务编码，用于接口传参和前端筛选。
    pub code: String,
    /// 展示名称。
    pub name: String,
    /// 彩种分类编码。
    pub category: String,
    /// 彩种 LOGO 图片地址。
    pub logo_url: Option<String>,
    /// 彩票期号。
    pub issue: Option<String>,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: String,
    /// 外部开奖源提示的下一期开奖时间。
    pub next_draw_time: Option<String>,
    /// 下一期停止销售时间。
    pub sale_stop_time: Option<String>,
    /// 开奖周期秒数。
    pub draw_interval: Option<u32>,
    /// 开奖时间展示文案。
    pub draw_time_text: String,
    /// 开奖排期展示文案。
    pub schedule_text: String,
    /// 最近一期开奖号码列表。
    pub latest_result: Vec<String>,
    /// 前端展示开奖号码的样式类型。
    pub result_style: String,
    /// 开奖号码数量。
    pub result_count: usize,
    /// 手机端是否展示合买入口。
    pub group_buy_enabled: bool,
    /// 最新开奖字段。
    pub latest_draw: Option<MobileLotteryLatestDraw>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页按分类分组后的彩种列表。
pub struct MobileLotteryGroup {
    /// 业务编码，用于接口传参和前端筛选。
    pub code: String,
    /// 展示名称。
    pub name: String,
    /// 可选彩种列表。
    pub lotteries: Vec<MobileLotteryCard>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页高频极速推荐区配置和推荐彩种。
pub struct MobileLotteryFeaturedSection {
    /// 功能开关。
    pub enabled: bool,
    /// 展示标题。
    pub title: String,
    /// 可选彩种列表。
    pub lotteries: Vec<MobileLotteryCard>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页统计卡片数据。
pub struct MobileLotteryStats {
    /// todaywinnercount字段。
    pub today_winner_count: u64,
    /// total派奖display字段。
    pub total_payout_display: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端首页聚合响应，统一返回站点时间、模块配置和分组彩种。
pub struct MobileLotteryHomeResponse {
    /// 服务端当前时间。
    pub server_time: String,
    /// 移动端或模块配置集合。
    pub settings: MobileLotteryHomeSettings,
    /// 首页滚动公告配置。
    pub ticker: MobileLotteryTicker,
    /// 首页高频极速区域配置。
    pub featured_section: MobileLotteryFeaturedSection,
    /// 首页按分类分组的彩种列表。
    pub groups: Vec<MobileLotteryGroup>,
    /// 首页统计数据。
    pub stats: MobileLotteryStats,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端下注页当前彩种基础信息。
pub struct MobileBetLottery {
    /// 业务编码，用于接口传参和前端筛选。
    pub code: String,
    /// 展示名称。
    pub name: String,
    /// 彩种分类编码。
    pub category: String,
    /// 开奖周期秒数。
    pub draw_interval: u32,
    /// 手机端是否展示合买入口。
    pub group_buy_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端下注页合买设置展示值。
pub struct MobileBetGroupBuySettings {
    /// 每份金额展示文案。
    pub min_share_amount: String,
    /// 发起人最低自购比例展示文案。
    pub initiator_min_buy_ratio: String,
    /// 单份金额展示文案。
    pub share_amount: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端下注页当前期号与销售状态。
pub struct MobileBetRound {
    /// 彩票期号。
    pub issue: String,
    /// 业务状态，用于筛选、禁用或流转。
    pub status: String,
    /// 计划开奖时间。
    pub scheduled_draw_at: Option<String>,
    /// 停止销售时间。
    pub sale_stop_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端下注页展示的最近期开奖结果。
pub struct MobileBetLatestDraw {
    /// 彩票期号。
    pub issue: String,
    /// 已拆分的开奖号码列表。
    pub result_numbers: Vec<String>,
    /// 最近期开奖时间。
    pub opened_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端玩法选号位置定义，例如百位、十位、个位。
pub struct MobileBetPosition {
    /// 配置键或选项键。
    pub key: String,
    /// 前端展示文案。
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端玩法选项，支持号码、属性和赔率展示。
pub struct MobileBetOption {
    /// 配置值或选项值。
    pub value: String,
    /// 前端展示文案。
    pub label: String,
    /// 配置或记录的中文说明。
    pub description: String,
    /// odds字段。
    pub odds: Option<String>,
    /// 选项是否不可选择。
    pub disabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端组选项组，用于大小单双等按位置选择的玩法。
pub struct MobileBetOptionGroup {
    /// 配置键或选项键。
    pub key: String,
    /// 前端展示文案。
    pub label: String,
    /// 该选号位至少选择数量。
    pub min_select_count: u32,
    /// 该位置最多可选择的号码数量。
    pub max_select_count: u32,
    /// 该选号位可选号码列表。
    pub options: Vec<MobileBetOption>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端单个玩法配置，包含选号模式、赔率、示例和选项组。
pub struct MobileBetPlay {
    /// 业务编码，用于接口传参和前端筛选。
    pub code: PlayRuleCode,
    /// 展示名称。
    pub name: String,
    /// fullname字段。
    pub full_name: String,
    /// 玩法规则编码。
    pub rule_code: PlayRuleCode,
    /// inputmode字段。
    pub input_mode: String,
    /// 直选玩法各位置选号。
    pub positions: Vec<MobileBetPosition>,
    /// 数字字段。
    pub digits: Vec<String>,
    /// 号码grid值字段。
    pub number_grid_values: Vec<String>,
    /// option值字段。
    pub option_value: Option<String>,
    /// 该选号位至少选择数量。
    pub min_select_count: u32,
    /// bet号码count字段。
    pub bet_number_count: u32,
    /// odds字段。
    pub odds: String,
    /// unitamountfixed字段。
    pub unit_amount_fixed: bool,
    /// unitamount字段。
    pub unit_amount: String,
    /// multiplefixed字段。
    pub multiple_fixed: bool,
    /// multiple字段。
    pub multiple: u32,
    /// minmultiple字段。
    pub min_multiple: u32,
    /// maxmultiple字段。
    pub max_multiple: Option<u32>,
    /// simpledescription字段。
    pub simple_description: String,
    /// detaildescription字段。
    pub detail_description: String,
    /// exampledescription字段。
    pub example_description: String,
    /// positiongridkind字段。
    pub position_grid_kind: String,
    /// maxselectperposition字段。
    pub max_select_per_position: Option<u32>,
    /// 各选号位置的最多选择数量限制。
    #[serde(default)]
    pub position_select_limits: Vec<LotteryPlayPositionSelectLimit>,
    /// option分组字段。
    pub option_groups: Vec<MobileBetOptionGroup>,
    /// option分组错误字段。
    pub option_groups_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端下注页完整配置，进入下注页时一次性返回。
pub struct MobileBetPageConfig {
    /// 彩种字段。
    pub lottery: MobileBetLottery,
    /// 合买合买settings字段。
    pub group_buy_settings: MobileBetGroupBuySettings,
    /// round字段。
    pub round: MobileBetRound,
    /// 最新开奖字段。
    pub latest_draw: Option<MobileBetLatestDraw>,
    /// 可选玩法列表。
    pub plays: Vec<MobileBetPlay>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端创建单笔投注订单时提交的投注选择。
pub struct MobileCreateBetOrderRequest {
    /// 彩种 ID。
    pub lottery_id: String,
    /// 彩票期号。
    pub issue: String,
    /// 玩法规则编码。
    pub rule_code: PlayRuleCode,
    /// 用户选择的投注号码结构。
    pub selection: PlaySelection,
    /// 单注金额，单位为分。
    pub unit_amount_minor: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端批量提交购彩篮订单时提交的订单集合。
pub struct MobileCreateBetOrderBatchRequest {
    /// 结算涉及的订单列表。
    pub orders: Vec<MobileCreateBetOrderRequest>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 手机端批量投注成功后返回的订单详情集合。
pub struct MobileCreateBetOrderBatchResponse {
    /// 结算涉及的订单列表。
    pub orders: Vec<OrderDetail>,
}
