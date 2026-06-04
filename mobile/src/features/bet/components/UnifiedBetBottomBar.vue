<script setup lang="ts">
const props = withDefaults(defineProps<{
  selectedCount: number
  cartCount: number
  totalAmount: number
  editing?: boolean
  canAdd?: boolean
  canSubmit?: boolean
  addText?: string
  submitText?: string
}>(), {
  editing: false,
  canAdd: true,
  canSubmit: true,
  addText: '加入篮子',
  submitText: '立即投注',
})

const emit = defineEmits<{ add: []; submit: []; edit: [] }>()
</script>

<template>
  <section class="fixed bottom-0 left-0 right-0 z-50 bg-[#f9f9f9] px-6 pb-[calc(1.75rem+env(safe-area-inset-bottom))] pt-5 shadow-[0_-8px_40px_rgba(140,10,21,0.06)]">
    <div class="mx-auto max-w-md">
      <div class="mb-5 flex items-center justify-between">
        <div class="flex items-baseline gap-2">
          <span class="text-sm text-[#5a403e]">已选</span>
          <span class="font-headline text-2xl font-extrabold text-[#8c0a15]">{{ props.selectedCount }}</span>
          <span class="text-sm text-[#5a403e]">注</span>
          <span class="mx-2 text-[#e2e2e2]">|</span>
          <span class="text-sm text-[#5a403e]">共</span>
          <span class="font-headline text-2xl font-extrabold text-[#735c00]">¥{{ Number(props.totalAmount || 0).toFixed(2) }}</span>
        </div>
        <button class="rounded-full bg-[#f3f3f3] px-3 py-2 text-xs text-[#5a403e] disabled:opacity-40" type="button" :disabled="props.cartCount <= 0" @click="emit('edit')">编辑单据</button>
      </div>
      <div class="flex gap-4">
        <button class="h-14 flex-1 rounded-2xl border-2 border-[#8c0a15] bg-white text-lg font-bold text-[#8c0a15] disabled:opacity-40" type="button" :disabled="!props.canAdd" @click="emit('add')">{{ props.addText }}</button>
        <button class="unified-submit-button h-14 flex-[2] rounded-2xl text-lg font-bold shadow-lg shadow-red-900/20 disabled:opacity-60" type="button" :disabled="!props.canSubmit" @click="emit('submit')">{{ props.editing ? '确认修改' : props.submitText }}</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.unified-submit-button {
  color: #fff;
  background: #af2829;
}
</style>
