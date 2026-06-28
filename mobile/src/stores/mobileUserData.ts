import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  fetchCurrentUserProfile,
  fetchRechargeConfig,
  fetchRechargeOrders,
  fetchUserLedgerEntries,
  fetchWithdrawalMethods,
  fetchWithdrawalOrders,
  fetchWithdrawalTurnoverProgress,
  type LedgerEntry,
  type MobileUserProfile,
  type RechargeConfig,
  type RechargeOrder,
  type WithdrawalMethod,
  type WithdrawalOrder,
  type WithdrawalTurnoverProgress,
} from '../api/user'
import { useAuthStore } from './auth'

const PROFILE_CACHE_MS = 15_000
const RECHARGE_CONFIG_CACHE_MS = 5 * 60_000
const USER_LIST_CACHE_MS = 30_000
const WITHDRAWAL_METHOD_CACHE_MS = 60_000
const USER_RECORD_PAGE_SIZE = 20

type LoadOptions = {
  force?: boolean
  silent?: boolean
  append?: boolean
}

type LoadResult<T> = {
  data: T
  refreshed: boolean
}

function hasFreshCache(fetchedAt: number, ttlMs: number) {
  return fetchedAt > 0 && Date.now() - fetchedAt < ttlMs
}

// 手机端用户侧服务端状态缓存：页面切换优先复用 Pinia，需要刷新时再强制请求后端。
export const useMobileUserDataStore = defineStore('mobileUserData', () => {
  const profile = ref<MobileUserProfile | null>(null)
  const rechargeConfig = ref<RechargeConfig | null>(null)
  const rechargeOrders = ref<RechargeOrder[]>([])
  const withdrawalMethods = ref<WithdrawalMethod[]>([])
  const withdrawalOrders = ref<WithdrawalOrder[]>([])
  const withdrawalTurnoverProgress = ref<WithdrawalTurnoverProgress | null>(null)
  const ledgerEntries = ref<LedgerEntry[]>([])

  const loadingProfile = ref(false)
  const loadingRechargeConfig = ref(false)
  const loadingRechargeOrders = ref(false)
  const loadingWithdrawalMethods = ref(false)
  const loadingWithdrawalOrders = ref(false)
  const loadingWithdrawalTurnoverProgress = ref(false)
  const loadingLedgerEntries = ref(false)

  const profileFetchedAt = ref(0)
  const rechargeConfigFetchedAt = ref(0)
  const rechargeOrdersFetchedAt = ref(0)
  const withdrawalMethodsFetchedAt = ref(0)
  const withdrawalOrdersFetchedAt = ref(0)
  const withdrawalTurnoverProgressFetchedAt = ref(0)
  const ledgerEntriesFetchedAt = ref(0)
  const userScopeId = ref('')
  const rechargeOrdersPage = ref(0)
  const withdrawalOrdersPage = ref(0)
  const ledgerEntriesPage = ref(0)
  const rechargeOrdersHasMore = ref(true)
  const withdrawalOrdersHasMore = ref(true)
  const ledgerEntriesHasMore = ref(true)

  let profileRequest: Promise<LoadResult<MobileUserProfile | null>> | null = null
  let rechargeConfigRequest: Promise<LoadResult<RechargeConfig | null>> | null = null
  let rechargeOrdersRequest: Promise<LoadResult<RechargeOrder[]>> | null = null
  let withdrawalMethodsRequest: Promise<LoadResult<WithdrawalMethod[]>> | null = null
  let withdrawalOrdersRequest: Promise<LoadResult<WithdrawalOrder[]>> | null = null
  let withdrawalTurnoverProgressRequest: Promise<LoadResult<WithdrawalTurnoverProgress | null>> | null = null
  let ledgerEntriesRequest: Promise<LoadResult<LedgerEntry[]>> | null = null

  function currentUserId() {
    return useAuthStore().user?.id || ''
  }

  function clearUserScopedState() {
    profile.value = null
    rechargeOrders.value = []
    withdrawalMethods.value = []
    withdrawalOrders.value = []
    withdrawalTurnoverProgress.value = null
    ledgerEntries.value = []
    profileFetchedAt.value = 0
    rechargeOrdersFetchedAt.value = 0
    withdrawalMethodsFetchedAt.value = 0
    withdrawalOrdersFetchedAt.value = 0
    withdrawalTurnoverProgressFetchedAt.value = 0
    ledgerEntriesFetchedAt.value = 0
    rechargeOrdersPage.value = 0
    withdrawalOrdersPage.value = 0
    ledgerEntriesPage.value = 0
    rechargeOrdersHasMore.value = true
    withdrawalOrdersHasMore.value = true
    ledgerEntriesHasMore.value = true
    userScopeId.value = ''
  }

  function mergeRecordsById<T extends { id: string }>(current: T[], incoming: T[]) {
    const seen = new Set<string>()
    return [...current, ...incoming].filter(item => {
      if (!item.id || seen.has(item.id)) return false
      seen.add(item.id)
      return true
    })
  }

  function syncUserScope() {
    const nextUserId = currentUserId()
    if (userScopeId.value && userScopeId.value !== nextUserId) {
      clearUserScopedState()
    }
    if (!userScopeId.value && nextUserId) {
      userScopeId.value = nextUserId
    }
  }

  async function loadProfile(options: LoadOptions = {}): Promise<LoadResult<MobileUserProfile | null>> {
    syncUserScope()
    if (!options.force && profile.value && hasFreshCache(profileFetchedAt.value, PROFILE_CACHE_MS)) {
      return { data: profile.value, refreshed: false }
    }
    if (profileRequest) return profileRequest

    if (!options.silent && !profile.value) loadingProfile.value = true
    profileRequest = (async () => {
      try {
        const data = await fetchCurrentUserProfile()
        profile.value = data
        profileFetchedAt.value = Date.now()
        userScopeId.value = data.id || userScopeId.value
        return { data: profile.value, refreshed: true }
      } finally {
        if (!options.silent) loadingProfile.value = false
        profileRequest = null
      }
    })()
    return profileRequest
  }

  async function loadRechargeConfig(options: LoadOptions = {}): Promise<LoadResult<RechargeConfig | null>> {
    if (!options.force && rechargeConfig.value && hasFreshCache(rechargeConfigFetchedAt.value, RECHARGE_CONFIG_CACHE_MS)) {
      return { data: rechargeConfig.value, refreshed: false }
    }
    if (rechargeConfigRequest) return rechargeConfigRequest

    if (!options.silent && !rechargeConfig.value) loadingRechargeConfig.value = true
    rechargeConfigRequest = (async () => {
      try {
        rechargeConfig.value = await fetchRechargeConfig()
        rechargeConfigFetchedAt.value = Date.now()
        return { data: rechargeConfig.value, refreshed: true }
      } finally {
        if (!options.silent) loadingRechargeConfig.value = false
        rechargeConfigRequest = null
      }
    })()
    return rechargeConfigRequest
  }

  async function loadRechargeOrders(options: LoadOptions = {}): Promise<LoadResult<RechargeOrder[]>> {
    syncUserScope()
    const append = Boolean(options.append)
    if (append && !rechargeOrdersHasMore.value) {
      return { data: rechargeOrders.value, refreshed: false }
    }
    if (!append && !options.force && hasFreshCache(rechargeOrdersFetchedAt.value, USER_LIST_CACHE_MS)) {
      return { data: rechargeOrders.value, refreshed: false }
    }
    if (rechargeOrdersRequest) return rechargeOrdersRequest

    const nextPage = append ? rechargeOrdersPage.value + 1 : 1
    if (!options.silent && (!rechargeOrdersFetchedAt.value || append)) loadingRechargeOrders.value = true
    rechargeOrdersRequest = (async () => {
      try {
        const items = await fetchRechargeOrders({ page: nextPage, pageSize: USER_RECORD_PAGE_SIZE })
        rechargeOrders.value = append ? mergeRecordsById(rechargeOrders.value, items) : items
        rechargeOrdersPage.value = items.length > 0 ? nextPage : (append ? rechargeOrdersPage.value : 0)
        rechargeOrdersHasMore.value = items.length >= USER_RECORD_PAGE_SIZE
        rechargeOrdersFetchedAt.value = Date.now()
        return { data: rechargeOrders.value, refreshed: true }
      } finally {
        if (!options.silent) loadingRechargeOrders.value = false
        rechargeOrdersRequest = null
      }
    })()
    return rechargeOrdersRequest
  }

  async function loadWithdrawalMethods(options: LoadOptions = {}): Promise<LoadResult<WithdrawalMethod[]>> {
    syncUserScope()
    if (!options.force && hasFreshCache(withdrawalMethodsFetchedAt.value, WITHDRAWAL_METHOD_CACHE_MS)) {
      return { data: withdrawalMethods.value, refreshed: false }
    }
    if (withdrawalMethodsRequest) return withdrawalMethodsRequest

    if (!options.silent && !withdrawalMethodsFetchedAt.value) loadingWithdrawalMethods.value = true
    withdrawalMethodsRequest = (async () => {
      try {
        withdrawalMethods.value = await fetchWithdrawalMethods()
        withdrawalMethodsFetchedAt.value = Date.now()
        return { data: withdrawalMethods.value, refreshed: true }
      } finally {
        if (!options.silent) loadingWithdrawalMethods.value = false
        withdrawalMethodsRequest = null
      }
    })()
    return withdrawalMethodsRequest
  }

  async function loadWithdrawalOrders(options: LoadOptions = {}): Promise<LoadResult<WithdrawalOrder[]>> {
    syncUserScope()
    const append = Boolean(options.append)
    if (append && !withdrawalOrdersHasMore.value) {
      return { data: withdrawalOrders.value, refreshed: false }
    }
    if (!append && !options.force && hasFreshCache(withdrawalOrdersFetchedAt.value, USER_LIST_CACHE_MS)) {
      return { data: withdrawalOrders.value, refreshed: false }
    }
    if (withdrawalOrdersRequest) return withdrawalOrdersRequest

    const nextPage = append ? withdrawalOrdersPage.value + 1 : 1
    if (!options.silent && (!withdrawalOrdersFetchedAt.value || append)) loadingWithdrawalOrders.value = true
    withdrawalOrdersRequest = (async () => {
      try {
        const items = await fetchWithdrawalOrders({ page: nextPage, pageSize: USER_RECORD_PAGE_SIZE })
        withdrawalOrders.value = append ? mergeRecordsById(withdrawalOrders.value, items) : items
        withdrawalOrdersPage.value = items.length > 0 ? nextPage : (append ? withdrawalOrdersPage.value : 0)
        withdrawalOrdersHasMore.value = items.length >= USER_RECORD_PAGE_SIZE
        withdrawalOrdersFetchedAt.value = Date.now()
        return { data: withdrawalOrders.value, refreshed: true }
      } finally {
        if (!options.silent) loadingWithdrawalOrders.value = false
        withdrawalOrdersRequest = null
      }
    })()
    return withdrawalOrdersRequest
  }

  async function loadWithdrawalTurnoverProgress(
    options: LoadOptions = {},
  ): Promise<LoadResult<WithdrawalTurnoverProgress | null>> {
    syncUserScope()
    if (
      !options.force
      && withdrawalTurnoverProgress.value
      && hasFreshCache(withdrawalTurnoverProgressFetchedAt.value, USER_LIST_CACHE_MS)
    ) {
      return { data: withdrawalTurnoverProgress.value, refreshed: false }
    }
    if (withdrawalTurnoverProgressRequest) return withdrawalTurnoverProgressRequest

    if (!options.silent && !withdrawalTurnoverProgress.value) {
      loadingWithdrawalTurnoverProgress.value = true
    }
    withdrawalTurnoverProgressRequest = (async () => {
      try {
        withdrawalTurnoverProgress.value = await fetchWithdrawalTurnoverProgress()
        withdrawalTurnoverProgressFetchedAt.value = Date.now()
        return { data: withdrawalTurnoverProgress.value, refreshed: true }
      } finally {
        if (!options.silent) loadingWithdrawalTurnoverProgress.value = false
        withdrawalTurnoverProgressRequest = null
      }
    })()
    return withdrawalTurnoverProgressRequest
  }

  async function loadLedgerEntries(options: LoadOptions = {}): Promise<LoadResult<LedgerEntry[]>> {
    syncUserScope()
    const append = Boolean(options.append)
    if (append && !ledgerEntriesHasMore.value) {
      return { data: ledgerEntries.value, refreshed: false }
    }
    if (!append && !options.force && hasFreshCache(ledgerEntriesFetchedAt.value, USER_LIST_CACHE_MS)) {
      return { data: ledgerEntries.value, refreshed: false }
    }
    if (ledgerEntriesRequest) return ledgerEntriesRequest

    const nextPage = append ? ledgerEntriesPage.value + 1 : 1
    if (!options.silent && (!ledgerEntriesFetchedAt.value || append)) loadingLedgerEntries.value = true
    ledgerEntriesRequest = (async () => {
      try {
        const items = await fetchUserLedgerEntries({ page: nextPage, pageSize: USER_RECORD_PAGE_SIZE })
        ledgerEntries.value = append ? mergeRecordsById(ledgerEntries.value, items) : items
        ledgerEntriesPage.value = items.length > 0 ? nextPage : (append ? ledgerEntriesPage.value : 0)
        ledgerEntriesHasMore.value = items.length >= USER_RECORD_PAGE_SIZE
        ledgerEntriesFetchedAt.value = Date.now()
        return { data: ledgerEntries.value, refreshed: true }
      } finally {
        if (!options.silent) loadingLedgerEntries.value = false
        ledgerEntriesRequest = null
      }
    })()
    return ledgerEntriesRequest
  }

  function invalidateProfile() {
    profileFetchedAt.value = 0
  }

  function setProfile(nextProfile: MobileUserProfile | null) {
    if (!nextProfile) {
      clearUserScopedState()
      return
    }
    profile.value = nextProfile
    profileFetchedAt.value = Date.now()
    userScopeId.value = nextProfile.id || userScopeId.value
  }

  return {
    profile,
    rechargeConfig,
    rechargeOrders,
    withdrawalMethods,
    withdrawalOrders,
    withdrawalTurnoverProgress,
    ledgerEntries,
    loadingProfile,
    loadingRechargeConfig,
    loadingRechargeOrders,
    loadingWithdrawalMethods,
    loadingWithdrawalOrders,
    loadingWithdrawalTurnoverProgress,
    loadingLedgerEntries,
    profileFetchedAt,
    rechargeConfigFetchedAt,
    rechargeOrdersFetchedAt,
    withdrawalMethodsFetchedAt,
    withdrawalOrdersFetchedAt,
    withdrawalTurnoverProgressFetchedAt,
    ledgerEntriesFetchedAt,
    rechargeOrdersPage,
    withdrawalOrdersPage,
    ledgerEntriesPage,
    rechargeOrdersHasMore,
    withdrawalOrdersHasMore,
    ledgerEntriesHasMore,
    loadProfile,
    loadRechargeConfig,
    loadRechargeOrders,
    loadWithdrawalMethods,
    loadWithdrawalOrders,
    loadWithdrawalTurnoverProgress,
    loadLedgerEntries,
    invalidateProfile,
    setProfile,
    clearUserScopedState,
  }
})
