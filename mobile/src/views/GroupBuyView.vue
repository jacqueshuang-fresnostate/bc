<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { storeToRefs } from 'pinia'
import { useRoute } from 'vue-router'
import { useBrandingStore } from '../stores/branding'
import CachedAvatarImage from '../components/mobile/CachedAvatarImage.vue'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { fetchGroupBuyDetail } from '../features/group-buy/api'
import { useGroupBuyHall } from '../features/group-buy/composables/useGroupBuyHall'
import { useGroupBuyDetail } from '../features/group-buy/composables/useGroupBuyDetail'
import { useGroupBuyCreate } from '../features/group-buy/composables/useGroupBuyCreate'
import type { GroupBuyParticipant } from '../features/group-buy/types'
import {
  canJoinPlan,
  formatMoney,
  formatPlanTitle,
  formatPlayName,
  initiatorAvatarText,
  initiatorAvatarUrl,
  initiatorDisplay,
  progressPercent,
  progressRemainingText,
  progressTrackWidth,
  statusText,
} from '../features/group-buy/presentation'
import { formatDateTime } from '../utils/lotteryFormat'

const route = useRoute()
const brandingStore = useBrandingStore()
const { branding } = storeToRefs(brandingStore)
const initialTab = String(route.query.tab || 'hall')
const activeTab = ref(initialTab === 'my' ? 'my' : 'hall')
const initialCreateVisible = initialTab === 'create'
const lotteryCode = ref(String(route.query.lottery_code || ''))

const quickAmountOptions: Array<{ label: string; value: number | 'all' }> = [
  { label: '1元', value: 1 },
  { label: '10元', value: 10 },
  { label: '50元', value: 50 },
  { label: '全包', value: 'all' },
]


const {
  loadingHall,
  activeFilter,
  hallHasMore,
  hallCategoryChips,
  displayedHallItems,
  loadHallGroups,
  loadHall,
} = useGroupBuyHall(lotteryCode)

const {
  createVisible,
  submittingCreate,
  loadingMy,
  balance,
  myGroupBuys,
  myGroupBuysHasMore,
  createLotteryOptions,
  createIssueOptions,
  createPlayOptions,
  createSettings,
  createExtras,
  createForm,
  computedShareAmount,
  fixedShareAmount,
  computedShareCount,
  requiredSelfShares,
  createPaymentAmount,
  loadBalance,
  selectCreateLottery,
  closeCreatePlan,
  loadCreateOptions,
  createGroupBuy: submitCreateGroupBuy,
  loadMyGroupBuys,
} = useGroupBuyCreate(lotteryCode, { loadHall, activeTab, initialVisible: initialCreateVisible })

const {
  selectedGroupBuy,
  loadingDetail,
  submittingJoin,
  joinAmountInput,
  canJoin,
  joinAmount,
  joinAmountHint,
  detailVisible,
  commitJoinAmountInput,
  decreaseJoinAmount,
  increaseJoinAmount,
  applyQuickAmount,
  openDetail,
  closeDetail,
  joinGroupBuy,
} = useGroupBuyDetail({ loadBalance, loadHall, loadMyGroupBuys, activeTab })

async function createGroupBuy() {
  const createdPlan = await submitCreateGroupBuy()
  selectedGroupBuy.value = createdPlan || selectedGroupBuy.value
}

function loadMoreHall() {
  if (loadingHall.value || !hallHasMore.value) return
  loadHall({ append: true })
}

function loadMoreMyGroupBuys() {
  if (loadingMy.value || !myGroupBuysHasMore.value) return
  loadMyGroupBuys({ append: true })
}

function participantName(participant: GroupBuyParticipant) {
  return participant.display_name || '会员'
}

function participantAvatarText(participant: GroupBuyParticipant) {
  return participantName(participant).trim().slice(0, 1).toUpperCase() || '会'
}

function participantAmountText(participant: GroupBuyParticipant) {
  return formatMoney(participant.amount)
}

function participantSharesText(participant: GroupBuyParticipant) {
  const shares = Number(participant.shares || 0)
  return `${Number.isFinite(shares) ? shares : 0}份`
}

function participantTimeText(participant: GroupBuyParticipant) {
  return formatDateTime(participant.created_at)
}

