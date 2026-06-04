<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Eye, EyeOff } from 'lucide-vue-next'
import { storeToRefs } from 'pinia'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useBrandingStore } from '../stores/branding'
import { showToast } from 'vant'
import { errorMessage, fetchRegisterOptions, loginUser, registerUser } from '../api/user'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const brandingStore = useBrandingStore()
const { branding } = storeToRefs(brandingStore)

const mode = ref<'login' | 'register'>('login')
const regType = ref<'username' | 'email'>('username')
const usernameRegEnabled = ref(true)
const emailRegEnabled = ref(false)
const inviteRequired = ref(false)
const showPassword = ref(false)

function getRedirectPath() {
  const redirect = route.query.redirect
  return typeof redirect === 'string' && redirect.startsWith('/') && !redirect.startsWith('//') ? redirect : '/'
}

function onLogoError() {
  brandingStore['set\u004cogoFallback']()
}

onMounted(async () => {
  try {
    const options = await fetchRegisterOptions()
    usernameRegEnabled.value = options.usernameEnabled
    emailRegEnabled.value = options.emailEnabled
    inviteRequired.value = options.agentInviteRequired
    if (!options.usernameEnabled && options.emailEnabled) regType.value = 'email'
  } catch {}
})

const account = ref('')
const password = ref('')
const email = ref('')
const invitationCode = ref('')
const loading = ref(false)

async function doLogin() {
  if (!account.value.trim()) { showToast('请输入用户名或邮箱'); return }
  if (!password.value) { showToast('请输入密码'); return }
  loading.value = true
  try {
    const session = await loginUser({ loginKey: account.value.trim(), password: password.value })
    await auth.setSession(session.token, session.user)
    router.replace(getRedirectPath())
  } catch (e: unknown) {
    showToast(errorMessage(e, '登录失败'))
  } finally {
    loading.value = false
  }
}

async function doRegister() {
  const invite = invitationCode.value.trim()
  if (inviteRequired.value && !invite) {
    showToast('邀请码必填')
    return
  }
  if (regType.value === 'email') {
    if (!email.value.trim()) { showToast('请输入邮箱'); return }
  } else if (!account.value.trim()) {
    showToast('请输入用户名')
    return
  }
  if (!password.value || password.value.length < 6) { showToast('请输入至少6位密码'); return }
  loading.value = true
  try {
    if (regType.value === 'email') {
      await registerUser({ email: email.value.trim(), password: password.value, inviteCode: invite || undefined })
      account.value = email.value
    } else {
      await registerUser({ username: account.value.trim(), password: password.value, inviteCode: invite || undefined })
    }
    await doLogin()
  } catch (e: unknown) {
    showToast(errorMessage(e, '注册失败'))
    loading.value = false
  }
}
</script>

