import { useCallback, useEffect, useState } from 'react';
import {
  fetchDrawIssues,
  fetchDrawSchedulerStatus,
  fetchLotteries,
  fetchLotteryDrawControls,
  fetchOrders,
  saveLotteryDrawControl,
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

  const refresh = useCallback(() => {
    setRefreshToken((current) => current + 1);
  }, []);

  useEffect(() => {
    const controller = new AbortController();

    setLoading(true);
    setError(null);

    Promise.all([
      fetchLotteries(controller.signal),
      fetchDrawIssues(controller.signal),
      fetchLotteryDrawControls(controller.signal),
      fetchDrawSchedulerStatus(controller.signal),
      fetchOrders(controller.signal),
    ])
      .then(([lotteryList, drawIssuePage, controls, scheduler, orderList]) => {
        setLotteries(lotteryList);
        setIssues(drawIssuePage.items);
        setDrawControls(controls);
        setSchedulerStatus(scheduler);
        setOrders(orderList);
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

    const intervalId = window.setInterval(refresh, pollIntervalMs);
    return () => {
      window.clearInterval(intervalId);
    };
  }, [pollIntervalMs, refresh]);

  useEffect(() => {
    const onWindowFocus = () => {
      refresh();
    };
    const onVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        refresh();
      }
    };

    window.addEventListener('focus', onWindowFocus);
    window.addEventListener('visibilitychange', onVisibilityChange);

    return () => {
      window.removeEventListener('focus', onWindowFocus);
      window.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, [refresh]);

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

  return {
    drawControls,
    error,
    issues,
    loading,
    lotteries,
    orders,
    refresh,
    schedulerStatus,
    saveDrawControl,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
