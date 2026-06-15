<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRoute, useRouter } from 'vue-router'
import { showNotify, showToast } from 'vant'
import { errorMessage } from '../../../utils/errorMessage'
import { parseChinaDateTime } from '../../../utils/lotteryFormat'
import { useMobileUserDataStore } from '../../../stores/mobileUserData'
import { createGroupBuyPlan } from '../../group-buy/api'
import { calculateFixedShareCount, calculateRecommendedSelfShares, calculateRequiredSelfShares } from '../../group-buy/presentation'
import { useBetBatchSubmit } from '../composables/useBetBatchSubmit'
import { useBetPageConfig } from '../dynamic/useBetPageConfig'
import { limitPositionValues, randomLimitPositionValues, randomSubsetValues } from '../dynamic/positionLimits'
import { useDynamicBetEngine } from '../dynamic/useDynamicBetEngine'
import type { DynamicBetPlay } from '../dynamic/types'
import BetRoundInfoCard from './BetRoundInfoCard.vue'
import DynamicInputRenderer from './DynamicInputRenderer.vue'
import DynamicPlayTabs from './DynamicPlayTabs.vue'
import MobilePageShell from './MobilePageShell.vue'
import MobileTopBar from './MobileTopBar.vue'
import UnifiedBetBottomBar from './UnifiedBetBottomBar.vue'

const props = defineProps<{ wsMessage?: any }>()
const OPENING_REFRESH_INTERVAL_MS = 3000

const route = useRoute()
const router = useRouter()
const userDataStore = useMobileUserDataStore()
const { profile } = storeToRefs(userDataStore)
const { config, loading, loadBetPageConfig } = useBetPageConfig()
const { submitBatch } = useBetBatchSubmit()

// 动态投注页状态边界：路由决定彩种，配置决定玩法，引擎只接管草稿和篮子。
const selectedPlayCode = ref('')
const currentTime = ref(Date.now())
const showPlayPopup = ref(false)
const groupBuyMode = ref(false)
const groupBuyShareCount = ref(10)
const groupBuySelfShares = ref(1)
const groupBuySelfSharesTouched = ref(false)
const submittingBet = ref(false)
const submittingGroupBuy = ref(false)
let timer: number | undefined
let openingRefreshTimer: number | undefined
let openingRefreshInFlight = false
let drawRefreshSeq = 0

const lotteryCode = computed(() => String(route.params.code || ''))
const balance = computed(() => profile.value?.balance || '0.00')
const selectedPlay = computed(() => config.value?.plays.find(play => play.code === selectedPlayCode.value) || config.value?.plays[0] || null)
// 批量提交需要玩法元数据，用于把本地篮子号码转换成后端标准 selection。
const playSubmitMeta = computed(() => Object.fromEntries((config.value?.plays || []).map(play => [play.code, {
  inputMode: play.input_mode,
  ruleCode: play.rule_code || play.code,
  positionGridKind: play.position_grid_kind,
  optionGroups: play.option_groups,
}])))
const latestNumbers = computed(() => config.value?.latest_draw?.result_numbers || [])
const latestIssue = computed(() => config.value?.latest_draw?.issue || '')
const multipleInputValue = ref('1')
function formatBetCutoffCountdown(diff: number) {
  if (diff <= 0) return '开奖中'
  const hours = Math.floor(diff / 3600000)
  const minutes = Math.floor((diff % 3600000) / 60000)
  const seconds = Math.floor((diff % 60000) / 1000)
  const minuteSecond = `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`
  return hours > 0 ? `${String(hours).padStart(2, '0')}:${minuteSecond}` : minuteSecond
}

const countdownText = computed(() => {
  const saleStopAt = config.value?.round.sale_stop_at
  const targetText = saleStopAt || config.value?.round.scheduled_draw_at
  if (!targetText) return '00:00'
  const target = parseChinaDateTime(targetText)
  if (!Number.isFinite(target)) return '00:00'
  return formatBetCutoffCountdown(target - currentTime.value)
})

