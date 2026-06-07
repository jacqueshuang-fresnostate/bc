<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { showToast } from 'vant'
import { useRouter } from 'vue-router'
import {
  claimChatHallRedPacket,
  errorMessage,
  fetchChatHallMessages,
  sendChatHallMessage,
  sendChatHallRedPacket,
  shareChatHallGroupBuyPlan,
  type ChatHallGroupBuyPlanPayload,
  type ChatHallMessage,
  type ChatHallRedPacketPayload,
} from '../api/user'
import LucideIcon from '../components/mobile/LucideIcon.vue'
import { fetchMyGroupBuys } from '../features/group-buy/api'
import type { GroupBuyPlan } from '../features/group-buy/types'
import { useAuthStore } from '../stores/auth'
import type { MobileRealtimeEvent } from '../types/realtime'
import { formatDateTime } from '../utils/lotteryFormat'

const props = defineProps<{ wsMessage?: MobileRealtimeEvent | null }>()
const router = useRouter()
const auth = useAuthStore()
const draft = ref('')
const loading = ref(false)
const sending = ref(false)
const attachmentVisible = ref(false)
const emojiPickerVisible = ref(false)
const emojiPickerLoading = ref(false)
const emojiPickerError = ref('')
const redPacketDialogVisible = ref(false)
const redPacketAmount = ref('1.00')
const redPacketCount = ref('1')
const redPacketGreeting = ref('恭喜发财，大吉大利')
const sendingRedPacket = ref(false)
const claimingRedPacketId = ref('')
const groupBuyDialogVisible = ref(false)
const loadingGroupBuys = ref(false)
const sharingGroupBuyId = ref('')
const myGroupBuyPlans = ref<GroupBuyPlan[]>([])
const messages = ref<ChatHallMessage[]>([])
const messageListRef = ref<HTMLElement | null>(null)
const messageInputRef = ref<HTMLInputElement | null>(null)
const emojiPickerHostRef = ref<HTMLElement | null>(null)
let emojiPickerElement: HTMLElement | null = null
const currentUserId = computed(() => auth.user?.id || '')
const hasMessages = computed(() => messages.value.length > 0)
const canSend = computed(() => draft.value.trim().length > 0 && !sending.value)
const shareableGroupBuyPlans = computed(() => myGroupBuyPlans.value.filter(plan => plan.status !== 'cancelled' && plan.status !== 'settled'))

type EmojiPickerConstructor = typeof import('emoji-mart').Picker

interface EmojiSelection {
  native?: unknown
  skins?: unknown
}

type RecordPayload = Record<string, unknown>

function formatMessageTime(value: string) {
  return formatDateTime(value)
}

function isMine(message: ChatHallMessage) {
  return Boolean(currentUserId.value) && message.userId === currentUserId.value
}

function avatarText(username: string) {
  return String(username || '会员').trim().slice(0, 1) || '会'
}

function upsertMessage(message: ChatHallMessage) {
  const index = messages.value.findIndex(item => item.id === message.id)
  if (index >= 0) {
    messages.value = messages.value.map(item => (item.id === message.id ? message : item))
    return
  }
  messages.value = [...messages.value, message].slice(-100)
  void scrollToBottom()
}

function messageType(message: ChatHallMessage) {
  return message.messageType || 'text'
}

function redPacketPayload(message: ChatHallMessage): ChatHallRedPacketPayload | null {
  if (messageType(message) !== 'redPacket' || !isRecord(message.payload)) return null
  const payload = message.payload as RecordPayload
  const redPacketId = stringValue(payload.redPacketId)
  if (!redPacketId) return null
  return {
    redPacketId,
    greeting: stringValue(payload.greeting) || '恭喜发财，大吉大利',
    totalAmountMinor: numberValue(payload.totalAmountMinor),
    remainingAmountMinor: numberValue(payload.remainingAmountMinor),
    claimCount: numberValue(payload.claimCount),
    claimedCount: numberValue(payload.claimedCount),
  }
}

