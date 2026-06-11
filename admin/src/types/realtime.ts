import type { SupportConversation, SupportMessage } from './support';

export type AdminRealtimeEvent =
  | AdminHeartbeatEvent
  | AdminSupportMessageCreatedEvent
  | AdminSupportConversationUpdatedEvent
  | AdminSupportConversationDeletedEvent;

export interface AdminHeartbeatEvent {
  event: 'system.heartbeat';
  occurredAt: string;
}

export interface AdminSupportMessageCreatedEvent {
  event: 'support.message_created';
  occurredAt: string;
  conversation: SupportConversation;
  conversationId: string;
  message: SupportMessage;
}

export interface AdminSupportConversationUpdatedEvent {
  event: 'support.conversation_updated';
  occurredAt: string;
  conversation: SupportConversation;
  conversationId: string;
}

export interface AdminSupportConversationDeletedEvent {
  event: 'support.conversation_deleted';
  occurredAt: string;
  conversationId: string;
  userId: string;
}

interface AdminRealtimeEnvelope {
  event?: string;
  occurredAt?: string;
  data?: unknown;
}

export function normalizeAdminRealtimeEvent(raw: unknown): AdminRealtimeEvent | null {
  if (!isPlainObject(raw)) {
    return null;
  }
  const envelope = raw as AdminRealtimeEnvelope;
  const event = String(envelope.event || '').trim();
  const occurredAt = String(envelope.occurredAt || '');

  if (event === 'system.heartbeat') {
    return { event, occurredAt };
  }
  if (event === 'support.message_created' && isPlainObject(envelope.data)) {
    const conversation = supportConversationValue(envelope.data.conversation);
    const message = supportMessageValue(envelope.data.message);
    const conversationId = String(envelope.data.conversationId || conversation?.id || '').trim();
    if (!conversation || !message || !conversationId) {
      return null;
    }
    return {
      conversation,
      conversationId,
      event,
      message,
      occurredAt,
    };
  }
  if (event === 'support.conversation_updated' && isPlainObject(envelope.data)) {
    const conversation = supportConversationValue(envelope.data.conversation);
    const conversationId = String(envelope.data.conversationId || conversation?.id || '').trim();
    if (!conversation || !conversationId) {
      return null;
    }
    return {
      conversation,
      conversationId,
      event,
      occurredAt,
    };
  }
  if (event === 'support.conversation_deleted' && isPlainObject(envelope.data)) {
    const conversationId = String(envelope.data.conversationId || '').trim();
    const userId = String(envelope.data.userId || '').trim();
    if (!conversationId || !userId) {
      return null;
    }
    return {
      conversationId,
      event,
      occurredAt,
      userId,
    };
  }

  return null;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function supportConversationValue(value: unknown): SupportConversation | null {
  if (!isPlainObject(value) || typeof value.id !== 'string' || !Array.isArray(value.messages)) {
    return null;
  }
  return value as unknown as SupportConversation;
}

function supportMessageValue(value: unknown): SupportMessage | null {
  if (!isPlainObject(value) || typeof value.id !== 'string') {
    return null;
  }
  return value as unknown as SupportMessage;
}
