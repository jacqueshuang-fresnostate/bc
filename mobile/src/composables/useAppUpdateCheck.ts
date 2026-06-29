import { showConfirmDialog, showToast } from 'vant'
import { fetchMobileAppUpdate } from '../api/user'
import { openUrl } from '@tauri-apps/plugin-opener'

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
    if (!systemInfo) {
      return
    }

    const platform = detectAppPlatform(systemInfo.os)
    const currentVersion = await readCurrentAppVersion(systemInfo)
    const updateConfig = await fetchMobileAppUpdate({
      platform,
      currentVersion,
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
      message: buildUpdateMessage(
        currentVersion,
        updateConfig.latestVersion,
        updateConfig.releaseNotes,
      ),
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

async function readCurrentAppVersion(systemInfo: NativeSystemInfo | null) {
  try {
    const { getVersion } = await import('@tauri-apps/api/app')
    return (await getVersion()) || systemInfo?.version || '0.1.0'
  } catch {
    return systemInfo?.version || '0.1.0'
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

function buildUpdateMessage(currentVersion: string, latestVersion: string, releaseNotes: string) {
  const notes = releaseNotes.trim() || '本次更新包含体验优化和问题修复。'
  return `当前版本：${currentVersion}\n最新版本：${latestVersion}\n\n更新说明：\n${notes}`
}

function openDownloadUrl(downloadUrl: string) {
  showToast('正在跳转下载页面…')
  try {
    openUrl(downloadUrl)
  } catch {
    const opened = window.open(downloadUrl, '_blank')
    if (!opened) {
      showToast('下载链接打开失败，请检查网络后重试')
      window.location.href = downloadUrl
    }
  }
}
