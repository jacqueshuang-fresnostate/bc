import type { ApiEnvelope, DashboardSummary } from '../types/dashboard';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

async function getJson<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, { signal });
  const envelope = (await response.json()) as ApiEnvelope<T>;

  if (!response.ok || !envelope.success || envelope.data === null) {
    throw new Error(envelope.message || '接口请求失败');
  }

  return envelope.data;
}

export function fetchDashboard(signal?: AbortSignal) {
  return getJson<DashboardSummary>('/api/admin/dashboard', signal);
}
