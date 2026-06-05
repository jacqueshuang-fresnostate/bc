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

const emit = defineEmits<{ open: [LotteryCard]; groupBuy: [LotteryCard] }>()
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
  <div v-if="variant === 'featured'" class="col-span-2 flex flex-col gap-4 rounded-xl border border-outline-variant/10 bg-surface-container-lowest p-5 shadow-sm">
    <div class="flex items-start justify-between">
      <div class="flex items-center gap-3">
        <img v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-12 w-12 flex-shrink-0 rounded-full object-cover" @error="logoLoadFailed = true" />
        <div v-else class="lottery-card-icon-fallback flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">★</div>
        <div>
          <h4 class="font-headline text-base font-bold">{{ lottery.name }}</h4>
          <div class="mt-1 flex flex-wrap items-start gap-1.5">
            <span class="text-[10px] text-on-surface-variant">第 {{ lottery.issue || '-' }} 期</span>
            <span class="lottery-status-stack inline-flex flex-col items-start gap-1">
              <span :class="['lottery-state-pill rounded-full px-1.5 py-0.5 text-[10px] font-bold ring-1', statusClass()]">{{ statusLabel() }}</span>
              <span v-if="lottery.groupBuyEnabled" class="lottery-group-buy-tag rounded-full bg-red-50 px-1.5 py-0.5 text-[10px] font-bold text-primary">合买</span>
            </span>
          </div>
        </div>
      </div>
      <CountdownBadge :text="countdownText(lottery)" />
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <div v-for="(digit, index) in displayDigits" :key="`${lottery.code}-featured-${index}`" class="home-result-ball home-result-ball--featured lacquer-gradient font-headline text-base font-bold !text-white shadow-sm">{{ digit }}</div>
      <div class="flex min-w-8 flex-1 flex-col items-end justify-center">
        <span class="text-[10px] font-medium uppercase text-on-surface-variant">和值</span>
        <span class="font-headline font-bold text-primary">{{ digitSum }}</span>
      </div>
    </div>
    <div class="grid grid-cols-1 gap-2">
      <button class="w-full rounded-full lacquer-gradient py-2.5 font-headline text-sm font-bold !text-on-primary shadow-md" @click="emit('open', lottery)">立即投注</button>
    </div>
  </div>

  <div v-else-if="variant === 'secondary'" class="flex flex-col gap-3 rounded-xl bg-surface-container-low p-4">
    <div class="flex items-center justify-between gap-3">
      <div class="flex min-w-0 items-center gap-2">
        <img v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-8 w-8 flex-shrink-0 rounded-full object-cover" @error="logoLoadFailed = true" />
        <div v-else class="lottery-card-icon-fallback flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs text-primary">★</div>
        <span class="truncate text-sm font-bold">{{ lottery.name }}</span>
        <span class="lottery-status-stack inline-flex flex-col items-start gap-1">
          <span :class="['lottery-state-pill rounded-full px-1.5 py-0.5 text-[10px] font-bold ring-1', statusClass()]">{{ statusLabel() }}</span>
          <span v-if="lottery.groupBuyEnabled" class="lottery-group-buy-tag rounded-full bg-red-50 px-1.5 py-0.5 text-[10px] font-bold text-primary">合买</span>
        </span>
      </div>
      <span class="shrink-0 text-[10px] font-bold text-primary">{{ countdownText(lottery) }}</span>
    </div>
    <div class="flex flex-wrap justify-center gap-1.5">
      <div v-for="(digit, index) in displayDigits" :key="`${lottery.code}-secondary-${index}`" class="home-result-ball home-result-ball--secondary border border-outline-variant/30 bg-surface-container-highest text-[9px] font-bold text-on-surface-variant">{{ digit }}</div>
    </div>
    <button class="w-full rounded-full border border-primary/10 bg-white py-1.5 text-xs font-bold text-primary shadow-sm" @click="emit('open', lottery)">进入</button>
    <button v-if="lottery.groupBuyEnabled" class="w-full rounded-full border border-primary/10 bg-red-50 py-1.5 text-xs font-bold text-primary" @click="emit('groupBuy', lottery)">合买</button>
  </div>

  <button v-else class="flex w-full flex-col gap-2 rounded-xl bg-surface-container-lowest p-3 text-left shadow-sm" @click="emit('open', lottery)">
    <div class="flex items-start gap-2">
      <img v-if="showImage" :src="logoUrl" :alt="`${lottery.name} 标志`" class="h-9 w-9 flex-shrink-0 rounded-full object-cover" @error="logoLoadFailed = true" />
      <div v-else class="lottery-card-icon-fallback flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full text-xs" :class="variant === 'regional' ? 'bg-tertiary/10 text-tertiary' : 'bg-secondary/10 text-secondary'">{{ variant === 'regional' ? '◇' : '★' }}</div>
      <div class="min-w-0 flex-1">
        <div class="flex items-start justify-between gap-2">
          <h5 class="truncate text-xs font-bold">{{ lottery.name }}</h5>
          <span class="lottery-status-stack inline-flex shrink-0 flex-col items-end gap-1">
            <span :class="['lottery-state-pill rounded-full px-1.5 py-0.5 text-[10px] font-bold ring-1', statusClass()]">{{ statusLabel() }}</span>
            <span v-if="lottery.groupBuyEnabled" class="lottery-group-buy-tag rounded-full bg-red-50 px-1.5 py-0.5 text-[10px] font-bold text-primary" @click.stop="emit('groupBuy', lottery)">合买</span>
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
}

.home-result-ball--featured {
  --home-result-ball-size: 2.25rem;
}

.home-result-ball--secondary {
  --home-result-ball-size: 1.5rem;
}
</style>
