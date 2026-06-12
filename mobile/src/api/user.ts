import type { AxiosResponse } from 'axios'
import http from './http'
import { errorMessage as formatErrorMessage, userFacingErrorMessage } from '../utils/errorMessage'

export type ApiEnvelope<T> = {
  success: boolean
  data: T | null
  message: string
}

export type UserKind = 'regular' | 'agent'
export type UserStatus = 'active' | 'suspended' | 'locked'

export type UserSummary = {
  id: string
  username: string
  email?: string | null
  avatarUrl: string
  kind: UserKind
  status: UserStatus
  balanceMinor: number
  agentId?: string | null
  inviteCode: string
}

export type InviteStatus = 'pending' | 'active' | 'disabled'
export type RebateMode = 'immediate' | 'rechargeTiered'

export type UserInvitationDirectUser = {
  id: string
  username: string
  status: UserStatus
  inviteStatus: InviteStatus
  rebateEnabled: boolean
  totalDepositMinor: number
  createdAt: string
}

export type UserInvitationSummary = {
  canInvite: boolean
  invitationCode: string
  directCount: number
  activeDirectCount: number
  totalDirectDepositMinor: number
  totalPaidCommissionMinor: number
  rebateMode: RebateMode
  defaultRechargeRebateBasisPoints: number
  directUsers: UserInvitationDirectUser[]
}

export type UserAuthSession = {
  token: string
  user: UserSummary
}

export type RegistrationConfig = {
  usernameEnabled: boolean
  emailEnabled: boolean
  agentInviteRequired: boolean
}

export type MobileUserProfile = UserSummary & {
  balance: string
  avatar_url: string
  invitation_code: string
  can_invite: boolean
  inviter?: {
    username: string
    invitation_code?: string | null
  } | null
  used_invitation_code?: string | null
  usdt_balance?: string | number | null
}

export type MobileSiteConfig = {
  platformName: string
  logoImageUrl?: string | null
  intro: string
}

export type MobileAdvertisement = {
  id: string
  title: string
  imageUrl: string
  linkUrl?: string | null
  sortOrder: number
}

export type WithdrawalMethodType = 'alipay' | 'wechat' | 'bankCard'

export type WithdrawalMethod = {
  id: string
  userId: string
  methodType: WithdrawalMethodType
  accountHolder: string
  accountNumber: string
  bankName?: string | null
  isDefault: boolean
  createdAt: string
  updatedAt: string
}

export type WithdrawalMethodPayload = {
  methodType: WithdrawalMethodType
  accountHolder: string
  accountNumber: string
  bankName?: string
  isDefault: boolean
}

export type WithdrawalOrderStatus = 'pending' | 'approved' | 'rejected' | 'cancelled'

export type WithdrawalOrder = {
  id: string
  userId: string
  username: string
  methodId: string
  methodType: WithdrawalMethodType
  accountHolder: string
  accountNumber: string
  bankName?: string | null
  amountMinor: number
  status: WithdrawalOrderStatus
  createdAt: string
  reviewedAt?: string | null
}

export type CreateWithdrawalOrderPayload = {
  methodId: string
  amountMinor: number
}

export type RechargeChannel = 'rainbowEpay' | 'customerService'

export type RechargeOrderStatus = 'pending' | 'waitingCustomerService' | 'paid' | 'cancelled'

export type RechargeChannelConfig = {
  channel: RechargeChannel
  name: string
  enabled: boolean
  description: string
  payTypes: string[]
}

export type RechargeConfig = {
  channels: RechargeChannelConfig[]
  minAmountMinor: number
  maxAmountMinor: number
}

export type RechargeOrder = {
  id: string
  userId: string
  username: string
  channel: RechargeChannel
  amountMinor: number
  status: RechargeOrderStatus
  payType?: string | null
  providerTradeNo?: string | null
  paymentUrl?: string | null
  supportConversationId?: string | null
  createdAt: string
  paidAt?: string | null
}

export type CreateRechargeOrderPayload = {
  channel: RechargeChannel
  amountMinor: number
  payType?: string
}

export type CreateRechargeOrderResponse = {
  order: RechargeOrder
  paymentUrl?: string | null
  supportConversationId?: string | null
  message: string
}

export type LedgerEntryKind =
  | 'agentRebateWithdrawal'
  | 'manualAdjustment'
  | 'orderDebit'
  | 'orderRefund'
  | 'payoutCredit'
  | 'rechargeCredit'
  | 'rechargeRebateCredit'
  | 'withdrawalFreeze'
  | 'withdrawalPayout'
  | 'withdrawalReject'
  | 'groupBuyDebit'
  | 'groupBuyRefund'

