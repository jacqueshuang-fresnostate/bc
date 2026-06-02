import { useCallback, useEffect, useState } from 'react';
import { fetchDrawSchedulerStatus, updateDrawSchedulerConfig } from '../api/client';
import type { DrawSchedulerConfig, DrawSchedulerStatus } from '../types/scheduler';

export function useDrawScheduler() {
  const [status, setStatus] = useState<DrawSchedulerStatus | null>(null);
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

    fetchDrawSchedulerStatus(controller.signal)
      .then((nextStatus) => {
        setStatus(nextStatus);
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

  const saveConfig = useCallback(async (payload: DrawSchedulerConfig) => {
    setSaving(true);
    setError(null);
    try {
      const nextStatus = await updateDrawSchedulerConfig(payload);
      setStatus(nextStatus);
      return nextStatus;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    error,
    loading,
    refresh,
    saveConfig,
    saving,
    status,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
