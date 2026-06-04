import { computed, onBeforeUnmount, ref, type Ref } from 'vue'
import { showToast } from 'vant'
import http from '../api/http'
import { fetchLatestLotteryHistory, type LotteryHistoryItem } from '../api/lottery'
import { parseChinaDateTime } from '../utils/lotteryFormat'

export type LatestDrawItem = Pick<LotteryHistoryItem, 'issue' | 'result' | 'result_numbers'>

export function useBettingRound(lotteryCode: Ref<string>) {
  const issue = ref('')
  const roundStatus = ref('')
  const scheduledDrawAt = ref('')
  const loadingIssue = ref(false)
  const latestFc3dDraw = ref<LatestDrawItem | null>(null)
  const currentTime = ref(Date.now())
  let roundRequestVersion = 0
  let countdownTimer: number | undefined

  const latestFc3dNumbers = computed(() => {
    const item = latestFc3dDraw.value
    if (!item) return []
    if (Array.isArray(item.result_numbers) && item.result_numbers.length) return item.result_numbers.slice(0, 3).map(String)
    return String(item.result || '').replace(/\D/g, '').split('').slice(0, 3)
  })

  const fc3dCountdownText = computed(() => {
    if (!scheduledDrawAt.value) return '投注截止时间待更新'
    const target = parseChinaDateTime(scheduledDrawAt.value)
    if (!Number.isFinite(target)) return '投注截止时间待更新'
    const diff = Math.max(0, target - currentTime.value)
    const hours = Math.floor(diff / 3600000)
    const minutes = Math.floor((diff % 3600000) / 60000)
    const seconds = Math.floor((diff % 60000) / 1000)
    const minuteSecond = `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
    return hours > 0 ? `${String(hours).padStart(2, '0')}:${minuteSecond}` : minuteSecond
  })

  async function loadCurrentRound(silent = false) {
    const currentLotteryCode = lotteryCode.value
    const requestVersion = ++roundRequestVersion
    if (!currentLotteryCode) return
    if (!silent) loadingIssue.value = true
    try {
      const res = await http.get(`/bet/current-round/${currentLotteryCode}`)
      if (requestVersion !== roundRequestVersion || currentLotteryCode !== lotteryCode.value) return
      issue.value = res.data?.issue || ''
      roundStatus.value = res.data?.status || ''
      scheduledDrawAt.value = res.data?.scheduled_draw_at || ''
    } catch (e: any) {
      if (requestVersion !== roundRequestVersion || currentLotteryCode !== lotteryCode.value) return
      if (!silent) showToast(e.response?.data?.detail || '加载当前期号失败')
    } finally {
      if (!silent && requestVersion === roundRequestVersion && currentLotteryCode === lotteryCode.value) loadingIssue.value = false
    }
  }

  async function loadLatestFc3dDraw() {
    try {
      const data = await fetchLatestLotteryHistory({ lottery_code: 'fc3d' })
      const items = Array.isArray(data.items) ? data.items : []
      latestFc3dDraw.value = items[0] || null
    } catch {
      latestFc3dDraw.value = null
    }
  }

  function resetRound() {
    issue.value = ''
    roundStatus.value = ''
    scheduledDrawAt.value = ''
    latestFc3dDraw.value = null
  }

  function startRoundClock() {
    if (countdownTimer !== undefined) return
    countdownTimer = window.setInterval(() => {
      currentTime.value = Date.now()
    }, 1000)
  }

  onBeforeUnmount(() => {
    if (countdownTimer !== undefined) window.clearInterval(countdownTimer)
  })

  return {
    issue,
    roundStatus,
    scheduledDrawAt,
    loadingIssue,
    latestFc3dDraw,
    latestFc3dNumbers,
    fc3dCountdownText,
    loadCurrentRound,
    loadLatestFc3dDraw,
    resetRound,
    startRoundClock,
  }
}