export type LedgerEntry = {
  id: string
  userId: string
  kind: LedgerEntryKind
  amountMinor: number
  balanceAfterMinor: number
  referenceId?: string | null
  description: string
  createdAt: string
}

export type SupportMessageAuthor = 'user' | 'admin' | 'system'
export type SupportMessageType = 'text' | 'image'

export type SupportConversationStatus = 'open' | 'pending' | 'resolved' | 'closed'

export type SupportPriority = 'normal' | 'urgent'

export type SupportMessage = {
  id: string
  author: SupportMessageAuthor
  authorId: string
  authorName: string
  messageType?: SupportMessageType
  content: string
  imageUrl?: string | null
  createdAt: string
}

export type SupportConversation = {
  id: string
  userId: string
  username: string
  subject: string
  status: SupportConversationStatus
  priority: SupportPriority
  assignedAdminId?: string | null
  assignedAdminName?: string | null
  unreadCount: number
  userUnreadCount: number
  createdAt: string
  updatedAt: string
  messages: SupportMessage[]
}

export type ChatHallMessage = {
  id: string
  userId: string
  username: string
  avatarUrl?: string
  content: string
  messageType?: 'text' | 'redPacket' | 'groupBuyPlan'
  payload?: ChatHallMessagePayload | null
  createdAt: string
}

export type CreateChatHallMessagePayload = {
  content: string
}

export type ChatHallRedPacketPayload = {
  redPacketId: string
  greeting: string
  totalAmountMinor: number
  remainingAmountMinor: number
  claimCount: number
  claimedCount: number
}

export type ChatHallGroupBuyPlanPayload = {
  planId: string
  lotteryId: string
  lotteryName: string
  issue: string
  playName: string
  title: string
  totalAmountMinor: number
  shareAmountMinor: number
  soldShares: number
  availableShares: number
  progressPercent: number
  status: string
}

export type ChatHallMessagePayload = ChatHallRedPacketPayload | ChatHallGroupBuyPlanPayload | Record<string, unknown>

export type CreateChatHallRedPacketPayload = {
  amountMinor: number
  claimCount: number
  greeting: string
}

export type ChatHallRedPacketClaim = {
  id: string
  redPacketId: string
  userId: string
  username: string
  amountMinor: number
  createdAt: string
}

export type ClaimChatHallRedPacketResponse = {
  message: ChatHallMessage
  claim: ChatHallRedPacketClaim
  availableBalanceMinor: number
}

type LoginPayload = {
  loginKey: string
  password: string
}

type RegisterPayload = {
  username?: string
  email?: string
  password: string
  inviteCode?: string
}

function isEnvelope<T>(payload: unknown): payload is ApiEnvelope<T> {
  return Boolean(
    payload
      && typeof payload === 'object'
      && 'success' in payload
      && 'message' in payload,
  )
}

export function unwrapApiData<T>(response: AxiosResponse<ApiEnvelope<T> | T>): T {
  const payload = response.data
  if (!isEnvelope<T>(payload)) return payload as T
  if (!payload.success || payload.data === null) {
    throw new Error(userFacingErrorMessage(payload.message, '请求失败'))
  }
  return payload.data
}

export function errorMessage(error: unknown, fallback: string) {
  return formatErrorMessage(error, fallback)
}

function formatMinorAmount(value: number) {
  return (Number(value || 0) / 100).toFixed(2)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value))
}

function fieldValue(record: Record<string, unknown>, camelKey: string, snakeKey?: string) {
  return record[camelKey] ?? (snakeKey ? record[snakeKey] : undefined)
}

function stringField(record: Record<string, unknown>, camelKey: string, snakeKey?: string) {
  const value = fieldValue(record, camelKey, snakeKey)
  return String(value ?? '').trim()
}

function optionalStringField(record: Record<string, unknown>, camelKey: string, snakeKey?: string) {
  const value = stringField(record, camelKey, snakeKey)
  return value || null
}

function numberField(record: Record<string, unknown>, camelKey: string, snakeKey?: string) {
  return Number(fieldValue(record, camelKey, snakeKey) ?? 0)
}

function normalizeSupportConversationStatus(value: unknown): SupportConversationStatus {
  const status = String(value ?? '').trim().toLowerCase()
  if (status === 'pending' || status === 'resolved' || status === 'closed') return status
  return 'open'
}

