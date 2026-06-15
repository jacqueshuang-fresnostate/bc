import { ref, onUnmounted, watch } from 'vue'
import { API_BASE } from '../api/http'
import { useAuthStore } from '../stores/auth'
import { normalizeRealtimeEvent } from '../types/realtime'
import type { MobileRealtimeEvent } from '../types/realtime'

const INITIAL_RECONNECT_DELAY_MS = 1000
const MAX_RECONNECT_DELAY_MS = 30000
const MAX_RECONNECT_ATTEMPTS = 8
const HEARTBEAT_TIMEOUT_MS = 75000

export function useWebSocket() {
  const auth = useAuthStore()
  const lastMessage = ref<MobileRealtimeEvent | null>(null)
  const connected = ref(false)
  const lastError = ref('')
  const lastHeartbeatAt = ref('')
  let ws: WebSocket | null = null
  let reconnectTimer: number | undefined
  let heartbeatTimer: number | undefined
  let reconnectAttempts = 0
  let lastInboundAt = Date.now()
  let stopped = false

  function websocketUrl() {
    const base = API_BASE || window.location.origin
    const url = new URL('/api/user/realtime', base)
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:'
    if (auth.accessToken) url.searchParams.set('token', auth.accessToken)
    return url.toString()
  }

  function scheduleReconnect() {
    if (stopped || reconnectTimer !== undefined || document.visibilityState === 'hidden') return
    if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
      lastError.value = '实时连接暂时不可用，请稍后返回页面自动重试'
      return
    }
    const delay = Math.min(
      INITIAL_RECONNECT_DELAY_MS * 2 ** reconnectAttempts,
      MAX_RECONNECT_DELAY_MS,
    ) + Math.floor(Math.random() * 600)
    reconnectAttempts += 1
    reconnectTimer = window.setTimeout(() => {
      reconnectTimer = undefined
      connect()
    }, delay)
  }

  function clearReconnectTimer() {
    if (reconnectTimer === undefined) return
    window.clearTimeout(reconnectTimer)
    reconnectTimer = undefined
  }

  function clearHeartbeatTimer() {
    if (heartbeatTimer === undefined) return
    window.clearInterval(heartbeatTimer)
    heartbeatTimer = undefined
  }

  function startHeartbeatWatchdog(socket: WebSocket) {
    clearHeartbeatTimer()
    lastInboundAt = Date.now()
    heartbeatTimer = window.setInterval(() => {
      if (stopped || ws !== socket) return
      if (Date.now() - lastInboundAt <= HEARTBEAT_TIMEOUT_MS) return
      lastError.value = '实时连接心跳超时，正在重连'
      socket.close()
    }, 10000)
  }

  function connect() {
    if (stopped || document.visibilityState === 'hidden') return
    ws?.close()
    const socket = new WebSocket(websocketUrl())
    ws = socket
    socket.onopen = () => {
      if (ws !== socket) return
      connected.value = true
      lastError.value = ''
      reconnectAttempts = 0
      startHeartbeatWatchdog(socket)
    }
    socket.onmessage = (event) => {
      if (ws !== socket) return
      lastInboundAt = Date.now()
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
      clearHeartbeatTimer()
      scheduleReconnect()
    }
  }

  function reconnectNow() {
    if (stopped) return
    clearReconnectTimer()
    reconnectAttempts = 0
    connected.value = false
    ws?.close()
    connect()
  }

  connect()

  watch(() => auth.accessToken, () => {
    if (stopped) return
    reconnectNow()
  })

  function handleVisibilityChange() {
    if (document.visibilityState === 'hidden') {
      clearReconnectTimer()
      clearHeartbeatTimer()
      connected.value = false
      ws?.close()
      return
    }
    reconnectNow()
  }

  document.addEventListener('visibilitychange', handleVisibilityChange)

  onUnmounted(() => {
    stopped = true
    document.removeEventListener('visibilitychange', handleVisibilityChange)
    clearReconnectTimer()
    clearHeartbeatTimer()
    ws?.close()
  })

  return { lastMessage, connected, lastError, lastHeartbeatAt }
}
