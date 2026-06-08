import { defineStore } from 'pinia'
import { ref } from 'vue'
import { fetchLotteryHomepage } from '../api/lottery'
import type { HomepageResponse } from '../api/lottery'
import { fetchCurrentUserProfile, fetchMobileAdvertisements } from '../api/user'
import type { MobileAdvertisement } from '../api/user'
import { useAuthStore } from './auth'

const HOMEPAGE_CACHE_MS = 30_000
const ADVERTISEMENT_CACHE_MS = 5 * 60_000
const BALANCE_CACHE_MS = 15_000

type LoadOptions = {
  force?: boolean
  silent?: boolean
}

type LoadResult<T> = {
  data: T
  refreshed: boolean
}

// 首页服务端状态缓存：避免底部导航反复进入首页时重新请求同一批数据。
export const useHomepageStore = defineStore('homepage', () => {
  const balance = ref('0.00')
  const homepage = ref<HomepageResponse | null>(null)
  const mobileAdvertisements = ref<MobileAdvertisement[]>([])
  const loadingHomepage = ref(false)
  const homepageFetchedAt = ref(0)
  const advertisementsFetchedAt = ref(0)
  const balanceFetchedAt = ref(0)
  const balanceUserId = ref('')

  let homepageRequest: Promise<LoadResult<HomepageResponse | null>> | null = null
  let advertisementsRequest: Promise<LoadResult<MobileAdvertisement[]>> | null = null
  let balanceRequest: Promise<LoadResult<string>> | null = null

  function hasFreshCache(fetchedAt: number, ttlMs: number) {
    return fetchedAt > 0 && Date.now() - fetchedAt < ttlMs
  }

  async function loadHomepage(options: LoadOptions = {}): Promise<LoadResult<HomepageResponse | null>> {
    if (!options.force && homepage.value && hasFreshCache(homepageFetchedAt.value, HOMEPAGE_CACHE_MS)) {
      return { data: homepage.value, refreshed: false }
    }
    if (homepageRequest) return homepageRequest

    if (!options.silent && !homepage.value) loadingHomepage.value = true
    homepageRequest = (async () => {
      try {
        const data = await fetchLotteryHomepage()
        homepage.value = data || null
        homepageFetchedAt.value = Date.now()
        return { data: homepage.value, refreshed: true }
      } catch {
        if (!options.silent && !homepage.value) homepage.value = null
        return { data: homepage.value, refreshed: false }
      } finally {
        if (!options.silent) loadingHomepage.value = false
        homepageRequest = null
      }
    })()
    return homepageRequest
  }

  async function loadMobileAdvertisements(options: LoadOptions = {}): Promise<LoadResult<MobileAdvertisement[]>> {
    if (!options.force && mobileAdvertisements.value.length && hasFreshCache(advertisementsFetchedAt.value, ADVERTISEMENT_CACHE_MS)) {
      return { data: mobileAdvertisements.value, refreshed: false }
    }
    if (advertisementsRequest) return advertisementsRequest

    advertisementsRequest = (async () => {
      try {
        mobileAdvertisements.value = await fetchMobileAdvertisements()
        advertisementsFetchedAt.value = Date.now()
        return { data: mobileAdvertisements.value, refreshed: true }
      } catch {
        if (!mobileAdvertisements.value.length) mobileAdvertisements.value = []
        return { data: mobileAdvertisements.value, refreshed: false }
      } finally {
        advertisementsRequest = null
      }
    })()
    return advertisementsRequest
  }

  async function loadBalance(options: LoadOptions = {}): Promise<LoadResult<string>> {
    const auth = useAuthStore()
    const currentUserId = auth.user?.id || ''
    if (balanceUserId.value && balanceUserId.value !== currentUserId) {
      balanceFetchedAt.value = 0
      balance.value = '0.00'
    }
    if (!options.force && hasFreshCache(balanceFetchedAt.value, BALANCE_CACHE_MS)) {
      return { data: balance.value, refreshed: false }
    }
    if (balanceRequest) return balanceRequest

    balanceRequest = (async () => {
      try {
        const profile = await fetchCurrentUserProfile()
        balance.value = profile.balance
        balanceUserId.value = profile.id || currentUserId
        balanceFetchedAt.value = Date.now()
        return { data: balance.value, refreshed: true }
      } catch {
        return { data: balance.value, refreshed: false }
      } finally {
        balanceRequest = null
      }
    })()
    return balanceRequest
  }

  function invalidateHomepage() {
    homepageFetchedAt.value = 0
  }

  return {
    balance,
    homepage,
    mobileAdvertisements,
    loadingHomepage,
    homepageFetchedAt,
    advertisementsFetchedAt,
    balanceFetchedAt,
    balanceUserId,
    loadHomepage,
    loadMobileAdvertisements,
    loadBalance,
    invalidateHomepage,
  }
})
