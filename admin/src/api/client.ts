import type {
  ApiEnvelope,
  DashboardSummary,
  LotteryKind,
  SaveDrawSourceRequest,
  LotteryCategoryConfig,
} from '../types/dashboard';
import type { DrawSource } from '../types/dashboard';
import type {
  AdvertisementSummary,
  SaveAdvertisementRequest,
} from '../types/advertisements';
import type {
  AdminPasswordResetRequest,
  AdminRole,
  AdminSaveRequest,
  AdminSummary,
  RegistrationConfig,
  StatusUpdateRequest,
  SystemSetting,
  UpdateSystemSettingRequest,
  UserSummary,
} from '../types/access';
import type {
  AdminAuthSession,
  AdminLoginRequest,
  AdminLogoutResponse,
  CurrentAdminProfile,
} from '../types/auth';
import type {
  CreateDrawIssueRequest,
  DrawAutomationRun,
  DrawAutomationRunRequest,
  DrawIssue,
  DrawIssuePage,
  DrawIssueQuery,
  DrawIssueGenerationPreview,
  DrawIssueResultRequest,
  GenerateDrawIssueRequest,
  GenerateDrawIssuesRequest,
  LotteryDrawControl,
  SaveLotteryDrawControlRequest,
} from '../types/draws';
import type {
  AdminFinancialAccountSummary,
  LedgerEntry,
  ConfirmRechargeOrderRequest,
  FinanceOverview,
  FinancePage,
  FinancePageQuery,
  ManualBalanceAdjustmentRequest,
  RechargeOrderSummary,
  WithdrawalOrderSummary,
} from '../types/finance';
import type {
  AddGroupBuyParticipantRequest,
  CreateGroupBuyPlanRequest,
  GroupBuyPlan,
  GroupBuyPlanSummary,
  UpdateGroupBuyPlanRequest,
} from '../types/groupBuy';
import type {
  CreateInviteRecordRequest,
  InviteRecord,
  UpdateInviteRecordRequest,
} from '../types/invitations';
import type {
  PlayRuleEvaluateRequest,
  PlayRuleEvaluation,
  PlayRuleSummary,
} from '../types/playRules';
import type { CreateOrderRequest, OrderDetail, OrderListQuery } from '../types/orders';
import type { DrawSchedulerConfig, DrawSchedulerStatus } from '../types/scheduler';
import type {
  GroupBuyRobotRun,
  RobotConfigSummary,
  RobotStatusUpdateRequest,
} from '../types/robots';
import type {
  InvitePolicySummary,
  InvitePolicyUpdateRequest,
} from '../types/rebates';
import type { SettlementRun } from '../types/settlements';
import type {
  CreateSupportConversationRequest,
  SupportConversation,
  SupportReplyRequest,
  UpdateSupportConversationRequest,
} from '../types/support';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';
const AUTH_TOKEN_STORAGE_KEY = 'bc.admin.authToken';

interface JsonRequestOptions {
  body?: unknown;
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  signal?: AbortSignal;
}

async function requestJson<T>(
  path: string,
  { body, method = 'GET', signal }: JsonRequestOptions = {},
): Promise<T> {
  const token = getStoredAuthToken();
  const headers = new Headers();
  if (body !== undefined) {
    headers.set('Content-Type', 'application/json');
  }
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  const response = await fetch(`${API_BASE_URL}${path}`, {
    body: body === undefined ? undefined : JSON.stringify(body),
    headers,
    method,
    signal,
  });
  const envelope = (await response.json()) as ApiEnvelope<T>;

  if (!response.ok || !envelope.success || envelope.data === null) {
    throw new Error(envelope.message || '接口请求失败');
  }

  return envelope.data;
}

export function getStoredAuthToken() {
  if (typeof window === 'undefined') {
    return null;
  }
  return window.localStorage.getItem(AUTH_TOKEN_STORAGE_KEY);
}

export function adminRealtimeUrl() {
  if (typeof window === 'undefined') {
    return null;
  }
  const token = getStoredAuthToken();
  if (!token) {
    return null;
  }
  const base = API_BASE_URL || window.location.origin;
  const url = new URL('/api/admin/realtime', base);
  url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
  url.searchParams.set('token', token);
  return url.toString();
}

