<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { detailHeroAmount, detailHeroNote, formatDateTime, moneyText, orderAmountLabel, orderAmountText, orderBetContentGroups, orderBetCount, orderDisplayAmount, orderMatchItems, orderMatchedBetKeys, orderMultiple, orderSourceText, orderStatusIcon, orderTone, orderUnitAmount, statusText } from '../../utils/lotteryFormat'

const props = defineProps<{
  selectedOrder: any
  selectedOrderNumbers: string[]
  selectedDrawNumbers: string[]
  selectedOrderNumber: string
}>()

const emit = defineEmits<{ close: []; copy: []; rebet: [] }>()
const ORDER_DETAIL_BATCH_SIZE = 10
const visibleOrderNumberGroupCount = ref(ORDER_DETAIL_BATCH_SIZE)

const selectedOrderBetGroups = computed(() => orderBetContentGroups(props.selectedOrder, props.selectedOrderNumbers))
const selectedAttributeGroups = computed(() => selectedOrderBetGroups.value.filter(group => group.kind === 'attributes' && group.values.length))
const selectedOrderNumberGroups = computed(() => selectedOrderBetGroups.value.filter(group => group.kind === 'numbers' && group.values.length))
const visibleOrderNumberGroups = computed(() => selectedOrderNumberGroups.value.slice(0, visibleOrderNumberGroupCount.value))
const hasMoreOrderNumberGroups = computed(() => visibleOrderNumberGroupCount.value < selectedOrderNumberGroups.value.length)
const selectedOrderMatchItems = computed(() => orderMatchItems(props.selectedOrder, props.selectedDrawNumbers))
const matchedBetKeys = computed(() => new Set(orderMatchedBetKeys(props.selectedOrder)))

function showMoreOrderNumberGroups() {
  visibleOrderNumberGroupCount.value += ORDER_DETAIL_BATCH_SIZE
}

function isMatchedOrderNumberGroup(group: { key: string }) {
  return matchedBetKeys.value.has(group.key)
}

function isMatchedAttributeValue(value: { key: string }) {
  return matchedBetKeys.value.has(value.key)
}

watch(() => props.selectedOrder?.id, () => {
  visibleOrderNumberGroupCount.value = ORDER_DETAIL_BATCH_SIZE
})
</script>

