type ErrorResponsePayload = {
  message?: unknown
  detail?: unknown
}

type ErrorLike = {
  message?: unknown
  response?: {
    status?: number
    data?: ErrorResponsePayload
  }
}

const REQUIRED_LABELS: Record<string, string> = {
  'admin password': '管理员密码',
  amount: '金额',
  content: '内容',
  email: '邮箱',
  issue: '期号',
  'login key': '登录账号',
  'lot code': '开奖代码',
  'lottery id': '彩种',
  password: '密码',
  username: '用户名',
  'user id': '用户ID',
}

export function errorMessage(error: unknown, fallback = '请求失败'): string {
  const err = error as ErrorLike
  const rawMessage = firstText(
    err.response?.data?.message,
    err.response?.data?.detail,
    err.message,
  )

  return userFacingErrorMessage(rawMessage, fallback, err.response?.status)
}

export function userFacingErrorMessage(message: unknown, fallback = '请求失败', status?: number): string {
  const fallbackText = fallback || statusFallback(status) || '请求失败'
  const text = stringValue(message)
  if (!text) return statusFallback(status) || fallbackText

  const prefixed = translateApiErrorPrefix(text, fallbackText, status)
  if (prefixed) return prefixed

  const known = translateKnownEnglishMessage(text, status)
  if (known) return known

  if (hasChinese(text)) return text
  return statusFallback(status) || fallbackText
}

function translateApiErrorPrefix(text: string, fallback: string, status?: number): string {
  const match = text.match(/^(bad request|unauthorized|forbidden|not found|conflict|internal error):\s*(.+)$/i)
  if (!match) return ''

  const kind = match[1].toLowerCase()
  const detailText = match[2].trim()
  const detail = userFacingErrorMessage(detailText, statusFallback(status) || fallback, status)

  if (kind === 'bad request') return detail || '请求参数有误'
  if (kind === 'unauthorized') return detail || '登录已过期，请重新登录'
  if (kind === 'forbidden') return detail ? `权限不足：${detail}` : '权限不足，无法执行该操作'
  if (kind === 'not found') return detail || '请求的数据不存在'
  if (kind === 'conflict') return detail ? `操作冲突：${detail}` : '当前操作与已有数据冲突'
  if (kind === 'internal error') {
    if (detail && detail !== (statusFallback(status) || fallback) && hasChinese(detail)) {
      return `服务异常：${detail}`
    }
    return '服务异常，请稍后重试'
  }

  return ''
}

function translateKnownEnglishMessage(text: string, status?: number): string {
  const normalized = text.trim().toLowerCase()

  if (normalized === 'network error') return '网络连接失败，请检查网络后重试'
  if (normalized.includes('timeout')) return '请求超时，请稍后重试'
  if (normalized.startsWith('request failed with status code')) {
    return statusFallback(status || Number(normalized.match(/\d+$/)?.[0])) || '请求失败，请稍后重试'
  }
  if (
    normalized.includes('authorization token is required')
    || normalized.includes('authorization bearer token is required')
    || normalized.includes('invalid user session')
  ) {
    return '登录已过期，请重新登录'
  }
  if (normalized.includes('invalid admin session')) return '后台登录已过期，请重新登录'
  if (normalized.includes('user account is not active')) return '用户账号未激活'
  if (normalized.includes('invalid admin credentials')) return '管理员账号或密码错误'
  if (normalized.includes('username') && normalized.includes('email') && normalized.includes('注册')) {
    return '用户名或邮箱至少填写一项用于注册'
  }
  if (normalized.includes('bank card') && text.includes('必填银行卡名称')) return '银行卡必填银行名称'
  if (normalized.includes('insufficient available balance')) return '可用余额不足'
  if (normalized.includes('not on sale')) return '彩种已停售'
  if (normalized.includes('does not configure this play rule')) return '彩种未配置该玩法'
  if (normalized.includes('amount is too large')) return '金额过大，请减少后重试'
  if (normalized.includes('share count is too large')) return '合买份数过大'
  if (normalized.includes('minimum amount overflow')) return '合买最低金额计算失败，请稍后重试'
  if (normalized.includes('filled amount overflow')) return '合买进度金额计算失败，请稍后重试'
  if (normalized.includes('filled amount underflow')) return '合买进度金额异常，请稍后重试'
  if (normalized.includes('overview amount overflow')) return '资金汇总金额计算失败，请稍后重试'

  const notFound = normalized.match(/^([a-z ]+) `[^`]+` not found$/)
  if (notFound) return notFoundMessage(notFound[1])

  const required = normalized.match(/^(.+?) is required$/)
  if (required) {
    const label = REQUIRED_LABELS[required[1]] || '必填信息'
    return `${label}不能为空`
  }

  if (normalized.includes('store lock poisoned')) return '服务繁忙，请稍后重试'
  if (normalized.includes('hash failed') || normalized.includes('hash is invalid')) {
    return '密码安全校验失败，请稍后重试'
  }

  return ''
}

function notFoundMessage(resource: string): string {
  const resourceName = resource.trim()
  if (resourceName === 'financial account') return '资金账户不存在'
  if (resourceName === 'group buy plan') return '合买计划不存在'
  if (resourceName === 'recharge order') return '充值订单不存在'
  if (resourceName === 'withdrawal order') return '提现订单不存在'
  if (resourceName === 'withdrawal method') return '提现方式不存在'
  if (resourceName === 'support conversation') return '客服会话不存在'
  if (resourceName === 'invite record') return '邀请记录不存在'
  if (resourceName === 'lottery') return '彩种不存在'
  if (resourceName === 'order') return '注单不存在'
  if (resourceName === 'settlement') return '计奖记录不存在'
  if (resourceName === 'user') return '用户不存在'
  return '请求的数据不存在'
}

function statusFallback(status?: number): string {
  if (!status) return ''
  if (status === 400 || status === 422) return '请求参数有误'
  if (status === 401) return '登录已过期，请重新登录'
  if (status === 403) return '权限不足，无法执行该操作'
  if (status === 404) return '请求的数据不存在'
  if (status === 409) return '当前操作与已有数据冲突'
  if (status >= 500) return '服务异常，请稍后重试'
  return ''
}

function firstText(...values: unknown[]): string {
  for (const value of values) {
    const text = stringValue(value)
    if (text) return text
  }
  return ''
}

function stringValue(value: unknown): string {
  return String(value ?? '').trim()
}

function hasChinese(text: string): boolean {
  return /[\u4e00-\u9fff]/.test(text)
}
