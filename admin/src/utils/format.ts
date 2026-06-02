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