<template>
  <div class="order-detail-overlay" role="presentation" @click.self="emit('close')">
    <section class="order-detail-sheet" role="dialog" aria-modal="true" aria-label="注单详情">
      <header class="order-detail-sheet__header">
        <div class="detail-header-spacer" aria-hidden="true"></div>
        <h2>注单详情</h2>
        <button class="detail-close" type="button" aria-label="关闭注单详情" @click="emit('close')">×</button>
      </header>

      <div class="order-detail-sheet__body">
        <section class="detail-hero" :class="`detail-hero--${orderTone(selectedOrder.status)}`">
          <div class="detail-status-pill" :class="`detail-status-pill--${orderTone(selectedOrder.status)}`">
            <span>{{ orderStatusIcon(selectedOrder.status) }}</span>
            {{ statusText(selectedOrder.status) }}
          </div>
          <div class="detail-prize" :class="{ 'detail-prize--text': orderTone(selectedOrder.status) !== 'won' }">
            {{ detailHeroAmount(selectedOrder) }}
          </div>
          <p>{{ detailHeroNote(selectedOrder) }}</p>
        </section>

        <section class="detail-lottery-card">
          <div class="detail-lottery-card__icon">彩</div>
          <div class="detail-lottery-card__main">
            <h3>{{ selectedOrder.lottery_name || selectedOrder.lottery_code }}</h3>
            <p>第 <strong>{{ selectedOrder.issue }}</strong> 期</p>
          </div>
          <div class="detail-lottery-card__amount">
            <strong>{{ moneyText(orderDisplayAmount(selectedOrder)) }}</strong>
            <span>{{ orderAmountLabel(selectedOrder) }}</span>
          </div>
        </section>

        <section class="detail-panel">
          <h3><span></span>投注内容</h3>
          <div v-if="selectedAttributeGroups.length" class="detail-attribute-groups">
            <article
              v-for="group in selectedAttributeGroups"
              :key="`bet-attribute-${selectedOrder.id}-${group.key}`"
              class="detail-attribute-group"
            >
              <span class="detail-attribute-group__label">{{ group.label }}</span>
              <div class="detail-attribute-group__values">
                <span
                  v-for="value in group.values"
                  :key="`bet-attribute-${selectedOrder.id}-${group.key}-${value.key}`"
                  class="detail-attribute-pill"
                  :class="{ 'detail-attribute-pill--matched': isMatchedAttributeValue(value) }"
                >
                  {{ value.label }}
                </span>
              </div>
            </article>
          </div>
          <div v-else-if="selectedOrderNumberGroups.length" class="detail-bet-groups">
            <div
              v-for="(group, groupIndex) in visibleOrderNumberGroups"
              :key="`bet-group-${selectedOrder.id}-${groupIndex}-${group.key}`"
              class="detail-number-balls detail-number-balls--bet"
              :class="{ 'detail-number-balls--matched': isMatchedOrderNumberGroup(group) }"
            >
              <span
                v-for="value in group.values"
                :key="`bet-${selectedOrder.id}-${group.key}-${value.key}`"
                class="detail-number-ball detail-number-ball--bet"
              >
                {{ value.label }}
              </span>
            </div>
            <button v-if="hasMoreOrderNumberGroups" class="detail-show-more" type="button" @click="showMoreOrderNumberGroups">
              查看更多
            </button>
          </div>
          <div v-else class="detail-empty-value">暂无投注号码</div>

          <div class="detail-grid">
            <div>
              <span>玩法名称</span>
              <strong>{{ selectedOrder.play_name || selectedOrder.play_code || '-' }}</strong>
            </div>
            <div>
              <span>注单类型</span>
              <strong>{{ orderSourceText(selectedOrder) }}</strong>
            </div>
            <div>
              <span>赔率</span>
              <strong>{{ selectedOrder.odds ? `1 : ${selectedOrder.odds}` : '-' }}</strong>
            </div>
            <div>
              <span>单注金额</span>
              <strong>{{ moneyText(orderUnitAmount(selectedOrder)) }}</strong>
            </div>
            <div>
              <span>注数</span>
              <strong>{{ orderBetCount(selectedOrder) }} 注</strong>
            </div>
            <div>
              <span>倍数</span>
              <strong>{{ orderMultiple(selectedOrder) }} 倍</strong>
            </div>
            <div>
              <span>{{ orderAmountLabel(selectedOrder) }}</span>
              <strong>{{ moneyText(orderDisplayAmount(selectedOrder)) }}</strong>
            </div>
            <div>
              <span>结算金额</span>
              <strong>{{ orderAmountText(selectedOrder) }}</strong>
            </div>
          </div>
        </section>

        <section class="detail-panel detail-panel--draw">
          <h3><span></span>开奖号码</h3>
          <div v-if="selectedDrawNumbers.length" class="detail-number-balls detail-number-balls--draw">
            <span
              v-for="(number, index) in selectedDrawNumbers"
              :key="`draw-${selectedOrder.id}-${index}-${number}`"
              class="detail-number-ball detail-number-ball--draw"
            >
              {{ number }}
            </span>
          </div>
          <div v-else class="detail-empty-value">{{ selectedOrder.status === 'pending' ? '待开奖' : '暂无开奖数据' }}</div>
        </section>

        <section class="detail-panel detail-panel--match">
          <h3><span></span>匹配项</h3>
          <div class="detail-match-list">
            <article
              v-for="(item, index) in selectedOrderMatchItems"
              :key="`match-${selectedOrder.id}-${index}-${item.label}-${item.value}`"
              class="detail-match-item"
              :class="`detail-match-item--${item.tone}`"
            >
              <span>{{ item.label }}</span>
              <strong>{{ item.value }}</strong>
              <p>{{ item.detail }}</p>
            </article>
          </div>
        </section>

        <section class="detail-meta-card">
          <div>
            <span>订单编号</span>
            <strong class="detail-order-number">{{ selectedOrderNumber }}</strong>
            <button type="button" aria-label="复制订单编号" @click="emit('copy')">复制</button>
          </div>
          <div>
            <span>投注时间</span>
            <strong>{{ formatDateTime(selectedOrder.created_at) }}</strong>
          </div>
        </section>
      </div>

      <footer class="order-detail-sheet__footer">
        <button type="button" @click="emit('rebet')">
          再来一注
          <span>→</span>
        </button>
      </footer>
    </section>
  </div>
