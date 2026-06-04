import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { UserSummary } from '../api/user'

// 移动端鉴权状态边界：Pinia 保存运行时状态，持久化优先写入 Tauri store，失败时回退 localStorage。
export const useAuthStore = defineStore('auth', () => {
  const accessToken = ref('')
  const refreshToken = ref('')
  const user = ref<UserSummary | null>(null)

  async function loadTokens() {
    try {
      // 桌面容器内从 Tauri store 恢复令牌，保持与原生存储生命周期一致。
      const { load } = await import('@tauri-apps/plugin-store')
      const store = await load('tokens.json')
      const access = await store.get('access_token')
      const storedUser = await store.get('user')
      accessToken.value = typeof access === 'string' ? access : ''
      refreshToken.value = ''
      user.value = isStoredUser(storedUser) ? storedUser : null
    } catch {
      // 浏览器或 Tauri store 不可用时使用 localStorage，保证 Web 端仍可登录态续接。
      accessToken.value = localStorage.getItem('access_token') || ''
      refreshToken.value = ''
      user.value = parseStoredUser(localStorage.getItem('user'))
    }
  }

  async function setSession(access: string, currentUser?: UserSummary | null) {
    // 后端用户登录当前只签发单 token；刷新令牌字段保留为空，方便现有路由守卫继续复用 accessToken。
    accessToken.value = access
    refreshToken.value = ''
    user.value = currentUser || null
    try {
      const { load } = await import('@tauri-apps/plugin-store')
      const store = await load('tokens.json')
      await store.set('access_token', access)
      await store.set('user', currentUser || null)
      await store.save()
    } catch {
      localStorage.setItem('access_token', access)
      if (currentUser) localStorage.setItem('user', JSON.stringify(currentUser))
      else localStorage.removeItem('user')
    }
  }

  async function setTokens(access: string) {
    await setSession(access, user.value)
  }

  async function logout() {
    // 退出时同时清空 token 和用户对象，避免页面继续展示旧用户信息。
    accessToken.value = ''
    refreshToken.value = ''
    user.value = null
    try {
      const { load } = await import('@tauri-apps/plugin-store')
      const store = await load('tokens.json')
      await store.clear()
      await store.save()
    } catch {
      localStorage.removeItem('access_token')
      localStorage.removeItem('refresh_token')
      localStorage.removeItem('user')
    }
  }

  return { accessToken, refreshToken, user, loadTokens, setSession, setTokens, logout }
})

function isStoredUser(value: unknown): value is UserSummary {
  return Boolean(value && typeof value === 'object' && 'id' in value && 'username' in value)
}

function parseStoredUser(value: string | null): UserSummary | null {
  if (!value) return null
  try {
    const parsed = JSON.parse(value)
    return isStoredUser(parsed) ? parsed : null
  } catch {
    return null
  }
}
