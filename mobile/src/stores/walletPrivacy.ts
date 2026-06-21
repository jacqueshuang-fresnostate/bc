import { defineStore } from 'pinia'
import { ref } from 'vue'

const WALLET_AMOUNT_HIDDEN_KEY = 'mobile_wallet_amount_hidden'

function readInitialHidden() {
  try {
    return localStorage.getItem(WALLET_AMOUNT_HIDDEN_KEY) === '1'
  } catch {
    return false
  }
}

function persistHidden(value: boolean) {
  try {
    localStorage.setItem(WALLET_AMOUNT_HIDDEN_KEY, value ? '1' : '0')
  } catch {
    // 隐私显示偏好属于本地 UI 状态，持久化失败时保留当前运行时状态即可。
  }
}

// 手机端钱包金额隐私开关：所有展示钱包余额的 Header 共享同一个本地偏好。
export const useWalletPrivacyStore = defineStore('walletPrivacy', () => {
  const hideWalletAmount = ref(readInitialHidden())

  function setWalletAmountHidden(value: boolean) {
    hideWalletAmount.value = value
    persistHidden(value)
  }

  function toggleWalletAmountHidden() {
    setWalletAmountHidden(!hideWalletAmount.value)
  }

  return {
    hideWalletAmount,
    setWalletAmountHidden,
    toggleWalletAmountHidden,
  }
})