</template>

<style scoped>
.order-detail-overlay {
  position: fixed;
  inset: 0;
  z-index: 80;
  display: flex;
  align-items: flex-end;
  justify-content: center;
  background: rgba(26, 28, 28, 0.6);
  backdrop-filter: blur(8px);
}

.order-detail-sheet {
  display: flex;
  width: min(100%, 512px);
  max-height: min(92dvh, 813px);
  flex-direction: column;
  overflow: hidden;
  border-radius: 32px 32px 0 0;
  background: #fff;
  box-shadow: 0 -10px 40px rgba(140, 10, 21, 0.12);
  animation: detail-sheet-in 0.24s ease-out;
}

@keyframes detail-sheet-in {
  from {
    opacity: 0;
    transform: translateY(24px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.order-detail-sheet__header {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  padding: 20px 24px 14px;
  background: #fff;
}

.detail-header-spacer,
.detail-close {
  width: 32px;
  height: 32px;
}

.order-detail-sheet__header h2 {
  margin: 0;
  color: #1a1c1c;
  font-size: 18px;
  font-weight: 900;
  letter-spacing: 0.02em;
}

.detail-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 999px;
  color: #5a403e;
  background: #eeeeee;
  font-size: 22px;
  line-height: 1;
}

.order-detail-sheet__body {
  flex: 1 1 auto;
  overflow-y: auto;
  padding: 0 24px 24px;
  scrollbar-width: none;
}

.order-detail-sheet__body::-webkit-scrollbar {
  display: none;
}

.detail-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 10px 0 18px;
  text-align: center;
}

.detail-status-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border-radius: 999px;
  margin-bottom: 16px;
  padding: 6px 12px;
  font-size: 14px;
  font-weight: 800;
}

.detail-status-pill--won {
  color: #8f0d17;
  background: #ffdad7;
}

.detail-status-pill--pending {
  color: #574500;
  background: #ffe088;
}

.detail-status-pill--lost {
  color: #5a403e;
  background: #dadada;
}

.detail-prize {
  color: #8c0a15;
  font-family: "Plus Jakarta Sans", Avenir Next, ui-sans-serif, system-ui, sans-serif;
  font-size: clamp(34px, 9vw, 44px);
  font-weight: 900;
  letter-spacing: -0.08em;
  line-height: 1;
}

.detail-prize--text {
  color: #5a403e;
  font-size: 34px;
  letter-spacing: -0.05em;
}

.detail-hero p {
  margin: 14px 0 0;
  border-radius: 999px;
  padding: 7px 16px;
  color: #5a403e;
  background: #f3f3f3;
  font-size: 13px;
  font-weight: 700;
}

.detail-lottery-card,
.detail-panel,
.detail-meta-card {
  margin-top: 16px;
  border: 1px solid rgba(238, 238, 238, 0.75);
  border-radius: 22px;
  background: #f3f3f3;
}

.detail-lottery-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 18px;
}

