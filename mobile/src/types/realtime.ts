export type MobileRealtimeEvent =
  | MobileDrawResultEvent
  | MobileIssueEvent
  | MobileUserRealtimeEvent
  | MobileSupportRealtimeEvent
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
  event: 'support_message_created' | 'support_conversation_updated'
  sourceEvent: 'support.message_created' | 'support.conversation_updated'
  conversationId: string
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
  if (event === 'support.message_created' || event === 'support.conversation_updated') {
    return {
      event: event === 'support.message_created'
        ? 'support_message_created'
        : 'support_conversation_updated',
      sourceEvent: event,
      conversationId: stringValue(data.conversationId),
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
