<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { showNotify } from 'vant'
import type { HomepageBanner, HomepageGroup, LotteryCard } from '../api/lottery'
import HomeDrawCard from '../components/lottery/HomeDrawCard.vue'
import WinningTicker from '../components/lottery/WinningTicker.vue'
import CachedRemoteImage from '../components/mobile/CachedRemoteImage.vue'
import WalletHeaderAmount from '../components/mobile/WalletHeaderAmount.vue'
import { useHomepageDrawUpdates } from '../composables/useHomepageDrawUpdates'
import type { LotteryDrawMessage } from '../composables/useHomepageDrawUpdates'
import { useBrandingStore } from '../stores/branding'
import { useHomepageStore } from '../stores/homepage'
import { useMobileUserDataStore } from '../stores/mobileUserData'
import { parseChinaDateTime } from '../utils/lotteryFormat'

const props = defineProps<{ wsMessage?: LotteryDrawMessage | null }>()
const router = useRouter()
const brandingStore = useBrandingStore()
const homepageStore = useHomepageStore()
const userDataStore = useMobileUserDataStore()
const { branding } = storeToRefs(brandingStore)
const { homepage, mobileAdvertisements, loadingHomepage } = storeToRefs(homepageStore)
const { profile } = storeToRefs(userDataStore)

// 首页数据边界：余额和首页模块数据由 homepage store 缓存，倒计时由 nowMs 驱动。
const nowMs = ref(Date.now())
const activeHeroBannerIndex = ref(0)
const activeGroupTab = ref('')
const heroBannerImageFailed = ref<Record<string, true>>({})
let countdownTimer: ReturnType<typeof setInterval> | null = null
let homepageRefreshInFlight = false
let lastSilentHomepageRefreshMs = 0

const lotteriesSetting = computed(() => homepage.value?.settings || {
  bannersEnabled: false,
  tickerEnabled: false,
  featuredEnabled: false,
  groupsEnabled: false,
  statsEnabled: false,
})
const balance = computed(() => profile.value?.balance || '0.00')
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
const showFeatured = computed(() => lotteriesSetting.value.featuredEnabled && featuredLotteries.value.length > 0)
const featuredLottery = computed(() => featuredLotteries.value[0])
const secondaryHighFrequencyLotteries = computed(() => featuredLotteries.value.slice(1))
const visibleGroups = computed(() => showGroups.value ? homepage.value?.groups?.filter(group => group.lotteries?.length) || [] : [])
const hasHomepageLotteries = computed(() => Boolean(showFeatured.value || visibleGroups.value.length))
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
const { statusText, roundDigits, countdownText, applyDrawResult, applyIssueUpdate } = useHomepageDrawUpdates(homepage, nowMs)

function lotteryName(code?: string) {
  // 通知文案从当前已渲染彩种中反查名称，找不到时保留后端推送的 code。
  const allLotteries = [
    ...featuredLotteries.value,
    ...visibleGroups.value.flatMap(group => group.lotteries || []),
  ]
  return allLotteries.find(item => item.code === code)?.name || code || '-'
}

function groupTabKey(group: HomepageGroup, index: number) {
  return String(group.code || group.name || `group-${index}`)
}

