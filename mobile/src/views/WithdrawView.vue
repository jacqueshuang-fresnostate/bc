<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { showToast } from 'vant'
import {
  createWithdrawalOrder,
  errorMessage,
  fetchCurrentUserProfile,
  fetchWithdrawalMethods,
  type WithdrawalMethod,
  type WithdrawalMethodType,
} from '../api/user'
import LucideIcon from '../components/mobile/LucideIcon.vue'

type MethodType = WithdrawalMethodType

const router = useRouter()
const profile = ref<any>(null)
const methods = ref<WithdrawalMethod[]>([])
const amount = ref('')
const selectedMethodId = ref<string | null>(null)
const loading = ref(false)
const submitting = ref(false)

const balanceText = computed(() => String(profile.value?.balance || '0.00'))
const enabledMethods = computed(() => methods.value)
const selectedMethod = computed(() => enabledMethods.value.find(item => item.id === selectedMethodId.value) || enabledMethods.value[0] || null)

function methodIcon(type?: MethodType) {
  if (type === 'bankCard') return 'account_balance'
  if (type === 'wechat') return 'chat'
  if (type === 'alipay') return 'qr_code_scanner'
  return 'account_balance_wallet'
}

function methodTitle(item: WithdrawalMethod) {
  if (item.methodType === 'alipay') return '支付宝'
  if (item.methodType === 'wechat') return '微信'
  return item.bankName ? `银行卡 · ${item.bankName}` : '银行卡'
}

function methodDescription(item?: WithdrawalMethod | null) {
  if (!item) return '请先添加收款账户'
  if (item.methodType === 'bankCard') return maskAccount(item.accountNumber)
  return maskEmail(item.accountNumber)
}

function maskAccount(value?: string | null) {
  const text = String(value || '')
  if (text.length <= 4) return text || '-'
  return `**** **** **** ${text.slice(-4)}`
}

function maskEmail(value?: string | null) {
  const text = String(value || '')
  const [name, domain] = text.split('@')
  if (!name || !domain) return maskAccount(text)
  return `${name.slice(0, 4)}***@${domain}`
}

function useMaxAmount() {
  amount.value = balanceText.value
}

async function loadWithdrawData() {
  loading.value = true
  try {
    const [currentProfile, methodRes] = await Promise.all([
      fetchCurrentUserProfile(),
      fetchWithdrawalMethods(),
    ])
    profile.value = currentProfile
    methods.value = methodRes
    selectedMethodId.value = methods.value.find(item => item.isDefault)?.id || enabledMethods.value[0]?.id || null
  } catch (e: unknown) {
    showToast(errorMessage(e, '加载失败'))
  } finally {
    loading.value = false
  }
}

async function submitWithdraw() {
  const amountMinor = amountToMinor(amount.value)
  if (amountMinor <= 0) {
    showToast('请输入提现金额')
    return
  }
  if (!selectedMethod.value) {
    showToast('请选择收款账户')
    router.push('/withdrawal-methods')
    return
  }
  submitting.value = true
  try {
    await createWithdrawalOrder({
      methodId: selectedMethod.value.id,
      amountMinor,
    })
    showToast('提现申请已提交')
    amount.value = ''
    await loadWithdrawData()
  } catch (e: unknown) {
    showToast(errorMessage(e, '提交失败'))
  } finally {
    submitting.value = false
  }
}

function amountToMinor(value: string) {
  const amount = Number(value)
  if (!Number.isFinite(amount)) return 0
  return Math.round(amount * 100)
}

onMounted(loadWithdrawData)
</script>

