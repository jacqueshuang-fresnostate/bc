import { useCallback, useEffect, useState } from 'react';
import {
  fetchAdmins,
  fetchSupportConversations,
  replySupportConversation,
  updateSupportConversation,
} from '../api/client';
import type { AdminSummary } from '../types/dashboard';
import type {
  SupportConversation,
  SupportReplyRequest,
  UpdateSupportConversationRequest,
} from '../types/support';

export function useSupportConversations() {
  const [admins, setAdmins] = useState<AdminSummary[]>([]);
  const [conversations, setConversations] = useState<SupportConversation[]>([]);
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

    Promise.all([
      fetchSupportConversations(controller.signal),
      fetchAdmins(controller.signal),
    ])
      .then(([nextConversations, nextAdmins]) => {
        setConversations(nextConversations);
        setAdmins(nextAdmins);
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

  const update = useCallback(
    async (id: string, payload: UpdateSupportConversationRequest) => {
      setSaving(true);
      setError(null);
      try {
        const updated = await updateSupportConversation(id, payload);
        setConversations((current) => upsertById(current, updated));
        return updated;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  const reply = useCallback(async (id: string, payload: SupportReplyRequest) => {
    setSaving(true);
    setError(null);
    try {
      const updated = await replySupportConversation(id, payload);
      setConversations((current) => upsertById(current, updated));
      return updated;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    admins,
    conversations,
    error,
    loading,
    refresh,
    reply,
    saving,
    update,
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
