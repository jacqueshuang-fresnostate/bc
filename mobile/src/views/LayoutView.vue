<script setup lang="ts">
import { computed } from 'vue'
import { Home, Trophy, UserRound, UsersRound } from 'lucide-vue-next'
import { useRoute, useRouter } from 'vue-router'
import { useWebSocket } from '../composables/useWebSocket'

const route = useRoute()
const router = useRouter()
const { lastMessage } = useWebSocket()

const navItems = [
  { label: '首页', icon: Home, path: '/' },
  { label: '合买', icon: UsersRound, path: '/group-buy' },
  { label: '开奖', icon: Trophy, path: '/history' },
  { label: '我的', icon: UserRound, path: '/me' },
]

const active = computed(() => {
  if (route.path === '/') return 0
  if (route.path.startsWith('/group-buy')) return 1
  if (route.path.startsWith('/history') || route.path.startsWith('/orders')) return 2
  if (route.path.startsWith('/me')) return 3
  return 0
})

const hideBottomNav = computed(() => route.path === '/support' || route.path.startsWith('/bet') || ['/deposit', '/withdraw', '/withdrawal-methods', '/security-center'].includes(route.path))

function onChange(path: string) {
  router.push(path)
}
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
          </span>
          <span class="relative z-10 max-w-full truncate" :class="active === index ? 'font-bold text-red-900' : ''">{{ item.label }}</span>
        </button>
      </div>
    </nav>
  </div>
</template>
