<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { showToast } from 'vant'
import {
  errorMessage,
  fetchInvitationSummary,
  type InviteStatus,
  type RebateMode,
  type UserInvitationSummary,
  type UserStatus,
} from '../api/user'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { formatDateTime } from '../utils/lotteryFormat'

const router = useRouter()
const loading = ref(false)
const summary = ref<UserInvitationSummary>({
  canInvite: false,
  invitationCode: '',
  directCount: 0,
  activeDirectCount: 0,
  totalDirectDepositMinor: 0,
  totalPaidCommissionMinor: 0,
  rebateMode: 'immediate',
  defaultRechargeRebateBasisPoints: 0,
  directUsers: [],
})

const canInvite = computed(() => summary.value.canInvite === true)
const invitationCode = computed(() => summary.value.invitationCode || '-')
const rebateRateText = computed(() => formatBasisPoints(summary.value.defaultRechargeRebateBasisPoints))
const statusTextMap: Record<UserStatus, string> = {
  active: '正常',
  suspended: '已停用',
  locked: '已锁定',
}
const inviteStatusTextMap: Record<InviteStatus, string> = {
  pending: '待生效',
  active: '已生效',
  disabled: '已禁用',
}
const rebateModeTextMap: Record<RebateMode, string> = {
  immediate: '立即返利',
  rechargeTiered: '充值阶梯返利',
}

onMounted(loadSummary)

async function loadSummary() {
  loading.value = true
  try {
    summary.value = await fetchInvitationSummary()
  } catch (e) {
    showToast(errorMessage(e, '邀请中心加载失败'))
  } finally {
    loading.value = false
  }
}

async function copyInvitationCode() {
  if (!canInvite.value) {
    showToast('当前账号暂无可用邀请权限')
    return
  }
  const code = summary.value.invitationCode
  if (!code) return
  try {
    await navigator.clipboard.writeText(code)
    showToast('邀请码已复制')
  } catch {
    showToast(code)
  }
}

function formatDate(value: string) {
  if (!value) return '注册时间未记录'
  return formatDateTime(value)
}

function formatMoney(value: number) {
  return (Number(value || 0) / 100).toFixed(2)
}

function formatBasisPoints(value: number) {
  const percent = (Number(value || 0) / 100).toFixed(2)
  return `${percent.replace(/\.00$/, '').replace(/(\.\d)0$/, '$1')}%`
}

function statusText(status: UserStatus) {
  return statusTextMap[status] || status || '-'
}

function inviteStatusText(status: InviteStatus) {
  return inviteStatusTextMap[status] || status || '-'
}

function rebateModeText(mode: RebateMode) {
  return rebateModeTextMap[mode] || mode || '-'
}
</script>

<template>
  <div class="min-h-screen bg-background pb-24 text-on-surface font-body">
    <header class="sticky top-0 z-30 flex h-14 items-center justify-between bg-white/85 px-4 shadow-sm shadow-red-900/5 backdrop-blur-md">
      <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-50 text-red-900" type="button" @click="router.back()">
        <LucideIcon name="arrow_back" class="h-5 w-5" />
      </button>
      <strong class="font-headline text-base text-red-900">邀请中心</strong>
      <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-50 text-red-900" type="button" :disabled="loading" @click="loadSummary">
        <LucideIcon name="refresh" class="h-4.5 w-4.5" />
      </button>
    </header>

    <main class="mx-auto max-w-lg space-y-4 px-3 pt-4">
      <section class="rounded-[1.75rem] bg-gradient-to-br from-red-800 to-red-950 p-5 text-white shadow-xl shadow-red-950/15">
        <p class="text-xs text-white/70">我的邀请码</p>
        <div class="mt-3 flex items-center justify-between gap-3">
          <div>
            <h1 class="font-headline text-3xl font-black tracking-tight">{{ invitationCode }}</h1>
            <p class="mt-1 text-xs text-white/70">仅正常代理可邀请直属下级</p>
            <p class="mt-1 text-[11px] text-white/60">返利模式：{{ rebateModeText(summary.rebateMode) }} · 默认 {{ rebateRateText }}</p>
          </div>
          <button class="rounded-2xl bg-white px-4 py-2 text-xs font-bold text-primary active:scale-95" type="button" @click="copyInvitationCode">
            复制
          </button>
        </div>
      </section>

      <section v-if="!canInvite" class="rounded-[1.35rem] bg-white p-4 text-sm text-on-surface-variant shadow-sm">
        当前账号暂无可用邀请权限，邀请码仅作账户标识
      </section>

      <section class="grid grid-cols-2 gap-3">
        <div class="rounded-[1.35rem] bg-white p-4 shadow-sm">
          <p class="text-[11px] text-on-surface-variant">直属人数</p>
          <strong class="mt-2 block font-headline text-xl text-on-surface">{{ summary.directCount }}</strong>
        </div>
        <div class="rounded-[1.35rem] bg-white p-4 shadow-sm">
          <p class="text-[11px] text-on-surface-variant">有效下级</p>
          <strong class="mt-2 block font-headline text-xl text-on-surface">{{ summary.activeDirectCount }}</strong>
        </div>
        <div class="rounded-[1.35rem] bg-white p-4 shadow-sm">
          <p class="text-[11px] text-on-surface-variant">直属充值</p>
          <strong class="mt-2 block font-headline text-xl text-on-surface">¥{{ formatMoney(summary.totalDirectDepositMinor) }}</strong>
        </div>
        <div class="rounded-[1.35rem] bg-white p-4 shadow-sm">
          <p class="text-[11px] text-on-surface-variant">已付返利</p>
          <strong class="mt-2 block font-headline text-xl text-on-surface">¥{{ formatMoney(summary.totalPaidCommissionMinor) }}</strong>
        </div>
      </section>

      <section class="rounded-[1.5rem] bg-white p-4 shadow-sm">
        <div class="mb-3 flex items-center justify-between">
          <h2 class="font-headline text-base font-bold text-on-surface">直属下级</h2>
          <span class="text-xs text-on-surface-variant">{{ summary.directCount }} 人</span>
        </div>
        <div v-if="!summary.directUsers.length" class="rounded-2xl bg-stone-50 py-8 text-center text-sm text-on-surface-variant">暂无直属下级</div>
        <div v-else class="space-y-2">
          <div v-for="item in summary.directUsers" :key="item.id" class="rounded-2xl bg-stone-50 px-3 py-3">
            <div class="flex items-start justify-between gap-3">
              <div>
                <strong class="block text-sm text-on-surface">{{ item.username }}</strong>
                <span class="text-[11px] text-on-surface-variant">{{ formatDate(item.createdAt) }}</span>
              </div>
              <span class="shrink-0 rounded-full bg-white px-3 py-1 text-[11px] font-bold text-primary">{{ statusText(item.status) }}</span>
            </div>
            <div class="mt-3 flex flex-wrap items-center gap-2 text-[11px] text-on-surface-variant">
              <span class="rounded-full bg-white px-2.5 py-1 font-bold text-red-900">邀请{{ inviteStatusText(item.inviteStatus) }}</span>
              <span class="rounded-full bg-white px-2.5 py-1">{{ item.rebateEnabled ? '返利开启' : '返利关闭' }}</span>
              <span class="rounded-full bg-white px-2.5 py-1">充值 ¥{{ formatMoney(item.totalDepositMinor) }}</span>
            </div>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>
