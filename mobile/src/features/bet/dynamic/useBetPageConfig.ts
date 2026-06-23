import { ref } from 'vue'
import { fetchUserBetPageConfig } from '../../../api/bet'
import type {
  BetPageConfig,
  DynamicBetOptionGroup,
  DynamicBetPlay,
  DynamicBetPositionSelectLimit,
  PositionGridKind,
} from './types'

const POSITION_GRID_KINDS: PositionGridKind[] = ['direct', 'direct_combination', 'group3_compound', 'group6_compound', 'group3_dantuo', 'group6_dantuo']

type LoadBetPageConfigOptions = {
  silent?: boolean
}

function normalizePositionGridKind(value: any): PositionGridKind {
  return POSITION_GRID_KINDS.includes(value) ? value : 'direct'
}

function valueOf(source: any, camelKey: string, snakeKey: string = camelKey) {
  return source?.[camelKey] ?? source?.[snakeKey]
}

function normalizeDrawNumbers(value: any): string[] {
  if (Array.isArray(value)) {
    return value
      .map(item => String(item).trim())
      .filter(Boolean)
  }
  const text = String(value ?? '').trim()
  if (!text) return []
  if (/[，,\s|/]+/.test(text)) {
    return text
      .split(/[，,\s|/]+/)
      .map(item => item.trim())
      .filter(Boolean)
  }
  return Array.from(text)
}

function latestDrawNumbers(latestDraw: any): string[] {
  return normalizeDrawNumbers(
    valueOf(latestDraw, 'resultNumbers', 'result_numbers')
      ?? valueOf(latestDraw, 'drawNumber', 'draw_number')
      ?? valueOf(latestDraw, 'drawResult', 'draw_result')
      ?? valueOf(latestDraw, 'resultNumber', 'result_number')
      ?? valueOf(latestDraw, 'result', 'result')
      ?? valueOf(latestDraw, 'openNumber', 'open_number')
      ?? valueOf(latestDraw, 'openCode', 'open_code')
      ?? valueOf(latestDraw, 'numbers', 'numbers')
      ?? valueOf(latestDraw, 'number', 'number'),
  )
}

function normalizeOptionGroups(value: any): DynamicBetOptionGroup[] {
  if (!Array.isArray(value)) return []
  return value.map((group: any) => {
    const options = Array.isArray(group?.options)
      ? group.options.map((option: any) => ({
        value: String(option?.value || '').trim(),
        label: String(option?.label || '').trim(),
        description: String(option?.description || '').trim(),
        odds: option?.odds == null || String(option.odds).trim() === '' ? null : String(option.odds),
        disabled: Boolean(option?.disabled),
      })).filter((option: any) => option.value && option.label)
      : []
    return {
      key: String(group?.key || '').trim(),
      label: String(group?.label || '').trim(),
      min_select_count: Math.max(0, Number(valueOf(group, 'minSelectCount', 'min_select_count') ?? 1)),
      max_select_count: Math.max(0, Number(valueOf(group, 'maxSelectCount', 'max_select_count') ?? 1)),
      options,
    }
  }).filter((group: DynamicBetOptionGroup) => group.key && group.label && group.options.length && group.max_select_count >= group.min_select_count)
}

function normalizeBackendUnitAmount(item: any) {
  const rawAmount = valueOf(item, 'unitAmount', 'unit_amount')
    ?? item?.extra_config?.unit_amount
    ?? item?.extraConfig?.unitAmount
    ?? item?.extra_config?.fixed_unit_amount
  const amount = Number(rawAmount)
  return Number.isFinite(amount) && amount > 0 ? String(rawAmount) : null
}

function normalizeBackendMultiple(item: any) {
  const rawMultiple = item?.multiple ?? item?.extra_config?.multiple ?? item?.extraConfig?.multiple ?? item?.extra_config?.fixed_multiple
  const multiple = Number(rawMultiple)
  return Number.isSafeInteger(multiple) && multiple > 0 ? multiple : null
}

function normalizeMinMultiple(item: any) {
  const value = Number(valueOf(item, 'minMultiple', 'min_multiple') ?? item?.extra_config?.min_multiple ?? item?.extraConfig?.minMultiple ?? 1)
  return Number.isSafeInteger(value) && value > 0 ? value : 1
}

function normalizeMaxMultiple(item: any, minMultiple: number) {
  const rawValue = valueOf(item, 'maxMultiple', 'max_multiple') ?? item?.extra_config?.max_multiple ?? item?.extraConfig?.maxMultiple
  if (rawValue == null || rawValue === '') return null
  const value = Number(rawValue)
  return Number.isSafeInteger(value) && value >= minMultiple ? value : null
}

