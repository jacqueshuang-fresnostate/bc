<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { showToast } from 'vant'
import { useRoute, useRouter } from 'vue-router'
import {
  errorMessage,
  fetchSupportConversation,
  fetchSupportConversations,
  replySupportConversation,
  type SupportConversation,
  type SupportMessage,
} from '../api/user'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import type { MobileRealtimeEvent } from '../types/realtime'
import { formatDateTime } from '../utils/lotteryFormat'

const props = defineProps<{ wsMessage?: MobileRealtimeEvent | null }>()
const router = useRouter()
const route = useRoute()
const draft = ref('')
const loading = ref(false)
const sending = ref(false)
const emojiPickerVisible = ref(false)
const emojiPickerLoading = ref(false)
const emojiPickerError = ref('')
const conversations = ref<SupportConversation[]>([])
const activeConversationId = ref('')
const currentConversation = ref<SupportConversation | null>(null)
const messageInputRef = ref<HTMLInputElement | null>(null)
const emojiPickerHostRef = ref<HTMLElement | null>(null)
let emojiPickerElement: HTMLElement | null = null

const messages = computed(() => currentConversation.value?.messages || [])
const adminOnline = computed(() => Boolean(currentConversation.value?.assignedAdminName))
const canSend = computed(() => Boolean(currentConversation.value) && draft.value.trim().length > 0 && !sending.value)
const supportStatusText = computed(() => {
  if (!currentConversation.value) return '请先从充值页发起客服直充'
  return adminOnline.value ? '客服已接入' : '客服会在这里继续回复'
})
const hasMessages = computed(() => messages.value.length > 0)
const routeConversationId = computed(() => (
  typeof route.query.conversationId === 'string' ? route.query.conversationId : ''
))

type EmojiPickerConstructor = typeof import('emoji-mart').Picker

interface EmojiSelection {
  native?: unknown
  skins?: unknown
}

function formatTime(value: string) {
  return formatDateTime(value)
}

function messageAuthorText(item: SupportMessage) {
  if (item.author === 'user') return '我'
  if (item.author === 'admin') return '客服'
  return '系统'
}

function conversationStatusText(conversation: SupportConversation) {
  const labels: Record<SupportConversation['status'], string> = {
    open: '处理中',
    pending: '待处理',
    resolved: '已解决',
    closed: '已关闭',
  }
  return labels[conversation.status] || conversation.status
}

function sortedConversations(items: SupportConversation[]) {
  return [...items].sort((a, b) => String(b.updatedAt || '').localeCompare(String(a.updatedAt || '')))
}

async function loadSupportData(preferredConversationId = activeConversationId.value || routeConversationId.value) {
  loading.value = true
  try {
    conversations.value = sortedConversations(await fetchSupportConversations())
    const nextId = preferredConversationId || conversations.value[0]?.id || ''
    activeConversationId.value = nextId
    currentConversation.value = nextId ? await fetchSupportConversation(nextId) : null
  } catch (error) {
    showToast(errorMessage(error, '加载客服会话失败'))
  } finally {
    loading.value = false
  }
}

async function sendMessage() {
  if (!currentConversation.value) {
    showToast('请先从充值页发起客服直充')
    return
  }
  const content = draft.value.trim()
  if (!content) return
  sending.value = true
  try {
    const updatedConversation = await replySupportConversation(currentConversation.value.id, content)
    currentConversation.value = updatedConversation
    draft.value = ''
    emojiPickerVisible.value = false
    conversations.value = conversations.value.map(conversation => (
      conversation.id === updatedConversation.id ? updatedConversation : conversation
    ))
  } catch (error) {
    showToast(errorMessage(error, '发送失败'))
  } finally {
    sending.value = false
  }
}

async function toggleEmojiPicker() {
  if (!currentConversation.value) {
    showToast('请先从充值页发起客服直充')
    return
  }
  emojiPickerVisible.value = !emojiPickerVisible.value
  if (emojiPickerVisible.value) {
    await mountEmojiPicker()
  }
}

async function mountEmojiPicker() {
  await nextTick()
  if (!emojiPickerHostRef.value) return
  if (!emojiPickerElement) {
    emojiPickerLoading.value = true
    emojiPickerError.value = ''
    try {
      const [{ Picker }, dataModule, i18nModule] = await Promise.all([
        import('emoji-mart'),
        import('@emoji-mart/data'),
        import('@emoji-mart/data/i18n/zh.json'),
      ])
      emojiPickerElement = createEmojiPicker(
        Picker,
        dataModule.default,
        i18nModule.default,
      )
    } catch {
      emojiPickerError.value = '表情面板加载失败，请稍后重试'
    } finally {
      emojiPickerLoading.value = false
    }
  }

  if (
    emojiPickerElement
    && emojiPickerHostRef.value
    && emojiPickerElement.parentElement !== emojiPickerHostRef.value
  ) {
    emojiPickerHostRef.value.appendChild(emojiPickerElement)
  }
}

