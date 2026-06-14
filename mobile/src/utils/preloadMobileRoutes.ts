type RoutePreloader = () => Promise<unknown>
type WindowWithIdleCallback = Window & {
  requestIdleCallback?: (
    callback: () => void,
    options?: { timeout?: number },
  ) => number
}

const routePreloaders: RoutePreloader[] = [
  () => import('../views/LayoutView.vue'),
  () => import('../views/HomeView.vue'),
  () => import('../views/GroupBuyView.vue'),
  () => import('../views/ChatHallView.vue'),
  () => import('../views/HistoryView.vue'),
  () => import('../views/ProfileView.vue'),
  () => import('../views/DepositView.vue'),
  () => import('../views/SupportView.vue'),
  () => import('../views/WithdrawView.vue'),
  () => import('../views/InvitationCenterView.vue'),
]

let preloaded = false

/** 在首屏挂载后分批预热常用页面 chunk，减少首次点击入口时的等待感。 */
export function preloadMobileRoutes() {
  if (preloaded || typeof window === 'undefined') {
    return
  }
  preloaded = true

  scheduleIdleTask(() => {
    let index = 0
    const loadNext = () => {
      const preload = routePreloaders[index]
      index += 1
      if (!preload) {
        return
      }

      void preload().catch(() => {})
      scheduleIdleTask(loadNext, 120)
    }

    loadNext()
  }, 900)
}

function scheduleIdleTask(callback: () => void, fallbackDelayMs: number) {
  const idleWindow = window as WindowWithIdleCallback
  if (idleWindow.requestIdleCallback) {
    idleWindow.requestIdleCallback(callback, { timeout: fallbackDelayMs + 800 })
    return
  }

  window.setTimeout(callback, fallbackDelayMs)
}