export function setStoredAuthToken(token: string) {
  window.localStorage.setItem(AUTH_TOKEN_STORAGE_KEY, token);
}

export function clearStoredAuthToken() {
  window.localStorage.removeItem(AUTH_TOKEN_STORAGE_KEY);
}

export function loginAdmin(payload: AdminLoginRequest) {
  return requestJson<AdminAuthSession>('/api/admin/auth/login', {
    body: payload,
    method: 'POST',
  });
}

export function fetchCurrentAdmin(signal?: AbortSignal) {
  return requestJson<CurrentAdminProfile>('/api/admin/auth/me', { signal });
}

export function logoutAdmin() {
  return requestJson<AdminLogoutResponse>('/api/admin/auth/logout', {
    method: 'POST',
  });
}

export function fetchDashboard(signal?: AbortSignal) {
  return requestJson<DashboardSummary>('/api/admin/dashboard', { signal });
}

export function fetchFinanceOverview(signal?: AbortSignal, query?: FinancePageQuery) {
  return requestJson<FinanceOverview>(
    adminQueryPath('/api/admin/finance-overview', query),
    { signal },
  );
}

export function fetchFinancialAccounts(
  signal?: AbortSignal,
  query?: FinancePageQuery,
) {
  return requestJson<FinancePage<AdminFinancialAccountSummary>>(
    adminQueryPath('/api/admin/financial-accounts', query),
    { signal },
  );
}

export function fetchLedgerEntries(signal?: AbortSignal, query?: FinancePageQuery) {
  return requestJson<FinancePage<LedgerEntry>>(
    adminQueryPath('/api/admin/ledger-entries', query),
    { signal },
  );
}

export function fetchRechargeOrders(signal?: AbortSignal, query?: FinancePageQuery) {
  return requestJson<FinancePage<RechargeOrderSummary>>(
    adminQueryPath('/api/admin/recharge-orders', query),
    { signal },
  );
}

export function confirmRechargeOrder(
  id: string,
  payload: ConfirmRechargeOrderRequest = {},
) {
  return requestJson<RechargeOrderSummary>(
    `/api/admin/recharge-orders/${encodeURIComponent(id)}/confirm`,
    {
      body: payload,
      method: 'POST',
    },
  );
}

export function fetchWithdrawalOrders(signal?: AbortSignal, query?: FinancePageQuery) {
  return requestJson<FinancePage<WithdrawalOrderSummary>>(
    adminQueryPath('/api/admin/withdrawal-orders', query),
    { signal },
  );
}

export function approveWithdrawalOrder(id: string) {
  return requestJson<WithdrawalOrderSummary>(
    `/api/admin/withdrawal-orders/${encodeURIComponent(id)}/approve`,
    {
      method: 'POST',
    },
  );
}

export function rejectWithdrawalOrder(id: string) {
  return requestJson<WithdrawalOrderSummary>(
    `/api/admin/withdrawal-orders/${encodeURIComponent(id)}/reject`,
    {
      method: 'POST',
    },
  );
}

export function createManualBalanceAdjustment(payload: ManualBalanceAdjustmentRequest) {
  return requestJson<LedgerEntry>('/api/admin/financial-adjustments', {
    body: payload,
    method: 'POST',
  });
}

function adminQueryPath(path: string, query?: FinancePageQuery | OrderListQuery) {
  const params = new URLSearchParams();
  const pageQuery = query as FinancePageQuery | undefined;
  if (pageQuery?.page && pageQuery.page > 0) {
    params.set('page', String(pageQuery.page));
  }
  if (pageQuery?.pageSize && pageQuery.pageSize > 0) {
    params.set('pageSize', String(pageQuery.pageSize));
  }
  if (query?.includeRobotData) {
    params.set('includeRobotData', 'true');
  }
  const queryString = params.toString();
  return queryString ? `${path}?${queryString}` : path;
}

export function fetchGroupBuyPlans(signal?: AbortSignal, query?: FinancePageQuery) {
  return requestJson<FinancePage<GroupBuyPlanSummary>>(
    adminQueryPath('/api/admin/group-buy/plans', query),
    { signal },
  );
}

