import { showConfirmDialog } from 'vant'
import { fetchMobileAppUpdate } from '../api/user'

type AppPlatform = 'android' | 'ios'

type NativeSystemInfo = {
  os: string
  arch: string
  version: string
}

let hasCheckedAppUpdate = false

export async function checkAppUpdateOnce() {
  if (hasCheckedAppUpdate) return
  hasCheckedAppUpdate = true

  try {
    const systemInfo = await readNativeSystemInfo()
    const platform = detectAppPlatform(systemInfo?.os)
    const updateConfig = await fetchMobileAppUpdate({
      platform,
      currentVersion: systemInfo?.version || '0.1.0',
      currentBuild: 1,
    })

    if (
      !updateConfig.enabled ||
      !updateConfig.updateAvailable ||
      !updateConfig.downloadUrl
    ) {
      return
    }

    await showConfirmDialog({
      title: updateConfig.forceUpdate ? '发现必要更新' : '发现新版本',
      message: buildUpdateMessage(updateConfig.latestVersion, updateConfig.releaseNotes),
      confirmButtonText: '立即更新',
      cancelButtonText: '稍后再说',
      closeOnClickOverlay: !updateConfig.forceUpdate,
      showCancelButton: !updateConfig.forceUpdate,
      messageAlign: 'left',
    })
    openDownloadUrl(updateConfig.downloadUrl)
  } catch (error) {
    console.warn('APP 更新检查未完成', error)
  }
}

async function readNativeSystemInfo(): Promise<NativeSystemInfo | null> {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    return await invoke<NativeSystemInfo>('get_system_info')
  } catch {
    return null
  }
}

function detectAppPlatform(os?: string): AppPlatform {
  const normalizedOs = String(os || '').toLowerCase()
  if (normalizedOs.includes('ios') || normalizedOs.includes('darwin')) {
    return 'ios'
  }

  const userAgent = navigator.userAgent.toLowerCase()
  if (userAgent.includes('iphone') || userAgent.includes('ipad')) {
    return 'ios'
  }
  return 'android'
}

function buildUpdateMessage(latestVersion: string, releaseNotes: string) {
  const notes = releaseNotes.trim() || '本次更新包含体验优化和问题修复。'
  return `最新版本：${latestVersion}\n\n更新说明：\n${notes}`
}

function openDownloadUrl(downloadUrl: string) {
  const opened = window.open(downloadUrl, '_blank')
  if (!opened) {
    window.location.href = downloadUrl
  }
}
