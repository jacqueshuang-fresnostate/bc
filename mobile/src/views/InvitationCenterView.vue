<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { showToast } from 'vant'
import http from '../api/http'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { formatDateTime } from '../utils/lotteryFormat'

type DirectUser = {
  id: number
  username: string
  status: string
  created_at: string
}

type InvitationSummary = {
  can_invite: boolean
  invitation_code: string
  direct_count: number
  total_direct_deposit: string
  total_paid_commission: string
  direct_users: DirectUser[]
}

const router = useRouter()
const loading = ref(false)
const summary = ref<InvitationSummary>({
  can_invite: false,
  invitation_code: '',
  direct_count: 0,
  total_direct_deposit: '0.00',
  total_paid_commission: '0.00',
  direct_users: [],
})

const canInvite = computed(() => summary.value.can_invite === true)
const invitationCode = computed(() => summary.value.invitation_code || '-')
const statusTextMap: Record<string, string> = {
  active: '正常',
  frozen: '已冻结',
  risk: '风控中',
  disabled: '已禁用',
}

onMounted(loadSummary)

async function loadSummary() {
  loading.value = true
  try {
    const res = await http.get('/auth/invitations/summary')
    summary.value = { ...summary.value, ...res.data }
  } catch (e: any) {
    showToast(e.response?.data?.detail || '邀请中心加载失败')
  } finally {
    loading.value = false
  }
}

async function copyInvitationCode() {
  if (!canInvite.value) {
    showToast('普通用户暂无邀请权限')
    return
  }
  const code = summary.value.invitation_code
  if (!code) return
  try {
    await navigator.clipboard.writeText(code)
    showToast('邀请码已复制')
  } catch {
    showToast(code)
  }
}

function formatDate(value: string) {
  return formatDateTime(value)
}

function statusText(status: string) {
  return statusTextMap[status] || status || '-'
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
          </div>
          <button class="rounded-2xl bg-white px-4 py-2 text-xs font-bold text-primary active:scale-95" type="button" @click="copyInvitationCode">
            复制
          </button>
        </div>
      </section>

      <section v-if="!canInvite" class="rounded-[1.35rem] bg-white p-4 text-sm text-on-surface-variant shadow-sm">
        普通用户暂无邀请权限
      </section>

      <section class="grid grid-cols-3 gap-3">
        <div class="rounded-[1.35rem] bg-white p-4 shadow-sm">
          <p class="text-[11px] text-on-surface-variant">直属人数</p>
          <strong class="mt-2 block font-headline text-xl text-on-surface">{{ summary.direct_count }}</strong>
        </div>
        <div class="rounded-[1.35rem] bg-white p-4 shadow-sm">
          <p class="text-[11px] text-on-surface-variant">直属充值</p>
          <strong class="mt-2 block font-headline text-xl text-on-surface">¥{{ summary.total_direct_deposit }}</strong>
        </div>
        <div class="rounded-[1.35rem] bg-white p-4 shadow-sm">
          <p class="text-[11px] text-on-surface-variant">已付返利</p>
          <strong class="mt-2 block font-headline text-xl text-on-surface">¥{{ summary.total_paid_commission }}</strong>
        </div>
      </section>

      <section class="rounded-[1.5rem] bg-white p-4 shadow-sm">
        <div class="mb-3 flex items-center justify-between">
          <h2 class="font-headline text-base font-bold text-on-surface">直属下级</h2>
          <span class="text-xs text-on-surface-variant">最多显示 100 人</span>
        </div>
        <div v-if="!summary.direct_users.length" class="rounded-2xl bg-stone-50 py-8 text-center text-sm text-on-surface-variant">暂无直属下级</div>
        <div v-else class="space-y-2">
          <div v-for="item in summary.direct_users" :key="item.id" class="flex items-center justify-between rounded-2xl bg-stone-50 px-3 py-3">
            <div>
              <strong class="block text-sm text-on-surface">{{ item.username }}</strong>
              <span class="text-[11px] text-on-surface-variant">{{ formatDate(item.created_at) }}</span>
            </div>
            <span class="rounded-full bg-white px-3 py-1 text-[11px] font-bold text-primary">{{ statusText(item.status) }}</span>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>
