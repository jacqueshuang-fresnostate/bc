<script setup lang="ts">
type PositionKey = string

defineProps<{
  mode: 'fc3d' | 'grid'
  modelValue?: string
  numberGridValues?: string[]
  fc3dDigits?: string[]
  fc3dPositions?: Array<{ key: PositionKey; label: string; showMiss: boolean }>
  fc3dMissValues?: string[]
  fc3dSelectedNumbers?: Record<PositionKey, string>
}>()

const emit = defineEmits<{ 'update:modelValue': [string]; toggleDigit: [PositionKey, string] }>()
</script>

<template>
  <section v-if="mode === 'fc3d'" class="space-y-6 rounded-xl bg-white p-5">
    <div v-for="position in fc3dPositions" :key="position.key" class="fc3d-number-section">
      <div class="mb-3 flex items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="h-4 w-1 rounded-full bg-[#8c0a15]"></span>
          <h3 class="font-headline font-semibold text-[#1a1c1c]">{{ position.label }}</h3>
        </div>
        <span v-if="position.showMiss" class="text-xs text-[#5a403e]">遗漏</span>
      </div>
      <div class="grid grid-cols-5 gap-3">
        <button
          v-for="(digit, index) in fc3dDigits"
          :key="`${position.key}-${digit}`"
          type="button"
          class="relative flex aspect-square flex-col items-center justify-center rounded-full p-2 !text-white transition-colors"
          :class="fc3dSelectedNumbers?.[position.key] === digit ? 'bg-[#8c0a15] shadow-sm shadow-red-900/20' : 'bg-[#f3f3f3] hover:bg-[#e2e2e2]'"
          @click="emit('toggleDigit', position.key, digit)"
        >
          <span class="font-headline text-lg font-bold">{{ digit }}</span>
          <span v-if="position.showMiss" class="absolute bottom-1 text-[10px] !text-white">{{ fc3dMissValues?.[index] }}</span>
        </button>
      </div>
    </div>
  </section>

  <div v-else style="padding: 12px;">
    <van-grid :column-num="7" :border="false">
      <van-grid-item
        v-for="n in numberGridValues"
        :key="n"
        :text="n"
        :class="{ 'picked': modelValue === n }"
        @click="emit('update:modelValue', n)"
        style="cursor: pointer;"
      />
    </van-grid>
  </div>
</template>

<style scoped>
.picked {
  border-radius: 4px;
  color: #fff !important;
  background: #4f46e5 !important;
}
</style>
