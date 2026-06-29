const VIEWPORT_BOTTOM_INSET_PROPERTY = '--mobile-viewport-bottom-inset'
const KEYBOARD_BOTTOM_INSET_PROPERTY = '--mobile-keyboard-bottom-inset'
const MAX_ANDROID_NAV_INSET_PX = 72
const MIN_KEYBOARD_INSET_PX = 96
const MIN_FALLBACK_KEYBOARD_INSET_PX = 260
const MAX_FALLBACK_KEYBOARD_INSET_PX = 430
const FALLBACK_KEYBOARD_RATIO = 0.42

let installed = false
let stableViewportHeight = 0

function isPackagedAppShell() {
  const { hostname, protocol } = window.location
  return hostname === 'tauri.localhost'
    || hostname.endsWith('.tauri.localhost')
    || protocol === 'tauri:'
    || protocol === 'asset:'
    || '__TAURI_INTERNALS__' in window
    || '__TAURI__' in window
}

function normalizeInset(value: number) {
  if (!Number.isFinite(value) || value <= 0) {
    return 0
  }
  return Math.min(MAX_ANDROID_NAV_INSET_PX, Math.round(value))
}

function currentVisualHeight() {
  return window.visualViewport?.height || window.innerHeight || 0
}

function hasEditableFocus() {
  const active = document.activeElement
  if (!(active instanceof HTMLElement)) return false
  return Boolean(active.closest('input, textarea, select, [contenteditable="true"]'))
}

function fallbackKeyboardInset() {
  const baseHeight = stableViewportHeight || window.innerHeight || currentVisualHeight()
  if (!baseHeight) return 0
  return Math.min(
    MAX_FALLBACK_KEYBOARD_INSET_PX,
    Math.max(MIN_FALLBACK_KEYBOARD_INSET_PX, Math.round(baseHeight * FALLBACK_KEYBOARD_RATIO)),
  )
}

function measureKeyboardInset() {
  const measuredInset = measureRawViewportInset()
  // 老安卓 WebView 有时既没有 visualViewport，也不会在输入法弹起时稳定触发窗口高度变化。
  // 只要当前焦点在输入控件里，就给聊天输入栏一个保守兜底高度，避免输入框被键盘完全遮住。
  if (hasEditableFocus() && measuredInset <= MIN_KEYBOARD_INSET_PX) {
    return fallbackKeyboardInset()
  }
  return measuredInset > MIN_KEYBOARD_INSET_PX ? Math.round(measuredInset) : 0
}

function measureRawViewportInset() {
  const viewport = window.visualViewport
  const visualBottomInset = viewport
    ? stableViewportHeight - viewport.height - viewport.offsetTop
    : 0
  const resizeBottomInset = stableViewportHeight - window.innerHeight
  return Math.max(0, visualBottomInset, resizeBottomInset)
}

function refreshStableViewportHeight() {
  const currentHeight = Math.max(window.innerHeight || 0, currentVisualHeight())
  if (!currentHeight) return
  if (!hasEditableFocus()) {
    stableViewportHeight = Math.max(stableViewportHeight, currentHeight)
    return
  }
  stableViewportHeight = Math.max(stableViewportHeight, currentHeight)
}

function keepFocusedInputVisible() {
  const active = document.activeElement
  if (!(active instanceof HTMLElement)) return
  if (!active.closest('input, textarea, select, [contenteditable="true"]')) return

  const rect = active.getBoundingClientRect()
  const visibleHeight = currentVisualHeight() || window.innerHeight
  if (!visibleHeight) return
  if (rect.bottom <= visibleHeight - 18 && rect.top >= 12) return

  active.scrollIntoView({ block: 'center', inline: 'nearest' })
}

function updateViewportInsets() {
  refreshStableViewportHeight()
  const rawViewportInset = measureRawViewportInset()
  const keyboardInset = measureKeyboardInset()
  const bottomInset = normalizeInset(rawViewportInset)
  document.documentElement.style.setProperty(
    VIEWPORT_BOTTOM_INSET_PROPERTY,
    `${bottomInset}px`,
  )
  document.documentElement.style.setProperty(
    KEYBOARD_BOTTOM_INSET_PROPERTY,
    `${keyboardInset}px`,
  )
  document.documentElement.classList.toggle('mobile-keyboard-open', keyboardInset > 0)
  if (keyboardInset > 0) {
    window.setTimeout(keepFocusedInputVisible, 30)
  }
}

function scheduleViewportUpdates() {
  updateViewportInsets()
  window.setTimeout(updateViewportInsets, 80)
  window.setTimeout(updateViewportInsets, 180)
  window.setTimeout(updateViewportInsets, 360)
  window.setTimeout(updateViewportInsets, 720)
}

export function installMobileViewportInsets() {
  if (installed || typeof window === 'undefined') {
    return
  }
  installed = true

  document.documentElement.dataset.mobileShell = isPackagedAppShell() ? 'app' : 'h5'

  // 部分安卓三键导航不会写入 safe-area-inset-bottom，这里用可视视口差值给固定底栏补一个兜底。
  // 输入法弹起时差值会明显变大，额外写入键盘高度，供聊天输入栏避开键盘。
  stableViewportHeight = Math.max(window.innerHeight || 0, currentVisualHeight())
  scheduleViewportUpdates()
  window.visualViewport?.addEventListener('resize', scheduleViewportUpdates)
  window.visualViewport?.addEventListener('scroll', scheduleViewportUpdates)
  window.addEventListener('resize', scheduleViewportUpdates)
  document.addEventListener('focusin', scheduleViewportUpdates)
  document.addEventListener('focusout', scheduleViewportUpdates)
  window.addEventListener('orientationchange', () => {
    stableViewportHeight = 0
    window.setTimeout(scheduleViewportUpdates, 120)
  })
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden) {
      scheduleViewportUpdates()
    }
  })
}
