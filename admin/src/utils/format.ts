export function formatMoney(amountMinor: number) {
  return new Intl.NumberFormat('zh-CN', {
    currency: 'CNY',
    style: 'currency',
  }).format(amountMinor / 100);
}

export function formatSignedMoney(amountMinor: number) {
  const sign = amountMinor > 0 ? '+' : '';
  return `${sign}${formatMoney(amountMinor)}`;
}

const CHINA_DATE_TIME_FORMATTER = new Intl.DateTimeFormat('zh-CN', {
  day: '2-digit',
  hour: '2-digit',
  hour12: false,
  minute: '2-digit',
  month: '2-digit',
  second: '2-digit',
  timeZone: 'Asia/Shanghai',
  year: 'numeric',
});

function formatDateParts(date: Date) {
  const parts = Object.fromEntries(
    CHINA_DATE_TIME_FORMATTER.formatToParts(date).map((part) => [
      part.type,
      part.value,
    ]),
  );
  return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}:${parts.second}`;
}

function normalizeDisplayDateTime(value: string) {
  const text = value.trim();
  const match = text.match(
    /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})(?::(\d{2}))?/,
  );
  if (!match || /(?:z|[+-]\d{2}:?\d{2})$/i.test(text)) {
    return null;
  }
  return `${match[1]}-${match[2]}-${match[3]} ${match[4]}:${match[5]}:${match[6] || '00'}`;
}

function parseDateTime(value: unknown) {
  if (value == null || value === '') {
    return NaN;
  }
  if (value instanceof Date) {
    return value.getTime();
  }
  if (typeof value === 'number') {
    return value < 1_000_000_000_000 ? value * 1000 : value;
  }
  const text = String(value).trim();
  if (!text) {
    return NaN;
  }
  const unixMatch = text.match(/^unix:(\d+)$/i);
  if (unixMatch) {
    return Number(unixMatch[1]) * 1000;
  }
  const timestamp = Date.parse(text);
  return Number.isNaN(timestamp) ? NaN : timestamp;
}

export function formatDateTime(value: unknown, fallback = '-') {
  if (value == null || value === '') {
    return fallback;
  }
  if (typeof value === 'string') {
    const displayText = normalizeDisplayDateTime(value);
    if (displayText) {
      return displayText;
    }
  }
  const timestamp = parseDateTime(value);
  if (Number.isNaN(timestamp)) {
    return fallback;
  }
  return formatDateParts(new Date(timestamp));
}