<template>
  <div class="bg-surface font-body text-on-surface flex flex-col min-h-screen relative overflow-hidden">
    <div class="absolute inset-0 opacity-[0.03] pointer-events-none select-none overflow-hidden">
      <div class="grid grid-cols-12 gap-4 w-full h-full">
        <div v-for="i in 144" :key="i" class="w-1 h-1 bg-primary rounded-full"></div>
      </div>
    </div>

    <main class="flex-grow pt-32 pb-20 px-6 flex flex-col items-center justify-center relative z-10">
      <div class="w-full max-w-md">
        <div class="mb-12 text-center">
          <div class="inline-block relative">
            <div class="absolute inset-0 bg-primary/10 blur-3xl rounded-full"></div>
            <img
              :alt="`${branding.site_name} 标志`"
              class="relative w-24 h-24 object-cover rounded-full border-2 border-primary-fixed shadow-xl mx-auto"
              :src="branding.logo_url"
              @error="onLogoError"
            />
          </div>
          <h2 class="mt-4 font-headline text-4xl font-black tracking-tighter text-primary italic">{{ branding.site_name }}</h2>
          <p class="mt-2 text-on-surface-variant font-medium tracking-widest uppercase text-xs">{{ branding.slogan }}</p>
        </div>

        <div class="bg-surface-container-lowest p-10 rounded-[2rem] shadow-2xl shadow-primary/5 relative overflow-hidden ring-1 ring-black/5">
          <form v-if="mode === 'login'" class="space-y-8" @submit.prevent="doLogin">
            <div class="space-y-2">
              <label class="block font-label text-sm font-bold text-on-surface-variant ml-1">用户名或邮箱</label>
              <input
                v-model="account"
                class="w-full px-6 py-4 bg-surface-container-low border-none rounded-2xl focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                placeholder="请输入您的账号"
                type="text"
              />
            </div>

            <div class="space-y-2">
              <div class="flex justify-between items-center px-1">
                <label class="block font-label text-sm font-bold text-on-surface-variant">密码</label>
                <button
                  type="button"
                  class="text-xs font-bold text-primary"
                  @click="router.push('/forgot-password')"
                >
                  忘记密码
                </button>
              </div>
              <div class="relative group">
                <input
                  v-model="password"
                  :type="showPassword ? 'text' : 'password'"
                  class="w-full px-6 py-4 bg-surface-container-low border-none rounded-2xl focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                  placeholder="请输入您的密码"
                />
                <button
                  type="button"
                  class="absolute right-5 top-1/2 -translate-y-1/2 text-outline hover:text-primary transition-colors"
                  @click="showPassword = !showPassword"
                >
                  <Eye v-if="!showPassword" class="w-5 h-5" />
                  <EyeOff v-else class="w-5 h-5" />
                </button>
              </div>
            </div>

            <button
              class="w-full lacquer-gradient !text-on-primary font-headline font-bold py-5 rounded-2xl shadow-xl shadow-primary/30 active:scale-[0.98] transition-all duration-300 cursor-pointer text-lg disabled:opacity-60"
              type="submit"
              :disabled="loading"
            >
              {{ loading ? '登录中...' : '登录' }}
            </button>

            <div class="flex items-center gap-4 py-2">
              <div class="flex-grow h-[1px] bg-outline-variant/30"></div>
              <span class="text-[10px] font-bold tracking-widest text-outline uppercase">或者</span>
              <div class="flex-grow h-[1px] bg-outline-variant/30"></div>
            </div>

            <div class="text-center">
              <p class="text-sm font-medium text-on-surface-variant">
                还没有账号？
                <button
                  type="button"
                  class="text-primary font-bold hover:underline ml-1 cursor-pointer"
                  @click="mode = 'register'"
                >
                  立即注册
                </button>
              </p>
            </div>
          </form>

          <form v-else class="space-y-6" @submit.prevent="doRegister">
            <div v-if="usernameRegEnabled && emailRegEnabled" class="grid grid-cols-2 gap-2 rounded-2xl bg-surface-container-low p-1">
              <button
                type="button"
                class="rounded-xl py-3 text-sm font-bold transition-all"
                :class="regType === 'username' ? 'bg-white text-primary shadow-sm' : 'text-on-surface-variant'"
                @click="regType = 'username'"
              >
                用户名注册
              </button>
              <button
                type="button"
                class="rounded-xl py-3 text-sm font-bold transition-all"
                :class="regType === 'email' ? 'bg-white text-primary shadow-sm' : 'text-on-surface-variant'"
                @click="regType = 'email'"
              >
                邮箱注册
              </button>
            </div>

            <div v-if="regType === 'username' && usernameRegEnabled" class="space-y-2">
              <label class="block font-label text-sm font-bold text-on-surface-variant ml-1">用户名</label>
              <input
                v-model="account"
                class="w-full px-6 py-4 bg-surface-container-low border-none rounded-2xl focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                placeholder="请输入用户名"
                type="text"
              />
            </div>

            <template v-else>
              <div class="space-y-2">
                <label class="block font-label text-sm font-bold text-on-surface-variant ml-1">邮箱</label>
                <input
                  v-model="email"
                  class="w-full px-6 py-4 bg-surface-container-low border-none rounded-2xl focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                  placeholder="请输入邮箱"
                  type="email"
                />
              </div>
            </template>

            <div class="space-y-2">
              <label class="block font-label text-sm font-bold text-on-surface-variant ml-1">密码</label>
              <div class="relative group">
                <input
                  v-model="password"
                  :type="showPassword ? 'text' : 'password'"
                  class="w-full px-6 py-4 bg-surface-container-low border-none rounded-2xl focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                  placeholder="请输入密码（至少6位）"
                />
                <button
                  type="button"
                  class="absolute right-5 top-1/2 -translate-y-1/2 text-outline hover:text-primary transition-colors"
                  @click="showPassword = !showPassword"
                >
                  <Eye v-if="!showPassword" class="w-5 h-5" />
                  <EyeOff v-else class="w-5 h-5" />
                </button>
              </div>
            </div>

            <div class="space-y-2">
              <label class="block font-label text-sm font-bold text-on-surface-variant ml-1">邀请码</label>
              <input
                v-model="invitationCode"
                class="w-full px-6 py-4 bg-surface-container-low border-none rounded-2xl focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                :placeholder="inviteRequired ? '请输入上级邀请码' : '选填上级邀请码'"
                maxlength="16"
                type="text"
              />
            </div>

            <button
              class="w-full lacquer-gradient !text-on-primary font-headline font-bold py-5 rounded-2xl shadow-xl shadow-primary/30 active:scale-[0.98] transition-all duration-300 cursor-pointer text-lg disabled:opacity-60"
              type="submit"
              :disabled="loading"
            >
              {{ loading ? '注册中...' : '注册' }}
            </button>

            <div class="text-center">
              <button
                type="button"
                class="text-primary font-bold hover:underline cursor-pointer"
                @click="mode = 'login'"
              >
                返回登录
              </button>
            </div>
          </form>
        </div>
      </div>
    </main>

    <footer class="w-full py-16 mt-auto relative z-10">
      <div class="flex flex-col items-center gap-8 max-w-4xl mx-auto text-center">
        <div class="w-px h-16 bg-primary/20"></div>
        <p class="font-headline text-[10px] tracking-[0.4em] uppercase font-bold text-outline">
          {{ branding.footer_text }}
        </p>
      </div>
    </footer>
  </div>
</template>
