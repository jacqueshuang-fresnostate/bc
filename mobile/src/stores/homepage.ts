import { defineStore } from 'pinia'
import { ref } from 'vue'
import { fetchLotteryHomepage } from '../api/lottery'
import type { HomepageResponse } from '../api/lottery'
import { fetchMobileAdvertisements } from '../api/user'
import type { MobileAdvertisement } from '../api/user'

const HOMEPAGE_CACHE_MS = 30_000
const ADVERTISEMENT_CACHE_MS = 5 * 60_000

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
  const homepage = ref<HomepageResponse | null>(null)
  const mobileAdvertisements = ref<MobileAdvertisement[]>([])
  const loadingHomepage = ref(false)
  const homepageFetchedAt = ref(0)
  const advertisementsFetchedAt = ref(0)

  let homepageRequest: Promise<LoadResult<HomepageResponse | null>> | null = null
  let advertisementsRequest: Promise<LoadResult<MobileAdvertisement[]>> | null = null

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

  function invalidateHomepage() {
    homepageFetchedAt.value = 0
  }

  return {
    homepage,
    mobileAdvertisements,
    loadingHomepage,
    homepageFetchedAt,
    advertisementsFetchedAt,
    loadHomepage,
    loadMobileAdvertisements,
    invalidateHomepage,
  }
})
