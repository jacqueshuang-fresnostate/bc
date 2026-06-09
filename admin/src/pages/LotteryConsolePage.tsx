import {
  Input,
  Banner,
  Button,
  Card,
  Select,
  SideSheet,
  Spin,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import {
  Activity,
  Clock3,
  ClipboardList,
  Hash,
  Radio,
  RefreshCcw,
  Save,
  Settings2,
  Timer,
} from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { MetricCard } from '../components/MetricCard';
import { OrderBetInfo } from '../components/OrderBetInfo';
import { useLotteryConsole } from '../hooks/useLotteryConsole';
import type { DrawMode, LotteryKind } from '../types/dashboard';
import type {
  DrawControlTargetScope,
  DrawIssue,
  DrawIssueStatus,
  LotteryDrawControl,
} from '../types/draws';
import type { OrderDetail, OrderStatus } from '../types/orders';
import type { DrawSchedulerStatus } from '../types/scheduler';
import { formatMoney } from '../utils/format';
import {
  drawNumberInputMeta,
  lotteryNumberTypeText as numberTypeText,
} from '../utils/lotteries';
import { formatBetInfoSummary } from '../utils/orderBetInfo';

interface LotteryConsolePageProps {
  onDashboardRefresh: () => void;
}

interface LotteryConsoleItem {
  lottery: LotteryKind;
  currentIssue: DrawIssue | null;
  currentIssueOrders: OrderDetail[];
  drawControl: LotteryDrawControl | null;
  issues: DrawIssue[];
  orders: OrderDetail[];
  recentDrawnIssue: DrawIssue | null;
  issueCount: number;
  waitingIssue: DrawIssue | null;
  waitingIssueCount: number;
}

interface LotteryDrawControlFormState {
  enabled: boolean;
  drawNumber: string;
  targetIssue: string;
  targetOrderId: string;
  targetScope: DrawControlTargetScope;
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
  const {
    drawControls,
    error,
    issues,
    loading,
    lotteries,
    orders,
    refresh,
    schedulerStatus,
    saveDrawControl,
    syncDrawSource,
  } = useLotteryConsole();
  const [now, setNow] = useState(() => new Date());
  const [statusFilter, setStatusFilter] =
    useState<LotteryConsoleStatusFilter>('all');
  const [selectedControlItem, setSelectedControlItem] =
    useState<LotteryConsoleItem | null>(null);
  const [controlForm, setControlForm] = useState<LotteryDrawControlFormState>(
    () => emptyDrawControlForm(),
  );
  const [controlSaving, setControlSaving] = useState(false);
  const [controlError, setControlError] = useState<string | null>(null);
  const [syncingLotteryId, setSyncingLotteryId] = useState<string | null>(null);

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      setNow(new Date());
    }, 1_000);

    return () => {
      window.clearInterval(intervalId);
    };
  }, []);

  const drawControlByLotteryId = useMemo(
    () =>
      new Map(drawControls.map((control) => [control.lotteryId, control] as const)),
    [drawControls],
  );
  const items = useMemo(
    () =>
      lotteries.map((lottery) =>
        lotteryConsoleItem(
          lottery,
          issues,
          orders,
          drawControlByLotteryId.get(lottery.id) ?? null,
        ),
      ),
    [drawControlByLotteryId, issues, lotteries, orders],
  );
  const statusFilterOptions = useMemo(
    () => lotteryConsoleStatusFilterOptions(items, now),
    [items, now],
  );
  const filteredItems = useMemo(
    () =>
      items.filter((item) =>
        lotteryConsoleItemMatchesFilter(item, statusFilter, now),
      ),
    [items, statusFilter, now],
  );

  const metrics = useMemo(() => {
    const saleEnabledCount = lotteries.filter((lottery) => lottery.saleEnabled).length;
    const openCount = items.filter(
      (item) => item.currentIssue?.status === 'open',
    ).length;
    const waitingDrawCount = items.filter((item) =>
      lotteryConsoleItemIsWaitingDraw(item, now),
    ).length;
    const controlEnabledCount = items.filter(
      (item) => item.drawControl?.enabled,
    ).length;

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
        trend: `${controlEnabledCount} 个彩种控制中`,
        value: `${waitingDrawCount}`,
      },
    ];
  }, [items, lotteries, now]);

  const openControlSheet = (item: LotteryConsoleItem) => {
    setSelectedControlItem(item);
    setControlForm(drawControlFormFromControl(item.drawControl));
    setControlError(null);
  };

  const closeControlSheet = () => {
    setSelectedControlItem(null);
    setControlError(null);
  };

  const submitDrawControl = async () => {
    if (!selectedControlItem) {
      return;
    }

    setControlSaving(true);
    setControlError(null);
    try {
      const trimmedDrawNumber = controlForm.drawNumber.trim();
      await saveDrawControl(selectedControlItem.lottery.id, {
        enabled: controlForm.enabled,
        drawNumber: controlForm.enabled ? trimmedDrawNumber || null : null,
        targetIssue: controlForm.enabled ? controlForm.targetIssue.trim() || null : null,
        targetOrderId: controlForm.enabled ? controlForm.targetOrderId.trim() || null : null,
        targetScope: controlForm.targetScope,
      });
      closeControlSheet();
      refresh();
    } catch (requestError) {
      setControlError(errorMessage(requestError));
    } finally {
      setControlSaving(false);
    }
  };

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const syncLotterySource = async (item: LotteryConsoleItem) => {
    if (item.lottery.drawMode !== 'api') {
      Toast.warning('只有 API 开奖彩种可以同步开奖源');
      return;
    }

    setSyncingLotteryId(item.lottery.id);
    try {
      const result = await syncDrawSource(item.lottery.id);
      Toast.success(result.message || `已同步 ${item.lottery.name} 开奖源`);
      onDashboardRefresh();
    } catch (requestError) {
      Toast.error(errorMessage(requestError));
    } finally {
      setSyncingLotteryId(null);
    }
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
          {schedulerStatus ? (
            <Tag color={schedulerStatus.enabled ? 'green' : 'red'}>
              {schedulerStatus.enabled
                ? `调度运行中 ${schedulerStatus.config.intervalSeconds}秒`
                : '调度已关闭'}
            </Tag>
          ) : null}
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
        <section className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
          {filteredItems.map((item) => (
            <LotteryConsoleCard
              key={item.lottery.id}
              item={item}
              now={now}
              schedulerStatus={schedulerStatus}
              onOpenControl={openControlSheet}
              onSyncSource={(nextItem) => void syncLotterySource(nextItem)}
              syncing={syncingLotteryId === item.lottery.id}
            />
          ))}
        </section>
      ) : (
        <Card className="rounded-md border border-line">
          <div className="py-8 text-center text-sm text-slate-500">
            {items.length > 0 ? '当前筛选下暂无彩种。' : '暂无彩种配置。'}
          </div>
        </Card>
      )}

      <DrawControlSideSheet
        error={controlError}
        form={controlForm}
        item={selectedControlItem}
        saving={controlSaving}
        visible={Boolean(selectedControlItem)}
        onChange={setControlForm}
        onClose={closeControlSheet}
        onSubmit={() => void submitDrawControl()}
      />
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
  schedulerStatus,
  onOpenControl,
  onSyncSource,
  syncing,
}: {
  item: LotteryConsoleItem;
  now: Date;
  schedulerStatus: DrawSchedulerStatus | null;
  onOpenControl: (item: LotteryConsoleItem) => void;
  onSyncSource: (item: LotteryConsoleItem) => void;
  syncing: boolean;
}) {
  const { currentIssue, drawControl, lottery, recentDrawnIssue } = item;
  const currentIssueDrawNumber =
    currentIssue?.status === 'drawn' && currentIssue.drawNumber
      ? currentIssue.drawNumber
      : null;
  const drawNumber = currentIssueDrawNumber ?? recentDrawnIssue?.drawNumber ?? null;
  const drawNumberLabel = currentIssueDrawNumber ? '本期开奖' : '最近开奖';
  const controlEnabled = Boolean(drawControl?.enabled);
  const currentOrderAmountMinor = item.currentIssueOrders.reduce(
    (total, order) => total + order.amountMinor,
    0,
  );

  return (
    <Card
      bodyStyle={{ padding: 12 }}
      shadows="hover"
      className="min-w-0 rounded-md border border-line"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <h2 className="truncate text-sm font-semibold text-ink">{lottery.name}</h2>
          <div className="mt-0.5 truncate text-[11px] text-slate-400">
            {lottery.id} · {scheduleText(lottery)}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Tag color={currentIssue ? statusColor(currentIssue.status) : 'grey'}>
            {currentIssue ? statusText(currentIssue.status) : '无当前期'}
          </Tag>
          <Tag color={lottery.saleEnabled ? 'green' : 'grey'}>
            {lottery.saleEnabled ? '开售' : '停售'}
          </Tag>
        </div>
      </div>

      <div className="mt-2 flex flex-wrap gap-1.5">
        <Tag color="cyan">{numberTypeText(lottery.numberType)}</Tag>
        <Tag color={drawModeColor(lottery.drawMode)}>{drawModeText(lottery.drawMode)}</Tag>
        <Tag color={controlEnabled ? 'red' : 'grey'}>
          {controlEnabled ? '控制开奖' : '未控制'}
        </Tag>
        {item.waitingIssueCount > 0 && item.waitingIssue?.id !== currentIssue?.id ? (
          <Tag color="orange">待补开奖 {item.waitingIssueCount}</Tag>
        ) : null}
      </div>

      <div className="mt-3 grid grid-cols-2 gap-2">
        <CountdownBlock
          icon={<Clock3 size={16} />}
          label="封盘"
          value={countdownText(currentIssue?.saleClosedAt, now, '已到封盘', '暂无期号')}
        />
        <CountdownBlock
          icon={<Timer size={16} />}
          label="开奖"
          value={drawCountdownText(currentIssue, now, schedulerStatus)}
        />
      </div>

      <div className="mt-2 grid grid-cols-2 gap-2">
        <CompactInfoRow
          icon={<Hash size={13} />}
          label="期号"
          meta={
            currentIssue
              ? `封 ${formatTimePoint(currentIssue.saleClosedAt)} / 开 ${formatTimePoint(currentIssue.scheduledAt)}`
              : `历史 ${item.issueCount} 期`
          }
          value={currentIssue?.issue ?? '暂无当前期'}
        />
        <CompactInfoRow
          icon={<Radio size={13} />}
          label={drawNumberLabel}
          meta={
            recentDrawnIssue
              ? `第 ${recentDrawnIssue.issue} 期 · ${recentDrawnIssue.drawnAt ? formatTimePoint(recentDrawnIssue.drawnAt) : '已开奖'}`
              : undefined
          }
          value={drawNumber ?? '待开奖'}
          valueClassName={drawNumber ? 'font-mono text-sm text-ink' : 'text-sm text-slate-400'}
        />
        <CompactInfoRow
          icon={<ClipboardList size={13} />}
          label="本期下注"
          meta={`最近 ${item.orders.length} 单`}
          value={`${item.currentIssueOrders.length} 单 / ${formatMoney(currentOrderAmountMinor)}`}
          valueClassName="text-sm text-ink"
        />
      </div>

      <div className="mt-2 flex min-w-0 items-center justify-between gap-2 rounded border border-slate-100 bg-slate-50 px-2.5 py-2">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 text-[11px] font-medium text-slate-500">
            <Settings2 size={13} />
            开奖控制
          </div>
          <div
            className={`mt-1 truncate text-sm font-semibold ${
              controlEnabled ? 'font-mono text-rose-700' : 'text-slate-400'
            }`}
            title={controlEnabled ? drawControl?.drawNumber ?? '-' : '未启用'}
          >
            {controlEnabled ? drawControl?.drawNumber ?? '-' : '未启用'}
          </div>
          {drawControl?.updatedAt ? (
            <div className="mt-0.5 truncate text-[11px] text-slate-500">
              {controlTargetText(drawControl)} · 更新 {formatTimePoint(drawControl.updatedAt)}
            </div>
          ) : null}
        </div>
        <div className="flex shrink-0 flex-col gap-1.5">
          <Button
            icon={<Settings2 size={14} />}
            size="small"
            onClick={() => onOpenControl(item)}
          >
            控制
          </Button>
          {lottery.drawMode === 'api' ? (
            <Button
              icon={<RefreshCcw size={14} />}
              loading={syncing}
              size="small"
              theme="light"
              onClick={() => onSyncSource(item)}
            >
              立即同步
            </Button>
          ) : null}
        </div>
      </div>

      <div className="mt-2 flex items-center justify-between gap-2 text-[11px] text-slate-500">
        <span className="flex items-center gap-1">
          <Activity size={13} />
          期号 {item.issueCount} 个
        </span>
        <span className="truncate">{lottery.drawMode === 'api' ? '开奖源同步' : '本地调度'}</span>
      </div>
    </Card>
  );
}

