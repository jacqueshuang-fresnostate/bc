import { computed, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { showToast } from 'vant'
import { errorMessage } from '../../../api/user'
import { useMobileUserDataStore } from '../../../stores/mobileUserData'
import { sortByCreatedTimeDesc } from '../../../utils/timeSort'
import { createGroupBuyPlan, fetchGroupBuyCreateOptions, fetchMyGroupBuys } from '../api'
import { buildCreateGroupBuyPayload, calculateCreatePaymentAmount, calculateFixedShareCount, calculateRequiredSelfShares, createDefaultGroupBuyForm, normalizeItems, normalizeOptionPayload } from '../presentation'
import type { GroupBuySettings, SelectOption } from '../types'

const MY_GROUP_BUY_PAGE_SIZE = 12

/** 创建发起合买弹窗的模板方法状态流。 */
export function useGroupBuyCreate(lotteryCode: { value: string }, options: { loadHall: () => Promise<void>; activeTab: { value: string }; initialVisible?: boolean }) {
  const userDataStore = useMobileUserDataStore()
  const { profile } = storeToRefs(userDataStore)
  const createVisible = ref(Boolean(options.initialVisible))
  const submittingCreate = ref(false)
  const loadingMy = ref(false)
  const myGroupBuys = ref<any[]>([])
  const myGroupBuysPage = ref(0)
  const myGroupBuysHasMore = ref(true)
  const createLotteryOptions = ref<SelectOption[]>([])
  const createIssueOptions = ref<SelectOption[]>([])
  const createPlayOptions = ref<SelectOption[]>([])
  const createOptionsRequestSeq = ref(0)
  const createSettings = ref<GroupBuySettings>({ min_share_amount: '0.01', initiator_min_buy_ratio: '0.00', share_amount: '1.00', participant_min_amount: '1.00' })
  const createExtras = ref({ commission_rate: '0', visibility: '公开可见' })
  const createForm = ref(createDefaultGroupBuyForm(lotteryCode.value))

  const minShareAmount = computed(() => Number(createSettings.value.min_share_amount || 0.01))
  const fixedShareAmount = computed(() => String(createSettings.value.share_amount || '1.00'))
  const computedShareCount = computed(() => calculateFixedShareCount(createForm.value.total_amount, fixedShareAmount.value))
  const computedShareAmount = fixedShareAmount
  const balance = computed(() => profile.value?.balance || '0.00')
  const initiatorMinBuyRatio = computed(() => Number(createSettings.value.initiator_min_buy_ratio || 0))
  const requiredSelfShares = computed(() => calculateRequiredSelfShares(createForm.value.total_amount, fixedShareAmount.value, initiatorMinBuyRatio.value))
  const createPaymentAmount = computed(() => calculateCreatePaymentAmount(fixedShareAmount.value, createForm.value.self_shares))

  /** 加载当前用户余额。 */
  async function loadBalance(options: { force?: boolean; silent?: boolean } = {}) {
    try {
      await userDataStore.loadProfile(options)
    } catch {}
  }

  /** 选择发起合买彩种并重新加载期号和玩法。 */
  async function selectCreateLottery(code: string) {
    createForm.value.lottery_code = code
    await loadCreateOptions()
  }

  /** 打开发起合买弹窗。 */
  function startCreatePlan() {
    createVisible.value = true
  }

  /** 关闭发起合买弹窗。 */
  function closeCreatePlan() {
    createVisible.value = false
  }

  /** 加载发起合买所需的彩种、期号和玩法选项。 */
  async function loadCreateOptions() {
    const requestId = ++createOptionsRequestSeq.value
    const requestedLotteryCode = createForm.value.lottery_code
    try {
      const res = await fetchGroupBuyCreateOptions(requestedLotteryCode)
      if (requestId !== createOptionsRequestSeq.value || createForm.value.lottery_code !== requestedLotteryCode) return
      createLotteryOptions.value = normalizeOptionPayload(res.data, ['lottery_options', 'lotteries'])
      createIssueOptions.value = normalizeOptionPayload(res.data, ['issue_options', 'issues'])
      createPlayOptions.value = normalizeOptionPayload(res.data, ['play_options', 'plays'])
      createSettings.value = {
        min_share_amount: String(res.data?.settings?.min_share_amount || res.data?.min_share_amount || '0.01'),
        initiator_min_buy_ratio: String(res.data?.settings?.initiator_min_buy_ratio || res.data?.initiator_min_buy_ratio || '0.00'),
        share_amount: String(res.data?.settings?.share_amount || res.data?.share_amount || '1.00'),
        participant_min_amount: String(res.data?.settings?.participant_min_amount || res.data?.participant_min_amount || res.data?.settings?.share_amount || '1.00'),
      }
      const firstLottery = createLotteryOptions.value[0]?.value || ''
      if (!createForm.value.lottery_code || !createLotteryOptions.value.some(option => option.value === createForm.value.lottery_code)) {
        createForm.value.lottery_code = firstLottery
        if (firstLottery) {
          await loadCreateOptions()
          return
        }
      }
      if (!createIssueOptions.value.some(option => option.value === createForm.value.issue)) {
        createForm.value.issue = createIssueOptions.value[0]?.value || ''
      }
      if (!createPlayOptions.value.some(option => option.value === createForm.value.play_code)) {
        createForm.value.play_code = createPlayOptions.value[0]?.value || ''
      }
    } catch (e: any) {
      if (requestId !== createOptionsRequestSeq.value || createForm.value.lottery_code !== requestedLotteryCode) return
      createLotteryOptions.value = []
      createIssueOptions.value = []
      createPlayOptions.value = []
      createSettings.value = { min_share_amount: '0.01', initiator_min_buy_ratio: '0.00', share_amount: '1.00', participant_min_amount: '1.00' }
      showToast(errorMessage(e, '加载发起选项失败'))
    }
  }

  /** 提交发起合买计划。 */
  async function createGroupBuy() {
    submittingCreate.value = true
    try {
      const payload = buildCreateGroupBuyPayload(createForm.value, lotteryCode.value, fixedShareAmount.value, computedShareCount.value)
      if (computedShareCount.value <= 0) {
        showToast('总金额必须能按每份金额整除')
        return null
      }
      if (Number(fixedShareAmount.value) < minShareAmount.value) {
        showToast(`每份金额不能低于 ${createSettings.value.min_share_amount}`)
        return null
      }
      if (requiredSelfShares.value > 0 && Number(createForm.value.self_shares || 0) < requiredSelfShares.value) {
        showToast(`发起人最低自购 ${requiredSelfShares.value} 份`)
        return null
      }
      const res = await createGroupBuyPlan(payload)
      const createdPlan = res.data || null
      showToast('发起成功')
      createVisible.value = false
      options.activeTab.value = 'hall'
      if (createdPlan) {
        myGroupBuys.value = sortByCreatedTimeDesc(mergePlans([createdPlan], myGroupBuys.value))
        myGroupBuysPage.value = Math.max(myGroupBuysPage.value, 1)
      }
      await loadBalance({ force: true, silent: true })
      await Promise.all([options.loadHall(), loadMyGroupBuys()])
      return createdPlan
    } catch (e: any) {
      showToast(errorMessage(e, '发起合买失败'))
      return null
    } finally {
      submittingCreate.value = false
    }
  }

  /** 加载我的合买列表。 */
  async function loadMyGroupBuys(options: { append?: boolean } = {}) {
    const append = Boolean(options.append)
    if (append && !myGroupBuysHasMore.value) return
    loadingMy.value = true
    try {
      const nextPage = append ? myGroupBuysPage.value + 1 : 1
      const res = await fetchMyGroupBuys({ page: nextPage, pageSize: MY_GROUP_BUY_PAGE_SIZE })
      const items = normalizeItems(res.data)
      myGroupBuys.value = sortByCreatedTimeDesc(append ? mergePlans(myGroupBuys.value, items) : items)
      myGroupBuysPage.value = items.length > 0 ? nextPage : (append ? myGroupBuysPage.value : 0)
      myGroupBuysHasMore.value = items.length >= MY_GROUP_BUY_PAGE_SIZE
    } catch (e: any) {
      if (!append) {
        myGroupBuys.value = []
        myGroupBuysPage.value = 0
        myGroupBuysHasMore.value = true
      }
      showToast(errorMessage(e, '加载我的合买失败'))
    } finally {
      loadingMy.value = false
    }
  }

  /** 按计划编号追加去重，避免翻页和实时刷新造成重复记录。 */
  function mergePlans(current: any[], incoming: any[]) {
    const seen = new Set<string>()
    return [...current, ...incoming].filter(item => {
      const id = String(item?.id || '')
      if (!id || seen.has(id)) return false
      seen.add(id)
      return true
    })
  }

  return {
    createVisible,
    submittingCreate,
    loadingMy,
    balance,
    myGroupBuys,
    myGroupBuysPage,
    myGroupBuysHasMore,
    createLotteryOptions,
    createIssueOptions,
    createPlayOptions,
    createOptionsRequestSeq,
    createSettings,
    minShareAmount,
    createExtras,
    createForm,
    computedShareAmount,
    fixedShareAmount,
    computedShareCount,
    initiatorMinBuyRatio,
    requiredSelfShares,
    createPaymentAmount,
    loadBalance,
    selectCreateLottery,
    startCreatePlan,
    closeCreatePlan,
    loadCreateOptions,
    createGroupBuy,
    loadMyGroupBuys,
  }
}
