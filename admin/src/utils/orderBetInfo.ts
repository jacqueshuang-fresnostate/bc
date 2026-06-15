import type {
  BigSmallOddEvenPosition,
  DigitAttribute,
  PlayRuleCode,
  PlaySelection,
} from '../types/playRules';
import {
  isBankerRule,
  isBigSmallOddEvenRule,
  isDirectRule,
} from './playRules';

export interface BetInfoLine {
  label: string;
  value: string;
}

const digitAttributeLabels: Record<DigitAttribute, string> = {
  big: '大',
  even: '双',
  odd: '单',
  small: '小',
};

const bigSmallOddEvenPositionLabels: Record<BigSmallOddEvenPosition, string> = {
  ones: '个位',
  tens: '十位',
};

/** 把后端投注选择结构转换为后台可读的中文下注信息。 */
export function formatPlaySelection(selection: PlaySelection): BetInfoLine[] {
  const lines: BetInfoLine[] = [];

  selection.positions?.forEach((position, index) => {
    if (position.length > 0) {
      lines.push({
        label: `第 ${index + 1} 位`,
        value: formatDigitList(position),
      });
    }
  });

  if (selection.numbers?.length) {
    lines.push({
      label: '选号',
      value: formatDigitList(selection.numbers),
    });
  }

  if (selection.bankerNumbers?.length) {
    lines.push({
      label: '胆码',
      value: formatDigitList(selection.bankerNumbers),
    });
  }

  if (selection.dragNumbers?.length) {
    lines.push({
      label: '拖码',
      value: formatDigitList(selection.dragNumbers),
    });
  }

  selection.bigSmallOddEven?.forEach((pick) => {
    if (pick.attributes.length > 0) {
      lines.push({
        label: bigSmallOddEvenPositionLabels[pick.position] ?? pick.position,
        value: pick.attributes
          .map((attribute) => digitAttributeLabels[attribute] ?? attribute)
          .join('、'),
      });
    }
  });

  return lines.length > 0
    ? lines
    : [
        {
          label: '投注',
          value: '未提供选号',
        },
      ];
}

/** 把合买计划保存的投注文本按玩法解析为中文展示行；解析失败时保留原文。 */
export function formatGroupBuyNumbersSelection(
  ruleCode: string,
  numbers: string,
): BetInfoLine[] {
  const rawNumbers = numbers.trim();
  if (!rawNumbers) {
    return [{ label: '投注', value: '未提供选号' }];
  }

  const selection = parseGroupBuyNumbers(ruleCode as PlayRuleCode, rawNumbers);
  if (!selection) {
    return [{ label: '原始内容', value: rawNumbers }];
  }

  return formatPlaySelection(selection);
}

/** 生成适合下拉选项或窄区域的下注摘要。 */
export function formatBetInfoSummary(
  selection: PlaySelection,
  expandedBets: string[] = [],
) {
  const selectionText = formatPlaySelection(selection)
    .map((line) => `${line.label}：${line.value}`)
    .join('；');
  if (selectionText && selectionText !== '投注：未提供选号') {
    return selectionText;
  }

  return expandedBets.length > 0
    ? `展开注码：${expandedBets.slice(0, 4).join('、')}${
        expandedBets.length > 4 ? ` 等 ${expandedBets.length} 注` : ''
      }`
    : '暂无下注信息';
}

