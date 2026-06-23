import {
  Input,
  Banner,
  Button,
  Card,
  Select,
  Spin,
  Switch,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import {
  Clock3,
  Hash,
  Radio,
  RefreshCcw,
  Save,
  Timer,
} from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { OrderBetInfo } from '../components/OrderBetInfo';
import { useControlGroupBuyPlans } from '../hooks/useControlGroupBuyPlans';
import { useLotteryConsole } from '../hooks/useLotteryConsole';
import { usePlayRules } from '../hooks/usePlayRules';
import type { LotteryKind } from '../types/dashboard';
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
    setAvoidWinningStatus,
    saveDrawControl,
    syncDrawSource,
  } = useLotteryConsole();
  const { rules: playRules } = usePlayRules();
  const [now, setNow] = useState(() => new Date());
  const [selectedControlItem, setSelectedControlItem] =
    useState<LotteryConsoleItem | null>(null);
  const [controlForm, setControlForm] = useState<LotteryDrawControlFormState>(
    () => emptyDrawControlForm(),
  );
  const [controlSaving, setControlSaving] = useState(false);
  const [controlError, setControlError] = useState<string | null>(null);
  const [syncingLotteryId, setSyncingLotteryId] = useState<string | null>(null);
  const [avoidSavingLotteryId, setAvoidSavingLotteryId] = useState<string | null>(
    null,
  );

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
  const controllableSaleItems = useMemo(
    () => items.filter((item) => lotteryConsoleItemIsControllableSale(item)),
    [items],
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
    const currentItem = selectedControlLotteryId
      ? controllableSaleItems.find(
          (item) => item.lottery.id === selectedControlLotteryId,
        ) ?? null
      : null;
    const nextItem = currentItem ?? controllableSaleItems[0] ?? null;

    if (!nextItem) {
      setSelectedControlItem(null);
      return;
    }

    if (nextItem.lottery.id !== selectedControlLotteryId) {
      setSelectedControlItem(nextItem);
      setControlForm(
        normalizeControlFormForIssueStatus(
          nextItem,
          drawControlFormFromControl(nextItem.drawControl, nextItem),
        ),
      );
      setControlError(null);
      return;
    }

    setSelectedControlItem(nextItem);
    setControlForm((current) => normalizeControlFormForIssueStatus(nextItem, current));
  }, [controllableSaleItems, selectedControlLotteryId]);

  const selectControlLottery = (item: LotteryConsoleItem) => {
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
        enabled: true,
        drawNumber: trimmedDrawNumber || null,
        targetIssue: normalizedForm.targetIssue.trim() || null,
        targetOrderId: null,
        targetScope: 'issue',
      });
      Toast.success('开奖号码控制已保存');
      refresh();
    } catch (requestError) {
      setControlError(errorMessage(requestError));
    } finally {
      setControlSaving(false);
    }
  };

  const toggleAvoidWinning = async (
    item: LotteryConsoleItem,
    avoidWinningEnabled: boolean,
  ) => {
    setAvoidSavingLotteryId(item.lottery.id);
    try {
      const updated = await setAvoidWinningStatus(
        item.lottery.id,
        avoidWinningEnabled,
      );
      setSelectedControlItem((current) =>
        current?.lottery.id === updated.id
          ? {
              ...current,
              lottery: updated,
            }
          : current,
      );
      Toast.success(avoidWinningEnabled ? '避奖已开启' : '避奖已关闭');
      onDashboardRefresh();
    } catch (requestError) {
      Toast.error(errorMessage(requestError));
    } finally {
      setAvoidSavingLotteryId((current) =>
        current === item.lottery.id ? null : current,
      );
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

      <LotteryConsoleControlLotteryStrip
        items={controllableSaleItems}
        selectedLotteryId={selectedControlLotteryId}
        onSelect={selectControlLottery}
      />

      {selectedControlItem ? (
        <LotteryConsoleControlPanel
          error={controlError}
          form={controlForm}
          groupBuyError={controlGroupBuyError}
          groupBuyIssue={controlGroupBuyIssue}
          groupBuyLoading={controlGroupBuyLoading}
          groupBuyPlans={controlGroupBuyPlans}
          item={selectedControlItem}
          now={now}
          playRules={playRules}
          refreshing={loading || controlGroupBuyLoading}
          savingAvoidWinning={
            avoidSavingLotteryId === selectedControlItem.lottery.id
          }
          saving={controlSaving}
          schedulerStatus={schedulerStatus}
          syncingSource={syncingLotteryId === selectedControlItem.lottery.id}
          onChange={setControlForm}
          onAvoidWinningChange={(checked) =>
            void toggleAvoidWinning(selectedControlItem, checked)
          }
          onRefreshOrders={refreshControlData}
          onSubmit={() => void submitDrawControl()}
          onSyncSource={(nextItem) => void syncLotterySource(nextItem)}
        />
      ) : (
        <Card className="rounded-md border border-line">
          <div className="py-8 text-center text-sm text-slate-500">
            {items.length > 0 ? '暂无销售中且允许控制的彩种。' : '暂无彩种配置。'}
          </div>
        </Card>
      )}
    </div>
  );
}

