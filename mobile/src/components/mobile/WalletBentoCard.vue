<script setup lang="ts">
import LucideIcon from './LucideIcon.vue'

defineProps<{
  balance: string
  usdtBalance?: string
}>()

const emit = defineEmits<{ deposit: []; withdraw: [] }>()
</script>

<template>
  <section class="wallet-bento-card relative overflow-hidden rounded-[1.35rem] bg-gradient-to-br from-[#9d101c] via-[#b71624] to-[#c9373a] p-5 text-white shadow-[0_16px_32px_rgba(156,16,28,0.25)]">
    <div class="relative z-10">
      <div class="mb-8 flex items-start justify-between">
        <span class="text-[10px] font-semibold uppercase tracking-[0.18em] text-white/85">账户总资产 (CNY)</span>
        <LucideIcon name="account_balance_wallet" class="h-5 w-5 text-white/90" />
      </div>
      <div class="mb-8">
        <h2 class="mb-1 font-headline text-[2rem] font-extrabold leading-none tracking-tight">¥ {{ balance }}</h2>
        <div class="flex items-center gap-2 text-xs text-white/90">
          <LucideIcon name="currency_bitcoin" class="h-3.5 w-3.5" />
          <span class="font-label">{{ usdtBalance || '0.00 USDT' }}</span>
        </div>
      </div>
      <div class="flex gap-3">
        <button class="flex flex-1 items-center justify-center gap-2 rounded-lg bg-white py-2.5 text-sm font-bold !text-primary transition-all active:scale-95" @click="emit('deposit')">
          <LucideIcon name="add_circle" class="h-4 w-4" />
          <span>充值</span>
        </button>
        <button class="flex flex-1 items-center justify-center gap-2 rounded-lg border border-[#d84a4b]/70 bg-[#af2829] py-2.5 text-sm font-bold !text-white shadow-[0_8px_18px_rgba(95,10,18,0.22)] transition-all active:scale-95" @click="emit('withdraw')">
          <LucideIcon name="output_circle" class="h-4 w-4 !text-white" />
          <span class="!text-white">提现</span>
        </button>
      </div>
    </div>
    <div class="absolute -bottom-10 -right-10 h-40 w-40 rounded-full bg-white/10 blur-3xl"></div>
    <div class="absolute -left-12 top-1/2 h-28 w-28 rounded-full bg-black/10 blur-3xl"></div>
  </section>
</template>

<style scoped>
.wallet-bento-card {
  position: relative;
  overflow: hidden;
  border-radius: 1.35rem;
  background: #af2829;
  background: linear-gradient(135deg, #9d101c 0%, #b71624 52%, #c9373a 100%);
  padding: 1.25rem;
  color: #ffffff;
  box-shadow: 0 16px 32px rgba(156, 16, 28, 0.25);
}

.wallet-bento-card :deep(svg),
.wallet-bento-card span,
.wallet-bento-card h2,
.wallet-bento-card div {
  color: inherit;
}

.wallet-bento-card button {
  min-height: 2.75rem;
  border-radius: 0.85rem;
}

.wallet-bento-card button:first-child {
  background: #ffffff;
  color: #8c0a15 !important;
}

.wallet-bento-card button:first-child :deep(svg),
.wallet-bento-card button:first-child span {
  color: #8c0a15 !important;
}

.wallet-bento-card button:last-child {
  border: 1px solid rgba(216, 74, 75, 0.7);
  background: #af2829;
  color: #ffffff !important;
  box-shadow: 0 8px 18px rgba(95, 10, 18, 0.22);
}

.wallet-bento-card button:last-child :deep(svg),
.wallet-bento-card button:last-child span {
  color: #ffffff !important;
}
</style>
