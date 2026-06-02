import { useCallback, useEffect, useState } from 'react';
import { fetchDrawSchedulerStatus } from '../api/client';
import type { DrawSchedulerStatus } from '../types/scheduler';

export function useDrawScheduler() {
  const [status, setStatus] = useState<DrawSchedulerStatus | null>(null);
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

  return {
    error,
    loading,
    refresh,
    status,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
