import { computed, ref } from 'vue'
import { showToast } from 'vant'
import { errorMessage } from '../../../api/user'
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
  const joinAmountHint = computed(() => {
    const maxCents = maxJoinAmountCents()
    const minCents = effectiveMinimumJoinAmountCents()
    const shareCents = shareAmountCents()
    if (maxCents > 0 && maxCents <= participantMinAmountCents()) {
      return `剩余 ${centsToMoney(maxCents)} 元，可直接全包`
    }
    return `最低认购 ${centsToMoney(minCents)} 元，按每份 ${centsToMoney(shareCents)} 元递增`
  })
  const detailVisible = computed({
    get: () => selectedGroupBuy.value !== null,
    set: (visible: boolean) => {
      if (!visible) selectedGroupBuy.value = null
    },
  })

  /** 计算当前详情最多可认购金额。 */
  function maxJoinAmount() {
    return Number(centsToMoney(maxJoinAmountCents()))
  }

  /** 把认购金额限制在可参与范围内。 */
  function normalizeJoinAmount(value: string | number = joinAmountInput.value) {
    const shareCents = shareAmountCents()
    const maxCents = maxJoinAmountCents()
    const minCents = effectiveMinimumJoinAmountCents()
    let cents = moneyToCents(value)
    if (cents <= 0) cents = minCents
    cents = roundDownToMultiple(cents, shareCents)
    if (cents < minCents) cents = minCents
    if (maxCents > 0 && cents > maxCents) cents = maxCents
    const remainingAfter = maxCents - cents
    if (remainingAfter > 0 && remainingAfter < participantMinAmountCents()) {
      cents = maxCents
    }
    return centsToMoney(cents)
  }

  /** 用户完成编辑认购金额后，再把金额校正到可参与范围。 */
  function commitJoinAmountInput() {
    joinAmountInput.value = normalizeJoinAmount()
  }

  /** 减少认购金额。 */
  function decreaseJoinAmount() {
    joinAmountInput.value = normalizeJoinAmount(centsToMoney(moneyToCents(joinAmountInput.value) - shareAmountCents()))
  }

  /** 增加认购金额。 */
  function increaseJoinAmount() {
    joinAmountInput.value = normalizeJoinAmount(centsToMoney(moneyToCents(joinAmountInput.value) + shareAmountCents()))
  }

  /** 应用快捷认购金额。 */
  function applyQuickAmount(value: number | 'all') {
    joinAmountInput.value = value === 'all' ? normalizeJoinAmount(maxJoinAmount()) : normalizeJoinAmount(value)
  }

  /** 加载合买详情并忽略过期响应。 */
  async function loadDetail(groupBuyId: string) {
    const requestId = ++detailRequestSeq.value
    loadingDetail.value = true
    try {
      const res = await fetchGroupBuyDetail(groupBuyId)
      if (requestId !== detailRequestSeq.value || selectedGroupBuy.value?.id !== groupBuyId) return
      selectedGroupBuy.value = res.data || selectedGroupBuy.value
      resetJoinAmountInput()
    } catch (e: any) {
      if (requestId !== detailRequestSeq.value || selectedGroupBuy.value?.id !== groupBuyId) return
      showToast(errorMessage(e, '加载合买详情失败'))
    } finally {
      if (requestId === detailRequestSeq.value && selectedGroupBuy.value?.id === groupBuyId) {
        loadingDetail.value = false
      }
    }
  }

  /** 打开合买详情弹层。 */
  function openDetail(item: GroupBuyPlan) {
    selectedGroupBuy.value = item
    resetJoinAmountInput()
    loadDetail(item.id)
  }

  /** 关闭合买详情弹层。 */
  function closeDetail() {
    selectedGroupBuy.value = null
  }

  /** 提交认购并刷新余额、详情和列表。 */
  async function joinGroupBuy() {
    if (!selectedGroupBuy.value) return
    commitJoinAmountInput()
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
      showToast(errorMessage(e, '参与合买失败'))
    } finally {
      submittingJoin.value = false
    }
  }

  /** 重置认购输入为当前计划最小可提交金额。 */
  function resetJoinAmountInput() {
    joinAmountInput.value = normalizeJoinAmount(centsToMoney(effectiveMinimumJoinAmountCents()))
  }

  /** 当前计划单份金额，所有认购金额都必须按完整份额取整。 */
  function shareAmountCents() {
    return Math.max(1, moneyToCents(selectedGroupBuy.value?.share_amount || '0.01'))
  }

  /** 当前计划参与人最低认购金额，至少等于一份金额。 */
  function participantMinAmountCents() {
    return roundUpToMultiple(
      Math.max(shareAmountCents(), moneyToCents(selectedGroupBuy.value?.participant_min_amount || selectedGroupBuy.value?.share_amount || '0.01')),
      shareAmountCents(),
    )
  }

  /** 当前计划剩余可认购金额。 */
  function maxJoinAmountCents() {
    const available = Math.max(0, Number(selectedGroupBuy.value?.available_shares || 0))
    const max = available * shareAmountCents()
    return Number.isFinite(max) ? Math.max(0, Math.trunc(max)) : 0
  }

  /** 剩余不足最低认购金额时，允许用户直接全包该尾单。 */
  function effectiveMinimumJoinAmountCents() {
    const maxCents = maxJoinAmountCents()
    const minCents = participantMinAmountCents()
    if (maxCents > 0 && maxCents < minCents) return maxCents
    return minCents
  }

  /** 把展示金额转换为分，避免浮点计算影响份额取整。 */
  function moneyToCents(value: string | number | null | undefined) {
    const text = String(value ?? '').trim()
    if (!/^\d+(?:\.\d{0,2})?$/.test(text)) return 0
    const [whole, fraction = ''] = text.split('.')
    return Number(whole || 0) * 100 + Number(fraction.padEnd(2, '0').slice(0, 2) || 0)
  }

  /** 把分格式化为两位小数金额。 */
  function centsToMoney(value: number) {
    const cents = Number.isFinite(value) ? Math.max(0, Math.trunc(value)) : 0
    return (cents / 100).toFixed(2)
  }

  /** 向下取到完整份额。 */
  function roundDownToMultiple(value: number, step: number) {
    if (step <= 0) return Math.max(0, Math.trunc(value))
    return Math.floor(Math.max(0, value) / step) * step
  }

  /** 向上取到完整份额，用于最低认购金额不是单份整数倍时。 */
  function roundUpToMultiple(value: number, step: number) {
    if (step <= 0) return Math.max(0, Math.trunc(value))
    return Math.ceil(Math.max(0, value) / step) * step
  }

  return {
    selectedGroupBuy,
    loadingDetail,
    submittingJoin,
    joinAmountInput,
    detailRequestSeq,
    canJoin,
    joinAmount,
    joinAmountHint,
    detailVisible,
    maxJoinAmount,
    normalizeJoinAmount,
    commitJoinAmountInput,
    decreaseJoinAmount,
    increaseJoinAmount,
    applyQuickAmount,
    loadDetail,
    openDetail,
    closeDetail,
    joinGroupBuy,
  }
}
