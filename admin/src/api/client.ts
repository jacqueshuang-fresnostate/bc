import type { ApiEnvelope, DashboardSummary, LotteryKind } from '../types/dashboard';
import type { DrawSource } from '../types/dashboard';
import type {
  AdminRole,
  AdminSummary,
  RegistrationConfig,
  StatusUpdateRequest,
  SystemSetting,
  UpdateSystemSettingRequest,
  UserSummary,
} from '../types/access';
import type {
  CreateDrawIssueRequest,
  DrawAutomationRun,
  DrawAutomationRunRequest,
  DrawIssue,
  DrawIssueGenerationPreview,
  DrawIssueResultRequest,
  GenerateDrawIssueRequest,
  GenerateDrawIssuesRequest,
} from '../types/draws';
import type {
  FinancialAccountSummary,
  LedgerEntry,
  ManualBalanceAdjustmentRequest,
} from '../types/finance';
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
import type { CreateOrderRequest, OrderDetail } from '../types/orders';
import type { DrawSchedulerStatus } from '../types/scheduler';
import type {
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

interface JsonRequestOptions {
  body?: unknown;
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';
  signal?: AbortSignal;
}

async function requestJson<T>(
  path: string,
  { body, method = 'GET', signal }: JsonRequestOptions = {},
): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    body: body === undefined ? undefined : JSON.stringify(body),
    headers: body === undefined ? undefined : { 'Content-Type': 'application/json' },
    method,
    signal,
  });
  const envelope = (await response.json()) as ApiEnvelope<T>;

  if (!response.ok || !envelope.success || envelope.data === null) {
    throw new Error(envelope.message || '接口请求失败');
  }

  return envelope.data;
}

export function fetchDashboard(signal?: AbortSignal) {
  return requestJson<DashboardSummary>('/api/admin/dashboard', { signal });
}

export function fetchFinancialAccounts(signal?: AbortSignal) {
  return requestJson<FinancialAccountSummary[]>('/api/admin/financial-accounts', {
    signal,
  });
}

export function fetchLedgerEntries(signal?: AbortSignal) {
  return requestJson<LedgerEntry[]>('/api/admin/ledger-entries', { signal });
}

export function createManualBalanceAdjustment(payload: ManualBalanceAdjustmentRequest) {
  return requestJson<LedgerEntry>('/api/admin/financial-adjustments', {
    body: payload,
    method: 'POST',
  });
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

export function createAdmin(payload: AdminSummary) {
  return requestJson<AdminSummary>('/api/admin/admins', {
    body: payload,
    method: 'POST',
  });
}

export function updateAdmin(id: string, payload: AdminSummary) {
  return requestJson<AdminSummary>(`/api/admin/admins/${encodeURIComponent(id)}`, {
    body: payload,
    method: 'PUT',
  });
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

export function fetchLotteries(signal?: AbortSignal) {
  return requestJson<LotteryKind[]>('/api/admin/lotteries', { signal });
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

export function fetchDrawIssues(signal?: AbortSignal) {
  return requestJson<DrawIssue[]>('/api/admin/draw-issues', { signal });
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

export function fetchPlayRules(signal?: AbortSignal) {
  return requestJson<PlayRuleSummary[]>('/api/admin/play-rules', { signal });
}

export function evaluatePlayRule(payload: PlayRuleEvaluateRequest) {
  return requestJson<PlayRuleEvaluation>('/api/admin/play-rules/evaluate', {
    body: payload,
    method: 'POST',
  });
}

export function fetchOrders(signal?: AbortSignal) {
  return requestJson<OrderDetail[]>('/api/admin/orders', { signal });
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
