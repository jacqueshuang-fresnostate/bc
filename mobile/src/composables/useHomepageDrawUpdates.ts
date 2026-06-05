import type { Ref } from 'vue'
import type { HomepageResponse, LotteryCard } from '../api/lottery'
import { parseChinaDateTime } from '../utils/lotteryFormat'

export type LotteryDrawMessage = {
  event?: string
  lotteryCode?: string
  lottery_code?: string
  issue?: string | null
  result?: unknown
  scheduledAt?: string
  saleClosedAt?: string
  status?: string
}

// 首页开奖更新边界：这里集中定义 websocket 开奖后本地更新规则。
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
    const result = lottery?.latestResult || []
    const normalizedFallbackCount = Number.isFinite(fallbackCount) && fallbackCount > 0 ? Math.floor(fallbackCount) : 3
    const displayCount = Math.max(result.length, normalizedFallbackCount)
    const digits = result.length
      ? [...result]
      : (lottery?.issue || '').replace(/\D/g, '').slice(-displayCount).split('')
    while (digits.length < displayCount) digits.push('?')
    return digits.slice(0, displayCount)
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
    // 倒计时先算封盘，再算开奖；开奖时间已过时由首页触发静默刷新，避免长期停在“开奖中”。
    if (!lottery) return '--:--'
    if (lottery.status === 'drawn') return '已开奖'
    if (lottery.status === 'closed') return '已关闭'

    const saleStopTime = parseChinaDateTime(lottery.saleStopTime)
    if (Number.isFinite(saleStopTime) && saleStopTime > nowMs.value) {
      return formatCountdown(Math.floor((saleStopTime - nowMs.value) / 1000))
    }

    const drawTime = parseChinaDateTime(lottery.nextDrawTime)
    if (Number.isFinite(drawTime) && drawTime > nowMs.value) {
      return `封盘 ${formatCountdown(Math.floor((drawTime - nowMs.value) / 1000))}`
    }

    if (Number.isFinite(drawTime)) {
      return '开奖中'
    }
    return lottery.drawTimeText || lottery.scheduleText || '--:--'
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
    if (!lottery.drawInterval || lottery.drawInterval <= 0) return null
    return Date.now() + lottery.drawInterval * 1000
  }

  function updateLotteryFromDrawResult(lottery: LotteryCard, msg: LotteryDrawMessage) {
    // 只更新推送命中的彩种：写入最新开奖号、推进下一期号，并用 draw_interval 估算下一次倒计时。
    const lotteryCode = msg?.lotteryCode || msg?.lottery_code
    if (lottery.code !== lotteryCode) return lottery
    return {
      ...lottery,
      issue: nextIssue(msg?.issue || lottery.issue),
      status: 'selling',
      nextDrawTime: nextDrawTime(lottery),
      drawTimeText: lottery.drawTimeText,
      latestResult: parseDrawResult(msg.result),
    }
  }

  function issueStatus(status?: string, event?: string) {
    if (event === 'issue_closed') return 'sealed'
    if (event === 'issue_opened') return 'selling'
    if (status === 'closed') return 'sealed'
    if (status === 'drawn') return 'drawn'
    if (status === 'cancelled') return 'closed'
    return 'selling'
  }

  function updateLotteryFromIssue(lottery: LotteryCard, msg: LotteryDrawMessage) {
    // 开盘/封盘推送只更新当前轮次和时间字段，不覆盖最近开奖号码。
    const lotteryCode = msg?.lotteryCode || msg?.lottery_code
    if (lottery.code !== lotteryCode) return lottery
    return {
      ...lottery,
      issue: msg.issue || lottery.issue,
      status: issueStatus(msg.status, msg.event),
      nextDrawTime: msg.scheduledAt || lottery.nextDrawTime,
      saleStopTime: msg.saleClosedAt || lottery.saleStopTime,
    }
  }

  function applyDrawResult(msg: LotteryDrawMessage) {
    // WebSocket 推送同时覆盖推荐区和分组区，保持首页所有同彩种卡片展示一致。
    if (!homepage.value || !(msg?.lotteryCode || msg?.lottery_code)) return
    homepage.value = {
      ...homepage.value,
      featuredSection: homepage.value.featuredSection
        ? {
            ...homepage.value.featuredSection,
            lotteries: (homepage.value.featuredSection.lotteries || []).map(lottery => updateLotteryFromDrawResult(lottery, msg)),
          }
        : homepage.value.featuredSection,
      groups: (homepage.value.groups || []).map(group => ({
        ...group,
        lotteries: (group.lotteries || []).map(lottery => updateLotteryFromDrawResult(lottery, msg)),
      })),
    }
  }

  function applyIssueUpdate(msg: LotteryDrawMessage) {
    if (!homepage.value || !(msg?.lotteryCode || msg?.lottery_code)) return
    homepage.value = {
      ...homepage.value,
      featuredSection: homepage.value.featuredSection
        ? {
            ...homepage.value.featuredSection,
            lotteries: (homepage.value.featuredSection.lotteries || []).map(lottery => updateLotteryFromIssue(lottery, msg)),
          }
        : homepage.value.featuredSection,
      groups: (homepage.value.groups || []).map(group => ({
        ...group,
        lotteries: (group.lotteries || []).map(lottery => updateLotteryFromIssue(lottery, msg)),
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
    applyIssueUpdate,
  }
}
