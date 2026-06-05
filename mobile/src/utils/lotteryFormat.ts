const statusTextMap: Record<string, string> = {
  pending: '待开奖',
  pendingDraw: '待开奖',
  drawn: '已开奖',
  won: '已中奖',
  lost: '未中奖',
  cancelled: '已取消',
}

export type OrderMatchItem = {
  label: string
  value: string
  detail: string
  tone: 'hit' | 'miss' | 'pending'
}

export type OrderBetContentValue = {
  label: string
  key: string
}

export type OrderBetContentGroup = {
  kind: 'numbers' | 'attributes'
  label: string
  values: OrderBetContentValue[]
  key: string
}

const BIG_SMALL_ODD_EVEN_POSITION_LABELS: Record<string, string> = {
  tens: '十位',
  ones: '个位',
  ten: '十位',
  one: '个位',
  '十位': '十位',
  '个位': '个位',
}

const BIG_SMALL_ODD_EVEN_POSITION_KEYS: Record<string, string> = {
  tens: 'tens',
  ones: 'ones',
  ten: 'tens',
  one: 'ones',
  '十位': 'tens',
  '个位': 'ones',
}

const BIG_SMALL_ODD_EVEN_ATTRIBUTE_LABELS: Record<string, string> = {
  big: '大',
  small: '小',
  odd: '单',
  even: '双',
  large: '大',
  大: '大',
  小: '小',
  单: '单',
  双: '双',
}

const BIG_SMALL_ODD_EVEN_ATTRIBUTE_KEYS: Record<string, string> = {
  big: 'big',
  small: 'small',
  odd: 'odd',
  even: 'even',
  large: 'big',
  大: 'big',
  小: 'small',
  单: 'odd',
  双: 'even',
}

export function statusText(status?: string) {
  return statusTextMap[status || ''] || status || '-'
}

export function splitNumbers(value: unknown) {
  if (Array.isArray(value)) return value.map(item => String(item).trim()).filter(Boolean)
  const text = String(value || '').trim()
  if (!text) return []
  if (/^\d+$/.test(text) && text.length > 1) return text.match(/\d{1,2}/g) || []
  return text.split(/[\s,，|/]+/).map(item => item.trim()).filter(Boolean)
}

export function orderBetCount(order: any) {
  const count = Number(order?.bet_count ?? 1)
  return Number.isSafeInteger(count) && count > 0 ? count : 1
}

export function orderMultiple(order: any) {
  const multiple = Number(order?.multiple ?? 1)
  return Number.isSafeInteger(multiple) && multiple > 0 ? multiple : 1
}

export function orderUnitAmount(order: any) {
  const configured = Number(order?.unit_amount)
  if (Number.isFinite(configured) && configured > 0) return configured
  const amount = Number(order?.amount || 0)
  return amount > 0 ? amount / orderBetCount(order) / orderMultiple(order) : 0
}

function orderRuleCode(order: any) {
  return String(order?.rule_code || order?.ruleCode || '').trim()
}

function normalizedRuleCode(order: any) {
  return orderRuleCode(order).toLowerCase()
}

function orderPositionGridKind(order: any) {
  const configuredKind = String(order?.position_grid_kind || '').trim()
  if (configuredKind) return configuredKind

  const ruleCode = normalizedRuleCode(order)
  if (ruleCode === 'fivebigsmalloddeven') return 'big_small_odd_even'
  if (/groupthreebanker/.test(ruleCode)) return 'group3_dantuo'
  if (/groupsixbanker/.test(ruleCode)) return 'group6_dantuo'
  if (/groupthree/.test(ruleCode)) return 'group3_compound'
  if (/groupsix/.test(ruleCode)) return 'group6_compound'
  if (/direct/.test(ruleCode)) return 'direct'
  return ''
}

function isBigSmallOddEvenOrder(order: any) {
  return orderPositionGridKind(order) === 'big_small_odd_even'
    || normalizedRuleCode(order) === 'fivebigsmalloddeven'
}

function bigSmallOddEvenPositionLabel(value: unknown) {
  const text = String(value || '').trim()
  const key = text.toLowerCase()
  return BIG_SMALL_ODD_EVEN_POSITION_LABELS[text] || BIG_SMALL_ODD_EVEN_POSITION_LABELS[key] || text || '位置'
}