const engine = useDynamicBetEngine(() => config.value, () => selectedPlay.value)
const multipleSliderMax = computed(() => engine.maxMultiple.value || Math.max(100, engine.minMultiple.value))
const canDecreaseMultiple = computed(() => Number(engine.multiple.value || 0) > engine.minMultiple.value)
const canIncreaseMultiple = computed(() => Number(engine.multiple.value || 0) < multipleSliderMax.value)
const groupBuyMinShareAmount = computed(() => Number(config.value?.group_buy_settings.min_share_amount || 0.01))
const groupBuyInitiatorMinBuyRatio = computed(() => config.value?.group_buy_settings.initiator_min_buy_ratio || '0.00')
const groupBuyAvailable = computed(() => Boolean(config.value?.lottery.group_buy_enabled))
const groupBuyTotalAmount = computed(() => engine.cartTotalAmount.value + engine.draftAmount.value)
const groupBuyTotalAmountText = computed(() => groupBuyTotalAmount.value.toFixed(2))
const groupBuyFixedShareAmount = computed(() => config.value?.group_buy_settings.share_amount || '1.00')
const groupBuyInitiatorMinBuyRatioText = computed(() => {
  const ratio = Number(groupBuyInitiatorMinBuyRatio.value)
  if (!Number.isFinite(ratio)) return `${groupBuyInitiatorMinBuyRatio.value}%`
  return `${ratio.toLocaleString('zh-CN', { maximumFractionDigits: 2 })}%`
})
const groupBuyDerivedShareCount = computed(() => calculateFixedShareCount(groupBuyTotalAmountText.value, groupBuyFixedShareAmount.value))
const groupBuySafeShareCount = computed(() => groupBuyDerivedShareCount.value > 0 ? groupBuyDerivedShareCount.value : Math.max(1, Math.floor(Number(groupBuyShareCount.value || 1))))
const groupBuySafeSelfShares = computed(() => Math.min(groupBuySafeShareCount.value, Math.max(0, Math.floor(Number(groupBuySelfShares.value || 0)))))
const groupBuyShareAmount = groupBuyFixedShareAmount
const groupBuyRequiredSelfShares = computed(() => calculateRequiredSelfShares(
  groupBuyTotalAmountText.value,
  groupBuyFixedShareAmount.value,
  groupBuyInitiatorMinBuyRatio.value,
))
const groupBuyRecommendedSelfShares = computed(() => calculateRecommendedSelfShares(
  groupBuyTotalAmountText.value,
  groupBuyFixedShareAmount.value,
  groupBuyInitiatorMinBuyRatio.value,
))
const groupBuySelfSharesHint = computed(() => {
  if (groupBuyRecommendedSelfShares.value > 0) {
    const actionText = groupBuySafeSelfShares.value === groupBuyRecommendedSelfShares.value ? '已自动匹配' : '建议'
    return `最低自购${groupBuyInitiatorMinBuyRatioText.value}，${actionText} ${groupBuyRecommendedSelfShares.value} 份`
  }
  return `发起人最低自购${groupBuyInitiatorMinBuyRatioText.value}`
})
const groupBuyPaymentAmount = computed(() => (Number(groupBuyShareAmount.value) * groupBuySafeSelfShares.value).toFixed(2))
const roundOpening = computed(() => config.value?.round.status === 'opening')
const roundSaleClosed = computed(() => {
  const saleStopAt = config.value?.round.sale_stop_at
  if (!saleStopAt) return false
  const target = parseChinaDateTime(saleStopAt)
  return Number.isFinite(target) && target <= currentTime.value
})
const roundSelling = computed(() => config.value?.round.status === 'selling' && Boolean(config.value.round.issue))
const roundAcceptingBet = computed(() => roundSelling.value && !roundSaleClosed.value)
// 后端短暂未把过期 open 期关盘时，前端也要进入轮询，避免页面停在“开奖中”。
const roundNeedsOpeningRefresh = computed(() => roundOpening.value || (roundSelling.value && roundSaleClosed.value))
// 展示态和投注态分开：封盘时间已到后，即使后端还短暂返回 selling，页面也展示为开奖中。
const roundDisplayStatus = computed(() => roundNeedsOpeningRefresh.value ? 'opening' : (config.value?.round.status || ''))
const canSubmitCurrentOrder = computed(() => roundAcceptingBet.value && (engine.draftBetCount.value > 0 || engine.cartTotalCount.value > 0))
const submitButtonText = computed(() => {
  if (groupBuyMode.value) return '发起合买'
  return '立即投注'
})
const submittingOrder = computed(() => submittingBet.value || submittingGroupBuy.value)
const submitButtonDisplayText = computed(() => {
  if (submittingGroupBuy.value) return '发布中...'
  if (submittingBet.value) return '投注中...'
  return submitButtonText.value
})
const submitLoadingText = computed(() => submittingGroupBuy.value ? '正在发布合买...' : '正在提交投注...')

