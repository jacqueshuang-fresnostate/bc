import { showToast } from 'vant'
import { createUserBetOrders, type CreateUserBetOrderPayload, type PlaySelection } from '../../../api/bet'
import type { BetCartItem, DynamicBetOptionGroup, PositionGridKind } from '../dynamic/types'

export type PlaySubmitMeta = {
  inputMode: string
  ruleCode: string
  positionGridKind: PositionGridKind
  optionGroups: DynamicBetOptionGroup[]
}

function parseDigitList(value: string) {
  return String(value || '')
    .split(/[,，\s]+/)
    .map(item => item.trim())
    .filter(Boolean)
    .map(item => Number(item))
    .filter(item => Number.isInteger(item) && item >= 0 && item <= 9)
}

function parsePositionSegments(value: string) {
  return String(value || '').split('|').map(parseDigitList)
}

function selectionFromOptionGroups(item: BetCartItem, meta: PlaySubmitMeta): PlaySelection {
  const segments = String(item.numbers || '').split('|')
  return {
    bigSmallOddEven: meta.optionGroups.map((group, index) => ({
      position: group.key,
      attributes: String(segments[index] || '')
        .split(/[,，\s]+/)
        .map(value => value.trim())
        .filter(Boolean),
    })).filter(pick => pick.attributes.length),
  }
}

function selectionFromCartItem(item: BetCartItem, meta: PlaySubmitMeta): PlaySelection {
  if (meta.optionGroups.length) return selectionFromOptionGroups(item, meta)

  const segments = parsePositionSegments(item.numbers)
  if (meta.positionGridKind === 'direct') return { positions: segments }
  if (meta.positionGridKind === 'direct_combination' || meta.positionGridKind === 'group3_compound' || meta.positionGridKind === 'group6_compound') {
    return { numbers: segments[0] || parseDigitList(item.numbers) }
  }
  if (meta.positionGridKind === 'group3_dantuo' || meta.positionGridKind === 'group6_dantuo') {
    return {
      bankerNumbers: segments[0] || [],
      dragNumbers: segments[1] || [],
    }
  }
  return { numbers: parseDigitList(item.numbers) }
}

// 投注批量提交边界：把前端篮子单据转换为后端用户下注接口需要的标准 selection 载荷。
export function buildUserBetOrders(cart: BetCartItem[], lotteryCode: string, issue: string, playMeta: Record<string, PlaySubmitMeta> = {}) {
  return cart.map<CreateUserBetOrderPayload>(item => {
    const meta = playMeta[item.play_code] || {
      inputMode: item.numbers.includes('|') ? 'position-grid' : 'text',
      ruleCode: item.play_code,
      positionGridKind: item.numbers.includes('|') ? 'direct' : 'direct_combination',
      optionGroups: [],
    }
    const unitAmountMinor = Math.round(Number(item.unit_amount || 0) * Math.max(Number(item.multiple || 1), 1) * 100)
    return {
      lotteryId: lotteryCode,
      issue,
      ruleCode: meta.ruleCode || item.play_code,
      selection: selectionFromCartItem(item, meta),
      unitAmountMinor,
    }
  })
}

export function useBetBatchSubmit() {
  async function submitBatch(lotteryCode: string, issue: string, cart: BetCartItem[], playMeta: Record<string, PlaySubmitMeta> = {}) {
    // 提交前先确认期号和篮子，避免把未绑定期号的投注送到后端。
    if (!issue) {
      showToast('当前期号未就绪')
      return null
    }
    if (!cart.length) {
      showToast('请先加入购彩篮')
      return null
    }
    if (cart.some(item => item.lottery_code !== lotteryCode)) {
      showToast('购彩篮只能提交同一个彩种的投注')
      return null
    }
    if (cart.some(item => item.issue !== issue)) {
      showToast('期号已变化，请清空购彩篮后重新选择')
      return null
    }
    const items = buildUserBetOrders(cart, lotteryCode, issue, playMeta)
    // 批量接口一次最多承载 50 条 compact 投注载荷。
    if (items.length > 50) {
      showToast('一次最多提交 50 笔投注，请减少选择')
      return null
    }
    // 仅在前端校验通过后调用接口；失败由调用页统一刷新余额和期号状态。
    const res = await createUserBetOrders(items)
    showToast(`已提交 ${items.length} 笔投注`)
    return res
  }

  return { submitBatch }
}
