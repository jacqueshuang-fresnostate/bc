import { useCallback, useEffect, useState } from 'react';
import {
  createLotteryCategory,
  deleteLotteryCategory,
  fetchLotteryCategories,
  updateLotteryCategory as updateLotteryCategoryApi,
} from '../api/client';
import type { LotteryCategoryConfig } from '../types/dashboard';

export function useLotteryCategories() {
  const [categories, setCategories] = useState<LotteryCategoryConfig[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshToken, setRefreshToken] = useState(0);
  const [saving, setSaving] = useState(false);

  const refresh = useCallback(() => {
    setRefreshToken((current) => current + 1);
  }, []);

  useEffect(() => {
    const controller = new AbortController();

    setLoading(true);
    setError(null);

    fetchLotteryCategories(controller.signal)
      .then(setCategories)
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

  const create = useCallback(async (payload: LotteryCategoryConfig) => {
    setSaving(true);
    setError(null);
    try {
      const created = await createLotteryCategory(payload);
      setCategories((current) => [...current, created].sort((a, b) => a.code.localeCompare(b.code)));
      return created;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const update = useCallback(async (code: string, payload: LotteryCategoryConfig) => {
    setSaving(true);
    setError(null);
    try {
      const updated = await updateLotteryCategoryApi(code, payload);
      setCategories((current) =>
        current
          .map((item) => (item.code === code ? updated : item))
          .sort((a, b) => a.code.localeCompare(b.code)),
      );
      return updated;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const remove = useCallback(async (code: string) => {
    setSaving(true);
    setError(null);
    try {
      const removed = await deleteLotteryCategory(code);
      setCategories((current) => current.filter((item) => item.code !== code));
      return removed;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    categories,
    create,
    error,
    loading,
    refresh,
    remove,
    saving,
    update,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
