import { useCallback, useEffect, useState } from 'react';
import { cancelOrder, clearBetOrders, createOrder, fetchOrders } from '../api/client';
import type {
  CreateOrderRequest,
  OrderDetail,
  OrderListQuery,
  OrderPage,
} from '../types/orders';

export function useOrders(query: OrderListQuery = {}) {
  const [orderPage, setOrderPage] = useState<OrderPage>(emptyPage);
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
      .then(setOrderPage)
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
  }, [query.includeRobotData, query.page, query.pageSize, query.userId, refreshToken]);

  const create = useCallback(
    async (payload: CreateOrderRequest) => {
      setSaving(true);
      setError(null);
      try {
        const created = await createOrder(payload);
        refresh();
        return created;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [refresh],
  );

  const cancel = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const cancelled = await cancelOrder(id);
      setOrderPage((current) => replacePageItem(current, cancelled));
      return cancelled;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const clearRecords = useCallback(
    async () => {
      setSaving(true);
      setError(null);
      try {
        const result = await clearBetOrders();
        refresh();
        return result;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [refresh],
  );

  return {
    cancel,
    clearRecords,
    create,
    error,
    loading,
    orderPage,
    orders: orderPage.items,
    refresh,
    saving,
  };
}

const emptyPage: OrderPage = {
  items: [],
  page: 1,
  pageSize: 20,
  totalCount: 0,
  totalPages: 0,
};

function replacePageItem(page: OrderPage, item: OrderDetail): OrderPage {
  return {
    ...page,
    items: page.items.map((current) =>
      current.id === item.id ? item : current,
    ),
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
