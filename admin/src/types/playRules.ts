import type { LotteryNumberType } from './dashboard';

export type PlayRuleCode =
  | 'threeDirect'
  | 'threeGroupThree'
  | 'threeGroupThreeBanker'
  | 'threeGroupSix'
  | 'threeGroupSixBanker'
  | 'fiveFrontDirect'
  | 'fiveMiddleDirect'
  | 'fiveBackDirect'
  | 'fiveFrontDirectCombination'
  | 'fiveMiddleDirectCombination'
  | 'fiveBackDirectCombination'
  | 'fiveFrontGroupThree'
  | 'fiveMiddleGroupThree'
  | 'fiveBackGroupThree'
  | 'fiveFrontGroupThreeBanker'
  | 'fiveMiddleGroupThreeBanker'
  | 'fiveBackGroupThreeBanker'
  | 'fiveFrontGroupSix'
  | 'fiveMiddleGroupSix'
  | 'fiveBackGroupSix'
  | 'fiveFrontGroupSixBanker'
  | 'fiveMiddleGroupSixBanker'
  | 'fiveBackGroupSixBanker'
  | 'fiveBigSmallOddEven';

export type ThreeDigitWindow = 'full' | 'front' | 'middle' | 'back';
export type BigSmallOddEvenPosition = 'tens' | 'ones';
export type DigitAttribute = 'big' | 'small' | 'odd' | 'even';

export interface BigSmallOddEvenPick {
  position: BigSmallOddEvenPosition;
  attributes: DigitAttribute[];
}

export interface PlaySelection {
  bankerNumbers?: number[];
  bigSmallOddEven?: BigSmallOddEvenPick[];
  dragNumbers?: number[];
  numbers?: number[];
  positions?: number[][];
}

export interface PlayRuleSummary {
  code: PlayRuleCode;
  description: string;
  label: string;
  numberType: LotteryNumberType;
  window: ThreeDigitWindow;
}

export interface PlayRuleEvaluateRequest {
  drawNumber: string;
  numberType: LotteryNumberType;
  ruleCode: PlayRuleCode;
  selection: PlaySelection;
}

export interface PlayRuleEvaluation {
  expandedBets: string[];
  isWinning: boolean;
  matchedBets: string[];
  ruleCode: PlayRuleCode;
  stakeCount: number;
}
