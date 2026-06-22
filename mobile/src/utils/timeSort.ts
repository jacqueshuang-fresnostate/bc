export interface SortableTimeRecord {
  createdAt?: string | null
  created_at?: string | null
  id?: string | number | null
}

export function sortByCreatedTimeDesc<T extends SortableTimeRecord>(items: T[]): T[] {
  return [...items].sort(compareCreatedTimeDesc)
}

export function compareCreatedTimeDesc(left: SortableTimeRecord, right: SortableTimeRecord) {
  const leftCreatedAt = createdTimeValue(left)
  const rightCreatedAt = createdTimeValue(right)
  return (
    timeScore(rightCreatedAt) - timeScore(leftCreatedAt)
    || rightCreatedAt.localeCompare(leftCreatedAt)
    || String(right.id ?? '').localeCompare(String(left.id ?? ''))
  )
}

function createdTimeValue(item: SortableTimeRecord) {
  return String(item.createdAt || item.created_at || '')
}

function timeScore(value: string) {
  const normalized = value.trim()
  if (!normalized) return 0
  if (normalized.startsWith('unix:')) {
    const seconds = Number(normalized.slice(5))
    return Number.isFinite(seconds) ? seconds * 1000 : 0
  }
  const parsed = Date.parse(normalized.replace(' ', 'T'))
  return Number.isFinite(parsed) ? parsed : 0
}
