import http from '../../api/http'
import { fetchLotteryGroups as fetchMobileLotteryGroups } from '../../api/lottery'
import { fetchCurrentUserProfile } from '../../api/user'
import type { CreateGroupBuyPayload } from './types'

export function fetchGroupBuyHall(params: Record<string, string>) {
  return http.get('/group-buys', { params })
}

export function fetchGroupBuyDetail(groupBuyId: number) {
  return http.get(`/group-buys/${groupBuyId}`)
}

export function joinGroupBuyPlan(groupBuyId: number, amount: string | number) {
  return http.post(`/group-buys/${groupBuyId}/join`, { amount: String(amount) })
}

export function createGroupBuyPlan(payload: CreateGroupBuyPayload) {
  return http.post('/group-buys', payload)
}

export function fetchMyGroupBuys() {
  return http.get('/group-buys/my')
}

export async function fetchLotteryGroups() {
  return { data: await fetchMobileLotteryGroups() }
}

export function fetchGroupBuyCreateOptions(requestedLotteryCode: string) {
  return http.get('/group-buys/create-options', {
    params: requestedLotteryCode ? { lottery_code: requestedLotteryCode } : undefined,
  })
}

export async function fetchCurrentBalance() {
  return { data: await fetchCurrentUserProfile() }
}
