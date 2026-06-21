<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { showToast } from 'vant'
import {
  errorMessage,
  type LedgerEntryKind,
} from '../api/user'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { useMobileUserDataStore } from '../stores/mobileUserData'
import { formatDateTime } from '../utils/lotteryFormat'

const router = useRouter()
const userDataStore = useMobileUserDataStore()
const {
  profile,
  ledgerEntries: entries,
  ledgerEntriesHasMore,
  loadingProfile,
  loadingLedgerEntries,
} = storeToRefs(userDataStore)
const refreshing = ref(false)

const balanceText = computed(() => profile.value?.balance || '0.00')
const incomeMinor = computed(() => entries.value.filter(item => item.amountMinor > 0).reduce((sum, item) => sum + item.amountMinor, 0))
const expenseMinor = computed(() => Math.abs(entries.value.filter(item => item.amountMinor < 0).reduce((sum, item) => sum + item.amountMinor, 0)))
const latestEntries = computed(() => entries.value)
const loading = computed(() => Boolean(
  (loadingProfile.value && !profile.value)
    || (loadingLedgerEntries.value && !entries.value.length),
))

const kindTextMap: Record<LedgerEntryKind, string> = {
  agentRebateWithdrawal: '返利提现',
  manualAdjustment: '财务调整',
  orderDebit: '投注扣款',
  orderRefund: '投注退款',
  payoutCredit: '派奖入账',
  rechargeBonusCredit: '充值赠送',
  rechargeCredit: '充值入账',
  rechargeRebateCredit: '充值返利',
  withdrawalFreeze: '提现冻结',
  withdrawalPayout: '提现打款',
  withdrawalReject: '提现驳回',
  groupBuyDebit: '合买认购',
  groupBuyRefund: '合买退款',
}

function kindText(kind: LedgerEntryKind) {
  return kindTextMap[kind] || '资金流水'
}

function kindIcon(kind: LedgerEntryKind) {
  if (kind === 'rechargeCredit' || kind === 'rechargeRebateCredit' || kind === 'rechargeBonusCredit') return 'output_circle'
  if (kind === 'payoutCredit' || kind === 'orderRefund' || kind === 'withdrawalReject' || kind === 'groupBuyRefund') return 'add_circle'
  if (kind === 'agentRebateWithdrawal' || kind === 'orderDebit' || kind === 'withdrawalFreeze' || kind === 'withdrawalPayout' || kind === 'groupBuyDebit') return 'payments'
  return 'account_balance_wallet'
}

function amountText(value: number) {
  const amount = formatMinorAmount(Math.abs(value))
  if (value > 0) return `+¥${amount}`
  if (value < 0) return `-¥${amount}`
  return `¥${amount}`
}

function amountTone(value: number) {
  if (value > 0) return 'text-emerald-600'
  if (value < 0) return 'text-primary'
  return 'text-on-surface'
}

function formatMinorAmount(value: number) {
  return (Number(value || 0) / 100).toFixed(2)
}

function balanceAfterText(value: number) {
  return `余额 ¥${formatMinorAmount(value)}`
}

async function loadLedger(options: { force?: boolean; silent?: boolean; append?: boolean } = {}) {
  try {
    await Promise.all([
      userDataStore.loadProfile(options),
      userDataStore.loadLedgerEntries(options),
    ])
  } catch (error) {
    showToast(errorMessage(error, '加载资金流水失败'))
  } finally {
    refreshing.value = false
  }
}

async function refreshLedger() {
  refreshing.value = true
  await loadLedger({ force: true, silent: true })
}

async function loadMoreLedger() {
  if (loadingLedgerEntries.value || !ledgerEntriesHasMore.value) return
  await loadLedger({ append: true })
}

onMounted(() => loadLedger())
</script>