function LotteryConsoleControlLotteryStrip({
  items,
  onSelect,
  selectedLotteryId,
}: {
  items: LotteryConsoleItem[];
  onSelect: (item: LotteryConsoleItem) => void;
  selectedLotteryId: string | null;
}) {
  return (
    <Card bodyStyle={{ padding: 16 }} className="rounded-md border border-line">
      <div className="flex flex-wrap items-center gap-3">
        {items.length > 0 ? (
          items.map((item) => (
            <button
              key={item.lottery.id}
              className={`h-10 min-w-[128px] rounded-md border px-4 text-center text-sm font-medium shadow-sm transition focus:outline-none focus:ring-2 focus:ring-blue-100 ${
                selectedLotteryId === item.lottery.id
                  ? 'border-blue-500 bg-blue-50 text-blue-700'
                  : 'border-line bg-white text-ink hover:border-blue-300 hover:bg-blue-50 hover:text-blue-600'
              }`}
              type="button"
              onClick={() => onSelect(item)}
            >
              {item.lottery.name}
            </button>
          ))
        ) : (
          <div className="py-6 text-sm text-slate-500">
            暂无销售中且允许控制的彩种。
          </div>
        )}
      </div>
    </Card>
  );
}

function LotteryConsoleControlPanel({
  error,
  form,
  groupBuyError,
  groupBuyIssue,
  groupBuyLoading,
  groupBuyPlans,
  item,
  onChange,
  onAvoidWinningChange,
  onRefreshOrders,
  onSubmit,
  onSyncSource,
  playRules,
  refreshing,
  savingAvoidWinning,
  saving,
  schedulerStatus,
  syncingSource,
  now,
}: {
  error: string | null;
  form: LotteryDrawControlFormState;
  groupBuyError: string | null;
  groupBuyIssue: string;
  groupBuyLoading: boolean;
  groupBuyPlans: GroupBuyPlan[];
  item: LotteryConsoleItem;
  onChange: (form: LotteryDrawControlFormState) => void;
  onAvoidWinningChange: (checked: boolean) => void;
  onRefreshOrders: () => void;
  onSubmit: () => void;
  onSyncSource: (item: LotteryConsoleItem) => void;
  playRules: PlayRuleSummary[];
  refreshing: boolean;
  savingAvoidWinning: boolean;
  saving: boolean;
  schedulerStatus: DrawSchedulerStatus | null;
  syncingSource: boolean;
  now: Date;
}) {
  const lottery = item.lottery;
  const inputMeta = drawNumberInputMeta(lottery.numberType);
  const controlIssues = controlCandidateIssues(item);
  const visibleOrders = visibleConsoleOrders(item, form);
  const groupBuyRows = groupBuyInitiatorParticipantRows(groupBuyPlans);
  const targetIssueInactive = controlFormTargetsInactiveIssue(item, form);
  const saveDisabled =
    saving ||
    targetIssueInactive ||
    !form.drawNumber.trim() ||
    !form.targetIssue.trim();

  const currentIssue = item.currentIssue;
  const recentDrawnIssue = item.recentDrawnIssue;
  const drawNumber = recentDrawnIssue?.drawNumber || currentIssue?.drawNumber || '';

  return (
    <Card bodyStyle={{ padding: 20 }} className="rounded-md border border-line">
      <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
          {error ? (
            <Banner type="danger" title="保存控制失败" description={error} />
          ) : null}

          <section className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
            <CountdownBlock
              icon={<Clock3 size={15} />}
              label="封盘"
              value={countdownText(currentIssue?.saleClosedAt, now, '已到封盘', '暂无期号')}
            />
            <CountdownBlock
              icon={<Timer size={15} />}
              label="开奖"
              value={drawCountdownText(currentIssue, now, schedulerStatus)}
            />
            <CountdownBlock
              icon={<Hash size={15} />}
              label="期号"
              value={currentIssue?.issue ?? '暂无当前期'}
            />
            <CountdownBlock
              icon={<Radio size={15} />}
              label="最近开奖"
              value={drawNumber || '待开奖'}
            />
          </section>

          <section
            className={`rounded-md border px-3 py-3 ${
              lottery.avoidWinningEnabled
                ? 'border-orange-200 bg-orange-50'
                : 'border-line bg-white'
            }`}
          >
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div>
                <div className="flex items-center gap-2">
                  <h3 className="text-sm font-semibold text-ink">避奖策略</h3>
                  <Tag color={lottery.avoidWinningEnabled ? 'orange' : 'grey'}>
                    {lottery.avoidWinningEnabled ? '已开启避奖' : '正常开奖'}
                  </Tag>
                </div>
                <p className="mt-1 text-xs leading-5 text-slate-500">
                  开启后，开奖前会尝试生成不命中当前待开奖订单的号码；关闭后按平台、API、手动或控制号码正常开奖。
                </p>
              </div>
              <div className="flex items-center gap-2 text-sm text-slate-600">
                <Switch
                  checked={lottery.avoidWinningEnabled}
                  disabled={saving || savingAvoidWinning}
                  loading={savingAvoidWinning}
                  onChange={onAvoidWinningChange}
                />
                <span>{lottery.avoidWinningEnabled ? '开启' : '关闭'}</span>
              </div>
            </div>
          </section>

          <section className="grid gap-3 lg:grid-cols-[1.2fr_1fr]">
            <div className="rounded-md border border-line bg-white p-3">
              <div className="flex items-center justify-between gap-2">
                <div className="text-xs font-medium text-slate-500">控制期号</div>
                <Tag color="blue">指定期号</Tag>
              </div>
              <Select
                className="mt-2 w-full"
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
              <p className="mt-2 text-xs leading-5 text-slate-500">
                默认选择当前销售中的期号；已开奖、已取消或已经过去的期号不会显示。
              </p>
              {targetIssueInactive ? (
                <Banner
                  className="mt-3"
                  type="warning"
                  title="指定期号已结束"
                  description="当前期号已经结束，控制配置已在页面上自动切回销售中期号。"
                />
              ) : null}
            </div>

            <div className="rounded-md border border-line bg-white p-3">
              <div className="text-xs font-medium text-slate-500">
                开奖号码（{numberTypeText(lottery.numberType)}）
              </div>
              <Input
                className="form-input mt-2 font-mono text-lg font-semibold"
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
                  默认显示当前指定期号相关订单。
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
                                targetOrderId: '',
                                targetScope: 'issue',
                              })
                            }
                          >
                            控制本期
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
                <h3 className="text-sm font-semibold text-ink">合买发起记录</h3>
                <p className="mt-1 text-xs text-slate-500">
                  只显示当前期号合买发起人的自购记录，跟单用户不在控奖列表中展示。
                </p>
              </div>
              <div className="flex items-center gap-2">
                {groupBuyIssue ? (
                  <Tag color="grey">期号 {groupBuyIssue}</Tag>
                ) : null}
                <Tag color="orange">{groupBuyRows.length} 条发起记录</Tag>
              </div>
            </div>
            {groupBuyError ? (
              <Banner
                className="mt-3"
                type="warning"
                title="合买发起记录读取失败"
                description={groupBuyError}
              />
            ) : null}
            {groupBuyLoading ? (
              <div className="mt-3 rounded border border-dashed border-line p-4 text-sm text-slate-500">
                正在加载合买发起记录...
              </div>
            ) : groupBuyRows.length > 0 ? (
              <div className="mt-3 max-h-[300px] overflow-auto">
                <table className="w-full min-w-[1320px] text-left text-xs">
                  <thead className="border-b border-line text-slate-500">
                    <tr>
                      <th className="py-2 pr-3 font-medium">合买计划</th>
                      <th className="py-2 pr-3 font-medium">状态</th>
                      <th className="py-2 pr-3 font-medium">发起人</th>
                      <th className="py-2 pr-3 font-medium">认购时间</th>
                      <th className="py-2 pr-3 font-medium">期号</th>
                      <th className="py-2 pr-3 font-medium">玩法</th>
                      <th className="py-2 pr-3 font-medium">投注内容</th>
                      <th className="py-2 pr-3 font-medium">自购金额</th>
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
                当前期号暂无合买发起人自购记录。
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
            {lottery.drawMode === 'api' ? (
              <Button
                disabled={saving}
                icon={<RefreshCcw size={15} />}
                loading={syncingSource}
                onClick={() => onSyncSource(item)}
              >
                立即同步开奖源
              </Button>
            ) : null}
          </div>
      </form>
    </Card>
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

function lotteryConsoleItemIsControllableSale(item: LotteryConsoleItem) {
  return item.lottery.saleEnabled && item.lottery.drawControlEnabled;
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
  const defaultIssue = sellingIssueForControl(item);
  const controlTargetIssue = control?.targetIssue?.trim() || '';
  const controlStillTargetsCurrentIssue =
    Boolean(control?.enabled) &&
    control?.targetScope === 'issue' &&
    controlTargetIssue !== '' &&
    controlIssueIsCurrent(item, controlTargetIssue);
  const activeTargetIssue = controlStillTargetsCurrentIssue
    ? controlTargetIssue
    : defaultIssue;

  return {
    enabled: Boolean(activeTargetIssue),
    drawNumber: controlStillTargetsCurrentIssue ? control?.drawNumber ?? '' : '',
    targetIssue: activeTargetIssue,
    targetOrderId: '',
    targetScope: 'issue',
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
    targetScope: 'issue',
  });
}

function normalizeControlFormForIssueStatus(
  item: LotteryConsoleItem | null,
  form: LotteryDrawControlFormState,
): LotteryDrawControlFormState {
  const sellingIssue = sellingIssueForControl(item);
  const normalizedScopeForm = {
    ...form,
    targetOrderId: '',
    targetScope: 'issue' as DrawControlTargetScope,
  };
  if (
    !normalizedScopeForm.targetIssue.trim() &&
    sellingIssue
  ) {
    return {
      ...normalizedScopeForm,
      enabled: true,
      targetIssue: sellingIssue,
    };
  }
  if (
    normalizedScopeForm.enabled &&
    normalizedScopeForm.targetIssue.trim() &&
    !controlIssueIsCurrent(item, normalizedScopeForm.targetIssue)
  ) {
    return {
      ...normalizedScopeForm,
      enabled: Boolean(sellingIssue),
      drawNumber: '',
      targetIssue: sellingIssue,
    };
  }
  if (!controlFormTargetsInactiveIssue(item, normalizedScopeForm)) {
    return normalizedScopeForm;
  }

  return {
    ...normalizedScopeForm,
    enabled: Boolean(sellingIssue),
    drawNumber: '',
    targetIssue: sellingIssue,
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
  return status === 'drawn' || status === 'cancelled' || !controlIssueIsCurrent(item, targetIssue);
}

function issueStatusForControl(
  item: LotteryConsoleItem | null,
  targetIssue: string,
): DrawIssueStatus | null {
  return (
    item?.issues.find((issue) => issue.issue === targetIssue.trim())?.status ?? null
  );
}

function controlCandidateIssues(item: LotteryConsoleItem | null) {
  return [...(item?.issues ?? [])]
    .filter((issue) => issue.status === 'open')
    .sort((left, right) => issueTimeValue(left) - issueTimeValue(right));
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
  return latestKnownIssue?.issue ?? '';
}

function controlIssueIsCurrent(
  item: LotteryConsoleItem | null | undefined,
  targetIssue: string,
) {
  const normalizedIssue = targetIssue.trim();
  if (!normalizedIssue) {
    return false;
  }
  const currentSellingIssue = sellingIssueForControl(item);
  return currentSellingIssue !== '' && normalizedIssue === currentSellingIssue;
}

function visibleConsoleOrders(
  item: LotteryConsoleItem,
  form: LotteryDrawControlFormState,
) {
  if (form.targetIssue.trim()) {
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
  if (form.targetIssue.trim()) {
    return form.targetIssue.trim();
  }
  return sellingIssueForControl(item);
}

function groupBuyInitiatorParticipantRows(plans: GroupBuyPlan[]) {
  return plans.flatMap((plan) =>
    plan.participants
      .filter((participant) => isGroupBuyInitiatorParticipant(plan, participant))
      .map((participant) => ({
        participant,
        plan,
      })),
  );
}

function isGroupBuyInitiatorParticipant(
  plan: GroupBuyPlan,
  participant: GroupBuyPlan['participants'][number],
) {
  return participant.userId.trim() === plan.initiatorUserId.trim();
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

function statusText(status: DrawIssueStatus) {
  const labels: Record<DrawIssueStatus, string> = {
    cancelled: '已取消',
    closed: '已封盘',
    drawn: '已开奖',
    open: '销售中',
  };
  return labels[status];
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

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
