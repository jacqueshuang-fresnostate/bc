import type { AxiosResponse } from 'axios'
import http from './http'

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
  kind: UserKind
  status: UserStatus
  balanceMinor: number
  agentId?: string | null
  inviteCode: string
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
  invitation_code: string
  can_invite: boolean
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
    throw new Error(payload.message || '请求失败')
  }
  return payload.data
}

export function errorMessage(error: unknown, fallback: string) {
  const err = error as {
    message?: string
    response?: { data?: { message?: string; detail?: string } }
  }
  return err.response?.data?.message || err.response?.data?.detail || err.message || fallback
}

function formatMinorAmount(value: number) {
  return (Number(value || 0) / 100).toFixed(2)
}

export function normalizeUserProfile(user: UserSummary): MobileUserProfile {
  return {
    ...user,
    balance: formatMinorAmount(user.balanceMinor),
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