function CompactInfoRow({
  action,
  icon,
  label,
  meta,
  value,
  valueClassName = 'font-mono text-sm text-ink',
}: {
  action?: ReactNode;
  icon: ReactNode;
  label: string;
  meta?: string;
  value: string;
  valueClassName?: string;
}) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-2 rounded border border-slate-100 bg-slate-50 px-2.5 py-2">
      <div className="min-w-0">
        <div className="flex items-center gap-1.5 text-[11px] font-medium text-slate-500">
          {icon}
          <span>{label}</span>
        </div>
        <div className={`mt-1 truncate font-semibold ${valueClassName}`} title={value}>
          {value}
        </div>
        {meta ? (
          <div className="mt-0.5 truncate text-[11px] text-slate-500" title={meta}>
            {meta}
          </div>
        ) : null}
      </div>
      {action ? <div className="shrink-0">{action}</div> : null}
    </div>
  );
}

function DrawControlSideSheet({
  error,
  form,
  item,
  onChange,
  onClose,
  onSubmit,
  saving,
  visible,
}: {
  error: string | null;
  form: LotteryDrawControlFormState;
  item: LotteryConsoleItem | null;
  onChange: (form: LotteryDrawControlFormState) => void;
  onClose: () => void;
  onSubmit: () => void;
  saving: boolean;
  visible: boolean;
}) {
  const lottery = item?.lottery ?? null;
  const inputMeta = lottery ? drawNumberInputMeta(lottery.numberType) : null;
  const controlOrders = item ? controlCandidateOrders(item) : [];
  const visibleOrders = item ? visibleConsoleOrders(item, form) : [];
  const saveDisabled =
    saving ||
    (form.enabled &&
      (!form.drawNumber.trim() ||
        (form.targetScope === 'issue' && !form.targetIssue.trim()) ||
        (form.targetScope === 'order' && !form.targetOrderId.trim())));

  return (
    <SideSheet
      aria-label="控制开奖号码"
      title="控制开奖号码"
      visible={visible}
      width={760}
      onCancel={onClose}
    >
      {lottery ? (
        <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
          <div className="rounded-md border border-line p-3">
            <div className="text-base font-semibold text-ink">{lottery.name}</div>
            <div className="mt-1 flex flex-wrap gap-2">
              <Tag color="cyan">{numberTypeText(lottery.numberType)}</Tag>
              <Tag color={drawModeColor(lottery.drawMode)}>
                {drawModeText(lottery.drawMode)}
              </Tag>
            </div>
          </div>

          {error ? (
            <Banner
              type="danger"
              title="保存控制失败"
              description={error}
            />
          ) : null}

          <label className="flex min-h-10 items-center gap-2 rounded border border-line px-3 py-2 text-sm text-slate-700">
            <input
              checked={form.enabled}
              className="h-4 w-4 rounded border-line text-teal-600"
              type="checkbox"
              onChange={(event) =>
                onChange({ ...form, enabled: event.target.checked })
              }
            />
            <span>启用控制开奖</span>
          </label>

          <Field label={`开奖号码（${numberTypeText(lottery.numberType)}）`}>
            <Input
              className="form-input font-mono"
              disabled={!form.enabled}
              maxLength={inputMeta?.maxLength}
              placeholder={inputMeta?.placeholder}
              value={form.drawNumber}
              onChange={(value) =>
                onChange({ ...form, drawNumber: value })
              }
            />
          </Field>

          <Field label="控制范围">
            <Select
              className="w-full"
              disabled={!form.enabled}
              value={form.targetScope}
              onChange={(value) =>
                onChange(controlFormWithScope(item, form, String(value) as DrawControlTargetScope))
              }
            >
              <Select.Option value="lottery">整个彩种后续开奖</Select.Option>
              <Select.Option value="issue">指定期号</Select.Option>
              <Select.Option value="order">指定订单所在期号</Select.Option>
            </Select>
          </Field>

          {form.targetScope === 'issue' ? (
            <Field label="控制期号">
              <Select
                className="w-full"
                disabled={!form.enabled}
                placeholder="请选择要控制的期号"
                value={form.targetIssue || undefined}
                onChange={(value) =>
                  onChange({
                    ...form,
                    targetIssue: String(value ?? ''),
                    targetOrderId: '',
                  })
                }
              >
                {(item?.issues ?? []).map((issue) => (
                  <Select.Option key={issue.id} value={issue.issue}>
                    {issue.issue} · {statusText(issue.status)} · 开 {formatTimePoint(issue.scheduledAt)}
                  </Select.Option>
                ))}
              </Select>
            </Field>
          ) : null}

          {form.targetScope === 'order' ? (
            <Field label="目标订单">
              <Select
                className="w-full"
                disabled={!form.enabled}
                placeholder="请选择要控制的订单"
                value={form.targetOrderId || undefined}
                onChange={(value) => {
                  const order = controlOrders.find(
                    (item) => item.id === String(value ?? ''),
                  );
                  onChange({
                    ...form,
                    targetIssue: order?.issue ?? '',
                    targetOrderId: order?.id ?? '',
                  });
                }}
              >
                {controlOrders.map((order) => (
                  <Select.Option key={order.id} value={order.id}>
                    <div className="min-w-0">
                      <div className="truncate">
                        {order.id} · 用户 {formatOrderUser(order)} · 第 {order.issue} 期 · {formatMoney(order.amountMinor)}
                      </div>
                      <div className="truncate text-xs text-slate-500">
                        {formatBetInfoSummary(order.selection, order.expandedBets)}
                      </div>
                    </div>
                  </Select.Option>
                ))}
              </Select>
            </Field>
          ) : null}

          <section className="rounded-md border border-line p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <h3 className="text-sm font-semibold text-ink">用户下单信息</h3>
                <p className="mt-1 text-xs text-slate-500">
                  默认显示当前控制范围相关订单，订单控制只匹配该订单所在期号。
                </p>
              </div>
              <Tag color="cyan">{visibleOrders.length} 单</Tag>
            </div>
            {visibleOrders.length > 0 ? (
              <div className="mt-3 max-h-[320px] overflow-auto">
                <table className="w-full min-w-[1040px] text-left text-xs">
                  <thead className="border-b border-line text-slate-500">
                    <tr>
                      <th className="py-2 pr-3 font-medium">订单</th>
                      <th className="py-2 pr-3 font-medium">用户</th>
                      <th className="py-2 pr-3 font-medium">期号</th>
                      <th className="py-2 pr-3 font-medium">玩法</th>
                      <th className="py-2 pr-3 font-medium">下注信息</th>
                      <th className="py-2 pr-3 font-medium">金额</th>
                      <th className="py-2 pr-3 font-medium">状态</th>
                      <th className="py-2 pr-3 font-medium">控制</th>
                    </tr>
                  </thead>
                  <tbody>
                    {visibleOrders.map((order) => (
                      <tr key={order.id} className="border-b border-slate-100">
                        <td className="py-2 pr-3 font-mono font-semibold text-ink">
                          {order.id}
                        </td>
                        <td className="py-2 pr-3">
                          <div className="font-medium text-slate-700">
                            {order.username ?? '未知用户'}
                          </div>
                          <div className="mt-1 text-[11px] text-slate-400">
                            {order.userId}
                          </div>
                        </td>
                        <td className="py-2 pr-3 text-slate-600">{order.issue}</td>
                        <td className="py-2 pr-3 text-slate-600">{order.ruleCode}</td>
                        <td className="py-2 pr-3">
                          <OrderBetInfo compact expandedLimit={6} order={order} />
                        </td>
                        <td className="py-2 pr-3 text-slate-600">
                          {formatMoney(order.amountMinor)}
                        </td>
                        <td className="py-2 pr-3">
                          <Tag color={orderStatusColor(order.status)}>
                            {orderStatusText(order.status)}
                          </Tag>
                        </td>
                        <td className="py-2 pr-3">
                          <Button
                            disabled={order.status !== 'pendingDraw'}
                            size="small"
                            onClick={() =>
                              onChange({
                                ...form,
                                enabled: true,
                                targetIssue: order.issue,
                                targetOrderId: order.id,
                                targetScope: 'order',
                              })
                            }
                          >
                            控制此单
                          </Button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="mt-3 rounded border border-dashed border-line p-4 text-sm text-slate-500">
                当前范围暂无用户下注订单。
              </div>
            )}
          </section>

          <div className="flex flex-wrap gap-2">
            <Button
              disabled={saveDisabled}
              icon={<Save size={15} />}
              loading={saving}
              theme="solid"
              onClick={onSubmit}
            >
              保存控制
            </Button>
            <Button disabled={saving} onClick={onClose}>
              关闭
            </Button>
          </div>
        </form>
      ) : (
        <div className="rounded-md border border-line p-4 text-sm text-slate-500">
          暂无可维护彩种。
        </div>
      )}
    </SideSheet>
  );
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <label className="block">
      <span className="text-xs font-medium text-slate-500">{label}</span>
      <div className="mt-1">{children}</div>
    </label>
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
    <div className="min-w-0 rounded border border-slate-100 bg-white px-2.5 py-2">
      <div className="flex items-center gap-1.5 whitespace-nowrap text-[11px] font-medium text-slate-500">
        {icon}
        {label}
      </div>
      <div className="mt-1 truncate font-mono text-sm font-semibold text-ink" title={value}>
        {value}
      </div>
    </div>
  );
}

function lotteryConsoleStatusFilterOptions(
  items: LotteryConsoleItem[],
  now: Date,
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
      count: items.filter((item) => lotteryConsoleItemIsWaitingDraw(item, now))
        .length,
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
  now: Date,
) {
  switch (filter) {
    case 'all':
      return true;
    case 'closed':
      return lotteryConsoleItemIsWaitingDraw(item, now);
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

function lotteryConsoleItemIsWaitingDraw(item: LotteryConsoleItem, now: Date) {
  const issue = item.currentIssue;
  if (item.waitingIssueCount > 0) {
    return true;
  }
  if (!issue) {
    return false;
  }
  if (issue.status === 'closed') {
    return true;
  }
  const scheduledAt = parseTimeLabel(issue.scheduledAt);
  return issue.status === 'open' && scheduledAt !== null && scheduledAt <= now.getTime();
}

function lotteryConsoleItem(
  lottery: LotteryKind,
  allIssues: DrawIssue[],
  allOrders: OrderDetail[],
  drawControl: LotteryDrawControl | null,
): LotteryConsoleItem {
  const issues = allIssues.filter((issue) => issue.lotteryId === lottery.id);
  const orders = allOrders
    .filter((order) => order.lotteryId === lottery.id)
    .sort((left, right) => orderTimeValue(right) - orderTimeValue(left));
  const issuesByStatus = (status: DrawIssueStatus) =>
    issues.filter((issue) => issue.status === status);
  const openIssue =
    pickLatestIssue(issuesByStatus('open'));
  const waitingIssues = issues
    .filter((issue) => issue.status === 'closed')
    .sort((left, right) => issueTimeValue(left) - issueTimeValue(right));
  const waitingIssue = waitingIssues[0] ?? null;
  const currentIssue =
    openIssue ??
    pickLatestIssue(issuesByStatus('closed')) ??
    pickLatestIssue(issuesByStatus('drawn')) ??
    pickLatestIssue(issuesByStatus('cancelled')) ??
    null;
  const recentDrawnIssue =
    issuesByStatus('drawn')
      .filter((issue) => issue.drawNumber)
      .sort((left, right) => latestIssueTimeValue(right) - latestIssueTimeValue(left))[0] ??
    null;

  return {
    currentIssue,
    currentIssueOrders: currentIssue
      ? orders.filter((order) => order.issue === currentIssue.issue)
      : [],
    drawControl,
    issues,
    issueCount: issues.length,
    lottery,
    orders,
    recentDrawnIssue,
    waitingIssue,
    waitingIssueCount: waitingIssues.length,
  };
}

function pickLatestIssue(issues: DrawIssue[]) {
  return issues.sort((left, right) => issueTimeValue(right) - issueTimeValue(left))[0] ?? null;
}

function emptyDrawControlForm(): LotteryDrawControlFormState {
  return {
    enabled: false,
    drawNumber: '',
    targetIssue: '',
    targetOrderId: '',
    targetScope: 'lottery',
  };
}

function drawControlFormFromControl(
  control: LotteryDrawControl | null,
): LotteryDrawControlFormState {
  return {
    enabled: control?.enabled ?? false,
    drawNumber: control?.drawNumber ?? '',
    targetIssue: control?.targetIssue ?? '',
    targetOrderId: control?.targetOrderId ?? '',
    targetScope: control?.targetScope ?? 'lottery',
  };
}

function controlFormWithScope(
  item: LotteryConsoleItem | null,
  form: LotteryDrawControlFormState,
  targetScope: DrawControlTargetScope,
): LotteryDrawControlFormState {
  if (targetScope === 'lottery') {
    return {
      ...form,
      targetIssue: '',
      targetOrderId: '',
      targetScope,
    };
  }
  if (targetScope === 'issue') {
    return {
      ...form,
      targetIssue: form.targetIssue || item?.currentIssue?.issue || item?.issues[0]?.issue || '',
      targetOrderId: '',
      targetScope,
    };
  }

  const order =
    item?.orders.find((order) => order.id === form.targetOrderId) ??
    controlCandidateOrders(item).find((order) => order.issue === item?.currentIssue?.issue) ??
    controlCandidateOrders(item)[0] ??
    null;

  return {
    ...form,
    targetIssue: order?.issue ?? '',
    targetOrderId: order?.id ?? '',
    targetScope,
  };
}

function controlCandidateOrders(item: LotteryConsoleItem | null) {
  return (item?.orders ?? []).filter((order) => order.status === 'pendingDraw');
}

function visibleConsoleOrders(
  item: LotteryConsoleItem,
  form: LotteryDrawControlFormState,
) {
  if (form.targetScope === 'order' && form.targetOrderId.trim()) {
    return item.orders.filter((order) => order.id === form.targetOrderId.trim());
  }
  if (form.targetScope === 'issue' && form.targetIssue.trim()) {
    return item.orders.filter((order) => order.issue === form.targetIssue.trim());
  }
  if (item.currentIssueOrders.length > 0) {
    return item.currentIssueOrders;
  }
  return item.orders.slice(0, 12);
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

function drawCountdownText(
  issue: DrawIssue | null | undefined,
  now: Date,
  schedulerStatus: DrawSchedulerStatus | null,
) {
  const targetMs = parseTimeLabel(issue?.scheduledAt);
  if (!issue || targetMs === null) {
    return '暂无期号';
  }

  const diffMs = targetMs - now.getTime();
  if (diffMs > 0) {
    return formatDuration(diffMs);
  }
  if (!schedulerStatus?.enabled) {
    return '调度已关闭';
  }
  if (issue.drawMode === 'api') {
    return '等待开奖源';
  }
  return '等待调度';
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

function orderTimeValue(order: OrderDetail) {
  return parseTimeLabel(order.createdAt) ?? 0;
}

function controlTargetText(control: LotteryDrawControl) {
  if (!control.enabled) {
    return '未启用';
  }
  if (control.targetScope === 'issue') {
    return control.targetIssue ? `期号 ${control.targetIssue}` : '指定期号';
  }
  if (control.targetScope === 'order') {
    return control.targetOrderId ? `订单 ${control.targetOrderId}` : '指定订单';
  }
  return '整彩种';
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

function formatTimePoint(value: string | null | undefined) {
  const timestamp = parseTimeLabel(value);
  if (timestamp === null) {
    return value || '-';
  }

  return formatClock(new Date(timestamp));
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

function orderStatusText(status: OrderStatus) {
  const labels: Record<OrderStatus, string> = {
    cancelled: '已取消',
    lost: '未中奖',
    pendingDraw: '待开奖',
    won: '已中奖',
  };
  return labels[status];
}

function orderStatusColor(status: OrderStatus) {
  const colors: Record<OrderStatus, 'blue' | 'green' | 'grey' | 'red'> = {
    cancelled: 'grey',
    lost: 'red',
    pendingDraw: 'blue',
    won: 'green',
  };
  return colors[status];
}

function formatOrderUser(order: OrderDetail) {
  return `${order.username ?? '未知用户'}（${order.userId}）`;
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

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
