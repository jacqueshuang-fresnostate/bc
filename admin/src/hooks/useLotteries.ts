import { useCallback, useEffect, useState } from 'react';
import {
  createLottery,
  deleteLottery,
  fetchLotteries,
  setLotterySaleStatus,
  updateLottery,
} from '../api/client';
import type { LotteryKind } from '../types/dashboard';

export function useLotteries() {
  const [lotteries, setLotteries] = useState<LotteryKind[]>([]);
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

    fetchLotteries(controller.signal)
      .then(setLotteries)
      .catch((requestError: unknown) => {
        if (controller.signal.aborted) {
          return;
        }
        setError(errorMessage(requestError));
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

  const create = useCallback(async (payload: LotteryKind) => {
    setSaving(true);
    setError(null);
    try {
      const created = await createLottery(payload);
      setLotteries((current) => [...current, created]);
      return created;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const update = useCallback(async (id: string, payload: LotteryKind) => {
    setSaving(true);
    setError(null);
    try {
      const updated = await updateLottery(id, payload);
      setLotteries((current) =>
        current.map((lottery) => (lottery.id === id ? updated : lottery)),
      );
      return updated;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const remove = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const deleted = await deleteLottery(id);
      setLotteries((current) => current.filter((lottery) => lottery.id !== id));
      return deleted;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const setSaleStatus = useCallback(async (id: string, saleEnabled: boolean) => {
    setSaving(true);
    setError(null);
    try {
      const updated = await setLotterySaleStatus(id, saleEnabled);
      setLotteries((current) =>
        current.map((lottery) => (lottery.id === id ? updated : lottery)),
      );
      return updated;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    create,
    error,
    loading,
    lotteries,
    refresh,
    remove,
    saving,
    setSaleStatus,
    update,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
