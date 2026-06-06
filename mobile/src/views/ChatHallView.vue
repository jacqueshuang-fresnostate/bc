<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from 'vue'
import { showToast } from 'vant'
import { useRouter } from 'vue-router'
import {
  errorMessage,
  fetchChatHallMessages,
  sendChatHallMessage,
  type ChatHallMessage,
} from '../api/user'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { useAuthStore } from '../stores/auth'
import type { MobileRealtimeEvent } from '../types/realtime'
import { formatDateTime } from '../utils/lotteryFormat'

const props = defineProps<{ wsMessage?: MobileRealtimeEvent | null }>()
const router = useRouter()
const auth = useAuthStore()
const draft = ref('')
const loading = ref(false)
const sending = ref(false)
const messages = ref<ChatHallMessage[]>([])
const messageListRef = ref<HTMLElement | null>(null)
const messageInputRef = ref<HTMLInputElement | null>(null)
const currentUserId = computed(() => auth.user?.id || '')
const hasMessages = computed(() => messages.value.length > 0)
const canSend = computed(() => draft.value.trim().length > 0 && !sending.value)

function formatMessageTime(value: string) {
  return formatDateTime(value)
}

function isMine(message: ChatHallMessage) {
  return Boolean(currentUserId.value) && message.userId === currentUserId.value
}

function avatarText(username: string) {
  return String(username || '会员').trim().slice(0, 1) || '会'
}

function appendMessage(message: ChatHallMessage) {
  if (messages.value.some(item => item.id === message.id)) return
  messages.value = [...messages.value, message].slice(-100)
  void scrollToBottom()
}

async function loadMessages() {
  loading.value = true
  try {
    messages.value = await fetchChatHallMessages()
    await scrollToBottom()
  } catch (error) {
    showToast(errorMessage(error, '加载聊天大厅失败'))
  } finally {
    loading.value = false
  }
}

async function sendMessage() {
  const content = draft.value.trim()
  if (!content || sending.value) return
  sending.value = true
  try {
    const message = await sendChatHallMessage(content)
    draft.value = ''
    appendMessage(message)
    void nextTick(() => messageInputRef.value?.focus())
  } catch (error) {
    showToast(errorMessage(error, '发送失败'))
  } finally {
    sending.value = false
  }
}

async function scrollToBottom() {
  await nextTick()
  const list = messageListRef.value
  if (!list) return
  list.scrollTop = list.scrollHeight
}

watch(() => props.wsMessage, (message) => {
  if (message?.event !== 'chat_hall_message_created') return
  appendMessage(message.message)
})

onMounted(() => {
  void loadMessages()
})
</script>

<template>
  <div class="chat-hall">
    <header class="chat-hall__topbar">
      <button class="chat-hall__icon-button" aria-label="返回" @click="router.back()">
        <LucideIcon name="arrow_back" />
      </button>
      <div class="chat-hall__title-group">
        <h1>聊天大厅</h1>
        <p>所有在线会员都可以在这里交流</p>
      </div>
      <button class="chat-hall__icon-button" aria-label="刷新" :disabled="loading" @click="loadMessages">
        <LucideIcon name="refresh" :class="{ 'chat-hall__spin': loading }" />
      </button>
    </header>

    <main ref="messageListRef" class="chat-hall__messages">
      <div v-if="loading && !hasMessages" class="chat-hall__state">正在加载聊天记录...</div>
      <div v-else-if="!hasMessages" class="chat-hall__state">
        <strong>还没有消息</strong>
        <span>发第一条消息，和大家打个招呼</span>
      </div>
      <template v-else>
        <div
          v-for="message in messages"
          :key="message.id"
          class="chat-hall__message-row"
          :class="{ 'chat-hall__message-row--mine': isMine(message) }"
        >
          <div class="chat-hall__avatar">{{ avatarText(message.username) }}</div>
          <div class="chat-hall__bubble-wrap">
            <div class="chat-hall__meta">
              <span>{{ isMine(message) ? '我' : message.username }}</span>
              <time>{{ formatMessageTime(message.createdAt) }}</time>
            </div>
            <div class="chat-hall__bubble">{{ message.content }}</div>
          </div>
        </div>
      </template>
    </main>

    <footer class="chat-hall__input-bar">
      <input
        ref="messageInputRef"
        v-model="draft"
        class="chat-hall__input"
        maxlength="500"
        placeholder="输入聊天内容"
        type="text"
        @keyup.enter="sendMessage"
      />
      <button class="chat-hall__send" :disabled="!canSend" type="button" @click="sendMessage">
        <LucideIcon name="send" />
        <span>发送</span>
      </button>
    </footer>
  </div>
</template>

