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

export async function fetchLotteryHomepage() {
  return unwrapApiData<HomepageResponse>(await http.get('/lottery/home'))
}
