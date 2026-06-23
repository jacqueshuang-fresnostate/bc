import { useCallback, useEffect, useRef, useState } from 'react';
import {
  fetchDrawIssues,
  fetchDrawSchedulerStatus,
  fetchLotteries,
  fetchLotteryDrawControls,
  fetchOrders,
  setLotteryAvoidWinningStatus,
  saveLotteryDrawControl,
  syncLotteryDrawSource,
} from '../api/client';
import type { LotteryKind } from '../types/dashboard';
import type {
  DrawIssue,
  LotteryDrawControl,
  SaveLotteryDrawControlRequest,
} from '../types/draws';
import type { OrderDetail } from '../types/orders';
import type { DrawSchedulerStatus } from '../types/scheduler';

export function useLotteryConsole(pollIntervalMs = 10_000) {
  const [lotteries, setLotteries] = useState<LotteryKind[]>([]);
  const [issues, setIssues] = useState<DrawIssue[]>([]);
  const [orders, setOrders] = useState<OrderDetail[]>([]);
  const [drawControls, setDrawControls] = useState<LotteryDrawControl[]>([]);
  const [schedulerStatus, setSchedulerStatus] =
    useState<DrawSchedulerStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);
  const lastRefreshAtRef = useRef(0);

  const refresh = useCallback(() => {
    lastRefreshAtRef.current = Date.now();
    setRefreshToken((current) => current + 1);
  }, []);

  const refreshWhenVisible = useCallback(() => {
    if (document.visibilityState === 'hidden') {
      return;
    }
    refresh();
  }, [refresh]);

  const refreshWhenVisibleThrottled = useCallback(() => {
    if (document.visibilityState === 'hidden') {
      return;
    }
    if (Date.now() - lastRefreshAtRef.current < 2500) {
      return;
    }
    refresh();
  }, [refresh]);

  useEffect(() => {
    const controller = new AbortController();

    setLoading(true);
    setError(null);

    Promise.all([
      fetchLotteries(controller.signal),
      fetchDrawIssues(controller.signal, { page: 1, pageSize: 300 }),
      fetchLotteryDrawControls(controller.signal),
      fetchDrawSchedulerStatus(controller.signal),
      fetchOrders(controller.signal, { includeRobotData: true, page: 1, pageSize: 300 }),
    ])
      .then(([lotteryList, drawIssuePage, controls, scheduler, orderPage]) => {
        setLotteries(lotteryList);
        setIssues(drawIssuePage.items);
        setDrawControls(controls);
        setSchedulerStatus(scheduler);
        setOrders(orderPage.items);
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

  useEffect(() => {
    if (pollIntervalMs <= 0) {
      return undefined;
    }

    const intervalId = window.setInterval(refreshWhenVisible, pollIntervalMs);
    return () => {
      window.clearInterval(intervalId);
    };
  }, [pollIntervalMs, refreshWhenVisible]);

  useEffect(() => {
    const onWindowFocus = () => {
      refreshWhenVisibleThrottled();
    };
    const onVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        refreshWhenVisibleThrottled();
      }
    };

    window.addEventListener('focus', onWindowFocus);
    window.addEventListener('visibilitychange', onVisibilityChange);

    return () => {
      window.removeEventListener('focus', onWindowFocus);
      window.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, [refreshWhenVisibleThrottled]);

  const saveDrawControl = useCallback(
    async (lotteryId: string, payload: SaveLotteryDrawControlRequest) => {
      const saved = await saveLotteryDrawControl(lotteryId, payload);
      setDrawControls((current) => {
        const exists = current.some((control) => control.lotteryId === saved.lotteryId);
        if (exists) {
          return current.map((control) =>
            control.lotteryId === saved.lotteryId ? saved : control,
          );
        }
        return [...current, saved];
      });
      return saved;
    },
    [],
  );

  const setAvoidWinningStatus = useCallback(
    async (lotteryId: string, avoidWinningEnabled: boolean) => {
      const updated = await setLotteryAvoidWinningStatus(
        lotteryId,
        avoidWinningEnabled,
      );
      setLotteries((current) =>
        current.map((lottery) => (lottery.id === lotteryId ? updated : lottery)),
      );
      return updated;
    },
    [],
  );

  const syncDrawSource = useCallback(async (lotteryId: string) => {
    const result = await syncLotteryDrawSource(lotteryId);
    refresh();
    return result;
  }, [refresh]);

  return {
    drawControls,
    error,
    issues,
    loading,
    lotteries,
    orders,
    refresh,
    schedulerStatus,
    setAvoidWinningStatus,
    saveDrawControl,
    syncDrawSource,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
