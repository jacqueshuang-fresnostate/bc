import { computed, ref } from 'vue'
import { showToast } from 'vant'
import type { BetCartItem, BetPageConfig, DynamicBetPlay } from './types'

// 动态投注引擎只维护当前玩法草稿与本地篮子，不直接发起网络请求。
function emptySelections(play: DynamicBetPlay | null) {
  const keys = play?.option_groups?.length ? play.option_groups.map(group => group.key) : play?.positions || []
  if (play?.option_groups?.length) return Object.fromEntries(play.option_groups.map(group => [group.key, [] as string[]])) as Record<string, string[]>
  return Object.fromEntries((keys as { key: string }[]).map(position => [position.key, [] as string[]])) as Record<string, string[]>
}

export function formatPositionGridNumbers(play: DynamicBetPlay, selections: Record<string, string[]>) {
  // 提交格式保留每个位置的占位顺序，空位置会形成空段并在注数校验中归零。
  return play.positions.map(position => (selections[position.key] || []).join(',')).join('|')
}

export function displayPositionGridNumbers(play: DynamicBetPlay, selections: Record<string, string[]>) {
  // 展示格式隐藏空位置，只让篮子和底栏显示用户已经选择的号码。
  return play.positions.map(position => (selections[position.key] || []).join(',')).filter(Boolean).join(' ')
}

export function formatOptionGroupNumbers(play: DynamicBetPlay, selections: Record<string, string[]>) {
  return play.option_groups.map(group => (selections[group.key] || []).join(',')).join('|')
}

export function displayOptionGroupNumbers(play: DynamicBetPlay, selections: Record<string, string[]>) {
  return play.option_groups.map((group) => {
    const optionLabelMap = new Map(group.options.map(option => [option.value, option.label]))
    const labels = (selections[group.key] || []).map(value => optionLabelMap.get(value) || value)
    return labels.length ? `${group.label}：${labels.join('、')}` : ''
  }).filter(Boolean).join('；')
}

function optionGroupSelectionError(play: DynamicBetPlay, selections: Record<string, string[]>) {
  if (play.option_groups_error) return play.option_groups_error
  for (const group of play.option_groups) {
    const selected = selections[group.key] || []
    if (selected.length < group.min_select_count) return `${group.label}至少选择 ${group.min_select_count} 项`
    if (selected.length > group.max_select_count) return `${group.label}最多选择 ${group.max_select_count} 项`
  }
  return ''
}

export function expandPositionGridNumbers(numbers: string) {
  // 与批量提交保持一致：按位置拆段并展开为逐注号码组合。
  const segments = String(numbers || '').split('|').map(segment => segment.split(',').map(value => value.trim()).filter(Boolean))
  if (!segments.length || segments.some(segment => segment.length === 0)) return []
  return segments.reduce<string[]>((items, segment) => items.flatMap(prefix => segment.map(value => (prefix ? `${prefix}|${value}` : value))), [''])
}

function combinationCount(n: number, k: number) {
  if (k < 0 || n < k) return 0
  let result = 1
  for (let index = 1; index <= k; index += 1) result = (result * (n - index + 1)) / index
  return result
}

function permutationCount(n: number, k: number) {
  if (k < 0 || n < k) return 0
  let result = 1
  for (let index = 0; index < k; index += 1) result *= n - index
  return result
}

function hasOverlap(left: string[], right: string[]) {
  const rightValues = new Set(right)
  return left.some(value => rightValues.has(value))
}

function selectedValues(play: DynamicBetPlay, selections: Record<string, string[]>, index: number) {
  const key = play.positions[index]?.key
  return key ? Array.from(new Set(selections[key] || [])) : []
}

function directPositionGridCount(play: DynamicBetPlay, selections: Record<string, string[]>) {
  return play.positions.reduce((count, position) => {
    if (count === 0) return 0
    const selectedCount = (selections[position.key] || []).length
    return selectedCount > 0 ? count * selectedCount : 0
  }, 1)
}

function countPositionGridBets(play: DynamicBetPlay, selections: Record<string, string[]>) {
  if (play.position_grid_kind === 'direct_combination') return permutationCount(selectedValues(play, selections, 0).length, 3)
  if (play.position_grid_kind === 'group3_compound') {
    const numbers = selectedValues(play, selections, 0)
    return numbers.length >= 2 ? numbers.length * (numbers.length - 1) : 0
  }
  if (play.position_grid_kind === 'group6_compound') return combinationCount(selectedValues(play, selections, 0).length, 3)
  if (play.position_grid_kind === 'group3_dantuo') {
    const dan = selectedValues(play, selections, 0)
    const tuo = selectedValues(play, selections, 1)
    return dan.length === 1 && tuo.length > 0 && !hasOverlap(dan, tuo) ? tuo.length * 2 : 0
  }
  if (play.position_grid_kind === 'group6_dantuo') {
    const dan = selectedValues(play, selections, 0)
    const tuo = selectedValues(play, selections, 1)
    return (dan.length === 1 || dan.length === 2) && !hasOverlap(dan, tuo) ? combinationCount(tuo.length, 3 - dan.length) : 0
  }
  return directPositionGridCount(play, selections)
}

