import { useCallback, useEffect, useState } from 'react';
import {
  addGroupBuyParticipant,
  createGroupBuyPlan,
  fetchDrawIssues,
  fetchGroupBuyPlan,
  fetchGroupBuyPlans,
  fetchLotteries,
  fetchUsers,
  updateGroupBuyPlan,
} from '../api/client';
import type { LotteryKind, UserSummary } from '../types/dashboard';
import type { DrawIssue } from '../types/draws';
import type { FinancePage, FinancePageQuery } from '../types/finance';
import type {
  AddGroupBuyParticipantRequest,
  CreateGroupBuyPlanRequest,
  GroupBuyPlan,
  GroupBuyPlanSummary,
  UpdateGroupBuyPlanRequest,
} from '../types/groupBuy';

interface UseGroupBuyPlansOptions {
  planQuery: FinancePageQuery;
}

export function useGroupBuyPlans({ planQuery }: UseGroupBuyPlansOptions) {
  const [lotteries, setLotteries] = useState<LotteryKind[]>([]);
  const [planPage, setPlanPage] =
    useState<FinancePage<GroupBuyPlanSummary>>(emptyPage);
  const [drawIssues, setDrawIssues] = useState<DrawIssue[]>([]);
  const [selectedPlan, setSelectedPlan] = useState<GroupBuyPlan | null>(null);
  const [users, setUsers] = useState<UserSummary[]>([]);
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
      fetchGroupBuyPlans(controller.signal, planQuery),
      fetchLotteries(controller.signal),
      fetchUsers(controller.signal),
      fetchDrawIssues(controller.signal, { pageSize: 300 }),
    ])
      .then(async ([nextPlanPage, nextLotteries, nextUsers, nextDrawIssuePage]) => {
        if (controller.signal.aborted) {
          return;
        }
        const nextPlans = nextPlanPage.items;
        setPlanPage(nextPlanPage);
        setLotteries(nextLotteries);
        setUsers(nextUsers);
        setDrawIssues(nextDrawIssuePage.items);

        const selectedId = selectedPlan?.id;
        if (!selectedId || !nextPlans.some((plan) => plan.id === selectedId)) {
          setSelectedPlan(null);
        }
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
  }, [
    planQuery.includeRobotData,
    planQuery.page,
    planQuery.pageSize,
    refreshToken,
  ]);

  const loadPlan = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const detail = await fetchGroupBuyPlan(id);
      setSelectedPlan(detail);
      setPlanPage((current) => upsertPageItem(current, summaryFromPlan(detail)));
      return detail;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const create = useCallback(async (payload: CreateGroupBuyPlanRequest) => {
    setSaving(true);
    setError(null);
    try {
      const created = await createGroupBuyPlan(payload);
      setPlanPage((current) =>
        upsertPageItem(current, summaryFromPlan(created), true),
      );
      return created;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const update = useCallback(
    async (id: string, payload: UpdateGroupBuyPlanRequest) => {
      setSaving(true);
      setError(null);
      try {
        const updated = await updateGroupBuyPlan(id, payload);
        setSelectedPlan(updated);
        setPlanPage((current) => upsertPageItem(current, summaryFromPlan(updated)));
        return updated;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  const addParticipant = useCallback(
    async (id: string, payload: AddGroupBuyParticipantRequest) => {
      setSaving(true);
      setError(null);
      try {
        const updated = await addGroupBuyParticipant(id, payload);
        setSelectedPlan(updated);
        setPlanPage((current) => upsertPageItem(current, summaryFromPlan(updated)));
        return updated;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  return {
    addParticipant,
    create,
    drawIssues,
    error,
    loadPlan,
    loading,
    lotteries,
    planPage,
    plans: planPage.items,
    refresh,
    saving,
    selectedPlan,
    update,
    users,
  };
}

const emptyPage: FinancePage<GroupBuyPlanSummary> = {
  items: [],
  page: 1,
  pageSize: 20,
  totalCount: 0,
  totalPages: 0,
};

function summaryFromPlan(plan: GroupBuyPlan): GroupBuyPlanSummary {
  return {
    filledAmountMinor: plan.filledAmountMinor,
    id: plan.id,
    initiatorUserId: plan.initiatorUserId,
    initiatorUsername: plan.initiatorUsername,
    lotteryId: plan.lotteryId,
    lotteryName: plan.lotteryName,
    shareCount: plan.shareCount,
    status: plan.status,
    totalAmountMinor: plan.totalAmountMinor,
    orderId: plan.orderId,
    issue: plan.issue,
    ruleCode: plan.ruleCode,
    title: plan.title,
    createdAt: plan.createdAt,
  };
}

function upsertById<T extends { id: string }>(items: T[], item: T) {
  return items.some((current) => current.id === item.id)
    ? items.map((current) => (current.id === item.id ? item : current))
    : [...items, item];
}

function upsertPageItem<T extends { id: string }>(
  page: FinancePage<T>,
  item: T,
  countNewItem = false,
): FinancePage<T> {
  const exists = page.items.some((current) => current.id === item.id);
  const items = upsertById(page.items, item);
  const totalCount = exists || !countNewItem ? page.totalCount : page.totalCount + 1;
  const totalPages =
    page.pageSize <= 0 ? page.totalPages : Math.ceil(totalCount / page.pageSize);

  return {
    ...page,
    items,
    totalCount,
    totalPages,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
