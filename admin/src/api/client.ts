import type { ApiEnvelope, DashboardSummary, LotteryKind } from '../types/dashboard';
import type {
  PlayRuleEvaluateRequest,
  PlayRuleEvaluation,
  PlayRuleSummary,
} from '../types/playRules';

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

export function fetchPlayRules(signal?: AbortSignal) {
  return requestJson<PlayRuleSummary[]>('/api/admin/play-rules', { signal });
}

export function evaluatePlayRule(payload: PlayRuleEvaluateRequest) {
  return requestJson<PlayRuleEvaluation>('/api/admin/play-rules/evaluate', {
    body: payload,
    method: 'POST',
  });
}
