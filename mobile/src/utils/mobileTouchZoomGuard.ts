const NO_ZOOM_VIEWPORT_CONTENT =
  'width=device-width, initial-scale=1.0, minimum-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover'

let installed = false
let touchStartAt = 0
let touchStartX = 0
let touchStartY = 0
let touchMoved = false
let singleTouchStarted = false
let lastTapAt = 0
let lastTapX = 0
let lastTapY = 0

const nonPassiveOptions: AddEventListenerOptions = { passive: false }
const TAP_MAX_DURATION_MS = 280
const DOUBLE_TAP_MAX_INTERVAL_MS = 320
const TOUCH_MOVE_TOLERANCE_PX = 12
const DOUBLE_TAP_POSITION_TOLERANCE_PX = 28

function preventCancelableDefault(event: Event) {
  if (event.cancelable) {
    event.preventDefault()
  }
}

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false
  }
  return Boolean(target.closest('input, textarea, select, [contenteditable="true"]'))
}

function ensureNoZoomViewport() {
  const existing = document.querySelector<HTMLMetaElement>('meta[name="viewport"]')
  if (existing) {
    existing.content = NO_ZOOM_VIEWPORT_CONTENT
    return
  }

  const meta = document.createElement('meta')
  meta.name = 'viewport'
  meta.content = NO_ZOOM_VIEWPORT_CONTENT
  document.head.appendChild(meta)
}

function preventMultiTouchZoom(event: TouchEvent) {
  if (event.touches.length > 1) {
    touchMoved = true
    preventCancelableDefault(event)
  }
}

function rememberSingleTouchStart(event: TouchEvent) {
  if (event.touches.length !== 1) {
    singleTouchStarted = false
    touchMoved = true
    return
  }

  const touch = event.touches[0]
  singleTouchStarted = true
  touchMoved = false
  touchStartAt = Date.now()
  touchStartX = touch.clientX
  touchStartY = touch.clientY
}

function trackSingleTouchMove(event: TouchEvent) {
  if (!singleTouchStarted || event.touches.length !== 1) {
    return
  }

  const touch = event.touches[0]
  const movedX = Math.abs(touch.clientX - touchStartX)
  const movedY = Math.abs(touch.clientY - touchStartY)
  if (movedX > TOUCH_MOVE_TOLERANCE_PX || movedY > TOUCH_MOVE_TOLERANCE_PX) {
    touchMoved = true
  }
}

function preventDoubleTapZoom(event: TouchEvent) {
  if (!singleTouchStarted || touchMoved || isEditableTarget(event.target)) {
    if (touchMoved) {
      lastTapAt = 0
    }
    return
  }

  const now = Date.now()
  if (now - touchStartAt > TAP_MAX_DURATION_MS || event.changedTouches.length !== 1) {
    lastTapAt = 0
    return
  }

  const touch = event.changedTouches[0]
  const movedX = Math.abs(touch.clientX - touchStartX)
  const movedY = Math.abs(touch.clientY - touchStartY)
  if (movedX > TOUCH_MOVE_TOLERANCE_PX || movedY > TOUCH_MOVE_TOLERANCE_PX) {
    lastTapAt = 0
    return
  }

  const tapDistanceX = Math.abs(touch.clientX - lastTapX)
  const tapDistanceY = Math.abs(touch.clientY - lastTapY)
  const isDoubleTap =
    now - lastTapAt < DOUBLE_TAP_MAX_INTERVAL_MS &&
    tapDistanceX < DOUBLE_TAP_POSITION_TOLERANCE_PX &&
    tapDistanceY < DOUBLE_TAP_POSITION_TOLERANCE_PX

  if (isDoubleTap) {
    preventCancelableDefault(event)
    lastTapAt = 0
    return
  }

  lastTapAt = now
  lastTapX = touch.clientX
  lastTapY = touch.clientY
}

// 安装移动端缩放防护：只拦截双指和短距离双击，保留单指上下滚动。
export function installMobileTouchZoomGuard() {
  if (installed || typeof window === 'undefined' || typeof document === 'undefined') {
    return
  }
  installed = true

  ensureNoZoomViewport()

  document.addEventListener('touchstart', rememberSingleTouchStart, { passive: true })
  document.addEventListener('touchmove', trackSingleTouchMove, { passive: true })
  document.addEventListener('touchmove', preventMultiTouchZoom, nonPassiveOptions)
  document.addEventListener('touchend', preventDoubleTapZoom, nonPassiveOptions)
  window.addEventListener('gesturestart', preventCancelableDefault, nonPassiveOptions)
  window.addEventListener('gesturechange', preventCancelableDefault, nonPassiveOptions)
  window.addEventListener('gestureend', preventCancelableDefault, nonPassiveOptions)
}
