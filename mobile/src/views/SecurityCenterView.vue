<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { showToast } from 'vant'
import { ArrowLeft, CheckCircle2, KeyRound, Mail, ShieldCheck } from 'lucide-vue-next'
import { useAuthStore } from '../stores/auth'
import {
  bindUserEmail,
  changeUserPassword,
  errorMessage,
  fetchCurrentUserProfile,
  normalizeUserProfile,
} from '../api/user'

const router = useRouter()
const auth = useAuthStore()
const profile = ref<any>(null)
const bindEmail = ref('')
const currentPassword = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const bindSubmitting = ref(false)
const passwordSubmitting = ref(false)
const bindEmailEnabled = ref(true)
const activeTab = ref<'email' | 'password'>('email')

const isEmailBound = computed(() => !!profile.value?.email)
const canBindEmail = computed(() => !isEmailBound.value && bindEmailEnabled.value)

async function loadProfile() {
  try {
    profile.value = await fetchCurrentUserProfile()
  } catch (e: unknown) {
    showToast(errorMessage(e, '账户信息加载失败'))
  }
}

onMounted(async () => {
  await loadProfile()
})

async function submitBindEmail() {
  if (!bindEmailEnabled.value) { showToast('邮箱绑定暂未开放'); return }
  if (!bindEmail.value) { showToast('请输入邮箱地址'); return }
  bindSubmitting.value = true
  try {
    const session = await bindUserEmail(bindEmail.value.trim())
    await auth.setSession(session.token, session.user)
    profile.value = normalizeUserProfile(session.user)
    showToast('邮箱已绑定')
    bindEmail.value = ''
    activeTab.value = 'password'
  } catch (e: unknown) {
    showToast(errorMessage(e, '绑定失败'))
  } finally {
    bindSubmitting.value = false
  }
}

async function submitPasswordChange() {
  if (!currentPassword.value) { showToast('请输入当前密码'); return }
  if (!newPassword.value || newPassword.value.length < 6) { showToast('请输入至少 6 位新密码'); return }
  if (newPassword.value !== confirmPassword.value) { showToast('两次输入的密码不一致'); return }
  passwordSubmitting.value = true
  try {
    const session = await changeUserPassword(currentPassword.value, newPassword.value)
    await auth.setSession(session.token, session.user)
    profile.value = normalizeUserProfile(session.user)
    currentPassword.value = ''
    newPassword.value = ''
    confirmPassword.value = ''
    showToast('密码已修改')
  } catch (e: unknown) {
    showToast(errorMessage(e, '修改失败'))
  } finally {
    passwordSubmitting.value = false
  }
}
</script>

