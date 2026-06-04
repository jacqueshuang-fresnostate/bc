import { computed, ref } from 'vue'
import http from '../api/http'

export function useLotteryHistory() {
  const activeGroupCode = ref('all')
  const lotteryGroups = ref<any[]>([])
  const drawItems = ref<any[]>([])
  const selectedLotteryCode = ref<string | null>(null)
  const selectedLotteryName = ref('')
  const selectedLotteryVisible = ref(false)
  const selectedLotteryItems = ref<any[]>([])
  const loadingDraws = ref(false)
  const drawRequestSeq = ref(0)
  const lotteryGroupsRequestSeq = ref(0)
  const loadingSelectedLottery = ref(false)
  const selectedLotteryRequestSeq = ref(0)

  const lotteryGroupFilters = computed(() => [
    { label: '全部', code: 'all' },
    ...lotteryGroups.value
      .map(group => ({ label: group.name || group.code, code: group.code }))
      .filter(group => group.code),
  ])
  const visibleDrawItems = computed(() => drawItems.value)

  async function loadLotteryGroups() {
    const requestId = ++lotteryGroupsRequestSeq.value
    try {
      const res = await http.get('/lottery/groups')
      if (requestId !== lotteryGroupsRequestSeq.value) return
      lotteryGroups.value = Array.isArray(res.data) ? res.data : []
    } catch {
      if (requestId !== lotteryGroupsRequestSeq.value) return
      lotteryGroups.value = []
    }
  }

  async function loadDrawHistory() {
    const requestId = ++drawRequestSeq.value
    const groupCode = activeGroupCode.value
    loadingDraws.value = true
    try {
      const params = activeGroupCode.value === 'all' ? undefined : { group_code: activeGroupCode.value }
      const res = await http.get('/lottery/history/latest', { params })
      if (requestId !== drawRequestSeq.value || activeGroupCode.value !== groupCode) return
      drawItems.value = Array.isArray(res.data?.items) ? res.data.items : []
    } catch {
      if (requestId !== drawRequestSeq.value || activeGroupCode.value !== groupCode) return
      drawItems.value = []
    } finally {
      if (requestId === drawRequestSeq.value && activeGroupCode.value === groupCode) {
        loadingDraws.value = false
      }
    }
  }

  async function loadSelectedDrawHistory(lotteryCode: string) {
    const requestId = ++selectedLotteryRequestSeq.value
    loadingSelectedLottery.value = true
    try {
      const res = await http.get('/lottery/history', { params: { lottery_code: lotteryCode } })
      if (requestId !== selectedLotteryRequestSeq.value || selectedLotteryCode.value !== lotteryCode) return
      selectedLotteryItems.value = Array.isArray(res.data?.items) ? res.data.items : []
    } catch {
      if (requestId !== selectedLotteryRequestSeq.value || selectedLotteryCode.value !== lotteryCode) return
      selectedLotteryItems.value = []
    } finally {
      if (requestId === selectedLotteryRequestSeq.value && selectedLotteryCode.value === lotteryCode) {
        loadingSelectedLottery.value = false
      }
    }
  }

  function openDrawHistory(item: any) {
    const lotteryCode = String(item?.lottery_code || '').trim()
    if (!lotteryCode) return
    selectedLotteryCode.value = lotteryCode
    selectedLotteryName.value = item.lottery_name || lotteryCode
    selectedLotteryItems.value = []
    selectedLotteryVisible.value = true
    loadSelectedDrawHistory(lotteryCode)
  }

  function closeDrawHistory() {
    selectedLotteryVisible.value = false
    selectedLotteryCode.value = null
    selectedLotteryName.value = ''
    selectedLotteryItems.value = []
  }

  return {
    activeGroupCode,
    lotteryGroups,
    lotteryGroupFilters,
    visibleDrawItems,
    selectedLotteryCode,
    selectedLotteryName,
    selectedLotteryVisible,
    selectedLotteryItems,
    loadingDraws,
    loadingSelectedLottery,
    loadLotteryGroups,
    loadDrawHistory,
    loadSelectedDrawHistory,
    openDrawHistory,
    closeDrawHistory,
  }
}
