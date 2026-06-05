<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { showNotify } from 'vant'
import { fetchLotteryHomepage } from '../api/lottery'
import type { HomepageBanner, HomepageResponse, LotteryCard } from '../api/lottery'
import { fetchCurrentUserProfile, fetchMobileAdvertisements } from '../api/user'
import type { MobileAdvertisement } from '../api/user'
import HomeDrawCard from '../components/lottery/HomeDrawCard.vue'
import WinningTicker from '../components/lottery/WinningTicker.vue'
import { useHomepageDrawUpdates } from '../composables/useHomepageDrawUpdates'
import type { LotteryDrawMessage } from '../composables/useHomepageDrawUpdates'
import { useBrandingStore } from '../stores/branding'
import { parseChinaDateTime } from '../utils/lotteryFormat'

const props = defineProps<{ wsMessage?: LotteryDrawMessage | null }>()
const router = useRouter()
const brandingStore = useBrandingStore()
const { branding } = storeToRefs(brandingStore)

// 首页数据边界：余额单独加载，首页模块数据整体保存在 homepage，倒计时由 nowMs 驱动。
const balance = ref('0.00')
const homepage = ref<HomepageResponse | null>(null)
const mobileAdvertisements = ref<MobileAdvertisement[]>([])
const loadingHomepage = ref(false)
const nowMs = ref(Date.now())
const activeHeroBannerIndex = ref(0)
const heroBannerImageFailed = ref<Record<string, true>>({})
let countdownTimer: ReturnType<typeof setInterval> | null = null

const lotteriesSetting = computed(() => homepage.value?.settings || {
  bannersEnabled: false,
  tickerEnabled: false,
  featuredEnabled: false,
  groupsEnabled: false,
  statsEnabled: false,
})
const heroBanners = computed<HomepageBanner[]>(() => mobileAdvertisements.value.map(advertisement => ({
  id: advertisement.id,
  title: advertisement.title,
  imageUrl: advertisement.imageUrl,
  linkUrl: advertisement.linkUrl || '',
})))
const showBanner = computed(() => heroBanners.value.length > 0)
const showTicker = computed(() => lotteriesSetting.value.tickerEnabled)
const showGroups = computed(() => lotteriesSetting.value.groupsEnabled)
const showStats = computed(() => lotteriesSetting.value.statsEnabled)
const featuredLotteries = computed(() => homepage.value?.featuredSection?.lotteries || [])
const featuredTitle = computed(() => homepage.value?.featuredSection?.title || '高频极速')
const featuredLottery = computed(() => featuredLotteries.value[0])
const secondaryHighFrequencyLotteries = computed(() => featuredLotteries.value.slice(1, 3))
const visibleGroups = computed(() => showGroups.value ? homepage.value?.groups?.filter(group => group.lotteries?.length) || [] : [])
const hasHomepageLotteries = computed(() => Boolean(featuredLotteries.value.length || visibleGroups.value.length))
const heroBannerSlides = computed<HomepageBanner[]>(() => heroBanners.value)
const heroBanner = computed(() => heroBannerSlides.value[activeHeroBannerIndex.value] || heroBannerSlides.value[0])
const heroBannerIndicators = computed(() => heroBanners.value.length > 1 ? heroBanners.value : [])
const tickerItems = computed(() => {
  // 只有跑马灯开关开启时才展示公告兜底文案，关闭时整个模块隐藏。
  const items = showTicker.value ? homepage.value?.ticker?.items || [] : []
  if (items.length) return items.map(item => item.text || '').filter(Boolean)
  return showTicker.value ? ['暂无中奖公告'] : []
})
const todayWinnerCount = computed(() => (homepage.value?.stats?.todayWinnerCount ?? 0).toLocaleString())
const totalPayoutDisplay = computed(() => homepage.value?.stats?.totalPayoutDisplay || '¥0')

// 开奖更新组合函数只负责把 homepage 中的轮次字段转换为卡片状态、开奖号和倒计时文本。
const { statusText, roundDigits, countdownText, applyDrawResult } = useHomepageDrawUpdates(homepage, nowMs)

