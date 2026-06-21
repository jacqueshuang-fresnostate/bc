<script setup lang="ts">
import { ref, watch } from 'vue'

const props = withDefaults(defineProps<{
  selectedCount: number
  totalAmount: number
  canSubmit?: boolean
  submitting?: boolean
  submitText?: string
  groupBuyMode?: boolean
  groupBuySelfShares?: number
  groupBuyShareCount?: number
  groupBuySelfSharesHint?: string
  groupBuyPaymentAmount?: string | number
}>(), {
  canSubmit: true,
  submitting: false,
  submitText: '立即投注',
  groupBuyMode: false,
  groupBuySelfShares: 0,
  groupBuyShareCount: 0,
  groupBuySelfSharesHint: '',
  groupBuyPaymentAmount: '0.00',
})

const emit = defineEmits<{
  submit: []
  'update:groupBuySelfShares': [value: number]
  groupBuySelfSharesInput: []
  groupBuySelfSharesBlur: []
}>()

const selfSharesDraft = ref('0')
const editingSelfShares = ref(false)

watch(
  () => props.groupBuySelfShares,
  (value) => {
    if (!editingSelfShares.value) selfSharesDraft.value = String(Math.max(0, Math.floor(Number(value || 0))))
  },
  { immediate: true },
)

function amountText(value: string | number) {
  const amount = Number(value || 0)
  return Number.isFinite(amount) ? amount.toFixed(2) : '0.00'
}

function submitText() {
  if (props.groupBuyMode && !props.submitting) return '投注'
  return props.submitText
}

function updateSelfShares(event: Event) {
  const input = event.target as HTMLInputElement
  const digits = input.value.replace(/\D/g, '')
  selfSharesDraft.value = digits
  input.value = digits
  emit('groupBuySelfSharesInput')
  emit('update:groupBuySelfShares', digits ? Number(digits) : 0)
}

function focusSelfShares() {
  editingSelfShares.value = true
  selfSharesDraft.value = props.groupBuySelfShares ? String(Math.floor(Number(props.groupBuySelfShares))) : ''
}

function blurSelfShares() {
  editingSelfShares.value = false
  emit('groupBuySelfSharesBlur')
}
</script>

<template>
  <section class="bet-bottom-bar fixed bottom-0 left-0 right-0 z-50 bg-[#f9f9f9] px-6 pt-5 shadow-[0_-8px_40px_rgba(140,10,21,0.06)]" :class="{ 'bet-bottom-bar--group-buy': props.groupBuyMode }">
    <div class="mx-auto max-w-md">
      <div class="standard-bottom-card" :class="{ 'standard-bottom-card--group-buy': props.groupBuyMode }">
        <div v-if="props.groupBuyMode" class="standard-bottom-card__summary group-buy-summary">
          <div class="group-buy-summary__top">
            <label class="group-buy-self-shares">
              <span>自购</span>
              <span class="group-buy-self-shares__input">
                <input
                  :value="selfSharesDraft"
                  type="text"
                  inputmode="numeric"
                  pattern="[0-9]*"
                  :disabled="props.submitting"
                  aria-label="自购份数"
                  @focus="focusSelfShares"
                  @input="updateSelfShares"
                  @blur="blurSelfShares"
                />
              </span>
              <em>/{{ props.groupBuyShareCount }}份</em>
            </label>
            <span class="group-buy-summary__count">共{{ props.selectedCount }}注</span>
          </div>
          <div class="group-buy-summary__amount">
            <span>共计 <b>¥{{ amountText(props.totalAmount) }}</b></span>
            <small>需付 ¥{{ amountText(props.groupBuyPaymentAmount) }}</small>
          </div>
          <p v-if="props.groupBuySelfSharesHint">{{ props.groupBuySelfSharesHint }}</p>
        </div>
        <div v-else class="standard-bottom-card__summary">
          <span>已选 <strong>{{ props.selectedCount }}</strong> 注</span>
          <b>¥{{ amountText(props.totalAmount) }}</b>
        </div>
        <button class="unified-submit-button h-14 rounded-2xl text-lg font-bold shadow-lg shadow-red-900/20 disabled:opacity-60" type="button" :disabled="!props.canSubmit || props.submitting" @click="emit('submit')">{{ submitText() }}</button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.bet-bottom-bar {
  bottom: var(--mobile-viewport-bottom-inset);
  padding-bottom: 1.75rem;
}

