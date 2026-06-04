import { computed, ref } from 'vue'
import type { Router } from 'vue-router'
import http from '../api/http'
import { orderBetNumbers, orderDrawNumbers, orderNumber } from '../utils/lotteryFormat'

export function useBetOrders(router: Router) {
  const orders = ref<any[]>([])
  const selectedOrder = ref<any | null>(null)
  const loadingOrders = ref(false)
  const selectedOrderNumbers = computed(() => selectedOrder.value ? orderBetNumbers(selectedOrder.value) : [])
  const selectedDrawNumbers = computed(() => selectedOrder.value ? orderDrawNumbers(selectedOrder.value) : [])
  const selectedOrderNumber = computed(() => selectedOrder.value ? orderNumber(selectedOrder.value) : '')

  async function loadOrders() {
    loadingOrders.value = true
    try {
      const res = await http.get('/bet/orders')
      orders.value = Array.isArray(res.data) ? res.data : (res.data?.items || [])
    } catch {
      orders.value = []
    } finally {
      loadingOrders.value = false
    }
  }

  function openOrderDetail(order: any) {
    selectedOrder.value = order
  }

  function closeOrderDetail() {
    selectedOrder.value = null
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
    selectedOrderNumbers,
    selectedDrawNumbers,
    selectedOrderNumber,
    loadingOrders,
    loadOrders,
    openOrderDetail,
    closeOrderDetail,
    copyOrderNumber,
    rebetSelectedOrder,
  }
}