async function loadBalance(options: { force?: boolean; silent?: boolean } = {}) {
  try {
    await userDataStore.loadProfile(options)
  } catch {
    // 余额失败不阻断投注页配置渲染，页面以缓存或 0.00 作为兜底展示。
  }
}

async function loadPage(options: { force?: boolean; silent?: boolean } = {}) {
  if (!lotteryCode.value) return
  // 页面进入、期开奖和投注后都复用同一加载流程，保持配置、期号和余额同步。
  await Promise.all([loadBetPageConfig(lotteryCode.value), loadBalance(options)])
}

function stopOpeningRefresh() {
  if (openingRefreshTimer === undefined) return
  window.clearInterval(openingRefreshTimer)
  openingRefreshTimer = undefined
}

async function refreshOpeningRoundConfig() {
  if (openingRefreshInFlight || !roundNeedsOpeningRefresh.value || !lotteryCode.value) return
  openingRefreshInFlight = true
  try {
    await loadBetPageConfig(lotteryCode.value, { silent: true })
    if (!roundNeedsOpeningRefresh.value) await loadBalance({ force: true, silent: true })
  } catch {
    // 开盘轮询允许短暂失败，下一轮继续探测期号状态。
  } finally {
    openingRefreshInFlight = false
    syncOpeningRefresh()
  }
}

function syncOpeningRefresh() {
  if (!roundNeedsOpeningRefresh.value || !lotteryCode.value) {
    stopOpeningRefresh()
    return
  }
  if (openingRefreshTimer !== undefined) return
  openingRefreshTimer = window.setInterval(() => {
    void refreshOpeningRoundConfig()
  }, OPENING_REFRESH_INTERVAL_MS)
}

function waitForDrawRefresh(delay: number) {
  return new Promise(resolve => window.setTimeout(resolve, delay))
}

function roundHasFutureDrawTime(issue: string) {
  const round = config.value?.round
  if (!round?.issue || round.issue === issue) return false
  const scheduledDrawAt = round.scheduled_draw_at ? parseChinaDateTime(round.scheduled_draw_at) : NaN
  return Number.isFinite(scheduledDrawAt) && scheduledDrawAt > Date.now()
}

async function refreshAfterDrawResult(msg: any) {
  const sequence = ++drawRefreshSeq
  for (const delay of [0, 600, 1200]) {
    if (delay) await waitForDrawRefresh(delay)
    if (sequence !== drawRefreshSeq) return
    await loadPage()
    if (roundHasFutureDrawTime(String(msg?.issue || ''))) return
  }
}

function selectPlay(play: DynamicBetPlay) {
  selectedPlayCode.value = play.code
  showPlayPopup.value = false
}

function selectAllPosition(positionKey: string) {
  const play = selectedPlay.value
  if (!play) return
  // 胆拖玩法不能让胆码/拖码互相重叠，全选时也要按当前位置排除对侧号码。
  if (play.position_grid_kind === 'group3_dantuo' || play.position_grid_kind === 'group6_dantuo') {
    const index = play.positions.findIndex(position => position.key === positionKey)
    const oppositeKey = play.positions[index === 0 ? 1 : 0]?.key
    const oppositeValues = new Set(oppositeKey ? engine.selections.value[oppositeKey] || [] : [])
    const availableDigits = play.digits.filter(digit => !oppositeValues.has(digit))
    const dantuoValues = index === 0 ? randomSubsetValues(availableDigits, play.position_grid_kind === 'group6_dantuo' ? 2 : 1) : availableDigits
    engine.setPositionNumbers(positionKey, randomLimitPositionValues(play, positionKey, dantuoValues))
    return
  }
  // 位置玩法的全选来源于当前玩法 digits，避免跨玩法复用旧号码池。
  engine.setPositionNumbers(positionKey, randomLimitPositionValues(play, positionKey, play.digits))
}

