import { computed, ref } from 'vue'
import { showToast } from 'vant'
import { fetchGroupBuyDetail, joinGroupBuyPlan } from '../api'
import type { GroupBuyPlan } from '../types'

/** 创建合买详情和认购的模板方法状态流。 */
export function useGroupBuyDetail(options: { loadBalance: () => Promise<void>; loadHall: () => Promise<void>; loadMyGroupBuys: () => Promise<void>; activeTab: { value: string } }) {
  const selectedGroupBuy = ref<any | null>(null)
  const loadingDetail = ref(false)
  const submittingJoin = ref(false)
  const joinAmountInput = ref('1.00')
  const detailRequestSeq = ref(0)

  const canJoin = computed(() => Boolean(selectedGroupBuy.value && selectedGroupBuy.value.status === 'open' && selectedGroupBuy.value.available_shares > 0))
  const joinAmount = computed(() => normalizeJoinAmount())
  const detailVisible = computed({
    get: () => selectedGroupBuy.value !== null,
    set: (visible: boolean) => {
      if (!visible) selectedGroupBuy.value = null
    },
  })

  /** 计算当前详情最多可认购金额。 */
  function maxJoinAmount() {
    const available = Math.max(0, Number(selectedGroupBuy.value?.available_shares || 0))
    const shareAmount = Number(selectedGroupBuy.value?.share_amount || 0)
    const max = available * shareAmount
    return Number.isFinite(max) ? max : 0
  }

  /** 把认购金额限制在可参与范围内。 */
  function normalizeJoinAmount(value: string | number = joinAmountInput.value) {
    const amount = Math.max(0, Number(value || 0))
    const min = Math.max(0.01, Number(selectedGroupBuy.value?.share_amount || 0.01))
    const max = maxJoinAmount()
    const normalized = max > 0 ? Math.min(Math.max(amount, min), max) : Math.max(amount, min)
    return Number.isFinite(normalized) ? normalized.toFixed(2) : min.toFixed(2)
  }

  /** 减少认购金额。 */
  function decreaseJoinAmount() {
    const step = Math.max(0.01, Number(selectedGroupBuy.value?.share_amount || 0.01))
    joinAmountInput.value = normalizeJoinAmount(Number(joinAmountInput.value || 0) - step)
  }

  /** 增加认购金额。 */
  function increaseJoinAmount() {
    const step = Math.max(0.01, Number(selectedGroupBuy.value?.share_amount || 0.01))
    joinAmountInput.value = normalizeJoinAmount(Number(joinAmountInput.value || 0) + step)
  }

  /** 应用快捷认购金额。 */
  function applyQuickAmount(value: number | 'all') {
    joinAmountInput.value = value === 'all' ? normalizeJoinAmount(maxJoinAmount()) : normalizeJoinAmount(value)
  }

  /** 加载合买详情并忽略过期响应。 */
  async function loadDetail(groupBuyId: number) {
    const requestId = ++detailRequestSeq.value
    loadingDetail.value = true
    try {
      const res = await fetchGroupBuyDetail(groupBuyId)
      if (requestId !== detailRequestSeq.value || selectedGroupBuy.value?.id !== groupBuyId) return
      selectedGroupBuy.value = res.data || selectedGroupBuy.value
      joinAmountInput.value = String(selectedGroupBuy.value?.share_amount || '1.00')
    } catch (e: any) {
      if (requestId !== detailRequestSeq.value || selectedGroupBuy.value?.id !== groupBuyId) return
      showToast(e.response?.data?.detail || '加载合买详情失败')
    } finally {
      if (requestId === detailRequestSeq.value && selectedGroupBuy.value?.id === groupBuyId) {
        loadingDetail.value = false
      }
    }
  }

  /** 打开合买详情弹层。 */
  function openDetail(item: GroupBuyPlan) {
    selectedGroupBuy.value = item
    joinAmountInput.value = String(selectedGroupBuy.value?.share_amount || '1.00')
    loadDetail(item.id)
  }

  /** 关闭合买详情弹层。 */
  function closeDetail() {
    selectedGroupBuy.value = null
  }

  /** 提交认购并刷新余额、详情和列表。 */
  async function joinGroupBuy() {
    if (!selectedGroupBuy.value) return
    joinAmountInput.value = normalizeJoinAmount()
    submittingJoin.value = true
    try {
      const res = await joinGroupBuyPlan(selectedGroupBuy.value.id, joinAmountInput.value)
      selectedGroupBuy.value = res.data?.plan || selectedGroupBuy.value
      showToast(`参与成功，余额 ${res.data?.balance || '-'}，金额 ${joinAmountInput.value}`)
      await options.loadBalance()
      await loadDetail(selectedGroupBuy.value.id)
      await options.loadHall()
      if (options.activeTab.value === 'my') await options.loadMyGroupBuys()
    } catch (e: any) {
      showToast(e.response?.data?.detail || '参与合买失败')
    } finally {
      submittingJoin.value = false
    }
  }

  return {
    selectedGroupBuy,
    loadingDetail,
    submittingJoin,
    joinAmountInput,
    detailRequestSeq,
    canJoin,
    joinAmount,
    detailVisible,
    maxJoinAmount,
    normalizeJoinAmount,
    decreaseJoinAmount,
    increaseJoinAmount,
    applyQuickAmount,
    loadDetail,
    openDetail,
    closeDetail,
    joinGroupBuy,
  }
}
