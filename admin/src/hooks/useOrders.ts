import { useCallback, useEffect, useState } from 'react';
import { cancelOrder, createOrder, fetchOrders } from '../api/client';
import type { CreateOrderRequest, OrderDetail, OrderListQuery } from '../types/orders';

export function useOrders(query: OrderListQuery = {}) {
  const [orders, setOrders] = useState<OrderDetail[]>([]);
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

    fetchOrders(controller.signal, query)
      .then(setOrders)
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
  }, [query.includeRobotData, refreshToken]);

  const create = useCallback(async (payload: CreateOrderRequest) => {
    setSaving(true);
    setError(null);
    try {
      const created = await createOrder(payload);
      setOrders((current) => [created, ...current]);
      return created;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const cancel = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const cancelled = await cancelOrder(id);
      setOrders((current) =>
        current.map((order) => (order.id === id ? cancelled : order)),
      );
      return cancelled;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    cancel,
    create,
    error,
    loading,
    orders,
    refresh,
    saving,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
