import { ref, onUnmounted } from 'vue'
import { API_BASE } from '../api/http'

export function useWebSocket() {
  const lastMessage = ref<any>(null)
  const connected = ref(false)
  const lastError = ref('')
  let ws: WebSocket | null = null
  let reconnectTimer: number | undefined
  let stopped = false

  function websocketUrl() {
    return API_BASE.replace(/^http/, 'ws') + '/ws/lottery'
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
        const message = JSON.parse(event.data)
        if (message?.event === 'ping') return
        lastMessage.value = message
      } catch {
        lastError.value = '开奖推送消息解析失败'
      }
    }
    socket.onerror = () => {
      if (ws !== socket) return
      connected.value = false
      lastError.value = '开奖推送连接异常'
    }
    socket.onclose = () => {
      if (ws !== socket) return
      connected.value = false
      scheduleReconnect()
    }
  }

  connect()

  onUnmounted(() => {
    stopped = true
    if (reconnectTimer !== undefined) window.clearTimeout(reconnectTimer)
    ws?.close()
  })

  return { lastMessage, connected, lastError }
}
