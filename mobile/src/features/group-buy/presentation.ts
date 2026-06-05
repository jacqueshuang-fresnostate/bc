import type { CreateGroupBuyForm, CreateGroupBuyPayload, GroupBuyPlan, SelectOption } from './types'

const planStatusBadges = ['高成功率', '保本 20%', '官方认证', '即将满员']

/** 归一化合买接口返回的列表数据。 */
export function normalizeItems(data: any): GroupBuyPlan[] {
  if (Array.isArray(data?.items)) return data.items
  if (Array.isArray(data)) return data
  return []
}

/** 把后端选项数据转换为下拉选项。 */
export function normalizeOptions(items: any[]): SelectOption[] {
  return items.map(item => ({
    label: String(item.label || item.name || item.issue || item.code || item.value || ''),
    value: String(item.value || item.code || item.issue || ''),
    icon: item.icon ? String(item.icon) : undefined,
  })).filter(option => option.value)
}

/** 从多个可能字段中提取选项数组。 */
export function normalizeOptionPayload(data: any, keys: string[]): SelectOption[] {
  for (const key of keys) {
    if (Array.isArray(data?.[key])) return normalizeOptions(data[key])
  }
  return []
}

/** 计算合买进度百分比并限制最大值。 */
export function progressPercent(item: GroupBuyPlan) {
  return Math.min(100, Number(item.progress_percent || 0))
}

/** 生成合买进度条宽度。 */
export function progressTrackWidth(item: GroupBuyPlan) {
  return `${progressPercent(item)}%`
}

/** 选择合买计划状态徽标。 */
export function planStatusBadge(index: number) {
  return planStatusBadges[index % planStatusBadges.length]
}

/** 根据彩种名称选择大厅图标。 */
export function hallLotteryIcon(item: GroupBuyPlan) {
  const text = `${item.lottery_name}${item.lottery_code}`
  if (text.includes('六合')) return '✿'
  if (text.includes('足球')) return '◎'
  if (text.includes('大乐透')) return '★'
  return '◈'
}

/** 展示合买剩余份数。 */
export function progressRemainingText(item: GroupBuyPlan) {
  if (item.available_shares <= 80) return `仅剩 ${item.available_shares} 份`
  return `剩余 ${item.available_shares.toLocaleString()} 份`
}

/** 判断合买计划是否允许认购。 */
export function canJoinPlan(item: GroupBuyPlan) {
  return item.status === 'open' && item.available_shares > 0
}

/** 格式化移动端合买金额。 */
export function formatMoney(value: string | number | null | undefined) {
  const amount = Number(value || 0)
  return Number.isFinite(amount) ? `¥${amount.toFixed(2)}` : `¥${String(value || '0.00')}`
}

/** 展示合买发起人名称。 */
export function initiatorDisplay(item: GroupBuyPlan) {
  return item.initiator_display || '平台发起'
}

/** 展示合买计划标题。 */
export function formatPlanTitle(item: GroupBuyPlan) {
  return item.title || `${item.lottery_name || item.lottery_code} 第${item.issue}期`
}

/** 展示合买玩法中文名，接口暂缺时才回退编码。 */
export function formatPlayName(item: GroupBuyPlan) {
  return item.play_name || item.play_code || '-'
}

/** 把合买状态转换为移动端中文文案。 */
export function statusText(status: string) {
  if (status === 'draft') return '草稿'
  if (status === 'open') return '可参与'
  if (status === 'full' || status === 'filled') return '已满员'
  if (status === 'settled') return '已结算'
  if (status === 'cancelled') return '已取消'
  if (status === 'expired') return '已流局'
  return status || '-'
}

/** 创建移动端发起合买默认表单。 */
export function createDefaultGroupBuyForm(lotteryCode: string): CreateGroupBuyForm {
  return {
    lottery_code: lotteryCode,
    issue: '',
    play_code: '',
    title: '用户发起合买',
    numbers: '',
    total_amount: '10.00',
    share_count: 10,
    share_amount: '1.00',
    self_shares: 1,
  }
}

/** 按固定每份金额计算发起合买份数，不能整除时返回0。 */
export function calculateFixedShareCount(totalAmount: string | number, shareAmount: string | number) {
  const totalCents = parseScaledDecimal(totalAmount, 2)
  const shareCents = parseScaledDecimal(shareAmount, 2)
  if (totalCents <= 0 || shareCents <= 0 || totalCents % shareCents !== 0) return 0
  return totalCents / shareCents
}

/** 计算发起合买需要支付的金额。 */
export function calculateCreatePaymentAmount(shareAmount: string, selfShares: number) {
  const amount = Number(shareAmount || 0) * Number(selfShares || 0)
  return Number.isFinite(amount) ? amount.toFixed(2) : '0.00'
}

function parseScaledDecimal(value: string | number, decimals: number) {
  const text = String(value ?? '').trim()
  if (!/^\d+(?:\.\d+)?$/.test(text)) return 0
  const [wholeText, decimalText = ''] = text.split('.')
  const scale = 10 ** decimals
  const whole = Number(wholeText)
  const fraction = Number(decimalText.padEnd(decimals, '0').slice(0, decimals) || '0')
  if (!Number.isSafeInteger(whole) || whole > Number.MAX_SAFE_INTEGER / scale) return 0
  return whole * scale + fraction
}

/** 计算发起人最低自购份数。 */
export function calculateRequiredSelfShares(totalAmount: string | number, shareAmount: string | number, ratio: string | number) {
  const totalCents = parseScaledDecimal(totalAmount, 2)
  const shareCents = parseScaledDecimal(shareAmount, 2)
  const ratioBp = parseScaledDecimal(ratio, 2)
  if (totalCents <= 0 || shareCents <= 0 || ratioBp <= 0) return 0
  const minimumCents = Math.ceil((totalCents * ratioBp) / 10000)
  return Math.ceil(minimumCents / shareCents)
}

/** 判断福彩3D合买是否仍在使用无逗号旧号码。 */
export function isLegacyPlainFc3dNumbers(lotteryCode: string, numbers: string) {
  return lotteryCode === 'fc3d' && /^\d{2,}$/.test(String(numbers || '').trim())
}

/** 构建移动端创建合买请求体。 */
export function buildCreateGroupBuyPayload(form: CreateGroupBuyForm, fallbackLotteryCode: string, shareAmount: string, shareCount: number): CreateGroupBuyPayload {
  return {
    lottery_code: form.lottery_code || fallbackLotteryCode,
    issue: form.issue,
    play_code: form.play_code,
    title: form.title,
    numbers: form.numbers,
    total_amount: form.total_amount,
    share_count: Number(shareCount),
    share_amount: shareAmount,
    reserved_shares: 0,
    self_shares: Number(form.self_shares),
  }
}