function normalizeMaxSelectPerPosition(item: any) {
  const rawValue = valueOf(item, 'maxSelectPerPosition', 'max_select_per_position') ?? item?.extra_config?.max_select_per_position ?? item?.extraConfig?.maxSelectPerPosition
  const value = Number(rawValue)
  return Number.isSafeInteger(value) && value > 0 ? value : null
}

function normalizePositionSelectLimits(item: any): DynamicBetPositionSelectLimit[] {
  const rawValue = valueOf(item, 'positionSelectLimits', 'position_select_limits')
    ?? item?.extra_config?.position_select_limits
    ?? item?.extraConfig?.positionSelectLimits
  if (!Array.isArray(rawValue)) return []
  return rawValue.map((limit: any) => {
    const positionKey = String(valueOf(limit, 'positionKey', 'position_key') || '').trim()
    const maxSelectCount = Number(valueOf(limit, 'maxSelectCount', 'max_select_count'))
    return {
      position_key: positionKey,
      max_select_count: Number.isSafeInteger(maxSelectCount) && maxSelectCount > 0 ? maxSelectCount : 0,
    }
  }).filter((limit: DynamicBetPositionSelectLimit) => limit.position_key && limit.max_select_count > 0)
}

// 投注页配置边界：后端返回的松散 JSON 在这里被规整成动态投注页面可直接消费的类型。
function normalizePlay(item: any): DynamicBetPlay {
  // 输入模式只接受动态渲染器支持的枚举，未知模式统一降级为手动文本输入。
  const rawInputMode = valueOf(item, 'inputMode', 'input_mode')
  const inputMode = rawInputMode === 'position-grid'
    || rawInputMode === 'number-grid'
    || rawInputMode === 'fixed-option'
    ? rawInputMode
    : 'text'
  const backendUnitAmount = normalizeBackendUnitAmount(item)
  const backendMultiple = normalizeBackendMultiple(item)
  const minMultiple = normalizeMinMultiple(item)
  const extraConfig = item?.extra_config || item?.extraConfig || {}
  return {
    code: String(item?.code || ''),
    name: String(item?.name || item?.code || ''),
    full_name: String(valueOf(item, 'fullName', 'full_name') || item?.name || item?.code || ''),
    rule_code: String(valueOf(item, 'ruleCode', 'rule_code') || item?.code || ''),
    input_mode: inputMode,
    positions: Array.isArray(item?.positions)
      ? item.positions
        .map((position: any) => ({ key: String(position.key || ''), label: String(position.label || '') }))
        .filter((position: any) => position.key && position.label)
      : [],
    digits: Array.isArray(item?.digits) ? item.digits.map(String) : Array.from({ length: 10 }, (_, index) => String(index)),
    number_grid_values: Array.isArray(valueOf(item, 'numberGridValues', 'number_grid_values'))
      ? valueOf(item, 'numberGridValues', 'number_grid_values').map(String)
      : Array.from({ length: 49 }, (_, index) => String(index + 1).padStart(2, '0')),
    option_value: valueOf(item, 'optionValue', 'option_value') == null ? null : String(valueOf(item, 'optionValue', 'option_value')),
    min_select_count: Math.max(1, Number(valueOf(item, 'minSelectCount', 'min_select_count') || 1)),
    bet_number_count: Math.max(1, Number(valueOf(item, 'betNumberCount', 'bet_number_count') || 1)),
    odds: String(item?.odds || ''),
    unit_amount_fixed: Boolean(valueOf(item, 'unitAmountFixed', 'unit_amount_fixed') ?? true),
    unit_amount: backendUnitAmount,
    multiple_fixed: Boolean(valueOf(item, 'multipleFixed', 'multiple_fixed') ?? false),
    multiple: backendMultiple,
    min_multiple: minMultiple,
    max_multiple: normalizeMaxMultiple(item, minMultiple),
    simple_description: String(valueOf(item, 'simpleDescription', 'simple_description') || ''),
    detail_description: String(valueOf(item, 'detailDescription', 'detail_description') || ''),
    example_description: String(valueOf(item, 'exampleDescription', 'example_description') || ''),
    position_grid_kind: normalizePositionGridKind(valueOf(item, 'positionGridKind', 'position_grid_kind') || extraConfig.position_grid_kind || extraConfig.positionGridKind),
    max_select_per_position: normalizeMaxSelectPerPosition(item),
    position_select_limits: normalizePositionSelectLimits(item),
    option_groups: normalizeOptionGroups(valueOf(item, 'optionGroups', 'option_groups') || extraConfig.option_groups || extraConfig.optionGroups),
    option_groups_error: valueOf(item, 'optionGroupsError', 'option_groups_error') == null ? null : String(valueOf(item, 'optionGroupsError', 'option_groups_error')),
  }
}

