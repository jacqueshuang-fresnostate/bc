const CACHE_TTL_MS = 30 * 24 * 60 * 60 * 1000
const MAX_PERSISTENT_DATA_URL_LENGTH = 900_000
const MAX_PERSISTED_ITEMS = 80
const STORAGE_PREFIX = 'bc_avatar_image_cache:'
const STORAGE_INDEX_KEY = `${STORAGE_PREFIX}index`

type CacheIndexItem = {
  key: string
  updatedAt: number
}

type CachedAvatarRecord = {
  sourceUrl: string
  dataUrl: string
  updatedAt: number
}

const memoryCache = new Map<string, string>()
const pendingCache = new Map<string, Promise<string>>()

export async function cachedAvatarImageUrl(value: string | null | undefined) {
  const sourceUrl = normalizeImageUrl(value)
  if (!sourceUrl) return ''

  const memoryValue = memoryCache.get(sourceUrl)
  if (memoryValue) return memoryValue

  const storedValue = readStoredAvatar(sourceUrl)
  if (storedValue) {
    memoryCache.set(sourceUrl, storedValue)
    return storedValue
  }

  const pendingValue = pendingCache.get(sourceUrl)
  if (pendingValue) return pendingValue

  const task = resolveAvatarImageUrl(sourceUrl).finally(() => pendingCache.delete(sourceUrl))
  pendingCache.set(sourceUrl, task)
  return task
}

async function resolveAvatarImageUrl(sourceUrl: string) {
  const browserDataUrl = await fetchAvatarAsDataUrl(sourceUrl)
  if (browserDataUrl) return rememberAvatar(sourceUrl, browserDataUrl)

  const tauriDataUrl = await downloadAvatarViaTauri(sourceUrl)
  if (tauriDataUrl) return rememberAvatar(sourceUrl, tauriDataUrl)

  memoryCache.set(sourceUrl, sourceUrl)
  return sourceUrl
}

function normalizeImageUrl(value: string | null | undefined) {
  return String(value ?? '').trim()
}

async function fetchAvatarAsDataUrl(sourceUrl: string) {
  if (typeof fetch !== 'function') return ''

  try {
    const response = await fetch(sourceUrl, {
      cache: 'force-cache',
      credentials: 'omit',
      mode: 'cors',
    })
    if (!response.ok) return ''

    const contentType = response.headers.get('content-type') || ''
    if (contentType && !contentType.startsWith('image/')) return ''

    const blob = await response.blob()
    if (!blob.type.startsWith('image/')) return ''
    if (blob.size > MAX_PERSISTENT_DATA_URL_LENGTH) return ''
    return await blobToDataUrl(blob)
  } catch {
    return ''
  }
}

async function downloadAvatarViaTauri(sourceUrl: string) {
  try {
    const { invoke } = await import('@tauri-apps/api/core')
    const dataUrl = await invoke<string>('cache_avatar_image', { url: sourceUrl })
    return isImageDataUrl(dataUrl) ? dataUrl : ''
  } catch {
    return ''
  }
}

function blobToDataUrl(blob: Blob) {
  return new Promise<string>((resolve) => {
    const reader = new FileReader()
    reader.onload = () => {
      const result = typeof reader.result === 'string' ? reader.result : ''
      resolve(isImageDataUrl(result) ? result : '')
    }
    reader.onerror = () => resolve('')
    reader.readAsDataURL(blob)
  })
}

function rememberAvatar(sourceUrl: string, dataUrl: string) {
  memoryCache.set(sourceUrl, dataUrl)
  if (dataUrl.length <= MAX_PERSISTENT_DATA_URL_LENGTH) {
    writeStoredAvatar(sourceUrl, dataUrl)
  }
  return dataUrl
}

function readStoredAvatar(sourceUrl: string) {
  const record = readStorageRecord(storageKeyForUrl(sourceUrl))
  if (!record || record.sourceUrl !== sourceUrl) return ''
  if (!isImageDataUrl(record.dataUrl)) return ''
  if (Date.now() - record.updatedAt > CACHE_TTL_MS) {
    removeStoredAvatar(sourceUrl)
    return ''
  }
  touchStorageIndex(sourceUrl)
  return record.dataUrl
}

function writeStoredAvatar(sourceUrl: string, dataUrl: string) {
  const record: CachedAvatarRecord = {
    dataUrl,
    sourceUrl,
    updatedAt: Date.now(),
  }
  safeLocalStorage(() => {
    localStorage.setItem(storageKeyForUrl(sourceUrl), JSON.stringify(record))
    touchStorageIndex(sourceUrl)
    trimStorageIndex()
  })
}

function removeStoredAvatar(sourceUrl: string) {
  safeLocalStorage(() => {
    const key = storageKeyForUrl(sourceUrl)
    localStorage.removeItem(key)
    const nextIndex = readStorageIndex().filter(item => item.key !== key)
    localStorage.setItem(STORAGE_INDEX_KEY, JSON.stringify(nextIndex))
  })
}

function touchStorageIndex(sourceUrl: string) {
  const key = storageKeyForUrl(sourceUrl)
  const now = Date.now()
  safeLocalStorage(() => {
    const nextIndex = [
      { key, updatedAt: now },
      ...readStorageIndex().filter(item => item.key !== key),
    ]
    localStorage.setItem(STORAGE_INDEX_KEY, JSON.stringify(nextIndex))
  })
}

function trimStorageIndex() {
  safeLocalStorage(() => {
    const index = readStorageIndex().sort((left, right) => right.updatedAt - left.updatedAt)
    const keep = index.slice(0, MAX_PERSISTED_ITEMS)
    const remove = index.slice(MAX_PERSISTED_ITEMS)
    for (const item of remove) localStorage.removeItem(item.key)
    localStorage.setItem(STORAGE_INDEX_KEY, JSON.stringify(keep))
  })
}

function readStorageRecord(key: string) {
  return safeLocalStorage(() => {
    const raw = localStorage.getItem(key)
    if (!raw) return null
    const parsed = JSON.parse(raw) as Partial<CachedAvatarRecord>
    if (
      typeof parsed.sourceUrl !== 'string'
      || typeof parsed.dataUrl !== 'string'
      || typeof parsed.updatedAt !== 'number'
    ) {
      return null
    }
    return parsed as CachedAvatarRecord
  }) || null
}

function readStorageIndex() {
  return safeLocalStorage(() => {
    const raw = localStorage.getItem(STORAGE_INDEX_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw) as Partial<CacheIndexItem>[]
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter(item => typeof item.key === 'string' && typeof item.updatedAt === 'number')
      .map(item => ({ key: item.key as string, updatedAt: item.updatedAt as number }))
  }) || []
}

function storageKeyForUrl(sourceUrl: string) {
  return `${STORAGE_PREFIX}${hashString(sourceUrl)}`
}

function hashString(value: string) {
  let hash = 2166136261
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index)
    hash = Math.imul(hash, 16777619)
  }
  return (hash >>> 0).toString(36)
}

function isImageDataUrl(value: string) {
  return /^data:image\/[a-z0-9.+-]+;base64,/i.test(value)
}

function safeLocalStorage<T>(callback: () => T) {
  try {
    if (typeof localStorage === 'undefined') return null
    return callback()
  } catch {
    return null
  }
}
