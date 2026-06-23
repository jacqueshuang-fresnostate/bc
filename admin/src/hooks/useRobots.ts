import { useCallback, useEffect, useState } from 'react';
import {
  createRobot,
  deleteRobot,
  fetchLotteries,
  fetchRobotSchedulerStatus,
  fetchRobots,
  runGroupBuyRobots,
  setRobotStatus,
  updateRobot,
  updateRobotSchedulerConfig,
} from '../api/client';
import type { LotteryKind } from '../types/dashboard';
import type {
  GroupBuyRobotRun,
  RobotConfigPayload,
  RobotConfigSummary,
  RobotSchedulerConfig,
  RobotSchedulerStatus,
  RobotStatus,
} from '../types/robots';

export function useRobots() {
  const [lotteries, setLotteries] = useState<LotteryKind[]>([]);
  const [robots, setRobots] = useState<RobotConfigSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [running, setRunning] = useState(false);
  const [lastGroupBuyRun, setLastGroupBuyRun] = useState<GroupBuyRobotRun | null>(null);
  const [robotSchedulerStatus, setRobotSchedulerStatus] =
    useState<RobotSchedulerStatus | null>(null);
  const [robotSchedulerSaving, setRobotSchedulerSaving] = useState(false);
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
      fetchRobots(controller.signal),
      fetchLotteries(controller.signal),
      fetchRobotSchedulerStatus(controller.signal),
    ])
      .then(([nextRobots, nextLotteries, nextRobotSchedulerStatus]) => {
        setRobots(nextRobots);
        setLotteries(nextLotteries);
        setRobotSchedulerStatus(nextRobotSchedulerStatus);
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

  const save = useCallback(
    async (payload: RobotConfigPayload, existingId?: string) => {
      setSaving(true);
      setError(null);
      try {
        const saved = existingId
          ? await updateRobot(existingId, payload)
          : await createRobot(payload);
        setRobots((current) => upsertById(current, saved));
        return saved;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  const remove = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const deleted = await deleteRobot(id);
      setRobots((current) => current.filter((robot) => robot.id !== id));
      return deleted;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const changeStatus = useCallback(async (id: string, status: RobotStatus) => {
    setSaving(true);
    setError(null);
    try {
      const saved = await setRobotStatus(id, { status });
      setRobots((current) => upsertById(current, saved));
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const executeGroupBuyRobots = useCallback(async () => {
    setRunning(true);
    setError(null);
    try {
      const run = await runGroupBuyRobots();
      setLastGroupBuyRun(run);
      refresh();
      return run;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setRunning(false);
    }
  }, [refresh]);

  const saveRobotSchedulerConfig = useCallback(
    async (payload: RobotSchedulerConfig) => {
      setRobotSchedulerSaving(true);
      setError(null);
      try {
        const status = await updateRobotSchedulerConfig(payload);
        setRobotSchedulerStatus(status);
        return status;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setRobotSchedulerSaving(false);
      }
    },
    [],
  );

  return {
    changeStatus,
    error,
    executeGroupBuyRobots,
    lastGroupBuyRun,
    loading,
    lotteries,
    remove,
    refresh,
    robots,
    robotSchedulerSaving,
    robotSchedulerStatus,
    running,
    save,
    saveRobotSchedulerConfig,
    saving,
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
