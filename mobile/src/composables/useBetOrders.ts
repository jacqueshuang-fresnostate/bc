import { computed, ref } from 'vue'
import type { Router } from 'vue-router'
import { fetchUserBetOrders } from '../api/bet'
import { fetchGroupBuyDetail } from '../features/group-buy/api'
import { orderDrawNumbers, orderNumber } from '../utils/lotteryFormat'

const ORDER_PAGE_SIZE = 20

export function useBetOrders(router: Router) {
  const orders = ref<any[]>([])
  const selectedOrder = ref<any | null>(null)
  const selectedGroupBuyParticipants = ref<any[]>([])
  const loadingOrders = ref(false)
  const loadingGroupBuyParticipants = ref(false)
  const ordersPage = ref(0)
  const hasMoreOrders = ref(true)
  const selectedDrawNumbers = computed(() => selectedOrder.value ? orderDrawNumbers(selectedOrder.value) : [])
  const selectedOrderNumber = computed(() => selectedOrder.value ? orderNumber(selectedOrder.value) : '')

  async function loadOrders(options: { append?: boolean; silent?: boolean } = {}) {
    const append = Boolean(options.append)
    if (append && !hasMoreOrders.value) return
    loadingOrders.value = true
    try {
      const nextPage = append ? ordersPage.value + 1 : 1
      const nextOrders = await fetchUserBetOrders({ page: nextPage, pageSize: ORDER_PAGE_SIZE })
      orders.value = append ? mergeOrders(orders.value, nextOrders) : nextOrders
      ordersPage.value = nextOrders.length > 0 ? nextPage : (append ? ordersPage.value : 0)
      hasMoreOrders.value = nextOrders.length >= ORDER_PAGE_SIZE
    } catch {
      if (!append) {
        orders.value = []
        ordersPage.value = 0
        hasMoreOrders.value = true
      }
    } finally {
      loadingOrders.value = false
    }
  }

  function mergeOrders(current: any[], incoming: any[]) {
    const seen = new Set<string>()
    return [...current, ...incoming].filter(order => {
      const id = String(order?.id || '')
      if (!id || seen.has(id)) return false
      seen.add(id)
      return true
    })
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
    ordersPage,
    hasMoreOrders,
    loadOrders,
    openOrderDetail,
    closeOrderDetail,
    copyOrderNumber,
    rebetSelectedOrder,
  }
}
