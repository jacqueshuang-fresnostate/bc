<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { showToast } from 'vant'
import { useRouter } from 'vue-router'
import { API_BASE } from '../api/http'
import { useAuthStore } from '../stores/auth'
import http from '../api/http'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { formatDateTime } from '../utils/lotteryFormat'

const router = useRouter()
const auth = useAuthStore()
const draft = ref('')
const loading = ref(false)
const sending = ref(false)
const uploading = ref(false)
const adminOnline = ref(false)
const messages = ref<any[]>([])
const imageObjectUrls = ref<Record<number, string>>({})
const fileInput = ref<HTMLInputElement | null>(null)
let ws: WebSocket | null = null
let reconnectTimer: number | null = null

async function getAccessToken() {
  if (auth.accessToken) return auth.accessToken
  await auth.loadTokens()
  return auth.accessToken
}

const canSend = computed(() => draft.value.trim().length > 0 && !sending.value)
const supportStatusText = computed(() => (adminOnline.value ? '客服在线' : '客服暂时不在线，留言后会通知客服'))
const hasMessages = computed(() => messages.value.length > 0)

function formatTime(value: string) {
  return formatDateTime(value)
}

function apiOrigin() {
  return new URL(API_BASE || window.location.origin, window.location.origin).origin
}

function isPublicImageUrl(path: string) {
  return /^https?:\/\//i.test(String(path || '').trim())
}

function protectedImagePath(item: any) {
  const path = String(item?.image_url || '').trim()
  if (!path) return ''
  try {
    const url = new URL(path, API_BASE || window.location.origin)
    const absolute = isPublicImageUrl(path)
    if (absolute && url.origin !== apiOrigin()) return ''
    if (url.pathname.startsWith('/api/support/files/')) {
      return url.pathname.slice(4) + url.search
    }
    if (url.pathname.startsWith('/support/files/')) {
      return url.pathname + url.search
    }
  } catch {
    return ''
  }
  return ''
}

function messageImageUrl(item: any) {
  const path = String(item?.image_url || '').trim()
  if (!path || protectedImagePath(item)) return ''
  if (isPublicImageUrl(path)) return path
  return `${API_BASE}${path.startsWith('/') ? path : `/${path}`}`
}

function displayImageUrl(item: any) {
  if (imageObjectUrls.value[item.id]) return imageObjectUrls.value[item.id]
  return protectedImagePath(item) ? '' : messageImageUrl(item)
}

function clearImageObjectUrls() {
  for (const url of Object.values(imageObjectUrls.value)) {
    URL.revokeObjectURL(url)
  }
  imageObjectUrls.value = {}
}

async function hydrateImageObjectUrls(items: any[]) {
  const nextUrls: Record<number, string> = {}
  try {
    for (const item of items) {
      if (!isImageMessage(item)) continue
      const path = protectedImagePath(item)
      if (!path) continue
      const res = await http.get(path, { responseType: 'blob' })
      nextUrls[item.id] = URL.createObjectURL(res.data)
    }
    clearImageObjectUrls()
    imageObjectUrls.value = nextUrls
  } catch (error) {
    for (const url of Object.values(nextUrls)) {
      URL.revokeObjectURL(url)
    }
    throw error
  }
}

function isImageMessage(item: any) {
  return item?.message_type === 'image' && !!item?.image_url
}

async function loadMessages() {
  loading.value = true
  try {
    const res = await http.get('/support/messages')
    const nextMessages = res.data.messages || []
    messages.value = nextMessages
    await hydrateImageObjectUrls(nextMessages)
    adminOnline.value = !!res.data.admin_online
  } catch (e: any) {
    showToast(e.response?.data?.detail || '加载消息失败')
  } finally {
    loading.value = false
  }
}

async function sendMessage() {
  const content = draft.value.trim()
  if (!content) return
  sending.value = true
  try {
    await http.post('/support/messages', { content })
    draft.value = ''
    await loadMessages()
  } catch (e: any) {
    showToast(e.response?.data?.detail || '发送失败')
  } finally {
    sending.value = false
  }
}

