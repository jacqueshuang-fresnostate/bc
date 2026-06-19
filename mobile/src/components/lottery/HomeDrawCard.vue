<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { LotteryCard } from '../../api/lottery'
import CachedRemoteImage from '../mobile/CachedRemoteImage.vue'
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
  <div v-if="variant === 'featured'" class="featured-lottery-card col-span-2 flex flex-col gap-2.5 overflow-hidden rounded-xl border border-primary/10 bg-surface-container-lowest p-3 shadow-sm shadow-red-900/5">
    <div class="flex items-start justify-between gap-2.5">
      <div class="flex min-w-0 items-center gap-2.5">
        <CachedRemoteImage v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-7 w-7 flex-shrink-0 rounded-lg object-cover shadow-sm" @error="logoLoadFailed = true">
          <div class="lottery-card-icon-fallback flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">★</div>
        </CachedRemoteImage>
        <div v-else class="lottery-card-icon-fallback flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">★</div>
        <div class="min-w-0">
          <h4 class="truncate font-headline text-base font-extrabold leading-tight">{{ lottery.name }}</h4>
          <div class="mt-0.5 flex flex-wrap items-center gap-1.5">
            <span class="whitespace-nowrap text-[10px] font-medium text-on-surface-variant">{{ issueText }}</span>
            <span :class="['lottery-state-pill rounded-full px-2 py-0.5 text-[10px] font-bold ring-1', statusClass()]">{{ statusLabel() }}</span>
          </div>
        </div>
      </div>
      <CountdownBadge class="featured-countdown shrink-0" :label="countdownDisplay.label" :text="countdownDisplay.value" />
    </div>
    <div class="featured-result-panel rounded-xl bg-primary/5 p-2.5">
      <div class="flex items-center justify-between gap-2">
        <span class="text-[10px] font-bold text-primary/80">最近开奖</span>
        <span class="truncate text-[10px] font-medium text-on-surface-variant">{{ issueText }}</span>
      </div>
      <div class="mt-2 flex items-center gap-2">
        <div class="flex min-w-0 flex-1 flex-wrap gap-1.5">
          <div v-for="(digit, index) in displayDigits" :key="`${lottery.code}-featured-${index}`" class="home-result-ball home-result-ball--featured lacquer-gradient font-headline text-sm font-bold !text-white shadow-sm">{{ digit }}</div>
        </div>
        <div class="sum-pill flex shrink-0 flex-col items-center justify-center rounded-xl bg-white px-2.5 py-1 shadow-sm shadow-red-900/5">
          <span class="text-[10px] font-medium text-on-surface-variant">和值</span>
          <span class="font-headline text-base font-extrabold leading-none text-primary">{{ digitSum }}</span>
        </div>
      </div>
    </div>
    <button class="w-full rounded-xl lacquer-gradient py-2 font-headline text-sm font-bold !text-on-primary shadow-md shadow-red-900/15 active:scale-[0.99]" @click="emit('open', lottery)">立即投注</button>
  </div>

  <button v-else-if="variant === 'secondary'" class="secondary-lottery-card flex min-h-[6.45rem] w-full flex-col justify-between rounded-xl border border-primary/5 bg-surface-container-lowest p-2.5 text-left shadow-sm shadow-red-900/5 active:scale-[0.99]" @click="emit('open', lottery)">
    <div class="flex items-start justify-between gap-2">
      <div class="flex min-w-0 items-start gap-2">
        <CachedRemoteImage v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-5 w-5 flex-shrink-0 rounded-md object-cover" @error="logoLoadFailed = true">
          <div class="lottery-card-icon-fallback flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-md bg-primary/10 text-xs text-primary">★</div>
        </CachedRemoteImage>
        <div v-else class="lottery-card-icon-fallback flex h-5 w-5 flex-shrink-0 items-center justify-center rounded-md bg-primary/10 text-xs text-primary">★</div>
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
    <div class="mt-2 flex items-center justify-between gap-2">
      <div class="flex min-w-0 flex-1 flex-wrap gap-1">
        <div v-for="(digit, index) in displayDigits" :key="`${lottery.code}-secondary-${index}`" class="home-result-ball home-result-ball--secondary border border-outline-variant/30 bg-surface-container-highest text-[9px] font-bold text-on-surface-variant">{{ digit }}</div>
      </div>
      <span class="enter-chip shrink-0 rounded-full bg-primary px-2 py-0.5 text-[10px] font-bold !text-on-primary">进入</span>
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
      <div class="group-lottery-card__copy">
        <div class="group-lottery-card__title-row">
          <h5>{{ lottery.name }}</h5>
        </div>
        <span class="group-lottery-card__issue">{{ issueText }}</span>
      </div>
      <div class="group-lottery-card__logo-shell">
        <CachedRemoteImage
          v-if="showImage"
          :src="logoUrl"
          :alt="`${lottery.name} 标志`"
          class="group-lottery-card__logo"
          @error="logoLoadFailed = true"
        >
          <div class="group-lottery-card__fallback">{{ variant === 'regional' ? '◇' : '★' }}</div>
        </CachedRemoteImage>
        <div v-else class="group-lottery-card__fallback">{{ variant === 'regional' ? '◇' : '★' }}</div>
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
  --home-result-ball-size: 1.62rem;
}

