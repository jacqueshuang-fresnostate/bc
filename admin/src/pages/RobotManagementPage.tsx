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
import { Bot, PlayCircle, Plus, RefreshCcw, Save, Trash2 } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { MetricCard } from '../components/MetricCard';
import { useRobots } from '../hooks/useRobots';
import type { LotteryKind } from '../types/dashboard';
import type {
  GroupBuyRobotFillStrategy,
  GroupBuyRobotRun,
  RobotConfigSummary,
  RobotConfigPayload,
  RobotKind,
  RobotStatus,
} from '../types/robots';
import { formatMoney } from '../utils/format';

interface RobotManagementPageProps {
  activeModuleKey: string;
  onDashboardRefresh: () => void;
}

interface RobotFormState {
  description: string;
  groupBuyFillBeforeDrawSeconds: string;
  groupBuyFillStrategy: GroupBuyRobotFillStrategy;
  id: string;
  kind: RobotKind;
  lotteryIds: string[];
  name: string;
  status: RobotStatus;
}

export function RobotManagementPage({
  activeModuleKey,
  onDashboardRefresh,
}: RobotManagementPageProps) {
  const {
    changeStatus,
    error,
    executeGroupBuyRobots,
    lastGroupBuyRun,
    loading,
    lotteries,
    remove,
    refresh,
    robots,
    running,
    save,
    saving,
  } = useRobots();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [filterKind, setFilterKind] = useState<RobotKind>(
    kindForModule(activeModuleKey),
  );
  const [robotSheetVisible, setRobotSheetVisible] = useState(false);
  const [form, setForm] = useState<RobotFormState>(() =>
    emptyRobotForm(kindForModule(activeModuleKey)),
  );
  const filteredRobots = useMemo(
    () => robots.filter((robot) => robot.kind === filterKind),
    [filterKind, robots],
  );
  const editingRobot = useMemo(
    () => robots.find((robot) => robot.id === editingId) ?? null,
    [editingId, robots],
  );
  const totals = useMemo(() => robotTotals(robots), [robots]);

  useEffect(() => {
    const nextKind = kindForModule(activeModuleKey);
    setFilterKind(nextKind);
    setForm((current) => ({ ...current, kind: nextKind }));
    setRobotSheetVisible(false);
  }, [activeModuleKey]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const openNewRobot = (kind: RobotKind = filterKind) => {
    setEditingId(null);
    setForm(emptyRobotForm(kind));
    setRobotSheetVisible(true);
  };

  const openEditRobot = (robot: RobotConfigSummary) => {
    setEditingId(robot.id);
    setForm(robotFormFromSummary(robot));
    setRobotSheetVisible(true);
  };

  const submit = async () => {
    const payload = robotPayload(form);
    if (!payload) {
      Toast.warning('开奖前补满秒数需要大于 0 且不超过 86400');
      return;
    }
    const saved = await save(payload, editingId ?? undefined);
    setEditingId(saved.id);
    setFilterKind(saved.kind);
    setForm(robotFormFromSummary(saved));
    setRobotSheetVisible(false);
    onDashboardRefresh();
  };

  const deleteRobotConfig = async (robot: RobotConfigSummary) => {
    if (!robot.deletable) {
      Toast.warning('内置机器人配置不能删除，请改为暂停或禁用');
      return;
    }
    if (
      !window.confirm(
        `确定删除机器人配置【${robot.name}】吗？删除后不会影响已生成的订单、流水或合买计划。`,
      )
    ) {
      return;
    }

    await remove(robot.id);
    if (editingId === robot.id) {
      setEditingId(null);
      setRobotSheetVisible(false);
      setForm(emptyRobotForm(filterKind));
    }
    Toast.success('机器人配置已删除');
    onDashboardRefresh();
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">机器人配置</h1>
          <p className="mt-1 text-sm text-slate-500">
            维护合买机器人和购彩机器人适用彩种、状态和说明；普通配置可删除，核心内置配置只能暂停或禁用。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            theme="solid"
            icon={<Plus size={16} />}
            onClick={() => openNewRobot()}
          >
            新增配置
          </Button>
          {filterKind === 'groupBuy' ? (
            <Button
              disabled={running}
              icon={<PlayCircle size={16} />}
              loading={running}
              onClick={() =>
                void executeGroupBuyRobots().then(onDashboardRefresh)
              }
            >
              立即执行
            </Button>
          ) : null}
          <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
            刷新
          </Button>
        </div>
      </section>

      {error ? <Banner type="danger" title="机器人接口错误" description={error} /> : null}
      {filterKind === 'groupBuy' && lastGroupBuyRun ? (
        <GroupBuyRobotRunSummary run={lastGroupBuyRun} lotteries={lotteries} />
      ) : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard label="机器人总数" trend="内存配置" value={`${robots.length}`} />
        <MetricCard
          label="合买机器人"
          trend="发起合买与满单辅助"
          value={`${totals.groupBuyCount}`}
        />
        <MetricCard
          label="购彩机器人"
          trend="模拟普通用户购彩"
          value={`${totals.purchaseCount}`}
        />
        <MetricCard
          label="已启用"
          trend={`${totals.pausedCount} 个暂停`}
          value={`${totals.enabledCount}`}
        />
      </section>

      <section className="flex flex-wrap gap-2">
        <Button
          theme={filterKind === 'groupBuy' ? 'solid' : 'light'}
          onClick={() => {
            setFilterKind('groupBuy');
            if (!robotSheetVisible) {
              setForm((current) => ({ ...current, kind: 'groupBuy' }));
            }
          }}
        >
          合买机器人
        </Button>
        <Button
          theme={filterKind === 'purchase' ? 'solid' : 'light'}
          onClick={() => {
            setFilterKind('purchase');
            if (!robotSheetVisible) {
              setForm((current) => ({ ...current, kind: 'purchase' }));
            }
          }}
        >
          购彩机器人
        </Button>
      </section>

      {loading ? (
        <Card className="rounded-md border border-line">
          <div className="grid min-h-[320px] place-items-center">
            <Spin tip="正在加载机器人配置" />
          </div>
        </Card>
      ) : (
        <section>
          <Card className="rounded-md border border-line">
            <div className="mb-3 flex items-center justify-between">
              <h2 className="text-base font-semibold text-ink">
                {robotKindText(filterKind)}
              </h2>
              <Tag color="cyan">{filteredRobots.length} 个配置</Tag>
            </div>
            {filteredRobots.length > 0 ? (
              <div className="overflow-x-auto">
                <table className="w-full min-w-[840px] text-left text-sm">
                  <thead className="border-b border-line text-xs text-slate-500">
                    <tr>
                      <th className="py-2 pr-4 font-medium">机器人</th>
                      <th className="py-2 pr-4 font-medium">彩种</th>
                      <th className="py-2 pr-4 font-medium">状态</th>
                      <th className="py-2 pr-4 font-medium">补满策略</th>
                      <th className="py-2 pr-4 font-medium">说明</th>
                      <th className="py-2 pr-4 font-medium">操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredRobots.map((robot) => (
                      <tr
                        key={robot.id}
                        className={`border-b border-slate-100 ${
                          editingId === robot.id ? 'bg-teal-50/60' : ''
                        }`}
                      >
                        <td className="py-3 pr-4">
                          <button
                            className="text-left font-semibold text-accent"
                            type="button"
                            onClick={() => openEditRobot(robot)}
                          >
                            {robot.name}
                          </button>
                          <div className="mt-1 text-xs text-slate-400">{robot.id}</div>
                        </td>
                        <td className="py-3 pr-4">
                          <div className="flex flex-wrap gap-2">
                            {robot.lotteryIds.map((lotteryId) => (
                              <Tag key={lotteryId} color="grey">
                                {lotteryName(lotteryId, lotteries)}
                              </Tag>
                            ))}
                          </div>
                        </td>
                        <td className="py-3 pr-4">
                          <Tag color={robotStatusColor(robot.status)}>
                            {robotStatusText(robot.status)}
                          </Tag>
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {robot.kind === 'groupBuy'
                            ? groupBuyFillStrategyText(robot)
                            : '-'}
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {robot.description}
                        </td>
                        <td className="py-3 pr-4">
                          <div className="flex flex-wrap gap-2">
                            <Button
                              size="small"
                              onClick={() => openEditRobot(robot)}
                            >
                              编辑
                            </Button>
                            <Button
                              disabled={robot.status === 'enabled'}
                              size="small"
                              onClick={() =>
                                void changeStatus(robot.id, 'enabled').then(
                                  onDashboardRefresh,
                                )
                              }
                            >
                              启用
                            </Button>
                            <Button
                              disabled={robot.status === 'paused'}
                              size="small"
                              onClick={() =>
                                void changeStatus(robot.id, 'paused').then(
                                  onDashboardRefresh,
                                )
                              }
                            >
                              暂停
                            </Button>
                            <Button
                              disabled={saving || !robot.deletable}
                              icon={<Trash2 size={14} />}
                              size="small"
                              type="danger"
                              onClick={() => void deleteRobotConfig(robot)}
                              title={
                                robot.deletable
                                  ? '删除机器人配置'
                                  : '内置机器人不能删除'
                              }
                            >
                              {robot.deletable ? '删除' : '内置不可删'}
                            </Button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            ) : (
              <div className="rounded-md border border-line p-4 text-sm text-slate-500">
                当前类型暂无机器人配置。
              </div>
            )}
          </Card>
        </section>
      )}

      <SideSheet
        aria-label={editingId ? '编辑机器人配置' : '新增机器人配置'}
        title={editingId ? '编辑机器人配置' : '新增机器人配置'}
        visible={robotSheetVisible}
        width={560}
        onCancel={() => setRobotSheetVisible(false)}
      >
        <div className="mb-4 flex items-start gap-3 rounded border border-slate-200 bg-slate-50 p-3">
          <div className="grid h-10 w-10 shrink-0 place-items-center rounded-md bg-teal-50 text-teal-700">
            <Bot size={18} />
          </div>
          <div>
            <p className="text-sm font-medium text-ink">
              {editingId ? '维护已有机器人配置' : '新增机器人配置'}
            </p>
            <p className="mt-1 text-xs text-slate-500">
              保存后会同步机器人列表和工作台概览；核心内置配置不允许删除。
            </p>
          </div>
        </div>
        <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
          <Field label="机器人 ID">
            <Input
              className="form-input"
              value={form.id}
              onChange={(value) => setFormValue(setForm, 'id', value)}
            />
          </Field>
          <Field label="名称">
            <Input
              className="form-input"
              value={form.name}
              onChange={(value) =>
                setFormValue(setForm, 'name', value)
              }
            />
          </Field>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-1">
            <Field label="类型">
              <Select
                className="form-input"
                value={form.kind}
                onChange={(value) =>
                  setFormValue(setForm, 'kind', value as RobotKind)
                }
              >
                <Select.Option value="groupBuy">合买机器人</Select.Option>
                <Select.Option value="purchase">购彩机器人</Select.Option>
              </Select>
            </Field>
            <Field label="状态">
              <Select
                className="form-input"
                value={form.status}
                onChange={(value) =>
                  setFormValue(setForm, 'status', value as RobotStatus)
                }
              >
                <Select.Option value="enabled">启用</Select.Option>
                <Select.Option value="paused">暂停</Select.Option>
                <Select.Option value="disabled">禁用</Select.Option>
              </Select>
            </Field>
          </div>
          {form.kind === 'groupBuy' ? (
            <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-1">
              <Field label="补满策略">
                <Select
                  className="form-input"
                  value={form.groupBuyFillStrategy}
                  onChange={(value) =>
                    setFormValue(
                      setForm,
                      'groupBuyFillStrategy',
                      value as GroupBuyRobotFillStrategy,
                    )
                  }
                >
                  <Select.Option value="rhythm">阶段性补单</Select.Option>
                  <Select.Option value="beforeDraw">开奖前补满</Select.Option>
                </Select>
              </Field>
              <Field label="开奖前补满秒数">
                <Input
                  className="form-input"
                  disabled={form.groupBuyFillStrategy !== 'beforeDraw'}
                  min={1}
                  max={86400}
                  type="number"
                  value={form.groupBuyFillBeforeDrawSeconds}
                  onChange={(value) =>
                    setFormValue(
                      setForm,
                      'groupBuyFillBeforeDrawSeconds',
                      value,
                    )
                  }
                />
              </Field>
            </div>
          ) : null}
          <div className="space-y-2">
            <div className="text-sm font-medium text-slate-600">适用彩种</div>
            <div className="grid grid-cols-2 gap-2">
              {lotteries.map((lottery) => (
                <label
                  key={lottery.id}
                  className="flex items-center gap-2 rounded border border-slate-200 bg-white px-2 py-2 text-sm text-slate-600"
                >
                  <input
                    checked={form.lotteryIds.includes(lottery.id)}
                    type="checkbox"
                    onChange={(event) =>
                      toggleLottery(setForm, lottery.id, event.target.checked)
                    }
                  />
                  {lottery.name}
                </label>
              ))}
            </div>
          </div>
          <Field label="说明">
            <Input
              className="form-input"
              value={form.description}
              onChange={(value) =>
                setFormValue(setForm, 'description', value)
              }
            />
          </Field>
          <div className="flex flex-wrap gap-2">
            <Button
              disabled={saving || form.lotteryIds.length === 0}
              icon={<Save size={16} />}
              theme="solid"
              onClick={() => void submit()}
            >
              {editingId ? '保存配置' : '新增配置'}
            </Button>
            <Button
              onClick={() => openNewRobot(form.kind)}
            >
              新建
            </Button>
            {editingRobot ? (
              <Button
                disabled={saving || !editingRobot.deletable}
                icon={<Trash2 size={16} />}
                type="danger"
                onClick={() => void deleteRobotConfig(editingRobot)}
                title={
                  editingRobot.deletable
                    ? '删除机器人配置'
                    : '内置机器人不能删除'
                }
              >
                {editingRobot.deletable ? '删除配置' : '内置不可删'}
              </Button>
            ) : null}
            <Button onClick={() => setRobotSheetVisible(false)}>取消</Button>
          </div>
        </form>
      </SideSheet>
    </div>
  );
}

function GroupBuyRobotRunSummary({
  lotteries,
  run,
}: {
  lotteries: LotteryKind[];
  run: GroupBuyRobotRun;
}) {
  const totalDebitMinor = run.ledgerEntries
    .filter((entry) => entry.amountMinor < 0)
    .reduce((total, entry) => total + Math.abs(entry.amountMinor), 0);

  return (
    <section className="rounded-md border border-teal-100 bg-teal-50/70 p-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h2 className="text-sm font-semibold text-teal-900">
            合买机器人执行结果
          </h2>
          <p className="mt-1 text-xs text-teal-700">执行时间：{run.now}</p>
        </div>
        <div className="flex flex-wrap gap-2 text-xs">
          <Tag color="cyan">新增 {run.createdPlans.length}</Tag>
          <Tag color="green">满单 {run.filledPlans.length}</Tag>
          <Tag color="blue">订单 {run.createdOrders.length}</Tag>
          <Tag color="orange">扣款 {formatMoney(totalDebitMinor)}</Tag>
          <Tag color="grey">跳过 {run.skippedItems.length}</Tag>
        </div>
      </div>
      {run.createdPlans.length > 0 ? (
        <div className="mt-3 grid gap-2 lg:grid-cols-2">
          {run.createdPlans.slice(0, 4).map((plan) => (
            <div
              key={plan.id}
              className="rounded border border-teal-100 bg-white/80 px-3 py-2 text-xs text-slate-600"
            >
              <div className="font-medium text-ink">
                {lotteryName(plan.lotteryId, lotteries)} {plan.issue}
              </div>
              <div className="mt-1">
                {plan.title}，金额 {formatMoney(plan.totalAmountMinor)}
              </div>
            </div>
          ))}
        </div>
      ) : null}
      {run.skippedItems.length > 0 ? (
        <div className="mt-3 grid gap-2 lg:grid-cols-2">
          {run.skippedItems.slice(0, 6).map((item, index) => (
            <div
              key={`${item.robotId}-${item.lotteryId}-${item.issue ?? 'none'}-${index}`}
              className="rounded border border-amber-100 bg-white/80 px-3 py-2 text-xs text-slate-600"
            >
              <div className="font-medium text-ink">
                {item.robotName}
                {item.lotteryId
                  ? ` · ${lotteryName(item.lotteryId, lotteries)}`
                  : ''}
                {item.issue ? ` · ${item.issue}` : ''}
              </div>
              <div className="mt-1">{item.reason}</div>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <label className="block text-sm font-medium text-slate-600">
      <span className="mb-1 block">{label}</span>
      {children}
    </label>
  );
}

function kindForModule(moduleKey: string): RobotKind {
  return moduleKey === 'group-buy-robot' ? 'groupBuy' : 'purchase';
}

function emptyRobotForm(kind: RobotKind): RobotFormState {
  return {
    description: kind === 'groupBuy' ? '开盘期间发起合买并辅助满单' : '模拟用户购彩',
    groupBuyFillBeforeDrawSeconds: '15',
    groupBuyFillStrategy: 'rhythm',
    id: kind === 'groupBuy' ? 'R-GROUP-NEW' : 'R-BUY-NEW',
    kind,
    lotteryIds: [],
    name: kind === 'groupBuy' ? '新合买机器人' : '新购彩机器人',
    status: 'paused',
  };
}

function robotFormFromSummary(robot: RobotConfigSummary): RobotFormState {
  return {
    description: robot.description,
    groupBuyFillBeforeDrawSeconds: String(
      robot.groupBuyFillBeforeDrawSeconds ?? 15,
    ),
    groupBuyFillStrategy: robot.groupBuyFillStrategy ?? 'rhythm',
    id: robot.id,
    kind: robot.kind,
    lotteryIds: robot.lotteryIds,
    name: robot.name,
    status: robot.status,
  };
}

function robotPayload(form: RobotFormState): RobotConfigPayload | null {
  const beforeDrawSeconds = Number.parseInt(
    form.groupBuyFillBeforeDrawSeconds,
    10,
  );
  if (
    form.kind === 'groupBuy' &&
    form.groupBuyFillStrategy === 'beforeDraw' &&
    (!Number.isFinite(beforeDrawSeconds) ||
      beforeDrawSeconds <= 0 ||
      beforeDrawSeconds > 86400)
  ) {
    return null;
  }

  return {
    description: form.description.trim(),
    groupBuyFillBeforeDrawSeconds:
      Number.isFinite(beforeDrawSeconds) && beforeDrawSeconds > 0
        ? beforeDrawSeconds
        : 15,
    groupBuyFillStrategy:
      form.kind === 'groupBuy' ? form.groupBuyFillStrategy : 'rhythm',
    id: form.id.trim(),
    kind: form.kind,
    lotteryIds: form.lotteryIds,
    name: form.name.trim(),
    status: form.status,
  };
}

function robotTotals(robots: RobotConfigSummary[]) {
  return {
    enabledCount: robots.filter((robot) => robot.status === 'enabled').length,
    groupBuyCount: robots.filter((robot) => robot.kind === 'groupBuy').length,
    pausedCount: robots.filter((robot) => robot.status === 'paused').length,
    purchaseCount: robots.filter((robot) => robot.kind === 'purchase').length,
  };
}

function setFormValue<K extends keyof RobotFormState>(
  setForm: Dispatch<SetStateAction<RobotFormState>>,
  key: K,
  value: RobotFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function toggleLottery(
  setForm: Dispatch<SetStateAction<RobotFormState>>,
  lotteryId: string,
  checked: boolean,
) {
  setForm((current) => ({
    ...current,
    lotteryIds: checked
      ? Array.from(new Set([...current.lotteryIds, lotteryId]))
      : current.lotteryIds.filter((id) => id !== lotteryId),
  }));
}

function lotteryName(id: string, lotteries: LotteryKind[]) {
  return lotteries.find((lottery) => lottery.id === id)?.name ?? id;
}

function robotKindText(kind: RobotKind) {
  return kind === 'groupBuy' ? '合买机器人' : '购彩机器人';
}

function groupBuyFillStrategyText(robot: RobotConfigSummary) {
  if (robot.groupBuyFillStrategy === 'beforeDraw') {
    return `开奖前 ${robot.groupBuyFillBeforeDrawSeconds} 秒补满`;
  }
  return '阶段性补单';
}

function robotStatusText(status: RobotStatus) {
  const labels: Record<RobotStatus, string> = {
    disabled: '禁用',
    enabled: '启用',
    paused: '暂停',
  };
  return labels[status];
}

function robotStatusColor(status: RobotStatus) {
  if (status === 'enabled') {
    return 'green';
  }
  if (status === 'paused') {
    return 'orange';
  }
  return 'grey';
}