function openLottery(lottery?: LotteryCard) {
  if (!lottery?.code) return
  router.push(`/bet/${lottery.code}`)
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

function homepageLotteries() {
  return [
    ...featuredLotteries.value,
    ...visibleGroups.value.flatMap(group => group.lotteries || []),
  ]
}

function needsHomepageRefresh(lottery: LotteryCard) {
  if (lottery.status === 'drawn' || lottery.status === 'closed') return false
  const drawTime = parseChinaDateTime(lottery.nextDrawTime)
  return Number.isFinite(drawTime) && drawTime + 1000 <= nowMs.value
}

async function refreshHomepageAfterDrawTime() {
  const currentTime = Date.now()
  if (
    homepageRefreshInFlight
    || currentTime - lastSilentHomepageRefreshMs < 5000
    || !homepageLotteries().some(needsHomepageRefresh)
  ) return

  homepageRefreshInFlight = true
  lastSilentHomepageRefreshMs = currentTime
  try {
    await loadHomepage({ silent: true, force: true })
  } finally {
    homepageRefreshInFlight = false
  }
}

async function loadHomepage(options: { silent?: boolean; force?: boolean } = {}) {
  // 首页接口一次返回 banner、跑马灯、推荐区、分组和统计数据；store 负责缓存和请求去重。
  const result = await homepageStore.loadHomepage(options)
  if (result.refreshed) {
    const serverTime = parseChinaDateTime(result.data?.serverTime)
    nowMs.value = Number.isFinite(serverTime) ? serverTime : Date.now()
  }
}

async function loadMobileAdvertisements(options: { force?: boolean } = {}) {
  await homepageStore.loadMobileAdvertisements(options)
}

onMounted(async () => {
  // 本地秒级计时只刷新展示倒计时，开奖结果仍以后端接口和 websocket 推送为准。
  countdownTimer = setInterval(() => {
    nowMs.value += 1000
    if (document.visibilityState === 'hidden') return
    void refreshHomepageAfterDrawTime()
  }, 1000)
  await Promise.all([
    brandingStore.loadBranding({ silent: true }),
    userDataStore.loadProfile(),
    loadHomepage(),
    loadMobileAdvertisements(),
  ])
})

onUnmounted(() => {
  if (countdownTimer) clearInterval(countdownTimer)
})

watch(() => props.wsMessage, (msg) => {
  if (msg?.event === 'draw_result') {
    // WebSocket 开奖推送先局部更新首页轮次展示，再弹出当前彩种开奖结果提示。
    applyDrawResult(msg)
    showNotify({ type: 'success', message: `${lotteryName(msg.lotteryCode || msg.lottery_code)} 第${msg.issue}期：${msg.result}` })
    void loadHomepage({ silent: true, force: true })
    return
  }
  if (msg?.event === 'issue_opened' || msg?.event === 'issue_closed') {
    applyIssueUpdate(msg)
  }
})

watch(heroBanners, (banners) => {
  if (activeHeroBannerIndex.value >= banners.length) activeHeroBannerIndex.value = 0
})

watch(visibleGroups, (groups) => {
  const keys = groups.map((group, index) => groupTabKey(group, index))
  if (!keys.length) {
    activeGroupTab.value = ''
    return
  }
  if (!keys.includes(activeGroupTab.value)) activeGroupTab.value = keys[0]
}, { immediate: true })
</script>

<template>
  <div class="home-dashboard relative min-h-screen overflow-x-hidden bg-surface text-on-surface font-body selection:bg-primary/10">
    <div class="home-top-blush" aria-hidden="true"></div>
    <header class="home-dashboard-header mobile-safe-header fixed top-0 left-0 z-40 flex h-16 w-full items-center justify-between border-b px-6 backdrop-blur-md">
      <div class="flex items-center gap-3">
        <CachedRemoteImage
          :alt="`${branding.site_name} 标志`"
          class="h-8 w-8 rounded-full border border-red-900/10 object-cover shadow-sm"
          :src="branding.logo_url"
          @error="brandingStore['set\u004cogoFallback']()"
        />
        <span class="font-headline text-xl font-bold italic tracking-tighter text-red-900">{{ branding.site_name }}</span>
      </div>
      <WalletHeaderAmount :balance="balance" />
    </header>

    <main class="mobile-safe-main-top relative z-10 mx-auto max-w-2xl space-y-4 px-4 pb-28">
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
            <CachedRemoteImage
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
        <section class="space-y-3" v-if="showFeatured">
          <div class="featured-section-heading flex items-end justify-between rounded-xl bg-surface-container-lowest px-3 py-2.5 shadow-sm shadow-red-900/5">
            <div class="min-w-0">
              <p class="text-[9px] font-bold uppercase tracking-[0.16em] text-primary/70">HOT DRAW</p>
              <h3 class="mt-0.5 truncate font-headline text-base font-extrabold tracking-tight">{{ featuredTitle }}</h3>
              <p class="mt-0.5 text-[10px] font-medium text-on-surface-variant">开奖后自动刷新下一期倒计时</p>
            </div>
            <span class="shrink-0 rounded-full bg-primary/10 px-2.5 py-1 text-[10px] font-bold text-primary">精选</span>
          </div>
          <div class="grid grid-cols-2 gap-2">
            <HomeDrawCard
              v-if="featuredLottery"
              :lottery="featuredLottery"
              variant="featured"
              :countdown-text="countdownText"
              :round-digits="roundDigits"
              :status-text="statusText"
              @open="openLottery"
            />

            <HomeDrawCard
              v-for="lottery in secondaryHighFrequencyLotteries"
              :key="lottery.code"
              :lottery="lottery"
              variant="secondary"
              :countdown-text="countdownText"
              :round-digits="roundDigits"
              @open="openLottery"
            />
          </div>
        </section>

        <!-- 分组区受 settings.groupsEnabled 控制，后端会返回全部销售中彩种的分类分组。 -->
        <section v-if="visibleGroups.length" class="home-category-section">
          <van-tabs
            v-model:active="activeGroupTab"
            class="home-category-tabs"
            swipeable
            shrink
            :ellipsis="false"
          >
            <van-tab
              v-for="(group, groupIndex) in visibleGroups"
              :key="groupTabKey(group, groupIndex)"
              :name="groupTabKey(group, groupIndex)"
              :title="group.name || '彩种分组'"
            >
              <div class="grid grid-cols-2 gap-2 pt-3">
                <HomeDrawCard
                  v-for="lottery in group.lotteries"
                  :key="lottery.code"
                  :lottery="lottery"
                  :variant="groupIndex % 2 === 0 ? 'classic' : 'regional'"
                  :countdown-text="countdownText"
                  :round-digits="roundDigits"
                  @open="openLottery"
                />
              </div>
            </van-tab>
          </van-tabs>
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

<style scoped>
.home-dashboard-header {
  border-color: var(--mobile-app-header-border);
  background: var(--mobile-app-header-background);
  box-shadow: var(--mobile-app-header-shadow);
}

.home-category-tabs :deep(.van-tabs__wrap) {
  height: 2.45rem;
  margin-bottom: 0.15rem;
  overflow: hidden;
  border: 1px solid rgba(255, 255, 255, 0.74);
  border-radius: 9999px;
  background:
    radial-gradient(circle at 92% 5%, rgba(255, 255, 255, 0.78), transparent 30%),
    linear-gradient(135deg, #c8f5ff 0%, #d7c8ff 48%, #ffc4d7 100%);
  box-shadow: 0 8px 20px rgba(123, 82, 156, 0.12);
}

.home-category-tabs :deep(.van-tabs__nav) {
  display: flex;
  width: 100%;
  gap: 0.25rem;
  justify-content: space-around;
  padding: 0.22rem;
  background: transparent;
}

.home-category-tabs :deep(.van-tab) {
  min-width: 4.4rem;
  height: 2rem;
  border-radius: 9999px;
  color: rgba(45, 38, 48, 0.72);
  font-size: 0.78rem;
  font-weight: 900;
  line-height: 1;
}

.home-category-tabs :deep(.van-tab--active) {
  background: rgba(255, 255, 255, 0.84);
  color: #8c0a15;
  box-shadow: 0 4px 12px rgba(123, 82, 156, 0.14);
}

.home-category-tabs :deep(.van-tabs__line) {
  display: none;
}

.home-top-blush {
  position: fixed;
  inset: 0 0 auto;
  z-index: 0;
  height: calc(9.5rem + var(--mobile-status-safe-top, 0px));
  pointer-events: none;
  background:
    linear-gradient(
      180deg,
      rgba(255, 225, 225, 0.96) 0%,
      rgba(255, 238, 238, 0.74) 52%,
      rgba(255, 248, 248, 0) 100%
    );
}
</style>
