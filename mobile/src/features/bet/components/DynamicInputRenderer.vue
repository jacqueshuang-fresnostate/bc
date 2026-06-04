<script setup lang="ts">
import { computed } from 'vue'
import NumberGridPicker from '../../../components/bet/NumberGridPicker.vue'
import FixedOptionDisplay from '../../../components/bet/FixedOptionDisplay.vue'
import ManualNumberInput from '../../../components/bet/ManualNumberInput.vue'
import PositionGridInput from './PositionGridInput.vue'
import OptionGroupPicker from './OptionGroupPicker.vue'
import type { DynamicBetPlay } from '../dynamic/types'

const props = defineProps<{
  play: DynamicBetPlay | null
  numbers: string
  selections: Record<string, string[]>
}>()

const emit = defineEmits<{
  'update:numbers': [string]
  togglePosition: [string, string]
  selectAllPosition: [string]
  selectPresetPosition: [string, string[]]
  clearPosition: [string]
  toggleOption: [string, string]
}>()

// 动态输入渲染边界：玩法 input_mode 决定具体输入组件，父页面只处理统一的号码/位置事件。
const inputMode = computed(() => props.play?.input_mode || 'text')
</script>

<template>
  <!-- option_groups 优先于 input_mode，适合大小单双、龙虎和、色波等配置化选项玩法。 -->
  <OptionGroupPicker
    v-if="props.play?.option_groups.length || props.play?.option_groups_error"
    :groups="props.play.option_groups"
    :selections="props.selections"
    :error="props.play.option_groups_error"
    @toggle="(groupKey, value) => emit('toggleOption', groupKey, value)"
  />
  <!-- 位置宫格需要把每个位置的选择事件原样抛回投注引擎，避免组件内部持有投注草稿状态。 -->
  <PositionGridInput
    v-else-if="inputMode === 'position-grid' && props.play"
    :positions="props.play.positions"
    :digits="props.play.digits"
    :selections="props.selections"
    :position-grid-kind="props.play.position_grid_kind"
    :max-select-per-position="props.play.max_select_per_position"
    @toggle="(positionKey, digit) => emit('togglePosition', positionKey, digit)"
    @select-all="emit('selectAllPosition', $event)"
    @select-preset="(positionKey, values) => emit('selectPresetPosition', positionKey, values)"
    @clear-position="emit('clearPosition', $event)"
  />
  <!-- 普通号码宫格和手动输入都使用同一个 numbers 双向值，提交前由引擎统一校验注数。 -->
  <NumberGridPicker
    v-else-if="inputMode === 'number-grid' && props.play"
    :model-value="props.numbers"
    mode="grid"
    :number-grid-values="props.play.number_grid_values"
    @update:model-value="emit('update:numbers', $event)"
  />
  <!-- 固定选项玩法只展示服务端配置的固定号码，不允许用户在前端改写 option_value。 -->
  <FixedOptionDisplay
    v-else-if="inputMode === 'fixed-option'"
    :selected-play-item="props.play as any"
    :numbers="props.numbers"
  />
  <ManualNumberInput v-else :model-value="props.numbers" @update:model-value="emit('update:numbers', $event)" />
</template>
