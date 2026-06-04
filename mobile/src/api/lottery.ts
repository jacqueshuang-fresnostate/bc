import http from './http'
import { unwrapApiData } from './user'

export type LotteryCard = {
  code: string
  name: string
  category: string
  logoUrl?: string | null
  issue?: string | null
  status?: string
  nextDrawTime?: string | number | null
  saleStopTime?: string | number | null
  drawInterval?: number | null
  drawTimeText?: string
  scheduleText?: string
  latestResult?: string[]
  resultStyle?: string
  resultCount?: number | null
  groupBuyEnabled?: boolean
}

export type HomepageBanner = {
  id?: string | number
  title?: string
  subtitle?: string
  imageUrl?: string
  linkUrl?: string
}

export type HomepageTickerItem = {
  id?: string
  text?: string
}

export type HomepageGroup = {
  code?: string
  name?: string
  lotteries?: LotteryCard[]
}

export type HomepageSettings = {
  bannersEnabled: boolean
  tickerEnabled: boolean
  featuredEnabled: boolean
  groupsEnabled: boolean
  statsEnabled: boolean
}

export type HomepageResponse = {
  serverTime?: string | number | null
  settings?: HomepageSettings
  banners?: HomepageBanner[]
  ticker?: { enabled?: boolean; items?: HomepageTickerItem[] }
  featuredSection?: { enabled?: boolean; title?: string; lotteries?: LotteryCard[] }
  groups?: HomepageGroup[]
  stats?: { todayWinnerCount?: number; totalPayoutDisplay?: string }
}

export type LotteryHistoryItem = {
  id: string
  lottery_code: string
  lottery_name: string
  category?: string
  logo_url?: string | null
  issue: string
  result: string
  result_numbers: string[]
  opened_at?: string | null
  status: string
}

export type LotteryGroupLottery = {
  code: string
  name: string
  category?: string | null
  logo_url?: string | null
  draw_interval?: number | null
  daily_draw_time?: string | null
  group_sort_order?: number | null
  is_recommended?: boolean
}

export type LotteryHistoryGroup = {
  code: string
  name: string
  lotteries?: LotteryGroupLottery[]
}

export type LotteryHistoryPage = {
  items: LotteryHistoryItem[]
  total_count?: number
  page?: number
  page_size?: number
  total_pages?: number
}

export type LotteryLatestHistoryParams = {
  lottery_code?: string
  group_code?: string
}

export type LotteryHistoryParams = LotteryLatestHistoryParams & {
  page?: number
  page_size?: number
}

export async function fetchLotteryHomepage() {
  return unwrapApiData<HomepageResponse>(await http.get('/lottery/home'))
}

export async function fetchLotteryGroups() {
  return unwrapApiData<LotteryHistoryGroup[]>(await http.get('/lottery/groups'))
}

export async function fetchLatestLotteryHistory(params?: LotteryLatestHistoryParams) {
  return unwrapApiData<LotteryHistoryPage>(await http.get('/lottery/history/latest', { params }))
}

export async function fetchLotteryHistory(params?: LotteryHistoryParams) {
  return unwrapApiData<LotteryHistoryPage>(await http.get('/lottery/history', { params }))
}
