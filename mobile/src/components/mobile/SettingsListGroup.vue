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
  badgeCount?: number
}

defineProps<{ items: SettingsListItem[] }>()
const emit = defineEmits<{ select: [item: SettingsListItem] }>()

function badgeContent(item: SettingsListItem) {
  const count = Math.max(0, Number(item.badgeCount || 0))
  if (!count) return ''
  return count > 99 ? '99+' : String(count)
}
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
          <van-badge
            v-if="badgeContent(item)"
            class="settings-list-group__badge"
            :content="badgeContent(item)"
          >
            <span class="settings-list-group__badge-anchor"></span>
          </van-badge>
          <span v-else-if="item.unread" class="h-2 w-2 rounded-full bg-red-600 shadow-[0_0_0_3px_rgba(220,38,38,0.12)]"></span>
          <span v-if="item.value" class="text-[9px]">{{ item.value }}</span>
          <span class="text-base leading-none">›</span>
        </div>
      </button>
    </div>
  </section>
</template>

<style scoped>
.settings-list-group {
  border-radius: 1.35rem;
  background: #f3f3f3;
  padding: 0.375rem;
  box-shadow: inset 0 0 0 1px rgba(26, 28, 28, 0.02);
}

.settings-list-group > div {
  overflow: hidden;
  border-radius: 1.05rem;
  background: #ffffff;
}

.settings-list-group button {
  min-height: 4rem;
  background: #ffffff;
}

.settings-list-group button + button {
  border-top: 1px solid #eeeeee;
}

.settings-list-group button span {
  color: inherit;
}

.settings-list-group button > div:first-child > span:first-child {
  width: 2rem;
  height: 2rem;
  border-radius: 0.55rem;
  background: #f3f3f3;
  color: #5a403e;
}

.settings-list-group__badge-anchor {
  display: block;
  width: 1px;
  height: 1px;
}

:deep(.settings-list-group__badge .van-badge) {
  min-width: 18px;
  height: 18px;
  border: 2px solid #fff;
  background: #dc2626;
  box-shadow: 0 4px 10px rgba(220, 38, 38, 0.22);
  font-size: 10px;
  font-weight: 900;
  line-height: 14px;
}
</style>
