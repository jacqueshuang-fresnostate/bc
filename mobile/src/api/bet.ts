import http from './http'
import { unwrapApiData, type UserListQuery } from './user'

export type PlaySelection = {
  positions?: number[][]
  numbers?: number[]
  bankerNumbers?: number[]
  dragNumbers?: number[]
  bigSmallOddEven?: Array<{ position: string; attributes: string[] }>
}

export type CreateUserBetOrderPayload = {
  lotteryId: string
  issue: string
  ruleCode: string
  selection: PlaySelection
  unitAmountMinor: number
}

export type CreateUserBetOrdersResponse = {
  orders: UserBetOrderDetail[]
}

export type UserBetOrderDetail = {
  id: string
  orderSource: 'direct' | 'groupBuy'
  userId: string
  lotteryId: string
  lotteryName: string
  issue: string
  ruleCode: string
  numberType: string
  selection: PlaySelection
  stakeCount: number
  unitAmountMinor: number
  amountMinor: number
  groupBuyPlanId?: string | null
  groupBuyPlanStatus?: string | null
  groupBuyPendingPlan?: boolean
  participationAmountMinor?: number | null
  participationShareCount?: number | null
  participationPayoutMinor?: number | null
  oddsBasisPoints: number
  expandedBets: string[]
  drawNumber?: string | null
  matchedBets: string[]
  payoutMinor: number
  status: string
  settledAt?: string | null
  createdAt: string
}

const playNameMap: Record<string, string> = {
  threeDirect: '3 位直选',
  threeGroupThree: '3 位组三复式',
  threeGroupThreeBanker: '3 位组三胆拖',
  threeGroupSix: '3 位组六复式',
  threeGroupSixBanker: '3 位组六胆拖',
  fiveFrontDirect: '前 3 直选',
  fiveMiddleDirect: '中 3 直选',
  fiveBackDirect: '后 3 直选',
  fiveFrontDirectCombination: '前 3 直选组合',
  fiveMiddleDirectCombination: '中 3 直选组合',
  fiveBackDirectCombination: '后 3 直选组合',
  fiveFrontGroupThree: '前 3 组三复式',
  fiveMiddleGroupThree: '中 3 组三复式',
  fiveBackGroupThree: '后 3 组三复式',
  fiveFrontGroupThreeBanker: '前 3 组三胆拖',
  fiveMiddleGroupThreeBanker: '中 3 组三胆拖',
  fiveBackGroupThreeBanker: '后 3 组三胆拖',
  fiveFrontGroupSix: '前 3 组六复式',
  fiveMiddleGroupSix: '中 3 组六复式',
  fiveBackGroupSix: '后 3 组六复式',
  fiveFrontGroupSixBanker: '前 3 组六胆拖',
  fiveMiddleGroupSixBanker: '中 3 组六胆拖',
  fiveBackGroupSixBanker: '后 3 组六胆拖',
  fiveBigSmallOddEven: '大小单双',
}

const statusMap: Record<string, string> = {
  pendingDraw: 'pending',
  won: 'won',
  lost: 'lost',
  cancelled: 'cancelled',
}

export async function fetchUserBetPageConfig(lotteryCode: string) {
  return unwrapApiData<any>(
    await http.get(`/user/bet/page-config/${encodeURIComponent(lotteryCode)}`),
  )
}

export async function createUserBetOrders(orders: CreateUserBetOrderPayload[]) {
  return unwrapApiData<CreateUserBetOrdersResponse>(
    await http.post('/user/bet/orders', { orders }),
  )
}

export async function fetchUserBetOrders(query: UserListQuery = {}) {
  const orders = unwrapApiData<UserBetOrderDetail[]>(await http.get('/user/bet/orders', {
    params: normalizeUserListQuery(query),
  }))
  return orders.map(normalizeUserBetOrder)
}

function normalizeUserListQuery(query: UserListQuery) {
  return {
    ...(query.page ? { page: query.page } : {}),
    ...(query.pageSize ? { pageSize: query.pageSize } : {}),
  }
}

function formatMinorAmount(value: number) {
  return (Number(value || 0) / 100).toFixed(2)
}

function normalizeOptionalMinor(value?: number | null) {
  if (value === null || value === undefined) return null
  const amount = Number(value)
  return Number.isFinite(amount) ? amount : null
}

function splitDrawNumber(value?: string | null) {
  const text = String(value || '').trim()
  if (!text) return []
  if (text.includes(',') || text.includes('，')) {
    return text.split(/[,，]/).map(item => item.trim()).filter(Boolean)
  }
  return Array.from(text)
}

function numbersText(values?: number[]) {
  return Array.isArray(values) ? values.map(String).join(',') : ''
}

function positionText(values?: number[][]) {
  return Array.isArray(values) ? values.map(numbersText).join('|') : ''
}

function bigSmallOddEvenText(selection: PlaySelection) {
  const positionTextMap: Record<string, string> = { tens: '十位', ones: '个位' }
  const attrTextMap: Record<string, string> = { big: '大', small: '小', odd: '单', even: '双' }
  return (selection.bigSmallOddEven || [])
    .map(item => {
      const label = positionTextMap[item.position] || item.position
      const attrs = (item.attributes || []).map(attr => attrTextMap[attr] || attr).join(',')
      return attrs ? `${label}:${attrs}` : ''
    })
    .filter(Boolean)
    .join('|')
}