function bigSmallOddEvenPositionKey(value: unknown) {
  const text = String(value || '').trim()
  const key = text.toLowerCase()
  return BIG_SMALL_ODD_EVEN_POSITION_KEYS[text] || BIG_SMALL_ODD_EVEN_POSITION_KEYS[key] || key || text
}

function bigSmallOddEvenAttributeLabel(value: unknown) {
  const text = String(value || '').trim()
  const key = text.toLowerCase()
  return BIG_SMALL_ODD_EVEN_ATTRIBUTE_LABELS[text] || BIG_SMALL_ODD_EVEN_ATTRIBUTE_LABELS[key] || text
}

function bigSmallOddEvenAttributeKey(value: unknown) {
  const text = String(value || '').trim()
  const key = text.toLowerCase()
  return BIG_SMALL_ODD_EVEN_ATTRIBUTE_KEYS[text] || BIG_SMALL_ODD_EVEN_ATTRIBUTE_KEYS[key] || key || text
}

function bigSmallOddEvenValues(values: unknown) {
  const items = Array.isArray(values)
    ? values
    : String(values || '').split(/[,，、/\s]+/)
  return items
    .map(value => ({
      label: bigSmallOddEvenAttributeLabel(value),
      key: bigSmallOddEvenAttributeKey(value),
    }))
    .filter(value => value.label)
}

function bigSmallOddEvenGroupsFromSelection(selection: any): OrderBetContentGroup[] {
  const items: any[] = Array.isArray(selection?.bigSmallOddEven) ? selection.bigSmallOddEven : []
  return items
    .map((item: any): OrderBetContentGroup => {
      const positionKey = bigSmallOddEvenPositionKey(item?.position)
      const values = bigSmallOddEvenValues(item?.attributes)
      return {
        kind: 'attributes' as const,
        label: bigSmallOddEvenPositionLabel(item?.position),
        values: values.map(value => ({ ...value, key: `${positionKey}:${value.key}` })),
        key: `${positionKey}:${values.map(value => value.key).join(',')}`,
      }
    })
    .filter(group => group.values.length > 0)
}

function bigSmallOddEvenGroupsFromText(value: unknown): OrderBetContentGroup[] {
  const text = String(value || '').trim()
  if (!text) return []
  return text
    .split(/[|;；]+/)
    .map(segment => segment.trim())
    .filter(Boolean)
    .map((segment): OrderBetContentGroup => {
      const [position = '', attributes = ''] = segment.split(/[:：]/).map(item => item.trim())
      const positionKey = bigSmallOddEvenPositionKey(position)
      const values = bigSmallOddEvenValues(attributes || position)
      return {
        kind: 'attributes' as const,
        label: attributes ? bigSmallOddEvenPositionLabel(position) : '投注属性',
        values: values.map(value => ({ ...value, key: attributes ? `${positionKey}:${value.key}` : value.key })),
        key: attributes ? `${positionKey}:${values.map(value => value.key).join(',')}` : values.map(value => value.key).join(','),
      }
    })
    .filter(group => group.values.length > 0)
}

function orderBigSmallOddEvenGroups(order: any): OrderBetContentGroup[] {
  const selectionGroups = bigSmallOddEvenGroupsFromSelection(order?.selection || {})
  if (selectionGroups.length) return selectionGroups
  return bigSmallOddEvenGroupsFromText(order?.numbers || order?.canonical_numbers)
}

function splitOrderDigits(value: unknown) {
  const text = String(value || '').trim().replace(/，/g, ',')
  if (!text) return []
  if (text.includes(',')) return text.split(',').map(item => item.trim()).filter(Boolean)
  return Array.from(text).map(item => item.trim()).filter(Boolean)
}

function splitCompactBetDigits(value: unknown) {
  const text = String(value || '').trim().replace(/，/g, ',')
  if (!text) return []
  if (/^\d+$/.test(text)) return Array.from(text)
  return text.split(/[\s,]+/).map(item => item.trim()).filter(Boolean)
}

function uniqueValues(values: string[]) {
  return Array.from(new Set(values))
}

function orderPositionSegments(value: unknown) {
  return String(value || '').trim().split('|').map(splitOrderDigits)
}

