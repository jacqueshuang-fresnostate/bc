<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { fetchLotteryGroups } from '../api/lottery'
import { fetchCurrentUserProfile } from '../api/user'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { useBrandingStore } from '../stores/branding'

type LotteryItem = {
  code: string
  name: string
  logo_url?: string | null
  category?: string | null
  draw_interval?: number | null
  daily_draw_time?: string | null
  group_sort_order?: number | null
  is_recommended?: boolean
}

type LotteryGroup = {
  code: string
  name: string
  lotteries?: LotteryItem[]
}

const router = useRouter()
const brandingStore = useBrandingStore()
const { branding } = storeToRefs(brandingStore)
const balance = ref('0.00')
const lotteryGroups = ref<LotteryGroup[]>([])
const searchKeyword = ref('')
const loadingGroups = ref(false)
const groupsRequestSeq = ref(0)
const failedLogoCodes = ref<Record<string, true>>({})

const allLotteries = computed(() => {
  const items = lotteryGroups.value.flatMap(group => group.lotteries || [])
  return items.filter(lottery => lottery.code && lottery.name)
})

const searchedLotteries = computed(() => {
  const keyword = searchKeyword.value.trim().toLowerCase()
  if (!keyword) return allLotteries.value
  return allLotteries.value.filter(lottery => {
    return lottery.name.toLowerCase().includes(keyword) || lottery.code.toLowerCase().includes(keyword)
  })
})

const popularLotteries = computed(() => searchedLotteries.value.filter(lottery => lottery.is_recommended).slice(0, 2))
const speedLotteries = computed(() => searchedLotteries.value.filter(lottery => isSpeedLottery(lottery)).slice(0, 8))
const classicLotteries = computed(() => searchedLotteries.value.filter(lottery => !isSpeedLottery(lottery)).slice(0, 6))
const filteredGroups = computed(() => {
  const allowedCodes = new Set(searchedLotteries.value.map(lottery => lottery.code))
  return lotteryGroups.value
    .map(group => ({
      ...group,
      lotteries: (group.lotteries || []).filter(lottery => allowedCodes.has(lottery.code)),
    }))
    .filter(group => group.lotteries.length)
})
const hasLotteries = computed(() => searchedLotteries.value.length > 0)

function isSpeedLottery(lottery: LotteryItem) {
  const interval = Number(lottery.draw_interval || 0)
  return interval > 0 && interval <= 300
}

function iconForLottery(lottery: LotteryItem) {
  if (lottery.is_recommended) return 'diamond'
  if (isSpeedLottery(lottery)) return 'speed'
  return 'account_balance'
}

function logoUrl(lottery: LotteryItem) {
  return String(lottery.logo_url || '').trim()
}

function showLotteryLogo(lottery: LotteryItem) {
  return Boolean(logoUrl(lottery)) && !failedLogoCodes.value[lottery.code]
}

function markLogoFailed(lottery: LotteryItem) {
  if (!lottery.code) return
  failedLogoCodes.value = { ...failedLogoCodes.value, [lottery.code]: true }
}

function scheduleText(lottery: LotteryItem) {
  if (isSpeedLottery(lottery)) return lottery.draw_interval ? `${Math.round(lottery.draw_interval / 60)}分钟开奖` : '高频开奖'
  return lottery.daily_draw_time ? `每日 ${lottery.daily_draw_time}` : '官方开奖'
}

function statusText(lottery: LotteryItem) {
  if (isSpeedLottery(lottery)) return '开奖中'
  return lottery.daily_draw_time ? `今日 ${lottery.daily_draw_time}` : '去投注'
}

function openLottery(lottery: LotteryItem) {
  if (!lottery.code) return
  router.push(`/bet/${lottery.code}`)
}

async function loadBalance() {
  try {
    const profile = await fetchCurrentUserProfile()
    balance.value = profile.balance
  } catch {}
}

async function loadLotteryGroups() {
  const requestId = ++groupsRequestSeq.value
  loadingGroups.value = true
  try {
    const groups = await fetchLotteryGroups()
    if (requestId !== groupsRequestSeq.value) return
    lotteryGroups.value = Array.isArray(groups) ? groups : []
  } catch {
    if (requestId !== groupsRequestSeq.value) return
    lotteryGroups.value = []
  } finally {
    if (requestId === groupsRequestSeq.value) loadingGroups.value = false
  }
}

onMounted(() => {
  loadBalance()
  loadLotteryGroups()
})
</script>

