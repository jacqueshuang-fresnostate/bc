import type { LotteryNumberType } from '../types/dashboard';

export const lotteryNumberTypeOptions: Array<{
  label: string;
  value: LotteryNumberType;
}> = [
  { label: '3 位号码', value: 'threeDigit' },
  { label: '5 位号码', value: 'fiveDigit' },
  { label: 'PK10', value: 'pk10' },
  { label: '11 选 5', value: 'elevenFive' },
  { label: '快 3', value: 'fastThree' },
  { label: '快乐 8 / 幸运 20', value: 'luckTwenty' },
];

export function lotteryNumberTypeText(numberType: LotteryNumberType) {
  const option = lotteryNumberTypeOptions.find((item) => item.value === numberType);
  return option?.label ?? numberType;
}

export function playLotteryNumberTypeText(numberType: LotteryNumberType) {
  if (numberType === 'threeDigit') {
    return '3 位玩法';
  }
  if (numberType === 'fiveDigit') {
    return '5 位玩法';
  }
  return lotteryNumberTypeText(numberType);
}

export function lotteryNumberTypeSupportsPlayRules(numberType: LotteryNumberType) {
  return numberType === 'threeDigit' || numberType === 'fiveDigit';
}

export function drawNumberInputMeta(numberType: LotteryNumberType) {
  const meta: Record<
    LotteryNumberType,
    { maxLength: number; placeholder: string }
  > = {
    elevenFive: {
      maxLength: 14,
      placeholder: '1,3,5,7,11',
    },
    fastThree: {
      maxLength: 5,
      placeholder: '1,4,6',
    },
    fiveDigit: {
      maxLength: 9,
      placeholder: '7,8,9,4,2',
    },
    luckTwenty: {
      maxLength: 80,
      placeholder: '1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20',
    },
    pk10: {
      maxLength: 29,
      placeholder: '1,6,2,4,3,5,7,9,10,8',
    },
    threeDigit: {
      maxLength: 5,
      placeholder: '2,4,7',
    },
  };

  return meta[numberType];
}
