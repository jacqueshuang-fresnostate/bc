import { showToast } from 'vant'
import http from '../../../api/http'
import type { BetCartItem } from '../dynamic/types'

// 投注批量提交边界：只把前端篮子单据转换为后端 /bet/place-batch 需要的载荷。
export function expandCartItems(cart: BetCartItem[], _playInputModes: Record<string, string> = {}) {
  return cart.map(item => ({
    play_code: item.play_code,
    numbers: item.numbers,
    amount: String(item.unit_amount * item.multiple * Math.max(item.bet_count || 1, 1)),
  }))
}

export function useBetBatchSubmit() {
  async function submitBatch(lotteryCode: string, issue: string, cart: BetCartItem[], playInputModes: Record<string, string> = {}) {
    // 提交前先确认期号和篮子，避免把未绑定期号的投注送到后端。
    if (!issue) {
      showToast('当前期号未就绪')
      return null
    }
    if (!cart.length) {
      showToast('请先加入篮子')
      return null
    }
    const items = expandCartItems(cart, playInputModes)
    // 批量接口一次最多承载 50 条 compact 投注载荷。
    if (items.length > 50) {
      showToast('一次最多提交 50 笔投注，请减少选择')
      return null
    }
    // 仅在前端校验通过后调用接口；失败由调用页统一刷新余额和期号状态。
    const res = await http.post('/bet/place-batch', {
      lottery_code: lotteryCode,
      issue,
      items,
    })
    showToast(`已提交 ${items.length} 笔投注`)
    return res.data
  }

  return { submitBatch }
}