export function normalizeBetPageConfig(data: any): BetPageConfig {
  // 页面配置拆成彩种、当前期、最新开奖和玩法清单，缺失字段都用安全默认值兜底。
  const latestDraw = valueOf(data, 'latestDraw', 'latest_draw')
  const latestDrawNumbersFallback = normalizeDrawNumbers(
    valueOf(data, 'latestDrawNumbers', 'latest_draw_numbers')
      ?? valueOf(data, 'latestResultNumbers', 'latest_result_numbers')
      ?? valueOf(data, 'latestDrawNumber', 'latest_draw_number')
      ?? valueOf(data, 'drawNumber', 'draw_number')
      ?? valueOf(data, 'resultNumbers', 'result_numbers'),
  )
  const normalizedLatestDrawNumbers = latestDrawNumbers(latestDraw)
  const normalizedLatestDrawIssue = String(
    latestDraw?.issue
      ?? valueOf(latestDraw, 'issueNumber', 'issue_number')
      ?? valueOf(data, 'latestIssue', 'latest_issue')
      ?? valueOf(data, 'latestDrawIssue', 'latest_draw_issue')
      ?? '',
  )
  const hasLatestDraw = Boolean(latestDraw) || normalizedLatestDrawNumbers.length > 0 || latestDrawNumbersFallback.length > 0 || normalizedLatestDrawIssue
  return {
    lottery: {
      code: String(data?.lottery?.code || ''),
      name: String(data?.lottery?.name || data?.lottery?.code || ''),
      category: String(data?.lottery?.category || ''),
      draw_interval: Number(valueOf(data?.lottery, 'drawInterval', 'draw_interval') || 0),
      group_buy_enabled: Boolean(valueOf(data?.lottery, 'groupBuyEnabled', 'group_buy_enabled')),
    },
    group_buy_settings: {
      min_share_amount: String(valueOf(valueOf(data, 'groupBuySettings', 'group_buy_settings'), 'minShareAmount', 'min_share_amount') || data?.settings?.min_share_amount || data?.min_share_amount || '0.01'),
      initiator_min_buy_ratio: String(valueOf(valueOf(data, 'groupBuySettings', 'group_buy_settings'), 'initiatorMinBuyRatio', 'initiator_min_buy_ratio') || data?.settings?.initiator_min_buy_ratio || data?.initiator_min_buy_ratio || '0.00'),
      share_amount: String(valueOf(valueOf(data, 'groupBuySettings', 'group_buy_settings'), 'shareAmount', 'share_amount') || data?.settings?.share_amount || data?.share_amount || '1.00'),
    },
    round: {
      issue: String(data?.round?.issue || ''),
      status: String(data?.round?.status || ''),
      scheduled_draw_at: valueOf(data?.round, 'scheduledDrawAt', 'scheduled_draw_at') == null ? null : String(valueOf(data?.round, 'scheduledDrawAt', 'scheduled_draw_at')),
      sale_stop_at: valueOf(data?.round, 'saleStopAt', 'sale_stop_at') == null ? null : String(valueOf(data?.round, 'saleStopAt', 'sale_stop_at')),
    },
    latest_draw: hasLatestDraw ? {
      issue: normalizedLatestDrawIssue,
      result_numbers: normalizedLatestDrawNumbers.length ? normalizedLatestDrawNumbers : latestDrawNumbersFallback,
      opened_at: valueOf(latestDraw, 'openedAt', 'opened_at') == null ? null : String(valueOf(latestDraw, 'openedAt', 'opened_at')),
    } : null,
    plays: Array.isArray(data?.plays) ? data.plays.map(normalizePlay).filter((play: DynamicBetPlay) => play.code) : [],
  }
}

export function useBetPageConfig() {
  const config = ref<BetPageConfig | null>(null)
  const loading = ref(false)

  async function loadBetPageConfig(lotteryCode: string, options: LoadBetPageConfigOptions = {}) {
    // 路由未带彩种时主动清空配置，避免投注页继续展示上一个彩种的玩法。
    if (!lotteryCode) {
      config.value = null
      return null
    }
    const showLoading = !options.silent
    if (showLoading) loading.value = true
    try {
      // 每次加载都重新规范化服务端配置，让动态玩法编辑后前端立即使用最新输入规则。
      const data = await fetchUserBetPageConfig(lotteryCode)
      config.value = normalizeBetPageConfig(data)
      return config.value
    } finally {
      if (showLoading) loading.value = false
    }
  }

  return { config, loading, loadBetPageConfig }
}
