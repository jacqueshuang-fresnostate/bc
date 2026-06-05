<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { showToast } from 'vant'
import { useRouter } from 'vue-router'
import {
  createRechargeOrder,
  errorMessage,
  fetchRechargeConfig,
  fetchRechargeOrders,
  type RechargeChannel,
  type RechargeChannelConfig,
  type RechargeConfig,
  type RechargeOrder,
  type RechargeOrderStatus,
} from '../api/user'

const router = useRouter()
const amount = ref('')
const selectedChannel = ref<RechargeChannel | ''>('')
const selectedPayType = ref('')
const config = ref<RechargeConfig | null>(null)
const orders = ref<RechargeOrder[]>([])
const loadingConfig = ref(false)
const loadingOrders = ref(false)
const submitting = ref(false)
const showChannelPopup = ref(false)

const enabledChannels = computed(() => (config.value?.channels || []).filter(channel => channel.enabled))
const selectedChannelConfig = computed(() => enabledChannels.value.find(channel => channel.channel === selectedChannel.value))
const isRainbowEpay = computed(() => selectedChannel.value === 'rainbowEpay')
const isCustomerService = computed(() => selectedChannel.value === 'customerService')
const currentPayTypes = computed(() => payTypesForChannel(selectedChannelConfig.value))
const recentOrders = computed(() => orders.value.slice(0, 5))
const amountLimitText = computed(() => {
  const min = formatMinorAmount(config.value?.minAmountMinor || 0)
  const max = formatMinorAmount(config.value?.maxAmountMinor || 0)
  return `单笔限额 ¥${min} - ¥${max}`
})
const submitText = computed(() => (isCustomerService.value ? '发起客服直充' : '立即支付'))
const selectedChannelTitle = computed(() => selectedChannelConfig.value?.name || '选择充值方式')
const selectedChannelDescription = computed(() => selectedChannelConfig.value?.description || '请先选择可用充值方式')

watch(enabledChannels, (channels) => {
  if (!channels.some(channel => channel.channel === selectedChannel.value)) {
    selectedChannel.value = channels[0]?.channel || ''
  }
}, { immediate: true })

watch(currentPayTypes, (payTypes) => {
  if (!payTypes.includes(selectedPayType.value)) {
    selectedPayType.value = payTypes[0] || ''
  }
}, { immediate: true })

async function loadRechargeConfig() {
  loadingConfig.value = true
  try {
    config.value = await fetchRechargeConfig()
  } catch (error) {
    showToast(errorMessage(error, '加载充值配置失败'))
  } finally {
    loadingConfig.value = false
  }
}

async function loadRechargeOrders() {
  loadingOrders.value = true
  try {
    orders.value = await fetchRechargeOrders()
  } catch (error) {
    showToast(errorMessage(error, '加载充值记录失败'))
  } finally {
    loadingOrders.value = false
  }
}

onMounted(async () => {
  await Promise.all([loadRechargeConfig(), loadRechargeOrders()])
})

function selectChannel(channel: RechargeChannel) {
  selectedChannel.value = channel
  showChannelPopup.value = false
}

function payTypesForChannel(channel?: RechargeChannelConfig) {
  if (channel?.channel !== 'rainbowEpay') return []
  return channel.payTypes.length ? channel.payTypes : ['alipay']
}

function payTypeText(type?: string | null) {
  const labels: Record<string, string> = {
    alipay: '支付宝',
    wxpay: '微信',
    wechat: '微信',
    qqpay: 'QQ 钱包',
    bank: '银行卡',
  }
  return labels[String(type || '')] || String(type || '默认')
}

function channelText(channel: RechargeChannel) {
  return channel === 'rainbowEpay' ? '彩虹易支付' : '客服直充'
}

function statusText(status: RechargeOrderStatus) {
  const labels: Record<RechargeOrderStatus, string> = {
    pending: '待支付',
    waitingCustomerService: '等待客服确认',
    paid: '已入账',
    cancelled: '已取消',
  }
  return labels[status] || status
}

