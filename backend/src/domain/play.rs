//! 玩法规则领域模型，定义玩法代码、选号和评估结果

use serde::{Deserialize, Serialize};

use crate::domain::lottery::{LotteryNumberType, PlayCategory};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 玩法规则代码，作为彩种赔率配置、下注和计奖的统一标识。
pub enum PlayRuleCode {
    ThreeDirect,
    ThreeGroupThree,
    ThreeGroupThreeBanker,
    ThreeGroupSix,
    ThreeGroupSixBanker,
    FiveFrontDirect,
    FiveMiddleDirect,
    FiveBackDirect,
    FiveFrontDirectCombination,
    FiveMiddleDirectCombination,
    FiveBackDirectCombination,
    FiveFrontGroupThree,
    FiveMiddleGroupThree,
    FiveBackGroupThree,
    FiveFrontGroupThreeBanker,
    FiveMiddleGroupThreeBanker,
    FiveBackGroupThreeBanker,
    FiveFrontGroupSix,
    FiveMiddleGroupSix,
    FiveBackGroupSix,
    FiveFrontGroupSixBanker,
    FiveMiddleGroupSixBanker,
    FiveBackGroupSixBanker,
    FiveBigSmallOddEven,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 三位窗口枚举，五位彩种前三、中三、后三玩法也复用该概念。
pub enum ThreeDigitWindow {
    Full,
    Front,
    Middle,
    Back,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 大小单双玩法的投注位置。
pub enum BigSmallOddEvenPosition {
    Tens,
    Ones,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 大小单双玩法可选择的数字属性。
pub enum DigitAttribute {
    Big,
    Small,
    Odd,
    Even,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 大小单双在某个位置上的属性选择。
pub struct BigSmallOddEvenPick {
    pub position: BigSmallOddEvenPosition,
    pub attributes: Vec<DigitAttribute>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
/// 统一选号结构，按玩法使用位置选号、复式号码、胆拖或大小单双属性。
pub struct PlaySelection {
    #[serde(default)]
    pub positions: Vec<Vec<u8>>,
    #[serde(default)]
    pub numbers: Vec<u8>,
    #[serde(default)]
    pub banker_numbers: Vec<u8>,
    #[serde(default)]
    pub drag_numbers: Vec<u8>,
    #[serde(default)]
    pub big_small_odd_even: Vec<BigSmallOddEvenPick>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 后台玩法规则验证请求，用于计算注数和中奖匹配项。
pub struct PlayRuleEvaluateRequest {
    pub number_type: LotteryNumberType,
    pub rule_code: PlayRuleCode,
    pub selection: PlaySelection,
    pub draw_number: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 玩法规则目录项，供后台配置赔率和手机端展示玩法说明。
pub struct PlayRuleSummary {
    pub code: PlayRuleCode,
    pub label: String,
    pub number_type: LotteryNumberType,
    pub category: PlayCategory,
    pub window: ThreeDigitWindow,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// 玩法评估结果，返回展开注码、中奖标记和命中投注。
pub struct PlayRuleEvaluation {
    pub rule_code: PlayRuleCode,
    pub stake_count: u32,
    pub expanded_bets: Vec<String>,
    pub is_winning: bool,
    pub matched_bets: Vec<String>,
}
