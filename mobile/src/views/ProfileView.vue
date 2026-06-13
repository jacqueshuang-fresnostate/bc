<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useBrandingStore } from '../stores/branding'
import { showDialog, showToast } from 'vant'
import { errorMessage, uploadUserAvatar } from '../api/user'
import CachedAvatarImage from '../components/mobile/CachedAvatarImage.vue'
import WalletBentoCard from '../components/mobile/WalletBentoCard.vue'
import SettingsListGroup from '../components/mobile/SettingsListGroup.vue'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { useMobileUserDataStore } from '../stores/mobileUserData'
import { useSupportUnreadStore } from '../stores/supportUnread'

const router = useRouter()
const auth = useAuthStore()
const brandingStore = useBrandingStore()
const userDataStore = useMobileUserDataStore()
const supportUnreadStore = useSupportUnreadStore()
const { branding } = storeToRefs(brandingStore)
const { profile } = storeToRefs(userDataStore)
const { hasUnread: hasSupportUnread, unreadTotal: supportUnreadTotal } = storeToRefs(supportUnreadStore)
const uploadingAvatar = ref(false)
const avatarInput = ref<HTMLInputElement | null>(null)
const balanceText = computed(() => String(profile.value?.balance || '0.00'))
const username = computed(() => profile.value?.username || '会员')
const avatarUrl = computed(() => profile.value?.avatar_url || profile.value?.avatarUrl || '')
const avatarText = computed(() => String(username.value || '会').trim().slice(0, 1).toUpperCase() || '会')
const memberLabel = computed(() => statusText(profile.value?.status || 'active'))
const inviteText = computed(() => profile.value?.invitation_code || '-')
const canInvite = computed(() => profile.value?.can_invite === true)
const inviterText = computed(() => {
  const inviter = profile.value?.inviter
  if (!inviter) return '无'
  return `${inviter.username}（${profile.value?.used_invitation_code || inviter.invitation_code}）`
})

const accountItems = computed(() => [
  { key: 'security', label: '安全中心与密码', icon: 'shield_lock', value: profile.value?.email ? '已绑定' : '未绑定', hint: profile.value?.email || '' },
  { key: 'ledger', label: '资金流水', icon: 'payments', hint: '查看充值、投注、提现与派奖记录' },
  { key: 'withdrawal', label: '提现管理', icon: 'account_balance', hint: '管理收款信息' },
])

const supportItems = computed(() => [
  { key: 'chatHall', label: '聊天大厅', icon: 'chat', value: '会员交流' },
  {
    key: 'support',
    label: '在线客服',
    icon: 'support_agent',
    value: hasSupportUnread.value ? '有新消息' : '24h 在线',
    unread: hasSupportUnread.value,
    badgeCount: supportUnreadTotal.value,
  },
  { key: 'help', label: '帮助中心', icon: 'help', hint: '查看常见问题' },
])

const inviteItems = computed(() => [
  {
    key: 'agentCenter',
    label: '代理中心',
    icon: 'star',
    value: canInvite.value ? inviteText.value : '申请代理',
    hint: canInvite.value ? '查看直属下级充值与返利' : '申请成为代理后可邀请下级',
  },
  { key: 'inviter', label: '邀请人', icon: 'group', value: inviterText.value },
])

const statusTextMap: Record<string, string> = {
  active: '正常',
  suspended: '已停用',
  locked: '已锁定',
}

onMounted(async () => {
  try {
    await userDataStore.loadProfile()
  } catch {}
  try {
    await supportUnreadStore.loadConversations({ silent: true })
  } catch {}
})

function statusText(status: string) {
  return statusTextMap[status] || status || '-'
}

function onAccountItem(item: { key: string }) {
  if (item.key === 'security') router.push('/security-center')
  if (item.key === 'agentCenter') router.push('/agent-center')
  if (item.key === 'ledger') router.push('/ledger')
  if (item.key === 'withdrawal') router.push('/withdrawal-methods')
}

function onSupportItem(item: { key: string }) {
  if (item.key === 'chatHall') router.push('/chat-hall')
  if (item.key === 'support') router.push('/support')
  if (item.key === 'help') showToast('帮助中心建设中')
}

function openAvatarPicker() {
  if (uploadingAvatar.value) return
  avatarInput.value?.click()
}

async function onAvatarFileChange(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!(file instanceof File)) {
    input.value = ''
    return
  }

  await uploadAvatarFile(file)
  input.value = ''
}