function createEmojiPicker(
  Picker: EmojiPickerConstructor,
  data: unknown,
  i18n: unknown,
) {
  const picker = new Picker({
    data,
    dynamicWidth: true,
    emojiButtonRadius: '10px',
    emojiButtonSize: 30,
    emojiSize: 20,
    i18n,
    locale: 'zh',
    maxFrequentRows: 1,
    navPosition: 'bottom',
    onEmojiSelect: insertEmoji,
    previewPosition: 'none',
    searchPosition: 'top',
    set: 'native',
    skinTonePosition: 'none',
    theme: 'light',
  }) as unknown as HTMLElement
  picker.style.width = '100%'
  picker.style.height = '300px'
  picker.style.minHeight = '240px'
  return picker
}

function insertEmoji(selection: unknown) {
  const emoji = nativeEmojiFromSelection(selection)
  if (!emoji) return

  const input = messageInputRef.value
  const selectionStart = input?.selectionStart ?? draft.value.length
  const selectionEnd = input?.selectionEnd ?? selectionStart
  draft.value = `${draft.value.slice(0, selectionStart)}${emoji}${draft.value.slice(selectionEnd)}`
  const nextCursor = selectionStart + emoji.length
  emojiPickerVisible.value = false

  void nextTick(() => {
    messageInputRef.value?.focus()
    messageInputRef.value?.setSelectionRange(nextCursor, nextCursor)
  })
}

function nativeEmojiFromSelection(selection: unknown) {
  if (!isRecord(selection)) return ''
  const emoji = selection as EmojiSelection
  if (typeof emoji.native === 'string') {
    return emoji.native
  }
  if (Array.isArray(emoji.skins)) {
    for (const skin of emoji.skins) {
      if (isRecord(skin) && typeof skin.native === 'string') {
        return skin.native
      }
    }
  }
  return ''
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

async function selectConversation(id: string) {
  try {
    activeConversationId.value = id
    currentConversation.value = await fetchSupportConversation(id)
    emojiPickerVisible.value = false
  } catch (error) {
    showToast(errorMessage(error, '加载客服会话失败'))
  }
}

watch(routeConversationId, (conversationId) => {
  if (conversationId && conversationId !== activeConversationId.value) {
    void loadSupportData(conversationId)
  }
})

watch(() => props.wsMessage, (message) => {
  if (
    (message?.event !== 'support_message_created'
      && message?.event !== 'support_conversation_updated')
    || !message.conversationId
  ) return
  const preferredConversationId = (
    !activeConversationId.value || activeConversationId.value === message.conversationId
      ? message.conversationId
      : activeConversationId.value
  )
  void loadSupportData(preferredConversationId)
})

onMounted(() => {
  void loadSupportData(routeConversationId.value)
})

onBeforeUnmount(() => {
  emojiPickerElement?.remove()
  emojiPickerElement = null
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
      <div v-if="conversations.length > 1" class="support-chat__conversation-tabs">
        <button
          v-for="conversation in conversations"
          :key="conversation.id"
          type="button"
          :class="{ 'is-active': activeConversationId === conversation.id }"
          @click="selectConversation(conversation.id)"
        >
          <span>{{ conversation.subject }}</span>
          <small>{{ conversationStatusText(conversation) }}</small>
        </button>
      </div>

      <div class="support-chat__date-pill">
        {{ messages[0]?.createdAt ? formatTime(messages[0].createdAt) : '今天' }}
      </div>

      <div v-if="loading" class="support-chat__loading">正在同步客服消息...</div>
      <div v-else-if="messages.length === 0" class="support-chat__empty">
        <div class="support-chat__empty-icon"><LucideIcon name="support_agent" /></div>
        <h2>{{ currentConversation ? '还没有更多消息' : '还没有客服会话' }}</h2>
        <p>{{ currentConversation ? '发送消息后，客服会在这里继续回复。' : '请先在充值页发起客服直充。' }}</p>
      </div>

      <div
        v-for="item in messages"
        :key="item.id"
        :class="[
          'support-chat__row',
          item.author === 'user' ? 'support-chat__row--user' : 'support-chat__row--agent',
        ]"
      >
        <div v-if="item.author !== 'user'" class="support-chat__avatar">
          <LucideIcon name="support_agent" />
        </div>

        <div
          :class="[
            'support-chat__bubble',
            item.author === 'user'
              ? 'support-chat__bubble--user'
              : 'support-chat__bubble--agent',
          ]"
        >
          <div class="support-chat__meta">
            <span>{{ messageAuthorText(item) }}</span>
            <time>{{ formatTime(item.createdAt) }}</time>
          </div>
          <div class="support-chat__content">{{ item.content }}</div>
        </div>
      </div>
    </main>

    <Teleport to="body">
      <div
        v-show="emojiPickerVisible"
        class="support-emoji-panel"
        @click.self="emojiPickerVisible = false"
      >
        <div class="support-emoji-panel__shell">
          <div v-if="emojiPickerLoading || emojiPickerError" class="support-emoji-panel__state">
            {{ emojiPickerLoading ? '正在加载表情面板...' : emojiPickerError }}
          </div>
          <div
            ref="emojiPickerHostRef"
            v-show="!emojiPickerLoading && !emojiPickerError"
            class="support-emoji-panel__host"
          ></div>
        </div>
      </div>
    </Teleport>

    <div class="support-input-bar">
      <button
        class="support-input-bar__emoji"
        type="button"
        :disabled="!currentConversation || sending"
        :aria-pressed="emojiPickerVisible"
        aria-label="选择表情"
        @click="toggleEmojiPicker"
      >
        <LucideIcon name="mood" />
      </button>
      <input
        ref="messageInputRef"
        v-model="draft"
        class="support-input-bar__field"
        maxlength="2000"
        :placeholder="currentConversation ? '输入消息...' : '请先发起客服直充'"
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
  min-height: calc(64px + var(--mobile-status-safe-top));
  padding: calc(10px + var(--mobile-status-safe-top)) 16px 10px;
  background: rgba(255, 250, 247, 0.9);
  border-bottom: 1px solid rgba(175, 40, 41, 0.09);
  box-shadow: 0 8px 30px rgba(95, 10, 18, 0.08);
  backdrop-filter: blur(18px);
}

