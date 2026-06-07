<script setup lang="ts">
import { computed } from 'vue'
import type { DynamicBetPosition, DynamicBetPositionSelectLimit, PositionGridKind } from '../dynamic/types'

const props = defineProps<{
  positions: DynamicBetPosition[]
  digits: string[]
  selections: Record<string, string[]>
  positionGridKind?: PositionGridKind
  maxSelectPerPosition?: number | null
  positionSelectLimits?: DynamicBetPositionSelectLimit[]
}>()

const emit = defineEmits<{ toggle: [string, string]; selectAll: [string]; selectPreset: [string, string[]]; clearPosition: [string] }>()

const numericDigits = computed(() => props.digits.every(digit => /^\d$/.test(digit)))
const presetButtons = [
  { label: '大', values: ['5', '6', '7', '8', '9'] },
  { label: '小', values: ['0', '1', '2', '3', '4'] },
  { label: '单', values: ['1', '3', '5', '7', '9'] },
  { label: '双', values: ['0', '2', '4', '6', '8'] },
]

function isDantuoGrid() {
  return props.positionGridKind === 'group3_dantuo' || props.positionGridKind === 'group6_dantuo'
}

function danLimit() {
  return props.positionGridKind === 'group6_dantuo' ? 2 : 1
}

function positionIndex(positionKey: string) {
  return props.positions.findIndex(position => position.key === positionKey)
}

function selectedAt(index: number) {
  const key = props.positions[index]?.key
  return key ? props.selections[key] || [] : []
}

function isDigitSelected(positionKey: string, digit: string) {
  return (props.selections[positionKey] || []).includes(digit)
}

function maxSelectForPosition(positionKey: string) {
  const positionLimit = props.positionSelectLimits?.find(limit => limit.position_key === positionKey)
  if (positionLimit?.max_select_count && positionLimit.max_select_count > 0) return positionLimit.max_select_count
  return props.maxSelectPerPosition && props.maxSelectPerPosition > 0 ? props.maxSelectPerPosition : null
}

function isDigitDisabled(positionKey: string, digit: string) {
  if (isDigitSelected(positionKey, digit)) return false
  const maxSelect = maxSelectForPosition(positionKey)
  if (maxSelect && (props.selections[positionKey] || []).length >= maxSelect) return true
  if (!isDantuoGrid()) return false
  const index = positionIndex(positionKey)
  if (index === 0) return selectedAt(1).includes(digit) || selectedAt(0).length >= danLimit()
  if (index === 1) return selectedAt(0).includes(digit)
  return false
}

function selectPreset(positionKey: string, values: string[]) {
  const availableValues = values.filter(value => props.digits.includes(value) && !isDigitDisabled(positionKey, value))
  emit('selectPreset', positionKey, availableValues)
}
</script>

<template>
  <section class="space-y-5 rounded-[24px] bg-white p-4 shadow-sm shadow-red-900/5">
    <div v-for="position in props.positions" :key="position.key" class="space-y-2.5">
      <div class="flex items-center justify-between">
        <span class="border-l-4 border-[#8c0a15] pl-3 text-sm font-bold text-[#1a1c1c]">{{ position.label }}</span>
        <div class="flex flex-wrap justify-end gap-2">
          <button class="rounded bg-[#f3f3f3] px-2 py-1 text-xs text-[#5a403e]" type="button" @click="emit('selectAll', position.key)">全</button>
          <button class="rounded bg-[#f3f3f3] px-2 py-1 text-xs text-[#5a403e]" type="button" @click="emit('clearPosition', position.key)">清</button>
          <button
            v-for="preset in presetButtons"
            v-if="numericDigits"
            :key="`${position.key}-${preset.label}`"
            class="rounded bg-[#f3f3f3] px-2 py-1 text-xs text-[#5a403e]"
            type="button"
            @click="selectPreset(position.key, preset.values)"
          >
            {{ preset.label }}
          </button>
        </div>
      </div>
      <div class="grid grid-cols-10 justify-items-center gap-1.5">
        <button
          v-for="digit in props.digits"
          :key="`${position.key}-${digit}`"
          type="button"
          class="flex h-7 w-7 min-w-7 shrink-0 items-center justify-center rounded-[999px] p-0 font-headline text-sm leading-none transition active:scale-95"
          :disabled="isDigitDisabled(position.key, digit)"
          :class="isDigitSelected(position.key, digit) ? 'position-grid-digit--selected font-semibold shadow-md shadow-red-900/20' : isDigitDisabled(position.key, digit) ? 'cursor-not-allowed bg-[#eeeeee] text-[#b7a5a3]' : 'bg-[#f3f3f3] text-[#1a1c1c]'"
          @click="!isDigitDisabled(position.key, digit) && emit('toggle', position.key, digit)"
        >
          {{ digit }}
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.position-grid-digit--selected {
  color: #fff;
  background: #af2829;
}
</style>
