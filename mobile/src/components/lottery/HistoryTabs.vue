<script setup lang="ts">
defineProps<{ activeTab: 'draws' | 'orders' }>()
const emit = defineEmits<{ change: ['draws' | 'orders'] }>()
</script>

<template>
  <div class="history-tabs" :class="{ 'history-tabs--records': activeTab === 'orders' }" role="tablist" aria-label="开奖与注单">
    <button
      class="history-tab"
      :class="{ active: activeTab === 'draws' }"
      type="button"
      role="tab"
      :aria-selected="activeTab === 'draws'"
      @click="emit('change', 'draws')"
    >
      开奖结果
    </button>
    <button
      class="history-tab"
      :class="{ active: activeTab === 'orders' }"
      type="button"
      role="tab"
      :aria-selected="activeTab === 'orders'"
      @click="emit('change', 'orders')"
    >
      我的注单
    </button>
  </div>
</template>

<style scoped>
.history-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 24px;
  border-radius: 999px;
  padding: 6px;
  background: #f3f3f3;
}

.history-tabs--records {
  max-width: 384px;
  margin-right: auto;
  margin-left: auto;
  background: #f3f3f3;
}

.history-tab {
  flex: 1;
  border: 0;
  border-radius: 999px;
  padding: 10px 12px;
  color: #5a403e;
  background: transparent;
  font-size: 14px;
  font-weight: 700;
  transition: color 0.2s ease, background 0.2s ease, box-shadow 0.2s ease, transform 0.2s ease;
}

.history-tab.active {
  color: #8c0a15;
  background: #fff;
  box-shadow: 0 2px 8px rgba(140, 10, 21, 0.06);
}

.history-tabs--records .history-tab.active {
  box-shadow: 0 2px 12px rgba(140, 10, 21, 0.08);
}

.history-tab:active {
  transform: scale(0.98);
}
</style>
