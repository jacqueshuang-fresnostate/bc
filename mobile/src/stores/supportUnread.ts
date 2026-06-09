import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import {
  fetchSupportConversations,
  isVisibleSupportConversation,
  markSupportConversationRead,
  type SupportConversation,
} from '../api/user'
import { useAuthStore } from './auth'

const SUPPORT_CONVERSATION_CACHE_MS = 15_000

type LoadOptions = {
  force?: boolean
  silent?: boolean
}

type LoadResult = {
  data: SupportConversation[]
  refreshed: boolean
}

function hasFreshCache(fetchedAt: number) {
  return fetchedAt > 0 && Date.now() - fetchedAt < SUPPORT_CONVERSATION_CACHE_MS
}

function sortedConversations(items: SupportConversation[]) {
  return [...items]
    .filter(isVisibleSupportConversation)
    .sort((a, b) => String(b.updatedAt || '').localeCompare(String(a.updatedAt || '')))
}

// 手机端在线客服未读缓存：用户进入个人中心或收到客服实时消息时刷新，用于红点提醒。
export const useSupportUnreadStore = defineStore('supportUnread', () => {
  const conversations = ref<SupportConversation[]>([])
  const loading = ref(false)
  const fetchedAt = ref(0)
  const userScopeId = ref('')
  let conversationsRequest: Promise<LoadResult> | null = null

  const unreadTotal = computed(() => (
    conversations.value.reduce((total, conversation) => (
      total + Math.max(0, Number(conversation.userUnreadCount || 0))
    ), 0)
  ))
  const hasUnread = computed(() => unreadTotal.value > 0)

  function currentUserId() {
    return useAuthStore().user?.id || ''
  }

  function clear() {
    conversations.value = []
    fetchedAt.value = 0
    userScopeId.value = ''
    conversationsRequest = null
  }

  function syncUserScope() {
    const nextUserId = currentUserId()
    if (userScopeId.value !== nextUserId) {
      clear()
    }
    if (nextUserId) {
      userScopeId.value = nextUserId
    }
    return nextUserId
  }

  function setConversations(items: SupportConversation[]) {
    conversations.value = sortedConversations(items)
    fetchedAt.value = Date.now()
  }

  function upsertConversation(conversation: SupportConversation) {
    const next = conversations.value.filter(item => item.id !== conversation.id)
    if (isVisibleSupportConversation(conversation)) {
      next.push(conversation)
    }
    setConversations(next)
  }

  async function loadConversations(options: LoadOptions = {}): Promise<LoadResult> {
    const userId = syncUserScope()
    if (!userId || !useAuthStore().accessToken) {
      clear()
      return { data: [], refreshed: false }
    }
    if (!options.force && hasFreshCache(fetchedAt.value)) {
      return { data: conversations.value, refreshed: false }
    }
    if (conversationsRequest) return conversationsRequest

    if (!options.silent && !fetchedAt.value) loading.value = true
    conversationsRequest = (async () => {
      try {
        const data = sortedConversations(await fetchSupportConversations())
        conversations.value = data
        fetchedAt.value = Date.now()
        return { data, refreshed: true }
      } finally {
        if (!options.silent) loading.value = false
        conversationsRequest = null
      }
    })()
    return conversationsRequest
  }

  async function markConversationRead(id: string) {
    if (!id) return null
    const conversation = await markSupportConversationRead(id)
    upsertConversation(conversation)
    return conversation
  }

  return {
    conversations,
    loading,
    unreadTotal,
    hasUnread,
    clear,
    loadConversations,
    markConversationRead,
    setConversations,
    upsertConversation,
  }
})
