<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { BetCartItem } from '../dynamic/types'

const props = defineProps<{
  show: boolean
  items: BetCartItem[]
}>()

const emit = defineEmits<{ 'update:show': [boolean]; confirm: [BetCartItem[]] }>()

// 篮子弹层只维护可编辑副本，避免用户关闭弹层时误改引擎中的真实篮子。
const editableItems = ref<BetCartItem[]>([])

const editableCount = computed(() => editableItems.value.reduce((sum, item) => sum + item.bet_count * item.multiple, 0))
const editableAmount = computed(() => editableItems.value.reduce((sum, item) => sum + item.unit_amount * item.bet_count * item.multiple, 0))
const slipAmount = (item: BetCartItem) => item.unit_amount * item.bet_count * item.multiple

watch(() => props.show, (visible) => {
  // 每次打开都从父级篮子重新拷贝，丢弃上一次未确认的弹层临时编辑。
  if (visible) editableItems.value = props.items.map(item => ({ ...item }))
})

function removeItem(id: string) {
  // 删除是弹层内的临时删除，只有确认修改后才会同步到父级篮子。
  editableItems.value = editableItems.value.filter(item => item.id !== id)
}

function clearEditableItems() {
  editableItems.value = []
}

function confirmChanges() {
  // 确认时再次拷贝单据，父组件接收后替换真实篮子并关闭弹层。
  emit('confirm', editableItems.value.map(item => ({ ...item })))
  emit('update:show', false)
}
</script>

<template>
  <van-popup :show="show" position="bottom" round :style="{ maxHeight: '68dvh' }" @update:show="emit('update:show', $event)">
    <div class="bg-[#f9f9f9] px-4 pb-4 pt-3">
      <div class="mx-auto mb-3 h-1.5 w-10 rounded-full bg-[#e2e2e2]"></div>
      <div class="mb-3 flex items-center justify-between">
        <div>
          <h2 class="font-headline text-xl font-extrabold text-[#1a1c1c]">编辑购彩篮</h2>
          <p class="mt-1 text-xs text-[#8e706d]">编辑后请确认修改</p>
        </div>
        <button class="text-sm text-[#5a403e] disabled:opacity-40" type="button" :disabled="editableItems.length === 0" @click="clearEditableItems">清空</button>
      </div>
      <div v-if="editableItems.length === 0" class="rounded-3xl bg-white px-5 py-10 text-center shadow-sm shadow-red-900/5">
        <div class="font-headline text-xl font-extrabold text-[#1a1c1c]">暂无单据</div>
        <p class="mt-2 text-sm text-[#8e706d]">先选择号码并加入购彩篮</p>
      </div>
      <div v-else class="max-h-[40dvh] space-y-3 overflow-y-auto pb-3">
        <article v-for="item in editableItems" :key="item.id" class="rounded-2xl bg-white p-4 shadow-sm shadow-red-900/5">
          <div class="mb-3 flex items-start justify-between gap-3">
            <div class="flex items-center gap-3">
              <strong class="text-lg text-[#1a1c1c]">{{ item.lottery_name }}</strong>
              <span class="rounded-lg bg-[#f3f3f3] px-2.5 py-1 text-xs text-[#5a403e]">{{ item.play_name }}</span>
            </div>
            <button class="text-[#8e706d]" type="button" @click="removeItem(item.id)">×</button>
          </div>
          <div class="mb-3 break-all font-headline text-lg font-extrabold tracking-widest text-[#8c0a15]">{{ item.display_numbers }}</div>
          <div class="mb-4 grid grid-cols-3 gap-2 rounded-2xl bg-[#fff8f0] px-3 py-2 text-center text-xs text-[#5a403e]">
            <div>
              <div>单注</div>
              <strong class="mt-1 block text-[#1a1c1c]">¥{{ item.unit_amount.toFixed(2) }}</strong>
            </div>
            <div>
              <div>共计</div>
              <strong class="mt-1 block text-[#1a1c1c]">{{ item.bet_count }} 注</strong>
            </div>
            <div>
              <div>单据金额</div>
              <strong class="mt-1 block text-[#8c0a15]">¥{{ slipAmount(item).toFixed(2) }}</strong>
            </div>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-sm text-[#5a403e]">下单倍数</span>
            <span class="rounded-xl bg-[#eeeeee] px-4 py-2 text-lg font-bold text-[#1a1c1c]">{{ item.multiple }}</span>
          </div>
        </article>
      </div>
      <div class="rounded-t-2xl bg-white px-1 pt-3">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-xs text-[#5a403e]">总计 {{ editableCount }} 注</div>
            <div class="font-headline text-xl font-extrabold text-[#8c0a15]">¥{{ editableAmount.toFixed(2) }}</div>
          </div>
          <button class="rounded-2xl bg-[#af2829] px-6 py-3 text-base font-bold text-white" type="button" @click="confirmChanges">确认修改</button>
        </div>
      </div>
    </div>
  </van-popup>
</template>
