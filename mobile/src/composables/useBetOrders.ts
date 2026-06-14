import { computed, ref } from 'vue'
import type { Router } from 'vue-router'
import { fetchUserBetOrders } from '../api/bet'
import { fetchGroupBuyDetail } from '../features/group-buy/api'
import { orderDrawNumbers, orderNumber } from '../utils/lotteryFormat'

export function useBetOrders(router: Router) {
  const orders = ref<any[]>([])
  const selectedOrder = ref<any | null>(null)
  const selectedGroupBuyParticipants = ref<any[]>([])
  const loadingOrders = ref(false)
  const loadingGroupBuyParticipants = ref(false)
  const selectedDrawNumbers = computed(() => selectedOrder.value ? orderDrawNumbers(selectedOrder.value) : [])
  const selectedOrderNumber = computed(() => selectedOrder.value ? orderNumber(selectedOrder.value) : '')

  async function loadOrders() {
    loadingOrders.value = true
    try {
      orders.value = await fetchUserBetOrders()
    } catch {
      orders.value = []
    } finally {
      loadingOrders.value = false
    }
  }

  async function openOrderDetail(order: any) {
    selectedOrder.value = order
    await loadSelectedGroupBuyParticipants(order)
  }

  function closeOrderDetail() {
    selectedOrder.value = null
    selectedGroupBuyParticipants.value = []
    loadingGroupBuyParticipants.value = false
  }

  async function loadSelectedGroupBuyParticipants(order: any) {
    selectedGroupBuyParticipants.value = []
    const planId = order?.group_buy_plan_id || order?.groupBuyPlanId
    if (!order?.is_group_buy || !planId) return
    loadingGroupBuyParticipants.value = true
    try {
      const result = await fetchGroupBuyDetail(String(planId))
      if (selectedOrder.value?.id !== order.id) return
      selectedGroupBuyParticipants.value = result.data.participants || []
    } catch {
      if (selectedOrder.value?.id === order.id) selectedGroupBuyParticipants.value = []
    } finally {
      if (selectedOrder.value?.id === order.id) loadingGroupBuyParticipants.value = false
    }
  }

  async function copyOrderNumber() {
    if (!selectedOrderNumber.value) return
    try {
      await navigator.clipboard?.writeText(selectedOrderNumber.value)
    } catch {}
  }

  function rebetSelectedOrder() {
    const code = selectedOrder.value?.lottery_code
    closeOrderDetail()
    if (code) router.push(`/bet/${code}`)
  }

  return {
    orders,
    selectedOrder,
    selectedGroupBuyParticipants,
    selectedDrawNumbers,
    selectedOrderNumber,
    loadingOrders,
    loadingGroupBuyParticipants,
    loadOrders,
    openOrderDetail,
    closeOrderDetail,
    copyOrderNumber,
    rebetSelectedOrder,
  }
}
