import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

export type MobileThemeId = 'rainbow' | 'cinnabar'

export type MobileThemeOption = {
  id: MobileThemeId
  label: string
  description: string
}

const MOBILE_THEME_STORAGE_KEY = 'mobile_theme_id'
const DEFAULT_MOBILE_THEME: MobileThemeId = 'rainbow'

export const MOBILE_THEME_OPTIONS: MobileThemeOption[] = [
  {
    id: 'rainbow',
    label: '彩虹',
    description: '保留当前柔和彩虹渐变配色',
  },
  {
    id: 'cinnabar',
    label: '殷红',
    description: '中国风彩票红金配色',
  },
]

function normalizeThemeId(value: unknown): MobileThemeId {
  return MOBILE_THEME_OPTIONS.some(option => option.id === value)
    ? value as MobileThemeId
    : DEFAULT_MOBILE_THEME
}

function readInitialTheme(): MobileThemeId {
  try {
    return normalizeThemeId(localStorage.getItem(MOBILE_THEME_STORAGE_KEY))
  } catch {
    return DEFAULT_MOBILE_THEME
  }
}

function persistTheme(themeId: MobileThemeId) {
  try {
    localStorage.setItem(MOBILE_THEME_STORAGE_KEY, themeId)
  } catch {
    // 主题偏好属于本地 UI 状态，持久化失败时保留当前运行时主题即可。
  }
}

function applyThemeAttribute(themeId: MobileThemeId) {
  if (typeof document === 'undefined') return
  document.documentElement.dataset.mobileTheme = themeId
}

// 手机端主题配色：默认彩虹保留当前视觉，殷红提供中国风彩票红金配色。
export const useMobileThemeStore = defineStore('mobileTheme', () => {
  const currentTheme = ref<MobileThemeId>(readInitialTheme())
  const themeOptions = MOBILE_THEME_OPTIONS
  const currentThemeOption = computed(() => (
    themeOptions.find(option => option.id === currentTheme.value) ?? themeOptions[0]
  ))
  const currentThemeLabel = computed(() => currentThemeOption.value.label)

  function applyTheme(themeId: MobileThemeId = currentTheme.value) {
    const normalizedThemeId = normalizeThemeId(themeId)
    currentTheme.value = normalizedThemeId
    applyThemeAttribute(normalizedThemeId)
  }

  function setTheme(themeId: MobileThemeId) {
    applyTheme(themeId)
    persistTheme(currentTheme.value)
  }

  return {
    currentTheme,
    currentThemeLabel,
    currentThemeOption,
    themeOptions,
    applyTheme,
    setTheme,
  }
})