function groupBuyPayload(message: ChatHallMessage): ChatHallGroupBuyPlanPayload | null {
  if (messageType(message) !== 'groupBuyPlan' || !isRecord(message.payload)) return null
  const payload = message.payload as RecordPayload
  const planId = stringValue(payload.planId)
  if (!planId) return null
  return {
    planId,
    lotteryId: stringValue(payload.lotteryId),
    lotteryName: stringValue(payload.lotteryName),
    issue: stringValue(payload.issue),
    playName: stringValue(payload.playName),
    title: stringValue(payload.title),
    totalAmountMinor: numberValue(payload.totalAmountMinor),
    shareAmountMinor: numberValue(payload.shareAmountMinor),
    soldShares: numberValue(payload.soldShares),
    availableShares: numberValue(payload.availableShares),
    progressPercent: numberValue(payload.progressPercent),
    status: stringValue(payload.status),
  }
}

function stringValue(value: unknown) {
  return String(value ?? '').trim()
}

function numberValue(value: unknown) {
  const number = Number(value ?? 0)
  return Number.isFinite(number) ? number : 0
}

function formatMinor(value: unknown) {
  return (numberValue(value) / 100).toFixed(2)
}

function moneyToMinor(value: string) {
  const text = String(value ?? '').trim()
  if (!/^\d+(?:\.\d{0,2})?$/.test(text)) return 0
  const [whole, fraction = ''] = text.split('.')
  return Number(whole || 0) * 100 + Number(fraction.padEnd(2, '0').slice(0, 2) || 0)
}

function canClaimRedPacket(message: ChatHallMessage) {
  const payload = redPacketPayload(message)
  return Boolean(payload && !isMine(message) && payload.remainingAmountMinor > 0 && payload.claimedCount < payload.claimCount)
}

function redPacketProgressText(message: ChatHallMessage) {
  const payload = redPacketPayload(message)
  if (!payload) return '红包'
  if (payload.claimedCount >= payload.claimCount || payload.remainingAmountMinor <= 0) return '已抢完'
  return `${payload.claimedCount}/${payload.claimCount} 已领`
}

function groupBuyProgressStyle(message: ChatHallMessage) {
  const payload = groupBuyPayload(message)
  return { width: `${Math.min(100, Math.max(0, payload?.progressPercent || 0))}%` }
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
    emojiPickerVisible.value = false
    attachmentVisible.value = false
    upsertMessage(message)
    void nextTick(() => messageInputRef.value?.focus())
  } catch (error) {
    showToast(errorMessage(error, '发送失败'))
  } finally {
    sending.value = false
  }
}

async function toggleEmojiPicker() {
  emojiPickerVisible.value = !emojiPickerVisible.value
  if (emojiPickerVisible.value) attachmentVisible.value = false
  if (emojiPickerVisible.value) {
    await mountEmojiPicker()
  }
}

function toggleAttachmentMenu() {
  attachmentVisible.value = !attachmentVisible.value
  if (attachmentVisible.value) emojiPickerVisible.value = false
}

function openRedPacketDialog() {
  attachmentVisible.value = false
  redPacketDialogVisible.value = true
}

async function submitRedPacket() {
  if (sendingRedPacket.value) return
  const amountMinor = moneyToMinor(redPacketAmount.value)
  const claimCount = Math.trunc(Number(redPacketCount.value || 0))
  if (amountMinor <= 0) {
    showToast('请输入红包金额')
    return
  }
  if (claimCount <= 0) {
    showToast('请输入红包个数')
    return
  }
  if (amountMinor < claimCount) {
    showToast('红包金额不能小于红包个数')
    return
  }

  sendingRedPacket.value = true
  try {
    const message = await sendChatHallRedPacket({
      amountMinor,
      claimCount,
      greeting: redPacketGreeting.value,
    })
    upsertMessage(message)
    redPacketDialogVisible.value = false
    redPacketAmount.value = '1.00'
    redPacketCount.value = '1'
    redPacketGreeting.value = '恭喜发财，大吉大利'
    showToast('红包已发送')
  } catch (error) {
    showToast(errorMessage(error, '发送红包失败'))
  } finally {
    sendingRedPacket.value = false
  }
}