<template>
  <div class="withdraw-application flex min-h-screen flex-col items-center bg-surface pb-24 text-on-surface antialiased font-body">
    <header class="fixed top-0 z-50 w-full bg-white/80 backdrop-blur-md shadow-[0_4px_40px_0_rgba(140,10,21,0.04)]">
      <div class="flex justify-between items-center px-6 py-4 w-full max-w-lg mx-auto relative">
        <button class="text-primary transition-opacity duration-200 active:scale-95 active:opacity-80" aria-label="返回" @click="router.back()">
          <LucideIcon name="arrow_back" class="h-5 w-5" />
        </button>
        <h1 class="absolute left-1/2 -translate-x-1/2 font-headline text-xl font-black tracking-tighter text-primary">申请提现</h1>
        <button class="text-primary transition-opacity duration-200 active:scale-95 active:opacity-80" aria-label="客服" @click="router.push('/support')">
          <LucideIcon name="support_agent" class="h-5 w-5" />
        </button>
      </div>
      <div class="mx-auto h-[1px] w-full max-w-lg bg-stone-100 opacity-20"></div>
    </header>

    <main class="w-full max-w-lg mx-auto px-6 pt-28 flex-1 flex flex-col gap-8">
      <section class="rounded-xl bg-surface-container-lowest p-6 shadow-[0_4px_40px_0_rgba(140,10,21,0.04)]">
        <p class="mb-2 text-sm text-on-surface-variant font-label">可用余额</p>
        <div class="flex items-baseline gap-2">
          <span class="font-headline text-4xl font-bold text-primary">¥ {{ balanceText }}</span>
        </div>
      </section>

      <section class="rounded-xl bg-surface-container-lowest p-6 shadow-[0_4px_40px_0_rgba(140,10,21,0.04)]">
        <label class="mb-4 block text-sm text-on-surface-variant font-label" for="amount">提现金额</label>
        <div class="relative rounded-lg bg-surface-container-low transition-all duration-200 focus-within:bg-surface-container-lowest focus-within:ring-1 focus-within:ring-outline-variant/20">
          <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-4">
            <span class="font-headline text-2xl text-on-surface-variant">¥</span>
          </div>
          <input id="amount" v-model="amount" class="block w-full border-none bg-transparent py-4 pl-12 pr-4 font-headline text-3xl text-on-surface outline-none placeholder:text-surface-dim focus:ring-0" min="0" placeholder="0.00" step="0.01" type="number" />
        </div>
        <div class="mt-4 flex items-center justify-between text-sm text-on-surface-variant">
          <span>单笔限额：¥100 - ¥50,000</span>
          <button class="font-semibold text-primary transition-opacity active:opacity-80" @click="useMaxAmount">全部提现</button>
        </div>
      </section>

      <section class="rounded-xl bg-surface-container-lowest p-6 shadow-[0_4px_40px_0_rgba(140,10,21,0.04)]">
        <h2 class="mb-4 text-sm text-on-surface-variant font-label">收款账户</h2>
        <van-loading v-if="loading" class="mx-auto block" />
        <van-empty v-else-if="enabledMethods.length === 0" description="暂无可用收款账户" />
        <div v-else class="flex flex-col gap-4">
          <label
            v-for="item in enabledMethods"
            :key="item.id"
            class="relative flex cursor-pointer items-center justify-between rounded-lg bg-surface p-4 transition-all duration-200"
            :class="selectedMethod?.id === item.id ? 'ring-1 ring-primary/20' : 'active:bg-surface-container-low'"
          >
            <input class="sr-only peer" name="method" type="radio" :value="item.id" :checked="selectedMethod?.id === item.id" @change="selectedMethodId = item.id" />
            <div class="flex min-w-0 items-center gap-4">
              <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-[#1677FF]/10 text-[#1677FF]">
                <LucideIcon :name="methodIcon(item.methodType)" class="h-5 w-5" />
              </div>
              <div class="min-w-0">
                <p class="font-body font-semibold text-on-surface">{{ methodTitle(item) }}</p>
                <p class="mt-1 truncate text-xs text-on-surface-variant font-label">{{ methodDescription(item) }}</p>
              </div>
            </div>
            <div class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full" :class="selectedMethod?.id === item.id ? 'border-2 border-primary' : 'border-2 border-surface-dim'">
              <div class="h-2.5 w-2.5 rounded-full" :class="selectedMethod?.id === item.id ? 'bg-primary' : 'bg-transparent'"></div>
            </div>
          </label>
        </div>
      </section>
    </main>

    <div class="fixed bottom-0 z-40 w-full bg-surface-container-lowest px-6 pb-8 pt-4 shadow-[0_-10px_40px_0_rgba(140,10,21,0.06)]">
      <div class="mx-auto w-full max-w-lg">
        <button class="flex w-full items-center justify-center gap-2 rounded-xl bg-gradient-to-br from-primary to-primary-container py-4 font-headline text-lg font-bold !text-on-primary shadow-sm transition-all duration-200 active:scale-[0.98] active:opacity-90 disabled:opacity-60" :disabled="submitting" @click="submitWithdraw">
          确认提现
        </button>
      </div>
    </div>
  </div>
</template>
