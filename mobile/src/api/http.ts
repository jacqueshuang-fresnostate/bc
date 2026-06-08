import axios from 'axios'
import router from '../router'
import { useAuthStore } from '../stores/auth'

const PACKAGED_API_BASE = 'https://bc.hippo-web3.cc.cd'

function normalizeApiBase(value: unknown) {
  return String(value ?? '').trim().replace(/\/+$/, '')
}

function isTauriLocalOrigin() {
  const { hostname, protocol } = window.location
  return hostname === 'tauri.localhost'
    || hostname.endsWith('.tauri.localhost')
    || protocol === 'tauri:'
    || protocol === 'asset:'
}

function resolveApiBase() {
  const configured = normalizeApiBase(import.meta.env.VITE_API_BASE_URL || import.meta.env.VITE_API_BASE)
  if (configured) return configured

  // Tauri Android 会用 http://tauri.localhost 承载本地页面；
  // 这个来源不能走相对 /api，否则请求会打到本地 WebView 资源服务。
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
  if (err.response?.status === 401 && !originalRequest._retry) {
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
