<script setup lang="ts">
import { formatOpenedAt } from '../../utils/lotteryFormat'
import ResultBalls from './ResultBalls.vue'

const props = defineProps<{ selectedLotteryName: string; selectedLotteryItems: any[]; loadingSelectedLottery: boolean; closeDrawHistory?: () => void }>()
const emit = defineEmits<{ close: [] }>()

function closeDrawHistory() {
  props.closeDrawHistory?.()
  emit('close')
}
</script>

<template>
  <section class="selected-lottery-history-sheet" role="dialog" aria-modal="true" :aria-label="`${selectedLotteryName || '彩种'}全部开奖结果`">
    <header class="selected-lottery-history-sheet__header">
      <div>
        <p>全部开奖</p>
        <h2>{{ selectedLotteryName || '开奖详情' }}</h2>
      </div>
      <button type="button" aria-label="关闭全部开奖" @click="closeDrawHistory">×</button>
    </header>

    <div v-if="loadingSelectedLottery" class="state-block selected-lottery-history-sheet__state">
      <van-loading>加载中...</van-loading>
    </div>
    <van-empty v-else-if="!selectedLotteryItems.length" description="暂无开奖结果" />
    <div v-else class="selected-lottery-history-sheet__list">
      <article v-for="item in selectedLotteryItems" :key="item.id" class="selected-lottery-history-card">
        <div>
          <h3>第{{ item.issue }}期</h3>
          <p>{{ formatOpenedAt(item) }}</p>
        </div>
        <ResultBalls :item="item" :key-prefix="`selected-${item.id}`" />
      </article>
    </div>
  </section>
</template>

<style scoped>
.selected-lottery-history-sheet {
  display: flex;
  width: min(100vw, 672px);
  max-height: 62dvh;
  flex-direction: column;
  margin: 0 auto;
  overflow: hidden;
  border-radius: 24px 24px 0 0;
  background: #fff;
}

.selected-lottery-history-sheet__header {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 18px 12px;
  border-bottom: 1px solid rgba(238, 238, 238, 0.78);
}

.selected-lottery-history-sheet__header p {
  margin: 0 0 4px;
  color: #8e706d;
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.12em;
}

.selected-lottery-history-sheet__header h2 {
  margin: 0;
  color: #7a0711;
  font-size: 18px;
  font-weight: 900;
  letter-spacing: -0.04em;
}

.selected-lottery-history-sheet__header button {
  display: inline-flex;
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 999px;
  color: #5a403e;
  background: #eeeeee;
  font-size: 23px;
  line-height: 1;
}

.selected-lottery-history-sheet__state {
  flex: 1 1 auto;
}

.selected-lottery-history-sheet__list {
  display: grid;
  flex: 1 1 auto;
  gap: 10px;
  overflow-y: auto;
  padding: 12px 14px calc(16px + env(safe-area-inset-bottom));
  scrollbar-width: none;
}

.selected-lottery-history-sheet__list::-webkit-scrollbar {
  display: none;
}

.selected-lottery-history-card {
  display: grid;
  gap: 12px;
  border-radius: 16px;
  padding: 12px;
  background: #f9f9f9;
}

.selected-lottery-history-card h3 {
  margin: 0;
  color: #1a1c1c;
  font-size: 15px;
  font-weight: 900;
}

.selected-lottery-history-card p {
  margin: 4px 0 0;
  color: #5a403e;
  font-size: 12px;
  font-weight: 700;
}

.state-block {
  padding: 40px 0;
  text-align: center;
}
</style>
