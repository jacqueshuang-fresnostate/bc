<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import LucideIcon from './LucideIcon.vue'
import { useWalletPrivacyStore } from '../../stores/walletPrivacy'

const props = withDefaults(defineProps<{
  balance?: string | number | null
  label?: string
  currency?: string
}>(), {
  balance: '0.00',
  label: '钱包',
  currency: '¥',
})

const walletPrivacyStore = useWalletPrivacyStore()
const { hideWalletAmount } = storeToRefs(walletPrivacyStore)

const normalizedBalance = computed(() => {
  const rawValue = String(props.balance ?? '0.00').replace(/,/g, '').trim()
  const amount = Number(rawValue || 0)
  return Number.isFinite(amount) ? amount.toFixed(2) : '0.00'
})

const displayAmount = computed(() => (
  hideWalletAmount.value ? '••••' : `${props.currency}${normalizedBalance.value}`
))

const toggleLabel = computed(() => (
  hideWalletAmount.value ? '显示钱包金额' : '隐藏钱包金额'
))
</script>

<template>
  <button
    type="button"
    class="wallet-header-amount flex shrink-0 items-center gap-1.5 rounded-full bg-stone-50/70 px-3.5 py-1.5 text-red-800 transition active:scale-95"
    :aria-label="toggleLabel"
    :aria-pressed="hideWalletAmount"
    @click="walletPrivacyStore.toggleWalletAmountHidden()"
  >
    <span v-if="props.label" class="text-sm">{{ props.label }}</span>
    <span class="font-headline text-sm font-semibold tracking-tight">{{ displayAmount }}</span>
    <LucideIcon
      :name="hideWalletAmount ? 'visibility_off' : 'visibility'"
      class="h-3.5 w-3.5 opacity-70"
      :stroke-width="2.5"
    />
  </button>
</template>
