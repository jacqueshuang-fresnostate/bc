<script setup lang="ts">
import { formatOpenedAt, lotteryLogoUrl } from '../../utils/lotteryFormat'
import ResultBalls from './ResultBalls.vue'

defineProps<{ item: any }>()
const emit = defineEmits<{ open: [any] }>()
</script>

<template>
  <article
    class="draw-card"
    role="button"
    tabindex="0"
    @click="emit('open', item)"
    @keydown.enter="emit('open', item)"
  >
    <div class="draw-card__head">
      <div class="draw-card__lottery">
        <img
          v-if="lotteryLogoUrl(item)"
          class="draw-card__lottery-icon"
          :src="lotteryLogoUrl(item)"
          :alt="`${item.lottery_name || item.lottery_code} 标志`"
        />
        <span v-else class="draw-card__lottery-fallback" aria-hidden="true">彩</span>
        <div>
          <h3>{{ item.lottery_name || item.lottery_code }}</h3>
          <p>第{{ item.issue }}期 · {{ formatOpenedAt(item) }}</p>
        </div>
      </div>
      <span class="draw-card__chevron" aria-hidden="true">›</span>
    </div>
    <ResultBalls :item="item" />
  </article>
</template>

<style scoped>
.draw-card {
  border-radius: 12px;
  padding: 20px;
  background: #fff;
  box-shadow: 0 -4px 20px rgba(140, 10, 21, 0.02), 0 8px 24px rgba(26, 28, 28, 0.03);
}

.draw-card__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.draw-card__lottery {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 12px;
}

.draw-card__lottery-icon,
.draw-card__lottery-fallback {
  width: 44px;
  height: 44px;
  flex: 0 0 auto;
  border-radius: 16px;
}

.draw-card__lottery-icon {
  object-fit: cover;
  box-shadow: 0 4px 14px rgba(26, 28, 28, 0.06);
}

.draw-card__lottery-fallback {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #8c0a15;
  background: #ffdad7;
  font-size: 16px;
  font-weight: 900;
}

.draw-card__lottery > div {
  min-width: 0;
}

.draw-card__lottery h3 {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.draw-card h3 {
  margin: 0;
  color: #1a1c1c;
  font-size: 16px;
  font-weight: 800;
  letter-spacing: -0.02em;
}

.draw-card p {
  margin: 4px 0 0;
  color: #5a403e;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.draw-card__chevron {
  color: #8e706d;
  font-size: 26px;
  line-height: 1;
}
</style>