export function fetchGroupBuyPlan(id: string, signal?: AbortSignal) {
  return requestJson<GroupBuyPlan>(
    `/api/admin/group-buy/plans/${encodeURIComponent(id)}`,
    { signal },
  );
}

export function createGroupBuyPlan(payload: CreateGroupBuyPlanRequest) {
  return requestJson<GroupBuyPlan>('/api/admin/group-buy/plans', {
    body: payload,
    method: 'POST',
  });
}

export function updateGroupBuyPlan(id: string, payload: UpdateGroupBuyPlanRequest) {
  return requestJson<GroupBuyPlan>(
    `/api/admin/group-buy/plans/${encodeURIComponent(id)}`,
    {
      body: payload,
      method: 'PUT',
    },
  );
}

export function addGroupBuyParticipant(
  id: string,
  payload: AddGroupBuyParticipantRequest,
) {
  return requestJson<GroupBuyPlan>(
    `/api/admin/group-buy/plans/${encodeURIComponent(id)}/participants`,
    {
      body: payload,
      method: 'POST',
    },
  );
}

export function fetchInvitations(signal?: AbortSignal) {
  return requestJson<InviteRecord[]>('/api/admin/invitations', { signal });
}

export function createInvitation(payload: CreateInviteRecordRequest) {
  return requestJson<InviteRecord>('/api/admin/invitations', {
    body: payload,
    method: 'POST',
  });
}

export function updateInvitation(id: string, payload: UpdateInviteRecordRequest) {
  return requestJson<InviteRecord>(`/api/admin/invitations/${encodeURIComponent(id)}`, {
    body: payload,
    method: 'PUT',
  });
}

export function fetchSupportConversations(signal?: AbortSignal) {
  return requestJson<SupportConversation[]>('/api/admin/support/conversations', {
    signal,
  });
}

export function createSupportConversation(
  payload: CreateSupportConversationRequest,
) {
  return requestJson<SupportConversation>('/api/admin/support/conversations', {
    body: payload,
    method: 'POST',
  });
}

export function updateSupportConversation(
  id: string,
  payload: UpdateSupportConversationRequest,
) {
  return requestJson<SupportConversation>(
    `/api/admin/support/conversations/${encodeURIComponent(id)}`,
    {
      body: payload,
      method: 'PUT',
    },
  );
}

export function replySupportConversation(id: string, payload: SupportReplyRequest) {
  return requestJson<SupportConversation>(
    `/api/admin/support/conversations/${encodeURIComponent(id)}/messages`,
    {
      body: payload,
      method: 'POST',
    },
  );
}

export function fetchUsers(signal?: AbortSignal) {
  return requestJson<UserSummary[]>('/api/admin/users', { signal });
}

export function createUser(payload: UserSummary) {
  return requestJson<UserSummary>('/api/admin/users', {
    body: payload,
    method: 'POST',
  });
}

export function updateUser(id: string, payload: UserSummary) {
  return requestJson<UserSummary>(`/api/admin/users/${encodeURIComponent(id)}`, {
    body: payload,
    method: 'PUT',
  });
}

export function setUserStatus(id: string, payload: StatusUpdateRequest) {
  return requestJson<UserSummary>(
    `/api/admin/users/${encodeURIComponent(id)}/status`,
    {
      body: payload,
      method: 'PATCH',
    },
  );
}

export function fetchAdmins(signal?: AbortSignal) {
  return requestJson<AdminSummary[]>('/api/admin/admins', { signal });
}

export function createAdmin(payload: AdminSaveRequest) {
  return requestJson<AdminSummary>('/api/admin/admins', {
    body: payload,
    method: 'POST',
  });
}

export function updateAdmin(id: string, payload: AdminSaveRequest) {
  return requestJson<AdminSummary>(`/api/admin/admins/${encodeURIComponent(id)}`, {
    body: payload,
    method: 'PUT',
  });
}

export function resetAdminPassword(id: string, payload: AdminPasswordResetRequest) {
  return requestJson<AdminSummary>(
    `/api/admin/admins/${encodeURIComponent(id)}/password`,
    {
      body: payload,
      method: 'PATCH',
    },
  );
}