<style scoped>
.chat-hall {
  min-height: 100vh;
  background: linear-gradient(180deg, #fff8f6 0%, #f5f1ed 56%, #eef2f7 100%);
  color: #2b1f1f;
}

.chat-hall__topbar {
  position: fixed;
  top: 0;
  left: 0;
  z-index: 40;
  display: grid;
  grid-template-columns: 2.5rem minmax(0, 1fr) 2.5rem;
  align-items: center;
  width: 100%;
  height: 4.5rem;
  padding: max(0.75rem, env(safe-area-inset-top)) 1rem 0.75rem;
  background: rgba(255, 255, 255, 0.88);
  border-bottom: 1px solid rgba(143, 20, 31, 0.08);
  box-shadow: 0 10px 30px rgba(143, 20, 31, 0.08);
  backdrop-filter: blur(18px);
}

.chat-hall__icon-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 2.5rem;
  height: 2.5rem;
  border: 0;
  border-radius: 1rem;
  background: #fff;
  color: #8f141f;
  box-shadow: 0 8px 20px rgba(143, 20, 31, 0.1);
}

.chat-hall__icon-button:disabled {
  opacity: 0.56;
}

.chat-hall__icon-button svg {
  width: 1.25rem;
  height: 1.25rem;
}

.chat-hall__title-group {
  min-width: 0;
  text-align: center;
}

.chat-hall__title-group h1 {
  margin: 0;
  font-size: 1.08rem;
  font-weight: 900;
  line-height: 1.2;
  color: #241819;
}

.chat-hall__title-group p {
  margin: 0.25rem 0 0;
  overflow: hidden;
  font-size: 0.72rem;
  color: #8d6f6e;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-hall__messages {
  height: 100vh;
  overflow-y: auto;
  padding: calc(5rem + env(safe-area-inset-top)) 1rem calc(6.25rem + env(safe-area-inset-bottom));
}

.chat-hall__state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 55vh;
  gap: 0.35rem;
  color: #8d6f6e;
  font-size: 0.85rem;
  text-align: center;
}

.chat-hall__state strong {
  color: #2b1f1f;
  font-size: 1rem;
}

.chat-hall__message-row {
  display: flex;
  align-items: flex-end;
  gap: 0.6rem;
  margin-bottom: 0.9rem;
}

.chat-hall__message-row--mine {
  flex-direction: row-reverse;
}

.chat-hall__avatar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 2.1rem;
  height: 2.1rem;
  border-radius: 9999px;
  background: #fff;
  color: #9f1724;
  font-size: 0.8rem;
  font-weight: 900;
  box-shadow: 0 8px 18px rgba(43, 31, 31, 0.08);
}

.chat-hall__message-row--mine .chat-hall__avatar {
  background: #9f1724;
  color: #fff;
}

.chat-hall__bubble-wrap {
  max-width: min(76%, 24rem);
  min-width: 0;
}

.chat-hall__message-row--mine .chat-hall__bubble-wrap {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.chat-hall__meta {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  max-width: 100%;
  margin: 0 0 0.25rem;
  color: #9a8582;
  font-size: 0.68rem;
  line-height: 1;
}

.chat-hall__meta span,
.chat-hall__meta time {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-hall__bubble {
  max-width: 100%;
  padding: 0.72rem 0.85rem;
  border-radius: 1rem 1rem 1rem 0.35rem;
  background: rgba(255, 255, 255, 0.96);
  color: #2b1f1f;
  font-size: 0.9rem;
  line-height: 1.55;
  overflow-wrap: anywhere;
  box-shadow: 0 8px 24px rgba(43, 31, 31, 0.08);
}

.chat-hall__message-row--mine .chat-hall__bubble {
  border-radius: 1rem 1rem 0.35rem 1rem;
  background: linear-gradient(135deg, #b01626, #8f141f);
  color: #fff;
}

.chat-hall__input-bar {
  position: fixed;
  left: 0;
  bottom: 0;
  z-index: 45;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.65rem;
  width: 100%;
  padding: 0.75rem 1rem max(0.9rem, env(safe-area-inset-bottom));
  background: rgba(255, 255, 255, 0.9);
  border-top: 1px solid rgba(143, 20, 31, 0.08);
  box-shadow: 0 -10px 28px rgba(43, 31, 31, 0.08);
  backdrop-filter: blur(18px);
}

.chat-hall__input {
  min-width: 0;
  height: 2.85rem;
  border: 1px solid rgba(143, 20, 31, 0.12);
  border-radius: 1rem;
  background: #fff;
  color: #2b1f1f;
  font-size: 0.9rem;
  outline: none;
  padding: 0 0.9rem;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.9);
}

.chat-hall__input:focus {
  border-color: rgba(176, 22, 38, 0.42);
  box-shadow: 0 0 0 0.2rem rgba(176, 22, 38, 0.08);
}

.chat-hall__send {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.35rem;
  height: 2.85rem;
  padding: 0 1rem;
  border: 0;
  border-radius: 1rem;
  background: #9f1724;
  color: #fff;
  font-size: 0.85rem;
  font-weight: 900;
  white-space: nowrap;
  box-shadow: 0 10px 22px rgba(159, 23, 36, 0.22);
}

.chat-hall__send:disabled {
  background: #d7c9c8;
  box-shadow: none;
}

.chat-hall__send svg {
  width: 1rem;
  height: 1rem;
}

.chat-hall__spin {
  animation: chat-hall-spin 0.8s linear infinite;
}

@keyframes chat-hall-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
