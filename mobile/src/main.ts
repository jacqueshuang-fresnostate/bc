import { createApp } from 'vue'
import { createPinia } from 'pinia'
import 'vant/lib/index.css'
import './index.css'
import './styles/vant-cinnabar.css'
import {
  Badge, Button, Cell, CellGroup, Field, NavBar, Tabbar, TabbarItem,
  Tab, Tabs, Tag, Grid, GridItem, Loading, Empty, NoticeBar,
  SwipeCell, Dialog, Popup, Form, DropdownMenu, DropdownItem, Switch, Slider,
} from 'vant'
import App from './App.vue'
import router from './router'
import { useAuthStore } from './stores/auth'
import { useBrandingStore } from './stores/branding'
import { checkAppUpdateOnce } from './composables/useAppUpdateCheck'
import { preloadMobileRoutes } from './utils/preloadMobileRoutes'

async function bootstrap() {
  const app = createApp(App)
  const pinia = createPinia()
  app.use(pinia)

  const auth = useAuthStore(pinia)
  const branding = useBrandingStore(pinia)
  await auth.loadTokens()

  app.use(router)

  const components = [
    Badge, Button, Cell, CellGroup, Field, NavBar, Tabbar, TabbarItem,
    Tab, Tabs, Tag, Grid, GridItem, Loading, Empty, NoticeBar,
    SwipeCell, Dialog, Popup, Form, DropdownMenu, DropdownItem, Switch, Slider,
  ]
  components.forEach(c => app.use(c))

  app.mount('#app')
  void branding.loadBranding()
  void checkAppUpdateOnce()
  preloadMobileRoutes()
}

bootstrap().catch((error) => {
  console.error('手机端应用启动失败', error)
  const root = document.getElementById('app')
  if (!root) return
  root.innerHTML = `
    <main style="min-height:100vh;display:flex;align-items:center;justify-content:center;padding:24px;background:#f9f9f9;color:#1a1c1c;font-family:system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;">
      <section style="width:min(100%,360px);border-radius:24px;background:#fff;padding:24px;text-align:center;box-shadow:0 18px 48px rgba(140,10,21,0.12);">
        <h1 style="margin:0 0 10px;font-size:20px;font-weight:900;">应用启动失败</h1>
        <p style="margin:0;color:#8e706d;font-size:14px;font-weight:700;line-height:1.7;">请关闭应用后重新打开；如果仍然异常，请联系平台客服处理。</p>
      </section>
    </main>
  `
})
