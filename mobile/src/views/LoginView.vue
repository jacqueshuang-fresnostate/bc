<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
import { Eye, EyeOff } from 'lucide-vue-next'
import { storeToRefs } from 'pinia'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'
import { useBrandingStore } from '../stores/branding'
import { showToast } from 'vant'
import { errorMessage, fetchRegisterOptions, loginUser, registerUser } from '../api/user'
import CachedRemoteImage from '../components/mobile/CachedRemoteImage.vue'
import LoginPageSkeleton from '../components/mobile/LoginPageSkeleton.vue'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const brandingStore = useBrandingStore()
const { branding, loaded: brandingLoaded } = storeToRefs(brandingStore)

const mode = ref<'login' | 'register'>('login')
const regType = ref<'username' | 'email'>('username')
const usernameRegEnabled = ref(true)
const emailRegEnabled = ref(false)
const inviteRequired = ref(false)
const showPassword = ref(false)
const registerOptionsLoading = ref(true)
const showPageSkeleton = computed(() => registerOptionsLoading.value || !brandingLoaded.value)

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
  } catch {
    // 注册入口配置读取失败时保留默认用户名注册，保证登录页仍可使用。
  } finally {
    registerOptionsLoading.value = false
  }
})

const account = ref('')
const password = ref('')
const email = ref('')
const invitationCode = ref('')
const contactQq = ref('')
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
  const qq = contactQq.value.trim()
  if (inviteRequired.value && !invite) {
    showToast('邀请码必填')
    return
  }
  if (qq && (!/^\d+$/.test(qq) || qq.length < 5 || qq.length > 12)) {
    showToast('QQ 号码需要是 5-12 位数字')
    return
  }
  if (regType.value === 'email') {
    if (!email.value.trim()) { showToast('请输入邮箱'); return }
  } else if (!account.value.trim()) {
    showToast('请输入用户名')
    return
  }
  if (!password.value || password.value.length < 8) { showToast('请输入至少8位密码'); return }
  loading.value = true
  try {
    if (regType.value === 'email') {
      await registerUser({
        contactQq: qq || undefined,
        email: email.value.trim(),
        password: password.value,
        inviteCode: invite || undefined,
      })
      account.value = email.value
    } else {
      await registerUser({
        contactQq: qq || undefined,
        username: account.value.trim(),
        password: password.value,
        inviteCode: invite || undefined,
      })
    }
    await doLogin()
  } catch (e: unknown) {
    showToast(errorMessage(e, '注册失败'))
    loading.value = false
  }
}
</script>

