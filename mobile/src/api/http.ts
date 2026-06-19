import axios from 'axios'
import router from '../router'
import { useAuthStore } from '../stores/auth'

const DEFAULT_PACKAGED_API_BASE = 'https://ad.1666666.site'

function normalizeApiBase(value: unknown) {
  return String(value ?? '').trim().replace(/\/+$/, '')
}

const CONFIGURED_API_BASE = normalizeApiBase(import.meta.env.VITE_API_BASE_URL || import.meta.env.VITE_API_BASE)
const PACKAGED_API_BASE = normalizeApiBase(
  CONFIGURED_API_BASE || DEFAULT_PACKAGED_API_BASE,
)

function isTauriLocalOrigin() {
  const { hostname, protocol } = window.location
  return hostname === 'tauri.localhost'
    || hostname.endsWith('.tauri.localhost')
    || protocol === 'tauri:'
    || protocol === 'asset:'
}

function resolveApiBase() {
  if (CONFIGURED_API_BASE) return CONFIGURED_API_BASE

  // Tauri Android 会用 http://tauri.localhost 承载本地页面；
  // 打包时必须通过 VITE_API_BASE_URL 写入真实后端，避免请求本地 WebView 资源服务。
  if (isTauriLocalOrigin()) return PACKAGED_API_BASE

  return window.location.protocol.startsWith('http') ? '' : PACKAGED_API_BASE
}

const API_BASE = resolveApiBase()
const http = axios.create({ baseURL: `${API_BASE}/api` })

http.interceptors.request.use(config => {
  const auth = useAuthStore()
  if (auth.accessToken) {
    config.headers.Authorization = `Bearer ${auth.accessToken}`
  }
  return config
})

http.interceptors.response.use(res => res, async err => {
  const originalRequest = err.config || {}
  if (shouldLogoutForAuthError(err) && !originalRequest._retry) {
    const auth = useAuthStore()
    originalRequest._retry = true
    await auth.logout()
    if (router.currentRoute.value.path !== '/login') {
      router.replace({ path: '/login', query: { redirect: router.currentRoute.value.fullPath } })
    }
  }
  return Promise.reject(err)
})

export default http
export { API_BASE }

function shouldLogoutForAuthError(err: unknown) {
  const status = Number((err as { response?: { status?: number } })?.response?.status || 0)
  if (status === 401) return true
  if (status !== 403) return false
  const message = responseMessage(err)
  return ['用户账号已停用', '用户账号已锁定', '用户账号未激活'].some(text => message.includes(text))
}

function responseMessage(err: unknown) {
  const data = (err as { response?: { data?: unknown } })?.response?.data
  if (!data || typeof data !== 'object') return ''
  const message = (data as { message?: unknown }).message
  return typeof message === 'string' ? message : ''
}
