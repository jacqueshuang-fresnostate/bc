<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { Home, MessageCircle, Trophy, UserRound, UsersRound } from 'lucide-vue-next'
import { storeToRefs } from 'pinia'
import { useRoute, useRouter } from 'vue-router'
import { useWebSocket } from '../composables/useWebSocket'
import { useAuthStore } from '../stores/auth'
import { useSupportUnreadStore } from '../stores/supportUnread'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const supportUnreadStore = useSupportUnreadStore()
const { unreadTotal: supportUnreadTotal } = storeToRefs(supportUnreadStore)
const { lastMessage } = useWebSocket()

const navItems = computed(() => [
  { label: '首页', icon: Home, path: '/' },
  { label: '合买', icon: UsersRound, path: '/group-buy' },
  { label: '聊天', icon: MessageCircle, path: '/chat-hall' },
  { label: '开奖', icon: Trophy, path: '/history' },
  { label: '我的', icon: UserRound, path: '/me', unreadCount: supportUnreadTotal.value },
])

const active = computed(() => {
  if (route.path === '/') return 0
  if (route.path.startsWith('/group-buy')) return 1
  if (route.path.startsWith('/chat-hall')) return 2
  if (route.path.startsWith('/history')) return 3
  if (route.path.startsWith('/me') || route.path.startsWith('/orders')) return 4
  return 0
})

const hideBottomNav = computed(() => route.path === '/support' || route.path.startsWith('/bet') || ['/deposit', '/withdraw', '/withdrawal-methods', '/ledger', '/security-center'].includes(route.path))

function onChange(path: string) {
  router.push(path)
}

function refreshSupportUnreadSilently(force = false) {
  void supportUnreadStore.loadConversations({ force, silent: true }).catch(() => {})
}

function badgeContent(count: unknown) {
  const value = Math.max(0, Number(count || 0))
  if (!value) return ''
  return value > 99 ? '99+' : String(value)
}

watch(() => auth.accessToken, (token) => {
  if (!token) {
    supportUnreadStore.clear()
    return
  }
  refreshSupportUnreadSilently(true)
})

watch(lastMessage, (message) => {
  if (
    message?.event === 'support_message_created'
    || message?.event === 'support_conversation_updated'
    || message?.event === 'support_conversation_deleted'
  ) {
    refreshSupportUnreadSilently(true)
  }
})

onMounted(() => {
  refreshSupportUnreadSilently()
})
</script>

<template>
  <div class="min-h-screen bg-surface">
    <router-view :ws-message="lastMessage" />
    <nav v-if="!hideBottomNav" class="mobile-bottom-nav fixed bottom-0 left-0 z-50 w-full px-3 pb-[max(1rem,env(safe-area-inset-bottom))] pt-2">
      <div class="liquid-glass-nav mx-auto flex max-w-md items-center justify-around rounded-[2rem] border border-white/50 bg-white/70 px-2 py-2 text-[10px] font-semibold tracking-wide shadow-[0_-10px_35px_rgba(140,10,21,0.12),inset_0_1px_0_rgba(255,255,255,0.75)] backdrop-blur-2xl saturate-[1.8]">
        <button
          v-for="(item, index) in navItems"
          :key="item.label"
          class="relative flex min-w-0 flex-1 flex-col items-center justify-center rounded-[1.5rem] px-1 py-1.5 transition-all duration-200 active:scale-95"
          :class="active === index ? 'liquid-glass-active text-[#af2829]' : 'text-stone-500 hover:text-red-800'"
          @click="onChange(item.path)"
        >
          <span class="relative z-10 mb-0.5 flex h-7 w-7 items-center justify-center rounded-full transition-transform duration-200" :class="active === index ? 'scale-110 text-[#af2829]' : 'text-stone-500'">
            <component :is="item.icon" class="h-5 w-5 mobile-bottom-nav-icon" :stroke-width="2.4" />
            <van-badge
              v-if="badgeContent(item.unreadCount)"
              class="mobile-bottom-nav__badge absolute -right-1 -top-1"
              :content="badgeContent(item.unreadCount)"
            >
              <span class="mobile-bottom-nav__badge-anchor"></span>
            </van-badge>
          </span>
          <span class="relative z-10 max-w-full truncate" :class="active === index ? 'font-bold text-red-900' : ''">{{ item.label }}</span>
        </button>
      </div>
    </nav>
  </div>
</template>

<style scoped>
.mobile-bottom-nav__badge-anchor {
  display: block;
  width: 1px;
  height: 1px;
}

:deep(.mobile-bottom-nav__badge .van-badge) {
  min-width: 16px;
  height: 16px;
  border: 2px solid #fff;
  background: #dc2626;
  box-shadow: 0 4px 10px rgba(220, 38, 38, 0.24);
  font-size: 9px;
  font-weight: 900;
  line-height: 12px;
}
</style>
