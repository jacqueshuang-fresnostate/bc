<script setup lang="ts">
import type { DynamicBetPlay } from '../dynamic/types'

const props = defineProps<{
  plays: DynamicBetPlay[]
  selectedCode: string
}>()

const emit = defineEmits<{ select: [DynamicBetPlay] }>()
</script>

<template>
  <section class="grid gap-3">
    <button
      v-for="play in props.plays"
      :key="play.code"
      type="button"
      class="rounded-2xl px-5 py-4 text-left text-sm font-semibold transition active:scale-[0.99]"
      :class="props.selectedCode === play.code ? 'dynamic-play-tab--selected shadow-sm shadow-red-900/20' : 'bg-white text-[#5a403e]'"
      @click="emit('select', play)"
    >
      <span class="block font-bold">{{ play.name }}</span>
      <span v-if="play.simple_description" class="mt-1 block text-xs opacity-80">{{ play.simple_description }}</span>
    </button>
  </section>
</template>

<style scoped>
.dynamic-play-tab--selected {
  color: #fff;
  background: #af2829;
}
</style>