<template>
  <div class="auth-page bg-surface font-body text-on-surface relative flex h-full max-h-full flex-col overflow-hidden">
    <div class="absolute inset-0 opacity-[0.03] pointer-events-none select-none overflow-hidden">
      <div class="grid grid-cols-12 gap-4 w-full h-full">
        <div v-for="i in 144" :key="i" class="w-1 h-1 bg-primary rounded-full"></div>
      </div>
    </div>

    <main class="auth-main relative z-10 flex min-h-0 flex-1 items-center justify-center px-5 py-4">
      <LoginPageSkeleton v-if="showPageSkeleton" />

      <div v-else class="w-full max-w-sm">
        <div class="auth-brand text-center">
          <div class="inline-block relative">
            <div class="absolute inset-0 bg-primary/10 blur-3xl rounded-full"></div>
            <CachedRemoteImage
              :alt="`${branding.site_name} 标志`"
              class="auth-logo relative mx-auto rounded-full border-2 border-primary-fixed object-cover shadow-xl"
              :src="branding.logo_url"
              @error="onLogoError"
            />
          </div>
          <h2 class="auth-title font-headline font-black tracking-tight text-primary italic">{{ branding.site_name }}</h2>
          <p class="auth-slogan truncate text-on-surface-variant font-medium">{{ branding.slogan }}</p>
        </div>

        <div class="auth-card bg-surface-container-lowest relative overflow-hidden rounded-[1.5rem] shadow-xl shadow-primary/5 ring-1 ring-black/5">
          <form v-if="mode === 'login'" class="auth-form" @submit.prevent="doLogin">
            <div class="auth-field">
              <label class="auth-label block font-label font-bold text-on-surface-variant">用户名或邮箱</label>
              <input
                v-model="account"
                class="auth-input w-full bg-surface-container-low border-none focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                placeholder="请输入您的账号"
                type="text"
              />
            </div>

            <div class="auth-field">
              <div class="flex justify-between items-center px-1">
                <label class="auth-label block font-label font-bold text-on-surface-variant">密码</label>
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
                  class="auth-input w-full bg-surface-container-low border-none focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
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
              class="auth-primary-button w-full lacquer-gradient !text-on-primary font-headline font-bold shadow-xl shadow-primary/30 active:scale-[0.98] transition-all duration-300 cursor-pointer disabled:opacity-60"
              type="submit"
              :disabled="loading"
            >
              {{ loading ? '登录中...' : '登录' }}
            </button>

            <div class="auth-separator flex items-center gap-3">
              <div class="flex-grow h-[1px] bg-outline-variant/30"></div>
              <span class="text-[10px] font-bold text-outline">或者</span>
              <div class="flex-grow h-[1px] bg-outline-variant/30"></div>
            </div>

            <div class="text-center">
              <p class="auth-switch-text font-medium text-on-surface-variant">
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

          <form v-else class="auth-form" @submit.prevent="doRegister">
            <div v-if="usernameRegEnabled && emailRegEnabled" class="grid grid-cols-2 gap-2 rounded-xl bg-surface-container-low p-1">
              <button
                type="button"
                class="auth-segment-button rounded-lg font-bold transition-all"
                :class="regType === 'username' ? 'bg-white text-primary shadow-sm' : 'text-on-surface-variant'"
                @click="regType = 'username'"
              >
                用户名注册
              </button>
              <button
                type="button"
                class="auth-segment-button rounded-lg font-bold transition-all"
                :class="regType === 'email' ? 'bg-white text-primary shadow-sm' : 'text-on-surface-variant'"
                @click="regType = 'email'"
              >
                邮箱注册
              </button>
            </div>

            <div v-if="regType === 'username' && usernameRegEnabled" class="auth-field">
              <label class="auth-label block font-label font-bold text-on-surface-variant">用户名</label>
              <input
                v-model="account"
                class="auth-input w-full bg-surface-container-low border-none focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                placeholder="请输入用户名"
                type="text"
              />
            </div>

            <template v-else>
              <div class="auth-field">
                <label class="auth-label block font-label font-bold text-on-surface-variant">邮箱</label>
                <input
                  v-model="email"
                  class="auth-input w-full bg-surface-container-low border-none focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                  placeholder="请输入邮箱"
                  type="email"
                />
              </div>
            </template>

            <div class="auth-field">
              <label class="auth-label block font-label font-bold text-on-surface-variant">密码</label>
              <div class="relative group">
                <input
                  v-model="password"
                  :type="showPassword ? 'text' : 'password'"
                  class="auth-input w-full bg-surface-container-low border-none focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                  placeholder="请输入密码（至少8位）"
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

            <div class="grid grid-cols-2 gap-2">
              <div class="auth-field">
                <label class="auth-label block font-label font-bold text-on-surface-variant">邀请码</label>
                <input
                  v-model="invitationCode"
                  class="auth-input w-full bg-surface-container-low border-none focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                  :placeholder="inviteRequired ? '必填' : '选填'"
                  maxlength="16"
                  type="text"
                />
              </div>

              <div class="auth-field">
                <label class="auth-label block font-label font-bold text-on-surface-variant">QQ</label>
                <input
                  v-model="contactQq"
                  class="auth-input w-full bg-surface-container-low border-none focus:ring-2 focus:ring-primary/10 focus:bg-white transition-all duration-300 text-on-surface outline-none placeholder:text-outline"
                  inputmode="numeric"
                  maxlength="12"
                  placeholder="选填"
                  type="text"
                />
              </div>
            </div>

            <button
              class="auth-primary-button w-full lacquer-gradient !text-on-primary font-headline font-bold shadow-xl shadow-primary/30 active:scale-[0.98] transition-all duration-300 cursor-pointer disabled:opacity-60"
              type="submit"
              :disabled="loading"
            >
              {{ loading ? '注册中...' : '注册' }}
            </button>

            <div class="text-center">
              <button
                type="button"
                class="auth-switch-text text-primary font-bold hover:underline cursor-pointer"
                @click="mode = 'login'"
              >
                返回登录
              </button>
            </div>
          </form>
        </div>
      </div>
    </main>
  </div>
</template>

<style scoped>
.auth-main {
  padding-top: max(0.75rem, env(safe-area-inset-top));
  padding-bottom: max(0.75rem, env(safe-area-inset-bottom));
}

.auth-brand {
  margin-bottom: 1rem;
}

.auth-logo {
  width: 4.5rem;
  height: 4.5rem;
}

.auth-title {
  margin-top: 0.625rem;
  font-size: 2rem;
  line-height: 1;
}

.auth-slogan {
  margin-top: 0.375rem;
  font-size: 0.75rem;
  line-height: 1.25rem;
}

.auth-card {
  padding: 1.25rem;
}

.auth-form {
  display: grid;
  gap: 0.875rem;
}

.auth-field {
  display: grid;
  gap: 0.375rem;
}

.auth-label {
  margin-left: 0.25rem;
  font-size: 0.8125rem;
}

.auth-input {
  min-height: 3rem;
  border-radius: 1rem;
  padding: 0.75rem 1rem;
}

.auth-primary-button {
  min-height: 3rem;
  border-radius: 1rem;
  font-size: 1rem;
}

.auth-separator {
  min-height: 1rem;
}

.auth-switch-text {
  font-size: 0.875rem;
  line-height: 1.25rem;
}

.auth-segment-button {
  min-height: 2.5rem;
  font-size: 0.8125rem;
}

@media (max-height: 720px) {
  .auth-brand {
    margin-bottom: 0.75rem;
  }

  .auth-logo {
    width: 3.75rem;
    height: 3.75rem;
  }

  .auth-title {
    margin-top: 0.5rem;
    font-size: 1.75rem;
  }

  .auth-slogan {
    margin-top: 0.25rem;
    font-size: 0.6875rem;
    line-height: 1rem;
  }

  .auth-card {
    padding: 1rem;
    border-radius: 1.25rem;
  }

  .auth-form {
    gap: 0.625rem;
  }

  .auth-input,
  .auth-primary-button {
    min-height: 2.625rem;
  }

  .auth-input {
    border-radius: 0.875rem;
    padding-top: 0.625rem;
    padding-bottom: 0.625rem;
  }

  .auth-segment-button {
    min-height: 2.25rem;
  }
}
</style>
