<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { showToast } from 'vant'
import {
  fetchAgentApplication,
  errorMessage,
  fetchInvitationSummary,
  submitAgentApplication,
  type AgentApplication,
  type AgentApplicationStatus,
  type InviteStatus,
  type RebateMode,
  type UserInvitationBetPlaySummary,
  type UserInvitationDirectUser,
  type UserInvitationSummary,
  type UserStatus,
} from '../api/user'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { useMobileUserDataStore } from '../stores/mobileUserData'
import { formatDateTime } from '../utils/lotteryFormat'

const router = useRouter()
const userDataStore = useMobileUserDataStore()
const { profile } = storeToRefs(userDataStore)
const loading = ref(false)
const submitting = ref(false)
const application = ref<AgentApplication | null>(null)
const applicationReason = ref('')
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
const isAgentAccount = computed(() => profile.value?.kind === 'agent')
const invitationCode = computed(() => summary.value.invitationCode || '-')
const rebateRateText = computed(() => formatBasisPoints(summary.value.defaultRechargeRebateBasisPoints))
const latestApplication = computed(() => application.value)
const canSubmitApplication = computed(() => {
  const status = latestApplication.value?.status
  return !isAgentAccount.value && status !== 'pending' && status !== 'approved'
})
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
const applicationStatusTextMap: Record<AgentApplicationStatus, string> = {
  pending: '待审核',
  approved: '已通过',
  rejected: '已驳回',
}

onMounted(loadAgentCenter)

