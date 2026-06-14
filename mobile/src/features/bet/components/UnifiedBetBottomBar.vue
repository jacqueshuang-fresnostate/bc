<script setup lang="ts">
const props = withDefaults(defineProps<{
  selectedCount: number
  cartCount: number
  totalAmount: number
  editing?: boolean
  canAdd?: boolean
  canSubmit?: boolean
  submitting?: boolean
  addText?: string
  submitText?: string
  groupBuyMode?: boolean
  groupBuySelfShares?: number
  groupBuyShareCount?: number
  groupBuyPaymentAmount?: string | number
}>(), {
  editing: false,
  canAdd: true,
  canSubmit: true,
  submitting: false,
  addText: '加入购彩篮',
  submitText: '立即投注',
  groupBuyMode: false,
  groupBuySelfShares: 0,
  groupBuyShareCount: 0,
  groupBuyPaymentAmount: '0.00',
})

const emit = defineEmits<{ add: []; submit: []; edit: [] }>()

function amountText(value: string | number) {
  const amount = Number(value || 0)
  return Number.isFinite(amount) ? amount.toFixed(2) : '0.00'
}
</script>

<template>
  <section class="bet-bottom-bar fixed bottom-0 left-0 right-0 z-50 bg-[#f9f9f9] px-6 pb-[calc(1.75rem+env(safe-area-inset-bottom))] pt-5 shadow-[0_-8px_40px_rgba(140,10,21,0.06)]" :class="{ 'bet-bottom-bar--group-buy': props.groupBuyMode }">
    <div class="mx-auto max-w-md">
      <div v-if="props.groupBuyMode" class="mb-3 grid grid-cols-3 gap-2 rounded-2xl bg-[#fff4dc] p-2.5 text-center">
        <div>
          <span class="block text-[10px] font-extrabold text-[#8e706d]">方案总额</span>
          <strong class="mt-0.5 block font-headline text-sm font-black text-[#735c00]">¥{{ amountText(props.totalAmount) }}</strong>
        </div>
        <div>
          <span class="block text-[10px] font-extrabold text-[#8e706d]">自购份数</span>
          <strong class="mt-0.5 block font-headline text-sm font-black text-[#8c0a15]">{{ props.groupBuySelfShares }}/{{ props.groupBuyShareCount }}份</strong>
        </div>
        <div>
          <span class="block text-[10px] font-extrabold text-[#8e706d]">需支付</span>
          <strong class="mt-0.5 block font-headline text-sm font-black text-[#8c0a15]">¥{{ amountText(props.groupBuyPaymentAmount) }}</strong>
        </div>
      </div>
      <div class="flex items-center justify-between" :class="props.groupBuyMode ? 'mb-3' : 'mb-5'">
        <div class="flex items-baseline gap-2">
          <span class="text-sm text-[#5a403e]">{{ props.groupBuyMode ? '合买' : '已选' }}</span>
          <span class="font-headline text-2xl font-extrabold text-[#8c0a15]">{{ props.selectedCount }}</span>
          <span class="text-sm text-[#5a403e]">注</span>
          <span class="mx-2 text-[#e2e2e2]">|</span>
          <span class="text-sm text-[#5a403e]">{{ props.groupBuyMode ? '方案' : '共' }}</span>
          <span class="font-headline text-2xl font-extrabold text-[#735c00]">¥{{ Number(props.totalAmount || 0).toFixed(2) }}</span>
        </div>
        <button class="rounded-full bg-[#f3f3f3] px-3 py-2 text-xs text-[#5a403e] disabled:opacity-40" type="button" :disabled="props.cartCount <= 0 || props.submitting" @click="emit('edit')">编辑单据</button>
      </div>
      <div class="flex gap-4">
        <button class="h-14 flex-1 rounded-2xl border-2 border-[#8c0a15] bg-white text-lg font-bold text-[#8c0a15] disabled:opacity-40" type="button" :disabled="!props.canAdd || props.submitting" @click="emit('add')">{{ props.addText }}</button>
        <button class="unified-submit-button h-14 flex-[2] rounded-2xl text-lg font-bold shadow-lg shadow-red-900/20 disabled:opacity-60" type="button" :disabled="!props.canSubmit || props.submitting" @click="emit('submit')">{{ props.editing ? '确认修改' : props.submitText }}</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.unified-submit-button {
  color: #fff;
  background: #af2829;
}

.bet-bottom-bar--group-buy {
  padding-top: 14px;
}
</style>