function selectPresetPosition(positionKey: string, values: string[]) {
  const play = selectedPlay.value
  if (!play) return
  const validValues = values.filter(value => play.digits.includes(value))
  if (play.position_grid_kind === 'group3_dantuo' || play.position_grid_kind === 'group6_dantuo') {
    const index = play.positions.findIndex(position => position.key === positionKey)
    const oppositeKey = play.positions[index === 0 ? 1 : 0]?.key
    const oppositeValues = new Set(oppositeKey ? engine.selections.value[oppositeKey] || [] : [])
    const availableDigits = validValues.filter(digit => !oppositeValues.has(digit))
    const dantuoValues = index === 0 ? availableDigits.slice(0, play.position_grid_kind === 'group6_dantuo' ? 2 : 1) : availableDigits
    engine.setPositionNumbers(positionKey, limitPositionValues(play, positionKey, dantuoValues))
    return
  }
  engine.setPositionNumbers(positionKey, limitPositionValues(play, positionKey, validValues))
}

function clearPosition(positionKey: string) {
  engine.setPositionNumbers(positionKey, [])
}

function normalizeMultipleInput() {
  engine.multiple.value = engine.clampMultiple(multipleInputValue.value || engine.minMultiple.value, selectedPlay.value)
  multipleInputValue.value = String(engine.multiple.value)
}

function updateMultipleInput(event: Event) {
  const input = event.target as HTMLInputElement
  const digits = input.value.replace(/\D/g, '')
  multipleInputValue.value = digits
  input.value = digits
  if (!digits) return
  engine.multiple.value = engine.clampMultiple(Number(digits), selectedPlay.value)
}

function adjustMultiple(delta: number) {
  const current = Number(engine.multiple.value || engine.minMultiple.value)
  engine.multiple.value = engine.clampMultiple(current + delta, selectedPlay.value)
  multipleInputValue.value = String(engine.multiple.value)
}

function normalizeGroupBuyShares() {
  if (groupBuyDerivedShareCount.value > 0) groupBuyShareCount.value = groupBuyDerivedShareCount.value
  applyRecommendedGroupBuySelfShares()
}

function clampGroupBuySelfShares() {
  groupBuySelfSharesTouched.value = true
  const maxShares = groupBuySafeShareCount.value
  const recommendedShares = groupBuyRecommendedSelfShares.value
  if (maxShares <= 0 || recommendedShares <= 0) {
    groupBuySelfShares.value = 0
    return
  }
  const currentShares = Math.floor(Number(groupBuySelfShares.value || 0))
  groupBuySelfShares.value = Math.min(maxShares, Math.max(recommendedShares, currentShares))
}

function touchGroupBuySelfShares() {
  groupBuySelfSharesTouched.value = true
}

function applyRecommendedGroupBuySelfShares(force = false) {
  if (!groupBuyMode.value) return
  const maxShares = groupBuySafeShareCount.value
  const recommendedShares = groupBuyRecommendedSelfShares.value
  if (maxShares <= 0 || recommendedShares <= 0) {
    groupBuySelfShares.value = 0
    return
  }

  const currentShares = Math.floor(Number(groupBuySelfShares.value || 0))
  if (force || !groupBuySelfSharesTouched.value || currentShares < recommendedShares) {
    groupBuySelfShares.value = recommendedShares
    return
  }
  if (currentShares > maxShares) {
    groupBuySelfShares.value = maxShares
  }
}

