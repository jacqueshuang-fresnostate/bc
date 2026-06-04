import axios from 'axios'
import router from '../router'
import { useAuthStore } from '../stores/auth'

const API_BASE = import.meta.env.VITE_API_BASE_URL
  || import.meta.env.VITE_API_BASE
  || (window.location.protocol.startsWith('http') ? '' : 'https://bc.hippo-web3.cc.cd')
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
