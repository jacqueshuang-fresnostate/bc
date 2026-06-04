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
  <section class="rounded-[28px] bg-white p-6 shadow-sm shadow-red-900/5">
    <div class="mb-1.5 flex items-start justify-between">
      <div>
        <div class="mb-1 text-xs tracking-wider text-[#5a403e]">当前期号</div>
        <div class="font-headline text-lg font-bold tracking-tight text-[#1a1c1c]">{{ roundIssueText }}</div>
      </div>
      <div class="text-right">
        <div class="mb-1 text-xs tracking-wider text-[#5a403e]">投注截止</div>
        <div class="font-headline text-3xl font-extrabold tracking-tighter text-[#8c0a15]">{{ roundDeadlineText }}</div>
      </div>
    </div>
    <div class="flex items-center justify-between gap-2">
      <div class="shrink-0 text-xs tracking-wider text-[#5a403e]">上期开奖 {{ latestIssue || '暂无' }} 期</div>
      <div class="flex flex-nowrap justify-end gap-1.5">
        <span v-for="(number, index) in latestNumbers" :key="`${number}-${index}`" class="flex h-7 w-7 items-center justify-center rounded-full bg-gradient-to-br from-[#8c0a15] to-[#af2829] font-headline text-sm font-bold text-white">{{ number }}</span>
        <span v-if="!latestNumbers.length" class="latest-draw-empty-dot inline-flex h-7 items-center rounded-full bg-[#f8f1ef] px-2.5 text-xs font-bold text-[#8e706d]">待开奖</span>
      </div>
    </div>
  </section>
</template>