.home-result-ball--secondary {
  --home-result-ball-size: 1.02rem;
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
  min-height: 4.55rem;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.86);
  border-radius: 0.85rem;
  padding: 0.28rem 0.58rem;
  text-align: left;
  box-shadow: 0 8px 20px rgba(123, 82, 156, 0.12);
  transition: transform 0.18s ease, box-shadow 0.18s ease, opacity 0.18s ease;
}

.group-lottery-card--classic,
.group-lottery-card--regional {
  background:
    radial-gradient(circle at 92% 5%, rgba(255, 255, 255, 0.78), transparent 30%),
    linear-gradient(135deg, #c8f5ff 0%, #d7c8ff 48%, #ffc4d7 100%);
}

.group-lottery-card::after {
  content: '';
  position: absolute;
  right: -1.55rem;
  bottom: -1.95rem;
  width: 4.75rem;
  height: 4.75rem;
  border-radius: 9999px;
  background: rgba(255, 255, 255, 0.36);
  pointer-events: none;
}

.group-lottery-card:active {
  transform: scale(0.985);
  box-shadow: 0 5px 16px rgba(77, 41, 68, 0.1);
}

.group-lottery-card__content {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  grid-template-areas:
    'copy logo'
    'digits logo';
  min-height: 3.35rem;
  column-gap: 0.48rem;
  row-gap: 0.32rem;
}

.group-lottery-card__copy {
  grid-area: copy;
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 0.18rem;
}

.group-lottery-card__title-row {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 0.28rem;
}

.group-lottery-card__copy h5 {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  flex: 1;
  color: #2d2630;
  font-size: 0.96rem;
  font-weight: 900;
  line-height: 1.05;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-lottery-card__issue {
  display: block;
  max-width: 92%;
  overflow: hidden;
  color: #0d0d0dd1;
  font-size: 0.75rem;
  font-weight: 800;
  line-height: 1.15;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-lottery-card__digits {
  grid-area: digits;
  display: flex;
  max-width: 100%;
  flex-wrap: nowrap;
  gap: 0.18rem;
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
  flex: 0 0 1rem;
  width: 1rem;
  height: 1rem;
  align-items: center;
  justify-content: center;
  border-radius: 9999px;
  background: linear-gradient(180deg, #fff9da 0%, #ffd35e 100%);
  color: #8c0a15;
  font-size: 0.76rem;
  font-weight: 900;
  line-height: 1;
  box-shadow:
    0 3px 8px rgba(140, 10, 21, 0.14),
    inset 0 0 0 1px rgba(255, 255, 255, 0.72);
}

.group-lottery-card__logo-shell {
  grid-area: logo;
  display: flex;
  width: 2.82rem;
  height: 2.82rem;
  align-self: end;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  border-radius: 0.75rem;
  border: 1px solid rgba(255, 255, 255, 0.78);
  background: rgba(255, 255, 255, 0.82);
  box-shadow:
    0 8px 18px rgba(92, 54, 92, 0.18),
    inset 0 1px 0 rgba(255, 255, 255, 0.86);
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
  font-size: 0.85rem;
  font-weight: 900;
}

@media (max-width: 374px) {
  .home-result-ball--featured {
    --home-result-ball-size: 1.52rem;
  }

  .home-result-ball--secondary {
    --home-result-ball-size: 0.96rem;
  }

  .group-lottery-card {
    min-height: 4.3rem;
    padding: 0.28rem 0.58rem;
  }

  .group-lottery-card__logo-shell {
    width: 2.68rem;
    height: 2.68rem;
    flex-basis: 2.68rem;
    border-radius: 0.62rem;
  }

  .group-lottery-card__copy h5 {
    font-size: 0.96rem;
  }

  .group-lottery-card__digit {
    flex-basis: 0.94rem;
    width: 0.94rem;
    height: 0.94rem;
    font-size: 0.72rem;
  }
}
</style>
