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