function statusClass(status: RechargeOrderStatus) {
  if (status === 'paid') return 'deposit-order-status--paid'
  if (status === 'cancelled') return 'deposit-order-status--cancelled'
  if (status === 'waitingCustomerService') return 'deposit-order-status--service'
  return 'deposit-order-status--pending'
}

function formatMinorAmount(value: number) {
  return (Number(value || 0) / 100).toFixed(2)
}

function amountMinorFromYuan(value: string) {
  const text = value.trim()
  if (!/^\d+(?:\.\d{0,2})?$/.test(text)) return null
  const [yuan, cent = ''] = text.split('.')
  const yuanMinor = Number(yuan) * 100
  const centMinor = Number(cent.padEnd(2, '0').slice(0, 2))
  const total = yuanMinor + centMinor
  return Number.isSafeInteger(total) ? total : null
}

function validateAmount() {
  const amountMinor = amountMinorFromYuan(amount.value)
  if (!amountMinor || amountMinor <= 0) {
    showToast('请输入正确的充值金额')
    return null
  }
  if (config.value && amountMinor < config.value.minAmountMinor) {
    showToast(`单笔最低充值 ¥${formatMinorAmount(config.value.minAmountMinor)}`)
    return null
  }
  if (config.value && amountMinor > config.value.maxAmountMinor) {
    showToast(`单笔最高充值 ¥${formatMinorAmount(config.value.maxAmountMinor)}`)
    return null
  }
  return amountMinor
}

function openPaymentUrl(paymentUrl: string) {
  const opened = window.open(paymentUrl, '_blank')
  if (!opened) window.location.href = paymentUrl
}