function triggerImagePick() {
  if (uploading.value) return
  fileInput.value?.click()
}

async function uploadImage(file: File) {
  const form = new FormData()
  form.append('file', file)
  uploading.value = true
  try {
    await http.post('/support/messages/image', form, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })
    await loadMessages()
  } catch (e: any) {
    showToast(e.response?.data?.detail || '图片发送失败')
  } finally {
    uploading.value = false
  }
}

async function handleFileChange(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  await uploadImage(file)
}

function previewImage(item: any) {
  const url = displayImageUrl(item)
  if (!url) return
  window.open(url, '_blank')
}

async function connectWs() {
  const token = await getAccessToken()
  if (!token) return
  ws?.close()
  const wsUrl = `${API_BASE.replace(/^http/, 'ws')}/ws/support/user?token=${encodeURIComponent(token)}`
  ws = new WebSocket(wsUrl)
  ws.onmessage = async (event) => {
    const payload = JSON.parse(event.data)
    if (payload?.event === 'support_message_created') {
      await loadMessages()
      return
    }
    if (payload?.event === 'support_admin_presence') {
      adminOnline.value = !!payload.online
    }
  }
  ws.onclose = () => {
    if (reconnectTimer !== null) window.clearTimeout(reconnectTimer)
    reconnectTimer = window.setTimeout(() => { void connectWs() }, 3000)
  }
}

onMounted(async () => {
  await loadMessages()
  await connectWs()
})

onUnmounted(() => {
  if (reconnectTimer !== null) window.clearTimeout(reconnectTimer)
  ws?.close()
  clearImageObjectUrls()
})
</script>