.detail-lottery-card__icon {
  display: inline-flex;
  width: 48px;
  height: 48px;
  flex: 0 0 auto;
  align-items: center;
  justify-content: center;
  border-radius: 18px;
  color: #8c0a15;
  background: #fff;
  box-shadow: 0 4px 14px rgba(26, 28, 28, 0.04);
  font-size: 18px;
  font-weight: 900;
}

.detail-lottery-card__main {
  min-width: 0;
  flex: 1;
}

.detail-lottery-card h3 {
  overflow: hidden;
  margin: 0;
  color: #1a1c1c;
  font-size: 16px;
  font-weight: 900;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail-lottery-card p {
  margin: 5px 0 0;
  color: #5a403e;
  font-size: 12px;
  font-weight: 700;
}

.detail-lottery-card__amount {
  flex: 0 0 auto;
  text-align: right;
}

.detail-lottery-card__amount strong {
  display: block;
  color: #1a1c1c;
  font-family: "Plus Jakarta Sans", Avenir Next, ui-sans-serif, system-ui, sans-serif;
  font-size: 17px;
  font-weight: 900;
}

.detail-lottery-card__amount span {
  display: block;
  margin-top: 4px;
  color: #5a403e;
  font-size: 11px;
  font-weight: 700;
}

.detail-panel {
  padding: 18px;
}

.detail-panel h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 0 14px;
  color: #1a1c1c;
  font-size: 14px;
  font-weight: 900;
}

.detail-panel h3 span {
  display: inline-flex;
  width: 4px;
  height: 14px;
  border-radius: 999px;
  background: #8c0a15;
}

.detail-panel--draw h3 span {
  background: #e9c349;
}

.detail-panel--match h3 span {
  background: #2f7d32;
}

.detail-bet-groups {
  display: grid;
  gap: 10px;
  margin-bottom: 18px;
}

.detail-attribute-groups {
  display: grid;
  gap: 10px;
  margin-bottom: 18px;
}

.detail-attribute-group {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  border-radius: 16px;
  padding: 12px;
  background: #fff;
  box-shadow: inset 0 0 0 1px rgba(140, 10, 21, 0.08);
}

.detail-attribute-group__label {
  flex: 0 0 auto;
  color: #5a403e;
  font-size: 12px;
  font-weight: 900;
}

.detail-attribute-group__values {
  display: flex;
  min-width: 0;
  flex: 1;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 8px;
}

.detail-attribute-pill {
  display: inline-flex;
  min-width: 34px;
  height: 34px;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  padding: 0 12px;
  color: #8c0a15;
  background: #ffdad7;
  font-size: 15px;
  font-weight: 900;
}

