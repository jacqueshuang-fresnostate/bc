<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { LotteryCard } from '../../api/lottery'
import CountdownBadge from './CountdownBadge.vue'

const props = defineProps<{
  lottery: LotteryCard
  variant: 'featured' | 'secondary' | 'classic' | 'regional'
  countdownText: (lottery?: LotteryCard) => string
  roundDigits: (lottery?: LotteryCard, fallbackCount?: number) => string[]
  statusText?: (status?: string) => string
}>()

const emit = defineEmits<{ open: [LotteryCard] }>()
const logoLoadFailed = ref(false)

const logoUrl = computed(() => String(props.lottery.logoUrl || '').trim())
const showImage = computed(() => Boolean(logoUrl.value) && !logoLoadFailed.value)

watch(logoUrl, () => {
  logoLoadFailed.value = false
})

const resultDigitCount = computed(() => {
  // 首页所有卡片统一按真实开奖结果长度和后端号码类型配置决定展示位数，兼容 3 位和 5 位彩种。
  const configuredCount = Number(props.lottery.resultCount || 0)
  const latestResultCount = props.lottery.latestResult?.length || 0
  const count = Math.max(configuredCount, latestResultCount, 3)
  return Number.isFinite(count) ? count : 3
})

const displayDigits = computed(() => props.roundDigits(props.lottery, resultDigitCount.value))

const digitSum = computed(() => {
  const values = displayDigits.value
    .filter(digit => /^\d+$/.test(digit))
    .map(digit => Number(digit))
  return values.length ? values.reduce((sum, value) => sum + value, 0) : '-'
})

const countdownDisplay = computed(() => {
  // 倒计时文案由首页组合函数统一计算，这里只负责拆成适合卡片展示的短标签和值。
  const rawText = String(props.countdownText(props.lottery) || '--:--').trim()
  const sealedMatch = rawText.match(/^封盘\s+(.+)$/)
  if (sealedMatch) return { label: '封盘', value: sealedMatch[1] }
  if (/^\d{2}:\d{2}(:\d{2})?$/.test(rawText)) return { label: '计时', value: rawText }
  return { label: '状态', value: rawText || '--:--' }
})

const issueText = computed(() => props.lottery.issue ? `第 ${props.lottery.issue} 期` : '暂无期号')

function statusLabel() {
  switch (props.lottery.status) {
    case 'selling':
      return '可下注'
    case 'sealed':
      return '封盘'
    case 'drawn':
      return '已开奖'
    case 'waiting':
      return '待开奖'
    case 'closed':
      return '已关闭'
    default:
      return '待更新'
  }
}

function statusClass() {
  if (props.lottery.status === 'selling') return 'bg-green-50 text-green-700 ring-green-100'
  if (props.lottery.status === 'sealed') return 'bg-orange-50 text-orange-700 ring-orange-100'
  if (props.lottery.status === 'drawn') return 'bg-red-50 text-primary ring-red-100'
  return 'bg-stone-100 text-on-surface-variant ring-stone-200'
}
</script>

