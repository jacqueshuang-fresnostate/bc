<script setup lang="ts">
import type { DynamicBetOptionGroup } from '../dynamic/types'

const props = defineProps<{
  groups: DynamicBetOptionGroup[]
  selections: Record<string, string[]>
  error?: string | null
}>()

const emit = defineEmits<{ toggle: [string, string] }>()

function isSelected(groupKey: string, value: string) {
  return (props.selections[groupKey] || []).includes(value)
}

function maxReached(group: DynamicBetOptionGroup) {
  return (props.selections[group.key] || []).length >= group.max_select_count
}

function isOptionDisabled(group: DynamicBetOptionGroup, value: string, disabled: boolean) {
  return disabled || (!isSelected(group.key, value) && maxReached(group))
}

function requirementText(group: DynamicBetOptionGroup) {
  if (group.min_select_count === group.max_select_count) return `请选择 ${group.min_select_count} 项`
  return `请选择 ${group.min_select_count}-${group.max_select_count} 项`
}
</script>

<template>
  <section class="rounded-[28px] bg-[#fffaf7] p-5 shadow-sm shadow-red-900/5">
    <div v-if="props.error" class="rounded-2xl border border-orange-200 bg-orange-50 px-4 py-3 text-sm font-semibold text-orange-700">
      {{ props.error }}
    </div>
    <div v-for="group in props.groups" :key="group.key" class="option-group">
      <div class="mb-3 flex items-center justify-between gap-3">
        <h3 class="text-base font-extrabold text-[#1a1c1c]">{{ group.label }}</h3>
        <span class="rounded-full bg-[#f7eee9] px-3 py-1 text-xs font-bold text-[#8c0a15]">{{ requirementText(group) }}</span>
      </div>
      <div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
        <button
          v-for="option in group.options"
          :key="option.value"
          class="option-button"
          :class="{
            selected: isSelected(group.key, option.value),
            unavailable: isOptionDisabled(group, option.value, option.disabled),
          }"
          type="button"
          :disabled="isOptionDisabled(group, option.value, option.disabled)"
          @click="emit('toggle', group.key, option.value)"
        >
          <strong>{{ option.label }}</strong>
          <span v-if="option.description">{{ option.description }}</span>
          <em v-if="option.odds">赔率 {{ option.odds }}</em>
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.option-group + .option-group {
  margin-top: 20px;
}

.option-button {
  min-height: 68px;
  border: 1px solid #eaded8;
  border-radius: 18px;
  background: #fdfaf8;
  color: #342220;
  padding: 10px 12px;
  text-align: left;
  transition: transform 180ms ease-out, border-color 180ms ease-out, background 180ms ease-out;
}

.option-button strong,
.option-button span,
.option-button em {
  display: block;
}

.option-button strong {
  font-size: 16px;
  line-height: 1.2;
}

.option-button span {
  margin-top: 3px;
  color: #735c58;
  font-size: 12px;
}

.option-button em {
  margin-top: 5px;
  color: #7a5d00;
  font-size: 12px;
  font-style: normal;
  font-weight: 700;
}

.option-button:not(:disabled):active {
  transform: scale(0.98);
}

.option-button.selected {
  border-color: #af2829;
  background: #fff0ed;
  color: #8c0a15;
  box-shadow: inset 0 0 0 1px #af2829;
}

.option-button.unavailable {
  opacity: 0.46;
}
</style>