<template>
  <div class="min-h-screen bg-background text-on-surface font-body">
    <header class="sticky top-0 z-40 flex h-14 items-center justify-between bg-white/90 px-4 shadow-[0_1px_0_rgba(140,10,21,0.06)] backdrop-blur-md">
      <button class="flex h-9 w-9 items-center justify-center rounded-full bg-surface-container-low text-on-surface-variant active:bg-surface-container" type="button" @click="router.back()">
        <ArrowLeft class="h-5 w-5" />
      </button>
      <span class="font-headline text-sm font-extrabold text-on-surface">安全中心</span>
      <span class="h-9 w-9"></span>
    </header>

    <main class="mx-auto max-w-md px-5 pb-12 pt-5">
      <section class="mb-5 overflow-hidden rounded-[1.75rem] lacquer-gradient p-6 !text-on-primary shadow-[0_16px_32px_rgba(140,10,21,0.22)]">
        <div class="mb-8 flex items-center justify-between">
          <div class="flex h-12 w-12 items-center justify-center rounded-2xl bg-white/15">
            <ShieldCheck class="h-6 w-6" />
          </div>
          <div class="rounded-full bg-white/15 px-3 py-1 text-[10px] font-bold tracking-[0.18em]">SECURITY</div>
        </div>
        <h1 class="font-headline text-3xl font-black tracking-tight">安全中心</h1>
        <p class="mt-2 text-sm font-medium text-white/85">管理邮箱绑定与登录密码</p>
      </section>

      <van-tabs v-model:active="activeTab" class="security-center-tabs" swipeable>
        <van-tab title="绑定邮箱" name="email">
          <section class="mb-5 rounded-[1.5rem] bg-white p-5 shadow-[0_12px_28px_rgba(26,28,28,0.06)]">
            <div class="mb-4 flex items-center justify-between">
              <div>
                <h2 class="text-sm font-extrabold text-on-surface">账号信息</h2>
                <p class="mt-1 text-xs text-on-surface-variant">{{ profile?.username || '会员' }}</p>
              </div>
              <span class="rounded-full bg-primary-fixed px-3 py-1 text-[11px] font-bold text-primary">{{ isEmailBound ? '邮箱已绑定' : '邮箱未绑定' }}</span>
            </div>
            <div class="flex items-center gap-3 rounded-2xl bg-surface-container-low p-4">
              <Mail class="h-5 w-5 text-primary" />
              <div class="min-w-0 flex-1">
                <p class="text-xs font-bold text-on-surface-variant">当前邮箱</p>
                <p class="mt-1 truncate text-sm font-bold text-on-surface">{{ profile?.email || '未绑定邮箱' }}</p>
              </div>
              <CheckCircle2 v-if="isEmailBound" class="h-5 w-5 text-primary" />
            </div>
          </section>

          <section v-if="canBindEmail" class="mb-5 space-y-5 rounded-[1.5rem] bg-white p-5 shadow-[0_12px_28px_rgba(26,28,28,0.06)]">
            <div>
              <h2 class="text-sm font-extrabold text-on-surface">绑定邮箱</h2>
              <p class="mt-1 text-xs leading-5 text-on-surface-variant">绑定后可通过邮箱找回密码。</p>
            </div>

            <div class="space-y-2">
              <label class="ml-1 block text-xs font-bold text-on-surface-variant">邮箱地址</label>
              <input
                v-model="bindEmail"
                class="w-full rounded-2xl bg-surface-container-low px-4 py-4 text-sm text-on-surface outline-none transition-all placeholder:text-outline focus:bg-white focus:ring-2 focus:ring-primary/10"
                placeholder="请输入邮箱地址"
                type="email"
              />
            </div>

            <button class="flex w-full items-center justify-center gap-2 rounded-2xl lacquer-gradient py-4 font-headline text-base font-bold !text-on-primary shadow-lg shadow-primary/20 active:scale-[0.98] disabled:opacity-60" type="button" :disabled="bindSubmitting" @click="submitBindEmail">
              <Mail class="h-5 w-5" />
              {{ bindSubmitting ? '绑定中...' : '确认绑定' }}
            </button>
          </section>

          <section v-else-if="!isEmailBound && !bindEmailEnabled" class="mb-5 rounded-[1.5rem] bg-primary-fixed/70 p-5 text-on-primary-fixed">
            <h2 class="text-sm font-extrabold text-primary">邮箱绑定暂未开放</h2>
            <p class="mt-2 text-xs leading-5 text-on-surface-variant">当前站点暂未开启安全中心邮箱绑定。修改密码不受影响，如需协助可联系在线客服。</p>
          </section>

          <section v-else class="mb-5 rounded-[1.5rem] bg-white p-5 shadow-[0_12px_28px_rgba(26,28,28,0.06)]">
            <h2 class="text-sm font-extrabold text-on-surface">邮箱已完成绑定</h2>
            <p class="mt-2 text-xs leading-5 text-on-surface-variant">如需更换邮箱，请联系在线客服核验身份后处理。</p>
          </section>
        </van-tab>

        <van-tab title="修改密码" name="password">
          <section class="space-y-5 rounded-[1.5rem] bg-white p-5 shadow-[0_12px_28px_rgba(26,28,28,0.06)]">
            <div>
              <h2 class="text-sm font-extrabold text-on-surface">修改密码</h2>
              <p class="mt-1 text-xs leading-5 text-on-surface-variant">使用当前密码验证身份，修改后当前设备会自动刷新登录令牌。</p>
            </div>

            <div class="space-y-2">
              <label class="ml-1 block text-xs font-bold text-on-surface-variant">当前密码</label>
              <div class="relative">
                <KeyRound class="absolute left-4 top-1/2 h-5 w-5 -translate-y-1/2 text-outline" />
                <input
                  v-model="currentPassword"
                  class="w-full rounded-2xl bg-surface-container-low py-4 pl-12 pr-4 text-sm text-on-surface outline-none transition-all placeholder:text-outline focus:bg-white focus:ring-2 focus:ring-primary/10"
                  placeholder="请输入当前密码"
                  type="password"
                />
              </div>
            </div>

            <div class="space-y-2">
              <label class="ml-1 block text-xs font-bold text-on-surface-variant">新密码</label>
              <div class="relative">
                <KeyRound class="absolute left-4 top-1/2 h-5 w-5 -translate-y-1/2 text-outline" />
                <input
                  v-model="newPassword"
                  class="w-full rounded-2xl bg-surface-container-low py-4 pl-12 pr-4 text-sm text-on-surface outline-none transition-all placeholder:text-outline focus:bg-white focus:ring-2 focus:ring-primary/10"
                  placeholder="请输入至少 6 位新密码"
                  type="password"
                />
              </div>
            </div>

            <div class="space-y-2">
              <label class="ml-1 block text-xs font-bold text-on-surface-variant">确认新密码</label>
              <div class="relative">
                <ShieldCheck class="absolute left-4 top-1/2 h-5 w-5 -translate-y-1/2 text-outline" />
                <input
                  v-model="confirmPassword"
                  class="w-full rounded-2xl bg-surface-container-low py-4 pl-12 pr-4 text-sm text-on-surface outline-none transition-all placeholder:text-outline focus:bg-white focus:ring-2 focus:ring-primary/10"
                  placeholder="请再次输入新密码"
                  type="password"
                />
              </div>
            </div>

            <button class="flex w-full items-center justify-center gap-2 rounded-2xl bg-primary py-4 font-headline text-base font-bold !text-on-primary shadow-lg shadow-primary/20 active:scale-[0.98] disabled:opacity-60" type="button" :disabled="passwordSubmitting" @click="submitPasswordChange">
              <ShieldCheck class="h-5 w-5" />
              {{ passwordSubmitting ? '修改中...' : '确认修改密码' }}
            </button>
          </section>
        </van-tab>
      </van-tabs>
    </main>
  </div>
</template>

<style scoped>
.security-center-tabs :deep(.van-tabs__wrap) {
  margin-bottom: 1rem;
  border-radius: 1.25rem;
  overflow: hidden;
  box-shadow: 0 12px 28px rgba(26, 28, 28, 0.06);
}

.security-center-tabs :deep(.van-tabs__nav) {
  background: #fff;
}
</style>
