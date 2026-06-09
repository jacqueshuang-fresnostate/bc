// 把后端保存的最小货币单位转换成后台表单里展示的“元”输入值。
export function minorToYuanInput(value: number | string | null | undefined) {
  if (value === null || value === undefined) {
    return '';
  }
  const normalized = String(value).trim();
  if (!normalized) {
    return '';
  }
  const amountMinor = Number(normalized);
  if (!Number.isFinite(amountMinor)) {
    return '';
  }
  return (Math.trunc(amountMinor) / 100).toFixed(2);
}

// 把后台表单输入的“元”转换成后端接口继续使用的最小货币单位。
export function yuanInputToMinor(value: string) {
  const normalized = value.trim();
  if (!/^-?(?:\d+|\d+\.\d{1,2}|\.\d{1,2})$/.test(normalized)) {
    return null;
  }
  const amountYuan = Number(normalized);
  if (!Number.isFinite(amountYuan)) {
    return null;
  }
  return Math.round(amountYuan * 100);
}