export function useDynamicBetEngine(config: () => BetPageConfig | null, selectedPlay: () => DynamicBetPlay | null) {
  // 草稿区状态：金额、倍数、文本号码、位置选择和未提交篮子互相独立。
  const unitAmount = ref('0')
  const multiple = ref(1)
  const textNumbers = ref('')
  const selections = ref<Record<string, string[]>>({})
  const cart = ref<BetCartItem[]>([])

  const backendUnitAmount = computed(() => {
    const amount = Number(selectedPlay()?.unit_amount || 0)
    return Number.isFinite(amount) && amount > 0 ? String(selectedPlay()?.unit_amount) : '0'
  })
  const fixedUnitAmount = computed(() => backendUnitAmount.value)
  const effectiveUnitAmount = computed(() => backendUnitAmount.value)
  const unitAmountLocked = computed(() => true)
  const minMultiple = computed(() => {
    const value = Number(selectedPlay()?.min_multiple || 1)
    return Number.isSafeInteger(value) && value > 0 ? value : 1
  })
  const maxMultiple = computed(() => {
    const value = Number(selectedPlay()?.max_multiple || 0)
    return Number.isSafeInteger(value) && value >= minMultiple.value ? value : null
  })
  const backendMultiple = computed(() => clampMultiple(selectedPlay()?.multiple || minMultiple.value, selectedPlay()))
  const multipleLocked = computed(() => false)

  function clampMultiple(value: number | string | null | undefined, range: { min_multiple?: number; max_multiple?: number | null } | null = selectedPlay()) {
    const minimum = Number(range?.min_multiple || 1)
    const safeMin = Number.isSafeInteger(minimum) && minimum > 0 ? minimum : 1
    const maximum = Number(range?.max_multiple || 0)
    const safeMax = Number.isSafeInteger(maximum) && maximum >= safeMin ? maximum : null
    const current = Math.floor(Number(value || safeMin))
    const safeValue = Number.isFinite(current) ? Math.max(safeMin, current) : safeMin
    return safeMax == null ? safeValue : Math.min(safeValue, safeMax)
  }

  const draftNumbers = computed(() => {
    const play = selectedPlay()
    if (!play) return ''
    // 不同输入配置生成不同提交号码：配置化选项组、位置宫格、固定选项和文本输入分别处理。
    if (play.option_groups_error) return ''
    if (play.option_groups.length) return formatOptionGroupNumbers(play, selections.value)
    if (play.input_mode === 'position-grid') return formatPositionGridNumbers(play, selections.value)
    if (play.input_mode === 'fixed-option') return play.option_value || ''
    return textNumbers.value
  })
  const draftDisplayNumbers = computed(() => {
    const play = selectedPlay()
    if (!play) return ''
    // 展示号码与提交号码分离，避免配置和位置玩法的内部分隔符直接暴露到卡片 UI。
    if (play.option_groups_error) return play.option_groups_error
    if (play.option_groups.length) return displayOptionGroupNumbers(play, selections.value)
    if (play.input_mode === 'position-grid') return displayPositionGridNumbers(play, selections.value)
    if (play.input_mode === 'fixed-option') return play.option_value || play.name
    return textNumbers.value
  })
  const draftBetCount = computed(() => {
    const play = selectedPlay()
    if (!play) return 0
    // 配置化选项组选满即为一注；位置宫格按后端声明的玩法类型计算注数。
    if (play.option_groups_error) return 0
    if (play.option_groups.length) return draftDisplayNumbers.value && !optionGroupSelectionError(play, selections.value) ? 1 : 0
    if (play.input_mode !== 'position-grid') return draftNumbers.value ? 1 : 0
    return countPositionGridBets(play, selections.value)
  })
  // 底栏汇总同时展示当前草稿金额和已入篮金额，所有金额都由单注金额、注数与倍数计算。
  const draftAmount = computed(() => Number(effectiveUnitAmount.value || 0) * multiple.value * draftBetCount.value)
  const cartTotalCount = computed(() => cart.value.reduce((sum, item) => sum + item.bet_count * item.multiple, 0))
  const cartTotalAmount = computed(() => cart.value.reduce((sum, item) => sum + item.unit_amount * item.bet_count * item.multiple, 0))

  function resetDraft(play = selectedPlay()) {
    // 切换玩法或入篮后重建位置键，固定选项玩法则把 option_value 回填为草稿号码。
    selections.value = emptySelections(play)
    textNumbers.value = play?.input_mode === 'fixed-option' ? (play.option_value || '') : ''
    unitAmount.value = backendUnitAmount.value
    multiple.value = clampMultiple(backendMultiple.value, play)
  }

  function togglePositionNumber(positionKey: string, digit: string) {
    const play = selectedPlay()
    const current = selections.value[positionKey] || []
    if (!current.includes(digit) && play?.max_select_per_position && current.length >= play.max_select_per_position) {
      showToast(`每个位最多选择 ${play.max_select_per_position} 个号码`)
      return
    }
    // 用新对象替换 selections，确保 Vue 能追踪单个位置的增删变化。
    selections.value = {
      ...selections.value,
      [positionKey]: current.includes(digit) ? current.filter(item => item !== digit) : [...current, digit].sort(),
    }
  }

  function setPositionNumbers(positionKey: string, values: string[]) {
    // 批量全选/清空时复制数组，避免调用方继续持有同一引用。
    selections.value = { ...selections.value, [positionKey]: values.slice() }
  }

  function toggleOptionValue(groupKey: string, value: string) {
    const play = selectedPlay()
    const group = play?.option_groups.find(item => item.key === groupKey)
    const option = group?.options.find(item => item.value === value)
    if (!group || !option || option.disabled) return
    const current = selections.value[groupKey] || []
    if (current.includes(value)) {
      selections.value = { ...selections.value, [groupKey]: current.filter(item => item !== value) }
      return
    }
    if (current.length >= group.max_select_count) return
    selections.value = { ...selections.value, [groupKey]: [...current, value] }
  }

  function addDraftToCart() {
    const pageConfig = config()
    const play = selectedPlay()
    // 入篮必须同时具备页面配置、当前期号和当前玩法，保证单据可追溯到期号。
    if (!pageConfig || !pageConfig.round.issue || !play) {
      showToast('当前期号未就绪')
      return false
    }
    if (draftBetCount.value <= 0) {
      showToast(play.option_groups_error || (play.option_groups.length ? optionGroupSelectionError(play, selections.value) || '请选择选项' : '请选择号码'))
      return false
    }
    const amount = Number(effectiveUnitAmount.value || 0)
    if (!Number.isFinite(amount) || amount <= 0) {
      showToast('请输入投注金额')
      return false
    }
    if (cart.value.some(item => item.lottery_code !== pageConfig.lottery.code)) {
      showToast('购彩篮只能加入同一个彩种的投注')
      return false
    }
    if (cart.value.some(item => item.issue !== pageConfig.round.issue)) {
      showToast('当前期号已变化，请先清空购彩篮后重新选择')
      return false
    }
    unitAmount.value = backendUnitAmount.value
    multiple.value = clampMultiple(multiple.value || backendMultiple.value, play)
    // 篮子单据保存提交号码和展示号码两份数据，后续弹层编辑不需要重新读取玩法配置。
    cart.value.push({
      id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
      lottery_code: pageConfig.lottery.code,
      lottery_name: pageConfig.lottery.name,
      issue: pageConfig.round.issue,
      play_code: play.code,
      play_name: play.name,
      numbers: draftNumbers.value,
      display_numbers: draftDisplayNumbers.value,
      unit_amount: amount,
      multiple: multiple.value,
      min_multiple: play.min_multiple,
      max_multiple: play.max_multiple,
      bet_count: draftBetCount.value,
    })
    resetDraft(play)
    showToast('已加入购彩篮')
    return true
  }

  function replaceCart(items: BetCartItem[]) {
    // 弹层确认后用防御性拷贝刷新篮子，倍数保留用户在下单前拖动选择的值。
    cart.value = items.map(item => ({ ...item, multiple: clampMultiple(item.multiple, { min_multiple: item.min_multiple || 1, max_multiple: item.max_multiple ?? null }) }))
  }

  function clearCart() {
    cart.value = []
  }

  return {
    unitAmount,
    fixedUnitAmount,
    effectiveUnitAmount,
    unitAmountLocked,
    minMultiple,
    maxMultiple,
    backendMultiple,
    multipleLocked,
    clampMultiple,
    multiple,
    textNumbers,
    selections,
    cart,
    draftNumbers,
    draftDisplayNumbers,
    draftBetCount,
    draftAmount,
    cartTotalCount,
    cartTotalAmount,
    resetDraft,
    togglePositionNumber,
    setPositionNumbers,
    toggleOptionValue,
    addDraftToCart,
    replaceCart,
    clearCart,
  }
}
