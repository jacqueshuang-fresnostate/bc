import http from './http'
import { unwrapApiData } from './user'

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

export async function fetchUserBetOrders() {
  const orders = unwrapApiData<UserBetOrderDetail[]>(await http.get('/user/bet/orders'))
  return orders.map(normalizeUserBetOrder)
}

function formatMinorAmount(value: number) {
  return (Number(value || 0) / 100).toFixed(2)
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
  const numbers = selectionNumbers(order)
  return {
    ...order,
    lottery_code: order.lotteryId,
    lottery_name: order.lotteryName,
    play_code: order.ruleCode,
    play_name: playNameMap[order.ruleCode] || order.ruleCode,
    rule_code: order.ruleCode,
    numbers,
    canonical_numbers: numbers,
    result_numbers: splitDrawNumber(order.drawNumber),
    draw_numbers: splitDrawNumber(order.drawNumber),
    matched_bets: order.matchedBets || [],
    expanded_bets: order.expandedBets || [],
    status: statusMap[order.status] || order.status,
    bet_count: order.stakeCount,
    unit_amount: formatMinorAmount(order.unitAmountMinor),
    multiple: 1,
    amount: formatMinorAmount(order.amountMinor),
    odds: formatMinorAmount(order.oddsBasisPoints / 100),
    payout: formatMinorAmount(order.payoutMinor),
    created_at: order.createdAt,
    settled_at: order.settledAt,
    position_grid_kind: positionGridKind(order.ruleCode),
  }
}