async function openPlanFromQuery() {
  const planId = String(route.query.plan_id || '').trim()
  if (!planId) return
  const localPlan = displayedHallItems.value.find(item => item.id === planId)
    || myGroupBuys.value.find(item => item.id === planId)
  if (localPlan) {
    openDetail(localPlan)
    return
  }
  try {
    const result = await fetchGroupBuyDetail(planId)
    if (result.data?.id) openDetail(result.data)
  } catch {
    // 查询参数只用于聊天大厅跳转定位，详情加载失败时保留大厅默认列表。
  }
}

watch(() => route.query.lottery_code, (value) => {
  lotteryCode.value = String(value || '')
  createForm.value.lottery_code = lotteryCode.value
  loadCreateOptions()
  loadHall()
})

watch(() => route.query.plan_id, () => {
  void openPlanFromQuery()
})

watch(activeFilter, () => {
  loadHall()
})

watch(activeTab, (tab) => {
  if (tab === 'hall') loadHall()
  if (tab === 'my') loadMyGroupBuys()
})

onMounted(async () => {
  await Promise.all([loadBalance(), loadHallGroups(), loadCreateOptions(), loadHall()])
  if (activeTab.value === 'my') await loadMyGroupBuys()
  await openPlanFromQuery()
})
</script>

<template>
  <div class="group-buy-page min-h-screen bg-[#f7f7f7] pb-24 text-[#171717]">
    <header v-if="activeTab === 'hall'" class="group-buy-brand-header mobile-safe-header fixed top-0 left-0 z-40 flex h-16 w-full items-center justify-between bg-white/80 px-6 shadow-sm shadow-red-900/5 backdrop-blur-md">
      <h1 class="sr-only">合买大厅</h1>
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
    <van-nav-bar v-else title="我的合买" left-arrow @click-left="activeTab = 'hall'" />

    <van-tabs v-model:active="activeTab" sticky class="group-buy-tabs hidden-tab-header">
      <van-tab title="大厅" name="hall">
        <section class="group-buy-hall group-buy-hall-scroll mobile-safe-main-top-tight space-y-2 px-3 pb-4">
          <div class="hallCategoryChips filterChips flex min-h-11 gap-2 overflow-x-auto pb-1">
            <button
              v-for="chip in hallCategoryChips"
              :key="chip.value"
              class="flex min-h-10 shrink-0 items-center justify-center rounded-xl px-4 py-2 text-xs font-bold leading-none transition"
              :class="activeFilter === chip.value ? 'bg-red-900 !text-white shadow-md shadow-red-900/20' : 'bg-white text-stone-700 shadow-sm'"
              @click="activeFilter = chip.value"
            >
              {{ chip.label }}
            </button>
          </div>

          <van-loading v-if="loadingHall && !displayedHallItems.length" class="mx-auto my-8 block" />
          <van-empty v-else-if="!displayedHallItems.length" description="暂无合买计划" />
          <template v-else>
            <article
              v-for="item in displayedHallItems"
              :key="item.id"
              class="group-buy-plan-card rounded-xl bg-white px-3 py-2.5 shadow-[0_4px_14px_rgba(15,23,42,0.045)] transition active:scale-[0.99]"
              @click="openDetail(item)"
            >
              <div class="flex min-w-0 items-start justify-between gap-2">
                <div class="flex min-w-0 items-center gap-2">
                  <div class="group-buy-initiator-avatar flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-red-50 font-headline text-sm font-black text-red-800 ring-1 ring-red-900/10">
                    <CachedAvatarImage
                      v-if="initiatorAvatarUrl(item)"
                      :alt="`${initiatorDisplay(item)}头像`"
                      class="h-full w-full object-cover"
                      :src="initiatorAvatarUrl(item)"
                    >
                      <span>{{ initiatorAvatarText(item) }}</span>
                    </CachedAvatarImage>
                    <span v-else>{{ initiatorAvatarText(item) }}</span>
                  </div>
                  <div class="min-w-0">
                    <h3 class="truncate font-headline text-sm font-black leading-tight text-stone-950">{{ item.lottery_name || item.title || formatPlanTitle(item) }}</h3>
                    <p class="mt-0.5 truncate text-[10px] font-medium text-stone-500">第{{ item.issue }}期 · {{ formatPlayName(item) }}</p>
                  </div>
                </div>
                <span
                  class="shrink-0 rounded-full px-2 py-1 text-[10px] font-black"
                  :class="canJoinPlan(item) ? 'bg-red-900 text-white' : 'bg-stone-200 text-stone-500'"
                  @click.stop="canJoinPlan(item) && openDetail(item)"
                >
                  {{ canJoinPlan(item) ? '参与' : statusText(item.status) }}
                </span>
              </div>

              <div class="mt-2 grid grid-cols-[1fr_0.8fr_0.8fr] gap-2 text-[10px] leading-tight">
                <div class="min-w-0">
                  <span class="block text-stone-400">发起人</span>
                  <b class="group-buy-initiator-name mt-0.5 block truncate">{{ initiatorDisplay(item) }}</b>
                </div>
                <div>
                  <span class="block text-stone-400">总额</span>
                  <b class="mt-0.5 block font-headline text-xs text-stone-950">{{ formatMoney(item.total_amount) }}</b>
                </div>
                <div>
                  <span class="block text-stone-400">单份</span>
                  <b class="mt-0.5 block font-headline text-xs text-stone-950">{{ formatMoney(item.share_amount) }}</b>
                </div>
              </div>

              <div class="mt-2">
                <div class="mb-1 flex items-center justify-between text-[10px]">
                  <span class="font-black text-red-900">已满 {{ progressPercent(item) }}%</span>
                  <span class="text-stone-500">{{ progressRemainingText(item) }}</span>
                </div>
                <div class="h-1 w-full overflow-hidden rounded-full bg-stone-100">
                  <div class="h-full rounded-full lacquer-gradient" :style="{ width: progressTrackWidth(item) }"></div>
                </div>
              </div>
            </article>
            <button
              v-if="hallHasMore"
              type="button"
              class="w-full rounded-xl bg-red-50 px-4 py-3 text-xs font-black text-red-900 active:scale-[0.99] disabled:opacity-60"
              :disabled="loadingHall"
              @click="loadMoreHall"
            >
              {{ loadingHall ? '加载中...' : '加载更多合买计划' }}
            </button>
            <p v-else class="py-2 text-center text-[11px] font-semibold text-stone-500">已加载全部合买计划</p>
          </template>
        </section>
      </van-tab>

      <van-tab title="我的" name="my">
        <section class="space-y-3 p-3">
          <van-loading v-if="loadingMy && !myGroupBuys.length" class="mx-auto my-8 block" />
          <van-empty v-else-if="!myGroupBuys.length" description="暂无参与记录" />
          <template v-else>
            <button v-for="item in myGroupBuys" :key="item.id" class="w-full rounded-xl bg-white p-4 text-left shadow-sm" @click="openDetail(item)">
              <div class="flex items-start justify-between gap-3">
                <div>
                  <h3 class="text-sm font-bold">{{ formatPlanTitle(item) }}</h3>
                  <p class="mt-1 text-xs text-stone-500">{{ item.lottery_name }} · {{ statusText(item.status) }}</p>
                </div>
                <span class="text-xs font-bold text-red-800">我的出资 {{ item.my_participation?.amount || '0.00' }}元</span>
              </div>
              <p class="mt-2 text-xs text-stone-500">我的出资 {{ item.my_participation?.amount || '0.00' }} 元</p>
            </button>
            <button
              v-if="myGroupBuysHasMore"
              type="button"
              class="w-full rounded-xl bg-red-50 px-4 py-3 text-xs font-black text-red-900 active:scale-[0.99] disabled:opacity-60"
              :disabled="loadingMy"
              @click="loadMoreMyGroupBuys"
            >
              {{ loadingMy ? '加载中...' : '加载更多我的合买' }}
            </button>
            <p v-else class="py-2 text-center text-[11px] font-semibold text-stone-500">已加载全部参与记录</p>
          </template>
        </section>
      </van-tab>
    </van-tabs>

    <van-popup v-model:show="createVisible" position="bottom" round overlay-class="backdrop-blur-sm" :style="{ height: '70dvh', maxHeight: '70dvh' }">
      <section class="group-buy-create-popup group-buy-create-modal flex h-[70dvh] max-h-[70dvh] flex-col overflow-hidden bg-[#f9f9f9]">
        <header class="group-buy-create-header z-10 flex shrink-0 items-center justify-between border-b border-stone-100 bg-white/90 px-4 py-3 backdrop-blur-md">
          <h2 class="font-headline text-lg font-black tracking-wide text-red-900">发起合买计划</h2>
          <button aria-label="关闭发起合买弹窗" class="flex h-8 w-8 items-center justify-center rounded-full text-stone-500 transition-colors hover:bg-stone-100" @click="closeCreatePlan">×</button>
        </header>

        <van-form class="flex min-h-0 flex-1 flex-col" @submit="createGroupBuy">
          <main class="group-buy-create-scroll flex-1 space-y-4 overflow-y-auto bg-[#f9f9f9] px-4 py-4">
            <section class="rounded-xl bg-white p-4 shadow-[0_2px_10px_rgba(140,10,21,0.02)]">
              <h3 class="mb-3 font-headline text-base font-black text-stone-950">选择彩种</h3>
              <div class="grid grid-cols-3 gap-3">
                <button
                  v-for="option in createLotteryOptions"
                  :key="option.value"
                  type="button"
                  class="flex flex-col items-center justify-center rounded-lg p-3 transition-all"
                  :class="createForm.lottery_code === option.value ? 'border border-red-900/20 bg-red-50 text-red-900' : 'bg-stone-100 text-stone-600 hover:bg-stone-200'"
                  @click="selectCreateLottery(option.value)"
                >
                  <LucideIcon :name="option.icon" class="mb-1 h-5 w-5" />
                  <span class="text-sm font-medium">{{ option.label }}</span>
                </button>
              </div>
              <div class="mt-4 grid grid-cols-2 gap-3">
                <select v-model="createForm.issue" class="rounded-lg border-0 bg-stone-100 px-3 py-3 text-sm focus:ring-1 focus:ring-red-900/30">
                  <option value="" disabled>请选择期号</option>
                  <option v-for="option in createIssueOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
                </select>
                <select v-model="createForm.play_code" class="rounded-lg border-0 bg-stone-100 px-3 py-3 text-sm focus:ring-1 focus:ring-red-900/30">
                  <option value="" disabled>请选择玩法</option>
                  <option v-for="option in createPlayOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
                </select>
              </div>
            </section>

            <section class="rounded-xl border border-stone-100 bg-white p-4 shadow-[0_2px_10px_rgba(140,10,21,0.02)]">
              <div class="mb-4 flex items-center justify-between">
                <h3 class="font-headline text-lg font-black text-stone-950">投注内容</h3>
              </div>
              <div class="rounded-lg bg-stone-50 p-4">
                <div class="flex items-start justify-between gap-3">
                  <textarea v-model="createForm.numbers" class="min-h-20 flex-1 resize-none border-0 bg-transparent p-0 text-sm font-bold text-stone-900 focus:ring-0" placeholder="直选 1|2|3；组合 1,2,3；胆拖 1|2,3,4；大小单双 tens:big|ones:odd"></textarea>
                  <button type="button" class="text-xl text-stone-400" @click="createForm.numbers = ''">×</button>
                </div>
                <div class="mt-3 flex justify-between text-xs text-stone-500">
                  <span>普通投注</span>
                  <span>1注 {{ createForm.total_amount || '0.00' }}元</span>
                </div>
              </div>
              <div class="mt-5 rounded-lg bg-stone-50 p-3 transition-all focus-within:bg-white focus-within:shadow-[inset_0_0_0_1px_rgba(140,10,21,0.2)]">
                <label class="mb-1 block text-xs text-stone-500">总金额 (元)</label>
                <div class="flex items-center">
                  <span class="mr-2 font-headline text-xl font-black text-red-900">¥</span>
                  <input v-model="createForm.total_amount" class="w-full border-0 bg-transparent p-0 font-headline text-xl font-black text-stone-950 focus:ring-0" placeholder="0.00" type="number" />
                </div>
              </div>
            </section>

            <section class="rounded-xl border border-stone-100 bg-white p-4 shadow-[0_2px_10px_rgba(140,10,21,0.02)]">
              <h3 class="mb-4 font-headline text-lg font-black text-stone-950">合买设置</h3>
              <div class="space-y-4">
                <div class="flex items-center justify-between rounded-lg bg-stone-50 p-3 transition-all focus-within:bg-white focus-within:shadow-[inset_0_0_0_1px_rgba(140,10,21,0.2)]">
                  <div>
                    <label class="mb-1 block text-xs text-stone-500">分成份数</label>
                    <div class="flex items-center">
                      <span class="font-headline text-lg font-black text-stone-950">{{ computedShareCount }}</span>
                      <span class="ml-1 text-sm text-stone-500">份</span>
                    </div>
                  </div>
                  <div class="text-right">
                    <span class="mb-1 block text-xs text-stone-500">每份金额</span>
                    <span class="font-headline text-sm font-black text-red-900">{{ formatMoney(computedShareAmount) }}</span>
                    <span class="mt-1 block text-[10px] text-stone-500">固定每份金额 {{ formatMoney(fixedShareAmount) }}</span>
                    <span class="mt-1 block text-[10px] text-stone-500">最低每份金额 {{ formatMoney(createSettings.min_share_amount) }}</span>
                  </div>
                </div>

                <div class="rounded-lg bg-stone-50 p-3 transition-all focus-within:bg-white focus-within:shadow-[inset_0_0_0_1px_rgba(140,10,21,0.2)]">
                  <div class="mb-2 flex items-center justify-between">
                    <label class="text-sm font-medium text-stone-950">发起人提成</label>
                    <span class="font-headline text-sm font-black text-stone-950">{{ createExtras.commission_rate || 0 }}%</span>
                  </div>
                  <input v-model="createExtras.commission_rate" class="h-1 w-full accent-red-900" max="10" min="0" type="range" />
                  <div class="mt-1 flex justify-between text-xs text-stone-500"><span>0%</span><span>10%</span></div>
                </div>

                <div class="flex items-center justify-between rounded-lg bg-stone-50 p-3 transition-all focus-within:bg-white focus-within:shadow-[inset_0_0_0_1px_rgba(140,10,21,0.2)]">
                  <div>
                    <label class="mb-1 block text-sm font-medium text-stone-950">发起人自购</label>
                    <span class="text-xs text-stone-500">发起人最低自购{{ createSettings.initiator_min_buy_ratio }}%</span>
                    <span v-if="requiredSelfShares > 0" class="mt-1 block text-xs font-medium text-red-900">至少 {{ requiredSelfShares }} 份</span>
                  </div>
                  <div class="flex items-center rounded border border-red-100 bg-white px-2 py-1">
                    <input v-model.number="createForm.self_shares" class="w-16 border-0 bg-transparent p-0 text-right font-headline text-sm font-medium text-stone-950 focus:ring-0" min="0" type="number" />
                    <span class="ml-1 text-sm font-medium text-stone-900">份</span>
                  </div>
                </div>

                <div class="flex rounded-lg bg-stone-50 p-1">
                  <button type="button" class="flex-1 rounded-md bg-white py-2 text-sm font-medium text-red-900 shadow-sm" @click="createExtras.visibility = '公开可见'">公开可见</button>
                  <button type="button" class="flex-1 rounded-md py-2 text-sm font-medium text-stone-500" @click="createExtras.visibility = '仅参与者可见'">仅参与者可见</button>
                </div>
              </div>
            </section>
          </main>

          <div class="group-buy-create-dock z-10 shrink-0 border-t border-stone-200 bg-white px-4 py-3 shadow-[0_-10px_20px_rgba(0,0,0,0.03)]">
            <div class="flex items-center justify-between">
              <div>
                <span class="text-xs text-stone-500">共需支付</span>
                <div class="flex items-baseline">
                  <span class="mr-1 font-headline text-sm font-black text-red-900">¥</span>
                  <span class="font-headline text-2xl font-black text-red-900">{{ createPaymentAmount }}</span>
                </div>
              </div>
              <button type="submit" class="rounded-full bg-gradient-to-br from-red-900 to-red-700 px-8 py-3 font-headline text-base font-black text-white shadow-[0_4px_12px_rgba(140,10,21,0.2)] transition-all active:scale-95" :disabled="submittingCreate">
                {{ submittingCreate ? '发布中...' : '发布计划' }}
              </button>
            </div>
          </div>
        </van-form>
      </section>
    </van-popup>
    <van-popup v-model:show="detailVisible" position="bottom" round :style="{ maxHeight: '66dvh' }">
      <section v-if="selectedGroupBuy" class="group-buy-detail-sheet flex max-h-[66dvh] flex-col overflow-y-auto bg-[#fff8f1] p-3">
        <div class="mx-auto mb-3 h-1.5 w-12 rounded-full bg-stone-300"></div>
        <div class="mb-4 flex items-start justify-between gap-3">
          <div>
            <p class="text-[10px] font-bold uppercase tracking-[0.2em] text-red-300">方案详情</p>
            <h2 class="mt-1 font-headline text-xl font-black text-red-950">{{ formatPlanTitle(selectedGroupBuy) }}</h2>
            <p class="mt-1 text-xs text-stone-500">{{ selectedGroupBuy.lottery_name }} · 第{{ selectedGroupBuy.issue }}期</p>
          </div>
          <van-button size="small" plain @click="closeDetail">关闭</van-button>
        </div>

        <van-loading v-if="loadingDetail" />
        <template v-else>
          <div class="space-y-3">
            <div class="rounded-3xl bg-white p-4 shadow-sm">
              <div class="grid grid-cols-2 gap-3 text-xs">
                <div class="rounded-2xl bg-red-50 p-3"><span class="text-stone-500">总金额</span><b class="mt-1 block text-base text-red-900">{{ formatMoney(selectedGroupBuy.total_amount) }}</b></div>
                <div class="rounded-2xl bg-stone-50 p-3"><span class="text-stone-500">每份金额</span><b class="mt-1 block text-base text-stone-900">{{ formatMoney(selectedGroupBuy.share_amount) }}</b></div>
                <div class="rounded-2xl bg-stone-50 p-3"><span class="text-stone-500">剩余份数</span><b class="mt-1 block text-base text-stone-900">{{ selectedGroupBuy.available_shares }}份</b></div>
                <div class="rounded-2xl bg-stone-50 p-3"><span class="text-stone-500">参与人数</span><b class="mt-1 block text-base text-stone-900">{{ selectedGroupBuy.participant_count || 0 }}人</b></div>
              </div>
              <div class="mt-4">
                <div class="mb-2 flex items-center justify-between text-xs text-stone-500">
                  <span>合买进度</span>
                  <b class="text-red-900">{{ selectedGroupBuy.progress_percent }}%</b>
                </div>
                <van-progress :percentage="progressPercent(selectedGroupBuy)" color="#8c0a15" track-color="#f3e7df" :show-pivot="false" />
              </div>
            </div>

            <div class="rounded-3xl bg-white p-4 shadow-sm">
              <p class="mb-2 text-xs font-bold text-stone-400">投注信息</p>
              <div class="space-y-2 text-sm">
                <div class="flex justify-between gap-4"><span class="text-stone-500">玩法</span><b class="text-right">{{ formatPlayName(selectedGroupBuy) }}</b></div>
                <div class="flex justify-between gap-4"><span class="text-stone-500">成单订单</span><b class="text-right">{{ selectedGroupBuy.order_id || '未成单' }}</b></div>
                <div class="flex justify-between gap-4"><span class="text-stone-500">号码</span><b class="text-right">{{ selectedGroupBuy.numbers }}</b></div>
                <div class="flex justify-between gap-4"><span class="text-stone-500">我的参与</span><b class="text-right">{{ selectedGroupBuy.my_participation ? `${selectedGroupBuy.my_participation.amount}元` : '暂未参与' }}</b></div>
              </div>
            </div>

            <div class="rounded-3xl bg-white p-4 shadow-sm">
              <div class="mb-3 flex items-center justify-between gap-3">
                <div>
                  <p class="text-xs font-bold text-stone-400">参与人</p>
                  <h3 class="mt-0.5 font-headline text-base font-black text-stone-950">认购明细</h3>
                </div>
                <span class="rounded-full bg-red-50 px-2.5 py-1 text-xs font-black text-red-900">
                  {{ selectedGroupBuy.participant_count || selectedGroupBuy.participants?.length || 0 }}人
                </span>
              </div>
              <div v-if="selectedGroupBuy.participants?.length" class="space-y-2">
                <article
                  v-for="participant in selectedGroupBuy.participants"
                  :key="participant.id"
                  class="flex items-center justify-between gap-3 rounded-2xl bg-stone-50 p-2.5"
                  :class="{ 'bg-red-50 ring-1 ring-red-100': participant.is_mine }"
                >
                  <div class="flex min-w-0 items-center gap-2.5">
                    <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-white font-headline text-sm font-black text-red-900 shadow-sm">
                      {{ participantAvatarText(participant) }}
                    </div>
                    <div class="min-w-0">
                      <div class="flex min-w-0 items-center gap-1.5">
                        <b class="truncate text-sm font-black text-stone-950">{{ participantName(participant) }}</b>
                        <span v-if="participant.is_mine" class="shrink-0 rounded-full bg-red-900 px-1.5 py-0.5 text-[10px] font-bold text-white">我</span>
                      </div>
                      <span class="mt-0.5 block truncate text-[11px] text-stone-500">{{ participantTimeText(participant) }}</span>
                    </div>
                  </div>
                  <div class="shrink-0 text-right">
                    <b class="block font-headline text-sm font-black text-red-900">{{ participantAmountText(participant) }}</b>
                    <span class="mt-0.5 block text-[11px] font-medium text-stone-500">{{ participantSharesText(participant) }}</span>
                  </div>
                </article>
              </div>
              <div v-else class="rounded-2xl bg-stone-50 px-3 py-4 text-center text-xs font-medium text-stone-500">
                暂无参与人记录
              </div>
            </div>

            <div v-if="canJoin" class="rounded-3xl bg-white p-4 shadow-sm">
              <div class="mb-3 flex items-center justify-between">
                <h3 class="font-headline text-base font-black text-stone-950">认购金额</h3>
                <span class="text-xs text-stone-500">余额 {{ formatMoney(balance) }}</span>
              </div>
              <div class="flex items-center justify-between rounded-2xl bg-stone-50 p-2">
                <button class="flex h-10 w-10 items-center justify-center rounded-full bg-white text-xl font-black text-red-900 shadow-sm" @click="decreaseJoinAmount">−</button>
                <div class="flex flex-1 items-center justify-center px-3">
                  <span class="mr-1 font-headline text-xl font-black text-red-900">¥</span>
                  <input
                    v-model="joinAmountInput"
                    class="w-28 border-0 bg-transparent text-center font-headline text-2xl font-black text-stone-950 focus:ring-0"
                    inputmode="decimal"
                    min="0.01"
                    step="0.01"
                    type="number"
                    @blur="commitJoinAmountInput"
                    @keyup.enter="commitJoinAmountInput"
                  />
                </div>
                <button class="flex h-10 w-10 items-center justify-center rounded-full bg-white text-xl font-black text-red-900 shadow-sm" @click="increaseJoinAmount">＋</button>
              </div>
              <p class="mt-2 text-xs font-medium text-stone-500">{{ joinAmountHint }}</p>
              <div class="mt-3 grid grid-cols-4 gap-2">
                <button v-for="option in quickAmountOptions" :key="option.label" class="rounded-full bg-red-50 py-2 text-xs font-bold text-red-900" @click="applyQuickAmount(option.value)">{{ option.label }}</button>
              </div>
            </div>
          </div>

          <div class="mt-auto pt-4">
            <div v-if="canJoin" class="rounded-3xl bg-white p-3 shadow-[0_-8px_24px_rgba(124,45,18,0.08)]">
              <div class="mb-3 flex items-center justify-between px-1">
                <span class="text-xs text-stone-500">共计金额</span>
                <b class="font-headline text-xl font-black text-red-900">{{ formatMoney(joinAmount) }}</b>
              </div>
              <van-button block round type="primary" :loading="submittingJoin" class="group-buy-join-button font-headline font-black" @click="joinGroupBuy">确认认购</van-button>
            </div>
            <van-empty v-else image="search" description="该合买计划暂不可参与" />
          </div>
        </template>
      </section>
    </van-popup>
  </div>
</template>

<style scoped>
.group-buy-initiator-name {
  font-size: 13px;
  font-weight: 900;
  line-height: 1.1;
  color: #7f111c;
}

.group-buy-join-button {
  height: 48px;
  border: 0 !important;
  border-radius: 16px !important;
  background: linear-gradient(135deg, #8c0a15 0%, #b91c1c 100%) !important;
  color: #ffffff !important;
  box-shadow: 0 10px 24px rgba(140, 10, 21, 0.24);
}

.group-buy-join-button :deep(.van-button__text),
.group-buy-join-button :deep(.van-loading),
.group-buy-join-button :deep(.van-loading__spinner) {
  color: #ffffff !important;
}

.group-buy-join-button.van-button--disabled {
  background: #d8d0ce !important;
  color: rgba(26, 28, 28, 0.56) !important;
  box-shadow: none;
}

.group-buy-join-button.van-button--disabled :deep(.van-button__text),
.group-buy-join-button.van-button--disabled :deep(.van-loading),
.group-buy-join-button.van-button--disabled :deep(.van-loading__spinner) {
  color: rgba(26, 28, 28, 0.56) !important;
}
</style>
