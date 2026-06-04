<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { showConfirmDialog, showToast } from 'vant'
import http from '../api/http'
import LucideIcon from '../components/mobile/LucideIcon.vue'

type MethodType = 'alipay' | 'wechat' | 'bank' | 'usdt'

type WithdrawalMethod = {
  id: number
  method_type: MethodType
  method_label: string
  type_enabled: boolean
  account_name: string
  account_no: string
  bank_name?: string | null
  branch_name?: string | null
  network?: string | null
  is_default: boolean
}

const router = useRouter()
const labels: Record<MethodType, string> = {
  alipay: '支付宝',
  wechat: '微信',
  bank: '银行卡',
  usdt: 'USDT',
}

const typeOrder: MethodType[] = ['alipay', 'wechat', 'bank', 'usdt']
const methods = ref<WithdrawalMethod[]>([])
const config = ref<Record<MethodType, boolean>>({ alipay: true, wechat: true, bank: true, usdt: true })
const loading = ref(false)
const saving = ref(false)
const actionLoading = ref(false)
const showEditor = ref(false)
const editingId = ref<number | null>(null)
const form = ref({
  method_type: 'alipay' as MethodType,
  account_name: '',
  account_no: '',
  bank_name: '',
  branch_name: '',
  network: 'TRC20',
  is_default: false,
})

const enabledTypeOptions = computed(() =>
  typeOrder.filter(type => config.value[type]).map(type => ({ text: labels[type], value: type })),
)

const editingMethod = computed(() => methods.value.find(item => item.id === editingId.value) || null)

function resetForm(type: MethodType = enabledTypeOptions.value[0]?.value || 'alipay') {
  editingId.value = null
  form.value = {
    method_type: type,
    account_name: '',
    account_no: '',
    bank_name: '',
    branch_name: '',
    network: 'TRC20',
    is_default: false,
  }
}

function methodIcon(type: MethodType) {
  if (type === 'bank') return 'account_balance'
  if (type === 'usdt' || type === 'alipay') return 'qr_code_scanner'
  if (type === 'wechat') return 'chat'
  return 'payments'
}

function methodDescription(item: WithdrawalMethod) {
  if (item.method_type === 'bank') return maskAccount(item.account_no)
  if (item.method_type === 'usdt') return shortAddress(item.account_no)
  return item.account_no || '-'
}

function maskAccount(value?: string | null) {
  const text = String(value || '')
  if (!text) return '-'
  if (text.length <= 4) return text
  return `**** **** **** ${text.slice(-4)}`
}

function shortAddress(value?: string | null) {
  const text = String(value || '')
  if (!text) return '-'
  if (text.length <= 6) return text
  return `${text.slice(0, 3)}...${text.slice(-3)}`
}

async function loadMethods() {
  loading.value = true
  try {
    const res = await http.get('/user/withdrawal-methods')
    config.value = { ...config.value, ...(res.data?.config || {}) }
    methods.value = res.data?.items || []
  } catch (e: any) {
    showToast(e.response?.data?.detail || '加载失败')
  } finally {
    loading.value = false
  }
}

function openCreate(type?: MethodType) {
  resetForm(type || enabledTypeOptions.value[0]?.value || 'alipay')
  showEditor.value = true
}

function fillForm(item: WithdrawalMethod) {
  editingId.value = item.id
  form.value = {
    method_type: item.method_type,
    account_name: item.account_name,
    account_no: item.account_no,
    bank_name: item.bank_name || '',
    branch_name: item.branch_name || '',
    network: item.network || 'TRC20',
    is_default: item.is_default,
  }
}

function openEdit(item: WithdrawalMethod) {
  if (!item.type_enabled) {
    fillForm(item)
    showEditor.value = true
    showToast('该提现方式已关闭，只能删除')
    return
  }
  fillForm(item)
  showEditor.value = true
}

async function saveMethod() {
  saving.value = true
  try {
    const payload = { ...form.value }
    if (editingId.value) {
      await http.put(`/user/withdrawal-methods/${editingId.value}`, payload)
    } else {
      await http.post('/user/withdrawal-methods', payload)
    }
    showToast('已保存')
    showEditor.value = false
    await loadMethods()
  } catch (e: any) {
    showToast(e.response?.data?.detail || '保存失败')
  } finally {
    saving.value = false
  }
}

