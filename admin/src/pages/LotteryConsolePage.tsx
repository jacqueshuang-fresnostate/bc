import {
  Input,
  Banner,
  Button,
  Card,
  Select,
  SideSheet,
  Spin,
  Switch,
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
import { useControlGroupBuyPlans } from '../hooks/useControlGroupBuyPlans';
import { useLotteryConsole } from '../hooks/useLotteryConsole';
import { usePlayRules } from '../hooks/usePlayRules';
import type { DrawMode, LotteryKind } from '../types/dashboard';
import type {
  DrawControlTargetScope,
  DrawIssue,
  DrawIssueStatus,
  LotteryDrawControl,
} from '../types/draws';
import type { GroupBuyPlan, GroupBuyPlanStatus } from '../types/groupBuy';
import type { OrderDetail, OrderStatus } from '../types/orders';
import type { PlayRuleSummary } from '../types/playRules';
import type { DrawSchedulerStatus } from '../types/scheduler';
import { formatDateTime, formatMoney } from '../utils/format';
import {
  drawNumberInputMeta,
  lotteryNumberTypeText as numberTypeText,
} from '../utils/lotteries';
import {
  formatBetInfoSummary,
  formatGroupBuyNumbersSelection,
} from '../utils/orderBetInfo';
import { formatPlayRuleLabel } from '../utils/playRules';

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
  const { rules: playRules } = usePlayRules();
  const [now, setNow] = useState(() => new Date());
  const [statusFilter, setStatusFilter] =
    useState<LotteryConsoleStatusFilter>('saleEnabled');
  const [nameSearch, setNameSearch] = useState('');
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
  const searchMatchedItems = useMemo(
    () => items.filter((item) => lotteryConsoleItemMatchesName(item, nameSearch)),
    [items, nameSearch],
  );
  const statusFilterOptions = useMemo(
    () => lotteryConsoleStatusFilterOptions(searchMatchedItems, now),
    [searchMatchedItems, now],
  );
  const filteredItems = useMemo(
    () =>
      searchMatchedItems.filter((item) =>
        lotteryConsoleItemMatchesFilter(item, statusFilter, now),
      ),
    [searchMatchedItems, statusFilter, now],
  );
  const selectedControlLotteryId = selectedControlItem?.lottery.id ?? null;
  const controlGroupBuyIssue = useMemo(
    () =>
      selectedControlItem
        ? groupBuyIssueForControl(selectedControlItem, controlForm)
        : '',
    [
      selectedControlItem,
      controlForm.targetIssue,
      controlForm.targetOrderId,
      controlForm.targetScope,
    ],
  );
  const {
    error: controlGroupBuyError,
    loading: controlGroupBuyLoading,
    plans: controlGroupBuyPlans,
    refresh: refreshControlGroupBuys,
  } = useControlGroupBuyPlans(selectedControlLotteryId, controlGroupBuyIssue);

  const refreshControlData = () => {
    refresh();
    refreshControlGroupBuys();
  };

  useEffect(() => {
    if (!selectedControlLotteryId) {
      return;
    }
    const nextItem = items.find((item) => item.lottery.id === selectedControlLotteryId);
    if (!nextItem) {
      setSelectedControlItem(null);
      return;
    }
    setSelectedControlItem(nextItem);
    setControlForm((current) => normalizeControlFormForIssueStatus(nextItem, current));
  }, [items, selectedControlLotteryId]);

  const metrics = useMemo(() => {
    const saleEnabledCount = lotteries.filter((lottery) => lottery.saleEnabled).length;
    const openCount = items.filter(
      (item) => item.currentIssue?.status === 'open',
    ).length;
    const waitingDrawCount = items.filter((item) =>
      lotteryConsoleItemIsWaitingDraw(item, now),
    ).length;
    const controlEnabledCount = items.filter(
      (item) => item.lottery.drawControlEnabled && item.drawControl?.enabled,
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
    if (!item.lottery.drawControlEnabled) {
      Toast.warning('该彩种未开启开奖号码控制');
      return;
    }
    refreshControlData();
    setSelectedControlItem(item);
    setControlForm(
      normalizeControlFormForIssueStatus(
        item,
        drawControlFormFromControl(item.drawControl, item),
      ),
    );
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
      const normalizedForm = normalizeControlFormForIssueStatus(
        selectedControlItem,
        controlForm,
      );
      if (normalizedForm !== controlForm) {
        setControlForm(normalizedForm);
      }
      const trimmedDrawNumber = normalizedForm.drawNumber.trim();
      await saveDrawControl(selectedControlItem.lottery.id, {
        enabled: normalizedForm.enabled,
        drawNumber: normalizedForm.enabled ? trimmedDrawNumber || null : null,
        targetIssue: normalizedForm.enabled ? normalizedForm.targetIssue.trim() || null : null,
        targetOrderId: normalizedForm.enabled ? normalizedForm.targetOrderId.trim() || null : null,
        targetScope: normalizedForm.targetScope,
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
        nameSearch={nameSearch}
        options={statusFilterOptions}
        totalCount={items.length}
        onChange={setStatusFilter}
        onNameSearchChange={setNameSearch}
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
        groupBuyError={controlGroupBuyError}
        groupBuyIssue={controlGroupBuyIssue}
        groupBuyLoading={controlGroupBuyLoading}
        groupBuyPlans={controlGroupBuyPlans}
        item={selectedControlItem}
        playRules={playRules}
        refreshing={loading || controlGroupBuyLoading}
        saving={controlSaving}
        visible={Boolean(selectedControlItem)}
        onChange={setControlForm}
        onClose={closeControlSheet}
        onRefreshOrders={refreshControlData}
        onSubmit={() => void submitDrawControl()}
      />
    </div>
  );
}

function LotteryConsoleStatusFilterBar({
  active,
  filteredCount,
  nameSearch,
  onChange,
  onNameSearchChange,
  options,
  totalCount,
}: {
  active: LotteryConsoleStatusFilter;
  filteredCount: number;
  nameSearch: string;
  onChange: (filter: LotteryConsoleStatusFilter) => void;
  onNameSearchChange: (keyword: string) => void;
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
        <div className="flex flex-col gap-2 lg:flex-row lg:items-center">
          <Input
            className="form-input min-w-[220px]"
            placeholder="搜索彩种名称"
            value={nameSearch}
            onChange={onNameSearchChange}
          />
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
  const { currentIssue, lottery, recentDrawnIssue } = item;
  const currentIssueDrawNumber =
    currentIssue?.status === 'drawn' && currentIssue.drawNumber
      ? currentIssue.drawNumber
      : null;
  const drawNumber = currentIssueDrawNumber ?? recentDrawnIssue?.drawNumber ?? null;
  const drawNumberLabel = currentIssueDrawNumber ? '本期开奖' : '最近开奖';
  const controlAllowed = lottery.drawControlEnabled;
  const visibleDrawControl = controlAllowed ? displayableDrawControl(item, now) : null;
  const controlEnabled = Boolean(visibleDrawControl);
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
        <Tag color={controlEnabled ? 'red' : controlAllowed ? 'grey' : 'blue'}>
          {controlEnabled ? '控制开奖' : controlAllowed ? '未控制' : '不控制'}
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
            title={
              controlEnabled
                ? visibleDrawControl?.drawNumber ?? '-'
                : controlAllowed
                  ? '未启用'
                  : '未开启控制'
            }
          >
            {controlEnabled
              ? visibleDrawControl?.drawNumber ?? '-'
              : controlAllowed
                ? '未启用'
                : '未开启控制'}
          </div>
          {controlAllowed && visibleDrawControl?.updatedAt ? (
            <div className="mt-0.5 truncate text-[11px] text-slate-500">
              {controlTargetText(visibleDrawControl)} · 更新{' '}
              {formatTimePoint(visibleDrawControl.updatedAt)}
            </div>
          ) : null}
        </div>
        <div className="flex shrink-0 flex-col gap-1.5">
          {controlAllowed ? (
            <Button
              icon={<Settings2 size={14} />}
              size="small"
              onClick={() => onOpenControl(item)}
            >
              控制
            </Button>
          ) : null}
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
  groupBuyError,
  groupBuyIssue,
  groupBuyLoading,
  groupBuyPlans,
  item,
  onChange,
  onClose,
  onRefreshOrders,
  onSubmit,
  playRules,
  refreshing,
  saving,
  visible,
}: {
  error: string | null;
  form: LotteryDrawControlFormState;
  groupBuyError: string | null;
  groupBuyIssue: string;
  groupBuyLoading: boolean;
  groupBuyPlans: GroupBuyPlan[];
  item: LotteryConsoleItem | null;
  onChange: (form: LotteryDrawControlFormState) => void;
  onClose: () => void;
  onRefreshOrders: () => void;
  onSubmit: () => void;
  playRules: PlayRuleSummary[];
  refreshing: boolean;
  saving: boolean;
  visible: boolean;
}) {
  const lottery = item?.lottery ?? null;
  const inputMeta = lottery ? drawNumberInputMeta(lottery.numberType) : null;
  const controlIssues = item ? controlCandidateIssues(item) : [];
  const controlOrders = item ? controlCandidateOrders(item) : [];
  const visibleOrders = item ? visibleConsoleOrders(item, form) : [];
  const groupBuyRows = groupBuyParticipantRows(groupBuyPlans);
  const targetIssueInactive = controlFormTargetsInactiveIssue(item, form);
  const targetSelectDisabled = !form.enabled && !targetIssueInactive;
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
      width="80%"
      onCancel={onClose}
    >
      {lottery ? (
        <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
          {error ? (
            <Banner type="danger" title="保存控制失败" description={error} />
          ) : null}

          <section className="grid gap-3 lg:grid-cols-[1fr_1.1fr_1.4fr]">
            <div className="rounded-md border border-line bg-slate-50/70 p-3">
              <div className="text-xs font-medium text-slate-500">控制彩种</div>
              <div className="mt-2 truncate text-lg font-semibold text-ink" title={lottery.name}>
                {lottery.name}
              </div>
              <div className="mt-2 flex flex-wrap gap-1.5">
                <Tag color="cyan">{numberTypeText(lottery.numberType)}</Tag>
                <Tag color={drawModeColor(lottery.drawMode)}>
                  {drawModeText(lottery.drawMode)}
                </Tag>
              </div>
            </div>

            <div className="rounded-md border border-line bg-white p-3">
              <div className="text-xs font-medium text-slate-500">控制范围</div>
              <Select
                className="mt-2 w-full"
                disabled={targetSelectDisabled}
                value={form.targetScope}
                onChange={(value) => {
                  onChange(
                    controlFormWithScope(
                      item,
                      form,
                      String(value) as DrawControlTargetScope,
                    ),
                  );
                  onRefreshOrders();
                }}
              >
                <Select.Option value="lottery">整个彩种后续开奖</Select.Option>
                <Select.Option value="issue">指定期号</Select.Option>
                <Select.Option value="order">指定订单所在期号</Select.Option>
              </Select>
            </div>

            <div className="rounded-md border border-line bg-white p-3">
              <div className="text-xs font-medium text-slate-500">
                {form.targetScope === 'order' ? '指定订单' : '控制期号'}
              </div>
              {form.targetScope === 'issue' ? (
              <Select
                className="mt-2 w-full"
                disabled={targetSelectDisabled}
                placeholder="请选择要控制的期号"
                value={form.targetIssue || undefined}
                onChange={(value) => {
                  onChange(controlFormWithIssue(item, form, String(value ?? '')));
                  onRefreshOrders();
                }}
              >
                {controlIssues.map((issue) => (
                  <Select.Option key={issue.id} value={issue.issue}>
                    {issue.issue} · {statusText(issue.status)} · 开 {formatTimePoint(issue.scheduledAt)}
                  </Select.Option>
                ))}
              </Select>
              ) : null}
              {form.targetScope === 'order' ? (
              <Select
                className="mt-2 w-full"
                disabled={targetSelectDisabled}
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
                  onRefreshOrders();
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
              ) : null}
              {form.targetScope === 'lottery' ? (
                <div className="mt-2 rounded border border-dashed border-line bg-slate-50 px-3 py-2 text-sm text-slate-600">
                  当前设置将作用于该彩种后续所有未开奖期号。
                </div>
              ) : null}
            </div>
          </section>

          <section className="grid gap-3 lg:grid-cols-[1fr_1.25fr]">
            <div className="rounded-md border border-line bg-white p-3">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="text-xs font-medium text-slate-500">
                    总开关（是/否）
                  </div>
                  <div className="mt-1 text-base font-semibold text-ink">
                    {form.enabled ? '已启用控制开奖' : '未启用控制开奖'}
                  </div>
                  <p className="mt-1 text-xs leading-5 text-slate-500">
                    开启后将按下方号码开奖；关闭后恢复正常开奖流程。
                  </p>
                </div>
                <div className="flex shrink-0 flex-col items-end gap-2">
                  <Switch
                    checked={form.enabled}
                    disabled={targetIssueInactive}
                    onChange={(checked) =>
                      onChange(
                        normalizeControlFormForIssueStatus(item, {
                          ...form,
                          enabled: checked,
                        }),
                      )
                    }
                  />
                  <Tag color={form.enabled ? 'red' : 'grey'}>
                    {form.enabled ? '是' : '否'}
                  </Tag>
                </div>
              </div>
              {targetIssueInactive ? (
                <Banner
                  className="mt-3"
                  type="warning"
                  title="指定期号已结束"
                  description="已开奖或已取消期号不能继续启用开奖号码控制，系统已自动取消勾选。请选择未结束期号后再启用。"
                />
              ) : null}
            </div>

            <div className="rounded-md border border-line bg-white p-3">
              <div className="text-xs font-medium text-slate-500">
                开奖号码（{numberTypeText(lottery.numberType)}）
              </div>
              <Input
                className="form-input mt-2 font-mono text-lg font-semibold"
                disabled={!form.enabled}
                maxLength={inputMeta?.maxLength}
                placeholder={inputMeta?.placeholder}
                value={form.drawNumber}
                onChange={(value) => onChange({ ...form, drawNumber: value })}
              />
              <p className="mt-2 text-xs leading-5 text-slate-500">
                多个号码用英文逗号分隔，例如 1,2,3,4,5。
              </p>
            </div>
          </section>

          <section className="rounded-md border border-line p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <h3 className="text-sm font-semibold text-ink">用户下单信息</h3>
                <p className="mt-1 text-xs text-slate-500">
                  默认显示当前控制范围相关订单，订单控制只匹配该订单所在期号。
                </p>
              </div>
              <div className="flex items-center gap-2">
                <Tag color="cyan">{visibleOrders.length} 单</Tag>
                <Button
                  htmlType="button"
                  icon={<RefreshCcw size={14} />}
                  loading={refreshing}
                  size="small"
                  onClick={onRefreshOrders}
                >
                  刷新订单/认购
                </Button>
              </div>
            </div>
            {visibleOrders.length > 0 ? (
              <div className="mt-3 max-h-[320px] overflow-auto">
                <table className="w-full min-w-[1240px] text-left text-xs">
                  <thead className="border-b border-line text-slate-500">
                    <tr>
                      <th className="py-2 pr-3 font-medium">订单</th>
                      <th className="py-2 pr-3 font-medium">用户</th>
                      <th className="py-2 pr-3 font-medium">来源</th>
                      <th className="py-2 pr-3 font-medium">下单时间</th>
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
                        <td className="py-2 pr-3">
                          <Tag color={orderSourceColor(order.orderSource)}>
                            {orderSourceText(order.orderSource)}
                          </Tag>
                        </td>
                        <td className="py-2 pr-3 whitespace-nowrap text-slate-600">
                          {formatDateTime(order.createdAt, order.createdAt || '-')}
                        </td>
                        <td className="py-2 pr-3 text-slate-600">{order.issue}</td>
                        <td className="py-2 pr-3 text-slate-600">
                          {formatPlayRuleLabel(order.ruleCode, playRules)}
                        </td>
                        <td className="py-2 pr-3">
                          <OrderBetInfo compact order={order} showExpandedBets={false} />
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

          <section className="rounded-md border border-line p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <h3 className="text-sm font-semibold text-ink">合买认购记录</h3>
                <p className="mt-1 text-xs text-slate-500">
                  显示当前期号的合买计划和认购明细，未成单、未满单也会展示。
                </p>
              </div>
              <div className="flex items-center gap-2">
                {groupBuyIssue ? (
                  <Tag color="grey">期号 {groupBuyIssue}</Tag>
                ) : null}
                <Tag color="orange">{groupBuyRows.length} 条认购</Tag>
              </div>
            </div>
            {groupBuyError ? (
              <Banner
                className="mt-3"
                type="warning"
                title="合买认购记录读取失败"
                description={groupBuyError}
              />
            ) : null}
            {groupBuyLoading ? (
              <div className="mt-3 rounded border border-dashed border-line p-4 text-sm text-slate-500">
                正在加载合买认购记录...
              </div>
            ) : groupBuyRows.length > 0 ? (
              <div className="mt-3 max-h-[300px] overflow-auto">
                <table className="w-full min-w-[1320px] text-left text-xs">
                  <thead className="border-b border-line text-slate-500">
                    <tr>
                      <th className="py-2 pr-3 font-medium">合买计划</th>
                      <th className="py-2 pr-3 font-medium">状态</th>
                      <th className="py-2 pr-3 font-medium">参与用户</th>
                      <th className="py-2 pr-3 font-medium">认购时间</th>
                      <th className="py-2 pr-3 font-medium">期号</th>
                      <th className="py-2 pr-3 font-medium">玩法</th>
                      <th className="py-2 pr-3 font-medium">投注内容</th>
                      <th className="py-2 pr-3 font-medium">认购金额</th>
                      <th className="py-2 pr-3 font-medium">份数</th>
                      <th className="py-2 pr-3 font-medium">占比</th>
                      <th className="py-2 pr-3 font-medium">进度</th>
                      <th className="py-2 pr-3 font-medium">真实订单</th>
                    </tr>
                  </thead>
                  <tbody>
                    {groupBuyRows.map(({ participant, plan }) => (
                      <tr
                        key={`${plan.id}:${participant.id}`}
                        className="border-b border-slate-100"
                      >
                        <td className="py-2 pr-3">
                          <div className="font-mono font-semibold text-ink">
                            {plan.id}
                          </div>
                          <div className="mt-1 max-w-[180px] truncate text-[11px] text-slate-400">
                            {plan.title || '-'}
                          </div>
                        </td>
                        <td className="py-2 pr-3">
                          <Tag color={groupBuyStatusColor(plan.status)}>
                            {groupBuyStatusText(plan.status)}
                          </Tag>
                        </td>
                        <td className="py-2 pr-3">
                          <div className="font-medium text-slate-700">
                            {participant.username || '未知用户'}
                          </div>
                          <div className="mt-1 text-[11px] text-slate-400">
                            {participant.userId}
                          </div>
                        </td>
                        <td className="py-2 pr-3 whitespace-nowrap text-slate-600">
                          {formatDateTime(participant.createdAt, participant.createdAt || '-')}
                        </td>
                        <td className="py-2 pr-3 text-slate-600">{plan.issue}</td>
                        <td className="py-2 pr-3 text-slate-600">
                          {formatGroupBuyPlayRuleLabel(plan.ruleCode, playRules)}
                        </td>
                        <td className="py-2 pr-3">
                          <GroupBuyNumbersInfo
                            numbers={plan.numbers}
                            ruleCode={plan.ruleCode}
                          />
                        </td>
                        <td className="py-2 pr-3 font-semibold text-ink">
                          {formatMoney(participant.amountMinor)}
                        </td>
                        <td className="py-2 pr-3 text-slate-600">
                          {participant.shareCount} 份
                        </td>
                        <td className="py-2 pr-3 text-slate-600">
                          {participantPercent(participant.amountMinor, plan)}%
                        </td>
                        <td className="py-2 pr-3">
                          <div className="min-w-[120px]">
                            <div className="mb-1 flex items-center justify-between text-[11px] text-slate-500">
                              <span>{formatMoney(plan.filledAmountMinor)}</span>
                              <span>{groupBuyProgressPercent(plan)}%</span>
                            </div>
                            <div className="h-1.5 overflow-hidden rounded-full bg-slate-100">
                              <div
                                className="h-full rounded-full bg-orange-500"
                                style={{ width: `${groupBuyProgressPercent(plan)}%` }}
                              />
                            </div>
                          </div>
                        </td>
                        <td className="py-2 pr-3">
                          {plan.orderId ? (
                            <Tag color="green">{plan.orderId}</Tag>
                          ) : (
                            <Tag color="grey">未成单</Tag>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="mt-3 rounded border border-dashed border-line p-4 text-sm text-slate-500">
                当前期号暂无合买认购记录。
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

function GroupBuyNumbersInfo({
  numbers,
  ruleCode,
}: {
  numbers: string;
  ruleCode: string;
}) {
  const lines = formatGroupBuyNumbersSelection(ruleCode, numbers);
  return (
    <div
      className="min-w-[220px] max-w-[320px] space-y-1"
      title={numbers || undefined}
    >
      {lines.map((line) => (
        <div
          key={`${line.label}-${line.value}`}
          className="grid grid-cols-[64px_minmax(0,1fr)] gap-2 text-xs leading-5"
        >
          <span className="whitespace-nowrap text-slate-400">{line.label}</span>
          <span className="min-w-0 break-words font-medium text-slate-700">
            {line.value}
          </span>
        </div>
      ))}
    </div>
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

function lotteryConsoleItemMatchesName(item: LotteryConsoleItem, keyword: string) {
  const normalizedKeyword = keyword.trim().toLowerCase();
  if (!normalizedKeyword) {
    return true;
  }
  return item.lottery.name.toLowerCase().includes(normalizedKeyword);
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
    targetScope: 'issue',
  };
}

function drawControlFormFromControl(
  control: LotteryDrawControl | null,
  item: LotteryConsoleItem | null,
): LotteryDrawControlFormState {
  const targetScope = control?.enabled ? control.targetScope ?? 'issue' : 'issue';
  const targetOrder = item?.orders.find((order) => order.id === control?.targetOrderId);
  const defaultIssue = sellingIssueForControl(item);

  return {
    enabled: control?.enabled ?? false,
    drawNumber: control?.drawNumber ?? '',
    targetIssue:
      targetScope === 'lottery'
        ? ''
        : control?.targetIssue ?? targetOrder?.issue ?? defaultIssue,
    targetOrderId: targetScope === 'order' ? control?.targetOrderId ?? '' : '',
    targetScope,
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
    return normalizeControlFormForIssueStatus(item, {
      ...form,
      targetIssue: form.targetIssue || sellingIssueForControl(item),
      targetOrderId: '',
      targetScope,
    });
  }

  const defaultIssue = sellingIssueForControl(item);
  const candidateOrders = controlCandidateOrders(item);
  const order =
    item?.orders.find((order) => order.id === form.targetOrderId) ??
    candidateOrders.find((order) => order.issue === defaultIssue) ??
    candidateOrders.find((order) => order.issue === item?.currentIssue?.issue) ??
    candidateOrders[0] ??
    null;

  return {
    ...form,
    targetIssue: order?.issue ?? '',
    targetOrderId: order?.id ?? '',
    targetScope,
  };
}

function controlFormWithIssue(
  item: LotteryConsoleItem | null,
  form: LotteryDrawControlFormState,
  targetIssue: string,
): LotteryDrawControlFormState {
  return normalizeControlFormForIssueStatus(item, {
    ...form,
    targetIssue,
    targetOrderId: '',
  });
}

function normalizeControlFormForIssueStatus(
  item: LotteryConsoleItem | null,
  form: LotteryDrawControlFormState,
): LotteryDrawControlFormState {
  if (!form.enabled || !controlFormTargetsInactiveIssue(item, form)) {
    return form;
  }

  return {
    ...form,
    enabled: false,
  };
}

function controlFormTargetsInactiveIssue(
  item: LotteryConsoleItem | null,
  form: LotteryDrawControlFormState,
) {
  if (form.targetScope !== 'issue') {
    return false;
  }
  const targetIssue = form.targetIssue.trim();
  if (!targetIssue) {
    return false;
  }
  const status = issueStatusForControl(item, targetIssue);
  return status === 'drawn' || status === 'cancelled';
}

function issueStatusForControl(
  item: LotteryConsoleItem | null,
  targetIssue: string,
): DrawIssueStatus | null {
  return (
    item?.issues.find((issue) => issue.issue === targetIssue.trim())?.status ?? null
  );
}

function displayableDrawControl(item: LotteryConsoleItem, now: Date) {
  const control = item.drawControl;
  if (!control?.enabled) {
    return null;
  }

  if (control.targetScope === 'lottery') {
    return control;
  }

  const targetIssue = targetIssueForDrawControl(item, control);
  if (!targetIssue || !issueStillDisplayableForControl(targetIssue, now)) {
    return null;
  }

  return control;
}

function targetIssueForDrawControl(
  item: LotteryConsoleItem,
  control: LotteryDrawControl,
) {
  const explicitIssue = control.targetIssue?.trim();
  if (explicitIssue) {
    return item.issues.find((issue) => issue.issue === explicitIssue) ?? null;
  }

  if (control.targetScope !== 'order' || !control.targetOrderId) {
    return null;
  }

  const order = item.orders.find((order) => order.id === control.targetOrderId);
  if (!order) {
    return null;
  }

  return item.issues.find((issue) => issue.issue === order.issue) ?? null;
}

function issueStillDisplayableForControl(issue: DrawIssue, now: Date) {
  if (issue.status !== 'open' && issue.status !== 'closed') {
    return false;
  }

  const scheduledAt = parseTimeLabel(issue.scheduledAt);
  return scheduledAt === null || scheduledAt >= now.getTime();
}

function controlCandidateIssues(item: LotteryConsoleItem | null) {
  return [...(item?.issues ?? [])]
    .filter((issue) => issue.status === 'open' || issue.status === 'closed')
    .sort((left, right) => issueTimeValue(left) - issueTimeValue(right));
}

function controlCandidateOrders(item: LotteryConsoleItem | null) {
  return (item?.orders ?? []).filter((order) => order.status === 'pendingDraw');
}

function sellingIssueForControl(item: LotteryConsoleItem | null | undefined) {
  if (item?.currentIssue?.status === 'open') {
    return item.currentIssue.issue;
  }

  const candidateIssues = controlCandidateIssues(item ?? null);
  const nearestOpenIssue = candidateIssues
    .filter((issue) => issue.status === 'open')
    .sort((left, right) => issueTimeValue(left) - issueTimeValue(right))[0];
  if (nearestOpenIssue) {
    return nearestOpenIssue.issue;
  }

  const latestKnownIssue = candidateIssues.sort(
    (left, right) => issueTimeValue(right) - issueTimeValue(left),
  )[0];
  return item?.currentIssue?.issue ?? latestKnownIssue?.issue ?? '';
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

function groupBuyIssueForControl(
  item: LotteryConsoleItem,
  form: LotteryDrawControlFormState,
) {
  if (form.targetScope === 'order' && form.targetOrderId.trim()) {
    const order = item.orders.find((order) => order.id === form.targetOrderId.trim());
    return order?.issue ?? form.targetIssue.trim();
  }
  if (form.targetScope === 'issue' && form.targetIssue.trim()) {
    return form.targetIssue.trim();
  }
  return sellingIssueForControl(item);
}

function groupBuyParticipantRows(plans: GroupBuyPlan[]) {
  return plans.flatMap((plan) =>
    plan.participants.map((participant) => ({
      participant,
      plan,
    })),
  );
}

function participantPercent(amountMinor: number, plan: GroupBuyPlan) {
  if (plan.totalAmountMinor <= 0) {
    return 0;
  }

  return Math.round((amountMinor / plan.totalAmountMinor) * 10000) / 100;
}

function groupBuyProgressPercent(plan: GroupBuyPlan) {
  if (plan.totalAmountMinor <= 0) {
    return 0;
  }
  const percent = Math.round((plan.filledAmountMinor / plan.totalAmountMinor) * 100);
  return Math.max(0, Math.min(100, percent));
}

function groupBuyStatusText(status: GroupBuyPlanStatus) {
  const labels: Record<GroupBuyPlanStatus, string> = {
    cancelled: '已取消',
    draft: '草稿',
    filled: '已满单',
    open: '认购中',
    settled: '已结算',
  };
  return labels[status];
}

function groupBuyStatusColor(status: GroupBuyPlanStatus) {
  const colors = {
    cancelled: 'grey',
    draft: 'blue',
    filled: 'green',
    open: 'cyan',
    settled: 'teal',
  } as const satisfies Record<GroupBuyPlanStatus, string>;

  return colors[status];
}

function formatGroupBuyPlayRuleLabel(code: string, playRules: PlayRuleSummary[]) {
  return playRules.find((rule) => rule.code === code)?.label ?? code;
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

function orderSourceText(source: OrderDetail['orderSource']) {
  return source === 'groupBuy' ? '合买下单' : '独立下单';
}

function orderSourceColor(source: OrderDetail['orderSource']) {
  return source === 'groupBuy' ? 'orange' : 'blue';
}

function formatOrderUser(order: OrderDetail) {
  return `${order.username ?? '未知用户'}（${order.userId}）`;
}

function scheduleText(lottery: LotteryKind) {
  const { schedule } = lottery;
  if ('periodic' in schedule) {
    return `${schedule.periodic.intervalSeconds} 秒一期`;
  }
  if ('timeNode' in schedule) {
    return `时间节点 ${schedule.timeNode.startTime} 起，每 ${schedule.timeNode.intervalSeconds} 秒一期`;
  }
  if ('daily' in schedule) {
    return `每日 ${schedule.daily.time}`;
  }
  return `${schedule.weekly.weekdays.join('、')} ${schedule.weekly.time}`;
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
