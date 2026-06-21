<script setup lang="ts">
import {
  ArrowLeft,
  Bitcoin,
  Camera,
  CircleArrowDown,
  CirclePlus,
  Flame,
  FileText,
  Gauge,
  Gem,
  Grid2X2,
  Headset,
  HelpCircle,
  Eye,
  EyeOff,
  Landmark,
  LogOut,
  MessageCircle,
  MoreVertical,
  Plus,
  QrCode,
  RotateCw,
  Search,
  Send,
  Settings,
  Shield,
  Smile,
  Star,
  Wallet,
  X,
  Zap,
} from 'lucide-vue-next'

const props = withDefaults(defineProps<{
  name?: string | null
  strokeWidth?: number
}>(), {
  strokeWidth: 2.4,
})

const icons = {
  account_balance: Landmark,
  account_balance_wallet: Wallet,
  add: Plus,
  add_circle: CirclePlus,
  apps: Grid2X2,
  arrow_back: ArrowLeft,
  bolt: Zap,
  camera: Camera,
  chat: MessageCircle,
  chevron_right: ArrowLeft,
  close: X,
  currency_bitcoin: Bitcoin,
  diamond: Gem,
  group: CirclePlus,
  grid_view: Grid2X2,
  help: HelpCircle,
  local_fire_department: Flame,
  logout: LogOut,
  more_vert: MoreVertical,
  mood: Smile,
  output_circle: CircleArrowDown,
  payments: Wallet,
  qr_code_scanner: QrCode,
  receipt_text: FileText,
  refresh: RotateCw,
  search: Search,
  send: Send,
  settings: Settings,
  shield_lock: Shield,
  speed: Gauge,
  star: Star,
  support_agent: Headset,
  visibility: Eye,
  visibility_off: EyeOff,
} as const

const fallbackIcon = CirclePlus
</script>

<template>
  <component :is="icons[props.name as keyof typeof icons] || fallbackIcon" aria-hidden="true" :stroke-width="strokeWidth" />
</template>
