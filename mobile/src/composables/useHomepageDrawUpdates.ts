import type { Ref } from 'vue'
import { parseChinaDateTime } from '../utils/lotteryFormat'

// 首页开奖更新边界：这里集中定义首页接口卡片结构和 websocket 开奖后本地更新规则。
export type LotteryCard = {
  code: string
  name: string
  issue?: string | null
  status?: string
  next_draw_time?: string | number | null
  sale_stop_time?: string | number | null
  draw_interval?: number | null
  draw_time_text?: string
  schedule_text?: string
  latest_result?: string[]
  result_style?: string
  result_count?: number | null
  logo_url?: string | null
  group_buy_enabled?: boolean
}

export type HomepageBanner = {
  id?: string | number
  title?: string
  subtitle?: string
  image_url?: string
  link_url?: string
}

export type HomepageTickerItem = {
  id?: string
  text?: string
}

export type HomepageGroup = {
  code?: string
  name?: string
  lotteries?: LotteryCard[]
}

export type HomepageResponse = {
  server_time?: string | number | null
  settings?:any,
  banners?: HomepageBanner[]
  ticker?: { enabled?: boolean; items?: HomepageTickerItem[] }
  featured_section?: { enabled?: boolean; title?: string; lotteries?: LotteryCard[] }
  groups?: HomepageGroup[]
  stats?: { today_winner_count?: number; total_payout_display?: string }
}

export function useHomepageDrawUpdates(homepage: Ref<HomepageResponse | null>, nowMs: Ref<number>) {
  function statusText(status?: string) {
    // 状态文案只用于首页卡片展示，实际可投注判断仍以后端投注页接口为准。
    if (status === 'selling') return '可下注'
    if (status === 'sealed') return '已封盘'
    if (status === 'drawn') return '已开奖'
    if (status === 'waiting') return '待开奖'
    if (status === 'closed') return '已关闭'
    return '-'
  }

  function roundDigits(lottery?: LotteryCard, fallbackCount = 3) {
    // 优先展示真实开奖结果；没有结果时用期号尾号补位，避免首页卡片出现空白号码格。
    const result = lottery?.latest_result || []
    const digits = result.length ? result : (lottery?.issue || '').replace(/\D/g, '').slice(-fallbackCount).split('')
    while (digits.length < fallbackCount) digits.push('?')
    return digits.slice(0, fallbackCount)
  }

  function formatCountdown(totalSeconds: number) {
    if (totalSeconds <= 0) return '开奖中'
    const hours = Math.floor(totalSeconds / 3600)
    const minutes = Math.floor((totalSeconds % 3600) / 60)
    const seconds = totalSeconds % 60
    const parts = [minutes, seconds].map(value => String(value).padStart(2, '0'))
    return hours > 0 ? `${String(hours).padStart(2, '0')}:${parts.join(':')}` : parts.join(':')
  }

  function countdownText(lottery?: LotteryCard) {
    // sealed/drawn 直接显示业务状态；其它状态优先按封盘时间计算倒计时，再回退到开奖时间。
    if (!lottery) return '--:--'
    if (lottery.status === 'sealed') return '封盘中'
    if (lottery.status === 'drawn') return '已开奖'
    const targetTime = parseChinaDateTime(lottery.sale_stop_time || lottery.next_draw_time)
    if (Number.isFinite(targetTime)) {
      return formatCountdown(Math.max(0, Math.floor((targetTime - nowMs.value) / 1000)))
    }
    return lottery.draw_time_text || lottery.schedule_text || '--:--'
  }

  function parseDrawResult(result?: unknown) {
    if (Array.isArray(result)) return result.map(item => String(item).trim()).filter(Boolean)
    const text = String(result || '').trim()
    if (text && /^\d+$/.test(text)) return text.split('')
    return text.split(/[\s,，]+/).map(item => item.trim()).filter(Boolean)
  }

  function nextIssue(issue?: string | null) {
    const text = String(issue || '')
    const match = text.match(/^(.*?)(\d+)$/)
    if (!match) return issue || null
    const [, prefix, suffix] = match
    return `${prefix}${String(Number(suffix) + 1).padStart(suffix.length, '0')}`
  }

  function nextDrawTime(lottery: LotteryCard) {
    if (!lottery.draw_interval || lottery.draw_interval <= 0) return null
    return Date.now() + lottery.draw_interval * 1000
  }

  function updateLotteryFromDrawResult(lottery: LotteryCard, msg: any) {
    // 只更新推送命中的彩种：写入最新开奖号、推进下一期号，并用 draw_interval 估算下一次倒计时。
    if (lottery.code !== msg?.lottery_code) return lottery
    return {
      ...lottery,
      issue: nextIssue(msg?.issue || lottery.issue),
      status: 'selling',
      next_draw_time: nextDrawTime(lottery),
      draw_time_text: lottery.draw_time_text,
      latest_result: parseDrawResult(msg.result),
    }
  }

  function applyDrawResult(msg: any) {
    // WebSocket 推送同时覆盖推荐区和分组区，保持首页所有同彩种卡片展示一致。
    if (!homepage.value || !msg?.lottery_code) return
    homepage.value = {
      ...homepage.value,
      featured_section: homepage.value.featured_section
        ? {
            ...homepage.value.featured_section,
            lotteries: (homepage.value.featured_section.lotteries || []).map(lottery => updateLotteryFromDrawResult(lottery, msg)),
          }
        : homepage.value.featured_section,
      groups: (homepage.value.groups || []).map(group => ({
        ...group,
        lotteries: (group.lotteries || []).map(lottery => updateLotteryFromDrawResult(lottery, msg)),
      })),
    }
  }

  return {
    statusText,
    roundDigits,
    formatCountdown,
    countdownText,
    parseDrawResult,
    nextIssue,
    nextDrawTime,
    applyDrawResult,
  }
}
