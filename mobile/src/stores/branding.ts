import { defineStore } from 'pinia'
import { ref } from 'vue'
import { fetchMobileSiteConfig } from '../api/user'

export type BrandingSettings = {
  site_name: string
  logo_url: string
  slogan: string
  footer_text: string
}

export const DEFAULT_BRANDING: BrandingSettings = {
  site_name: '鸿福',
  logo_url: '/logo.svg',
  slogan: '开启您的幸运之门',
  footer_text: '传承现代美学 • 尊享服务',
}

function normalizeBranding(data: Partial<BrandingSettings> | null | undefined): BrandingSettings {
  return {
    site_name: data?.site_name || DEFAULT_BRANDING.site_name,
    logo_url: data?.logo_url || DEFAULT_BRANDING.logo_url,
    slogan: data?.slogan || DEFAULT_BRANDING.slogan,
    footer_text: data?.footer_text || DEFAULT_BRANDING.footer_text,
  }
}

export const useBrandingStore = defineStore('branding', () => {
  const branding = ref<BrandingSettings>({ ...DEFAULT_BRANDING })
  const loaded = ref(false)

  async function loadBranding() {
    if (loaded.value) return
    try {
      const config = await fetchMobileSiteConfig()
      branding.value = normalizeBranding({
        site_name: config.platformName,
        logo_url: config.logoImageUrl || undefined,
        slogan: config.intro,
      })
    } catch {
      branding.value = { ...DEFAULT_BRANDING }
    } finally {
      loaded.value = true
      document.title = branding.value.site_name || DEFAULT_BRANDING.site_name
    }
  }

  function setLogoFallback() {
    if (branding.value.logo_url !== DEFAULT_BRANDING.logo_url) {
      branding.value = { ...branding.value, logo_url: DEFAULT_BRANDING.logo_url }
    }
  }

  return { branding, loaded, loadBranding, setLogoFallback }
})