async function loadAgentCenter() {
  loading.value = true
  try {
    const [nextSummary, nextApplication] = await Promise.all([
      fetchInvitationSummary(),
      fetchAgentApplication(),
      userDataStore.loadProfile({ silent: true }).catch(() => null),
    ])
    summary.value = nextSummary
    application.value = nextApplication.application ?? null
  } catch (e) {
    showToast(errorMessage(e, '代理中心加载失败'))
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

function topBetPlaySummaries(item: UserInvitationDirectUser): UserInvitationBetPlaySummary[] {
  return (item.betPlaySummaries || []).slice(0, 3)
}

function hasBetProfile(item: UserInvitationDirectUser) {
  return Number(item.totalBetAmountMinor || 0) > 0 || Boolean(item.latestBet)
}

function latestBetMainText(item: UserInvitationDirectUser) {
  const latestBet = item.latestBet
  if (!latestBet) return '暂无投注记录'
  return `${latestBet.lotteryName || '未知彩种'} · ${latestBet.playName || latestBet.ruleCode || '未知玩法'}`
}

function latestBetMetaText(item: UserInvitationDirectUser) {
  const latestBet = item.latestBet
  if (!latestBet) return ''
  const issue = latestBet.issue ? `第 ${latestBet.issue} 期` : '期号未记录'
  return `${issue} · ¥${formatMoney(latestBet.amountMinor)}`
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

function applicationStatusText(status: AgentApplicationStatus) {
  return applicationStatusTextMap[status] || status || '-'
}

function applicationStatusClass(status: AgentApplicationStatus) {
  if (status === 'approved') return 'bg-emerald-50 text-emerald-700'
  if (status === 'rejected') return 'bg-red-50 text-red-700'
  return 'bg-amber-50 text-amber-700'
}

async function submitApplication() {
  if (!canSubmitApplication.value) return
  const reason = applicationReason.value.trim()
  if (!reason) {
    showToast('请输入申请说明')
    return
  }
  submitting.value = true
  try {
    application.value = await submitAgentApplication({ reason })
    applicationReason.value = ''
    showToast('代理申请已提交，请等待后台审核')
  } catch (e) {
    showToast(errorMessage(e, '代理申请提交失败'))
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="agent-center-page min-h-screen bg-background text-on-surface font-body">
    <header class="mobile-safe-compact-header sticky top-0 z-30 flex h-14 items-center justify-between bg-white/85 px-4 shadow-sm shadow-red-900/5 backdrop-blur-md">
      <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-50 text-red-900" type="button" @click="router.back()">
        <LucideIcon name="arrow_back" class="h-5 w-5" />
      </button>
      <strong class="font-headline text-base text-red-900">代理中心</strong>
      <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-stone-50 text-red-900" type="button" :disabled="loading" @click="loadAgentCenter">
        <LucideIcon name="refresh" class="h-4.5 w-4.5" />
      </button>
    </header>

    <main class="agent-center-main mx-auto max-w-lg space-y-4 px-3 pt-4">
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

      <section v-if="!canInvite" class="space-y-3">
        <div v-if="isAgentAccount" class="rounded-[1.35rem] bg-white p-4 text-sm text-on-surface-variant shadow-sm">
          当前账号已是代理，但代理邀请入口暂未开启，请等待平台配置开放。
        </div>
        <div v-else class="rounded-[1.5rem] bg-white p-4 shadow-sm">
          <div class="flex items-start justify-between gap-3">
            <div>
              <h2 class="font-headline text-base font-bold text-on-surface">申请成为代理</h2>
              <p class="mt-1 text-xs text-on-surface-variant">审核通过后，你的邀请码才可以邀请下级并参与返利统计。</p>
            </div>
            <span
              v-if="latestApplication"
              class="shrink-0 rounded-full px-3 py-1 text-[11px] font-bold"
              :class="applicationStatusClass(latestApplication.status)"
            >
              {{ applicationStatusText(latestApplication.status) }}
            </span>
          </div>

          <div v-if="latestApplication" class="mt-3 rounded-2xl bg-stone-50 p-3 text-xs text-on-surface-variant">
            <p class="font-bold text-on-surface">最近申请：{{ formatDate(latestApplication.createdAt) }}</p>
            <p class="mt-1 whitespace-pre-wrap leading-5">{{ latestApplication.reason }}</p>
            <p v-if="latestApplication.reviewNote" class="mt-2 rounded-xl bg-white px-3 py-2 text-red-900">
              审核备注：{{ latestApplication.reviewNote }}
            </p>
          </div>

          <div v-if="canSubmitApplication" class="mt-3 space-y-3">
            <textarea
              v-model="applicationReason"
              class="min-h-[6rem] w-full resize-none rounded-2xl border border-red-900/10 bg-stone-50 px-3 py-3 text-sm text-on-surface outline-none focus:border-red-800"
              maxlength="500"
              placeholder="简单说明你的推广资源或申请原因"
            />
            <button
              class="flex w-full items-center justify-center rounded-2xl bg-primary py-3 text-sm font-bold text-white shadow-lg shadow-red-900/15 active:scale-[0.99] disabled:opacity-60"
              type="button"
              :disabled="submitting"
              @click="submitApplication"
            >
              {{ submitting ? '提交中' : latestApplication?.status === 'rejected' ? '重新提交申请' : '提交代理申请' }}
            </button>
          </div>
          <p v-else-if="latestApplication?.status === 'pending'" class="mt-3 text-xs text-on-surface-variant">
            申请正在审核中，审核通过后这里会展示可邀请的代理数据。
          </p>
        </div>
      </section>

      <section v-if="canInvite" class="grid grid-cols-2 gap-3">
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

      <section v-if="canInvite" class="rounded-[1.5rem] bg-white p-4 shadow-sm">
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
                <span class="text-[11px] text-on-surface-variant">注册 {{ formatDate(item.registeredAt || item.createdAt) }}</span>
              </div>
              <span class="shrink-0 rounded-full bg-white px-3 py-1 text-[11px] font-bold text-primary">{{ statusText(item.status) }}</span>
            </div>
            <div class="mt-3 flex flex-wrap items-center gap-2 text-[11px] text-on-surface-variant">
              <span class="rounded-full bg-white px-2.5 py-1 font-bold text-red-900">邀请{{ inviteStatusText(item.inviteStatus) }}</span>
              <span class="rounded-full bg-white px-2.5 py-1">{{ item.rebateEnabled ? '返利开启' : '返利关闭' }}</span>
              <span class="rounded-full bg-white px-2.5 py-1 font-bold text-emerald-700">余额 ¥{{ formatMoney(item.availableBalanceMinor) }}</span>
              <span class="rounded-full bg-white px-2.5 py-1">充值 ¥{{ formatMoney(item.totalDepositMinor) }}</span>
              <span class="rounded-full bg-white px-2.5 py-1">提现 ¥{{ formatMoney(item.totalWithdrawalMinor) }}</span>
              <span class="rounded-full bg-white px-2.5 py-1 font-bold text-red-900">投注 ¥{{ formatMoney(item.totalBetAmountMinor) }}</span>
            </div>
            <div class="mt-3 rounded-2xl border border-red-900/5 bg-white/80 px-3 py-2.5">
              <div class="flex items-start justify-between gap-3">
                <div class="min-w-0">
                  <p class="text-[11px] font-bold text-red-900">最近投注</p>
                  <p class="mt-1 truncate text-xs font-bold text-on-surface">{{ latestBetMainText(item) }}</p>
                  <p v-if="item.latestBet" class="mt-0.5 text-[11px] text-on-surface-variant">{{ latestBetMetaText(item) }}</p>
                </div>
                <span class="shrink-0 rounded-full bg-red-50 px-2.5 py-1 text-[11px] font-bold text-primary">
                  {{ hasBetProfile(item) ? `${topBetPlaySummaries(item).length || 1} 类玩法` : '无投注' }}
                </span>
              </div>
              <div v-if="topBetPlaySummaries(item).length" class="mt-2 space-y-1.5">
                <div
                  v-for="play in topBetPlaySummaries(item)"
                  :key="`${item.id}-${play.lotteryId}-${play.ruleCode}`"
                  class="flex items-center justify-between gap-2 rounded-xl bg-stone-50 px-2.5 py-2 text-[11px]"
                >
                  <span class="min-w-0 truncate font-bold text-on-surface">{{ play.lotteryName }} · {{ play.playName }}</span>
                  <span class="shrink-0 text-red-900">¥{{ formatMoney(play.amountMinor) }} · {{ play.orderCount }}笔</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>
    </main>
  </div>
</template>

<style scoped>
.agent-center-page {
  padding-bottom: var(--mobile-bottom-nav-space);
}

.agent-center-main {
  padding-bottom: calc(var(--mobile-bottom-nav-space) + 1rem);
}
</style>
