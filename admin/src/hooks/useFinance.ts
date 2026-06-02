import { useCallback, useEffect, useState } from 'react';
import {
  createManualBalanceAdjustment,
  fetchFinancialAccounts,
  fetchLedgerEntries,
} from '../api/client';
import type {
  FinancialAccountSummary,
  LedgerEntry,
  ManualBalanceAdjustmentRequest,
} from '../types/finance';

export function useFinance() {
  const [accounts, setAccounts] = useState<FinancialAccountSummary[]>([]);
  const [ledgerEntries, setLedgerEntries] = useState<LedgerEntry[]>([]);
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

    Promise.all([
      fetchFinancialAccounts(controller.signal),
      fetchLedgerEntries(controller.signal),
    ])
      .then(([nextAccounts, nextEntries]) => {
        setAccounts(nextAccounts);
        setLedgerEntries(nextEntries);
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
  }, [refreshToken]);

  const adjustBalance = useCallback(async (payload: ManualBalanceAdjustmentRequest) => {
    setSaving(true);
    setError(null);
    try {
      const entry = await createManualBalanceAdjustment(payload);
      setLedgerEntries((current) => [entry, ...current]);
      const nextAccounts = await fetchFinancialAccounts();
      setAccounts(nextAccounts);
      return entry;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    accounts,
    adjustBalance,
    error,
    ledgerEntries,
    loading,
    refresh,
    saving,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