@supports (padding-bottom: max(1px, 2px)) {
  .bet-bottom-bar {
    padding-bottom: max(1.75rem, env(safe-area-inset-bottom, 0px));
  }
}

.unified-submit-button {
  color: #fff;
  background: #af2829;
}

.bet-bottom-bar--group-buy {
  padding-top: 12px;
}

.standard-bottom-card {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(128px, 42%);
  gap: 12px;
  align-items: center;
}

.standard-bottom-card--group-buy {
  grid-template-columns: minmax(0, 1fr) minmax(118px, 36%);
}

.standard-bottom-card__summary {
  min-width: 0;
  color: #5a403e;
  font-size: 13px;
  font-weight: 800;
}

.standard-bottom-card__summary > span,
.standard-bottom-card__summary > b {
  display: block;
  white-space: nowrap;
}

.standard-bottom-card__summary > span > strong {
  color: #8c0a15;
  font-family: var(--font-headline, inherit);
  font-size: 22px;
  font-weight: 900;
}

.standard-bottom-card__summary b {
  margin-top: 2px;
  color: #735c00;
  font-family: var(--font-headline, inherit);
  font-size: 20px;
  font-weight: 900;
}

.group-buy-summary {
  display: grid;
  gap: 4px;
}

.group-buy-summary__top,
.group-buy-summary__amount {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.group-buy-summary__top {
  justify-content: space-between;
}

.group-buy-self-shares {
  display: inline-flex;
  min-width: 0;
  align-items: center;
  gap: 5px;
  border-radius: 14px;
  border: 1px solid rgba(175, 40, 41, 0.12);
  padding: 4px 5px 4px 8px;
  background: #fff8ed;
  color: #5a403e;
  font-size: 11px;
  font-weight: 900;
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.85);
  white-space: nowrap;
}

.group-buy-self-shares__input {
  display: inline-flex;
  min-height: 32px;
  min-width: 58px;
  align-items: center;
  justify-content: center;
  border: 1px solid rgba(175, 40, 41, 0.32);
  border-radius: 11px;
  background: #fff;
  box-shadow:
    0 2px 8px rgba(140, 10, 21, 0.08),
    inset 0 1px 0 rgba(255, 255, 255, 0.9);
  transition:
    border-color 0.16s ease,
    box-shadow 0.16s ease,
    background 0.16s ease;
}

.group-buy-self-shares:focus-within .group-buy-self-shares__input {
  border-color: #af2829;
  background: #fffafa;
  box-shadow:
    0 0 0 3px rgba(175, 40, 41, 0.12),
    0 4px 12px rgba(140, 10, 21, 0.14);
}

.group-buy-self-shares__input input {
  width: 54px;
  min-width: 0;
  border: 0;
  background: transparent;
  color: #8c0a15;
  font-family: var(--font-headline, inherit);
  font-size: 20px;
  font-weight: 900;
  line-height: 1.05;
  outline: none;
  text-align: center;
}

.group-buy-self-shares__input input:disabled {
  color: #b29b96;
}

.group-buy-self-shares em {
  color: #735c00;
  font-style: normal;
  font-size: 10px;
}

.group-buy-summary__count {
  overflow: hidden;
  color: #8e706d;
  font-size: 11px;
  font-weight: 900;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.group-buy-summary__amount {
  flex-wrap: wrap;
  color: #5a403e;
  font-size: 12px;
  font-weight: 900;
}

.group-buy-summary__amount b {
  color: #8c0a15;
  font-family: var(--font-headline, inherit);
  font-size: 18px;
  font-weight: 900;
}

.group-buy-summary__amount small {
  color: #735c00;
  font-size: 11px;
  font-weight: 900;
  white-space: nowrap;
}

.group-buy-summary p {
  overflow: hidden;
  margin: 0;
  color: #8e706d;
  font-size: 10px;
  font-weight: 800;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
