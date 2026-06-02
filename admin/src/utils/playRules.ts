import type { PlayCategory } from '../types/dashboard';
import type { PlayRuleCode } from '../types/playRules';

export function playCategoryForRule(code: PlayRuleCode): PlayCategory {
  if (isDirectRule(code)) {
    return 'direct';
  }
  if (code.endsWith('DirectCombination')) {
    return 'directCombination';
  }
  if (code.includes('GroupThree')) {
    return 'groupThree';
  }
  if (code.includes('GroupSix')) {
    return 'groupSix';
  }
  return 'bigSmallOddEven';
}

export function isDirectRule(code: PlayRuleCode) {
  return code.endsWith('Direct') && !code.endsWith('DirectCombination');
}

export function isBigSmallOddEvenRule(code: PlayRuleCode) {
  return code === 'fiveBigSmallOddEven';
}

export function isBankerRule(code: PlayRuleCode) {
  return code.endsWith('Banker');
}

export function isGroupSixRule(code: PlayRuleCode) {
  return code.includes('GroupSix');
}

export function formatOdds(oddsBasisPoints: number) {
  return `${(oddsBasisPoints / 10000).toFixed(2)} 倍`;
}

export function oddsBasisPointsToInput(oddsBasisPoints: number) {
  return (oddsBasisPoints / 10000).toFixed(2);
}

export function oddsInputToBasisPoints(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? Math.round(parsed * 10000) : 0;
}
