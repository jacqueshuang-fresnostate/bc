<script setup lang="ts">
defineProps<{ lotteryGroupFilters: Array<{ label: string; code: string }>; activeGroupCode: string }>()
const emit = defineEmits<{ select: [string] }>()
</script>

<template>
  <div class="category-filter" aria-label="开奖分组筛选">
    <button
      v-for="group in lotteryGroupFilters"
      :key="group.code"
      :class="{ active: activeGroupCode === group.code }"
      type="button"
      @click="emit('select', group.code)"
    >
      {{ group.label }}
    </button>
  </div>
</template>

<style scoped>
.category-filter {
  display: flex;
  gap: 8px;
  margin-bottom: 24px;
  overflow-x: auto;
  padding-bottom: 8px;
  scrollbar-width: none;
}

.category-filter::-webkit-scrollbar {
  display: none;
}

.category-filter button {
  flex: 0 0 auto;
  border: 0;
  border-radius: 999px;
  padding: 7px 16px;
  color: #5a403e;
  background: #e2e2e2;
  font-size: 12px;
  font-weight: 700;
  white-space: nowrap;
  transition: color 0.2s ease, background 0.2s ease, transform 0.2s ease;
}

.category-filter button.active {
  color: #8f0d17;
  background: #ffdad7;
}

.category-filter button:active {
  transform: scale(0.96);
}
</style>