function valueCombinations(values: string[], size: number): string[][] {
  if (size === 0) return [[]]
  if (values.length < size) return []
  return values.flatMap((value, index) => valueCombinations(values.slice(index + 1), size - 1).map(items => [value, ...items]))
}

function valuePermutations(values: string[], size: number): string[][] {
  if (size === 0) return [[]]
  if (values.length < size) return []
  return values.flatMap((value, index) => {
    const rest = values.slice(0, index).concat(values.slice(index + 1))
    return valuePermutations(rest, size - 1).map(items => [value, ...items])
  })
}

function expandDirectOrderNumbers(value: unknown) {
  const segments = orderPositionSegments(value)
  if (segments.length >= 2 && segments.every(segment => segment.length)) {
    return segments.reduce<string[]>((items, segment) => items.flatMap(prefix => segment.map(value => (prefix ? `${prefix},${value}` : value))), [''])
  }
  const digits = splitOrderDigits(value)
  return digits.length ? [digits.join(',')] : []
}

function expandGroup3CompoundOrderNumbers(value: unknown) {
  const digits = uniqueValues(splitOrderDigits(value))
  if (digits.length < 2) return []
  return digits.flatMap(repeated => digits.filter(single => single !== repeated).map(single => `${repeated},${repeated},${single}`))
}

function expandDirectCombinationOrderNumbers(value: unknown) {
  return valuePermutations(uniqueValues(splitOrderDigits(value)), 3).map(items => items.join(','))
}

function expandGroup3DantuoOrderNumbers(value: unknown) {
  const [danSegment = [], tuoSegment = []] = orderPositionSegments(value)
  const dan = uniqueValues(danSegment)
  const tuo = uniqueValues(tuoSegment).filter(value => !dan.includes(value))
  if (dan.length !== 1 || !tuo.length) return []
  const danValue = dan[0]
  return tuo.flatMap(tuoValue => [`${danValue},${danValue},${tuoValue}`, `${danValue},${tuoValue},${tuoValue}`])
}

function expandGroup6CompoundOrderNumbers(value: unknown) {
  return valueCombinations(uniqueValues(splitOrderDigits(value)), 3).map(items => items.join(','))
}

function expandGroup6DantuoOrderNumbers(value: unknown) {
  const [danSegment = [], tuoSegment = []] = orderPositionSegments(value)
  const dan = uniqueValues(danSegment)
  const tuo = uniqueValues(tuoSegment).filter(value => !dan.includes(value))
  return valueCombinations(tuo, 3 - dan.length).map(items => [...dan, ...items].join(','))
}

export function orderBetNumbers(order: any) {
  if (!order) return []
  if (isBigSmallOddEvenOrder(order)) {
    return orderBigSmallOddEvenGroups(order).map(group => `${group.label}：${group.values.map(value => value.label).join('、')}`)
  }
  const kind = orderPositionGridKind(order)
  const expanded = kind === 'direct'
    ? expandDirectOrderNumbers(order.numbers)
    : kind === 'direct_combination'
      ? expandDirectCombinationOrderNumbers(order.numbers)
      : kind === 'group3_compound'
        ? expandGroup3CompoundOrderNumbers(order.numbers)
        : kind === 'group3_dantuo'
          ? expandGroup3DantuoOrderNumbers(order.numbers)
          : kind === 'group6_compound'
            ? expandGroup6CompoundOrderNumbers(order.numbers)
            : kind === 'group6_dantuo'
              ? expandGroup6DantuoOrderNumbers(order.numbers)
              : []
  return expanded.length ? expanded : splitNumbers(order.numbers)
}

function betNumberParts(value: string) {
  return String(value || '').split(/[,，|]/).map(item => item.trim()).filter(Boolean)
}

export function orderBetContentText(order: any) {
  if (isBigSmallOddEvenOrder(order)) {
    const groups = orderBigSmallOddEvenGroups(order)
    if (groups.length) {
      return groups.map(group => `${group.label}：${group.values.map(value => value.label).join('、')}`).join('；')
    }
  }
  return String(order?.numbers || order?.canonical_numbers || '').trim() || orderBetNumbers(order).join(' ') || '-'
}