async function deleteMethod(item: WithdrawalMethod | null) {
  if (!item || actionLoading.value) return
  try {
    await showConfirmDialog({ title: '确认删除', message: `删除 ${item.method_label} 提现方式？` })
  } catch {
    return
  }
  actionLoading.value = true
  try {
    await http.delete(`/user/withdrawal-methods/${item.id}`)
    showToast('已删除')
    showEditor.value = false
    await loadMethods()
  } catch (e: any) {
    showToast(e.response?.data?.detail || '删除失败')
  } finally {
    actionLoading.value = false
  }
}

async function setDefault(item: WithdrawalMethod | null) {
  if (!item || actionLoading.value) return
  if (!item.type_enabled) {
    showToast('该提现方式已关闭，不能设为默认')
    return
  }
  actionLoading.value = true
  try {
    await http.put(`/user/withdrawal-methods/${item.id}/default`)
    showToast('已设为默认')
    showEditor.value = false
    await loadMethods()
  } catch (e: any) {
    showToast(e.response?.data?.detail || '设置失败')
  } finally {
    actionLoading.value = false
  }
}

onMounted(loadMethods)
</script>

<template>
  <div class="withdrawal-management relative flex min-h-screen flex-col items-center bg-surface text-on-surface font-body">
    <nav class="fixed top-0 z-50 w-full bg-white/80 backdrop-blur-md shadow-[0_4px_40px_0_rgba(140,10,21,0.04)]">
      <div class="flex justify-between items-center px-6 py-4 w-full max-w-lg mx-auto">
        <button class="text-primary transition-opacity duration-200 active:scale-95 active:opacity-80" aria-label="返回" @click="router.back()">
          <LucideIcon name="arrow_back" class="h-5 w-5" />
        </button>
        <div class="font-headline font-bold tracking-tight text-xl font-black text-primary tracking-tighter">鸿福</div>
        <button class="text-primary transition-opacity duration-200 active:scale-95 active:opacity-80" aria-label="客服" @click="router.push('/support')">
          <LucideIcon name="support_agent" class="h-5 w-5" />
        </button>
      </div>
      <div class="h-[1px] w-full bg-stone-100 opacity-20"></div>
    </nav>

    <main class="w-full max-w-lg mx-auto pt-24 pb-32 px-6 flex flex-col gap-8">
      <header class="mb-4">
        <h1 class="mb-2 font-headline text-3xl font-extrabold text-on-surface">提现管理</h1>
        <p class="text-sm text-on-surface-variant font-label">管理您的提现方式</p>
      </header>

      <section class="flex flex-col gap-4">
        <van-loading v-if="loading" class="mx-auto block py-8" />
        <van-empty v-else-if="methods.length === 0" description="暂无提现方式" />
        <article
          v-for="item in methods"
          v-else
          :key="item.id"
          class="group relative overflow-hidden rounded-xl bg-surface-container-lowest p-6 shadow-[0_4px_40px_0_rgba(140,10,21,0.04)]"
        >
          <div class="pointer-events-none absolute inset-0 rounded-xl border border-outline-variant opacity-15"></div>
          <button class="relative flex w-full items-center justify-between text-left" @click="openEdit(item)">
            <span class="flex min-w-0 items-center gap-4">
              <span class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-surface-container-low text-primary">
                <LucideIcon :name="methodIcon(item.method_type)" class="h-6 w-6" />
              </span>
              <span class="min-w-0">
                <span class="flex items-center gap-2 font-headline text-lg font-bold text-on-surface">
                  {{ item.method_label }}
                  <span v-if="item.is_default" class="rounded-full bg-primary/10 px-2 py-0.5 text-[10px] font-bold text-primary">默认</span>
                  <span v-if="!item.type_enabled" class="rounded-full bg-error/10 px-2 py-0.5 text-[10px] font-bold text-error">已关闭</span>
                </span>
                <span class="block truncate text-sm text-on-surface-variant font-label">{{ methodDescription(item) }}</span>
              </span>
            </span>
            <LucideIcon name="more_vert" class="h-5 w-5 text-on-surface-variant transition-colors group-active:text-primary" />
          </button>
        </article>
      </section>

      <section class="mt-8">
        <button class="flex w-full items-center justify-center gap-2 rounded-xl bg-gradient-to-br from-primary to-primary-container py-4 font-headline text-lg font-bold !text-on-primary shadow-[0_10px_40px_0_rgba(140,10,21,0.15)] transition-transform duration-200 active:scale-95" @click="openCreate()">
          <LucideIcon name="add" class="h-5 w-5" />
          新增提现方式
        </button>
      </section>
    </main>

    <van-popup v-model:show="showEditor" position="bottom" round class="withdrawal-method-popup overflow-hidden !rounded-t-[2rem] bg-surface-container-lowest">
      <van-form class="flex h-[90vh] max-h-[90vh] flex-col" @submit="saveMethod">
        <div class="withdrawal-method-popup__header relative flex flex-col items-center px-6 pb-5 pt-7 text-center">
          <h3 class="withdrawal-method-popup__title font-headline text-xl font-black tracking-tight text-on-surface">{{ editingId ? '编辑提现方式' : '新增提现方式' }}</h3>
          <p class="withdrawal-method-popup__subtitle mt-2 text-sm font-label text-on-surface-variant">请填写真实收款信息，避免提现失败</p>
          <button class="withdrawal-method-popup__close absolute right-5 top-5 flex h-9 w-9 items-center justify-center rounded-full bg-surface-container-low text-on-surface-variant transition-colors active:bg-surface-dim" type="button" aria-label="关闭" @click="showEditor = false">
            <LucideIcon name="close" class="h-5 w-5" />
          </button>
        </div>

        <div class="flex-1 overflow-y-auto px-6 pb-6">
          <section class="withdrawal-method-popup__form-card rounded-[1.75rem]  p-4 shadow-[0_4px_40px_0_rgba(140,10,21,0.04)]">
            <van-cell-group inset class="space-y-5">
              <van-field class="withdrawal-method-popup__field" name="method_type" label="类型">
                <template #input>
                  <van-dropdown-menu class="withdrawal-method-popup__dropdown w-full">
                    <van-dropdown-item v-model="form.method_type" :options="enabledTypeOptions" :disabled="!!editingId && !editingMethod?.type_enabled" />
                  </van-dropdown-menu>
                </template>
              </van-field>
              <van-field class="withdrawal-method-popup__field" v-model="form.account_name" label="账户名" placeholder="请输入收款人或账户名" required :readonly="!!editingId && !editingMethod?.type_enabled" />
              <van-field class="withdrawal-method-popup__field" v-model="form.account_no" label="账号" placeholder="请输入账号、卡号或钱包地址" required :readonly="!!editingId && !editingMethod?.type_enabled" />
              <van-field class="withdrawal-method-popup__field" v-if="form.method_type === 'bank'" v-model="form.bank_name" label="银行" placeholder="请输入银行名称" required :readonly="!!editingId && !editingMethod?.type_enabled" />
              <van-field class="withdrawal-method-popup__field" v-if="form.method_type === 'bank'" v-model="form.branch_name" label="支行" placeholder="请输入支行名称" required :readonly="!!editingId && !editingMethod?.type_enabled" />
              <van-field class="withdrawal-method-popup__field" v-if="form.method_type === 'usdt'" v-model="form.network" label="网络" placeholder="TRC20" required :readonly="!!editingId && !editingMethod?.type_enabled" />
              <van-field class="withdrawal-method-popup__field" label="设为默认">
                <template #input><van-switch v-model="form.is_default" size="20" :disabled="!!editingId && !editingMethod?.type_enabled" /></template>
              </van-field>
            </van-cell-group>
          </section>
        </div>

        <div class="space-y-3 border-t border-outline-variant/20 bg-surface-container-lowest px-6 pb-8 pt-4 shadow-[0_-10px_40px_0_rgba(140,10,21,0.06)]">
          <div v-if="editingId" class="flex gap-3">
            <van-button block plain type="danger" native-type="button" :loading="actionLoading" class="!flex-1" @click="deleteMethod(editingMethod)">删除</van-button>
            <van-button v-if="editingMethod?.type_enabled" block plain type="primary" native-type="button" :loading="actionLoading" class="!flex-1" @click="setDefault(editingMethod)">设默认</van-button>
          </div>
          <van-button v-if="!editingId || editingMethod?.type_enabled" block round type="primary" native-type="submit" :loading="saving" class="!h-12 !rounded-2xl !border-0 !bg-gradient-to-br !from-primary !to-primary-container font-headline !text-base !font-bold !text-on-primary shadow-[0_10px_40px_0_rgba(140,10,21,0.15)]">
            保存
          </van-button>
        </div>
      </van-form>
    </van-popup>
  </div>
</template>
