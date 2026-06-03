import { useCallback, useEffect, useState } from 'react';
import {
  fetchDrawIssues,
  fetchLotteries,
  fetchLotteryDrawControls,
  saveLotteryDrawControl,
} from '../api/client';
import type { LotteryKind } from '../types/dashboard';
import type {
  DrawIssue,
  LotteryDrawControl,
  SaveLotteryDrawControlRequest,
} from '../types/draws';

export function useLotteryConsole(pollIntervalMs = 10_000) {
  const [lotteries, setLotteries] = useState<LotteryKind[]>([]);
  const [issues, setIssues] = useState<DrawIssue[]>([]);
  const [drawControls, setDrawControls] = useState<LotteryDrawControl[]>([]);
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
    ])
      .then(([lotteryList, drawIssues, controls]) => {
        setLotteries(lotteryList);
        setIssues(drawIssues);
        setDrawControls(controls);
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
    refresh,
    saveDrawControl,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
