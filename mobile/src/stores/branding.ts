import { defineStore } from 'pinia'
import { ref } from 'vue'
import { fetchMobileSiteConfig } from '../api/user'

const BRANDING_CACHE_MS = 60_000
const BLANK_LOGO_DATA_URL =
  'data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw=='

export type BrandingSettings = {
  site_name: string
  logo_url: string
  slogan: string
  footer_text: string
}

export const DEFAULT_BRANDING: BrandingSettings = {
  site_name: '',
  logo_url: BLANK_LOGO_DATA_URL,
  slogan: '',
  footer_text: '',
}

type LoadBrandingOptions = {
  force?: boolean
  silent?: boolean
}

type PackagedBrandingConfig = {
  footer_text?: string
  intro?: string
  logoImageUrl?: string
  logo_url?: string
  platformName?: string
  site_name?: string
  slogan?: string
}

function cleanConfigValue(value: string | null | undefined) {
  const normalized = String(value ?? '').trim()
  if (!normalized || normalized === '未配置') return ''
  return normalized
}

function normalizeBranding(data: Partial<BrandingSettings> | null | undefined): BrandingSettings {
  return {
    site_name: cleanConfigValue(data?.site_name) || DEFAULT_BRANDING.site_name,
    logo_url: cleanConfigValue(data?.logo_url) || DEFAULT_BRANDING.logo_url,
    slogan: cleanConfigValue(data?.slogan) || DEFAULT_BRANDING.slogan,
    footer_text: cleanConfigValue(data?.footer_text) || DEFAULT_BRANDING.footer_text,
  }
}

function normalizePackagedBranding(data: PackagedBrandingConfig | null | undefined): Partial<BrandingSettings> {
  return {
    site_name: cleanConfigValue(data?.platformName || data?.site_name),
    logo_url: cleanConfigValue(data?.logoImageUrl || data?.logo_url),
    slogan: cleanConfigValue(data?.intro || data?.slogan),
    footer_text: cleanConfigValue(data?.footer_text),
  }
}

function hasBrandingValue(data: Partial<BrandingSettings>) {
  return Boolean(data.site_name || data.logo_url || data.slogan || data.footer_text)
}

export const useBrandingStore = defineStore('branding', () => {
  const branding = ref<BrandingSettings>({ ...DEFAULT_BRANDING })
  const loaded = ref(false)
  const loading = ref(false)
  const loadedAt = ref(0)
  let loadingRequest: Promise<void> | null = null

  async function loadBranding(options: LoadBrandingOptions = {}) {
    if (
      !options.force
      && loaded.value
      && loadedAt.value > 0
      && Date.now() - loadedAt.value < BRANDING_CACHE_MS
    ) return
    if (loadingRequest) return loadingRequest

    if (!options.silent) loading.value = true
    loadingRequest = (async () => {
      try {
        const config = await fetchMobileSiteConfig()
        branding.value = normalizeBranding({
          site_name: config.platformName,
          logo_url: config.logoImageUrl || undefined,
          slogan: config.intro,
        })
        loadedAt.value = Date.now()
      } catch {
        if (!loaded.value) branding.value = { ...DEFAULT_BRANDING }
      } finally {
        loaded.value = true
        if (!options.silent) loading.value = false
        document.title = branding.value.site_name || DEFAULT_BRANDING.site_name
        loadingRequest = null
      }
    })()
    return loadingRequest
  }

  async function loadPackagedBranding() {
    try {
      const response = await fetch('/mobile-branding.json', {
        cache: 'no-store',
        credentials: 'omit',
      })
      if (!response.ok) return false

      const config = normalizePackagedBranding(await response.json() as PackagedBrandingConfig)
      if (!hasBrandingValue(config)) return false
      applyBranding(config)
      return true
    } catch {
      return false
    }
  }

  async function refreshBranding() {
    await loadBranding({ force: true, silent: true })
  }

  function applyBranding(value: Partial<BrandingSettings>) {
    branding.value = normalizeBranding(value)
    loaded.value = true
    loadedAt.value = Date.now()
    document.title = branding.value.site_name || DEFAULT_BRANDING.site_name
  }

  function invalidateBranding() {
    loadedAt.value = 0
  }

  function setLogoFallback() {
    if (branding.value.logo_url !== DEFAULT_BRANDING.logo_url) {
      branding.value = { ...branding.value, logo_url: DEFAULT_BRANDING.logo_url }
    }
  }

  return {
    branding,
    loaded,
    loading,
    loadedAt,
    applyBranding,
    invalidateBranding,
    loadBranding,
    loadPackagedBranding,
    refreshBranding,
    setLogoFallback,
  }
})