async function claimRedPacket(message: ChatHallMessage) {
  const payload = redPacketPayload(message)
  if (!payload || !canClaimRedPacket(message) || claimingRedPacketId.value) return
  claimingRedPacketId.value = payload.redPacketId
  try {
    const response = await claimChatHallRedPacket(payload.redPacketId)
    upsertMessage(response.message)
    showToast(`领取红包 ¥${formatMinor(response.claim.amountMinor)}`)
  } catch (error) {
    showToast(errorMessage(error, '领取红包失败'))
  } finally {
    claimingRedPacketId.value = ''
  }
}

async function openGroupBuyDialog() {
  attachmentVisible.value = false
  groupBuyDialogVisible.value = true
  if (!myGroupBuyPlans.value.length) {
    await loadMyGroupBuyPlans()
  }
}

async function loadMyGroupBuyPlans() {
  loadingGroupBuys.value = true
  try {
    const result = await fetchMyGroupBuys()
    myGroupBuyPlans.value = result.data.items || []
  } catch (error) {
    showToast(errorMessage(error, '加载合买计划失败'))
  } finally {
    loadingGroupBuys.value = false
  }
}

async function shareGroupBuy(plan: GroupBuyPlan) {
  if (sharingGroupBuyId.value) return
  sharingGroupBuyId.value = plan.id
  try {
    const message = await shareChatHallGroupBuyPlan(plan.id)
    upsertMessage(message)
    groupBuyDialogVisible.value = false
    showToast('合买计划已发送')
  } catch (error) {
    showToast(errorMessage(error, '发送合买计划失败'))
  } finally {
    sharingGroupBuyId.value = ''
  }
}