function selectionNumbers(order: UserBetOrderDetail) {
  const selection = order.selection || {}
  if (Array.isArray(selection.positions) && selection.positions.length) return positionText(selection.positions)
  if (selection.bankerNumbers?.length || selection.dragNumbers?.length) {
    return `${numbersText(selection.bankerNumbers)}|${numbersText(selection.dragNumbers)}`
  }
  if (selection.numbers?.length) return numbersText(selection.numbers)
  if (selection.bigSmallOddEven?.length) return bigSmallOddEvenText(selection)
  return (order.expandedBets || []).join('|')
}

function positionGridKind(ruleCode: string) {
  if (ruleCode === 'fiveBigSmallOddEven') return 'big_small_odd_even'
  if (/DirectCombination$/.test(ruleCode)) return 'direct_combination'
  if (/GroupThreeBanker$/.test(ruleCode) || ruleCode === 'threeGroupThreeBanker') return 'group3_dantuo'
  if (/GroupSixBanker$/.test(ruleCode) || ruleCode === 'threeGroupSixBanker') return 'group6_dantuo'
  if (/GroupThree$/.test(ruleCode) || ruleCode === 'threeGroupThree') return 'group3_compound'
  if (/GroupSix$/.test(ruleCode) || ruleCode === 'threeGroupSix') return 'group6_compound'
  return 'direct'
}

export function normalizeUserBetOrder(order: UserBetOrderDetail) {
  const rawOrder = order as UserBetOrderDetail & {
    created_at?: string
    draw_number?: string | null
    draw_result?: string | null
    group_buy_pending_plan?: boolean
    group_buy_plan_id?: string | null
    group_buy_plan_status?: string | null
    participation_share_count?: number | null
    result?: string | null
    settled_at?: string | null
  }
  const numbers = selectionNumbers(order)
  const isGroupBuy = order.orderSource === 'groupBuy'
  const groupBuyPlanStatus = order.groupBuyPlanStatus || rawOrder.group_buy_plan_status || null
  const groupBuyPendingPlan = Boolean(order.groupBuyPendingPlan || rawOrder.group_buy_pending_plan)
  const drawNumber = order.drawNumber || rawOrder.draw_number || rawOrder.draw_result || rawOrder.result || ''
  const participationAmountMinor = normalizeOptionalMinor(order.participationAmountMinor)
  const participationPayoutMinor = normalizeOptionalMinor(order.participationPayoutMinor)
  const participationShareCount = Number(order.participationShareCount ?? rawOrder.participation_share_count ?? 0)
  const participationAmount = participationAmountMinor !== null
    ? formatMinorAmount(participationAmountMinor)
    : undefined
  const participationPayout = participationPayoutMinor !== null
    ? formatMinorAmount(participationPayoutMinor)
    : undefined
  const displayPayoutMinor = isGroupBuy && participationPayoutMinor !== null
    ? participationPayoutMinor
    : order.payoutMinor
  const normalizedStatus = groupBuyPendingPlan
    ? groupBuyPlanStatus === 'cancelled' ? 'cancelled' : 'groupBuyPending'
    : statusMap[order.status] || order.status
  const odds = Number(order.oddsBasisPoints || 0) > 0
    ? formatMinorAmount(order.oddsBasisPoints / 100)
    : ''
  return {
    ...order,
    order_source: order.orderSource,
    source_name: order.orderSource,
    is_group_buy: isGroupBuy,
    group_buy_plan_id: order.groupBuyPlanId || rawOrder.group_buy_plan_id || null,
    group_buy_plan_status: groupBuyPlanStatus,
    groupBuyPlanStatus,
    group_buy_pending_plan: groupBuyPendingPlan,
    groupBuyPendingPlan,
    lottery_code: order.lotteryId,
    lottery_name: order.lotteryName,
    play_code: order.ruleCode,
    play_name: playNameMap[order.ruleCode] || order.ruleCode,
    rule_code: order.ruleCode,
    numbers,
    canonical_numbers: numbers,
    result: drawNumber,
    draw_result: drawNumber,
    result_numbers: splitDrawNumber(drawNumber),
    draw_numbers: splitDrawNumber(drawNumber),
    matched_bets: order.matchedBets || [],
    expanded_bets: order.expandedBets || [],
    status: normalizedStatus,
    bet_count: order.stakeCount,
    unit_amount: formatMinorAmount(order.unitAmountMinor),
    multiple: 1,
    amount: formatMinorAmount(order.amountMinor),
    participation_amount_minor: participationAmountMinor,
    participation_amount: participationAmount,
    participation_share_count: Number.isFinite(participationShareCount) ? participationShareCount : 0,
    participationShareCount: Number.isFinite(participationShareCount) ? participationShareCount : 0,
    participation_payout_minor: participationPayoutMinor,
    participation_payout: participationPayout,
    display_amount: participationAmount || formatMinorAmount(order.amountMinor),
    odds,
    payout: formatMinorAmount(displayPayoutMinor),
    created_at: order.createdAt || rawOrder.created_at || '',
    settled_at: order.settledAt || rawOrder.settled_at,
    position_grid_kind: positionGridKind(order.ruleCode),
  }
}
