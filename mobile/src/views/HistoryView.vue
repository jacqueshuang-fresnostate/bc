<script setup lang="ts">
import { computed, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRoute, useRouter } from 'vue-router'
import DrawResultCard from '../components/lottery/DrawResultCard.vue'
import LotteryGroupFilter from '../components/lottery/LotteryGroupFilter.vue'
import SelectedLotteryHistorySheet from '../components/lottery/SelectedLotteryHistorySheet.vue'
import CachedRemoteImage from '../components/mobile/CachedRemoteImage.vue'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import WalletHeaderAmount from '../components/mobile/WalletHeaderAmount.vue'
import BetOrderCard from '../components/orders/BetOrderCard.vue'
import OrderDetailSheet from '../components/orders/OrderDetailSheet.vue'
import { useBetOrders } from '../composables/useBetOrders'
import { useLotteryHistory } from '../composables/useLotteryHistory'
import { useBrandingStore } from '../stores/branding'
import { useMobileUserDataStore } from '../stores/mobileUserData'
import type { BetOrderView } from '../composables/useBetOrders'

const props = defineProps<{ wsMessage?: Record<string, any> | null }>()
const route = useRoute()
const router = useRouter()
const brandingStore = useBrandingStore()
const userDataStore = useMobileUserDataStore()
const { branding } = storeToRefs(brandingStore)
const { profile } = storeToRefs(userDataStore)

const pageMode = computed<'draws' | 'orders'>(() => route.path === '/orders' ? 'orders' : 'draws')
const balance = computed(() => profile.value?.balance || '0.00')

const {
  activeGroupCode,
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
} = useLotteryHistory()

const {
  activeOrderView,
  orders,
  selectedOrder,
  selectedGroupBuyParticipants,
  selectedDrawNumbers,
  selectedOrderNumber,
  loadingOrders,
  loadingGroupBuyParticipants,
  hasMoreOrders,
  loadOrders,
  setOrderView,
  openOrderDetail,
  closeOrderDetail,
  copyOrderNumber,
  rebetSelectedOrder,
} = useBetOrders(router)
const orderTabs: Array<{ key: BetOrderView; label: string; empty: string; more: string; done: string }> = [
  {
    key: 'groupBuy',
    label: '我的合买',
    empty: '暂无合买记录',
    more: '加载更多合买',
    done: '已加载全部合买',
  },
  {
    key: 'orders',
    label: '我的注单',
    empty: '暂无已下单注单',
    more: '加载更多注单',
    done: '已加载全部注单',
  },
]
const activeOrderTab = computed(
  () => orderTabs.find(tab => tab.key === activeOrderView.value) || orderTabs[0],
)

async function loadBalance() {
  try {
    await userDataStore.loadProfile()
  } catch {}
}

function loadCurrentPage(options: { resetOrderView?: boolean } = {}) {
  loadBalance()
  if (pageMode.value === 'orders') {
    if (options.resetOrderView && activeOrderView.value !== 'groupBuy') {
      setOrderView('groupBuy')
      return
    }
    loadOrders()
    return
  }
  loadLotteryGroups()
  loadDrawHistory()
}

function loadMoreOrders() {
  if (loadingOrders.value || !hasMoreOrders.value) return
  loadOrders({ append: true })
}

function refreshCurrentPage() {
  loadCurrentPage()
}

function switchOrderTab(view: BetOrderView) {
  setOrderView(view)
}

watch(activeGroupCode, () => {
  if (pageMode.value !== 'draws') return
  selectedLotteryVisible.value = false
  selectedLotteryCode.value = null
  selectedLotteryName.value = ''
  selectedLotteryItems.value = []
  loadDrawHistory()
})

watch(() => props.wsMessage, (msg) => {
  if (msg?.event === 'draw_result') {
    if (pageMode.value === 'draws') {
      loadDrawHistory()
      if (selectedLotteryVisible.value && selectedLotteryCode.value) {
        loadSelectedDrawHistory(selectedLotteryCode.value)
      }
    } else {
      loadOrders()
    }
  }
})

watch(() => route.path, () => loadCurrentPage({ resetOrderView: true }), { immediate: true })
</script>

