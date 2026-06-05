import { useCallback, useEffect, useState } from 'react';
import {
  adminRealtimeUrl,
  fetchAdmins,
  fetchSupportConversations,
  replySupportConversation,
  updateSupportConversation,
} from '../api/client';
import { normalizeAdminRealtimeEvent } from '../types/realtime';
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

  useEffect(() => {
    let socket: WebSocket | null = null;
    let reconnectTimer: number | undefined;
    let stopped = false;

    const scheduleReconnect = () => {
      if (stopped || reconnectTimer !== undefined) {
        return;
      }
      reconnectTimer = window.setTimeout(() => {
        reconnectTimer = undefined;
        connect();
      }, 3000);
    };

    const connect = () => {
      if (stopped) {
        return;
      }
      const url = adminRealtimeUrl();
      if (!url) {
        return;
      }
      socket?.close();
      const nextSocket = new WebSocket(url);
      socket = nextSocket;

      nextSocket.onmessage = (event) => {
        if (socket !== nextSocket) {
          return;
        }
        try {
          const message = normalizeAdminRealtimeEvent(JSON.parse(event.data));
          if (message?.event === 'support.message_created') {
            setConversations((current) => upsertById(current, message.conversation));
          }
        } catch {
          setError('后台实时客服消息解析失败');
        }
      };
      nextSocket.onclose = () => {
        if (socket === nextSocket) {
          scheduleReconnect();
        }
      };
      nextSocket.onerror = () => {
        if (socket === nextSocket) {
          setError('后台实时客服连接异常');
        }
      };
    };

    connect();

    return () => {
      stopped = true;
      if (reconnectTimer !== undefined) {
        window.clearTimeout(reconnectTimer);
      }
      socket?.close();
    };
  }, []);

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
