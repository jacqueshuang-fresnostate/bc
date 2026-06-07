import { computed, ref, type ComputedRef, type Ref } from 'vue'
import { showToast } from 'vant'
import http from '../api/http'
import { errorMessage } from '../utils/errorMessage'

export type PlayItem = {
  code: string
  name: string
  full_name: string
  rule_code: string
  input_mode: string
  option_value: string | null
  odds: string
  simple_description: string
  detail_description: string
  example_description: string
  min_select_count: number
  bet_number_count: number
}

export type Fc3dPosition = 'hundreds' | 'tens' | 'units'

type BetCartItem = {
  play: string
  play_name: string
  numbers: string
  display_numbers: string
  amount: number
}

type UseBetCartOptions = {
  issue: Ref<string>
  lotteryCode: ComputedRef<string>
  selectedPlay: Ref<string>
  selectedPlayItem: ComputedRef<PlayItem | null>
  loadBalance: () => Promise<void>
  loadCurrentRound: (silent?: boolean) => Promise<void>
}

export function useBetCart(options: UseBetCartOptions) {
  const numbers = ref('')
  const amount = ref('10')
  const cart = ref<BetCartItem[]>([])
  const fc3dSelectedNumbers = ref<Record<Fc3dPosition, string>>({ hundreds: '', tens: '', units: '' })

  const fc3dBetNumbers = computed(() => [fc3dSelectedNumbers.value.hundreds, fc3dSelectedNumbers.value.tens, fc3dSelectedNumbers.value.units].join(','))
  const fc3dSelectedCount = computed(() => Object.values(fc3dSelectedNumbers.value).every(Boolean) ? 1 : 0)
  const fc3dTotalAmount = computed(() => {
    const total = fc3dSelectedCount.value * Number(amount.value || 0)
    return Number.isFinite(total) ? total.toFixed(2) : '0.00'
  })
  const genericSelectedCount = computed(() => cart.value.length + (numbers.value ? 1 : 0))
  const genericTotalAmount = computed(() => {
    const cartTotal = cart.value.reduce((sum, item) => sum + Number(item.amount || 0), 0)
    const draftAmount = numbers.value ? Number(amount.value || 0) : 0
    const total = cartTotal + draftAmount
    return Number.isFinite(total) ? total : 0
  })

  function selectedNumberCount(value: string) {
    const text = String(value || '').trim()
    if (!text) return 0
    const normalized = text.replace(/，/g, ',')
    if (normalized.includes(',')) return normalized.split(',').filter(part => part.trim()).length
    return 1
  }

  function validateNumberCountBeforeCart(playItem: PlayItem) {
    if (playItem.input_mode === 'fixed-option') return true
    const minCount = Math.max(1, Number(playItem.min_select_count || 1))
    const exactCount = Math.max(1, Number(playItem.bet_number_count || 1))
    if (minCount <= 1 && exactCount <= 1) return true
    const count = selectedNumberCount(numbers.value)
    if (count < minCount) {
      showToast(`至少选择 ${minCount} 个号码`)
      return false
    }
    if (exactCount > 1 && count !== exactCount) {
      showToast(`需要选择 ${exactCount} 个号码`)
      return false
    }
    return true
  }

  function validateFc3dBetNumbers(playItem: PlayItem, value: string) {
    if (playItem.input_mode === 'fixed-option') return true
    const minCount = Math.max(1, Number(playItem.min_select_count || 1))
    const exactCount = Math.max(1, Number(playItem.bet_number_count || 1))
    const count = selectedNumberCount(value)
    if (count < minCount) {
      showToast(`至少选择 ${minCount} 个号码`)
      return false
    }
    if (exactCount > 1 && count !== exactCount) {
      showToast(`需要选择 ${exactCount} 个号码`)
      return false
    }
    return true
  }

  function toggleFc3dDigit(position: Fc3dPosition, digit: string) {
    fc3dSelectedNumbers.value[position] = fc3dSelectedNumbers.value[position] === digit ? '' : digit
  }

  function clearFc3dSelection() {
    fc3dSelectedNumbers.value = { hundreds: '', tens: '', units: '' }
  }

  async function submitFc3dBet() {
    if (!options.issue.value) {
      showToast('当前期号未就绪')
      return
    }
    if (!options.selectedPlay.value || !options.selectedPlayItem.value) {
      showToast('请选择玩法')
      return
    }
    if (fc3dSelectedCount.value !== 1) {
      showToast('请选择百位、十位、个位')
      return
    }
    if (!validateFc3dBetNumbers(options.selectedPlayItem.value, fc3dBetNumbers.value)) return
    try {
      await http.post('/bet/place', {
        lottery_code: options.lotteryCode.value,
        issue: options.issue.value,
        play_code: options.selectedPlay.value,
        numbers: fc3dBetNumbers.value,
        amount: Number(amount.value),
      })
      showToast('投注成功')
      clearFc3dSelection()
      await Promise.all([options.loadBalance(), options.loadCurrentRound(true)])
    } catch (e) {
      showToast(errorMessage(e, '投注失败'))
      await options.loadCurrentRound(true)
    }
  }

  function addToCart() {
    if (!options.issue.value) {
      showToast('当前期号未就绪')
      return
    }
    if (!numbers.value || !options.selectedPlay.value) {
      showToast('请选择号码')
      return
    }
    const currentPlayItem = options.selectedPlayItem.value
    if (currentPlayItem && !validateNumberCountBeforeCart(currentPlayItem)) return
    cart.value.push({
      play: options.selectedPlay.value,
      play_name: currentPlayItem?.name || options.selectedPlay.value,
      numbers: numbers.value,
      display_numbers: currentPlayItem?.input_mode === 'fixed-option'
        ? (currentPlayItem?.name || numbers.value)
        : numbers.value,
      amount: Number(amount.value),
    })
    numbers.value = currentPlayItem?.input_mode === 'fixed-option' ? (currentPlayItem.option_value || '') : ''
    showToast('已加入投注篮')
  }

  async function submitCart() {
    if (!options.issue.value) {
      showToast('当前期号未就绪')
      return
    }
    for (const item of cart.value) {
      try {
        await http.post('/bet/place', {
          lottery_code: options.lotteryCode.value,
          issue: options.issue.value,
          play_code: item.play,
          numbers: item.numbers,
          amount: item.amount,
        })
      } catch (e) {
        showToast(errorMessage(e, '投注失败'))
        await options.loadCurrentRound(true)
        return
      }
    }
    showToast(`已提交 ${cart.value.length} 笔投注`)
    cart.value = []
    await options.loadCurrentRound(true)
  }

  function removeFromCart(i: number) {
    cart.value.splice(i, 1)
  }

  function resetCart() {
    cart.value = []
    numbers.value = ''
    clearFc3dSelection()
  }

  return {
    numbers,
    amount,
    cart,
    fc3dSelectedNumbers,
    fc3dBetNumbers,
    fc3dSelectedCount,
    fc3dTotalAmount,
    genericSelectedCount,
    genericTotalAmount,
    selectedNumberCount,
    validateNumberCountBeforeCart,
    validateFc3dBetNumbers,
    toggleFc3dDigit,
    clearFc3dSelection,
    submitFc3dBet,
    addToCart,
    submitCart,
    removeFromCart,
    resetCart,
  }
}
