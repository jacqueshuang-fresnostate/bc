import { computed, ref } from 'vue'
import type { Router } from 'vue-router'
import { fetchUserBetOrders } from '../api/bet'
import { fetchGroupBuyDetail } from '../features/group-buy/api'
import { orderDrawNumbers, orderNumber } from '../utils/lotteryFormat'

const ORDER_PAGE_SIZE = 20
export type BetOrderView = 'groupBuy' | 'orders'

export function useBetOrders(router: Router) {
  const activeOrderView = ref<BetOrderView>('orders')
  const ordersByView = ref<Record<BetOrderView, any[]>>({
    groupBuy: [],
    orders: [],
  })
  const selectedOrder = ref<any | null>(null)
  const selectedGroupBuyParticipants = ref<any[]>([])
  const loadingGroupBuyParticipants = ref(false)
  const loadingOrdersByView = ref<Record<BetOrderView, boolean>>({
    groupBuy: false,
    orders: false,
  })
  const ordersPageByView = ref<Record<BetOrderView, number>>({
    groupBuy: 0,
    orders: 0,
  })
  const hasMoreOrdersByView = ref<Record<BetOrderView, boolean>>({
    groupBuy: true,
    orders: true,
  })
  const orders = computed(() => ordersByView.value[activeOrderView.value])
  const ordersPage = computed(() => ordersPageByView.value[activeOrderView.value])
  const hasMoreOrders = computed(() => hasMoreOrdersByView.value[activeOrderView.value])
  const loadingOrders = computed(() => loadingOrdersByView.value[activeOrderView.value])
  const selectedDrawNumbers = computed(() => selectedOrder.value ? orderDrawNumbers(selectedOrder.value) : [])
  const selectedOrderNumber = computed(() => selectedOrder.value ? orderNumber(selectedOrder.value) : '')

  async function loadOrders(options: { append?: boolean; silent?: boolean; view?: BetOrderView } = {}) {
    const view = options.view || activeOrderView.value
    const append = Boolean(options.append)
    if (append && !hasMoreOrdersByView.value[view]) return
    if (loadingOrdersByView.value[view]) return
    if (!options.silent) {
      loadingOrdersByView.value = { ...loadingOrdersByView.value, [view]: true }
    }
    try {
      const nextPage = append ? ordersPageByView.value[view] + 1 : 1
      const nextOrders = await fetchUserBetOrders({
        page: nextPage,
        pageSize: ORDER_PAGE_SIZE,
        view,
      })
      const scopedOrders = filterOrdersByView(nextOrders, view)
      ordersByView.value = {
        ...ordersByView.value,
        [view]: append ? mergeOrders(ordersByView.value[view], scopedOrders) : scopedOrders,
      }
      ordersPageByView.value = {
        ...ordersPageByView.value,
        [view]: nextOrders.length > 0 ? nextPage : (append ? ordersPageByView.value[view] : 0),
      }
      hasMoreOrdersByView.value = {
        ...hasMoreOrdersByView.value,
        [view]: nextOrders.length >= ORDER_PAGE_SIZE,
      }
    } catch {
      if (!append) {
        ordersByView.value = { ...ordersByView.value, [view]: [] }
        ordersPageByView.value = { ...ordersPageByView.value, [view]: 0 }
        hasMoreOrdersByView.value = { ...hasMoreOrdersByView.value, [view]: true }
      }
    } finally {
      if (!options.silent) {
        loadingOrdersByView.value = { ...loadingOrdersByView.value, [view]: false }
      }
    }
  }

  function setOrderView(view: BetOrderView) {
    if (activeOrderView.value === view) return
    activeOrderView.value = view
    void loadOrders({ view })
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

  function filterOrdersByView(items: any[], view: BetOrderView) {
    return items.filter(order => view === 'groupBuy' ? isUnformedGroupBuyOrder(order) : !isUnformedGroupBuyOrder(order))
  }

  function isUnformedGroupBuyOrder(order: any) {
    const source = order?.orderSource || order?.order_source || order?.source_name || ''
    return Boolean(
      source === 'groupBuy'
        && (
          order?.groupBuyPendingPlan
          || order?.group_buy_pending_plan
          || order?.status === 'groupBuyPending'
          || String(order?.id || '').startsWith('GB-')
        ),
    )
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
    activeOrderView,
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
    setOrderView,
    openOrderDetail,
    closeOrderDetail,
    copyOrderNumber,
    rebetSelectedOrder,
  }
}