<template>
  <section class="history-center">
    <header
      v-if="pageMode === 'draws'"
      class="mobile-safe-header fixed top-0 left-0 z-40 flex h-16 w-full items-center justify-between bg-white/80 px-6 shadow-sm shadow-red-900/5 backdrop-blur-md"
    >
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

    <header
      v-else
      class="mobile-safe-compact-header sticky top-0 z-30 flex h-14 items-center justify-between bg-white/85 px-4 shadow-sm shadow-red-900/5 backdrop-blur-md"
    >
      <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-50 text-red-900" type="button" @click="router.back()">
        <LucideIcon name="arrow_back" class="h-5 w-5" />
      </button>
      <strong class="font-headline text-base text-red-900">我的记录</strong>
      <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-50 text-red-900 disabled:opacity-60" type="button" :disabled="loadingOrders" @click="refreshCurrentPage">
        <LucideIcon name="refresh" class="h-4.5 w-4.5" />
      </button>
    </header>

    <main
      class="history-content"
      :class="pageMode === 'orders' ? 'history-content--orders' : 'history-content--draws'"
    >
      <!-- 开奖页只展示最新开奖；注单记录从“我的”进入。 -->
      <section v-if="pageMode === 'draws'" class="draw-panel">
        <div class="draw-panel__header">
          <h2>最新开奖</h2>
        </div>

        <LotteryGroupFilter
          :lottery-group-filters="lotteryGroupFilters"
          :active-group-code="activeGroupCode"
          @select="activeGroupCode = $event"
        />

        <div v-if="loadingDraws" class="state-block">
          <van-loading>加载中...</van-loading>
        </div>
        <van-empty v-else-if="!visibleDrawItems.length" description="暂无开奖结果" />
        <div v-else class="draw-list">
          <DrawResultCard
            v-for="item in visibleDrawItems"
            :key="item.id"
            :item="item"
            @open="openDrawHistory(item)"
          />
        </div>
      </section>

      <section v-else class="orders-panel">
        <div class="orders-tabs" role="tablist" aria-label="注单记录分类">
          <button
            v-for="tab in orderTabs"
            :key="tab.key"
            type="button"
            role="tab"
            :aria-selected="activeOrderView === tab.key"
            class="orders-tabs__button"
            :class="{ 'orders-tabs__button--active': activeOrderView === tab.key }"
            @click="switchOrderTab(tab.key)"
          >
            {{ tab.label }}
          </button>
        </div>
        <div v-if="loadingOrders" class="state-block">
          <van-loading>加载中...</van-loading>
        </div>
        <van-empty v-else-if="!orders.length" :description="activeOrderTab.empty" />
        <div v-else class="orders-list orders-list--records">
          <BetOrderCard
            v-for="order in orders"
            :key="order.id"
            :order="order"
            @open="openOrderDetail(order)"
          />
          <button
            v-if="hasMoreOrders"
            type="button"
            class="rounded-2xl bg-red-50 px-4 py-3 text-xs font-black text-primary active:scale-[0.99] disabled:opacity-60"
            :disabled="loadingOrders"
            @click="loadMoreOrders"
          >
            {{ loadingOrders ? '加载中...' : activeOrderTab.more }}
          </button>
          <p v-else class="py-1 text-center text-[11px] font-semibold text-stone-500">{{ activeOrderTab.done }}</p>
        </div>
      </section>
    </main>

    <van-popup
      v-model:show="selectedLotteryVisible"
      position="bottom"
      round
      class="selected-lottery-history-popup"
      @closed="closeDrawHistory"
    >
      <SelectedLotteryHistorySheet
        :selected-lottery-name="selectedLotteryName"
        :selected-lottery-items="selectedLotteryItems"
        :loading-selected-lottery="loadingSelectedLottery"
        :close-draw-history="closeDrawHistory"
        @close="closeDrawHistory"
      />
    </van-popup>

    <OrderDetailSheet
      v-if="selectedOrder"
      :selected-order="selectedOrder"
      :group-buy-participants="selectedGroupBuyParticipants"
      :loading-group-buy-participants="loadingGroupBuyParticipants"
      :selected-draw-numbers="selectedDrawNumbers"
      :selected-order-number="selectedOrderNumber"
      @close="closeOrderDetail"
      @copy="copyOrderNumber"
      @rebet="rebetSelectedOrder"
    />
  </section>
</template>

<style scoped>
.history-center {
  min-height: 100vh;
  min-height: 100dvh;
  background:
    radial-gradient(circle at 6% 0%, rgba(255, 218, 215, 0.78), transparent 28%),
    linear-gradient(180deg, #f9f9f9 0%, #f3f3f3 48%, #eeeeee 100%);
  color: #1a1c1c;
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}

.history-center ::selection {
  background: rgba(140, 10, 21, 0.1);
}

.history-content {
  width: min(100%, 672px);
  margin: 0 auto;
}

.history-content--draws {
  padding: var(--mobile-brand-page-top) 16px calc(var(--mobile-bottom-nav-space) + 16px);
}

.history-content--orders {
  padding: 16px 16px 28px;
}

.draw-panel__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.draw-panel__header h2 {
  margin: 0;
  color: #7a0711;
  font-size: 18px;
  font-weight: 900;
  letter-spacing: -0.04em;
}

.draw-list,
.orders-list {
  display: grid;
  gap: 16px;
}

.orders-list--records {
  gap: 18px;
}

.orders-tabs {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px;
  margin-bottom: 14px;
  border-radius: 14px;
  padding: 4px;
  background: rgba(255, 255, 255, 0.74);
  box-shadow: inset 0 0 0 1px rgba(140, 10, 21, 0.06);
  backdrop-filter: blur(14px);
}

.orders-tabs__button {
  min-height: 34px;
  border: 0;
  border-radius: 10px;
  background: transparent;
  color: #7e625f;
  font-size: 13px;
  font-weight: 900;
  transition: background 0.2s ease, box-shadow 0.2s ease, color 0.2s ease, transform 0.2s ease;
}

.orders-tabs__button:active {
  transform: scale(0.98);
}

.orders-tabs__button--active {
  background: linear-gradient(135deg, #8c0a15, #b42327);
  color: #fff;
  box-shadow: 0 8px 18px rgba(140, 10, 21, 0.16);
}

.selected-lottery-history-popup {
  overflow: hidden;
  background: transparent;
}

.state-block {
  padding: 40px 0;
  text-align: center;
}

:deep(.van-empty) {
  padding: 48px 0;
}

@media (max-width: 360px) {
  .history-center > header {
    padding-right: 16px;
    padding-left: 16px;
  }

  .history-center > header span:first-of-type {
    font-size: 18px;
  }
}
</style>
