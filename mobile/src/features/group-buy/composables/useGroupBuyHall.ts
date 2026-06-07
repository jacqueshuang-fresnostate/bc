import { computed, ref } from 'vue'
import { showToast } from 'vant'
import { errorMessage } from '../../../api/user'
import { fetchGroupBuyHall, fetchLotteryGroups } from '../api'
import { normalizeItems } from '../presentation'
import type { GroupBuyPlan } from '../types'

/** 创建合买大厅的模板方法状态流。 */
export function useGroupBuyHall(lotteryCode: { value: string }) {
  const hallItems = ref<GroupBuyPlan[]>([])
  const hallGroups = ref<any[]>([])
  const loadingHall = ref(false)
  const activeFilter = ref('all')
  const hallRequestSeq = ref(0)

  const hallCategoryChips = computed(() => [
    { label: '全部', value: 'all' },
    ...hallGroups.value.map(group => ({ label: String(group.name || group.code), value: String(group.code) })),
  ])
  const displayedHallItems = computed(() => hallItems.value)

  /** 加载后台配置的彩票分组。 */
  async function loadHallGroups() {
    try {
      const res = await fetchLotteryGroups()
      hallGroups.value = Array.isArray(res.data) ? res.data : []
    } catch {
      hallGroups.value = []
    }
  }

  /** 加载合买大厅，并保证只有最新请求能落盘。 */
  async function loadHall() {
    const requestId = ++hallRequestSeq.value
    const requestedLotteryCode = lotteryCode.value
    const requestedFilter = activeFilter.value
    loadingHall.value = true
    try {
      const params = {
        ...(requestedLotteryCode ? { lottery_code: requestedLotteryCode } : {}),
        ...(requestedFilter !== 'all' ? { group_code: requestedFilter } : {}),
      }
      const res = await fetchGroupBuyHall(params)
      if (requestId !== hallRequestSeq.value || lotteryCode.value !== requestedLotteryCode || activeFilter.value !== requestedFilter) return
      hallItems.value = normalizeItems(res.data)
    } catch (e: any) {
      if (requestId !== hallRequestSeq.value || lotteryCode.value !== requestedLotteryCode || activeFilter.value !== requestedFilter) return
      hallItems.value = []
      showToast(errorMessage(e, '加载合买大厅失败'))
    } finally {
      if (requestId === hallRequestSeq.value && lotteryCode.value === requestedLotteryCode && activeFilter.value === requestedFilter) {
        loadingHall.value = false
      }
    }
  }

  return {
    hallItems,
    hallGroups,
    loadingHall,
    activeFilter,
    hallRequestSeq,
    hallCategoryChips,
    displayedHallItems,
    loadHallGroups,
    loadHall,
  }
}
