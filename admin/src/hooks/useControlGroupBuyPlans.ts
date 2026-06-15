import { useCallback, useEffect, useState } from 'react';
import { fetchGroupBuyPlansByIssue } from '../api/client';
import type { GroupBuyPlan } from '../types/groupBuy';

export function useControlGroupBuyPlans(lotteryId: string | null, issue: string) {
  const [plans, setPlans] = useState<GroupBuyPlan[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);

  const refresh = useCallback(() => {
    setRefreshToken((current) => current + 1);
  }, []);

  useEffect(() => {
    const normalizedLotteryId = lotteryId?.trim() ?? '';
    const normalizedIssue = issue.trim();
    if (!normalizedLotteryId || !normalizedIssue) {
      setPlans([]);
      setLoading(false);
      setError(null);
      return undefined;
    }

    const controller = new AbortController();
    setLoading(true);
    setError(null);

    fetchGroupBuyPlansByIssue(controller.signal, {
      issue: normalizedIssue,
      lotteryId: normalizedLotteryId,
    })
      .then((nextPlans) => {
        if (!controller.signal.aborted) {
          setPlans(nextPlans);
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
  }, [issue, lotteryId, refreshToken]);

  return {
    error,
    loading,
    plans,
    refresh,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '合买认购记录读取失败';
}
