import { useCallback, useEffect, useState } from 'react';
import {
  adminRealtimeUrl,
  deleteSupportConversation,
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
        setConversations(visibleSupportConversations(nextConversations));
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
          if (
            message?.event === 'support.message_created' ||
            message?.event === 'support.conversation_updated'
          ) {
            setConversations((current) =>
              upsertVisibleConversation(current, message.conversation),
            );
          } else if (message?.event === 'support.conversation_deleted') {
            setConversations((current) =>
              current.filter(
                (conversation) => conversation.id !== message.conversationId,
              ),
            );
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
        setConversations((current) => upsertVisibleConversation(current, updated));
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

  const remove = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const deleted = await deleteSupportConversation(id);
      setConversations((current) =>
        current.filter((conversation) => conversation.id !== deleted.id),
      );
      return deleted;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const reply = useCallback(async (id: string, payload: SupportReplyRequest) => {
    setSaving(true);
    setError(null);
    try {
      const updated = await replySupportConversation(id, payload);
      setConversations((current) => upsertVisibleConversation(current, updated));
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
    remove,
    saving,
    update,
  };
}

function visibleSupportConversations(items: SupportConversation[]) {
  return sortSupportConversations(items.filter(isVisibleSupportConversation));
}

function isVisibleSupportConversation(conversation: SupportConversation) {
  return conversation.status !== 'closed';
}

function upsertVisibleConversation(
  items: SupportConversation[],
  conversation: SupportConversation,
) {
  const nextItems = items.filter((current) => current.id !== conversation.id);
  return isVisibleSupportConversation(conversation)
    ? sortSupportConversations([...nextItems, conversation])
    : sortSupportConversations(nextItems);
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}

function sortSupportConversations(items: SupportConversation[]) {
  return [...items].sort((left, right) => {
    const leftUnread = left.unreadCount > 0;
    const rightUnread = right.unreadCount > 0;

    if (leftUnread !== rightUnread) {
      return leftUnread ? -1 : 1;
    }

    return (
      supportActivityTime(right).localeCompare(supportActivityTime(left)) ||
      right.id.localeCompare(left.id)
    );
  });
}

function supportActivityTime(conversation: SupportConversation) {
  const lastMessage = conversation.messages[conversation.messages.length - 1];

  return (
    lastMessage?.createdAt ||
    conversation.updatedAt ||
    conversation.createdAt ||
    ''
  );
}
