import { createApp } from 'vue'
import { createPinia } from 'pinia'
import 'vant/lib/index.css'
import './index.css'
import './styles/vant-cinnabar.css'
import {
  Button, Cell, CellGroup, Field, NavBar, Tabbar, TabbarItem,
  Tab, Tabs, Tag, Grid, GridItem, Loading, Empty, NoticeBar,
  SwipeCell, Dialog, Popup, Form, DropdownMenu, DropdownItem, Switch, Slider,
} from 'vant'
import App from './App.vue'
import router from './router'
import { useAuthStore } from './stores/auth'
import { useBrandingStore } from './stores/branding'

async function bootstrap() {
  const app = createApp(App)
  const pinia = createPinia()
  app.use(pinia)

  const auth = useAuthStore(pinia)
  const branding = useBrandingStore(pinia)
  await branding.loadBranding()
  await auth.loadTokens()

  app.use(router)

  const components = [
    Button, Cell, CellGroup, Field, NavBar, Tabbar, TabbarItem,
    Tab, Tabs, Tag, Grid, GridItem, Loading, Empty, NoticeBar,
    SwipeCell, Dialog, Popup, Form, DropdownMenu, DropdownItem, Switch, Slider,
  ]
  components.forEach(c => app.use(c))

  app.mount('#app')
}

bootstrap()