function parseGroupBuyNumbers(
  ruleCode: PlayRuleCode,
  numbers: string,
): PlaySelection | null {
  if (isDirectRule(ruleCode)) {
    const segments = parsePositionSegments(numbers);
    if (segments.length === 3) {
      const positions = segments.map(parseDigitList);
      return positions.every((position): position is number[] => Boolean(position))
        ? { positions }
        : null;
    }

    const digits = parseDigitList(numbers);
    return digits && digits.length === 3
      ? { positions: digits.map((digit) => [digit]) }
      : null;
  }

  if (isBigSmallOddEvenRule(ruleCode)) {
    const picks = parseBigSmallOddEven(numbers);
    return picks ? { bigSmallOddEven: picks } : null;
  }

  if (isBankerRule(ruleCode)) {
    const segments = parsePositionSegments(numbers);
    if (segments.length !== 2) {
      return null;
    }
    const bankerNumbers = parseDigitList(segments[0]);
    const dragNumbers = parseDigitList(segments[1]);
    return bankerNumbers && dragNumbers ? { bankerNumbers, dragNumbers } : null;
  }

  const firstSegment = numbers.split('|')[0] ?? numbers;
  const parsedNumbers = parseDigitList(firstSegment);
  return parsedNumbers ? { numbers: parsedNumbers } : null;
}

function parseBigSmallOddEven(numbers: string) {
  const segments = parsePositionSegments(numbers);
  if (segments.length === 0 || segments.length > 2) {
    return null;
  }

  const picks = segments.map((segment, index) =>
    parseBigSmallOddEvenSegment(segment, index),
  );
  return picks.every(Boolean)
    ? (picks as NonNullable<(typeof picks)[number]>[])
    : null;
}

function parseBigSmallOddEvenSegment(segment: string, index: number) {
  const [positionText, valueText] = splitBigSmallSegment(segment);
  const position = parseBigSmallPosition(positionText, index);
  const parsedAttributes = splitTokens(valueText).map(parseDigitAttribute);

  if (
    !position ||
    parsedAttributes.length === 0 ||
    parsedAttributes.some((attribute) => !attribute)
  ) {
    return null;
  }
  const attributes = parsedAttributes as DigitAttribute[];
  return { attributes, position };
}

function splitBigSmallSegment(segment: string): [string, string] {
  const englishColonIndex = segment.indexOf(':');
  const chineseColonIndex = segment.indexOf('：');
  const colonIndex =
    englishColonIndex >= 0
      ? englishColonIndex
      : chineseColonIndex >= 0
        ? chineseColonIndex
        : -1;
  if (colonIndex < 0) {
    return ['', segment];
  }
  return [segment.slice(0, colonIndex), segment.slice(colonIndex + 1)];
}

function parseBigSmallPosition(
  value: string,
  fallbackIndex: number,
): BigSmallOddEvenPosition | null {
  const normalized = value.trim().toLowerCase();
  if (normalized === 'tens' || normalized === 'ten' || normalized === '十位') {
    return 'tens';
  }
  if (normalized === 'ones' || normalized === 'one' || normalized === '个位') {
    return 'ones';
  }
  if (!normalized) {
    return fallbackIndex === 0 ? 'tens' : fallbackIndex === 1 ? 'ones' : null;
  }
  return null;
}

function parseDigitAttribute(value: string): DigitAttribute | null {
  const normalized = value.trim().toLowerCase();
  if (normalized === 'big' || normalized === '大') {
    return 'big';
  }
  if (normalized === 'small' || normalized === '小') {
    return 'small';
  }
  if (normalized === 'odd' || normalized === 'single' || normalized === '单') {
    return 'odd';
  }
  if (normalized === 'even' || normalized === '双') {
    return 'even';
  }
  return null;
}

function parsePositionSegments(numbers: string) {
  return numbers
    .split('|')
    .map((segment) => segment.trim())
    .filter(Boolean);
}

function parseDigitList(value: string): number[] | null {
  const digits = splitTokens(value).map((token) => {
    const normalized = token.trim();
    return normalized.length === 1 && /^\d$/.test(normalized)
      ? Number(normalized)
      : Number.NaN;
  });
  return digits.length > 0 && digits.every((digit) => Number.isInteger(digit))
    ? digits
    : null;
}

function splitTokens(value: string) {
  return value
    .split(/[,，\s]+/)
    .map((token) => token.trim())
    .filter(Boolean);
}

function formatDigitList(values: number[]) {
  return values.map((value) => String(value)).join('、');
}