export function orderBetContentGroups(order: any, fallbackNumbers: string[] = []): OrderBetContentGroup[] {
  if (isBigSmallOddEvenOrder(order)) return orderBigSmallOddEvenGroups(order)
  const numbers = fallbackNumbers.length ? fallbackNumbers : orderBetNumbers(order)
  return numbers
    .map(rawValue => {
      const values = betNumberParts(rawValue).map(value => ({ label: value, key: value }))
      return {
        kind: 'numbers' as const,
        label: '',
        values,
        key: values.map(value => value.label).join('').replace(/\s+/g, ''),
      }
    })
    .filter(group => group.values.length > 0)
}

function ruleMatchKind(order: any) {
  const ruleCode = normalizedRuleCode(order)
  if (ruleCode === 'fivebigsmalloddeven') return 'big_small_odd_even'
  if (/directcombination/.test(ruleCode)) return 'direct_combination'
  if (/groupthree/.test(ruleCode)) return 'group_three'
  if (/groupsix/.test(ruleCode)) return 'group_six'
  if (/direct/.test(ruleCode)) return 'direct'
  return 'unknown'
}

function isBankerRule(order: any) {
  return /banker/.test(normalizedRuleCode(order))
}

function ruleWindowText(order: any) {
  const ruleCode = normalizedRuleCode(order)
  if (ruleCode === 'fivebigsmalloddeven') return '后两位'
  if (ruleCode.startsWith('fivefront')) return '前 3 位'
  if (ruleCode.startsWith('fivemiddle')) return '中 3 位'
  if (ruleCode.startsWith('fiveback')) return '后 3 位'
  return '完整 3 位'
}

function drawWindowNumbers(order: any, drawNumbers: string[]) {
  const ruleCode = normalizedRuleCode(order)
  if (!drawNumbers.length) return []
  if (ruleCode === 'fivebigsmalloddeven' && drawNumbers.length >= 5) return drawNumbers.slice(3, 5)
  if (ruleCode.startsWith('fivefront') && drawNumbers.length >= 3) return drawNumbers.slice(0, 3)
  if (ruleCode.startsWith('fivemiddle') && drawNumbers.length >= 4) return drawNumbers.slice(1, 4)
  if (ruleCode.startsWith('fiveback') && drawNumbers.length >= 5) return drawNumbers.slice(2, 5)
  return drawNumbers.slice(0, 3)
}

function matchWindowDetail(order: any, drawNumbers: string[]) {
  const windowNumbers = drawWindowNumbers(order, drawNumbers)
  return windowNumbers.length ? `${ruleWindowText(order)}：${windowNumbers.join(',')}` : ruleWindowText(order)
}

function matchKey(value: unknown) {
  return String(value || '').trim().replace(/[,，\s]+/g, '')
}

export function orderMatchedBetValues(order: any) {
  const values = order?.matched_bets ?? order?.matchedBets
  if (Array.isArray(values)) return values.map(value => String(value).trim()).filter(Boolean)
  return []
}

export function orderMatchedBetKeys(order: any) {
  return orderMatchedBetValues(order).map(matchKey).filter(Boolean)
}

function matchItemLabel(order: any) {
  const kind = ruleMatchKind(order)
  if (kind === 'direct_combination') return '直选组合匹配'
  if (kind === 'group_three') return isBankerRule(order) ? '组三胆拖匹配' : '组三匹配'
  if (kind === 'group_six') return isBankerRule(order) ? '组六胆拖匹配' : '组六匹配'
  if (kind === 'big_small_odd_even') return '大小单双匹配'
  if (kind === 'direct') return '直选匹配'
  return '号码匹配'
}

function missItemText(order: any) {
  const kind = ruleMatchKind(order)
  if (kind === 'direct_combination') return '直选组合未命中'
  if (kind === 'group_three') return isBankerRule(order) ? '组三胆拖未命中' : '组三未命中'
  if (kind === 'group_six') return isBankerRule(order) ? '组六胆拖未命中' : '组六未命中'
  if (kind === 'big_small_odd_even') return '大小单双未命中'
  if (kind === 'direct') return '直选未命中'
  return '未命中'
}

