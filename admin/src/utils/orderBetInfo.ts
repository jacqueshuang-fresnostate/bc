import type {
  BigSmallOddEvenPosition,
  DigitAttribute,
  PlaySelection,
} from '../types/playRules';

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

function formatDigitList(values: number[]) {
  return values.map((value) => String(value)).join('、');
}
