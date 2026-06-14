import type { DynamicBetPlay } from './types'

/** 读取玩法中某个位置的最大选号数；返回 null 表示不限制。 */
export function maxSelectForPosition(play: DynamicBetPlay | null | undefined, positionKey: string) {
  if (!play) return null
  const positionLimit = play.position_select_limits.find(limit => limit.position_key === positionKey)
  if (positionLimit?.max_select_count && positionLimit.max_select_count > 0) {
    return positionLimit.max_select_count
  }
  return play.max_select_per_position && play.max_select_per_position > 0
    ? play.max_select_per_position
    : null
}

/** 按当前位置上限裁剪批量选择结果，保留“不配置则不限制”的行为。 */
export function limitPositionValues(play: DynamicBetPlay, positionKey: string, values: string[]) {
  const maxSelect = maxSelectForPosition(play, positionKey)
  return maxSelect ? values.slice(0, maxSelect) : values
}

/** 从候选号码中随机抽取指定数量，展示顺序仍按原号码池顺序排列。 */
export function randomSubsetValues(values: string[], count: number) {
  const normalizedCount = Math.max(0, Math.floor(count))
  if (normalizedCount <= 0) return []
  if (normalizedCount >= values.length) return values.slice()
  const pool = values.map((value, index) => ({ value, index }))
  for (let index = pool.length - 1; index > 0; index -= 1) {
    const swapIndex = Math.floor(Math.random() * (index + 1))
    const current = pool[index]
    pool[index] = pool[swapIndex]
    pool[swapIndex] = current
  }
  const picked = pool.slice(0, normalizedCount)
  if (picked.every(item => item.index < normalizedCount)) {
    picked[picked.length - 1] = pool.slice(normalizedCount).sort((left, right) => left.index - right.index)[0]
  }
  return picked
    .sort((left, right) => left.index - right.index)
    .map(item => item.value)
}

/** 全选遇到位置上限时随机抽取号码，避免每次固定选择 0、1、2... 的前缀。 */
export function randomLimitPositionValues(play: DynamicBetPlay, positionKey: string, values: string[]) {
  const maxSelect = maxSelectForPosition(play, positionKey)
  return maxSelect ? randomSubsetValues(values, maxSelect) : values.slice()
}