<template>
  <div class="all-lottery-page min-h-screen bg-surface pb-28 text-on-surface font-body">
    <header class="mobile-safe-header fixed top-0 left-0 z-40 flex h-16 w-full items-center justify-between bg-white/80 px-6 shadow-sm shadow-red-900/5 backdrop-blur-md">
      <div class="flex items-center gap-3">
        <img
          :alt="`${branding.site_name} 标志`"
          class="h-8 w-8 rounded-full border border-red-900/10 object-cover shadow-sm"
          :src="branding.logo_url"
          @error="brandingStore['setLogoFallback']()"
        />
        <span class="font-headline text-xl font-bold italic tracking-tighter text-red-900">{{ branding.site_name }}</span>
      </div>
      <div class="flex items-center gap-2 rounded-full bg-stone-50/70 px-4 py-1.5 text-red-800 active:scale-95">
        <span class="text-sm">钱包</span>
        <span class="font-headline text-sm font-semibold tracking-tight">¥{{ balance }}</span>
      </div>
    </header>

    <main class="mobile-safe-main-top mx-auto w-full max-w-2xl space-y-6 px-4 pb-28">
      <section class="space-y-2">
        <h2 class="font-headline text-3xl font-extrabold tracking-tight text-primary">全部彩种</h2>
        <p class="text-sm all-lottery-muted">探索全网最全的高频与经典彩票</p>
      </section>

      <section class="rounded-full bg-surface-container-lowest px-4 py-3 shadow-sm transition focus-within:shadow-md focus-within:ring-2 focus-within:ring-primary/10">
        <div class="flex items-center gap-3">
          <LucideIcon name="search" class="h-5 w-5 text-primary" />
          <input v-model="searchKeyword" class="w-full border-0 bg-transparent text-sm text-on-surface placeholder:all-lottery-muted/50 focus:outline-none focus:ring-0" placeholder="搜索彩种名称..." type="search" />
        </div>
      </section>

      <div v-if="loadingGroups" class="rounded-2xl bg-surface-container-lowest p-8 text-center shadow-sm">
        <van-loading>加载中...</van-loading>
      </div>

      <van-empty v-else-if="!hasLotteries" description="暂无彩种" />

      <template v-else>
        <section v-if="popularLotteries.length" class="space-y-4">
          <div class="flex items-center gap-3 rounded-2xl bg-surface-container-low px-4 py-3">
            <span class="h-6 w-1.5 rounded-full bg-primary"></span>
            <LucideIcon name="local_fire_department" class="h-5 w-5 text-primary" />
            <h3 class="font-headline text-xl font-bold tracking-wide">热门推荐</h3>
          </div>
          <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <article
              v-for="lottery in popularLotteries"
              :key="`popular-${lottery.code}`"
              class="relative overflow-hidden rounded-2xl bg-surface-container-lowest p-5 shadow-[0_4px_20px_rgba(140,10,21,0.04)]"
              @click="openLottery(lottery)"
            >
              <div class="absolute right-0 top-0 h-28 w-28 rounded-bl-full bg-gradient-to-bl from-primary/5 to-transparent"></div>
              <div class="relative z-10 flex items-start justify-between gap-4">
                <div class="flex min-w-0 gap-4">
                  <div class="flex h-14 w-14 flex-shrink-0 items-center justify-center overflow-hidden rounded-xl bg-gradient-to-br from-primary to-primary-container text-on-primary shadow-lg shadow-primary/20">
                    <img v-if="showLotteryLogo(lottery)" :src="logoUrl(lottery)" :alt="`${lottery.name} 标志`" class="h-full w-full object-cover" @error="markLogoFailed(lottery)" />
                    <LucideIcon v-else :name="iconForLottery(lottery)" class="h-6 w-6" />
                  </div>
                  <div class="min-w-0 pt-1">
                    <h4 class="truncate font-headline text-lg font-bold">{{ lottery.name }}</h4>
                    <p class="mt-1 text-xs all-lottery-muted">{{ scheduleText(lottery) }}</p>
                  </div>
                </div>
                <span class="rounded-full bg-primary/5 px-2.5 py-0.5 text-[10px] font-bold tracking-wider text-primary">HOT</span>
              </div>
              <div class="relative z-10 mt-6 flex items-center justify-between rounded-xl bg-surface-container-low px-3 py-3">
                <span class="text-sm font-bold text-secondary">{{ statusText(lottery) }}</span>
                <button class="rounded-full bg-white px-5 py-2 text-xs font-bold text-primary shadow-sm" type="button">去投注</button>
              </div>
            </article>
          </div>
        </section>

        <section v-if="speedLotteries.length" class="space-y-4">
          <div class="flex items-center gap-3 rounded-2xl bg-surface-container-low px-4 py-3">
            <span class="h-6 w-1.5 rounded-full bg-secondary"></span>
            <LucideIcon name="bolt" class="h-5 w-5 text-secondary" />
            <h3 class="font-headline text-xl font-bold tracking-wide">高频极速</h3>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <button
              v-for="lottery in speedLotteries"
              :key="`speed-${lottery.code}`"
              class="rounded-2xl bg-surface-container-lowest p-4 text-center shadow-[0_4px_20px_rgba(140,10,21,0.04)] active:scale-95"
              type="button"
              @click="openLottery(lottery)"
            >
              <span class="mx-auto mb-3 flex h-12 w-12 items-center justify-center overflow-hidden rounded-full bg-surface-container-low text-primary">
                <img v-if="showLotteryLogo(lottery)" :src="logoUrl(lottery)" :alt="`${lottery.name} 标志`" class="h-full w-full object-cover" @error="markLogoFailed(lottery)" />
                <LucideIcon v-else :name="iconForLottery(lottery)" class="h-6 w-6" />
              </span>
              <span class="block truncate font-headline text-sm font-bold">{{ lottery.name }}</span>
              <span class="mt-2 inline-flex rounded-full bg-secondary/10 px-3 py-1 text-[11px] font-bold text-secondary">{{ statusText(lottery) }}</span>
            </button>
          </div>
        </section>

        <section v-if="classicLotteries.length" class="space-y-4">
          <div class="flex items-center gap-3 rounded-2xl bg-surface-container-low px-4 py-3">
            <span class="h-6 w-1.5 rounded-full bg-on-surface-variant"></span>
            <LucideIcon name="account_balance" class="h-5 w-5 all-lottery-muted" />
            <h3 class="font-headline text-xl font-bold tracking-wide">经典数字</h3>
          </div>
          <div class="space-y-3">
            <button
              v-for="lottery in classicLotteries"
              :key="`classic-${lottery.code}`"
              class="flex w-full items-center justify-between gap-4 rounded-2xl bg-surface-container-lowest p-5 text-left shadow-[0_4px_20px_rgba(140,10,21,0.04)] active:scale-[0.98]"
              type="button"
              @click="openLottery(lottery)"
            >
              <span class="flex min-w-0 items-center gap-4">
                <span class="flex h-12 w-12 flex-shrink-0 items-center justify-center overflow-hidden rounded-full bg-primary/5 text-primary">
                  <img v-if="showLotteryLogo(lottery)" :src="logoUrl(lottery)" :alt="`${lottery.name} 标志`" class="h-full w-full object-cover" @error="markLogoFailed(lottery)" />
                  <LucideIcon v-else :name="iconForLottery(lottery)" class="h-5 w-5" />
                </span>
                <span class="min-w-0">
                  <span class="block truncate font-headline text-base font-bold">{{ lottery.name }}</span>
                  <span class="mt-1 block text-[11px] all-lottery-muted">{{ scheduleText(lottery) }}</span>
                </span>
              </span>
              <span class="text-right">
                <span class="block text-sm font-bold text-secondary">{{ statusText(lottery) }}</span>
              </span>
            </button>
          </div>
        </section>

        <section v-if="filteredGroups.length" class="space-y-4">
          <div class="flex items-center gap-3 rounded-2xl bg-surface-container-low px-4 py-3">
            <span class="h-6 w-1.5 rounded-full bg-primary-fixed-dim"></span>
            <LucideIcon name="grid_view" class="h-5 w-5 text-primary" />
            <h3 class="font-headline text-xl font-bold tracking-wide">全部分类</h3>
          </div>
          <div class="space-y-4">
            <div v-for="group in filteredGroups" :key="group.code" class="rounded-2xl bg-surface-container-low p-4">
              <h4 class="mb-3 font-headline text-sm font-bold all-lottery-muted">{{ group.name }}</h4>
              <div class="grid grid-cols-2 gap-2">
                <button
                  v-for="lottery in group.lotteries"
                  :key="`${group.code}-${lottery.code}`"
                  class="flex items-center gap-3 rounded-xl bg-surface-container-lowest px-3 py-3 text-left text-sm font-bold shadow-sm active:scale-95"
                  type="button"
                  @click="openLottery(lottery)"
                >
                  <span class="flex h-9 w-9 flex-shrink-0 items-center justify-center overflow-hidden rounded-full bg-primary/5 text-primary">
                    <img v-if="showLotteryLogo(lottery)" :src="logoUrl(lottery)" :alt="`${lottery.name} 标志`" class="h-full w-full object-cover" @error="markLogoFailed(lottery)" />
                    <LucideIcon v-else :name="iconForLottery(lottery)" class="h-4 w-4" />
                  </span>
                  <span class="min-w-0">
                    <span class="block truncate">{{ lottery.name }}</span>
                  </span>
                </button>
              </div>
            </div>
          </div>
        </section>
      </template>
    </main>
  </div>
</template>

<style scoped>
.all-lottery-muted {
  color: #5a403e;
}

.placeholder\:all-lottery-muted\/50::placeholder {
  color: rgb(90 64 62 / 50%);
}
</style>
