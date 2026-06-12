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
  const drawMatch = rawText.match(/^开奖\s+(.+)$/)
  if (drawMatch) return { label: '开奖', value: drawMatch[1] }
  const sealedMatch = rawText.match(/^封盘\s+(.+)$/)
  if (sealedMatch) return { label: '封盘', value: sealedMatch[1] }
  if (/^\d{2}:\d{2}(:\d{2})?$/.test(rawText)) return { label: '开奖', value: rawText }
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
        <img v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-9 w-9 flex-shrink-0 rounded-lg object-cover shadow-sm" @error="logoLoadFailed = true" />
        <div v-else class="lottery-card-icon-fallback flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">★</div>
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
        <img v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-6 w-6 flex-shrink-0 rounded-md object-cover" @error="logoLoadFailed = true" />
        <div v-else class="lottery-card-icon-fallback flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-md bg-primary/10 text-xs text-primary">★</div>
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

  <button
    v-else
    :class="[
      'group-lottery-card',
      variant === 'regional' ? 'group-lottery-card--regional' : 'group-lottery-card--classic',
    ]"
    @click="emit('open', lottery)"
  >
    <div class="group-lottery-card__content">
      <div class="group-lottery-card__top">
        <div class="group-lottery-card__copy">
          <h5>{{ lottery.name }}</h5>
          <div class="group-lottery-card__meta">
            <span :class="['lottery-state-pill rounded-full px-1.5 py-0.5 text-[10px] font-bold ring-1', statusClass()]">{{ statusLabel() }}</span>
          </div>
          <span class="group-lottery-card__issue">{{ issueText }}</span>
        </div>
        <div class="group-lottery-card__logo-shell">
          <img
            v-if="showImage"
            :src="logoUrl"
            :alt="`${lottery.name} 标志`"
            class="group-lottery-card__logo"
            @error="logoLoadFailed = true"
          />
          <div v-else class="group-lottery-card__fallback">{{ variant === 'regional' ? '◇' : '★' }}</div>
        </div>
      </div>
      <div class="group-lottery-card__digits" aria-label="最近开奖号码">
        <span
          v-for="(digit, index) in displayDigits"
          :key="`${lottery.code}-group-${index}`"
          class="group-lottery-card__digit"
        >
          {{ digit }}
        </span>
      </div>
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

.group-lottery-card {
  position: relative;
  width: 100%;
  min-height: 5.35rem;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.74);
  border-radius: 1rem;
  padding: 0.75rem;
  text-align: left;
  box-shadow: 0 8px 22px rgba(77, 41, 68, 0.08);
  transition: transform 0.18s ease, box-shadow 0.18s ease, opacity 0.18s ease;
}

.group-lottery-card--classic {
  background:
    radial-gradient(circle at 100% 8%, rgba(255, 204, 89, 0.35), transparent 30%),
    linear-gradient(135deg, #f7f2ff 0%, #f8f3ff 44%, #fff5ef 100%);
}

.group-lottery-card--regional {
  background:
    radial-gradient(circle at 100% 8%, rgba(255, 177, 196, 0.35), transparent 30%),
    linear-gradient(135deg, #eef8ff 0%, #f4f2ff 48%, #fff7ed 100%);
}

.group-lottery-card::after {
  content: '';
  position: absolute;
  right: -1.35rem;
  bottom: -1.75rem;
  width: 5.25rem;
  height: 5.25rem;
  border-radius: 9999px;
  background: rgba(255, 255, 255, 0.48);
  pointer-events: none;
}

.group-lottery-card:active {
  transform: scale(0.985);
  box-shadow: 0 5px 16px rgba(77, 41, 68, 0.1);
}

.group-lottery-card__content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  min-height: 3.85rem;
  gap: 0.5rem;
}

.group-lottery-card__top {
  display: flex;
  min-width: 0;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.55rem;
}

.group-lottery-card__copy {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-direction: column;
  gap: 0.38rem;
}

.group-lottery-card__copy h5 {
  margin: 0;
  overflow: hidden;
  color: #2d2630;
  font-size: 0.92rem;
  font-weight: 900;
  line-height: 1.12;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-lottery-card__meta {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 0.35rem;
}

.group-lottery-card__issue {
  display: block;
  max-width: 100%;
  overflow: hidden;
  color: rgba(73, 61, 68, 0.68);
  font-size: 0.56rem;
  font-weight: 800;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-lottery-card__digits {
  display: flex;
  max-width: 100%;
  flex-wrap: nowrap;
  gap: 0.22rem;
  overflow-x: auto;
  overflow-y: hidden;
  padding-bottom: 1px;
  scrollbar-width: none;
}

.group-lottery-card__digits::-webkit-scrollbar {
  display: none;
}

.group-lottery-card__digit {
  display: inline-flex;
  flex: 0 0 1.12rem;
  width: 1.12rem;
  height: 1.12rem;
  align-items: center;
  justify-content: center;
  border-radius: 9999px;
  background: rgba(255, 255, 255, 0.72);
  color: #8b1d24;
  font-size: 0.56rem;
  font-weight: 900;
  line-height: 1;
  box-shadow: inset 0 0 0 1px rgba(139, 29, 36, 0.08);
}

.group-lottery-card__logo-shell {
  display: flex;
  width: 2.25rem;
  height: 2.25rem;
  flex: 0 0 2.25rem;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 1rem;
  background: rgba(255, 255, 255, 0.68);
  box-shadow: 0 8px 18px rgba(92, 54, 92, 0.12);
}

.group-lottery-card__logo,
.group-lottery-card__fallback {
  width: 100%;
  height: 100%;
  border-radius: inherit;
}

.group-lottery-card__logo {
  object-fit: cover;
}

.group-lottery-card__fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  color: #af2829;
  font-size: 1rem;
  font-weight: 900;
}

@media (max-width: 374px) {
  .home-result-ball--featured {
    --home-result-ball-size: 1.95rem;
  }

  .home-result-ball--secondary {
    --home-result-ball-size: 1.25rem;
  }

  .group-lottery-card {
    min-height: 5rem;
    padding: 0.65rem;
  }

  .group-lottery-card__logo-shell {
    width: 2rem;
    height: 2rem;
    flex-basis: 2rem;
    border-radius: 0.7rem;
  }

  .group-lottery-card__copy h5 {
    font-size: 0.84rem;
  }

  .group-lottery-card__digit {
    flex-basis: 1.02rem;
    width: 1.02rem;
    height: 1.02rem;
    font-size: 0.52rem;
  }
}
</style>
