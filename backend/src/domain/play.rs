use serde::{Deserialize, Serialize};

use crate::domain::lottery::{LotteryNumberType, PlayCategory};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub enum ThreeDigitWindow {
    Full,
    Front,
    Middle,
    Back,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BigSmallOddEvenPosition {
    Tens,
    Ones,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DigitAttribute {
    Big,
    Small,
    Odd,
    Even,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BigSmallOddEvenPick {
    pub position: BigSmallOddEvenPosition,
    pub attributes: Vec<DigitAttribute>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
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
pub struct PlayRuleEvaluateRequest {
    pub number_type: LotteryNumberType,
    pub rule_code: PlayRuleCode,
    pub selection: PlaySelection,
    pub draw_number: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
pub struct PlayRuleEvaluation {
    pub rule_code: PlayRuleCode,
    pub stake_count: u32,
    pub expanded_bets: Vec<String>,
    pub is_winning: bool,
    pub matched_bets: Vec<String>,
}