function lotteryName(code?: string) {
  // 通知文案从当前已渲染彩种中反查名称，找不到时保留后端推送的 code。
  const allLotteries = [
    ...featuredLotteries.value,
    ...visibleGroups.value.flatMap(group => group.lotteries || []),
  ]
  return allLotteries.find(item => item.code === code)?.name || code || '-'
}

function openLottery(lottery?: LotteryCard) {
  if (!lottery?.code) return
  router.push(`/bet/${lottery.code}`)
}

function openGroupBuy(lottery?: LotteryCard) {
  // 团购入口只对后端标记可团购且有 code 的彩种开放。
  if (!lottery?.groupBuyEnabled || !lottery.code) return
  router.push({ path: '/group-buy', query: { lottery_code: lottery.code } })
}

function openBanner(banner?: HomepageBanner) {
  // Banner 仅处理站内路径，外部链接不在当前移动端路由中跳转。
  if (banner?.linkUrl?.startsWith('/')) router.push(banner.linkUrl)
}

function heroBannerImageUrl(banner?: HomepageBanner) {
  return String(banner?.imageUrl || '').trim()
}

function heroBannerHasImageFor(banner?: HomepageBanner) {
  const imageUrl = heroBannerImageUrl(banner)
  return Boolean(imageUrl) && !heroBannerImageFailed.value[imageUrl]
}

function heroBannerTitle(banner: HomepageBanner) {
  return String(banner.title || '').trim()
}

function heroBannerSubtitleText(banner: HomepageBanner) {
  return String(banner.subtitle || '').trim()
}

function setActiveHeroBanner(index: number) {
  if (!heroBanners.value.length) return
  activeHeroBannerIndex.value = Math.max(0, Math.min(index, heroBanners.value.length - 1))
}

function nextHeroBanner() {
  if (heroBanners.value.length <= 1) return
  activeHeroBannerIndex.value = (activeHeroBannerIndex.value + 1) % heroBanners.value.length
}

function handleHeroBannerImageError(banner = heroBanner.value) {
  const imageUrl = heroBannerImageUrl(banner)
  if (!imageUrl) return
  heroBannerImageFailed.value = { ...heroBannerImageFailed.value, [imageUrl]: true }
}

async function loadHomepage() {
  loadingHomepage.value = true
  try {
    // 首页接口一次返回 banner、跑马灯、推荐区、分组和统计数据。
    const data = await fetchLotteryHomepage()
    homepage.value = data || null
    const serverTime = parseChinaDateTime(data?.serverTime)
    nowMs.value = Number.isFinite(serverTime) ? serverTime : Date.now()
  } catch {
    // 加载失败时置空首页数据，由模板进入“暂无彩种”兜底分支。
    homepage.value = null
  } finally {
    loadingHomepage.value = false
  }
}

async function loadMobileAdvertisements() {
  try {
    mobileAdvertisements.value = await fetchMobileAdvertisements()
  } catch {
    mobileAdvertisements.value = []
  }
}

onMounted(async () => {
  // 本地秒级计时只刷新展示倒计时，开奖结果仍以后端接口和 websocket 推送为准。
  countdownTimer = setInterval(() => {
    nowMs.value += 1000
  }, 1000)
  try {
    const profile = await fetchCurrentUserProfile()
    balance.value = profile.balance
  } catch {}
  await Promise.all([loadHomepage(), loadMobileAdvertisements()])
})

onUnmounted(() => {
  if (countdownTimer) clearInterval(countdownTimer)
})

watch(() => props.wsMessage, (msg) => {
  if (msg?.event === 'draw_result') {
    // WebSocket 开奖推送先局部更新首页轮次展示，再弹出当前彩种开奖结果提示。
    applyDrawResult(msg)
    showNotify({ type: 'success', message: `${lotteryName(msg.lotteryCode || msg.lottery_code)} 第${msg.issue}期：${msg.result}` })
  }
})