.detail-attribute-pill--matched {
  color: #fff;
  background: linear-gradient(135deg, #8c0a15, #af2829);
  box-shadow: 0 6px 16px rgba(140, 10, 21, 0.2);
}

.detail-number-balls {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-bottom: 18px;
}

.detail-show-more {
  width: 100%;
  border: 0;
  border-radius: 14px;
  padding: 12px 14px;
  color: #8c0a15;
  background: #ffdad7;
  font-size: 13px;
  font-weight: 900;
}

.detail-number-balls--bet,
.detail-number-balls--draw {
  margin-bottom: 0;
}

.detail-number-balls--matched {
  border-radius: 16px;
  padding: 8px;
  background: #fff7f6;
  box-shadow: inset 0 0 0 1px rgba(140, 10, 21, 0.16);
}

.detail-number-ball {
  display: inline-flex;
  width: 40px;
  height: 40px;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  font-family: "Plus Jakarta Sans", Avenir Next, ui-sans-serif, system-ui, sans-serif;
  font-size: 18px;
  font-weight: 900;
}

.detail-number-ball--bet {
  color: #fff;
  background: linear-gradient(135deg, #8c0a15, #af2829);
  box-shadow: 0 6px 16px rgba(140, 10, 21, 0.22);
}

.detail-number-ball--draw {
  border: 1px solid #dadada;
  color: #1a1c1c;
  background: #e2e2e2;
  box-shadow: 0 2px 8px rgba(26, 28, 28, 0.05);
}

.detail-match-list {
  display: grid;
  gap: 10px;
}

.detail-match-item {
  border: 1px solid #e2e2e2;
  border-radius: 16px;
  padding: 14px;
  background: #fff;
}

.detail-match-item--hit {
  border-color: #ffb3ad;
  background: #fff7f6;
}

.detail-match-item--miss {
  background: #f9f9f9;
}

.detail-match-item--pending {
  border-color: #f2d675;
  background: #fff9de;
}

.detail-match-item span {
  display: block;
  color: #5a403e;
  font-size: 11px;
  font-weight: 900;
  letter-spacing: 0.08em;
}

.detail-match-item strong {
  display: block;
  margin-top: 5px;
  color: #1a1c1c;
  font-size: 16px;
  font-weight: 900;
}

.detail-match-item p {
  margin: 6px 0 0;
  color: #5a403e;
  font-size: 12px;
  font-weight: 700;
}

.detail-empty-value {
  display: inline-flex;
  border-radius: 999px;
  padding: 8px 12px;
  color: #5a403e;
  background: #fff;
  font-size: 13px;
  font-weight: 800;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 18px 16px;
  border-radius: 14px;
  padding: 16px;
  background: #fff;
}

.detail-grid span,
.detail-meta-card span {
  display: block;
  margin-bottom: 5px;
  color: #5a403e;
  font-size: 11px;
  font-weight: 800;
  letter-spacing: 0.08em;
}

.detail-grid strong,
.detail-meta-card strong {
  display: block;
  min-width: 0;
  overflow: hidden;
  color: #1a1c1c;
  font-size: 13px;
  font-weight: 900;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.detail-meta-card {
  display: grid;
  gap: 14px;
  padding: 16px;
  background: #fff;
  border-color: #e2e2e2;
}

.detail-meta-card > div {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.detail-meta-card span {
  flex: 0 0 auto;
  margin: 0;
}

.detail-meta-card strong {
  flex: 1;
  text-align: right;
}

.detail-order-number {
  border-radius: 8px;
  padding: 6px 8px;
  background: #f3f3f3;
}

.detail-meta-card button {
  flex: 0 0 auto;
  border: 0;
  border-radius: 8px;
  padding: 6px 8px;
  color: #8c0a15;
  background: #ffdad7;
  font-size: 11px;
  font-weight: 900;
}

.order-detail-sheet__footer {
  flex: 0 0 auto;
  border-top: 1px solid #f3f3f3;
  padding: 16px 24px max(24px, env(safe-area-inset-bottom));
  background: rgba(255, 255, 255, 0.92);
  backdrop-filter: blur(16px);
}

.order-detail-sheet__footer button {
  display: flex;
  width: 100%;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border: 0;
  border-radius: 14px;
  padding: 15px 18px;
  color: #fff;
  background: linear-gradient(135deg, #8c0a15, #af2829);
  box-shadow: 0 8px 20px rgba(140, 10, 21, 0.25);
  font-size: 16px;
  font-weight: 900;
  transition: opacity 0.2s ease, transform 0.2s ease, box-shadow 0.2s ease;
}

.order-detail-sheet__footer button:active {
  transform: scale(0.98);
}

@media (min-width: 768px) {
  .order-detail-overlay {
    align-items: center;
  }

  .order-detail-sheet {
    border-radius: 32px;
    box-shadow: 0 20px 40px rgba(140, 10, 21, 0.15);
  }
}

@media (max-width: 420px) {
  .detail-lottery-card {
    align-items: flex-start;
  }

  .detail-lottery-card__amount {
    display: none;
  }

  .detail-grid {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 360px) {
  .detail-number-ball {
    width: 36px;
    height: 36px;
    font-size: 16px;
  }
}
</style>