<template>
  <div class="support-chat">
    <header class="support-chat__topbar">
      <button class="support-chat__icon-button" aria-label="返回" @click="router.back()">
        <LucideIcon name="arrow_back" />
      </button>
      <div class="support-chat__title-group">
        <h1>在线客服</h1>
        <p :class="adminOnline ? 'is-online' : 'is-offline'">{{ supportStatusText }}</p>
      </div>
      <span class="support-chat__presence" :class="adminOnline ? 'is-online' : 'is-offline'" aria-hidden="true"></span>
    </header>

    <main class="support-chat__messages" :class="{ 'is-empty': !hasMessages && !loading }">
      <div class="support-chat__date-pill">
        {{ messages[0]?.created_at ? formatTime(messages[0].created_at) : '今天' }}
      </div>

      <div v-if="loading" class="support-chat__loading">正在同步客服消息...</div>
      <div v-else-if="messages.length === 0" class="support-chat__empty">
        <div class="support-chat__empty-icon"><LucideIcon name="support_agent" /></div>
        <h2>还没有聊天记录</h2>
        <p>发送问题后，客服会在这里继续回复。</p>
      </div>

      <div
        v-for="item in messages"
        :key="item.id"
        :class="[
          'support-chat__row',
          item.sender_type === 'user' ? 'support-chat__row--user' : 'support-chat__row--agent',
        ]"
      >
        <div v-if="item.sender_type !== 'user'" class="support-chat__avatar">
          <LucideIcon name="support_agent" />
        </div>

        <div
          :class="[
            'support-chat__bubble',
            item.sender_type === 'user'
              ? 'support-chat__bubble--user'
              : 'support-chat__bubble--agent',
          ]"
        >
          <div class="support-chat__meta">
            <span>{{ item.sender_type === 'user' ? '我' : '客服' }}</span>
            <time>{{ formatTime(item.created_at) }}</time>
          </div>
          <template v-if="isImageMessage(item)">
            <img :src="displayImageUrl(item)" class="support-chat__image" alt="客服会话图片" @click="previewImage(item)" />
            <div v-if="item.image_file_name" class="support-chat__file-name">{{ item.image_file_name }}</div>
          </template>
          <div v-else class="support-chat__content">{{ item.content }}</div>
        </div>
      </div>
    </main>

    <div class="support-input-bar">
      <input ref="fileInput" type="file" accept="image/jpeg,image/png,image/webp,image/gif" class="file-input" @change="handleFileChange" />
      <button class="support-input-bar__attach" type="button" :disabled="uploading" aria-label="发送图片" @click="triggerImagePick">
        <LucideIcon name="add_circle" />
      </button>
      <input
        v-model="draft"
        class="support-input-bar__field"
        maxlength="2000"
        placeholder="输入消息..."
        type="text"
        @keyup.enter="sendMessage"
      />
      <button class="support-input-bar__send" type="button" :disabled="!canSend" aria-label="发送" @click="sendMessage">
        <LucideIcon name="send" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.support-chat {
  min-height: 100vh;
  background:
    radial-gradient(circle at 18% 0%, rgba(175, 40, 41, 0.11), transparent 30%),
    linear-gradient(180deg, #fff8f5 0%, #f7eee9 48%, #f4ebe6 100%);
  color: #241f1d;
  padding-bottom: 92px;
}

.support-chat__topbar {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  justify-content: space-between;
  min-height: 64px;
  padding: calc(10px + env(safe-area-inset-top)) 16px 10px;
  background: rgba(255, 250, 247, 0.9);
  border-bottom: 1px solid rgba(175, 40, 41, 0.09);
  box-shadow: 0 8px 30px rgba(95, 10, 18, 0.08);
  backdrop-filter: blur(18px);
}

.support-chat__icon-button,
.support-chat__agent-icon,
.support-input-bar__attach,
.support-input-bar__send {
  border: 0;
  appearance: none;
  display: flex;
  align-items: center;
  justify-content: center;
}

.support-chat__icon-button,
.support-chat__agent-icon {
  width: 40px;
  height: 40px;
  border-radius: 999px;
  color: #af2829;
  background: #f5e6e2;
}

.support-chat__presence {
  width: 10px;
  height: 10px;
  border-radius: 999px;
  box-shadow: 0 0 0 4px rgba(4, 120, 87, 0.12);
}

.support-chat__presence.is-online {
  background: #047857;
}

.support-chat__presence.is-offline {
  background: #c2410c;
  box-shadow: 0 0 0 4px rgba(194, 65, 12, 0.12);
}

.support-chat__icon-button,
.support-input-bar__attach,
.support-input-bar__send {
  transition: transform 0.18s ease, opacity 0.18s ease, box-shadow 0.18s ease;
}

.support-chat__icon-button:focus-visible,
.support-input-bar__attach:focus-visible,
.support-input-bar__send:focus-visible,
.support-input-bar__field:focus-visible {
  outline: 2px solid rgba(175, 40, 41, 0.28);
  outline-offset: 2px;
}

.support-chat__icon-button:active,
.support-input-bar__attach:active,
.support-input-bar__send:active {
  transform: scale(0.96);
}

.support-chat__title-group {
  min-width: 0;
  text-align: center;
}

.support-chat__title-group h1 {
  margin: 0;
  font-size: 20px;
  font-weight: 900;
  line-height: 1.1;
  color: #af2829;
  letter-spacing: -0.03em;
}

.support-chat__title-group p {
  margin: 4px 0 0;
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 11px;
  font-weight: 700;
}

.support-chat__title-group .is-online {
  color: #047857;
}

.support-chat__title-group .is-offline {
  color: #c2410c;
}

.support-chat__messages {
  width: 100%;
  max-width: 540px;
  min-height: 100vh;
  margin: 0 auto;
  padding: calc(86px + env(safe-area-inset-top)) 16px 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.support-chat__messages.is-empty {
  justify-content: flex-start;
}

.support-chat__date-pill {
  align-self: center;
  border-radius: 999px;
  padding: 5px 12px;
  color: #6d5b57;
  background: rgba(255, 250, 247, 0.82);
  box-shadow: 0 4px 14px rgba(95, 10, 18, 0.06);
  font-size: 12px;
  font-weight: 700;
}

.support-chat__loading,
.support-chat__empty {
  display: flex;
  min-height: 52vh;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: #8e706d;
  text-align: center;
}

.support-chat__empty-icon {
  display: inline-flex;
  width: 56px;
  height: 56px;
  align-items: center;
  justify-content: center;
  border-radius: 20px;
  color: #af2829;
  background: #ffdad7;
  box-shadow: 0 12px 26px rgba(95, 10, 18, 0.08);
}

.support-chat__empty h2 {
  margin: 16px 0 6px;
  color: #241f1d;
  font-size: 20px;
  font-weight: 900;
}

.support-chat__empty p {
  margin: 0;
  max-width: 240px;
  font-size: 13px;
  font-weight: 700;
  line-height: 1.6;
}

.support-chat__row {
  display: flex;
  align-items: flex-end;
  gap: 10px;
  max-width: 86%;
}

.support-chat__row--agent {
  align-self: flex-start;
}

.support-chat__row--user {
  align-self: flex-end;
  justify-content: flex-end;
}

.support-chat__avatar {
  flex: 0 0 auto;
  display: flex;
  width: 34px;
  height: 34px;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  color: #af2829;
  background: #f1ded9;
}

.support-chat__avatar svg {
  width: 18px;
  height: 18px;
}

.support-chat__bubble {
  min-width: 0;
  border-radius: 22px;
  padding: 12px 14px;
  font-size: 14px;
  line-height: 1.55;
  box-shadow: 0 10px 24px rgba(95, 10, 18, 0.08);
}

.support-chat__bubble--agent {
  border-bottom-left-radius: 6px;
  border: 1px solid #eadbd5;
  background: rgba(255, 250, 247, 0.96);
  color: #2f2927;
}

.support-chat__bubble--user {
  border-bottom-right-radius: 6px;
  background: #af2829;
  color: #fff;
  box-shadow: 0 12px 26px rgba(175, 40, 41, 0.22);
}

.support-chat__meta {
  display: flex;
  gap: 10px;
  justify-content: space-between;
  margin-bottom: 5px;
  font-size: 11px;
  opacity: 0.72;
}

.support-chat__content {
  white-space: pre-wrap;
  word-break: break-word;
}

.support-chat__image {
  display: block;
  max-width: min(240px, 60vw);
  max-height: 260px;
  border-radius: 16px;
  object-fit: cover;
  cursor: pointer;
  background: rgba(255, 255, 255, 0.16);
}

.support-chat__image:focus-visible {
  outline: 2px solid rgba(175, 40, 41, 0.28);
  outline-offset: 2px;
}

.support-chat__file-name {
  margin-top: 6px;
  font-size: 12px;
  opacity: 0.78;
  word-break: break-all;
}

.support-input-bar {
  position: fixed;
  left: 0;
  right: 0;
  bottom: 0;
  z-index: 50;
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  max-width: 540px;
  margin: 0 auto;
  padding: 12px 16px calc(12px + env(safe-area-inset-bottom));
  background: rgba(255, 250, 247, 0.94);
  border-top: 1px solid rgba(175, 40, 41, 0.09);
  box-shadow: 0 -10px 28px rgba(95, 10, 18, 0.08);
  backdrop-filter: blur(18px);
}

.support-input-bar__attach {
  width: 40px;
  height: 40px;
  flex: 0 0 auto;
  border-radius: 999px;
  color: #af2829;
  background: #f5e6e2;
}

.support-input-bar__attach:disabled,
.support-input-bar__send:disabled {
  opacity: 0.5;
}

.support-input-bar__field {
  min-width: 0;
  flex: 1;
  border: 0;
  outline: none;
  border-radius: 999px;
  padding: 13px 18px;
  background: #f3e8e3;
  color: #241f1d;
  font-size: 14px;
  box-shadow: inset 0 1px 2px rgba(95, 10, 18, 0.05);
}

.support-input-bar__field::placeholder {
  color: rgba(87, 73, 68, 0.55);
}

.support-input-bar__field:focus {
  background: #fff;
  box-shadow: 0 0 0 2px rgba(175, 40, 41, 0.14);
}

.support-input-bar__send {
  width: 42px;
  height: 42px;
  flex: 0 0 auto;
  border-radius: 999px;
  color: #fff;
  background: #af2829;
  box-shadow: 0 8px 18px rgba(175, 40, 41, 0.24);
}

.file-input {
  display: none;
}
</style>
