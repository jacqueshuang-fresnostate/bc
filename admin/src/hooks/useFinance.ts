import { useCallback, useEffect, useState } from 'react';
import {
  approveWithdrawalOrder,
  clearLedgerEntries,
  clearRechargeOrders,
  clearWithdrawalOrders,
  confirmRechargeOrder,
  createManualBalanceAdjustment,
  exportRechargeOrders,
  fetchFinanceOverview,
  fetchFinancialAccounts,
  fetchLedgerEntries,
  fetchRechargeOrders,
  fetchWithdrawalOrders,
  rejectWithdrawalOrder,
} from '../api/client';
import type {
  AdminFinancialAccountSummary,
  AdminRechargeOrderSummary,
  AdminWithdrawalOrderSummary,
  FinanceOverview,
  FinancePage,
  FinancePageQuery,
  ConfirmRechargeOrderRequest,
  LedgerEntry,
  ManualBalanceAdjustmentRequest,
} from '../types/finance';

interface UseFinanceOptions {
  accountQuery: FinancePageQuery;
  includeRobotData: boolean;
  ledgerQuery: FinancePageQuery;
  rechargeQuery: FinancePageQuery;
  withdrawalQuery: FinancePageQuery;
}

export function useFinance({
  accountQuery,
  includeRobotData,
  ledgerQuery,
  rechargeQuery,
  withdrawalQuery,
}: UseFinanceOptions) {
  const [overview, setOverview] = useState<FinanceOverview | null>(null);
  const [accounts, setAccounts] = useState<FinancePage<AdminFinancialAccountSummary>>(
    emptyPage,
  );
  const [ledgerEntries, setLedgerEntries] = useState<FinancePage<LedgerEntry>>(emptyPage);
  const [rechargeOrders, setRechargeOrders] =
    useState<FinancePage<AdminRechargeOrderSummary>>(emptyPage);
  const [withdrawalOrders, setWithdrawalOrders] =
    useState<FinancePage<AdminWithdrawalOrderSummary>>(emptyPage);
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
      fetchFinanceOverview(controller.signal, { includeRobotData }),
      fetchFinancialAccounts(controller.signal, accountQuery),
      fetchLedgerEntries(controller.signal, ledgerQuery),
      fetchRechargeOrders(controller.signal, rechargeQuery),
      fetchWithdrawalOrders(controller.signal, withdrawalQuery),
    ])
      .then(
        ([
          nextOverview,
          nextAccounts,
          nextEntries,
          nextRechargeOrders,
          nextWithdrawalOrders,
        ]) => {
          setOverview(nextOverview);
          setAccounts(nextAccounts);
          setLedgerEntries(nextEntries);
          setRechargeOrders(nextRechargeOrders);
          setWithdrawalOrders(nextWithdrawalOrders);
        },
      )
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
  }, [
    accountQuery.page,
    accountQuery.pageSize,
    accountQuery.includeRobotData,
    accountQuery.userId,
    accountQuery.username,
    includeRobotData,
    ledgerQuery.page,
    ledgerQuery.pageSize,
    ledgerQuery.includeRobotData,
    ledgerQuery.userId,
    ledgerQuery.kind,
    rechargeQuery.page,
    rechargeQuery.pageSize,
    refreshToken,
    withdrawalQuery.page,
    withdrawalQuery.pageSize,
  ]);

  const adjustBalance = useCallback(
    async (payload: ManualBalanceAdjustmentRequest) => {
      setSaving(true);
      setError(null);
      try {
        const entry = await createManualBalanceAdjustment(payload);
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

  const confirmRecharge = useCallback(
    async (id: string, payload: ConfirmRechargeOrderRequest = {}) => {
      setSaving(true);
      setError(null);
      try {
        const order = await confirmRechargeOrder(id, payload);
        refresh();
        return order;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [refresh],
  );

  const approveWithdrawal = useCallback(
    async (id: string) => {
      setSaving(true);
      setError(null);
      try {
        const order = await approveWithdrawalOrder(id);
        refresh();
        return order;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [refresh],
  );

  const rejectWithdrawal = useCallback(
    async (id: string) => {
      setSaving(true);
      setError(null);
      try {
        const order = await rejectWithdrawalOrder(id);
        refresh();
        return order;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [refresh],
  );

  const exportRechargeRecords = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      return await exportRechargeOrders();
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const clearRechargeRecords = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const result = await clearRechargeOrders();
      refresh();
      return result;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, [refresh]);

  const clearLedgerRecords = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const result = await clearLedgerEntries();
      refresh();
      return result;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, [refresh]);

  const clearWithdrawalRecords = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const result = await clearWithdrawalOrders();
      refresh();
      return result;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, [refresh]);

  return {
    accounts,
    adjustBalance,
    approveWithdrawal,
    clearLedgerRecords,
    clearRechargeRecords,
    clearWithdrawalRecords,
    confirmRecharge,
    error,
    exportRechargeRecords,
    ledgerEntries,
    loading,
    overview,
    rechargeOrders,
    refresh,
    rejectWithdrawal,
    saving,
    withdrawalOrders,
  };
}

const emptyPage = {
  items: [],
  page: 1,
  pageSize: 20,
  totalCount: 0,
  totalPages: 0,
};

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