<template>
  <div class="account-ledger min-h-screen bg-surface pb-10 text-on-surface font-body">
    <header class="mobile-safe-header fixed top-0 left-0 z-50 w-full bg-white/85 shadow-[0_4px_40px_0_rgba(140,10,21,0.04)] backdrop-blur-md">
      <div class="mx-auto flex h-16 w-full max-w-lg items-center justify-between px-5">
        <button class="flex h-10 w-10 items-center justify-center rounded-full text-primary transition active:scale-95 active:bg-red-50" aria-label="返回" type="button" @click="router.back()">
          <LucideIcon name="arrow_back" class="h-5 w-5" />
        </button>
        <h1 class="font-headline text-lg font-black tracking-tight text-primary">资金流水</h1>
        <button class="flex h-10 w-10 items-center justify-center rounded-full text-primary transition active:scale-95 active:bg-red-50" aria-label="刷新" type="button" @click="refreshLedger">
          <LucideIcon name="refresh" class="h-5 w-5" />
        </button>
      </div>
    </header>

    <main class="mobile-safe-main-top mx-auto flex w-full max-w-lg flex-col gap-4 px-4">
      <section class="rounded-2xl bg-white p-5 shadow-[0_12px_40px_rgba(140,10,21,0.07)]">
        <div class="flex items-center justify-between gap-4">
          <div>
            <p class="text-xs font-semibold text-on-surface-variant">当前余额</p>
            <p class="mt-2 font-headline text-3xl font-black tracking-tight text-primary">¥{{ balanceText }}</p>
          </div>
          <div class="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-red-50 text-primary">
            <LucideIcon name="account_balance_wallet" class="h-6 w-6" />
          </div>
        </div>
        <div class="mt-5 grid grid-cols-3 gap-2">
          <div class="rounded-xl bg-surface-container-low px-3 py-3">
            <p class="text-[10px] text-on-surface-variant">流水笔数</p>
            <p class="mt-1 text-sm font-bold text-on-surface">{{ entries.length }}</p>
          </div>
          <div class="rounded-xl bg-emerald-50 px-3 py-3">
            <p class="text-[10px] text-emerald-700">入账合计</p>
            <p class="mt-1 text-sm font-bold text-emerald-700">¥{{ formatMinorAmount(incomeMinor) }}</p>
          </div>
          <div class="rounded-xl bg-red-50 px-3 py-3">
            <p class="text-[10px] text-primary">支出合计</p>
            <p class="mt-1 text-sm font-bold text-primary">¥{{ formatMinorAmount(expenseMinor) }}</p>
          </div>
        </div>
      </section>

      <section class="rounded-2xl bg-white px-2 py-2 shadow-[0_12px_40px_rgba(140,10,21,0.05)]">
        <div class="flex items-center justify-between px-3 py-2">
          <h2 class="text-sm font-bold text-on-surface">最近流水</h2>
          <span v-if="refreshing" class="text-[11px] text-on-surface-variant">刷新中</span>
        </div>

        <div v-if="loading" class="flex justify-center py-12">
          <van-loading color="#af2829" />
        </div>
        <van-empty v-else-if="latestEntries.length === 0" description="暂无资金流水" />
        <div v-else class="flex flex-col">
          <article
            v-for="entry in latestEntries"
            :key="entry.id"
            class="flex gap-3 border-t border-surface-container px-3 py-4 first:border-t-0"
          >
            <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-surface-container-low text-on-surface-variant">
              <LucideIcon :name="kindIcon(entry.kind)" class="h-5 w-5" />
            </div>
            <div class="min-w-0 flex-1">
              <div class="flex min-w-0 items-start justify-between gap-3">
                <div class="min-w-0">
                  <p class="truncate text-sm font-bold text-on-surface">{{ kindText(entry.kind) }}</p>
                  <p class="mt-1 text-[11px] leading-relaxed text-on-surface-variant">{{ entry.description || '资金变动' }}</p>
                </div>
                <div class="shrink-0 text-right">
                  <p class="font-headline text-base font-black tracking-tight" :class="amountTone(entry.amountMinor)">{{ amountText(entry.amountMinor) }}</p>
                  <p class="mt-1 text-[10px] text-on-surface-variant">{{ balanceAfterText(entry.balanceAfterMinor) }}</p>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px] text-on-surface-variant">
                <span>{{ formatDateTime(entry.createdAt) }}</span>
              </div>
            </div>
          </article>
          <button
            v-if="ledgerEntriesHasMore"
            type="button"
            class="mx-3 my-3 rounded-xl bg-red-50 px-4 py-3 text-xs font-bold text-primary active:scale-[0.99] disabled:opacity-60"
            :disabled="loadingLedgerEntries"
            @click="loadMoreLedger"
          >
            {{ loadingLedgerEntries ? '加载中...' : '加载更多流水' }}
          </button>
          <p v-else class="py-3 text-center text-[11px] text-on-surface-variant">已加载全部流水</p>
        </div>
      </section>
    </main>
  </div>
</template>
