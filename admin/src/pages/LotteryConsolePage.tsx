import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import {
  Activity,
  Clock3,
  Hash,
  Radio,
  RefreshCcw,
  Timer,
} from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { MetricCard } from '../components/MetricCard';
import { useLotteryConsole } from '../hooks/useLotteryConsole';
import type { DrawMode, LotteryKind, LotteryNumberType } from '../types/dashboard';
import type { DrawIssue, DrawIssueStatus } from '../types/draws';

interface LotteryConsolePageProps {
  onDashboardRefresh: () => void;
}

interface LotteryConsoleItem {
  lottery: LotteryKind;
  currentIssue: DrawIssue | null;
  recentDrawnIssue: DrawIssue | null;
  issueCount: number;
}

type LotteryConsoleStatusFilter =
  | 'all'
  | 'closed'
  | 'drawn'
  | 'noCurrent'
  | 'open'
  | 'saleDisabled'
  | 'saleEnabled';

interface LotteryConsoleStatusFilterOption {
  count: number;
  key: LotteryConsoleStatusFilter;
  label: string;
}

export function LotteryConsolePage({
  onDashboardRefresh,
}: LotteryConsolePageProps) {
  const { error, issues, loading, lotteries, refresh } = useLotteryConsole();
  const [now, setNow] = useState(() => new Date());
  const [statusFilter, setStatusFilter] =
    useState<LotteryConsoleStatusFilter>('all');

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      setNow(new Date());
    }, 1_000);

    return () => {
      window.clearInterval(intervalId);
    };
  }, []);

  const items = useMemo(
    () => lotteries.map((lottery) => lotteryConsoleItem(lottery, issues)),
    [issues, lotteries],
  );
  const statusFilterOptions = useMemo(
    () => lotteryConsoleStatusFilterOptions(items),
    [items],
  );
  const filteredItems = useMemo(
    () => items.filter((item) => lotteryConsoleItemMatchesFilter(item, statusFilter)),
    [items, statusFilter],
  );

  const metrics = useMemo(() => {
    const saleEnabledCount = lotteries.filter((lottery) => lottery.saleEnabled).length;
    const openCount = items.filter(
      (item) => item.currentIssue?.status === 'open',
    ).length;
    const waitingDrawCount = items.filter(
      (item) => item.currentIssue?.status === 'closed',
    ).length;
    const drawnCount = items.filter((item) => item.recentDrawnIssue).length;

    return [
      {
        key: 'lotteries',
        label: '彩种总数',
        trend: '当前后台配置',
        value: `${lotteries.length}`,
      },
      {
        key: 'saleEnabled',
        label: '销售开启',
        trend: '允许生成销售期号',
        value: `${saleEnabledCount}`,
      },
      {
        key: 'open',
        label: '开盘中',
        trend: '存在 open 期号',
        value: `${openCount}`,
      },
      {
        key: 'waitingDraw',
        label: '待开奖',
        trend: `${drawnCount} 个彩种已有结果`,
        value: `${waitingDrawCount}`,
      },
    ];
  }, [items, lotteries]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  if (loading && lotteries.length === 0 && issues.length === 0) {
    return (
      <div className="grid min-h-[420px] place-items-center">
        <Spin size="large" tip="正在加载彩种控制台" />
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">彩种控制台</h1>
          <p className="mt-1 text-sm text-slate-500">
            按彩种查看开盘、封盘、开奖倒计时和最近开奖号码。
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Tag color="cyan">本地时间 {formatClock(now)}</Tag>
          <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
            刷新
          </Button>
        </div>
      </section>

      {error ? (
        <Banner
          type="warning"
          title="彩种控制台刷新失败"
          description={`当前展示上一次成功数据。错误：${error}`}
        />
      ) : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {metrics.map((metric) => (
          <MetricCard
            key={metric.key}
            label={metric.label}
            value={metric.value}
            trend={metric.trend}
          />
        ))}
      </section>

      <LotteryConsoleStatusFilterBar
        active={statusFilter}
        filteredCount={filteredItems.length}
        options={statusFilterOptions}
        totalCount={items.length}
        onChange={setStatusFilter}
      />

      {filteredItems.length > 0 ? (
        <section className="grid gap-4 xl:grid-cols-2 2xl:grid-cols-3">
          {filteredItems.map((item) => (
            <LotteryConsoleCard key={item.lottery.id} item={item} now={now} />
          ))}
        </section>
      ) : (
        <Card className="rounded-md border border-line">
          <div className="py-8 text-center text-sm text-slate-500">
            {items.length > 0 ? '当前筛选下暂无彩种。' : '暂无彩种配置。'}
          </div>
        </Card>
      )}
    </div>
  );
}

function LotteryConsoleStatusFilterBar({
  active,
  filteredCount,
  onChange,
  options,
  totalCount,
}: {
  active: LotteryConsoleStatusFilter;
  filteredCount: number;
  onChange: (filter: LotteryConsoleStatusFilter) => void;
  options: LotteryConsoleStatusFilterOption[];
  totalCount: number;
}) {
  return (
    <Card className="rounded-md border border-line">
      <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
        <div>
          <div className="text-sm font-semibold text-ink">状态筛选</div>
          <div className="mt-1 text-xs text-slate-500">
            当前显示 {filteredCount} / {totalCount} 个彩种
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          {options.map((option) => (
            <Button
              key={option.key}
              size="small"
              theme={active === option.key ? 'solid' : 'light'}
              onClick={() => onChange(option.key)}
            >
              {option.label}
              <span className="ml-1 font-mono text-xs opacity-75">{option.count}</span>
            </Button>
          ))}
        </div>
      </div>
    </Card>
  );
}

function LotteryConsoleCard({
  item,
  now,
}: {
  item: LotteryConsoleItem;
  now: Date;
}) {
  const { currentIssue, lottery, recentDrawnIssue } = item;
  const drawNumber = currentIssue?.drawNumber ?? recentDrawnIssue?.drawNumber ?? null;
  const drawNumberLabel = currentIssue?.drawNumber ? '本期开奖号码' : '最近开奖号码';

  return (
    <Card shadows="hover" className="rounded-md border border-line">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h2 className="truncate text-base font-semibold text-ink">{lottery.name}</h2>
          <div className="mt-1 text-xs text-slate-400">{lottery.id}</div>
        </div>
        <Tag color={lottery.saleEnabled ? 'green' : 'grey'}>
          {lottery.saleEnabled ? '销售开启' : '已停售'}
        </Tag>
      </div>

      <div className="mt-3 flex flex-wrap gap-2">
        <Tag color="cyan">{numberTypeText(lottery.numberType)}</Tag>
        <Tag color={drawModeColor(lottery.drawMode)}>
          {drawModeText(lottery.drawMode)}
        </Tag>
        {currentIssue ? (
          <Tag color={statusColor(currentIssue.status)}>
            {statusText(currentIssue.status)}
          </Tag>
        ) : (
          <Tag color="grey">暂无当前期</Tag>
        )}
      </div>

      <div className="mt-4 grid gap-3 sm:grid-cols-2">
        <CountdownBlock
          icon={<Clock3 size={16} />}
          label="封盘倒计时"
          value={countdownText(currentIssue?.saleClosedAt, now, '已到封盘', '暂无期号')}
        />
        <CountdownBlock
          icon={<Timer size={16} />}
          label="开奖倒计时"
          value={countdownText(currentIssue?.scheduledAt, now, '等待开奖', '暂无期号')}
        />
      </div>

      <div className="mt-4 rounded-md bg-slate-50 p-3">
        <div className="flex items-center gap-2 text-xs font-medium text-slate-500">
          <Hash size={14} />
          当前期号
        </div>
        <div className="mt-2 flex flex-wrap items-end justify-between gap-2">
          <div>
            <div className="text-lg font-semibold text-ink">
              {currentIssue?.issue ?? '暂无 open/closed 期号'}
            </div>
            {currentIssue ? (
              <div className="mt-1 text-xs text-slate-500">
                封盘 {currentIssue.saleClosedAt} · 开奖 {currentIssue.scheduledAt}
              </div>
            ) : (
              <div className="mt-1 text-xs text-slate-500">
                已配置 {item.issueCount} 个历史期号
              </div>
            )}
          </div>
          <Tag color={currentIssue ? statusColor(currentIssue.status) : 'grey'}>
            {currentIssue ? statusText(currentIssue.status) : '无当前期'}
          </Tag>
        </div>
      </div>

      <div className="mt-3 rounded-md border border-line p-3">
        <div className="flex items-center gap-2 text-xs font-medium text-slate-500">
          <Radio size={14} />
          {drawNumberLabel}
        </div>
        {drawNumber ? (
          <div className="mt-2 font-mono text-2xl font-semibold text-ink">
            {drawNumber}
          </div>
        ) : (
          <div className="mt-2 text-sm text-slate-400">待开奖</div>
        )}
        {recentDrawnIssue ? (
          <div className="mt-2 text-xs text-slate-500">
            最近期号 {recentDrawnIssue.issue} · {recentDrawnIssue.drawnAt ?? '已开奖'}
          </div>
        ) : null}
      </div>

      <div className="mt-3 flex items-center justify-between text-xs text-slate-500">
        <span className="flex items-center gap-1">
          <Activity size={13} />
          期号 {item.issueCount} 个
        </span>
        <span>{scheduleText(lottery)}</span>
      </div>
    </Card>
  );
}

function CountdownBlock({
  icon,
  label,
  value,
}: {
  icon: ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="rounded-md border border-line px-3 py-2">
      <div className="flex items-center gap-2 text-xs font-medium text-slate-500">
        {icon}
        {label}
      </div>
      <div className="mt-2 font-mono text-xl font-semibold text-ink">{value}</div>
    </div>
  );
}

function lotteryConsoleStatusFilterOptions(
  items: LotteryConsoleItem[],
): LotteryConsoleStatusFilterOption[] {
  return [
    { count: items.length, key: 'all', label: '全部' },
    {
      count: items.filter((item) => item.lottery.saleEnabled).length,
      key: 'saleEnabled',
      label: '销售开启',
    },
    {
      count: items.filter((item) => !item.lottery.saleEnabled).length,
      key: 'saleDisabled',
      label: '已停售',
    },
    {
      count: items.filter((item) => item.currentIssue?.status === 'open').length,
      key: 'open',
      label: '开盘中',
    },
    {
      count: items.filter((item) => item.currentIssue?.status === 'closed').length,
      key: 'closed',
      label: '待开奖',
    },
    {
      count: items.filter((item) => item.recentDrawnIssue !== null).length,
      key: 'drawn',
      label: '已开奖',
    },
    {
      count: items.filter((item) => item.currentIssue === null).length,
      key: 'noCurrent',
      label: '无当前期',
    },
  ];
}

function lotteryConsoleItemMatchesFilter(
  item: LotteryConsoleItem,
  filter: LotteryConsoleStatusFilter,
) {
  switch (filter) {
    case 'all':
      return true;
    case 'closed':
      return item.currentIssue?.status === 'closed';
    case 'drawn':
      return item.recentDrawnIssue !== null;
    case 'noCurrent':
      return item.currentIssue === null;
    case 'open':
      return item.currentIssue?.status === 'open';
    case 'saleDisabled':
      return !item.lottery.saleEnabled;
    case 'saleEnabled':
      return item.lottery.saleEnabled;
  }
}

function lotteryConsoleItem(
  lottery: LotteryKind,
  allIssues: DrawIssue[],
): LotteryConsoleItem {
  const issues = allIssues.filter((issue) => issue.lotteryId === lottery.id);
  const currentIssue =
    issues
      .filter((issue) => issue.status === 'open' || issue.status === 'closed')
      .sort((left, right) => issueTimeValue(left) - issueTimeValue(right))[0] ?? null;
  const recentDrawnIssue =
    issues
      .filter((issue) => issue.status === 'drawn' && issue.drawNumber)
      .sort((left, right) => latestIssueTimeValue(right) - latestIssueTimeValue(left))[0] ??
    null;

  return {
    currentIssue,
    issueCount: issues.length,
    lottery,
    recentDrawnIssue,
  };
}

function countdownText(
  targetTime: string | null | undefined,
  now: Date,
  reachedLabel: string,
  emptyLabel: string,
) {
  const targetMs = parseTimeLabel(targetTime);
  if (targetMs === null) {
    return emptyLabel;
  }

  const diffMs = targetMs - now.getTime();
  if (diffMs <= 0) {
    return reachedLabel;
  }

  return formatDuration(diffMs);
}

function formatDuration(diffMs: number) {
  const totalSeconds = Math.max(0, Math.floor(diffMs / 1_000));
  const days = Math.floor(totalSeconds / 86_400);
  const hours = Math.floor((totalSeconds % 86_400) / 3_600);
  const minutes = Math.floor((totalSeconds % 3_600) / 60);
  const seconds = totalSeconds % 60;
  const clock = [hours, minutes, seconds]
    .map((value) => value.toString().padStart(2, '0'))
    .join(':');

  return days > 0 ? `${days}天 ${clock}` : clock;
}

function issueTimeValue(issue: DrawIssue) {
  return parseTimeLabel(issue.saleClosedAt) ?? parseTimeLabel(issue.scheduledAt) ?? 0;
}

function latestIssueTimeValue(issue: DrawIssue) {
  return (
    parseTimeLabel(issue.drawnAt) ??
    parseTimeLabel(issue.scheduledAt) ??
    parseTimeLabel(issue.createdAt) ??
    0
  );
}

function parseTimeLabel(value: string | null | undefined) {
  if (!value) {
    return null;
  }

  if (value.startsWith('unix:')) {
    const seconds = Number.parseInt(value.slice(5), 10);
    return Number.isFinite(seconds) ? seconds * 1_000 : null;
  }

  const normalized = value.includes('T') ? value : value.replace(' ', 'T');
  const timestamp = Date.parse(normalized);
  return Number.isNaN(timestamp) ? null : timestamp;
}

function formatClock(value: Date) {
  const pad = (part: number) => part.toString().padStart(2, '0');
  return `${pad(value.getHours())}:${pad(value.getMinutes())}:${pad(value.getSeconds())}`;
}

function numberTypeText(numberType: LotteryNumberType) {
  return numberType === 'threeDigit' ? '3 位号码' : '5 位号码';
}

function drawModeText(mode: DrawMode) {
  const labels: Record<DrawMode, string> = {
    api: 'API 开奖',
    manual: '手动开奖',
    platform: '平台开奖',
  };
  return labels[mode];
}

function drawModeColor(mode: DrawMode) {
  if (mode === 'api') {
    return 'blue';
  }
  if (mode === 'manual') {
    return 'orange';
  }
  return 'green';
}

function statusText(status: DrawIssueStatus) {
  const labels: Record<DrawIssueStatus, string> = {
    cancelled: '已取消',
    closed: '已封盘',
    drawn: '已开奖',
    open: '销售中',
  };
  return labels[status];
}

function statusColor(status: DrawIssueStatus) {
  if (status === 'cancelled') {
    return 'grey';
  }
  if (status === 'closed') {
    return 'orange';
  }
  if (status === 'drawn') {
    return 'green';
  }
  return 'blue';
}

function scheduleText(lottery: LotteryKind) {
  const { schedule } = lottery;
  if ('periodic' in schedule) {
    return `${schedule.periodic.intervalSeconds} 秒一期`;
  }
  if ('daily' in schedule) {
    return `每日 ${schedule.daily.time}`;
  }
  return `${schedule.weekly.weekdays.join('、')} ${schedule.weekly.time}`;
}
