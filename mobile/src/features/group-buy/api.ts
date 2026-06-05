import http from '../../api/http'
import { fetchLotteryGroups as fetchMobileLotteryGroups } from '../../api/lottery'
import { fetchCurrentUserProfile, unwrapApiData } from '../../api/user'
import type { CreateGroupBuyPayload, GroupBuyPlan } from './types'

export async function fetchGroupBuyHall(params: Record<string, string>) {
  const data = unwrapApiData<any>(await http.get('/user/group-buy/plans', {
    params: normalizeGroupBuyQuery(params),
  }))
  return { data: { items: normalizeGroupBuyPlanItems(data?.items) } }
}

export async function fetchGroupBuyDetail(groupBuyId: string) {
  const data = unwrapApiData<any>(await http.get(`/user/group-buy/plans/${groupBuyId}`))
  return { data: normalizeGroupBuyPlan(data) }
}

export async function joinGroupBuyPlan(groupBuyId: string, amount: string | number) {
  const data = unwrapApiData<any>(await http.post(`/user/group-buy/plans/${groupBuyId}/participants`, {
    amountMinor: moneyToMinor(amount),
  }))
  return {
    data: {
      plan: normalizeGroupBuyPlan(data?.plan),
      balance: minorToMoney(data?.availableBalanceMinor),
    },
  }
}

export async function createGroupBuyPlan(payload: CreateGroupBuyPayload) {
  const data = unwrapApiData<any>(await http.post('/user/group-buy/plans', {
    lotteryId: payload.lottery_code,
    issue: payload.issue,
    ruleCode: payload.play_code,
    title: payload.title,
    numbers: payload.numbers,
    totalAmountMinor: moneyToMinor(payload.total_amount),
    selfAmountMinor: Math.max(0, Number(payload.self_shares || 0)) * moneyToMinor(payload.share_amount),
  }))
  return { data: normalizeGroupBuyPlan(data?.plan) }
}

export async function fetchMyGroupBuys() {
  const data = unwrapApiData<any>(await http.get('/user/group-buy/my'))
  return { data: { items: normalizeGroupBuyPlanItems(data?.items) } }
}

export async function fetchLotteryGroups() {
  return { data: await fetchMobileLotteryGroups() }
}

export async function fetchGroupBuyCreateOptions(requestedLotteryCode: string) {
  const data = unwrapApiData<any>(await http.get('/user/group-buy/create-options', {
    params: requestedLotteryCode ? { lotteryId: requestedLotteryCode } : undefined,
  }))
  return {
    data: {
      lotteries: data?.lotteries || [],
      issues: data?.issues || [],
      plays: data?.plays || [],
      min_share_amount: minorToMoney(data?.settings?.minShareAmountMinor),
      initiator_min_buy_ratio: String(data?.settings?.initiatorMinPercent ?? 0),
      share_amount: minorToMoney(data?.settings?.minShareAmountMinor),
      settings: {
        min_share_amount: minorToMoney(data?.settings?.minShareAmountMinor),
        initiator_min_buy_ratio: String(data?.settings?.initiatorMinPercent ?? 0),
        share_amount: minorToMoney(data?.settings?.minShareAmountMinor),
        participant_min_amount: minorToMoney(data?.settings?.participantMinAmountMinor),
      },
    },
  }
}

function normalizeGroupBuyQuery(params: Record<string, string>) {
  return {
    ...(params.lottery_code ? { lotteryId: params.lottery_code } : {}),
    ...(params.group_code ? { groupCode: params.group_code } : {}),
  }
}

function normalizeGroupBuyPlanItems(items: any): GroupBuyPlan[] {
  return Array.isArray(items) ? items.map(normalizeGroupBuyPlan).filter(Boolean) : []
}

function normalizeGroupBuyPlan(item: any): GroupBuyPlan {
  const totalAmountMinor = Number(item?.totalAmountMinor || 0)
  const shareAmountMinor = Number(item?.shareAmountMinor || 0)
  const soldShares = Number(item?.soldShares || 0)
  const availableShares = Number(item?.availableShares || 0)
  const shareCount = Number(item?.shareCount || 0)
  const myParticipation = item?.myParticipation

  return {
    id: String(item?.id || ''),
    order_id: item?.orderId ? String(item.orderId) : null,
    lottery_code: String(item?.lotteryId || ''),
    lottery_name: String(item?.lotteryName || ''),
    category: item?.category ? String(item.category) : undefined,
    issue: String(item?.issue || ''),
    play_code: String(item?.ruleCode || ''),
    play_name: String(item?.playName || ''),
    title: String(item?.title || ''),
    numbers: String(item?.numbers || ''),
    total_amount: minorToMoney(totalAmountMinor),
    share_count: shareCount,
    share_amount: minorToMoney(shareAmountMinor),
    reserved_shares: 0,
    sold_shares: soldShares,
    available_shares: availableShares,
    progress_percent: Number(item?.progressPercent || 0),
    status: String(item?.status || ''),
    created_at: item?.createdAt ? String(item.createdAt) : undefined,
    updated_at: item?.updatedAt ? String(item.updatedAt) : undefined,
    participant_count: Number(item?.participantCount || 0),
    initiator_display: String(item?.initiatorDisplay || ''),
    my_participation: myParticipation
      ? {
          shares: Number(myParticipation.shareCount || 0),
          paid_shares: Number(myParticipation.shareCount || 0),
          reserved_shares: 0,
          amount: minorToMoney(myParticipation.amountMinor),
        }
      : null,
  }
}

function moneyToMinor(value: string | number) {
  const text = String(value ?? '').trim()
  if (!/^\d+(?:\.\d{0,2})?$/.test(text)) return 0
  const [whole, fraction = ''] = text.split('.')
  return Number(whole || 0) * 100 + Number(fraction.padEnd(2, '0').slice(0, 2) || 0)
}

function minorToMoney(value: string | number | null | undefined) {
  const amount = Number(value || 0)
  if (!Number.isFinite(amount)) return '0.00'
  return (amount / 100).toFixed(2)
}

export async function fetchCurrentBalance() {
  return { data: await fetchCurrentUserProfile() }
}