.support-chat__icon-button,
.support-chat__agent-icon,
.support-input-bar__attach,
.support-input-bar__emoji,
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
.support-input-bar__emoji,
.support-input-bar__send {
  transition: transform 0.18s ease, opacity 0.18s ease, box-shadow 0.18s ease;
}

.support-chat__icon-button:focus-visible,
.support-input-bar__attach:focus-visible,
.support-input-bar__emoji:focus-visible,
.support-input-bar__send:focus-visible,
.support-input-bar__field:focus-visible {
  outline: 2px solid rgba(175, 40, 41, 0.28);
  outline-offset: 2px;
}

.support-chat__icon-button:active,
.support-input-bar__attach:active,
.support-input-bar__emoji:active,
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
  padding: calc(86px + var(--mobile-status-safe-top)) 16px 24px;
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

.support-chat__conversation-tabs {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  padding-bottom: 2px;
}

.support-chat__conversation-tabs button {
  flex: 0 0 auto;
  max-width: 180px;
  border: 1px solid rgba(175, 40, 41, 0.12);
  border-radius: 14px;
  background: rgba(255, 250, 247, 0.88);
  color: #6d5b57;
  padding: 8px 10px;
  text-align: left;
}

.support-chat__conversation-tabs button.is-active {
  border-color: #af2829;
  background: #af2829;
  color: #fff;
}

.support-chat__conversation-tabs span,
.support-chat__conversation-tabs small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.support-chat__conversation-tabs span {
  font-size: 12px;
  font-weight: 900;
}

.support-chat__conversation-tabs small {
  margin-top: 2px;
  font-size: 10px;
  font-weight: 700;
  opacity: 0.74;
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

.support-input-bar__emoji {
  width: 40px;
  height: 40px;
  flex: 0 0 auto;
  border-radius: 999px;
  color: #af2829;
  background: #f5e6e2;
}

.support-input-bar__emoji[aria-pressed='true'] {
  background: #af2829;
  color: #fff;
  box-shadow: 0 8px 18px rgba(175, 40, 41, 0.2);
}

.support-input-bar__attach:disabled,
.support-input-bar__emoji:disabled,
.support-input-bar__send:disabled {
  opacity: 0.5;
}

.support-emoji-panel {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  padding: 0 12px calc(74px + env(safe-area-inset-bottom));
  pointer-events: auto;
}

.support-emoji-panel__shell {
  width: min(300px, calc(100vw - 32px));
  height: min(300px, 46dvh);
  min-height: 240px;
  overflow: hidden;
  border-radius: 16px;
  background: #fff;
  box-shadow: 0 18px 50px rgba(95, 10, 18, 0.18);
}

.support-emoji-panel__host {
  height: 100%;
  min-height: 240px;
}

.support-emoji-panel__state {
  display: grid;
  min-height: 240px;
  place-items: center;
  padding: 18px;
  color: #8e706d;
  font-size: 13px;
  font-weight: 700;
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