function normalizeSupportMessageType(value: unknown, imageUrl: string | null): SupportMessageType {
  const messageType = String(value ?? '').trim().toLowerCase()
  if (messageType === 'image' || imageUrl) return 'image'
  return 'text'
}

function contentLooksLikeImageUrl(value: string) {
  return /^https?:\/\/\S+\.(?:apng|avif|gif|jpe?g|png|webp)(?:[?#]\S*)?$/i.test(value)
}

function normalizeSupportMessage(raw: unknown): SupportMessage | null {
  if (!isRecord(raw)) return null
  const content = stringField(raw, 'content')
  const explicitImageUrl = optionalStringField(raw, 'imageUrl', 'image_url')
  const imageUrl = explicitImageUrl || (contentLooksLikeImageUrl(content) ? content : null)
  const messageType = normalizeSupportMessageType(
    fieldValue(raw, 'messageType', 'message_type'),
    imageUrl,
  )

  return {
    id: stringField(raw, 'id'),
    author: stringField(raw, 'author') as SupportMessageAuthor,
    authorId: stringField(raw, 'authorId', 'author_id'),
    authorName: stringField(raw, 'authorName', 'author_name'),
    messageType,
    content: imageUrl === content ? '' : content,
    imageUrl,
    createdAt: stringField(raw, 'createdAt', 'created_at'),
  }
}

function normalizeSupportConversation(raw: unknown): SupportConversation {
  const record = isRecord(raw) ? raw : {}
  const messages = Array.isArray(record.messages)
    ? record.messages.map(normalizeSupportMessage).filter((message): message is SupportMessage => Boolean(message))
    : []
  const status = normalizeSupportConversationStatus(fieldValue(record, 'status'))

  return {
    id: stringField(record, 'id'),
    userId: stringField(record, 'userId', 'user_id'),
    username: stringField(record, 'username'),
    subject: stringField(record, 'subject'),
    status,
    priority: stringField(record, 'priority') as SupportPriority,
    assignedAdminId: optionalStringField(record, 'assignedAdminId', 'assigned_admin_id'),
    assignedAdminName: optionalStringField(record, 'assignedAdminName', 'assigned_admin_name'),
    unreadCount: numberField(record, 'unreadCount', 'unread_count'),
    userUnreadCount: numberField(record, 'userUnreadCount', 'user_unread_count'),
    createdAt: stringField(record, 'createdAt', 'created_at'),
    updatedAt: stringField(record, 'updatedAt', 'updated_at'),
    messages,
  }
}

export function isVisibleSupportConversation(conversation: SupportConversation) {
  return conversation.status !== 'closed'
}

function normalizeSupportConversations(raw: unknown): SupportConversation[] {
  return Array.isArray(raw)
    ? raw.map(normalizeSupportConversation).filter(isVisibleSupportConversation)
    : []
}

export function normalizeUserProfile(user: UserSummary): MobileUserProfile {
  return {
    ...user,
    balance: formatMinorAmount(user.balanceMinor),
    avatar_url: user.avatarUrl || '',
    invitation_code: user.inviteCode,
    can_invite: user.kind === 'agent',
  }
}

export async function fetchRegisterOptions() {
  return unwrapApiData<RegistrationConfig>(await http.get('/user/register-options'))
}

export async function loginUser(payload: LoginPayload) {
  return unwrapApiData<UserAuthSession>(await http.post('/user/login', payload))
}

export async function registerUser(payload: RegisterPayload) {
  return unwrapApiData<UserSummary>(await http.post('/user/register', payload))
}

export async function fetchCurrentUser() {
  const profile = unwrapApiData<{ user: UserSummary }>(await http.get('/user/me'))
  return profile.user
}

export async function fetchCurrentUserProfile() {
  return normalizeUserProfile(await fetchCurrentUser())
}

export async function updateUserAvatarUrl(avatarUrl: string) {
  const profile = unwrapApiData<{ user: UserSummary }>(
    await http.put('/user/avatar', { avatarUrl }),
  )
  return normalizeUserProfile(profile.user)
}

export async function uploadUserAvatar(file: File) {
  const formData = new FormData()
  formData.append('file', file)
  const profile = unwrapApiData<{ user: UserSummary }>(
    await http.post('/user/avatar/upload', formData),
  )
  return normalizeUserProfile(profile.user)
}

export async function bindUserEmail(email: string) {
  return unwrapApiData<UserAuthSession>(await http.post('/user/bind-email', { email }))
}

export async function changeUserPassword(oldPassword: string, newPassword: string) {
  return unwrapApiData<UserAuthSession>(
    await http.post('/user/password/change', { oldPassword, newPassword }),
  )
}

export async function requestPasswordReset(loginKey: string) {
  return unwrapApiData<{ resetToken: string; expiresAt: string }>(
    await http.post('/user/forgot-password', { loginKey }),
  )
}

export async function resetUserPassword(resetToken: string, newPassword: string) {
  return unwrapApiData<{ reset: boolean }>(
    await http.post('/user/reset-password', { resetToken, newPassword }),
  )
}

export async function fetchMobileSiteConfig() {
  return unwrapApiData<MobileSiteConfig>(await http.get('/user/mobile/site-config'))
}

export async function fetchMobileAdvertisements() {
  return unwrapApiData<MobileAdvertisement[]>(await http.get('/user/mobile/advertisements'))
}

export async function fetchWithdrawalMethods() {
  return unwrapApiData<WithdrawalMethod[]>(await http.get('/user/withdrawal-methods'))
}

export async function createWithdrawalMethod(payload: WithdrawalMethodPayload) {
  return unwrapApiData<WithdrawalMethod>(await http.post('/user/withdrawal-methods', payload))
}

export async function updateWithdrawalMethod(id: string, payload: WithdrawalMethodPayload) {
  return unwrapApiData<WithdrawalMethod>(await http.put(`/user/withdrawal-methods/${id}`, payload))
}

export async function deleteWithdrawalMethod(id: string) {
  await http.delete(`/user/withdrawal-methods/${id}`)
}

export async function fetchWithdrawalOrders() {
  return unwrapApiData<WithdrawalOrder[]>(await http.get('/user/withdrawals'))
}

export async function createWithdrawalOrder(payload: CreateWithdrawalOrderPayload) {
  return unwrapApiData<WithdrawalOrder>(await http.post('/user/withdrawals', payload))
}

export async function fetchRechargeConfig() {
  return unwrapApiData<RechargeConfig>(await http.get('/user/recharge/config'))
}

export async function fetchRechargeOrders() {
  return unwrapApiData<RechargeOrder[]>(await http.get('/user/recharge/orders'))
}

export async function fetchUserLedgerEntries() {
  return unwrapApiData<LedgerEntry[]>(await http.get('/user/ledger-entries'))
}

export async function fetchInvitationSummary() {
  return unwrapApiData<UserInvitationSummary>(await http.get('/user/invitations/summary'))
}

export async function createRechargeOrder(payload: CreateRechargeOrderPayload) {
  return unwrapApiData<CreateRechargeOrderResponse>(
    await http.post('/user/recharge/orders', payload),
  )
}

export async function fetchChatHallMessages() {
  return unwrapApiData<ChatHallMessage[]>(await http.get('/user/chat-hall/messages'))
}

export async function sendChatHallMessage(content: string) {
  const payload: CreateChatHallMessagePayload = { content }
  return unwrapApiData<ChatHallMessage>(await http.post('/user/chat-hall/messages', payload))
}

export async function sendChatHallRedPacket(payload: CreateChatHallRedPacketPayload) {
  return unwrapApiData<ChatHallMessage>(await http.post('/user/chat-hall/red-packets', payload))
}

export async function claimChatHallRedPacket(redPacketId: string) {
  return unwrapApiData<ClaimChatHallRedPacketResponse>(
    await http.post(`/user/chat-hall/red-packets/${encodeURIComponent(redPacketId)}/claim`),
  )
}

export async function shareChatHallGroupBuyPlan(planId: string) {
  return unwrapApiData<ChatHallMessage>(
    await http.post('/user/chat-hall/group-buy-plans', { planId }),
  )
}

export async function fetchSupportConversations() {
  return normalizeSupportConversations(
    unwrapApiData<unknown>(await http.get('/user/support/conversations')),
  )
}

export async function fetchSupportConversation(id: string) {
  return normalizeSupportConversation(
    unwrapApiData<unknown>(
      await http.get(`/user/support/conversations/${encodeURIComponent(id)}`),
    ),
  )
}

export async function replySupportConversation(id: string, content: string) {
  return normalizeSupportConversation(
    unwrapApiData<unknown>(
      await http.post(`/user/support/conversations/${encodeURIComponent(id)}/messages`, { content }),
    ),
  )
}

export async function markSupportConversationRead(id: string) {
  return normalizeSupportConversation(
    unwrapApiData<unknown>(
      await http.post(`/user/support/conversations/${encodeURIComponent(id)}/read`),
    ),
  )
}
