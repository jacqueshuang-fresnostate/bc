<script setup lang="ts">
import { computed, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRoute, useRouter } from 'vue-router'
import DrawResultCard from '../components/lottery/DrawResultCard.vue'
import LotteryGroupFilter from '../components/lottery/LotteryGroupFilter.vue'
import SelectedLotteryHistorySheet from '../components/lottery/SelectedLotteryHistorySheet.vue'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import BetOrderCard from '../components/orders/BetOrderCard.vue'
import OrderDetailSheet from '../components/orders/OrderDetailSheet.vue'
import { useBetOrders } from '../composables/useBetOrders'
import { useLotteryHistory } from '../composables/useLotteryHistory'
import { useBrandingStore } from '../stores/branding'
import { useMobileUserDataStore } from '../stores/mobileUserData'

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
  orders,
  selectedOrder,
  selectedGroupBuyParticipants,
  selectedDrawNumbers,
  selectedOrderNumber,
  loadingOrders,
  loadingGroupBuyParticipants,
  hasMoreOrders,
  loadOrders,
  openOrderDetail,
  closeOrderDetail,
  copyOrderNumber,
  rebetSelectedOrder,
} = useBetOrders(router)

async function loadBalance() {
  try {
    await userDataStore.loadProfile()
  } catch {}
}

function loadCurrentPage() {
  loadBalance()
  if (pageMode.value === 'orders') {
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

watch(() => route.path, () => loadCurrentPage(), { immediate: true })
</script>

<template>
  <section class="history-center">
    <header
      v-if="pageMode === 'draws'"
      class="mobile-safe-header fixed top-0 left-0 z-40 flex h-16 w-full items-center justify-between bg-white/80 px-6 shadow-sm shadow-red-900/5 backdrop-blur-md"
    >
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

    <header
      v-else
      class="mobile-safe-compact-header sticky top-0 z-30 flex h-14 items-center justify-between bg-white/85 px-4 shadow-sm shadow-red-900/5 backdrop-blur-md"
    >
      <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-50 text-red-900" type="button" @click="router.back()">
        <LucideIcon name="arrow_back" class="h-5 w-5" />
      </button>
      <strong class="font-headline text-base text-red-900">我的注单</strong>
      <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-50 text-red-900 disabled:opacity-60" type="button" :disabled="loadingOrders" @click="loadCurrentPage">
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
        <div v-if="loadingOrders" class="state-block">
          <van-loading>加载中...</van-loading>
        </div>
        <van-empty v-else-if="!orders.length" description="暂无注单" />
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
            {{ loadingOrders ? '加载中...' : '加载更多注单' }}
          </button>
          <p v-else class="py-1 text-center text-[11px] font-semibold text-stone-500">已加载全部注单</p>
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
  padding: var(--mobile-brand-page-top) 16px 112px;
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