watch(heroBanners, (banners) => {
  if (activeHeroBannerIndex.value >= banners.length) activeHeroBannerIndex.value = 0
})
</script>

<template>
  <div class="home-dashboard min-h-screen bg-surface text-on-surface font-body selection:bg-primary/10">
    <header class="fixed top-0 left-0 z-40 flex h-16 w-full items-center justify-between bg-white/80 px-6 shadow-sm shadow-red-900/5 backdrop-blur-md">
      <div class="flex items-center gap-3">
        <img
          :alt="`${branding.site_name} 标志`"
          class="h-8 w-8 rounded-full border border-red-900/10 object-cover shadow-sm"
          :src="branding.logo_url"
          @error="brandingStore['set\u004cogoFallback']()"
        />
        <span class="font-headline text-xl font-bold italic tracking-tighter text-red-900">{{ branding.site_name }}</span>
      </div>
      <div class="flex items-center gap-2 rounded-full bg-stone-50/70 px-4 py-1.5 text-red-800 active:scale-95">
        <span class="text-sm">钱包</span>
        <span class="font-headline text-sm font-semibold tracking-tight">¥{{ balance }}</span>
      </div>
    </header>

    <main class="mx-auto max-w-2xl space-y-6 px-4 pb-28 pt-20">
      <!-- 首页容器只负责品牌、数据加载与区块编排。 -->
      <section
        v-if="showBanner"
        class="hero-banner relative aspect-[21/9] w-full overflow-hidden rounded-xl bg-[radial-gradient(circle_at_18%_10%,#fed65b,transparent_28%),linear-gradient(135deg,#2b0618,#7c0714_52%,#b22a2b)] shadow-sm"
      >
        <div
          class="hero-banner-track flex h-full transition-transform duration-500 ease-out"
          :style="{ transform: `translateX(-${activeHeroBannerIndex * 100}%)` }"
        >
          <button
            v-for="(banner, index) in heroBannerSlides"
            :key="banner.id || banner.imageUrl || index"
            type="button"
            class="hero-banner-slide relative h-full w-full shrink-0 text-left"
            @click="openBanner(banner)"
          >
            <img
              v-if="heroBannerHasImageFor(banner)"
              :src="heroBannerImageUrl(banner)"
              class="hero-banner-media absolute inset-0 h-full w-full object-cover"
              :alt="heroBannerTitle(banner)"
              @error="handleHeroBannerImageError(banner)"
            />
            <div v-else class="hero-banner-fallback absolute inset-0"></div>
            <div class="absolute inset-0 opacity-20" style="background-image: repeating-linear-gradient(135deg, rgba(255,255,255,.35) 0 1px, transparent 1px 12px);"></div>
            <div class="absolute inset-0 flex flex-col justify-center bg-gradient-to-r from-black/65 via-black/28 to-transparent px-6">
              <span v-if="heroBannerSubtitleText(banner)" class="mb-1 font-headline text-xs font-bold uppercase tracking-widest text-secondary-fixed">{{ heroBannerSubtitleText(banner) }}</span>
              <h2 v-if="heroBannerTitle(banner)" class="max-w-[72%] font-headline text-xl font-extrabold leading-tight text-white">{{ heroBannerTitle(banner) }}</h2>
            </div>
          </button>
        </div>
        <button
          v-if="heroBanners.length > 1"
          type="button"
          class="hero-banner-next absolute right-3 top-1/2 flex h-8 w-8 -translate-y-1/2 items-center justify-center rounded-full bg-black/35 text-lg font-bold text-white backdrop-blur-sm active:scale-95"
          aria-label="切换下一张Banner"
          @click.stop="nextHeroBanner"
        >
          ›
        </button>
        <div v-if="heroBannerIndicators.length" class="absolute bottom-3 left-1/2 flex -translate-x-1/2 gap-1.5">
          <button
            v-for="(_, index) in heroBannerIndicators"
            :key="index"
            type="button"
            class="hero-banner-dot h-1 rounded-full transition-all"
            :class="activeHeroBannerIndex === index ? 'w-4 bg-white' : 'w-1.5 bg-white/40'"
            :aria-label="`切换到第${index + 1}张Banner`"
            @click.stop="setActiveHeroBanner(index)"
          ></button>
        </div>
      </section>

      <WinningTicker v-if="showTicker" :ticker-items="tickerItems" />

      <!-- 首页主内容按加载中、无彩种、有彩种三个分支渲染，避免空数据时继续渲染卡片。 -->
      <div v-if="loadingHomepage" class="rounded-xl bg-surface-container-lowest p-8 text-center shadow-sm">
        <van-loading>加载中...</van-loading>
      </div>

      <van-empty v-else-if="!hasHomepageLotteries" description="暂无彩种" />

      <template v-else>
        <!-- 推荐区受后端 settings.featuredEnabled 控制，关闭时不展示高频卡片组。 -->
        <section class="space-y-4" v-if="lotteriesSetting.featuredEnabled">
          <div class="flex items-end justify-between px-1">
            <h3 class="border-l-4 border-primary pl-3 font-headline text-lg font-extrabold tracking-tight">{{ featuredTitle }}</h3>
          </div>
          <div class="grid grid-cols-2 gap-3">
            <HomeDrawCard
              v-if="featuredLottery"
              :lottery="featuredLottery"
              variant="featured"
              :countdown-text="countdownText"
              :round-digits="roundDigits"
              :status-text="statusText"
              @open="openLottery"
              @group-buy="openGroupBuy"
            />

            <HomeDrawCard
              v-for="lottery in secondaryHighFrequencyLotteries"
              :key="lottery.code"
              :lottery="lottery"
              variant="secondary"
              :countdown-text="countdownText"
              :round-digits="roundDigits"
              @open="openLottery"
              @group-buy="openGroupBuy"
            />
          </div>
        </section>

        <!-- 分组区受 settings.groupsEnabled 控制，后端会返回全部销售中彩种的分类分组。 -->
        <section v-for="(group, groupIndex) in visibleGroups" :key="group.code || group.name || groupIndex" class="space-y-4">
          <div class="flex items-end justify-between px-1">
            <h3
              class="border-l-4 pl-3 font-headline text-lg font-extrabold tracking-tight"
              :class="groupIndex % 2 === 0 ? 'border-secondary' : 'border-tertiary'"
            >
              {{ group.name || '彩种分组' }}
            </h3>
          </div>
          <div class="grid grid-cols-2 gap-2">
            <HomeDrawCard
              v-for="lottery in group.lotteries"
              :key="lottery.code"
              :lottery="lottery"
              :variant="groupIndex % 2 === 0 ? 'classic' : 'regional'"
              :countdown-text="countdownText"
              :round-digits="roundDigits"
              @open="openLottery"
              @group-buy="openGroupBuy"
            />
          </div>
        </section>
      </template>

      <!-- 统计卡片受 settings.statsEnabled 控制，关闭时不展示任何兜底数值。 -->
      <section v-if="showStats" class="grid grid-cols-2 gap-3 pb-8">
        <div class="rounded-xl bg-surface-container-high/50 p-4">
          <p class="mb-1 text-[10px] font-medium uppercase tracking-widest text-on-surface-variant">今日中奖人数</p>
          <div class="flex items-baseline gap-1">
            <span class="font-headline text-2xl font-extrabold text-primary">{{ todayWinnerCount }}</span>
            <span class="text-[10px] text-on-surface-variant">人</span>
          </div>
        </div>
        <div class="rounded-xl bg-surface-container-high/50 p-4">
          <p class="mb-1 text-[10px] font-medium uppercase tracking-widest text-on-surface-variant">累计派奖金额</p>
          <span class="font-headline text-xl font-extrabold text-on-surface">{{ totalPayoutDisplay }}</span>
        </div>
      </section>
    </main>
  </div>
</template>