<template>
  <div v-if="variant === 'featured'" class="featured-lottery-card col-span-2 flex flex-col gap-3 overflow-hidden rounded-xl border border-primary/10 bg-surface-container-lowest p-4 shadow-sm shadow-red-900/5">
    <div class="flex items-start justify-between gap-3">
      <div class="flex min-w-0 items-center gap-3">
        <img v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-12 w-12 flex-shrink-0 rounded-xl object-cover shadow-sm" @error="logoLoadFailed = true" />
        <div v-else class="lottery-card-icon-fallback flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">★</div>
        <div class="min-w-0">
          <h4 class="truncate font-headline text-lg font-extrabold leading-tight">{{ lottery.name }}</h4>
          <div class="mt-1 flex flex-wrap items-center gap-1.5">
            <span class="whitespace-nowrap text-[11px] font-medium text-on-surface-variant">{{ issueText }}</span>
            <span :class="['lottery-state-pill rounded-full px-2 py-0.5 text-[10px] font-bold ring-1', statusClass()]">{{ statusLabel() }}</span>
          </div>
        </div>
      </div>
      <CountdownBadge class="featured-countdown shrink-0" :label="countdownDisplay.label" :text="countdownDisplay.value" />
    </div>
    <div class="featured-result-panel rounded-xl bg-primary/5 p-3">
      <div class="flex items-center justify-between gap-2">
        <span class="text-[10px] font-bold text-primary/80">最近开奖</span>
        <span class="truncate text-[10px] font-medium text-on-surface-variant">{{ issueText }}</span>
      </div>
      <div class="mt-3 flex items-center gap-2">
        <div class="flex min-w-0 flex-1 flex-wrap gap-2">
          <div v-for="(digit, index) in displayDigits" :key="`${lottery.code}-featured-${index}`" class="home-result-ball home-result-ball--featured lacquer-gradient font-headline text-base font-bold !text-white shadow-sm">{{ digit }}</div>
        </div>
        <div class="sum-pill flex shrink-0 flex-col items-center justify-center rounded-xl bg-white px-3 py-1.5 shadow-sm shadow-red-900/5">
          <span class="text-[10px] font-medium text-on-surface-variant">和值</span>
          <span class="font-headline text-lg font-extrabold leading-none text-primary">{{ digitSum }}</span>
        </div>
      </div>
    </div>
    <button class="w-full rounded-xl lacquer-gradient py-2.5 font-headline text-sm font-bold !text-on-primary shadow-md shadow-red-900/15 active:scale-[0.99]" @click="emit('open', lottery)">立即投注</button>
  </div>

  <button v-else-if="variant === 'secondary'" class="secondary-lottery-card flex min-h-[7.75rem] w-full flex-col justify-between rounded-xl border border-primary/5 bg-surface-container-lowest p-3 text-left shadow-sm shadow-red-900/5 active:scale-[0.99]" @click="emit('open', lottery)">
    <div class="flex items-start justify-between gap-2">
      <div class="flex min-w-0 items-start gap-2">
        <img v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-8 w-8 flex-shrink-0 rounded-xl object-cover" @error="logoLoadFailed = true" />
        <div v-else class="lottery-card-icon-fallback flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-xl bg-primary/10 text-xs text-primary">★</div>
        <div class="min-w-0">
          <span class="block truncate text-sm font-extrabold leading-tight">{{ lottery.name }}</span>
          <span :class="['lottery-state-pill mt-1 inline-flex rounded-full px-2 py-0.5 text-[10px] font-bold ring-1', statusClass()]">{{ statusLabel() }}</span>
        </div>
      </div>
      <span class="secondary-countdown shrink-0 rounded-full bg-red-50 px-2 py-1 text-right text-primary">
        <span class="block text-[9px] leading-none text-primary/70">{{ countdownDisplay.label }}</span>
        <span class="block font-headline text-[11px] font-extrabold leading-tight">{{ countdownDisplay.value }}</span>
      </span>
    </div>
    <div class="mt-3 flex items-center justify-between gap-2">
      <div class="flex min-w-0 flex-1 flex-wrap gap-1.5">
        <div v-for="(digit, index) in displayDigits" :key="`${lottery.code}-secondary-${index}`" class="home-result-ball home-result-ball--secondary border border-outline-variant/30 bg-surface-container-highest text-[9px] font-bold text-on-surface-variant">{{ digit }}</div>
      </div>
      <span class="enter-chip shrink-0 rounded-full bg-primary px-2.5 py-1 text-[10px] font-bold !text-on-primary">进入</span>
    </div>
  </button>

  <button v-else class="flex w-full flex-col gap-2 rounded-xl bg-surface-container-lowest p-3 text-left shadow-sm" @click="emit('open', lottery)">
    <div class="flex items-start gap-2">
      <img v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-9 w-9 flex-shrink-0 rounded-full object-cover" @error="logoLoadFailed = true" />
      <div v-else class="lottery-card-icon-fallback flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full text-xs" :class="variant === 'regional' ? 'bg-tertiary/10 text-tertiary' : 'bg-secondary/10 text-secondary'">{{ variant === 'regional' ? '◇' : '★' }}</div>
      <div class="min-w-0 flex-1">
        <div class="flex items-start justify-between gap-2">
          <h5 class="truncate text-xs font-bold">{{ lottery.name }}</h5>
          <span class="lottery-status-stack inline-flex shrink-0 flex-col items-end gap-1">
            <span :class="['lottery-state-pill rounded-full px-1.5 py-0.5 text-[10px] font-bold ring-1', statusClass()]">{{ statusLabel() }}</span>
          </span>
        </div>
      </div>
    </div>
    <div class="flex flex-wrap gap-1">
      <div v-for="(digit, index) in displayDigits" :key="`${lottery.code}-group-${index}`" class="flex h-5 min-w-5 items-center justify-center rounded-full px-1 text-[9px] font-bold" :class="variant === 'regional' ? 'bg-tertiary/10 text-tertiary' : 'bg-primary/5 text-primary'">{{ digit }}</div>
    </div>
  </button>
</template>

<style scoped>
.home-result-ball {
  display: inline-flex;
  flex: 0 0 var(--home-result-ball-size);
  width: var(--home-result-ball-size);
  height: var(--home-result-ball-size);
  aspect-ratio: 1 / 1;
  align-items: center;
  justify-content: center;
  border-radius: 9999px;
  line-height: 1;
  white-space: nowrap;
}

.home-result-ball--featured {
  --home-result-ball-size: 2.125rem;
}

.home-result-ball--secondary {
  --home-result-ball-size: 1.375rem;
}

.lottery-state-pill,
.featured-countdown,
.secondary-countdown,
.enter-chip {
  white-space: nowrap;
}

@media (max-width: 374px) {
  .home-result-ball--featured {
    --home-result-ball-size: 1.95rem;
  }

  .home-result-ball--secondary {
    --home-result-ball-size: 1.25rem;
  }
}
</style>
