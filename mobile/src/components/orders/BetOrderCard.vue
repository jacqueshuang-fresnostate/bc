<script setup lang="ts">
import { computed } from 'vue'
import { formatDateTime, moneyText, orderAmountLabel, orderBetContentText, orderBetCount, orderDisplayAmount, orderMultiple, orderResultLabel, orderResultText, orderSourceText, orderStatusIcon, orderTagText, orderTone, orderUnitAmount, statusText } from '../../utils/lotteryFormat'

const props = defineProps<{ order: any }>()
const emit = defineEmits<{ open: [any] }>()
const orderNumbersText = computed(() => orderBetContentText(props.order))
const orderCreatedAtText = computed(() => formatDateTime(props.order?.created_at || props.order?.createdAt))
</script>

<template>
  <article
    class="order-card bet-record-card"
    :class="[`order-card--${orderTone(order.status)}`, `bet-record-card--${orderTone(order.status)}`]"
    role="button"
    tabindex="0"
    @click="emit('open', order)"
    @keydown.enter="emit('open', order)"
  >
    <div class="bet-record-card__line" aria-hidden="true"></div>
    <div class="bet-record-card__head">
      <div class="bet-record-card__title-block">
        <h2>
          {{ order.lottery_name || order.lottery_code }}
          <span class="bet-record-card__tag">{{ orderTagText(order) }}</span>
          <span class="bet-record-card__source">{{ orderSourceText(order) }}</span>
        </h2>
        <div class="bet-record-card__meta">
          <p>第 {{ order.issue }} 期</p>
          <p>下注时间 {{ orderCreatedAtText }}</p>
        </div>
      </div>
      <span class="status-pill" :class="`status-pill--${orderTone(order.status)}`">
        <span class="status-pill__icon">{{ orderStatusIcon(order.status) }}</span>
        {{ statusText(order.status) }}
      </span>
    </div>

    <div class="bet-record-card__details">
      <p>投注号码</p>
      <strong>{{ orderNumbersText }}</strong>
    </div>

    <div class="bet-record-card__grid">
      <div>
        <p>单注金额</p>
        <strong>{{ moneyText(orderUnitAmount(order)) }}</strong>
      </div>
      <div>
        <p>注数</p>
        <strong>{{ orderBetCount(order) }} 注</strong>
      </div>
      <div>
        <p>倍数</p>
        <strong>{{ orderMultiple(order) }} 倍</strong>
      </div>
      <div>
        <p>{{ orderAmountLabel(order) }}</p>
        <strong>{{ moneyText(orderDisplayAmount(order)) }}</strong>
      </div>
      <div>
        <p>赔率</p>
        <strong>{{ order.odds ? `1 : ${order.odds}` : '-' }}</strong>
      </div>
    </div>

    <div class="bet-record-card__result">
      <span>{{ orderResultLabel(order) }}</span>
      <strong :class="{ 'is-prize': orderTone(order.status) === 'won' }">{{ orderResultText(order) }}</strong>
    </div>
  </article>
</template>

<style scoped>
.order-card {
  border: 0;
}

.bet-record-card {
  position: relative;
  display: block;
  overflow: hidden;
  border-radius: 12px;
  padding: 20px;
  background: #fff;
  box-shadow: 0 4px 18px rgba(26, 28, 28, 0.035);
  cursor: pointer;
  transition: box-shadow 0.28s ease, transform 0.28s ease, opacity 0.28s ease, filter 0.28s ease;
}

.bet-record-card:hover {
  box-shadow: 0 8px 30px rgba(140, 10, 21, 0.06);
}

.bet-record-card:active {
  transform: scale(0.985);
}

.bet-record-card:focus-visible {
  outline: 2px solid #ffb3ad;
  outline-offset: 3px;
}

.bet-record-card--lost {
  opacity: 0.76;
  filter: grayscale(0.18);
}

