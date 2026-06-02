import { useCallback, useEffect, useState } from 'react';
import { fetchSettlements, settleDrawIssue } from '../api/client';
import type { SettlementRun } from '../types/settlements';

export function useSettlements() {
  const [settlements, setSettlements] = useState<SettlementRun[]>([]);
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

    fetchSettlements(controller.signal)
      .then(setSettlements)
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
  }, [refreshToken]);

  const settle = useCallback(async (drawIssueId: string) => {
    setSaving(true);
    setError(null);
    try {
      const settlement = await settleDrawIssue(drawIssueId);
      setSettlements((current) => [settlement, ...current]);
      return settlement;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    error,
    loading,
    refresh,
    saving,
    settle,
    settlements,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