function matchItemDetail(order: any, drawNumbers: string[]) {
  const kind = ruleMatchKind(order)
  const windowDetail = matchWindowDetail(order, drawNumbers)
  if (kind === 'direct') return `${windowDetail}，按位完全一致`
  if (kind === 'direct_combination') return `${windowDetail}，排列组合命中`
  if (kind === 'group_three' || kind === 'group_six') {
    return isBankerRule(order) ? `${windowDetail}，由胆码和拖码组成` : `${windowDetail}，数字一致，顺序不限`
  }
  if (kind === 'big_small_odd_even') return `${windowDetail}，属性命中`
  return windowDetail
}

function displayBetCode(value: unknown) {
  const digits = splitCompactBetDigits(value)
  return digits.length ? digits.join(',') : String(value || '-')
}

function bigSmallOddEvenMatchItem(value: string, order: any, drawNumbers: string[]) {
  const [position, attribute] = value.split(':').map(item => item.trim())
  const positionLabel = bigSmallOddEvenPositionLabel(position)
  const label = positionLabel !== '位置' ? `${positionLabel}匹配` : matchItemLabel(order)
  const attrText = bigSmallOddEvenAttributeLabel(attribute) || attribute || value
  return {
    label,
    value: attrText,
    detail: matchItemDetail(order, drawNumbers),
    tone: 'hit' as const,
  }
}

export function orderMatchItems(order: any, drawNumbers: string[] = []) {
  const status = String(order?.status || '').trim()
  const draw = drawNumbers.length ? drawNumbers : orderDrawNumbers(order)
  if (status === 'pending' || status === 'pendingDraw') {
    return [{
      label: '待开奖',
      value: '开奖后显示匹配项',
      detail: `${ruleWindowText(order)}，以实际开奖号码为准`,
      tone: 'pending' as const,
    }]
  }
  if (status === 'cancelled') {
    return [{
      label: '已取消',
      value: '不参与开奖匹配',
      detail: '本注单已取消，不再计算命中项',
      tone: 'miss' as const,
    }]
  }
  if (!draw.length) {
    return [{
      label: '暂无开奖',
      value: '暂无匹配项',
      detail: '拿到开奖号码后会展示命中明细',
      tone: 'pending' as const,
    }]
  }

  const matchedValues = orderMatchedBetValues(order)
  if (!matchedValues.length) {
    return [{
      label: '未命中',
      value: missItemText(order),
      detail: `${matchWindowDetail(order, draw)}未匹配投注内容`,
      tone: 'miss' as const,
    }]
  }

  return matchedValues.map((value, index) => {
    if (ruleMatchKind(order) === 'big_small_odd_even') return bigSmallOddEvenMatchItem(value, order, draw)
    return {
      label: matchedValues.length > 1 ? `${matchItemLabel(order)} ${index + 1}` : matchItemLabel(order),
      value: displayBetCode(value),
      detail: matchItemDetail(order, draw),
      tone: 'hit' as const,
    }
  })
}

export function formatNumbers(item: any) {
  if (Array.isArray(item.result_numbers) && item.result_numbers.length) return item.result_numbers.join(' ')
  return item.result || '待开奖'
}

export function drawNumbers(item: any) {
  if (Array.isArray(item.result_numbers) && item.result_numbers.length) return splitNumbers(item.result_numbers)
  return splitNumbers(item.result)
}

export function lotteryLogoUrl(item: any) {
  return String(item && item.logo_url ? item.logo_url : '').trim()
}

export function isAccentBall(index: number, item: any) {
  const count = drawNumbers(item).length
  return count > 4 && index === count - 1
}

const CHINA_TIMEZONE = 'Asia/Shanghai'
const CHINA_DATE_TIME_FORMATTER = new Intl.DateTimeFormat('zh-CN', {
  timeZone: CHINA_TIMEZONE,
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
})

