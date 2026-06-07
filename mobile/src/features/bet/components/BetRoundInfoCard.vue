<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  issue: string
  status: string
  countdownText: string
  latestIssue: string
  latestNumbers: string[]
}>()

const isOpening = computed(() => props.status === 'opening')
const roundIssueText = computed(() => {
  if (isOpening.value) return props.issue ? `第 ${props.issue} 期` : '开盘中'
  return `第 ${props.issue || '-'} 期`
})
const roundDeadlineText = computed(() => {
  if (isOpening.value) return props.issue ? '开奖中' : '开盘中'
  return props.countdownText
})
</script>

<template>
  <section class="bet-round-card rounded-[28px] bg-white shadow-sm shadow-red-900/5">
    <div class="bet-round-card__top mb-1.5">
      <div class="min-w-0">
        <div class="mb-1 text-xs tracking-wider text-[#5a403e]">当前期号</div>
        <div class="bet-round-card__issue font-headline font-bold tracking-tight text-[#1a1c1c]">{{ roundIssueText }}</div>
      </div>
      <div class="bet-round-card__deadline text-right">
        <div class="mb-1 text-xs tracking-wider text-[#5a403e]">投注截止</div>
        <div class="bet-round-card__countdown font-headline font-extrabold tracking-tighter text-[#8c0a15]">{{ roundDeadlineText }}</div>
      </div>
    </div>
    <div class="latest-draw-row">
      <div class="latest-draw-label text-xs tracking-wider text-[#5a403e]">上期开奖 {{ latestIssue || '暂无' }} 期</div>
      <div class="latest-draw-numbers">
        <span v-for="(number, index) in latestNumbers" :key="`${number}-${index}`" class="latest-draw-ball bg-gradient-to-br from-[#8c0a15] to-[#af2829] font-headline font-bold text-white">{{ number }}</span>
        <span v-if="!latestNumbers.length" class="latest-draw-empty-dot inline-flex items-center rounded-full bg-[#f8f1ef] px-2.5 text-xs font-bold text-[#8e706d]">待开奖</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.bet-round-card {
  padding: clamp(1rem, 4.5vw, 1.5rem);
}

.bet-round-card__top {
  display: flex;
  min-width: 0;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.bet-round-card__issue {
  font-size: clamp(1rem, 4.4vw, 1.125rem);
  line-height: 1.25;
}

.bet-round-card__deadline {
  max-width: 44%;
  flex-shrink: 0;
}

.bet-round-card__countdown {
  font-size: clamp(1.35rem, 7vw, 1.875rem);
  line-height: 1;
}

.latest-draw-row {
  display: flex;
  min-width: 0;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.5rem;
}

.latest-draw-label {
  min-width: 0;
  flex: 1 1 auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.latest-draw-numbers {
  display: flex;
  max-width: min(9rem, 58%);
  min-width: 0;
  flex: 0 1 auto;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 0.25rem;
}

.latest-draw-ball {
  display: inline-flex;
  width: clamp(1.25rem, 6vw, 1.5rem);
  height: clamp(1.25rem, 6vw, 1.5rem);
  flex: 0 0 clamp(1.25rem, 6vw, 1.5rem);
  align-items: center;
  justify-content: center;
  border-radius: 9999px;
  font-size: clamp(0.68rem, 3vw, 0.78rem);
  line-height: 1;
  letter-spacing: 0;
}

.latest-draw-empty-dot {
  min-height: clamp(1.25rem, 6vw, 1.5rem);
}

@media (max-width: 360px) {
  .latest-draw-numbers {
    max-width: 7.5rem;
    gap: 0.2rem;
  }
}

@media (max-width: 330px) {
  .latest-draw-row {
    flex-direction: column;
  }

  .latest-draw-numbers {
    max-width: 100%;
    justify-content: flex-start;
  }
}
</style>
