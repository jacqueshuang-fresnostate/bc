<script setup lang="ts">
const props = withDefaults(defineProps<{
  title: string
  balance?: string | number
  subtitle?: string
}>(), {
  balance: '0.00',
  subtitle: '鸿运当头，理性投注',
})

const emit = defineEmits<{ back: [] }>()
</script>

<template>
  <header class="sticky top-0 z-40 bg-[#f9f9f9]/90 px-6 py-4 shadow-sm shadow-red-900/5 backdrop-blur">
    <div class="mx-auto flex max-w-md items-center justify-between gap-4">
      <div class="flex min-w-0 items-center gap-3">
        <button class="text-2xl text-[#8c0a15]" type="button" @click="emit('back')">←</button>
        <div class="min-w-0">
          <h1 class="truncate font-headline text-xl font-extrabold text-[#8c0a15]">{{ props.title }}</h1>
          <p class="text-xs font-semibold tracking-wider text-[#5a403e]">{{ props.subtitle }}</p>
        </div>
      </div>
      <div class="shrink-0 rounded-full border border-[#e2beba]/40 bg-white px-4 py-2 text-sm font-bold">¥{{ Number(props.balance || 0).toFixed(2) }}</div>
    </div>
  </header>
</template>
