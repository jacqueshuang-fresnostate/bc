import { useCallback, useEffect, useState } from 'react';
import {
  createAdvertisement,
  deleteAdvertisement,
  fetchAdvertisements,
  updateAdvertisement,
} from '../api/client';
import type {
  AdvertisementSummary,
  SaveAdvertisementRequest,
} from '../types/advertisements';

export function useAdvertisements() {
  const [advertisements, setAdvertisements] = useState<AdvertisementSummary[]>([]);
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

    fetchAdvertisements(controller.signal)
      .then(setAdvertisements)
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
    async (payload: SaveAdvertisementRequest, existingId?: string) => {
      setSaving(true);
      setError(null);
      try {
        const saved = existingId
          ? await updateAdvertisement(existingId, payload)
          : await createAdvertisement(payload);
        setAdvertisements((current) => upsertById(current, saved));
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
      const deleted = await deleteAdvertisement(id);
      setAdvertisements((current) =>
        current.filter((advertisement) => advertisement.id !== id),
      );
      return deleted;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    advertisements,
    error,
    loading,
    refresh,
    remove,
    save,
    saving,
  };
}

function upsertById<T extends { id: string }>(items: T[], item: T) {
  return items.some((current) => current.id === item.id)
    ? items.map((current) => (current.id === item.id ? item : current))
    : [...items, item].sort((left, right) => left.id.localeCompare(right.id));
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
