import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  fetchCurrentUserProfile,
  fetchRechargeConfig,
  fetchRechargeOrders,
  fetchUserLedgerEntries,
  fetchWithdrawalMethods,
  fetchWithdrawalOrders,
  type LedgerEntry,
  type MobileUserProfile,
  type RechargeConfig,
  type RechargeOrder,
  type WithdrawalMethod,
  type WithdrawalOrder,
} from '../api/user'
import { useAuthStore } from './auth'

const PROFILE_CACHE_MS = 15_000
const RECHARGE_CONFIG_CACHE_MS = 5 * 60_000
const USER_LIST_CACHE_MS = 30_000
const WITHDRAWAL_METHOD_CACHE_MS = 60_000

type LoadOptions = {
  force?: boolean
  silent?: boolean
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
  const ledgerEntries = ref<LedgerEntry[]>([])

  const loadingProfile = ref(false)
  const loadingRechargeConfig = ref(false)
  const loadingRechargeOrders = ref(false)
  const loadingWithdrawalMethods = ref(false)
  const loadingWithdrawalOrders = ref(false)
  const loadingLedgerEntries = ref(false)

  const profileFetchedAt = ref(0)
  const rechargeConfigFetchedAt = ref(0)
  const rechargeOrdersFetchedAt = ref(0)
  const withdrawalMethodsFetchedAt = ref(0)
  const withdrawalOrdersFetchedAt = ref(0)
  const ledgerEntriesFetchedAt = ref(0)
  const userScopeId = ref('')

  let profileRequest: Promise<LoadResult<MobileUserProfile | null>> | null = null
  let rechargeConfigRequest: Promise<LoadResult<RechargeConfig | null>> | null = null
  let rechargeOrdersRequest: Promise<LoadResult<RechargeOrder[]>> | null = null
  let withdrawalMethodsRequest: Promise<LoadResult<WithdrawalMethod[]>> | null = null
  let withdrawalOrdersRequest: Promise<LoadResult<WithdrawalOrder[]>> | null = null
  let ledgerEntriesRequest: Promise<LoadResult<LedgerEntry[]>> | null = null

  function currentUserId() {
    return useAuthStore().user?.id || ''
  }

  function clearUserScopedState() {
    profile.value = null
    rechargeOrders.value = []
    withdrawalMethods.value = []
    withdrawalOrders.value = []
    ledgerEntries.value = []
    profileFetchedAt.value = 0
    rechargeOrdersFetchedAt.value = 0
    withdrawalMethodsFetchedAt.value = 0
    withdrawalOrdersFetchedAt.value = 0
    ledgerEntriesFetchedAt.value = 0
    userScopeId.value = ''
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
    if (!options.force && hasFreshCache(rechargeOrdersFetchedAt.value, USER_LIST_CACHE_MS)) {
      return { data: rechargeOrders.value, refreshed: false }
    }
    if (rechargeOrdersRequest) return rechargeOrdersRequest

    if (!options.silent && !rechargeOrdersFetchedAt.value) loadingRechargeOrders.value = true
    rechargeOrdersRequest = (async () => {
      try {
        rechargeOrders.value = await fetchRechargeOrders()
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
    if (!options.force && hasFreshCache(withdrawalOrdersFetchedAt.value, USER_LIST_CACHE_MS)) {
      return { data: withdrawalOrders.value, refreshed: false }
    }
    if (withdrawalOrdersRequest) return withdrawalOrdersRequest

    if (!options.silent && !withdrawalOrdersFetchedAt.value) loadingWithdrawalOrders.value = true
    withdrawalOrdersRequest = (async () => {
      try {
        withdrawalOrders.value = await fetchWithdrawalOrders()
        withdrawalOrdersFetchedAt.value = Date.now()
        return { data: withdrawalOrders.value, refreshed: true }
      } finally {
        if (!options.silent) loadingWithdrawalOrders.value = false
        withdrawalOrdersRequest = null
      }
    })()
    return withdrawalOrdersRequest
  }

  async function loadLedgerEntries(options: LoadOptions = {}): Promise<LoadResult<LedgerEntry[]>> {
    syncUserScope()
    if (!options.force && hasFreshCache(ledgerEntriesFetchedAt.value, USER_LIST_CACHE_MS)) {
      return { data: ledgerEntries.value, refreshed: false }
    }
    if (ledgerEntriesRequest) return ledgerEntriesRequest

    if (!options.silent && !ledgerEntriesFetchedAt.value) loadingLedgerEntries.value = true
    ledgerEntriesRequest = (async () => {
      try {
        ledgerEntries.value = await fetchUserLedgerEntries()
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
    ledgerEntries,
    loadingProfile,
    loadingRechargeConfig,
    loadingRechargeOrders,
    loadingWithdrawalMethods,
    loadingWithdrawalOrders,
    loadingLedgerEntries,
    profileFetchedAt,
    rechargeConfigFetchedAt,
    rechargeOrdersFetchedAt,
    withdrawalMethodsFetchedAt,
    withdrawalOrdersFetchedAt,
    ledgerEntriesFetchedAt,
    loadProfile,
    loadRechargeConfig,
    loadRechargeOrders,
    loadWithdrawalMethods,
    loadWithdrawalOrders,
    loadLedgerEntries,
    invalidateProfile,
    setProfile,
    clearUserScopedState,
  }
})
