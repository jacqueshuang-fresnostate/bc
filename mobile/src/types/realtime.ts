export type MobileRealtimeEvent =
  | MobileDrawResultEvent
  | MobileIssueEvent
  | MobileUserRealtimeEvent
  | MobileSupportRealtimeEvent
  | MobileChatHallRealtimeEvent
  | MobileHeartbeatEvent

export type MobileDrawResultEvent = {
  event: 'draw_result'
  sourceEvent: 'lottery.draw_result'
  lotteryCode: string
  lottery_code: string
  lotteryName: string
  issue: string
  result: string
  resultNumbers: string[]
  drawNumber: string
  occurredAt: string
}

export type MobileIssueEvent = {
  event: 'issue_opened' | 'issue_closed'
  sourceEvent: 'lottery.issue_opened' | 'lottery.issue_closed'
  lotteryCode: string
  lottery_code: string
  lotteryName: string
  issue: string
  scheduledAt?: string
  saleClosedAt?: string
  status?: string
  occurredAt: string
}

export type MobileUserRealtimeEvent = {
  event: 'balance_changed' | 'order_changed' | 'recharge_changed' | 'withdrawal_changed'
  sourceEvent: 'user.balance_changed' | 'user.order_changed' | 'user.recharge_changed' | 'user.withdrawal_changed'
  data: Record<string, unknown>
  occurredAt: string
}

export type MobileSupportRealtimeEvent = {
  event: 'support_message_created' | 'support_conversation_updated' | 'support_conversation_deleted'
  sourceEvent: 'support.message_created' | 'support.conversation_updated' | 'support.conversation_deleted'
  conversationId: string
  data: Record<string, unknown>
  occurredAt: string
}

export type MobileChatHallMessage = {
  id: string
  userId: string
  username: string
  avatarUrl?: string
  content: string
  messageType?: 'text' | 'redPacket' | 'groupBuyPlan'
  payload?: Record<string, unknown> | null
  createdAt: string
}

export type MobileChatHallRealtimeEvent =
  | {
      event: 'chat_hall_message_created'
      sourceEvent: 'chat_hall.message_created'
      message: MobileChatHallMessage
      data: Record<string, unknown>
      occurredAt: string
    }
  | {
      event: 'chat_hall_messages_cleared'
      sourceEvent: 'chat_hall.messages_cleared'
      data: Record<string, unknown>
      occurredAt: string
    }

export type MobileHeartbeatEvent = {
  event: 'heartbeat'
  sourceEvent: 'system.heartbeat'
  occurredAt: string
}

type RealtimeEnvelope = {
  event?: string
  occurredAt?: string
  data?: Record<string, unknown>
}

export function normalizeRealtimeEvent(raw: unknown): MobileRealtimeEvent | null {
  if (!raw || typeof raw !== 'object') return null
  const envelope = raw as RealtimeEnvelope
  const event = String(envelope.event || '').trim()
  const data = isPlainObject(envelope.data) ? envelope.data : {}
  const occurredAt = String(envelope.occurredAt || '')

  if (event === 'system.heartbeat') {
    return { event: 'heartbeat', sourceEvent: event, occurredAt }
  }
  if (event === 'lottery.draw_result') {
    const lotteryCode = stringValue(data.lotteryId)
    const issue = stringValue(data.issue)
    const resultNumbers = normalizeResultNumbers(data.resultNumbers, data.drawNumber)
    return {
      event: 'draw_result',
      sourceEvent: event,
      lotteryCode,
      lottery_code: lotteryCode,
      lotteryName: stringValue(data.lotteryName),
      issue,
      result: resultNumbers.join(','),
      resultNumbers,
      drawNumber: stringValue(data.drawNumber),
      occurredAt,
    }
  }
  if (event === 'lottery.issue_opened' || event === 'lottery.issue_closed') {
    const lotteryCode = stringValue(data.lotteryId)
    return {
      event: event === 'lottery.issue_opened' ? 'issue_opened' : 'issue_closed',
      sourceEvent: event,
      lotteryCode,
      lottery_code: lotteryCode,
      lotteryName: stringValue(data.lotteryName),
      issue: stringValue(data.issue),
      scheduledAt: optionalString(data.scheduledAt),
      saleClosedAt: optionalString(data.saleClosedAt),
      status: optionalString(data.status),
      occurredAt,
    }
  }
  if (
    event === 'user.balance_changed'
    || event === 'user.order_changed'
    || event === 'user.recharge_changed'
    || event === 'user.withdrawal_changed'
  ) {
    return {
      event: event.replace('user.', '').replace(/_/g, '_') as MobileUserRealtimeEvent['event'],
      sourceEvent: event,
      data,
      occurredAt,
    }
  }
  if (
    event === 'support.message_created'
    || event === 'support.conversation_updated'
    || event === 'support.conversation_deleted'
  ) {
    const supportEventMap = {
      'support.message_created': 'support_message_created',
      'support.conversation_updated': 'support_conversation_updated',
      'support.conversation_deleted': 'support_conversation_deleted',
    } as const
    return {
      event: supportEventMap[event],
      sourceEvent: event,
      conversationId: stringValue(data.conversationId),
      data,
      occurredAt,
    }
  }
  if (event === 'chat_hall.message_created') {
    const message = normalizeChatHallMessage(data.message)
    if (!message) return null
    return {
      event: 'chat_hall_message_created',
      sourceEvent: event,
      message,
      data,
      occurredAt,
    }
  }
  if (event === 'chat_hall.messages_cleared') {
    return {
      event: 'chat_hall_messages_cleared',
      sourceEvent: event,
      data,
      occurredAt,
    }
  }

  return null
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value && typeof value === 'object' && !Array.isArray(value))
}

function stringValue(value: unknown) {
  return String(value ?? '').trim()
}

function optionalString(value: unknown) {
  const text = stringValue(value)
  return text || undefined
}

function normalizeResultNumbers(resultNumbers: unknown, drawNumber: unknown) {
  if (Array.isArray(resultNumbers)) {
    return resultNumbers.map(item => stringValue(item)).filter(Boolean)
  }

  const text = stringValue(drawNumber)
  if (!text) return []
  if (/^\d+$/.test(text)) return text.split('')
  return text.split(/[\s,，]+/).map(item => item.trim()).filter(Boolean)
}

function normalizeChatHallMessage(value: unknown): MobileChatHallMessage | null {
  if (!isPlainObject(value)) return null
  const id = stringValue(value.id)
  const userId = stringValue(value.userId)
  const username = stringValue(value.username)
  const content = stringValue(value.content)
  const createdAt = stringValue(value.createdAt)
  if (!id || !userId || !content) return null

  return {
    id,
    userId,
    username: username || '会员',
    avatarUrl: optionalString(value.avatarUrl),
    content,
    messageType: chatHallMessageType(value.messageType),
    payload: isPlainObject(value.payload) ? value.payload : null,
    createdAt,
  }
}

function chatHallMessageType(value: unknown): MobileChatHallMessage['messageType'] {
  const type = stringValue(value)
  if (type === 'redPacket' || type === 'groupBuyPlan' || type === 'text') return type
  return 'text'
}