export function setAdminStatus(id: string, payload: StatusUpdateRequest) {
  return requestJson<AdminSummary>(
    `/api/admin/admins/${encodeURIComponent(id)}/status`,
    {
      body: payload,
      method: 'PATCH',
    },
  );
}

export function fetchRoles(signal?: AbortSignal) {
  return requestJson<AdminRole[]>('/api/admin/roles', { signal });
}

export function createRole(payload: AdminRole) {
  return requestJson<AdminRole>('/api/admin/roles', {
    body: payload,
    method: 'POST',
  });
}

export function updateRole(id: string, payload: AdminRole) {
  return requestJson<AdminRole>(`/api/admin/roles/${encodeURIComponent(id)}`, {
    body: payload,
    method: 'PUT',
  });
}

export function deleteRole(id: string) {
  return requestJson<AdminRole>(`/api/admin/roles/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

export function fetchSystemSettings(signal?: AbortSignal) {
  return requestJson<SystemSetting[]>('/api/admin/system-settings', { signal });
}

export function updateSystemSetting(
  key: string,
  payload: UpdateSystemSettingRequest,
) {
  return requestJson<SystemSetting>(
    `/api/admin/system-settings/${encodeURIComponent(key)}`,
    {
      body: payload,
      method: 'PATCH',
    },
  );
}

export function fetchAdvertisements(signal?: AbortSignal) {
  return requestJson<AdvertisementSummary[]>('/api/admin/advertisements', {
    signal,
  });
}

export function createAdvertisement(payload: SaveAdvertisementRequest) {
  return requestJson<AdvertisementSummary>('/api/admin/advertisements', {
    body: payload,
    method: 'POST',
  });
}

export function updateAdvertisement(
  id: string,
  payload: SaveAdvertisementRequest,
) {
  return requestJson<AdvertisementSummary>(
    `/api/admin/advertisements/${encodeURIComponent(id)}`,
    {
      body: payload,
      method: 'PUT',
    },
  );
}

export function deleteAdvertisement(id: string) {
  return requestJson<AdvertisementSummary>(
    `/api/admin/advertisements/${encodeURIComponent(id)}`,
    {
      method: 'DELETE',
    },
  );
}

export async function uploadImageBedFile(
  file: File,
  uploadFieldName: string = 'file',
) {
  const token = getStoredAuthToken();
  const headers = new Headers();
  if (token) {
    headers.set('Authorization', `Bearer ${token}`);
  }

  const body = new FormData();
  body.append(uploadFieldName, file);

  const response = await fetch(`${API_BASE_URL}/api/admin/image-bed/upload`, {
    body,
    headers,
    method: 'POST',
  });
  const envelope = (await response.json()) as ApiEnvelope<unknown>;

  if (!response.ok || !envelope.success || envelope.data === null) {
    throw new Error(envelope.message || '图片上传请求失败');
  }

  return envelope.data;
}

export function fetchRegistrationConfig(signal?: AbortSignal) {
  return requestJson<RegistrationConfig>('/api/admin/registration', { signal });
}

export function updateRegistrationConfig(payload: RegistrationConfig) {
  return requestJson<RegistrationConfig>('/api/admin/registration', {
    body: payload,
    method: 'PUT',
  });
}

export function fetchInvitePolicy(signal?: AbortSignal) {
  return requestJson<InvitePolicySummary>('/api/admin/invite-policy', { signal });
}

export function updateInvitePolicy(payload: InvitePolicyUpdateRequest) {
  return requestJson<InvitePolicySummary>('/api/admin/invite-policy', {
    body: payload,
    method: 'PUT',
  });
}

export function fetchRobots(signal?: AbortSignal) {
  return requestJson<RobotConfigSummary[]>('/api/admin/robots', { signal });
}

export function createRobot(payload: RobotConfigSummary) {
  return requestJson<RobotConfigSummary>('/api/admin/robots', {
    body: payload,
    method: 'POST',
  });
}

export function updateRobot(id: string, payload: RobotConfigSummary) {
  return requestJson<RobotConfigSummary>(`/api/admin/robots/${encodeURIComponent(id)}`, {
    body: payload,
    method: 'PUT',
  });
}

export function deleteRobot(id: string) {
  return requestJson<RobotConfigSummary>(`/api/admin/robots/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

export function setRobotStatus(id: string, payload: RobotStatusUpdateRequest) {
  return requestJson<RobotConfigSummary>(
    `/api/admin/robots/${encodeURIComponent(id)}/status`,
    {
      body: payload,
      method: 'PATCH',
    },
  );
}

export function runGroupBuyRobots() {
  return requestJson<GroupBuyRobotRun>('/api/admin/robots/run', {
    method: 'POST',
  });
}

export function fetchLotteries(signal?: AbortSignal) {
  return requestJson<LotteryKind[]>('/api/admin/lotteries', { signal });
}

export function fetchLotteryCategories(signal?: AbortSignal) {
  return requestJson<LotteryCategoryConfig[]>('/api/admin/lottery-categories', {
    signal,
  });
}

export function createLotteryCategory(payload: LotteryCategoryConfig) {
  return requestJson<LotteryCategoryConfig>('/api/admin/lottery-categories', {
    body: payload,
    method: 'POST',
  });
}

export function updateLotteryCategory(
  code: string,
  payload: LotteryCategoryConfig,
) {
  return requestJson<LotteryCategoryConfig>(
    `/api/admin/lottery-categories/${encodeURIComponent(code)}`,
    {
      body: payload,
      method: 'PUT',
    },
  );
}

export function deleteLotteryCategory(code: string) {
  return requestJson<LotteryCategoryConfig>(
    `/api/admin/lottery-categories/${encodeURIComponent(code)}`,
    {
      method: 'DELETE',
    },
  );
}

export function createLottery(payload: LotteryKind) {
  return requestJson<LotteryKind>('/api/admin/lotteries', {
    body: payload,
    method: 'POST',
  });
}

export function updateLottery(id: string, payload: LotteryKind) {
  return requestJson<LotteryKind>(`/api/admin/lotteries/${encodeURIComponent(id)}`, {
    body: payload,
    method: 'PUT',
  });
}

export function deleteLottery(id: string) {
  return requestJson<LotteryKind>(`/api/admin/lotteries/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

export function setLotterySaleStatus(id: string, saleEnabled: boolean) {
  return requestJson<LotteryKind>(
    `/api/admin/lotteries/${encodeURIComponent(id)}/sale`,
    {
      body: { saleEnabled },
      method: 'PATCH',
    },
  );
}

export function fetchDrawSources(signal?: AbortSignal) {
  return requestJson<DrawSource[]>('/api/admin/draw-sources', { signal });
}

export function createDrawSource(payload: SaveDrawSourceRequest) {
  return requestJson<DrawSource>('/api/admin/draw-sources', {
    body: payload,
    method: 'POST',
  });
}

export function updateDrawSource(id: string, payload: SaveDrawSourceRequest) {
  return requestJson<DrawSource>(`/api/admin/draw-sources/${encodeURIComponent(id)}`, {
    body: payload,
    method: 'PUT',
  });
}

export function deleteDrawSource(id: string) {
  return requestJson<DrawSource>(`/api/admin/draw-sources/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

export function fetchDrawIssues(signal?: AbortSignal, query?: DrawIssueQuery) {
  const params = new URLSearchParams();
  if (query?.lotteryId) {
    params.set('lotteryId', query.lotteryId);
  }
  if (query?.page && query.page > 0) {
    params.set('page', String(query.page));
  }
  if (query?.pageSize && query.pageSize > 0) {
    params.set('pageSize', String(query.pageSize));
  }

  const path = params.toString()
    ? `/api/admin/draw-issues?${params.toString()}`
    : '/api/admin/draw-issues';

  return requestJson<DrawIssuePage>(path, { signal });
}

export function fetchLotteryDrawControls(signal?: AbortSignal) {
  return requestJson<LotteryDrawControl[]>('/api/admin/draw-controls', { signal });
}

export function fetchLotteryDrawControl(lotteryId: string, signal?: AbortSignal) {
  return requestJson<LotteryDrawControl>(
    `/api/admin/draw-controls/${encodeURIComponent(lotteryId)}`,
    { signal },
  );
}

export function saveLotteryDrawControl(
  lotteryId: string,
  payload: SaveLotteryDrawControlRequest,
) {
  return requestJson<LotteryDrawControl>(
    `/api/admin/draw-controls/${encodeURIComponent(lotteryId)}`,
    {
      body: payload,
      method: 'PUT',
    },
  );
}

export function createDrawIssue(payload: CreateDrawIssueRequest) {
  return requestJson<DrawIssue>('/api/admin/draw-issues', {
    body: payload,
    method: 'POST',
  });
}

export function generateNextDrawIssue(payload: GenerateDrawIssueRequest) {
  return requestJson<DrawIssue>('/api/admin/draw-issues/generate-next', {
    body: payload,
    method: 'POST',
  });
}

export function previewDrawIssueGeneration(payload: GenerateDrawIssuesRequest) {
  return requestJson<DrawIssueGenerationPreview[]>(
    '/api/admin/draw-issues/preview-generation',
    {
      body: payload,
      method: 'POST',
    },
  );
}

export function generateDrawIssueBatch(payload: GenerateDrawIssuesRequest) {
  return requestJson<DrawIssue[]>('/api/admin/draw-issues/generate-batch', {
    body: payload,
    method: 'POST',
  });
}

export function closeDrawIssue(id: string) {
  return requestJson<DrawIssue>(
    `/api/admin/draw-issues/${encodeURIComponent(id)}/close`,
    {
      method: 'PATCH',
    },
  );
}

export function drawIssueResult(id: string, payload: DrawIssueResultRequest) {
  return requestJson<DrawIssue>(
    `/api/admin/draw-issues/${encodeURIComponent(id)}/draw`,
    {
      body: payload,
      method: 'PATCH',
    },
  );
}

export function cancelDrawIssue(id: string) {
  return requestJson<DrawIssue>(
    `/api/admin/draw-issues/${encodeURIComponent(id)}/cancel`,
    {
      method: 'PATCH',
    },
  );
}

export function runDrawAutomation(payload: DrawAutomationRunRequest) {
  return requestJson<DrawAutomationRun>('/api/admin/draw-automation/run', {
    body: payload,
    method: 'POST',
  });
}

export function fetchDrawSchedulerStatus(signal?: AbortSignal) {
  return requestJson<DrawSchedulerStatus>('/api/admin/draw-scheduler/status', {
    signal,
  });
}

export function updateDrawSchedulerConfig(payload: DrawSchedulerConfig) {
  return requestJson<DrawSchedulerStatus>('/api/admin/draw-scheduler/config', {
    body: payload,
    method: 'PUT',
  });
}

export function fetchPlayRules(signal?: AbortSignal) {
  return requestJson<PlayRuleSummary[]>('/api/admin/play-rules', { signal });
}

export function evaluatePlayRule(payload: PlayRuleEvaluateRequest) {
  return requestJson<PlayRuleEvaluation>('/api/admin/play-rules/evaluate', {
    body: payload,
    method: 'POST',
  });
}

export function fetchOrders(signal?: AbortSignal, query?: OrderListQuery) {
  return requestJson<OrderDetail[]>(adminQueryPath('/api/admin/orders', query), { signal });
}

export function createOrder(payload: CreateOrderRequest) {
  return requestJson<OrderDetail>('/api/admin/orders', {
    body: payload,
    method: 'POST',
  });
}

export function cancelOrder(id: string) {
  return requestJson<OrderDetail>(`/api/admin/orders/${encodeURIComponent(id)}/cancel`, {
    method: 'PATCH',
  });
}

export function fetchSettlements(signal?: AbortSignal) {
  return requestJson<SettlementRun[]>('/api/admin/settlements', { signal });
}

export function fetchSettlement(id: string, signal?: AbortSignal) {
  return requestJson<SettlementRun>(
    `/api/admin/settlements/${encodeURIComponent(id)}`,
    { signal },
  );
}

export function settleDrawIssue(drawIssueId: string) {
  return requestJson<SettlementRun>(
    `/api/admin/settlements/draw-issues/${encodeURIComponent(drawIssueId)}`,
    {
      method: 'POST',
    },
  );
}