async function submitRecharge() {
  if (!selectedChannel.value) {
    showToast('请选择充值方式')
    return
  }

  const amountMinor = validateAmount()
  if (!amountMinor) return

  submitting.value = true
  try {
    const response = await createRechargeOrder({
      channel: selectedChannel.value,
      amountMinor,
      ...(isRainbowEpay.value && selectedPayType.value ? { payType: selectedPayType.value } : {}),
    })
    await loadRechargeOrders()

    if (isRainbowEpay.value) {
      if (response.paymentUrl) {
        openPaymentUrl(response.paymentUrl)
      } else {
        showToast(response.message || '支付订单已创建')
      }
      return
    }

    showToast(response.message || '客服直充申请已提交')
    const conversationId = response.supportConversationId || response.order.supportConversationId
    router.push({
      path: '/support',
      query: conversationId ? { conversationId } : undefined,
    })
  } catch (error) {
    showToast(errorMessage(error, '创建充值订单失败'))
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="deposit-page">
    <van-nav-bar title="充值" left-arrow @click-left="router.back()" />

    <main class="deposit-content">
      <div v-if="loadingConfig" class="deposit-state">
        <van-loading>正在加载充值配置...</van-loading>
      </div>

      <van-empty v-else-if="enabledChannels.length === 0" description="暂无可用充值方式" />

      <template v-else>
        <section class="deposit-method-card">
          <span class="deposit-section-label">充值方式</span>
          <button type="button" class="deposit-method-card__button" @click="showChannelPopup = true">
            <span>
              <strong>{{ selectedChannelTitle }}</strong>
              <small>{{ selectedChannelDescription }}</small>
            </span>
            <span class="deposit-method-card__chevron">›</span>
          </button>
        </section>

        <section class="deposit-form-card">
          <div class="deposit-form-card__header">
            <span class="deposit-section-label">充值金额</span>
            <small>{{ amountLimitText }}</small>
          </div>
          <van-field
            v-model="amount"
            type="number"
            input-align="right"
            label="金额（元）"
            placeholder="请输入充值金额"
          />

          <div v-if="isRainbowEpay" class="deposit-pay-types">
            <span class="deposit-section-label">支付渠道</span>
            <van-radio-group v-model="selectedPayType" direction="horizontal">
              <van-radio
                v-for="type in currentPayTypes"
                :key="type"
                :name="type"
              >
                {{ payTypeText(type) }}
              </van-radio>
            </van-radio-group>
          </div>

          <van-notice-bar
            v-if="isCustomerService"
            class="deposit-service-notice"
            :text="selectedChannelDescription"
            wrapable
          />

          <van-button
            type="primary"
            block
            round
            :loading="submitting"
            @click="submitRecharge"
          >
            {{ submitText }}
          </van-button>
        </section>

        <section class="deposit-orders">
          <div class="deposit-orders__header">
            <span class="deposit-section-label">最近充值</span>
            <button type="button" @click="loadRechargeOrders">刷新</button>
          </div>
          <div v-if="loadingOrders" class="deposit-state deposit-state--compact">
            <van-loading>正在加载...</van-loading>
          </div>
          <van-empty v-else-if="recentOrders.length === 0" description="暂无充值记录" />
          <div v-else class="deposit-order-list">
            <article v-for="order in recentOrders" :key="order.id" class="deposit-order-card">
              <div>
                <h3>{{ channelText(order.channel) }}</h3>
                <p>{{ order.id }} · {{ order.createdAt }}</p>
                <small v-if="order.payType">{{ payTypeText(order.payType) }}</small>
              </div>
              <div class="deposit-order-card__side">
                <strong>¥{{ formatMinorAmount(order.amountMinor) }}</strong>
                <span :class="['deposit-order-status', statusClass(order.status)]">
                  {{ statusText(order.status) }}
                </span>
              </div>
            </article>
          </div>
        </section>
      </template>
    </main>

    <van-popup v-model:show="showChannelPopup" position="bottom" round class="deposit-method-popup">
      <section class="deposit-method-sheet">
        <header class="deposit-method-sheet__header">
          <div>
            <p>充值方式</p>
            <h2>选择充值模式</h2>
          </div>
          <button type="button" aria-label="关闭充值方式选择" @click="showChannelPopup = false">×</button>
        </header>
        <div class="deposit-method-list">
          <button
            v-for="item in enabledChannels"
            :key="item.channel"
            type="button"
            class="deposit-method-option"
            :class="{ 'deposit-method-option--active': selectedChannel === item.channel }"
            @click="selectChannel(item.channel)"
          >
            <span>
              <strong>{{ item.name }}</strong>
              <small>{{ item.description }}</small>
            </span>
            <span class="deposit-method-option__mark">✓</span>
          </button>
        </div>
      </section>
    </van-popup>
  </div>
</template>

<style scoped>
.deposit-page {
  min-height: 100vh;
  background: linear-gradient(180deg, #fff8f5 0%, #f7eee9 100%);
  color: #241f1d;
}

.deposit-content {
  display: grid;
  gap: 14px;
  max-width: 540px;
  margin: 0 auto;
  padding: 16px 16px 28px;
}

.deposit-state {
  display: flex;
  min-height: 180px;
  align-items: center;
  justify-content: center;
  border-radius: 18px;
  background: rgba(255, 255, 255, 0.76);
}

.deposit-state--compact {
  min-height: 96px;
}

.deposit-section-label {
  display: block;
  color: #7c2d2d;
  font-size: 12px;
  font-weight: 900;
}

.deposit-method-card,
.deposit-form-card,
.deposit-orders {
  border: 1px solid rgba(175, 40, 41, 0.1);
  border-radius: 20px;
  background: rgba(255, 255, 255, 0.86);
  box-shadow: 0 10px 26px rgba(95, 10, 18, 0.07);
}

.deposit-method-card,
.deposit-form-card {
  padding: 14px;
}

.deposit-method-card__button,
.deposit-method-option {
  width: 100%;
  border: 1px solid rgba(175, 40, 41, 0.12);
  border-radius: 18px;
  background: #fff8f6;
  color: #7c2d2d;
  padding: 12px 14px;
  text-align: left;
}

.deposit-method-card__button {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  margin-top: 8px;
  box-shadow: 0 8px 22px rgba(175, 40, 41, 0.08);
}

.deposit-method-card__button strong,
.deposit-method-option strong,
.deposit-method-card__button small,
.deposit-method-option small {
  display: block;
}

.deposit-method-card__button strong,
.deposit-method-option strong {
  font-size: 15px;
  font-weight: 900;
  line-height: 1.25;
}

.deposit-method-card__button small,
.deposit-method-option small {
  margin-top: 4px;
  font-size: 11px;
  font-weight: 700;
  line-height: 1.35;
  opacity: 0.72;
}

.deposit-method-card__chevron {
  flex: 0 0 auto;
  color: #af2829;
  font-size: 24px;
  font-weight: 700;
  line-height: 1;
}

.deposit-form-card {
  display: grid;
  gap: 14px;
}

.deposit-form-card__header,
.deposit-orders__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.deposit-form-card__header small {
  color: #8e706d;
  font-size: 11px;
  font-weight: 700;
}

.deposit-pay-types {
  display: grid;
  gap: 10px;
}

.deposit-service-notice {
  border-radius: 14px;
}

.deposit-orders {
  padding: 14px;
}

.deposit-orders__header {
  margin-bottom: 12px;
}

.deposit-orders__header button {
  border: 0;
  color: #af2829;
  background: transparent;
  font-size: 12px;
  font-weight: 900;
}

.deposit-order-list {
  display: grid;
  gap: 10px;
}

.deposit-order-card {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  border-radius: 16px;
  background: #fff8f6;
  padding: 12px;
}

.deposit-order-card h3,
.deposit-order-card p {
  margin: 0;
}

.deposit-order-card h3 {
  color: #241f1d;
  font-size: 14px;
  font-weight: 900;
}

.deposit-order-card p,
.deposit-order-card small {
  color: #8e706d;
  font-size: 10px;
  font-weight: 700;
}

.deposit-order-card__side {
  display: grid;
  justify-items: end;
  gap: 6px;
  flex: 0 0 auto;
}

.deposit-order-card__side strong {
  color: #af2829;
  font-size: 14px;
}

.deposit-order-status {
  border-radius: 999px;
  padding: 3px 8px;
  font-size: 10px;
  font-weight: 900;
}

.deposit-order-status--pending {
  background: #fff7ed;
  color: #c2410c;
}

.deposit-order-status--service {
  background: #eef2ff;
  color: #4338ca;
}

.deposit-order-status--paid {
  background: #ecfdf5;
  color: #047857;
}

.deposit-order-status--cancelled {
  background: #f3f4f6;
  color: #6b7280;
}

.deposit-method-popup {
  overflow: hidden;
  background: transparent;
}

.deposit-method-sheet {
  border-radius: 28px 28px 0 0;
  background: #f9f9f9;
  padding: 22px 18px max(24px, env(safe-area-inset-bottom));
}

.deposit-method-sheet__header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.deposit-method-sheet__header p,
.deposit-method-sheet__header h2 {
  margin: 0;
}

.deposit-method-sheet__header p {
  margin-bottom: 4px;
  color: #5a403e;
  font-size: 12px;
  font-weight: 800;
}

.deposit-method-sheet__header h2 {
  color: #1a1c1c;
  font-size: 20px;
  font-weight: 900;
}

.deposit-method-sheet__header button {
  display: inline-flex;
  width: 32px;
  height: 32px;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 999px;
  color: #5a403e;
  background: #eeeeee;
  font-size: 22px;
  line-height: 1;
}

.deposit-method-list {
  display: grid;
  gap: 10px;
}

.deposit-method-option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  transition: transform 0.18s ease, border-color 0.18s ease, background 0.18s ease, box-shadow 0.18s ease;
}

.deposit-method-option:active {
  transform: scale(0.98);
}

.deposit-method-option--active {
  border-color: #af2829;
  background: #af2829;
  color: #fff7f4;
  box-shadow: 0 10px 24px rgba(175, 40, 41, 0.2);
}

.deposit-method-option__mark {
  flex: 0 0 auto;
  opacity: 0;
  font-size: 18px;
  font-weight: 900;
}

.deposit-method-option--active .deposit-method-option__mark {
  opacity: 1;
}
</style>
