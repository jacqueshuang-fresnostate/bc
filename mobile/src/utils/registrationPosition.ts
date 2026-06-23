import { isTauri } from '@tauri-apps/api/core'
import type { UserRegistrationPosition } from '../api/user'

const POSITION_OPTIONS = {
  enableHighAccuracy: true,
  timeout: 8000,
  maximumAge: 60000,
}

/// 注册提交前采集一次定位；失败时返回空值，让后端继续使用请求 IP 兜底。
export async function collectRegistrationPosition(): Promise<UserRegistrationPosition | undefined> {
  if (isTauri()) {
    const tauriPosition = await collectTauriRegistrationPosition()
    if (tauriPosition) return tauriPosition
  }

  return collectH5RegistrationPosition()
}

/// Android/iOS App 通过 Tauri 定位插件读取经纬度。
async function collectTauriRegistrationPosition(): Promise<UserRegistrationPosition | undefined> {
  try {
    const {
      checkPermissions,
      requestPermissions,
      getCurrentPosition,
    } = await import('@tauri-apps/plugin-geolocation')
    let permissions = await checkPermissions()
    if (
      permissions.location === 'prompt'
      || permissions.location === 'prompt-with-rationale'
    ) {
      permissions = await requestPermissions(['location'])
    }

    if (
      permissions.location !== 'granted'
      && permissions.coarseLocation !== 'granted'
    ) {
      return undefined
    }

    const position = await getCurrentPosition(POSITION_OPTIONS)
    return {
      latitude: position.coords.latitude,
      longitude: position.coords.longitude,
      accuracy: position.coords.accuracy,
      source: 'tauri',
    }
  } catch {
    return undefined
  }
}

/// H5 通过浏览器标准 Geolocation API 读取经纬度；非 HTTPS 或用户拒绝时会自动降级。
function collectH5RegistrationPosition(): Promise<UserRegistrationPosition | undefined> {
  if (typeof navigator === 'undefined' || !navigator.geolocation) {
    return Promise.resolve(undefined)
  }

  return new Promise((resolve) => {
    navigator.geolocation.getCurrentPosition(
      (position) => {
        resolve({
          latitude: position.coords.latitude,
          longitude: position.coords.longitude,
          accuracy: position.coords.accuracy,
          source: 'h5',
        })
      },
      () => resolve(undefined),
      POSITION_OPTIONS,
    )
  })
}
