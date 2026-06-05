const statusTextMap: Record<string, string> = {
  pending: '待开奖',
  pendingDraw: '待开奖',
  drawn: '已开奖',
  won: '已中奖',
  lost: '未中奖',
  cancelled: '已取消',
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

function orderPlaySnapshot(order: any) {
  return order?.play_snapshot && typeof order.play_snapshot === 'object' ? order.play_snapshot : {}
}

export function orderBetCount(order: any) {
  const snapshot = orderPlaySnapshot(order)
  const count = Number(order?.bet_count ?? snapshot.bet_count ?? 1)
  return Number.isSafeInteger(count) && count > 0 ? count : 1
}

export function orderMultiple(order: any) {
  const snapshot = orderPlaySnapshot(order)
  const multiple = Number(order?.multiple ?? snapshot.multiple ?? 1)
  return Number.isSafeInteger(multiple) && multiple > 0 ? multiple : 1
}

export function orderUnitAmount(order: any) {
  const snapshot = orderPlaySnapshot(order)
  const configured = Number(order?.unit_amount ?? snapshot.unit_amount)
  if (Number.isFinite(configured) && configured > 0) return configured
  const amount = Number(order?.amount || 0)
  return amount > 0 ? amount / orderBetCount(order) / orderMultiple(order) : 0
}

function orderDescriptor(order: any) {
  const snapshot = orderPlaySnapshot(order)
  return [
    order?.rule_code,
    snapshot.rule_code,
    order?.play_code,
    order?.play_name,
    snapshot.name,
  ].map(value => String(value || '').trim().toLowerCase()).filter(Boolean).join(' ')
}

function orderPositionGridKind(order: any) {
  const snapshot = orderPlaySnapshot(order)
  const configuredKind = String(order?.position_grid_kind || snapshot.position_grid_kind || '').trim()
  if (configuredKind) return configuredKind

  const descriptor = orderDescriptor(order)
  if (/group3_dantuo|zuxuan3_dantuo|组三胆拖/.test(descriptor)) return 'group3_dantuo'
  if (/group6_dantuo|zuxuan6_dantuo|组六胆拖/.test(descriptor)) return 'group6_dantuo'
  if (/group3_compound|zuxuan3|组三/.test(descriptor)) return 'group3_compound'
  if (/group6_compound|zuxuan6|组六/.test(descriptor)) return 'group6_compound'
  if (/\.direct\b|\bdirect\b|zhixuan|直选/.test(descriptor)) return 'direct'
  return ''
}

function splitOrderDigits(value: unknown) {
  const text = String(value || '').trim().replace(/，/g, ',')
  if (!text) return []
  if (text.includes(',')) return text.split(',').map(item => item.trim()).filter(Boolean)
  return Array.from(text).map(item => item.trim()).filter(Boolean)
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
  return Boolean(order?.is_group_buy || String(order?.source_name || '').startsWith('group_buy:'))
}

export function orderSourceText(order: any) {
  return isGroupBuyOrder(order) ? '合买' : '普通'
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
