import { useCallback, useEffect, useState } from 'react';
import { fetchDashboard } from '../api/client';
import type { DashboardSummary } from '../types/dashboard';

export function useDashboard() {
  const [data, setData] = useState<DashboardSummary | null>(null);
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

    fetchDashboard(controller.signal)
      .then((summary) => {
        setData(summary);
      })
      .catch((requestError: unknown) => {
        if (controller.signal.aborted) {
          return;
        }
        const message =
          requestError instanceof Error ? requestError.message : '接口请求失败';
        setError(message);
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

  return { data, loading, error, refresh };
}
