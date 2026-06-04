<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { showToast } from 'vant'
import { useRouter } from 'vue-router'
import http from '../api/http'

const router = useRouter()
const method = ref('fiat')
const amount = ref('')
const usdtAmount = ref('')
const fiatLoading = ref(false)
const fiatPayType = ref<'alipay' | 'wechat'>('alipay')
const usdtLoading = ref(false)
const showPaymentPopup = ref(false)
const paymentMethods = ref({ fiat: true, usdt: true })
const enabledPaymentTabs = computed(() => [
  { name: 'fiat', title: '支付宝 / 微信', subtitle: '快捷充值', enabled: paymentMethods.value.fiat },
  { name: 'usdt', title: 'USDT TRC20', subtitle: '链上充值', enabled: paymentMethods.value.usdt },
].filter(item => item.enabled))

const selectedPaymentMethod = computed(() => enabledPaymentTabs.value.find(item => item.name === method.value))

watch(enabledPaymentTabs, (tabs) => {
  if (!tabs.some(item => item.name === method.value)) method.value = tabs[0]?.name || ''
}, { immediate: true })

function selectPaymentMethod(name: string) {
  method.value = name
  showPaymentPopup.value = false
}

async function loadPaymentMethods() {
  try {
    const res = await http.get('/payment/methods')
    paymentMethods.value = { ...paymentMethods.value, ...(res.data?.methods || res.data) }
  } catch {}
}

onMounted(loadPaymentMethods)

function openPaymentUrl(paymentUrl: string) {
  const opened = window.open(paymentUrl, '_blank')
  if (!opened) window.location.href = paymentUrl
}

async function payWithFiat() {
  const val = parseFloat(amount.value)
  if (!val || val <= 0) {
    showToast('请输入充值金额')
    return
  }
  fiatLoading.value = true
  try {
    const res = await http.post('/payment/fiat/create-order', { amount: val, pay_type: fiatPayType.value })
    const paymentUrl = res.data.payment_url
    if (paymentUrl) {
      openPaymentUrl(paymentUrl)
    } else {
      showToast('订单已创建，请等待支付链接')
    }
  } catch (e: any) {
    showToast(e.response?.data?.detail || '支付下单失败')
  } finally {
    fiatLoading.value = false
  }
}

async function payWithUsdt() {
  const val = parseFloat(usdtAmount.value)
  if (!val || val <= 0) {
    showToast('请输入充值金额')
    return
  }
  usdtLoading.value = true
  try {
    const res = await http.post('/payment/usdt/create-order', { amount: val })
    const paymentUrl = res.data.payment_url
    if (paymentUrl) {
      openPaymentUrl(paymentUrl)
    } else {
      showToast('订单已创建，请在钱包中完成转账')
    }
  } catch (e: any) {
    showToast(e.response?.data?.detail || 'USDT 下单失败')
  } finally {
    usdtLoading.value = false
  }
}
</script>

<template>
  <div>
    <van-nav-bar title="充值" left-arrow @click-left="router.back()" />

    <van-empty v-if="enabledPaymentTabs.length === 0" description="暂无可用支付方式" />

    <template v-else>
      <section class="deposit-method-card">
        <span class="deposit-method-card__label">充值方式</span>
        <button type="button" class="deposit-method-card__button" @click="showPaymentPopup = true">
          <span>
            <strong>{{ selectedPaymentMethod?.title || '选择充值方式' }}</strong>
            <small>{{ selectedPaymentMethod?.subtitle || '请选择可用支付方式' }}</small>
          </span>
          <span class="deposit-method-card__chevron">›</span>
        </button>
      </section>

      <div v-if="method === 'fiat'" style="padding: 16px;">
        <van-cell-group inset>
          <van-field v-model="amount" type="digit" label="金额（元）" placeholder="请输入充值金额" />
          <van-field label="支付渠道">
            <template #input>
              <van-radio-group v-model="fiatPayType" direction="horizontal">
                <van-radio name="alipay">支付宝</van-radio>
                <van-radio name="wechat">微信</van-radio>
              </van-radio-group>
            </template>
          </van-field>
        </van-cell-group>
        <div style="padding: 16px;">
          <van-button type="primary" block :loading="fiatLoading" @click="payWithFiat">立即支付</van-button>
        </div>
      </div>

      <div v-if="method === 'usdt'" style="padding: 16px;">
        <van-notice-bar text="输入充值金额后，系统会自动生成 USDT-TRC20 收款订单" />
        <van-cell-group inset style="margin-top: 12px;">
          <van-field v-model="usdtAmount" type="number" label="金额（元）" placeholder="请输入充值金额" />
        </van-cell-group>
        <div style="padding: 16px;">
          <van-button type="primary" block :loading="usdtLoading" @click="payWithUsdt">创建 USDT 订单</van-button>
        </div>
      </div>

      <van-popup v-model:show="showPaymentPopup" position="bottom" round class="deposit-method-popup">
        <section class="deposit-method-sheet">
          <header class="deposit-method-sheet__header">
            <div>
              <p>充值方式</p>
              <h2>选择支付通道</h2>
            </div>
            <button type="button" aria-label="关闭充值方式选择" @click="showPaymentPopup = false">×</button>
          </header>
          <div class="deposit-method-list">
            <button
              v-for="item in enabledPaymentTabs"
              :key="item.name"
              type="button"
              class="deposit-method-option"
              :class="{ 'deposit-method-option--active': method === item.name }"
              @click="selectPaymentMethod(item.name)"
            >
              <span>
                <strong>{{ item.title }}</strong>
                <small>{{ item.subtitle }}</small>
              </span>
              <span class="deposit-method-option__mark">✓</span>
            </button>
          </div>
        </section>
      </van-popup>
    </template>
  </div>
</template>

<style scoped>
.deposit-method-card {
  padding: 14px 16px 2px;
}

.deposit-method-card__label {
  display: block;
  margin-bottom: 8px;
  color: #7c2d2d;
  font-size: 12px;
  font-weight: 800;
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
  line-height: 1.2;
  opacity: 0.72;
}

.deposit-method-card__chevron {
  color: #af2829;
  font-size: 24px;
  font-weight: 700;
  line-height: 1;
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
  opacity: 0;
  font-size: 18px;
  font-weight: 900;
}

.deposit-method-option--active .deposit-method-option__mark {
  opacity: 1;
}
</style>
