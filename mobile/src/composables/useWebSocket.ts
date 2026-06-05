import { ref, onUnmounted, watch } from 'vue'
import { API_BASE } from '../api/http'
import { useAuthStore } from '../stores/auth'
import { normalizeRealtimeEvent } from '../types/realtime'
import type { MobileRealtimeEvent } from '../types/realtime'

export function useWebSocket() {
  const auth = useAuthStore()
  const lastMessage = ref<MobileRealtimeEvent | null>(null)
  const connected = ref(false)
  const lastError = ref('')
  const lastHeartbeatAt = ref('')
  let ws: WebSocket | null = null
  let reconnectTimer: number | undefined
  let stopped = false

  function websocketUrl() {
    const base = API_BASE || window.location.origin
    const url = new URL('/api/user/realtime', base)
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
    if (auth.accessToken) url.searchParams.set('token', auth.accessToken)
    return url.toString()
  }

  function scheduleReconnect() {
    if (stopped || reconnectTimer !== undefined) return
    reconnectTimer = window.setTimeout(() => {
      reconnectTimer = undefined
      connect()
    }, 3000)
  }

  function connect() {
    if (stopped) return
    ws?.close()
    const socket = new WebSocket(websocketUrl())
    ws = socket
    socket.onopen = () => {
      if (ws !== socket) return
      connected.value = true
      lastError.value = ''
    }
    socket.onmessage = (event) => {
      if (ws !== socket) return
      try {
        const message = normalizeRealtimeEvent(JSON.parse(event.data))
        if (!message) return
        if (message.event === 'heartbeat') {
          lastHeartbeatAt.value = message.occurredAt
          return
        }
        lastMessage.value = message
      } catch {
        lastError.value = '实时消息解析失败'
      }
    }
    socket.onerror = () => {
      if (ws !== socket) return
      connected.value = false
      lastError.value = '实时连接异常'
    }
    socket.onclose = () => {
      if (ws !== socket) return
      connected.value = false
      scheduleReconnect()
    }
  }

  connect()

  watch(() => auth.accessToken, () => {
    if (stopped) return
    connected.value = false
    ws?.close()
    connect()
  })

  onUnmounted(() => {
    stopped = true
    if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer)
    ws?.close()
  })

  return { lastMessage, connected, lastError, lastHeartbeatAt }
}