.bet-record-card__line {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 3px;
  background: linear-gradient(90deg, #8c0a15, #af2829);
  opacity: 0.2;
  transition: opacity 0.25s ease;
}

.bet-record-card--pending .bet-record-card__line {
  background: #e9c349;
}

.bet-record-card--lost .bet-record-card__line {
  background: #dadada;
}

.bet-record-card:hover .bet-record-card__line {
  opacity: 1;
}

.bet-record-card__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 18px;
}

.bet-record-card__title-block {
  min-width: 0;
}

.bet-record-card__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 10px;
}

.bet-record-card h2 {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin: 0 0 4px;
  color: #1a1c1c;
  font-family: "Plus Jakarta Sans", Avenir Next, ui-sans-serif, system-ui, sans-serif;
  font-size: 18px;
  font-weight: 900;
  letter-spacing: -0.035em;
}

.bet-record-card p {
  margin: 0;
  color: rgba(90, 64, 62, 0.78);
  font-size: 12px;
  font-weight: 700;
}

.bet-record-card__tag,
.bet-record-card__source {
  display: inline-flex;
  overflow: hidden;
  border-radius: 4px;
  padding: 2px 8px;
  font-family: Inter, ui-sans-serif, system-ui, sans-serif;
  font-size: 10px;
  font-weight: 900;
  letter-spacing: 0.08em;
  text-overflow: ellipsis;
  text-transform: uppercase;
  white-space: nowrap;
}

.bet-record-card__tag {
  max-width: 112px;
  color: #8c0a15;
  background: rgba(140, 10, 21, 0.1);
}

.bet-record-card__source {
  color: #735c00;
  background: #fff4dc;
}

.status-pill {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 5px;
  border-radius: 999px;
  padding: 6px 10px;
  font-size: 13px;
  font-weight: 900;
  white-space: nowrap;
}

.status-pill__icon {
  display: inline-flex;
  width: 16px;
  height: 16px;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  line-height: 1;
}

.status-pill--won {
  color: #8c0a15;
  background: rgba(140, 10, 21, 0.05);
}

.status-pill--pending {
  color: #745c00;
  background: rgba(254, 214, 91, 0.28);
}

.status-pill--lost {
  color: #5a403e;
  background: #dadada;
}

.bet-record-card__details {
  margin-bottom: 14px;
  border-radius: 10px;
  padding: 12px;
  background: #fff8f0;
}

.bet-record-card__details strong {
  display: block;
  margin-top: 5px;
  overflow-wrap: anywhere;
  color: #8c0a15;
  font-family: "Plus Jakarta Sans", Avenir Next, ui-sans-serif, system-ui, sans-serif;
  font-size: 16px;
  font-weight: 900;
  letter-spacing: 0.04em;
}

.bet-record-card__grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 12px;
  margin-bottom: 16px;
}

.bet-record-card__details p,
.bet-record-card__grid p,
.bet-record-card__result span {
  margin: 0 0 5px;
  color: #5a403e;
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.08em;
}

.bet-record-card__grid strong {
  color: #1a1c1c;
  font-size: 14px;
  font-weight: 700;
}

.bet-record-card__result {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  border-radius: 10px;
  padding: 12px;
  background: #f3f3f3;
}

.bet-record-card__result span {
  margin: 0;
}

.bet-record-card__result strong {
  color: #5a403e;
  font-family: "Plus Jakarta Sans", Avenir Next, ui-sans-serif, system-ui, sans-serif;
  font-size: 15px;
  font-weight: 800;
  letter-spacing: -0.03em;
  white-space: nowrap;
}

.bet-record-card__result strong.is-prize {
  color: #8c0a15;
  font-size: 20px;
  font-weight: 900;
}

@media (max-width: 420px) {
  .bet-record-card__head {
    flex-direction: column;
  }

  .bet-record-card__grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .status-pill {
    align-self: flex-start;
  }
}
</style>
