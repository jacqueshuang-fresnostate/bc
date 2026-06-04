<script setup lang="ts">
import { drawNumbers, formatNumbers, isAccentBall } from '../../utils/lotteryFormat'

defineProps<{ item: any; keyPrefix?: string }>()
</script>

<template>
  <div v-if="drawNumbers(item).length" class="result-balls" :aria-label="formatNumbers(item)">
    <span
      v-for="(number, index) in drawNumbers(item)"
      :key="`${keyPrefix || item.id}-${index}-${number}`"
      class="result-ball"
      :class="{ 'result-ball--accent': isAccentBall(index, item) }"
    >
      {{ number }}
    </span>
  </div>
  <div v-else class="pending-result">{{ formatNumbers(item) }}</div>
</template>

<style scoped>
.result-balls {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.result-ball {
  display: inline-flex;
  width: 40px;
  height: 40px;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  color: #fff;
  background: #8c0a15;
  box-shadow: 0 6px 14px rgba(140, 10, 21, 0.2);
  font-family: "Plus Jakarta Sans", Avenir Next, ui-sans-serif, system-ui, sans-serif;
  font-size: 18px;
  font-weight: 900;
  letter-spacing: -0.04em;
}

.result-ball--accent {
  color: #fff;
  background: #735c00;
  box-shadow: 0 6px 14px rgba(115, 92, 0, 0.18);
}

.pending-result {
  display: inline-flex;
  border-radius: 999px;
  padding: 8px 12px;
  color: #8e706d;
  background: #f3f3f3;
  font-size: 13px;
  font-weight: 700;
}

@media (max-width: 360px) {
  .result-balls {
    gap: 8px;
  }

  .result-ball {
    width: 36px;
    height: 36px;
    font-size: 16px;
  }
}
</style>
