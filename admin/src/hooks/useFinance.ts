import { useCallback, useEffect, useState } from 'react';
import {
  approveWithdrawalOrder,
  confirmRechargeOrder,
  createManualBalanceAdjustment,
  fetchFinanceOverview,
  fetchFinancialAccounts,
  fetchLedgerEntries,
  fetchRechargeOrders,
  fetchWithdrawalOrders,
  rejectWithdrawalOrder,
} from '../api/client';
import type {
  AdminFinancialAccountSummary,
  FinanceOverview,
  FinancePage,
  FinancePageQuery,
  LedgerEntry,
  ManualBalanceAdjustmentRequest,
  RechargeOrderSummary,
  WithdrawalOrderSummary,
} from '../types/finance';

interface UseFinanceOptions {
  accountQuery: FinancePageQuery;
  ledgerQuery: FinancePageQuery;
  rechargeQuery: FinancePageQuery;
  withdrawalQuery: FinancePageQuery;
}

export function useFinance({
  accountQuery,
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
    useState<FinancePage<RechargeOrderSummary>>(emptyPage);
  const [withdrawalOrders, setWithdrawalOrders] =
    useState<FinancePage<WithdrawalOrderSummary>>(emptyPage);
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
      fetchFinanceOverview(controller.signal),
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
    ledgerQuery.page,
    ledgerQuery.pageSize,
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
    async (id: string) => {
      setSaving(true);
      setError(null);
      try {
        const order = await confirmRechargeOrder(id);
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

  return {
    accounts,
    adjustBalance,
    approveWithdrawal,
    confirmRecharge,
    error,
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
