<script setup lang="ts">
import { ref } from 'vue'
import { storeToRefs } from 'pinia'
import { useRouter } from 'vue-router'
import { showToast } from 'vant'
import { ArrowLeft, Headset, Info, LockKeyhole, Mail, ShieldCheck } from 'lucide-vue-next'
import { useBrandingStore } from '../stores/branding'
import { errorMessage, requestPasswordReset, resetUserPassword } from '../api/user'

const router = useRouter()
const brandingStore = useBrandingStore()
const { branding } = storeToRefs(brandingStore)

const email = ref('')
const newPassword = ref('')
const confirmPassword = ref('')
const resetting = ref(false)

async function resetPassword() {
  if (!email.value) { showToast('请输入用户名或邮箱'); return }
  if (!newPassword.value || newPassword.value.length < 6) { showToast('请输入至少 6 位新密码'); return }
  if (newPassword.value !== confirmPassword.value) { showToast('两次输入的密码不一致'); return }
  resetting.value = true
  try {
    const reset = await requestPasswordReset(email.value.trim())
    await resetUserPassword(reset.resetToken, newPassword.value)
    showToast('密码已重置，请重新登录')
    router.replace('/login')
  } catch (e: unknown) {
    showToast(errorMessage(e, '重置失败'))
  } finally {
    resetting.value = false
  }
}
</script>

<template>
  <div class="min-h-screen bg-background text-on-surface font-body">
    <header class="mobile-safe-compact-header sticky top-0 z-40 flex h-14 items-center justify-between bg-white/90 px-4 shadow-[0_1px_0_rgba(140,10,21,0.06)] backdrop-blur-md">
      <button class="flex h-9 w-9 items-center justify-center rounded-full bg-surface-container-low text-on-surface-variant active:bg-surface-container" @click="router.back()">
        <ArrowLeft class="h-5 w-5" />
      </button>
      <span class="font-headline text-sm font-extrabold text-on-surface">身份验证</span>
      <span class="h-9 w-9"></span>
    </header>

    <main class="mx-auto max-w-md px-5 pb-12 pt-5">
      <section class="mb-5 overflow-hidden rounded-[1.75rem] lacquer-gradient p-6 !text-on-primary shadow-[0_16px_32px_rgba(140,10,21,0.22)]">
        <div class="mb-8 flex items-center justify-between">
          <div class="flex h-12 w-12 items-center justify-center rounded-2xl bg-white/15">
            <LockKeyhole class="h-6 w-6" />
          </div>
          <div class="rounded-full bg-white/15 px-3 py-1 text-[10px] font-bold tracking-[0.18em]">SECURE</div>
        </div>
        <h1 class="font-headline text-3xl font-black tracking-tight">找回密码</h1>
        <p class="mt-2 text-sm font-medium text-white/85">通过用户名或邮箱重置您的安全密钥</p>
      </section>

      <section class="mb-5 rounded-[1.35rem] bg-surface-container-low p-1.5">
        <div class="grid grid-cols-2 gap-1 rounded-[1.05rem] bg-white p-1">
          <div class="rounded-xl bg-primary-fixed px-3 py-3 text-center text-primary">
            <span class="mx-auto mb-1 flex h-7 w-7 items-center justify-center rounded-full bg-primary text-xs font-bold !text-on-primary">1</span>
            <span class="text-xs font-bold">身份验证</span>
          </div>
          <div class="rounded-xl bg-primary-fixed px-3 py-3 text-center text-primary">
            <span class="mx-auto mb-1 flex h-7 w-7 items-center justify-center rounded-full bg-primary text-xs font-bold !text-on-primary">2</span>
            <span class="text-xs font-bold">重置密码</span>
          </div>
        </div>
      </section>

      <section class="space-y-5 rounded-[1.5rem] bg-white p-5 shadow-[0_12px_28px_rgba(26,28,28,0.06)]">
        <div class="space-y-2">
          <label class="ml-1 block text-xs font-bold text-on-surface-variant">用户名或邮箱</label>
          <div class="relative">
            <Mail class="absolute left-4 top-1/2 h-5 w-5 -translate-y-1/2 text-outline" />
            <input
              v-model="email"
              class="w-full rounded-2xl bg-surface-container-low py-4 pl-12 pr-4 text-sm text-on-surface outline-none transition-all placeholder:text-outline focus:bg-white focus:ring-2 focus:ring-primary/10"
              placeholder="请输入用户名或绑定邮箱"
              type="text"
            />
          </div>
        </div>

        <div class="space-y-2">
          <label class="ml-1 block text-xs font-bold text-on-surface-variant">新密码</label>
          <div class="relative">
            <LockKeyhole class="absolute left-4 top-1/2 h-5 w-5 -translate-y-1/2 text-outline" />
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

        <button class="flex w-full items-center justify-center gap-2 rounded-2xl lacquer-gradient py-4 font-headline text-base font-bold !text-on-primary shadow-lg shadow-primary/20 active:scale-[0.98] disabled:opacity-60" :disabled="resetting" @click="resetPassword">
          <ShieldCheck class="h-5 w-5" />
          {{ resetting ? '重置中...' : '重置密码' }}
        </button>

        <button class="flex w-full items-center justify-center gap-2 rounded-2xl bg-surface-container-low py-3 text-sm font-bold text-primary active:bg-surface-container" @click="router.push('/support')">
          <Headset class="h-5 w-5" />
          联系人工客服
        </button>
      </section>

      <section class="mt-5 flex gap-3 rounded-[1.35rem] bg-primary-fixed/70 p-4 text-on-primary-fixed">
        <Info class="mt-0.5 h-5 w-5 shrink-0 text-primary" />
        <div>
          <h2 class="text-sm font-bold text-primary">安全提示</h2>
          <p class="mt-1 text-xs leading-5 text-on-surface-variant">请确认账号信息属于本人。若无法完成密码重置，请联系人工客服协助处理。</p>
        </div>
      </section>
    </main>

    <footer class="px-6 pb-10 text-center">
      <div class="mb-3 flex items-center justify-center gap-4 text-[11px] font-bold text-primary">
        <span>隐私政策</span>
        <span>用户协议</span>
      </div>
      <p class="font-headline text-[10px] font-bold uppercase tracking-[0.35em] text-outline">{{ branding.footer_text }}</p>
    </footer>
  </div>
</template>
