<script setup lang="ts">
import LucideIcon from './LucideIcon.vue'

type QuickActionItem = {
  key: string
  title: string
  subtitle?: string
  icon?: string
  tone?: 'primary' | 'secondary' | 'neutral'
}

defineProps<{ items: QuickActionItem[] }>()
const emit = defineEmits<{ select: [item: QuickActionItem] }>()

function toneClass(tone?: QuickActionItem['tone']) {
  if (tone === 'secondary') return 'bg-amber-100 text-amber-900'
  if (tone === 'neutral') return 'bg-surface-container text-on-surface-variant'
  return 'bg-red-50 text-primary'
}
</script>

<template>
  <div class="quick-action-grid grid grid-cols-2 gap-3">
    <button
      v-for="item in items"
      :key="item.key"
      class="group flex min-h-[6.6rem] flex-col justify-between rounded-2xl border border-transparent bg-white p-3.5 text-left shadow-[0_8px_22px_rgba(26,28,28,0.04)] transition-all active:scale-[0.98]"
      @click="emit('select', item)"
    >
      <div class="flex h-9 w-9 items-center justify-center rounded-xl" :class="toneClass(item.tone)">
        <LucideIcon :name="item.icon || 'apps'" class="h-5 w-5" />
      </div>
      <div>
        <p class="text-[13px] font-bold leading-tight text-on-surface">{{ item.title }}</p>
        <p v-if="item.subtitle" class="mt-1 text-[9px] text-on-surface-variant">{{ item.subtitle }}</p>
      </div>
      <span class="self-end text-base leading-none text-primary transition-transform group-active:translate-x-1">›</span>
    </button>
  </div>
</template>