function openGroupBuyPlan(message: ChatHallMessage) {
  const payload = groupBuyPayload(message)
  if (!payload) return
  router.push({ path: '/group-buy', query: { tab: 'hall', plan_id: payload.planId } })
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
  return new Picker({
    data,
    i18n,
    locale: 'zh',
    navPosition: 'bottom',
    onEmojiSelect: insertEmoji,
    previewPosition: 'none',
    searchPosition: 'top',
    set: 'native',
    skinTonePosition: 'none',
    theme: 'light',
  }) as unknown as HTMLElement
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

async function scrollToBottom() {
  await nextTick()
  const list = messageListRef.value
  if (!list) return
  list.scrollTop = list.scrollHeight
}

watch(() => props.wsMessage, (message) => {
  if (message?.event !== 'chat_hall_message_created') return
  upsertMessage(message.message as ChatHallMessage)
})

onMounted(() => {
  void loadMessages()
})

onBeforeUnmount(() => {
  emojiPickerElement?.remove()
  emojiPickerElement = null
})
</script>

<template>
  <div class="chat-hall">
    <header class="chat-hall__topbar">
      <div class="chat-hall__title-group">
        <h1>聊天大厅</h1>
        <p>所有在线会员都可以在这里交流</p>
      </div>
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
            <button
              v-if="redPacketPayload(message)"
              class="chat-hall-red-packet"
              :class="{ 'chat-hall-red-packet--mine': isMine(message) }"
              :disabled="!canClaimRedPacket(message) || claimingRedPacketId === redPacketPayload(message)?.redPacketId"
              type="button"
              @click="claimRedPacket(message)"
            >
              <span class="chat-hall-red-packet__icon">¥</span>
              <span class="chat-hall-red-packet__body">
                <strong>{{ redPacketPayload(message)?.greeting }}</strong>
                <small>总额 ¥{{ formatMinor(redPacketPayload(message)?.totalAmountMinor) }} · {{ redPacketProgressText(message) }}</small>
              </span>
              <span class="chat-hall-red-packet__action">{{ canClaimRedPacket(message) ? '领取' : '查看' }}</span>
            </button>
            <button
              v-else-if="groupBuyPayload(message)"
              class="chat-hall-group-buy"
              :class="{ 'chat-hall-group-buy--mine': isMine(message) }"
              type="button"
              @click="openGroupBuyPlan(message)"
            >
              <span class="chat-hall-group-buy__badge">合买</span>
              <span class="chat-hall-group-buy__body">
                <strong>{{ groupBuyPayload(message)?.title }}</strong>
                <small>{{ groupBuyPayload(message)?.lotteryName }} · 第{{ groupBuyPayload(message)?.issue }}期 · {{ groupBuyPayload(message)?.playName }}</small>
                <span class="chat-hall-group-buy__progress">
                  <i :style="groupBuyProgressStyle(message)"></i>
                </span>
                <em>已满 {{ groupBuyPayload(message)?.progressPercent }}% · 剩 {{ groupBuyPayload(message)?.availableShares }} 份</em>
              </span>
            </button>
            <div v-else class="chat-hall__bubble">{{ message.content }}</div>
          </div>
        </div>
      </template>
    </main>

    <Teleport to="body">
      <div
        v-show="emojiPickerVisible"
        class="chat-hall-emoji-panel"
        @click.self="emojiPickerVisible = false"
      >
        <div class="chat-hall-emoji-panel__shell">
          <div v-if="emojiPickerLoading || emojiPickerError" class="chat-hall-emoji-panel__state">
            {{ emojiPickerLoading ? '正在加载表情面板...' : emojiPickerError }}
          </div>
          <div
            ref="emojiPickerHostRef"
            v-show="!emojiPickerLoading && !emojiPickerError"
            class="chat-hall-emoji-panel__host"
          ></div>
        </div>
      </div>
    </Teleport>

    <Teleport to="body">
      <div v-if="redPacketDialogVisible" class="chat-hall-modal" @click.self="redPacketDialogVisible = false">
        <section class="chat-hall-modal__sheet">
          <div class="chat-hall-modal__header">
            <h2>发送红包</h2>
            <button type="button" aria-label="关闭" @click="redPacketDialogVisible = false">
              <LucideIcon name="close" />
            </button>
          </div>
          <label class="chat-hall-form-field">
            <span>红包金额</span>
            <input v-model="redPacketAmount" inputmode="decimal" placeholder="例如 8.88" />
          </label>
          <label class="chat-hall-form-field">
            <span>红包个数</span>
            <input v-model="redPacketCount" inputmode="numeric" placeholder="例如 3" />
          </label>
          <label class="chat-hall-form-field">
            <span>祝福语</span>
            <input v-model="redPacketGreeting" maxlength="60" placeholder="恭喜发财，大吉大利" />
          </label>
          <button class="chat-hall-modal__primary" :disabled="sendingRedPacket" type="button" @click="submitRedPacket">
            {{ sendingRedPacket ? '发送中...' : '发送红包' }}
          </button>
        </section>
      </div>
    </Teleport>

    <Teleport to="body">
      <div v-if="groupBuyDialogVisible" class="chat-hall-modal" @click.self="groupBuyDialogVisible = false">
        <section class="chat-hall-modal__sheet chat-hall-modal__sheet--tall">
          <div class="chat-hall-modal__header">
            <h2>发送合买计划</h2>
            <button type="button" aria-label="关闭" @click="groupBuyDialogVisible = false">
              <LucideIcon name="close" />
            </button>
          </div>
          <div v-if="loadingGroupBuys" class="chat-hall-modal__state">正在加载合买计划...</div>
          <div v-else-if="!shareableGroupBuyPlans.length" class="chat-hall-modal__state">暂无可发送的合买计划</div>
          <div v-else class="chat-hall-plan-list">
            <button
              v-for="plan in shareableGroupBuyPlans"
              :key="plan.id"
              type="button"
              :disabled="Boolean(sharingGroupBuyId)"
              @click="shareGroupBuy(plan)"
            >
              <strong>{{ plan.title || `${plan.lottery_name} 第${plan.issue}期` }}</strong>
              <span>{{ plan.lottery_name }} · {{ plan.play_name || plan.play_code }} · 已满 {{ plan.progress_percent }}%</span>
              <em>{{ sharingGroupBuyId === plan.id ? '发送中...' : '发送' }}</em>
            </button>
          </div>
        </section>
      </div>
    </Teleport>

    <footer class="chat-hall__composer">
      <div v-show="attachmentVisible" class="chat-hall__action-panel">
        <button type="button" @click="openRedPacketDialog">
          <LucideIcon name="payments" />
          <span>红包</span>
        </button>
        <button type="button" @click="openGroupBuyDialog">
          <LucideIcon name="group" />
          <span>合买计划</span>
        </button>
      </div>
      <div class="chat-hall__input-row">
        <button
          class="chat-hall__tool"
          type="button"
          :aria-pressed="attachmentVisible"
          aria-label="更多功能"
          @click="toggleAttachmentMenu"
        >
          <LucideIcon name="add" />
        </button>
        <button
          class="chat-hall__tool"
          type="button"
          :disabled="sending"
          :aria-pressed="emojiPickerVisible"
          aria-label="选择表情"
          @click="toggleEmojiPicker"
        >
          <LucideIcon name="mood" />
        </button>
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
      </div>
    </footer>
  </div>
</template>

<style scoped>
.chat-hall {
  --chat-hall-bottom-nav-space: calc(5.25rem + max(1rem, env(safe-area-inset-bottom)));
  --chat-hall-composer-height: 4.25rem;
  min-height: 100vh;
  background: linear-gradient(180deg, #fffafa 0%, #fbf3f1 46%, #f4f6f8 100%);
  color: #2b1f1f;
}

.chat-hall__topbar {
  position: fixed;
  top: 0;
  left: 0;
  z-index: 40;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 4.5rem;
  padding: max(0.75rem, env(safe-area-inset-top)) 1rem 0.75rem;
  background: rgba(255, 255, 255, 0.88);
  border-bottom: 1px solid rgba(143, 20, 31, 0.08);
  box-shadow: 0 10px 30px rgba(143, 20, 31, 0.08);
  backdrop-filter: blur(18px);
}

.chat-hall__title-group {
  min-width: 0;
  width: min(100%, 22rem);
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
  padding: calc(5rem + env(safe-area-inset-top)) 1rem calc(var(--chat-hall-bottom-nav-space) + var(--chat-hall-composer-height) + 1.25rem);
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
  max-width: min(78%, 25rem);
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

.chat-hall-red-packet,
.chat-hall-group-buy {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  width: min(100%, 19rem);
  border: 0;
  text-align: left;
  box-shadow: 0 12px 28px rgba(95, 10, 18, 0.13);
}

.chat-hall-red-packet {
  padding: 0.78rem;
  border-radius: 1.1rem 1.1rem 1.1rem 0.35rem;
  background: linear-gradient(135deg, #e2412f, #ab1020);
  color: #fff;
}

.chat-hall-red-packet--mine {
  border-radius: 1.1rem 1.1rem 0.35rem 1.1rem;
}

.chat-hall-red-packet:disabled {
  cursor: default;
}

.chat-hall-red-packet__icon {
  display: grid;
  flex: 0 0 auto;
  width: 2.35rem;
  height: 2.35rem;
  place-items: center;
  border-radius: 0.85rem;
  background: rgba(255, 226, 181, 0.24);
  color: #ffefb9;
  font-size: 1.05rem;
  font-weight: 900;
}

.chat-hall-red-packet__body {
  min-width: 0;
  flex: 1;
}

.chat-hall-red-packet__body strong,
.chat-hall-group-buy__body strong {
  display: block;
  overflow: hidden;
  font-size: 0.9rem;
  font-weight: 900;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-hall-red-packet__body small {
  display: block;
  margin-top: 0.22rem;
  color: rgba(255, 248, 224, 0.86);
  font-size: 0.68rem;
  font-weight: 700;
  white-space: nowrap;
}

.chat-hall-red-packet__action {
  flex: 0 0 auto;
  border-radius: 9999px;
  background: rgba(255, 255, 255, 0.18);
  padding: 0.32rem 0.55rem;
  font-size: 0.68rem;
  font-weight: 900;
}

.chat-hall-group-buy {
  padding: 0.78rem;
  border: 1px solid rgba(143, 20, 31, 0.1);
  border-radius: 1.1rem 1.1rem 1.1rem 0.35rem;
  background: rgba(255, 255, 255, 0.96);
  color: #2b1f1f;
}

.chat-hall-group-buy--mine {
  border-radius: 1.1rem 1.1rem 0.35rem 1.1rem;
}

.chat-hall-group-buy__badge {
  display: grid;
  flex: 0 0 auto;
  width: 2.45rem;
  height: 2.45rem;
  place-items: center;
  border-radius: 0.9rem;
  background: #fff1e8;
  color: #9f1724;
  font-size: 0.72rem;
  font-weight: 900;
}

.chat-hall-group-buy__body {
  min-width: 0;
  flex: 1;
}

.chat-hall-group-buy__body small,
.chat-hall-group-buy__body em {
  display: block;
  overflow: hidden;
  color: #8d6f6e;
  font-size: 0.68rem;
  font-style: normal;
  font-weight: 700;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-hall-group-buy__progress {
  display: block;
  height: 0.28rem;
  margin: 0.42rem 0 0.3rem;
  overflow: hidden;
  border-radius: 9999px;
  background: #f1e4e1;
}

.chat-hall-group-buy__progress i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #c01627, #ec8b49);
}

.chat-hall__composer {
  position: fixed;
  left: 0;
  bottom: var(--chat-hall-bottom-nav-space);
  z-index: 45;
  width: 100%;
  padding: 0 1rem;
  pointer-events: none;
}

.chat-hall__input-row {
  pointer-events: auto;
  display: grid;
  grid-template-columns: auto auto minmax(0, 1fr) auto;
  gap: 0.45rem;
  width: min(100%, 30rem);
  margin: 0 auto;
  padding: 0.55rem;
  border: 1px solid rgba(143, 20, 31, 0.1);
  border-radius: 1.7rem;
  background: rgba(255, 255, 255, 0.92);
  box-shadow: 0 14px 38px rgba(43, 31, 31, 0.12);
  backdrop-filter: blur(18px);
}

.chat-hall__action-panel {
  pointer-events: auto;
  display: flex;
  gap: 0.55rem;
  width: min(100%, 30rem);
  margin: 0 auto 0.55rem;
  padding: 0.6rem;
  border: 1px solid rgba(143, 20, 31, 0.1);
  border-radius: 1.4rem;
  background: rgba(255, 255, 255, 0.94);
  box-shadow: 0 18px 40px rgba(43, 31, 31, 0.12);
  backdrop-filter: blur(18px);
}

.chat-hall__action-panel button {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  gap: 0.35rem;
  min-height: 2.65rem;
  border: 0;
  border-radius: 1rem;
  background: #fff3f0;
  color: #9f1724;
  font-size: 0.78rem;
  font-weight: 900;
}

.chat-hall__action-panel svg {
  width: 1rem;
  height: 1rem;
}

.chat-hall__tool {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 2.65rem;
  height: 2.65rem;
  border: 0;
  border-radius: 1.05rem;
  background: #f4e7e4;
  color: #9f1724;
}

.chat-hall__tool[aria-pressed='true'] {
  background: #9f1724;
  color: #fff;
  box-shadow: 0 10px 22px rgba(159, 23, 36, 0.2);
}

.chat-hall__tool:disabled,
.chat-hall__send:disabled {
  opacity: 0.56;
}

.chat-hall__tool svg {
  width: 1.12rem;
  height: 1.12rem;
}

.chat-hall-emoji-panel {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  padding: 0 12px calc(var(--chat-hall-bottom-nav-space) + var(--chat-hall-composer-height) + 0.5rem);
  pointer-events: auto;
}

.chat-hall-emoji-panel__shell {
  width: min(352px, calc(100vw - 24px));
  min-height: 300px;
  overflow: hidden;
  border-radius: 18px;
  background: #fff;
  box-shadow: 0 18px 50px rgba(95, 10, 18, 0.18);
}

.chat-hall-emoji-panel__host {
  min-height: 300px;
}

.chat-hall-emoji-panel__state {
  display: grid;
  min-height: 300px;
  place-items: center;
  padding: 18px;
  color: #8d6f6e;
  font-size: 13px;
  font-weight: 700;
}

.chat-hall__input {
  min-width: 0;
  height: 2.65rem;
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
  height: 2.65rem;
  padding: 0 0.85rem;
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
  opacity: 1;
}

.chat-hall__send svg {
  width: 1rem;
  height: 1rem;
}

.chat-hall-modal {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  padding: 1rem 1rem calc(1rem + env(safe-area-inset-bottom));
  background: rgba(43, 31, 31, 0.22);
}

.chat-hall-modal__sheet {
  width: min(100%, 26rem);
  max-height: min(80vh, 34rem);
  overflow: auto;
  border-radius: 1.4rem;
  background: #fff;
  padding: 1rem;
  box-shadow: 0 20px 60px rgba(43, 31, 31, 0.2);
}

.chat-hall-modal__sheet--tall {
  min-height: 18rem;
}

.chat-hall-modal__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 0.8rem;
}

.chat-hall-modal__header h2 {
  margin: 0;
  color: #241819;
  font-size: 1rem;
  font-weight: 900;
}

.chat-hall-modal__header button {
  display: grid;
  width: 2.2rem;
  height: 2.2rem;
  place-items: center;
  border: 0;
  border-radius: 0.8rem;
  background: #f6eeee;
  color: #8f141f;
}

.chat-hall-modal__header svg {
  width: 1rem;
  height: 1rem;
}

.chat-hall-form-field {
  display: block;
  margin-top: 0.72rem;
}

.chat-hall-form-field span {
  display: block;
  margin-bottom: 0.35rem;
  color: #8d6f6e;
  font-size: 0.75rem;
  font-weight: 900;
}

.chat-hall-form-field input {
  width: 100%;
  height: 2.85rem;
  border: 1px solid rgba(143, 20, 31, 0.13);
  border-radius: 1rem;
  background: #fffafa;
  color: #2b1f1f;
  font-size: 0.92rem;
  font-weight: 800;
  outline: none;
  padding: 0 0.9rem;
}

.chat-hall-modal__primary {
  width: 100%;
  height: 2.9rem;
  margin-top: 1rem;
  border: 0;
  border-radius: 1rem;
  background: #9f1724;
  color: #fff;
  font-weight: 900;
  box-shadow: 0 12px 24px rgba(159, 23, 36, 0.22);
}

.chat-hall-modal__primary:disabled {
  background: #d7c9c8;
  box-shadow: none;
}

.chat-hall-modal__state {
  display: grid;
  min-height: 10rem;
  place-items: center;
  color: #8d6f6e;
  font-size: 0.82rem;
  font-weight: 800;
  text-align: center;
}

.chat-hall-plan-list {
  display: grid;
  gap: 0.6rem;
}

.chat-hall-plan-list button {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 0.2rem 0.8rem;
  width: 100%;
  border: 1px solid rgba(143, 20, 31, 0.1);
  border-radius: 1rem;
  background: #fffafa;
  padding: 0.78rem;
  text-align: left;
}

.chat-hall-plan-list strong,
.chat-hall-plan-list span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chat-hall-plan-list strong {
  color: #241819;
  font-size: 0.86rem;
  font-weight: 900;
}

.chat-hall-plan-list span {
  color: #8d6f6e;
  font-size: 0.7rem;
  font-weight: 700;
}

.chat-hall-plan-list em {
  grid-row: span 2;
  align-self: center;
  border-radius: 9999px;
  background: #9f1724;
  color: #fff;
  font-size: 0.72rem;
  font-style: normal;
  font-weight: 900;
  padding: 0.42rem 0.62rem;
}

</style>