async function submitGroupBuyCart() {
  if (submittingOrder.value) return
  if (!config.value?.round.issue) {
    showToast('当前期号未就绪')
    return
  }
  if (engine.draftBetCount.value > 0 && !engine.addDraftToCart({ silent: true })) return
  if (engine.cart.value.length !== 1) {
    showToast('合买一次只能发起一张单据')
    return
  }
  normalizeGroupBuyShares()
  const item = engine.cart.value[0]
  const totalAmount = item.unit_amount * item.bet_count * item.multiple
  if (groupBuyShareCount.value <= 0 || calculateFixedShareCount(totalAmount.toFixed(2), groupBuyFixedShareAmount.value) <= 0) {
    showToast('总金额必须能按每份金额整除')
    return
  }
  if (Number(groupBuyFixedShareAmount.value) < groupBuyMinShareAmount.value) {
    showToast(`每份金额不能低于 ${config.value.group_buy_settings.min_share_amount}`)
    return
  }
  if (groupBuyRequiredSelfShares.value > 0 && groupBuySelfShares.value < groupBuyRequiredSelfShares.value) {
    showToast(`发起人最低自购 ${groupBuyRequiredSelfShares.value} 份`)
    return
  }
  try {
    submittingGroupBuy.value = true
    await createGroupBuyPlan({
      lottery_code: lotteryCode.value,
      issue: config.value.round.issue,
      play_code: item.play_code,
      title: `${config.value.lottery.name}合买`,
      numbers: item.numbers,
      total_amount: totalAmount.toFixed(2),
      share_count: groupBuyShareCount.value,
      share_amount: groupBuyFixedShareAmount.value,
      reserved_shares: 0,
      self_shares: groupBuySelfShares.value,
    })
    showToast('合买发起成功')
    engine.clearCart()
    groupBuyMode.value = false
    await loadBalance({ force: true, silent: true })
    await router.replace({ name: 'Home' })
  } catch (e: any) {
    showToast(errorMessage(e, '发起合买失败'))
    await loadPage()
  } finally {
    submittingGroupBuy.value = false
  }
}

async function submitCart() {
  if (submittingOrder.value) return
  if (groupBuyMode.value) {
    await submitGroupBuyCart()
    return
  }
  if (engine.draftBetCount.value > 0) {
    // 用户直接点提交时静默把当前有效草稿转成待提交单据，避免出现多余提示。
    const added = engine.addDraftToCart({ silent: true })
    if (!added) return
  }
  try {
    submittingBet.value = true
    const payload = await submitBatch(lotteryCode.value, config.value?.round.issue || '', engine.cart.value, playSubmitMeta.value)
    if (!payload) return
    // 提交成功后清空本地篮子并回到首页，避免用户继续停留在已完成的下注页重复操作。
    engine.clearCart()
    await loadBalance({ force: true, silent: true })
    await router.replace({ name: 'Home' })
  } catch (e: any) {
    // 提交失败也刷新一次，避免前端继续停留在已封盘或余额变化前的状态。
    showToast(errorMessage(e, '投注失败'))
    await loadPage()
  } finally {
    submittingBet.value = false
  }
}

watch(lotteryCode, async () => {
  // 路由切换彩种时必须清空玩法选择、草稿和篮子，防止旧彩种单据混入新彩种。
  stopOpeningRefresh()
  selectedPlayCode.value = ''
  groupBuyMode.value = false
  engine.clearCart()
  engine.resetDraft(null)
  try {
    await loadPage()
  } catch (e: any) {
    showToast(errorMessage(e, '加载投注页失败'))
  }
}, { immediate: true })

watch(() => config.value?.plays, (plays) => {
  if (!groupBuyAvailable.value) groupBuyMode.value = false
  if (!plays?.length) {
    selectedPlayCode.value = ''
    return
  }
  // 配置刷新后若原玩法仍存在则保留，否则自动落到服务端返回的第一个玩法。
  if (!plays.some(play => play.code === selectedPlayCode.value)) selectedPlayCode.value = plays[0].code
}, { immediate: true })

// 当前玩法变化即重置草稿，使输入模式、位置键和固定选项值与新玩法一致。
watch(selectedPlay, (play) => engine.resetDraft(play), { immediate: true })

watch(() => engine.multiple.value, (value) => {
  multipleInputValue.value = String(value || engine.minMultiple.value)
}, { immediate: true })

watch(roundNeedsOpeningRefresh, syncOpeningRefresh, { immediate: true })

watch(groupBuyMode, (enabled) => {
  if (!enabled) return
  groupBuySelfSharesTouched.value = false
  applyRecommendedGroupBuySelfShares(true)
})