function formatDateParts(date: Date) {
  const parts = Object.fromEntries(CHINA_DATE_TIME_FORMATTER.formatToParts(date).map(part => [part.type, part.value]))
  return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}:${parts.second}`
}

function normalizeDisplayDateTime(value: string) {
  const text = value.trim()
  const match = text.match(/^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})(?::(\d{2}))?/)
  if (!match || /(?:z|[+-]\d{2}:?\d{2})$/i.test(text)) return null
  return `${match[1]}-${match[2]}-${match[3]} ${match[4]}:${match[5]}:${match[6] || '00'}`
}

export function parseChinaDateTime(value: unknown) {
  if (value == null || value === '') return NaN
  if (value instanceof Date) return value.getTime()
  if (typeof value === 'number') return value
  const text = String(value).trim()
  if (!text) return NaN
  const match = text.match(/^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})(?::(\d{2}))?$/)
  if (match && !/(?:z|[+-]\d{2}:?\d{2})$/i.test(text)) {
    return Date.UTC(Number(match[1]), Number(match[2]) - 1, Number(match[3]), Number(match[4]) - 8, Number(match[5]), Number(match[6] || 0))
  }
  const timestamp = Date.parse(text)
  return Number.isNaN(timestamp) ? NaN : timestamp
}

export function formatDateTime(value: unknown, fallback = '-') {
  if (value == null || value === '') return fallback
  if (typeof value === 'string') {
    const displayText = normalizeDisplayDateTime(value)
    if (displayText) return displayText
  }
  const timestamp = parseChinaDateTime(value)
  if (Number.isNaN(timestamp)) return fallback
  return formatDateParts(new Date(timestamp))
}

export function formatOpenedAt(item: any) {
  return item.opened_at ? formatDateTime(item.opened_at, statusText(item.status)) : statusText(item.status)
}

export function formatMoneyValue(value: unknown) {
  const amount = Number(value || 0)
  if (!Number.isFinite(amount)) return String(value || '0.00')
  return amount.toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 })
}

export function moneyText(value: unknown) {
  return `¥${formatMoneyValue(value)}`
}

export function signedMoneyText(value: unknown) {
  return `+¥${formatMoneyValue(value)}`
}

export function orderTone(status?: string) {
  if (status === 'won') return 'won'
  if (status === 'lost' || status === 'cancelled') return 'lost'
  return 'pending'
}

export function orderStatusIcon(status?: string) {
  if (status === 'won') return '✓'
  if (status === 'lost' || status === 'cancelled') return '×'
  return '⌛'
}

export function orderTagText(order: any) {
  return order.play_name || order.play_code || '投注'
}

export function isGroupBuyOrder(order: any) {
  return Boolean(
    order?.is_group_buy
      || order?.orderSource === 'groupBuy'
      || order?.order_source === 'groupBuy'
      || order?.order_source === 'group_buy'
      || String(order?.source_name || '').startsWith('group_buy:'),
  )
}

export function orderSourceText(order: any) {
  return isGroupBuyOrder(order) ? '合买下单' : '独立下单'
}

export function orderAmountText(order: any) {
  const payout = Number(order.payout || 0)
  if (order.status === 'won' && payout > 0) return signedMoneyText(order.payout)
  return moneyText(order.amount || '0.00')
}

export function orderResultLabel(order: any) {
  if (order.status === 'pending') return '预计结果'
  if (order.status === 'cancelled') return '处理状态'
  return '中奖金额'
}

export function orderResultText(order: any) {
  const payout = Number(order.payout || 0)
  if (order.status === 'won' && payout > 0) return signedMoneyText(order.payout)
  if (order.status === 'pending') return '待开奖'
  if (order.status === 'cancelled') return '已取消'
  return '¥0.00'
}

export function detailHeroAmount(order: any) {
  if (order.status === 'won' && Number(order.payout || 0) > 0) return signedMoneyText(order.payout)
  if (order.status === 'pending') return '待开奖'
  if (order.status === 'cancelled') return '已取消'
  return '¥0.00'
}

export function detailHeroNote(order: any) {
  if (order.status === 'won' && Number(order.payout || 0) > 0) return '奖金已自动派发至您的余额'
  if (order.status === 'pending') return '开奖后将自动结算本注单'
  if (order.status === 'cancelled') return '本注单已取消，不参与结算'
  return '本期未中奖，感谢参与'
}

export function orderDrawNumbers(order: any) {
  if (Array.isArray(order.result_numbers) && order.result_numbers.length) return splitNumbers(order.result_numbers)
  if (Array.isArray(order.draw_numbers) && order.draw_numbers.length) return splitNumbers(order.draw_numbers)
  return splitNumbers(order.result || order.draw_result)
}

export function orderNumber(order: any) {
  if (order.order_no) return order.order_no
  if (order.order_number) return order.order_number
  if (order.serial_no) return order.serial_no
  return `ZD${String(order.id || '').padStart(10, '0')}`
}
