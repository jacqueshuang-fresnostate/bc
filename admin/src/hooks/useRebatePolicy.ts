import { useCallback, useEffect, useState } from 'react';
import {
  fetchAgentRebateRecords,
  fetchAgentRebateStatistics,
  fetchInvitePolicy,
  fetchRegistrationConfig,
  processAgentRebateWithdrawal,
  updateInvitePolicy,
} from '../api/client';
import type { RegistrationConfig } from '../types/dashboard';
import type {
  AgentRebatePage,
  AgentRebateQuery,
  AgentRebateRecordPage,
  AgentRebateWithdrawalRequest,
  InvitePolicySummary,
  InvitePolicyUpdateRequest,
} from '../types/rebates';

export function useRebatePolicy(statisticsQuery: AgentRebateQuery = {}) {
  const statisticsPage = statisticsQuery.page;
  const statisticsPageSize = statisticsQuery.pageSize;
  const [policy, setPolicy] = useState<InvitePolicySummary | null>(null);
  const [registration, setRegistration] = useState<RegistrationConfig | null>(null);
  const [statistics, setStatistics] = useState<AgentRebatePage>(() => emptyPage());
  const [records, setRecords] = useState<AgentRebateRecordPage>(() => emptyPage());
  const [loading, setLoading] = useState(true);
  const [recordsLoading, setRecordsLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);

  const refresh = useCallback(() => {
    setRefreshToken((current) => current + 1);
  }, []);

  useEffect(() => {
    const controller = new AbortController();

    setLoading(true);
    setError(null);

    Promise.all([
      fetchInvitePolicy(controller.signal),
      fetchRegistrationConfig(controller.signal),
      fetchAgentRebateStatistics(controller.signal, {
        page: statisticsPage,
        pageSize: statisticsPageSize,
      }),
    ])
      .then(([nextPolicy, nextRegistration, nextStatistics]) => {
        setPolicy(nextPolicy);
        setRegistration(nextRegistration);
        setStatistics(nextStatistics);
      })
      .catch((requestError: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(requestError));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => {
      controller.abort();
    };
  }, [refreshToken, statisticsPage, statisticsPageSize]);

  const save = useCallback(async (payload: InvitePolicyUpdateRequest) => {
    setSaving(true);
    setError(null);
    try {
      const saved = await updateInvitePolicy(payload);
      setPolicy(saved);
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const loadRecords = useCallback(async (agentUserId: string, query: AgentRebateQuery) => {
    setRecordsLoading(true);
    setError(null);
    try {
      const nextRecords = await fetchAgentRebateRecords(agentUserId, undefined, query);
      setRecords(nextRecords);
      return nextRecords;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setRecordsLoading(false);
    }
  }, []);

  const withdraw = useCallback(
    async (agentUserId: string, payload: AgentRebateWithdrawalRequest) => {
      setSaving(true);
      setError(null);
      try {
        const entry = await processAgentRebateWithdrawal(agentUserId, payload);
        refresh();
        return entry;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [refresh],
  );

  return {
    error,
    loadRecords,
    loading,
    policy,
    records,
    recordsLoading,
    refresh,
    registration,
    save,
    saving,
    statistics,
    withdraw,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}

function emptyPage<T>(): AgentRebatePage & AgentRebateRecordPage {
  return {
    items: [],
    page: 1,
    pageSize: 20,
    totalCount: 0,
    totalPages: 0,
  } as AgentRebatePage & AgentRebateRecordPage;
}
