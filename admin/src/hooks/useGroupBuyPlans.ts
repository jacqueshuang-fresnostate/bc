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
import type {
  AddGroupBuyParticipantRequest,
  CreateGroupBuyPlanRequest,
  GroupBuyPlan,
  GroupBuyPlanSummary,
  UpdateGroupBuyPlanRequest,
} from '../types/groupBuy';

export function useGroupBuyPlans() {
  const [lotteries, setLotteries] = useState<LotteryKind[]>([]);
  const [plans, setPlans] = useState<GroupBuyPlanSummary[]>([]);
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
      fetchGroupBuyPlans(controller.signal),
      fetchLotteries(controller.signal),
      fetchUsers(controller.signal),
      fetchDrawIssues(controller.signal, { pageSize: 300 }),
    ])
      .then(async ([nextPlans, nextLotteries, nextUsers, nextDrawIssuePage]) => {
        if (controller.signal.aborted) {
          return;
        }
        setPlans(nextPlans);
        setLotteries(nextLotteries);
        setUsers(nextUsers);
        setDrawIssues(nextDrawIssuePage.items);

        const selectedId = selectedPlan?.id ?? nextPlans[0]?.id;
        if (!selectedId) {
          setSelectedPlan(null);
          return;
        }

        if (nextPlans.some((plan) => plan.id === selectedId)) {
          const detail = await fetchGroupBuyPlan(selectedId, controller.signal);
          if (!controller.signal.aborted) {
            setSelectedPlan(detail);
          }
        } else {
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
  }, [refreshToken]);

  const loadPlan = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const detail = await fetchGroupBuyPlan(id);
      setSelectedPlan(detail);
      setPlans((current) => upsertById(current, summaryFromPlan(detail)));
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
      setSelectedPlan(created);
      setPlans((current) => upsertById(current, summaryFromPlan(created)));
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
        setPlans((current) => upsertById(current, summaryFromPlan(updated)));
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
        setPlans((current) => upsertById(current, summaryFromPlan(updated)));
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
    plans,
    refresh,
    saving,
    selectedPlan,
    update,
    users,
  };
}

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
  };
}

function upsertById<T extends { id: string }>(items: T[], item: T) {
  return items.some((current) => current.id === item.id)
    ? items.map((current) => (current.id === item.id ? item : current))
    : [...items, item];
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
