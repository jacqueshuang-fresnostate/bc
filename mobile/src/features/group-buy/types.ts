export type GroupBuyParticipation = {
  shares: number
  paid_shares: number
  reserved_shares: number
  amount: string
}

export type GroupBuyPlan = {
  id: number
  lottery_code: string
  lottery_name: string
  category?: string
  issue: string
  play_code: string
  play_name?: string
  title: string
  numbers: string
  total_amount: string
  share_count: number
  share_amount: string
  reserved_shares: number
  sold_shares: number
  available_shares: number
  progress_percent: number
  status: string
  created_at?: string
  updated_at?: string
  participant_count: number
  initiator_display: string
  my_participation?: GroupBuyParticipation | null
}

export type SelectOption = {
  label: string
  value: string
  icon?: string
}

export type GroupBuySettings = {
  min_share_amount: string
  initiator_min_buy_ratio: string
  share_amount: string
}

export type CreateGroupBuyForm = {
  lottery_code: string
  issue: string
  play_code: string
  title: string
  numbers: string
  total_amount: string
  share_count: number
  share_amount: string
  self_shares: number
}

export type CreateGroupBuyPayload = {
  lottery_code: string
  issue: string
  play_code: string
  title: string
  numbers: string
  total_amount: string
  share_count: number
  share_amount: string
  reserved_shares: number
  self_shares: number
}