watch(
  [groupBuyRecommendedSelfShares, groupBuySafeShareCount],
  () => applyRecommendedGroupBuySelfShares(),
)

watch(() => props.wsMessage, async (msg) => {
  const messageLotteryCode = msg?.lotteryCode || msg?.lottery_code
  if (!messageLotteryCode || messageLotteryCode !== lotteryCode.value) return

  if (msg?.event === 'draw_result') {
    // 广播可能早于下一期完全可读，短暂重试避免页面停在已封盘的 00:00。
    showNotify({ type: 'success', message: `开奖结果：${msg.result}` })
    await refreshAfterDrawResult(msg)
    return
  }

  if (msg?.event === 'issue_opened' || msg?.event === 'issue_closed') {
    await loadPage()
  }
})

// 用本地定时器只驱动倒计时文本，不参与服务端封盘判断。
timer = window.setInterval(() => {
  currentTime.value = Date.now()
}, 1000)

onBeforeUnmount(() => {
  if (timer !== undefined) window.clearInterval(timer)
  stopOpeningRefresh()
})
</script>

<template>
  <MobilePageShell>
    <MobileTopBar :title="config?.lottery.name || lotteryCode" :balance="balance" @back="router.back()" />

    <main class="bet-page-main mx-auto max-w-md space-y-3 px-4 pt-4" :class="{ 'bet-page-main--group-buy': groupBuyMode }">
      <BetRoundInfoCard :issue="config?.round.issue || ''" :status="roundDisplayStatus" :countdown-text="countdownText" :latest-issue="latestIssue" :latest-numbers="latestNumbers" />

      <button
        class="flex w-full items-center justify-between gap-3 rounded-[22px] border border-[#f1dedb] bg-[#fffdfc] px-4 py-3 text-left shadow-sm shadow-red-900/5 transition active:scale-[0.99] active:bg-[#fff7f5]"
        type="button"
        @click="showPlayPopup = true"
      >
        <span class="min-w-0 flex-1">
          <span class="block text-[11px] font-bold tracking-wider text-[#8e706d]">选择玩法</span>
          <strong class="mt-0.5 flex min-w-0 items-center gap-2 font-headline text-lg font-extrabold text-[#1a1c1c]">
            <span class="truncate">{{ selectedPlay ? (selectedPlay.full_name || selectedPlay.name) : '请选择玩法' }}</span>
            <span v-if="selectedPlay?.odds" class="shrink-0 rounded-full bg-[#fff4dc] px-2 py-0.5 text-[11px] font-bold text-[#735c00]">赔率 {{ selectedPlay.odds }}</span>
            <span v-if="selectedPlay" class="shrink-0 rounded-full bg-[#fff4dc] px-2 py-0.5 text-[11px] font-bold text-[#735c00]">单注 ¥{{ Number(engine.effectiveUnitAmount.value || 0).toFixed(2) }}</span>
          </strong>
          <small v-if="selectedPlay?.simple_description" class="mt-0.5 block truncate text-xs font-bold text-[#8c0a15]">{{ selectedPlay.simple_description }}</small>
        </span>
        <span class="flex shrink-0 items-center gap-1.5">
          <span class="rounded-full bg-[#ffdad7] px-2.5 py-1 text-xs font-bold text-[#8c0a15]">切换</span>
        </span>
      </button>

      <DynamicInputRenderer
        :play="selectedPlay"
        :numbers="engine.textNumbers.value"
        :selections="engine.selections.value"
        @update:numbers="engine.textNumbers.value = $event"
        @toggle-position="engine.togglePositionNumber"
        @select-all-position="selectAllPosition"
        @select-preset-position="selectPresetPosition"
        @clear-position="clearPosition"
        @toggle-option="engine.toggleOptionValue"
      />

      <section class="rounded-[28px] bg-white p-5 shadow-sm shadow-red-900/5">
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0">
            <div class="text-xs font-bold tracking-wider text-[#5a403e]">投注倍数</div>
            <p class="mt-1 text-xs font-bold text-[#8e706d]">允许 {{ engine.minMultiple.value }}-{{ multipleSliderMax }} 倍</p>
          </div>
          <div class="flex shrink-0 items-center rounded-[20px] border border-[#f1dedb] bg-[#fffdfc] p-1 shadow-inner shadow-red-900/5" role="group" aria-label="投注倍数">
            <button class="flex h-9 w-9 items-center justify-center rounded-2xl bg-[#f8f1ef] text-xl font-black text-[#8c0a15] transition active:scale-95 disabled:cursor-not-allowed disabled:text-[#b8aaa8] disabled:opacity-55" type="button" :disabled="!canDecreaseMultiple" aria-label="减少倍数" @click="adjustMultiple(-1)">-</button>
            <input
              :value="multipleInputValue"
              class="mx-1 h-9 w-16 rounded-2xl bg-transparent text-center font-headline text-2xl font-extrabold leading-none text-[#1a1c1c] outline-none"
              type="text"
              inputmode="numeric"
              pattern="[0-9]*"
              aria-label="投注倍数"
              @input="updateMultipleInput"
              @blur="normalizeMultipleInput"
              @keyup.enter="normalizeMultipleInput"
            />
            <span class="mr-1 text-sm font-extrabold text-[#5a403e]">倍</span>
            <button class="flex h-9 w-9 items-center justify-center rounded-2xl bg-[#8c0a15] text-xl font-black !text-white shadow-sm shadow-red-900/15 transition active:scale-95 disabled:cursor-not-allowed disabled:bg-[#d8d1cf] disabled:!text-white/80 disabled:shadow-none" type="button" :disabled="!canIncreaseMultiple" aria-label="增加倍数" @click="adjustMultiple(1)">
              <span class="!text-white">+</span>
            </button>
          </div>
        </div>
        <van-slider v-model="engine.multiple.value" class="mt-5" :min="engine.minMultiple.value" :max="multipleSliderMax" :step="1" active-color="#af2829" inactive-color="#eeeeee" />
      </section>

      <section
        v-if="groupBuyAvailable"
        class="group-buy-bet-mode overflow-hidden rounded-[30px] border bg-white p-5 shadow-sm transition-all duration-300"
        :class="groupBuyMode ? 'group-buy-bet-mode--active border-[#f6c9a6] shadow-[0_18px_42px_rgba(140,10,21,0.13)]' : 'border-[#f1dedb] shadow-red-900/5'"
      >
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <div class="text-xs font-bold tracking-wider text-[#8e706d]">投注模式</div>
            <strong class="mt-1 block font-headline text-xl font-extrabold text-[#1a1c1c]">合买</strong>
            <p class="mt-1 text-xs font-bold text-[#8e706d]">开启后按份公开合买，不需要设置保底。</p>
          </div>
          <van-switch v-model="groupBuyMode" active-color="#8c0a15" inactive-color="#d8d1cf" />
        </div>
        <div v-if="groupBuyMode" class="mt-5 space-y-4">
          <div class="rounded-[26px] bg-[#8c0a15] p-4 text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.22)]">
            <div class="flex items-start justify-between gap-4">
              <div>
                <p class="text-xs font-bold text-white/70">合买模式已开启</p>
              </div>
              <span class="shrink-0 rounded-full bg-white/15 px-3 py-1 text-xs font-extrabold text-white">固定每份 ¥{{ groupBuyFixedShareAmount }}</span>
            </div>
            <div class="mt-4 grid grid-cols-3 gap-2">
              <div class="rounded-2xl bg-white/12 p-3">
                <span class="block text-[11px] font-bold text-white/65">方案总额</span>
                <strong class="mt-1 block font-headline text-lg font-extrabold">¥{{ groupBuyTotalAmountText }}</strong>
              </div>
              <div class="rounded-2xl bg-white/12 p-3">
                <span class="block text-[11px] font-bold text-white/65">可分份数</span>
                <strong class="mt-1 block font-headline text-lg font-extrabold">{{ groupBuyDerivedShareCount || 0 }}份</strong>
              </div>
              <div class="rounded-2xl bg-white/12 p-3">
                <span class="block text-[11px] font-bold text-white/65">固定每份</span>
                <strong class="mt-1 block font-headline text-lg font-extrabold">¥{{ groupBuyFixedShareAmount }}</strong>
              </div>
            </div>
          </div>
          <div v-if="groupBuyDerivedShareCount <= 0" class="rounded-2xl bg-[#fff4dc] px-4 py-3 text-xs font-bold text-[#8c0a15]">总金额必须能按每份金额整除</div>
        </div>
      </section>

      <div v-if="loading" class="text-center text-sm text-[#5a403e]">加载中...</div>
    </main>

    <UnifiedBetBottomBar
      :selected-count="engine.cartTotalCount.value + engine.draftBetCount.value"
      :total-amount="engine.cartTotalAmount.value + engine.draftAmount.value"
      :can-submit="canSubmitCurrentOrder && !submittingOrder"
      :submitting="submittingOrder"
      :submit-text="submitButtonDisplayText"
      :group-buy-mode="groupBuyMode"
      v-model:group-buy-self-shares="groupBuySelfShares"
      :group-buy-share-count="groupBuySafeShareCount"
      :group-buy-self-shares-hint="groupBuySelfSharesHint"
      :group-buy-payment-amount="groupBuyPaymentAmount"
      @group-buy-self-shares-input="touchGroupBuySelfShares"
      @group-buy-self-shares-blur="clampGroupBuySelfShares"
      @submit="submitCart"
    />

    <div v-if="submittingOrder" class="bet-submit-loading" role="status" aria-live="assertive">
      <div class="bet-submit-loading__panel">
        <van-loading color="#8c0a15" size="30px" vertical>{{ submitLoadingText }}</van-loading>
        <p>请稍候，正在同步订单和余额</p>
      </div>
    </div>

    <van-popup v-model:show="showPlayPopup" position="bottom" round class="play-select-popup">
      <section class="play-select-sheet">
        <header class="play-select-sheet__header">
          <div>
            <p>当前彩种</p>
            <h2>{{ config?.lottery.name || lotteryCode }}</h2>
          </div>
          <button type="button" aria-label="关闭玩法选择" @click="showPlayPopup = false">×</button>
        </header>
        <DynamicPlayTabs :plays="config?.plays || []" :selected-code="selectedPlayCode" @select="selectPlay" />
      </section>
    </van-popup>
  </MobilePageShell>
