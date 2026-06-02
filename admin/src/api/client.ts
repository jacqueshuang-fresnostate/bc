import type { ApiEnvelope, DashboardSummary, LotteryKind } from '../types/dashboard';
import type { DrawSource } from '../types/dashboard';
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
  PlayRuleEvaluateRequest,
  PlayRuleEvaluation,
  PlayRuleSummary,
} from '../types/playRules';
import type { CreateOrderRequest, OrderDetail } from '../types/orders';
import type { SettlementRun } from '../types/settlements';

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