async function uploadAvatarFile(file: File) {
  uploadingAvatar.value = true
  try {
    const updatedProfile = await uploadUserAvatar(file)
    userDataStore.setProfile(updatedProfile)
    await auth.setSession(auth.accessToken, updatedProfile)
    showToast('头像更新成功')
  } catch (error) {
    showToast(errorMessage(error, '头像上传失败'))
  } finally {
    uploadingAvatar.value = false
  }
}

async function logout() {
  await showDialog({ title: '确认', message: '确定退出登录？' })
  await auth.logout()
  userDataStore.clearUserScopedState()
  supportUnreadStore.clear()
  router.push('/login')
}
</script>

<template>
  <div class="account-dashboard min-h-screen bg-background pb-28 text-on-surface font-body">
    <header class="mobile-safe-header fixed top-0 left-0 z-40 flex h-16 w-full items-center justify-between bg-white/80 px-6 shadow-sm shadow-red-900/5 backdrop-blur-md">
      <div class="flex items-center gap-3">
        <img
          :alt="`${branding.site_name} 标志`"
          class="h-8 w-8 rounded-full border border-red-900/10 object-cover shadow-sm"
          :src="branding.logo_url"
          @error="brandingStore['set\u004cogoFallback']()"
        />
        <span class="font-headline text-xl font-bold italic tracking-tighter text-red-900">{{ branding.site_name }}</span>
      </div>
      <div class="flex items-center gap-2 rounded-full bg-stone-50/70 px-4 py-1.5 text-red-800 active:scale-95">
        <span class="text-sm">钱包</span>
        <span class="font-headline text-sm font-semibold tracking-tight">¥{{ balanceText }}</span>
      </div>
    </header>

    <main class="mobile-safe-main-top mx-auto max-w-lg px-3">
      <section class="mb-5">
        <div class="mb-5 flex items-start justify-between px-0.5">
          <div class="flex items-center gap-3">
            <input
              id="profile-avatar-input"
              ref="avatarInput"
              accept="image/*"
              class="sr-only"
              :disabled="uploadingAvatar"
              type="file"
              @change="onAvatarFileChange"
            />
            <label
              aria-label="上传头像"
              class="relative flex h-16 w-16 shrink-0 cursor-pointer items-center justify-center overflow-hidden rounded-full border border-red-900/10 bg-white font-headline text-lg font-black text-red-900 shadow-sm transition active:scale-95"
              for="profile-avatar-input"
              role="button"
              tabindex="0"
              @keydown.enter.prevent="openAvatarPicker"
              @keydown.space.prevent="openAvatarPicker"
            >
              <CachedAvatarImage
                v-if="avatarUrl"
                :alt="`${username}头像`"
                class="h-full w-full object-cover"
                :src="avatarUrl"
              >
                <span>{{ avatarText }}</span>
              </CachedAvatarImage>
              <span v-else>{{ avatarText }}</span>
              <span class="absolute bottom-0 right-0 flex h-5 w-5 items-center justify-center rounded-full border-2 border-white bg-red-800 text-white">
                <LucideIcon name="camera" class="h-3.5 w-3.5" :stroke-width="2.6" />
              </span>
              <span
                v-if="uploadingAvatar"
                class="absolute inset-0 flex items-center justify-center rounded-full bg-red-950/60 text-[10px] font-bold text-white"
              >
                上传中
              </span>
            </label>
            <div>
              <h1 class="font-headline text-xl font-extrabold tracking-tight text-on-surface">我的账户</h1>
              <p class="mt-1 text-[11px] text-on-surface-variant">ID: {{ username }} • {{ memberLabel }}会员</p>
            </div>
          </div>
          <button class="flex h-9 w-9 items-center justify-center rounded-xl bg-white text-on-surface-variant shadow-sm transition-colors active:bg-stone-100" type="button">
            <LucideIcon name="settings" class="h-5 w-5" />
          </button>
        </div>

        <WalletBentoCard
          :balance="balanceText"
          :usdt-balance="profile?.usdt_balance ? `${profile.usdt_balance} USDT` : '0.00 USDT'"
          @deposit="router.push('/deposit')"
          @withdraw="router.push('/withdraw')"
        />
      </section>

      <section class="space-y-3">
        <SettingsListGroup :items="accountItems" @select="onAccountItem" />
        <SettingsListGroup :items="supportItems" @select="onSupportItem" />
        <SettingsListGroup :items="inviteItems" @select="onAccountItem" />

        <button class="flex w-full items-center justify-center gap-2 rounded-2xl bg-white py-3.5 text-xs font-bold text-primary transition-colors active:bg-red-50" @click="logout">
          <LucideIcon name="logout" class="h-4 w-4" />
          退出登录
        </button>
      </section>
    </main>
  </div>
</template>
