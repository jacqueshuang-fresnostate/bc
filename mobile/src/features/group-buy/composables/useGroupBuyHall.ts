import { computed, ref } from 'vue'
import { showToast } from 'vant'
import { errorMessage } from '../../../api/user'
import { fetchGroupBuyHall, fetchLotteryGroups } from '../api'
import { normalizeItems } from '../presentation'
import type { GroupBuyPlan } from '../types'

const GROUP_BUY_PAGE_SIZE = 12

/** 创建合买大厅的模板方法状态流。 */
export function useGroupBuyHall(lotteryCode: { value: string }) {
  const hallItems = ref<GroupBuyPlan[]>([])
  const hallGroups = ref<any[]>([])
  const loadingHall = ref(false)
  const activeFilter = ref('all')
  const hallRequestSeq = ref(0)
  const hallPage = ref(0)
  const hallHasMore = ref(true)

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
  async function loadHall(options: { append?: boolean } = {}) {
    const append = Boolean(options.append)
    if (append && !hallHasMore.value) return
    const requestId = ++hallRequestSeq.value
    const requestedLotteryCode = lotteryCode.value
    const requestedFilter = activeFilter.value
    const requestedPage = append ? hallPage.value + 1 : 1
    loadingHall.value = true
    try {
      const params = {
        ...(requestedLotteryCode ? { lottery_code: requestedLotteryCode } : {}),
        ...(requestedFilter !== 'all' ? { group_code: requestedFilter } : {}),
        page: requestedPage,
        pageSize: GROUP_BUY_PAGE_SIZE,
      }
      const res = await fetchGroupBuyHall(params)
      if (requestId !== hallRequestSeq.value || lotteryCode.value !== requestedLotteryCode || activeFilter.value !== requestedFilter) return
      const items = normalizeItems(res.data)
      hallItems.value = append ? mergePlans(hallItems.value, items) : items
      hallPage.value = items.length > 0 ? requestedPage : (append ? hallPage.value : 0)
      hallHasMore.value = items.length >= GROUP_BUY_PAGE_SIZE
    } catch (e: any) {
      if (requestId !== hallRequestSeq.value || lotteryCode.value !== requestedLotteryCode || activeFilter.value !== requestedFilter) return
      if (!append) {
        hallItems.value = []
        hallPage.value = 0
        hallHasMore.value = true
      }
      showToast(errorMessage(e, '加载合买大厅失败'))
    } finally {
      if (requestId === hallRequestSeq.value && lotteryCode.value === requestedLotteryCode && activeFilter.value === requestedFilter) {
        loadingHall.value = false
      }
    }
  }

  /** 按计划编号追加去重，避免实时刷新和翻页造成重复卡片。 */
  function mergePlans(current: GroupBuyPlan[], incoming: GroupBuyPlan[]) {
    const seen = new Set<string>()
    return [...current, ...incoming].filter(item => {
      if (!item.id || seen.has(item.id)) return false
      seen.add(item.id)
      return true
    })
  }

  return {
    hallItems,
    hallGroups,
    loadingHall,
    activeFilter,
    hallRequestSeq,
    hallPage,
    hallHasMore,
    hallCategoryChips,
    displayedHallItems,
    loadHallGroups,
    loadHall,
  }
}
