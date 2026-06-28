<script setup lang="ts">
import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import LucideIcon from './LucideIcon.vue'
import { useWalletPrivacyStore } from '../../stores/walletPrivacy'
import type { WithdrawalTurnoverProgress } from '../../api/user'

const props = defineProps<{
  progress: WithdrawalTurnoverProgress | null
  loading?: boolean
}>()

const walletPrivacyStore = useWalletPrivacyStore()
const { hideWalletAmount } = storeToRefs(walletPrivacyStore)

const requiredMinor = computed(() => Math.max(0, Number(props.progress?.requiredEffectiveBetMinor || 0)))
const completedMinor = computed(() => Math.max(0, Number(props.progress?.completedEffectiveBetMinor || 0)))
const remainingMinor = computed(() => Math.max(0, Number(props.progress?.remainingEffectiveBetMinor || 0)))
const enabled = computed(() => props.progress?.enabled === true)
const hasTask = computed(() => requiredMinor.value > 0)
const completed = computed(() => hasTask.value && remainingMinor.value <= 0)
const progressPercent = computed(() => {
  if (!hasTask.value) return 0
  return Math.min(100, Math.max(0, Math.round((completedMinor.value / requiredMinor.value) * 100)))
})
const progressWidth = computed(() => `${progressPercent.value}%`)
const statusText = computed(() => {
  if (props.loading && !props.progress) return '同步中'
  if (!enabled.value) return '任务未开启'
  if (!hasTask.value) return '暂无任务'
  if (completed.value) return '已完成'
  return '进行中'
})
const statusTone = computed(() => {
  if (!enabled.value || !hasTask.value) return 'text-stone-500 bg-stone-100'
  if (completed.value) return 'text-emerald-700 bg-emerald-50'
  return 'text-red-800 bg-red-50'
})
const summaryText = computed(() => {
  if (!enabled.value) return '当前不限制提现投注任务'
  if (!hasTask.value) return '充值后会自动生成任务'
  if (completed.value) return '已满足提现投注要求'
  return `还差 ${formatMoney(remainingMinor.value)}`
})

function formatMoney(value: number) {
  if (hideWalletAmount.value) return '••••'
  return `¥${(Math.max(0, Number(value || 0)) / 100).toFixed(2)}`
}
</script>

<template>
  <section class="withdrawal-turnover-card mt-3 rounded-[1.25rem] border border-red-900/10 bg-white px-4 py-3 shadow-sm shadow-red-900/5">
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0">
        <div class="flex items-center gap-2">
          <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-red-50 text-red-800">
            <LucideIcon name="receipt_text" class="h-4 w-4" :stroke-width="2.5" />
          </span>
          <div class="min-w-0">
            <h2 class="truncate text-sm font-black text-red-950">提现任务</h2>
            <p class="mt-0.5 truncate text-[11px] text-stone-500">{{ summaryText }}</p>
          </div>
        </div>
      </div>
      <span class="shrink-0 rounded-full px-2.5 py-1 text-[10px] font-black" :class="statusTone">
        {{ statusText }}
      </span>
    </div>

    <div class="mt-3">
      <div
        aria-label="提现任务完成进度"
        class="h-2.5 overflow-hidden rounded-full bg-stone-100"
        role="progressbar"
        :aria-valuemax="100"
        :aria-valuemin="0"
        :aria-valuenow="progressPercent"
      >
        <div class="withdrawal-turnover-card__bar h-full rounded-full" :style="{ width: progressWidth }"></div>
      </div>
      <div class="mt-2 grid grid-cols-3 gap-2 text-center">
        <div>
          <p class="text-[10px] text-stone-400">要求</p>
          <p class="mt-0.5 truncate text-[11px] font-black text-stone-800">{{ formatMoney(requiredMinor) }}</p>
        </div>
        <div>
          <p class="text-[10px] text-stone-400">已完成</p>
          <p class="mt-0.5 truncate text-[11px] font-black text-stone-800">{{ formatMoney(completedMinor) }}</p>
        </div>
        <div>
          <p class="text-[10px] text-stone-400">进度</p>
          <p class="mt-0.5 truncate text-[11px] font-black text-stone-800">{{ progressPercent }}%</p>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.withdrawal-turnover-card {
  border-radius: 1.25rem;
  background: #fff;
}

.withdrawal-turnover-card__bar {
  min-width: 0;
  background: linear-gradient(90deg, #9d101c 0%, #c9373a 58%, #f2b45b 100%);
  transition: width 180ms ease;
}
</style>
