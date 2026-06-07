export type DynamicInputMode = 'position-grid' | 'number-grid' | 'fixed-option' | 'text'
export type PositionGridKind = 'direct' | 'direct_combination' | 'group3_compound' | 'group6_compound' | 'group3_dantuo' | 'group6_dantuo'

export type DynamicBetPosition = {
  key: string
  label: string
}

export type DynamicBetOption = {
  value: string
  label: string
  description: string
  odds: string | null
  disabled: boolean
}

export type DynamicBetOptionGroup = {
  key: string
  label: string
  min_select_count: number
  max_select_count: number
  options: DynamicBetOption[]
}

export type DynamicBetPositionSelectLimit = {
  position_key: string
  max_select_count: number
}

export type DynamicBetPlay = {
  code: string
  name: string
  full_name: string
  rule_code: string
  input_mode: DynamicInputMode
  positions: DynamicBetPosition[]
  digits: string[]
  number_grid_values: string[]
  option_value: string | null
  min_select_count: number
  bet_number_count: number
  odds: string
  unit_amount_fixed: boolean
  unit_amount: string | null
  multiple_fixed: boolean
  multiple: number | null
  min_multiple: number
  max_multiple: number | null
  simple_description: string
  detail_description: string
  example_description: string
  position_grid_kind: PositionGridKind
  max_select_per_position: number | null
  position_select_limits: DynamicBetPositionSelectLimit[]
  option_groups: DynamicBetOptionGroup[]
  option_groups_error: string | null
}

export type BetPageConfig = {
  lottery: {
    code: string
    name: string
    category: string
    draw_interval: number
    group_buy_enabled: boolean
  }
  group_buy_settings: {
    min_share_amount: string
    initiator_min_buy_ratio: string
    share_amount: string
  }
  round: {
    issue: string
    status: string
    scheduled_draw_at: string | null
    sale_stop_at: string | null
  }
  latest_draw: {
    issue: string
    result_numbers: string[]
    opened_at: string | null
  } | null
  plays: DynamicBetPlay[]
}

export type DynamicBetDraft = {
  lottery_code: string
  issue: string
  play_code: string
  play_name: string
  input_mode: DynamicInputMode
  selections: Record<string, string[]>
  numbers: string
  display_numbers: string
  unit_amount: number
  multiple: number
  min_multiple?: number
  max_multiple?: number | null
  bet_count: number
}

export type BetCartItem = {
  id: string
  lottery_code: string
  lottery_name: string
  issue: string
  play_code: string
  play_name: string
  numbers: string
  display_numbers: string
  unit_amount: number
  multiple: number
  min_multiple?: number
  max_multiple?: number | null
  bet_count: number
}

export type DynamicBetCartItem = BetCartItem
