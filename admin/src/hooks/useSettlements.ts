import { useCallback, useEffect, useState } from 'react';
import { fetchSettlements, settleDrawIssue } from '../api/client';
import type { FinancePageQuery } from '../types/finance';
import type { SettlementPage } from '../types/settlements';

export function useSettlements(query: FinancePageQuery = {}) {
  const [settlementPage, setSettlementPage] = useState<SettlementPage>(emptyPage);
  const [loading, setLoading] = useState(true);
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

    fetchSettlements(controller.signal, query)
      .then(setSettlementPage)
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
  }, [query.page, query.pageSize, refreshToken]);

  const settle = useCallback(
    async (drawIssueId: string) => {
      setSaving(true);
      setError(null);
      try {
        const settlement = await settleDrawIssue(drawIssueId);
        refresh();
        return settlement;
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
    loading,
    refresh,
    saving,
    settle,
    settlementPage,
    settlements: settlementPage.items,
  };
}

const emptyPage: SettlementPage = {
  items: [],
  page: 1,
  pageSize: 20,
  totalCount: 0,
  totalPages: 0,
};

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