</template>

<style scoped>
.bet-page-main {
  padding-bottom: calc(9rem + env(safe-area-inset-bottom));
}

.bet-page-main--group-buy {
  padding-bottom: calc(10.75rem + env(safe-area-inset-bottom));
}

.play-select-popup {
  overflow: hidden;
  background: transparent;
}

.play-select-sheet {
  max-height: min(64dvh, 480px);
  overflow-y: auto;
  border-radius: 24px 24px 0 0;
  padding: 18px 16px max(18px, env(safe-area-inset-bottom));
  background: #f9f9f9;
}

.play-select-sheet__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 14px;
}

.play-select-sheet__header p {
  margin: 0 0 4px;
  color: #5a403e;
  font-size: 12px;
  font-weight: 800;
}

.play-select-sheet__header h2 {
  margin: 0;
  color: #1a1c1c;
  font-size: 18px;
  font-weight: 900;
}

.play-select-sheet__header button {
  display: inline-flex;
  width: 32px;
  height: 32px;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 999px;
  color: #5a403e;
  background: #eeeeee;
  font-size: 22px;
  line-height: 1;
}

.bet-submit-loading {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(249, 249, 249, 0.72);
  backdrop-filter: blur(8px);
}

.bet-submit-loading__panel {
  width: min(72vw, 240px);
  border: 1px solid rgba(140, 10, 21, 0.1);
  border-radius: 24px;
  padding: 24px 18px 20px;
  background: rgba(255, 255, 255, 0.96);
  box-shadow: 0 24px 64px rgba(140, 10, 21, 0.14);
  text-align: center;
}

.bet-submit-loading__panel :deep(.van-loading__text) {
  margin-top: 12px;
  color: #1a1c1c;
  font-size: 15px;
  font-weight: 900;
}

.bet-submit-loading__panel p {
  margin: 10px 0 0;
  color: #8e706d;
  font-size: 12px;
  font-weight: 800;
}
</style>
