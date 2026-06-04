<script setup lang="ts">
import type { DynamicBetPlay } from '../../features/bet/dynamic/types'

defineProps<{ selectedPlayItem: DynamicBetPlay | null; numbers: string }>()
</script>

<template>
  <van-cell-group inset style="margin-top: 12px;">
    <van-cell title="投注选项" :value="selectedPlayItem?.name || numbers || selectedPlayItem?.option_value || ''" />
  </van-cell-group>
</template>
