<script setup lang="ts">
import LucideIcon from './LucideIcon.vue'

type SettingsListItem = {
  key: string
  label: string
  icon?: string
  value?: string
  hint?: string
  danger?: boolean
  unread?: boolean
}

defineProps<{ items: SettingsListItem[] }>()
const emit = defineEmits<{ select: [item: SettingsListItem] }>()
</script>

<template>
  <section class="settings-list-group rounded-[1.35rem] bg-surface-container-low p-1.5 shadow-[inset_0_0_0_1px_rgba(26,28,28,0.02)]">
    <div class="overflow-hidden rounded-[1.05rem] bg-white">
      <button
        v-for="(item, index) in items"
        :key="item.key"
        class="flex w-full items-center justify-between px-3.5 py-3.5 text-left transition-colors active:bg-stone-50"
        :class="[
          index < items.length - 1 ? 'border-b border-surface-container' : '',
          item.danger ? 'text-primary' : 'text-on-surface',
        ]"
        @click="emit('select', item)"
      >
        <div class="flex min-w-0 items-center gap-3">
          <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-surface-container-low text-on-surface-variant">
            <LucideIcon :name="item.icon || 'chevron_right'" class="h-4 w-4" />
          </span>
          <div class="min-w-0">
            <span class="block truncate text-xs font-semibold">{{ item.label }}</span>
            <span v-if="item.hint" class="mt-0.5 block truncate text-[9px] text-on-surface-variant">{{ item.hint }}</span>
          </div>
        </div>
        <div class="flex shrink-0 items-center gap-2 pl-3 text-on-surface-variant">
          <span v-if="item.unread" class="h-2 w-2 rounded-full bg-red-600 shadow-[0_0_0_3px_rgba(220,38,38,0.12)]"></span>
          <span v-if="item.value" class="text-[9px]">{{ item.value }}</span>
          <span class="text-base leading-none">›</span>
        </div>
      </button>
    </div>
  </section>
</template>
