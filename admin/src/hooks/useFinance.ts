import { useCallback, useEffect, useState } from 'react';
import {
  confirmRechargeOrder,
  createManualBalanceAdjustment,
  fetchFinancialAccounts,
  fetchLedgerEntries,
  fetchRechargeOrders,
} from '../api/client';
import type {
  FinancialAccountSummary,
  LedgerEntry,
  ManualBalanceAdjustmentRequest,
  RechargeOrderSummary,
} from '../types/finance';

export function useFinance() {
  const [accounts, setAccounts] = useState<FinancialAccountSummary[]>([]);
  const [ledgerEntries, setLedgerEntries] = useState<LedgerEntry[]>([]);
  const [rechargeOrders, setRechargeOrders] = useState<RechargeOrderSummary[]>([]);
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
      fetchRechargeOrders(controller.signal),
    ])
      .then(([nextAccounts, nextEntries, nextRechargeOrders]) => {
        setAccounts(nextAccounts);
        setLedgerEntries(nextEntries);
        setRechargeOrders(nextRechargeOrders);
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
      const [nextAccounts, nextRechargeOrders] = await Promise.all([
        fetchFinancialAccounts(),
        fetchRechargeOrders(),
      ]);
      setAccounts(nextAccounts);
      setRechargeOrders(nextRechargeOrders);
      return entry;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const confirmRecharge = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const order = await confirmRechargeOrder(id);
      const [nextAccounts, nextEntries, nextRechargeOrders] = await Promise.all([
        fetchFinancialAccounts(),
        fetchLedgerEntries(),
        fetchRechargeOrders(),
      ]);
      setAccounts(nextAccounts);
      setLedgerEntries(nextEntries);
      setRechargeOrders(nextRechargeOrders);
      return order;
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
    confirmRecharge,
    error,
    ledgerEntries,
    loading,
    rechargeOrders,
    refresh,
    saving,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
