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
import { Ban, Plus, RefreshCcw, Trash2 } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { OrderBetInfo } from '../components/OrderBetInfo';
import { PageControls } from '../components/PageControls';
import { useDraws } from '../hooks/useDraws';
import { useLotteries } from '../hooks/useLotteries';
import { useOrders } from '../hooks/useOrders';
import { usePlayRules } from '../hooks/usePlayRules';
import type { LotteryKind } from '../types/dashboard';
import type { CreateOrderRequest, OrderDetail, OrderStatus } from '../types/orders';
import type {
  BigSmallOddEvenPick,
  DigitAttribute,
  PlayRuleCode,
  PlaySelection,
} from '../types/playRules';
import { formatMoney } from '../utils/format';
import { yuanInputToMinor } from '../utils/moneyInput';
import {
  formatOdds,
  isBankerRule,
  isBigSmallOddEvenRule,
  isDirectRule,
  isGroupSixRule,
  playCategoryForRule,
} from '../utils/playRules';

interface OrderManagementPageProps {
  onDashboardRefresh: () => void;
}

interface OrderFormState {
  bankerNumbers: string;
  dragNumbers: string;
  issue: string;
  lotteryId: string;
  numbers: string;
  onesAttributes: DigitAttribute[];
  positionA: string;
  positionB: string;
  positionC: string;
  ruleCode: PlayRuleCode | '';
  tensAttributes: DigitAttribute[];
  unitAmountYuan: string;
  userId: string;
}

const digitAttributeOptions: Array<{ label: string; value: DigitAttribute }> = [
  { label: '大', value: 'big' },
  { label: '小', value: 'small' },
  { label: '单', value: 'odd' },
  { label: '双', value: 'even' },
];

export function OrderManagementPage({ onDashboardRefresh }: OrderManagementPageProps) {
  const [includeRobotData, setIncludeRobotData] = useState(false);
  const [orderPageNumber, setOrderPageNumber] = useState(1);
  const [orderPageSize, setOrderPageSize] = useState(20);
  const {
    cancel,
    clearRecords,
    create,
    error: orderError,
    loading: ordersLoading,
    orderPage,
    orders,
    refresh: refreshOrders,
    saving,
  } = useOrders({
    includeRobotData,
    page: orderPageNumber,
    pageSize: orderPageSize,
  });
  const {
    error: lotteryError,
    loading: lotteriesLoading,
    lotteries,
    refresh: refreshLotteries,
  } = useLotteries();
  const {
    error: drawsError,
    loading: drawsLoading,
    issues: drawIssues,
    refresh: refreshDraws,
  } = useDraws();
  const { error: rulesError, loading: rulesLoading, rules } = usePlayRules();
  const [createdOrder, setCreatedOrder] = useState<OrderDetail | null>(null);
  const [createSheetVisible, setCreateSheetVisible] = useState(false);
  const [form, setForm] = useState<OrderFormState>(() => emptyForm());

  const selectedLottery = useMemo(
    () => lotteries.find((lottery) => lottery.id === form.lotteryId) ?? lotteries[0] ?? null,
    [form.lotteryId, lotteries],
  );
  const availableRules = useMemo(() => {
    if (!selectedLottery) {
      return [];
    }
    return rules.filter(
      (rule) =>
        rule.numberType === selectedLottery.numberType &&
        selectedLottery.playCategories.includes(playCategoryForRule(rule.code)),
    );
  }, [rules, selectedLottery]);
  const selectedRule = useMemo(
    () => availableRules.find((rule) => rule.code === form.ruleCode) ?? availableRules[0] ?? null,
    [availableRules, form.ruleCode],
  );
  const availableDrawIssues = useMemo(() => {
    if (!selectedLottery) {
      return [];
    }
    return drawIssues.filter(
      (issue) => issue.lotteryId === selectedLottery.id && issue.status === 'open',
    );
  }, [drawIssues, selectedLottery]);
  const selectedDrawIssue = useMemo(
    () => availableDrawIssues.find((issue) => issue.issue === form.issue) ?? null,
    [availableDrawIssues, form.issue],
  );

  useEffect(() => {
    if (!form.lotteryId && lotteries[0]) {
      setForm((current) => ({ ...current, lotteryId: lotteries[0].id }));
    }
  }, [form.lotteryId, lotteries]);

  useEffect(() => {
    if (selectedRule && form.ruleCode !== selectedRule.code) {
      setForm((current) => ({
        ...defaultSelectionFields(selectedRule.code),
        issue: current.issue,
        lotteryId: current.lotteryId,
        ruleCode: selectedRule.code,
        unitAmountYuan: current.unitAmountYuan,
        userId: current.userId,
      }));
    }
  }, [form.ruleCode, selectedRule]);

  useEffect(() => {
    const firstIssue = availableDrawIssues[0]?.issue ?? '';
    if (
      !availableDrawIssues.some((issue) => issue.issue === form.issue) &&
      form.issue !== firstIssue
    ) {
      setForm((current) => ({
        ...current,
        issue: firstIssue,
      }));
    }
  }, [availableDrawIssues, form.issue]);

  const refreshAll = () => {
    refreshDraws();
    refreshOrders();
    refreshLotteries();
  };

  const submit = async () => {
    if (!selectedLottery || !selectedRule || !selectedDrawIssue) {
      return;
    }
    const unitAmountMinor = yuanInputToMinor(form.unitAmountYuan);
    if (unitAmountMinor === null || unitAmountMinor <= 0) {
      Toast.warning('请输入正确的单注金额，金额必须大于 0 元且最多保留两位小数');
      return;
    }
    const payload: CreateOrderRequest = {
      issue: form.issue.trim(),
      lotteryId: selectedLottery.id,
      ruleCode: selectedRule.code,
      selection: selectionFromForm(selectedRule.code, form),
      unitAmountMinor,
      userId: form.userId.trim(),
    };
    const order = await create(payload);
    setCreatedOrder(order);
    setCreateSheetVisible(false);
    Toast.success(`投注单 ${order.id} 创建成功`);
    onDashboardRefresh();
  };

  const cancelPendingOrder = async (id: string) => {
    await cancel(id);
    onDashboardRefresh();
  };

  const clearBetOrderRecords = async () => {
    if (!window.confirm('确定一键清除全部用户投注记录吗？存在待开奖订单时系统会拒绝清理。')) {
      return;
    }
    try {
      const result = await clearRecords();
      setOrderPageNumber(1);
      setCreatedOrder(null);
      onDashboardRefresh();
      Toast.success(`已清除 ${result.deletedCount} 笔投注记录`);
    } catch {
      Toast.error('投注记录清除失败，请查看接口错误提示');
    }
  };

  const loading = ordersLoading || lotteriesLoading || rulesLoading || drawsLoading;
  const error = orderError ?? lotteryError ?? drawsError ?? rulesError;

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">订单管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            创建测试投注单，后端会校验期号仍处于销售中，再计算注数、金额和投注展开。
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <label className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-2 text-sm text-slate-600">
            <Switch
              checked={includeRobotData}
              onChange={(checked) => {
                setIncludeRobotData(checked);
                setOrderPageNumber(1);
              }}
            />
            <span>显示机器人数据</span>
          </label>
          <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
            刷新
          </Button>
          <Button
            icon={<Plus size={16} />}
            theme="solid"
            onClick={() => setCreateSheetVisible(true)}
          >
            创建投注单
          </Button>
          <Button
            disabled={saving || loading || orderPage.totalCount === 0}
            icon={<Trash2 size={16} />}
            theme="solid"
            type="danger"
            onClick={() => void clearBetOrderRecords()}
          >
            清除投注记录
          </Button>
        </div>
      </section>

      {error ? <Banner type="danger" title="订单接口错误" description={error} /> : null}

      <section>
        <Card className="rounded-md border border-line">
          <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex items-center gap-2">
              <h2 className="text-base font-semibold text-ink">订单列表</h2>
              <Tag color="cyan">{orderPage.totalCount} 个订单</Tag>
            </div>
            <PageControls
              loading={loading}
              page={orderPage.page}
              pageSize={orderPageSize}
              totalCount={orderPage.totalCount}
              totalPages={orderPage.totalPages}
              onPageChange={setOrderPageNumber}
              onPageSizeChange={(nextPageSize) => {
                setOrderPageNumber(1);
                setOrderPageSize(nextPageSize);
              }}
            />
          </div>
          {createdOrder ? (
            <div className="mb-3 rounded-md border border-emerald-100 bg-emerald-50 p-3 text-sm text-emerald-800">
              <div className="font-semibold text-emerald-950">最近创建：{createdOrder.id}</div>
              <div className="mt-1">
                {createdOrder.stakeCount} 注，金额 {formatMoney(createdOrder.amountMinor)}
              </div>
            </div>
          ) : null}
          {loading ? (
            <div className="grid min-h-[280px] place-items-center">
              <Spin tip="正在加载订单" />
            </div>
          ) : orders.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[1360px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">订单</th>
                    <th className="py-2 pr-4 font-medium">用户</th>
                    <th className="py-2 pr-4 font-medium">来源</th>
                    <th className="py-2 pr-4 font-medium">彩种</th>
                    <th className="py-2 pr-4 font-medium">玩法</th>
                    <th className="py-2 pr-4 font-medium">下注信息</th>
                    <th className="py-2 pr-4 font-medium">注数</th>
                    <th className="py-2 pr-4 font-medium">金额</th>
                    <th className="py-2 pr-4 font-medium">赔率</th>
                    <th className="py-2 pr-4 font-medium">开奖</th>
                    <th className="py-2 pr-4 font-medium">派奖</th>
                    <th className="py-2 pr-4 font-medium">状态</th>
                    <th className="py-2 pr-4 font-medium">操作</th>
                  </tr>
                </thead>
                <tbody>
                  {orders.map((order) => (
                    <tr key={order.id} className="border-b border-slate-100">
                      <td className="py-3 pr-4">
                        <div className="font-semibold text-ink">{order.id}</div>
                        <div className="mt-1 text-xs text-slate-400">{order.issue}</div>
                      </td>
                      <td className="py-3 pr-4 text-slate-600">{order.userId}</td>
                      <td className="py-3 pr-4">
                        <Tag color={order.orderSource === 'groupBuy' ? 'orange' : 'blue'}>
                          {orderSourceText(order.orderSource)}
                        </Tag>
                      </td>
                      <td className="py-3 pr-4">
                        <div className="font-medium text-ink">{order.lotteryName}</div>
                        <div className="mt-1 text-xs text-slate-400">{order.lotteryId}</div>
                      </td>
                      <td className="py-3 pr-4 text-slate-600">{ruleLabel(order.ruleCode, rules)}</td>
                      <td className="py-3 pr-4">
                        <OrderBetInfo compact expandedLimit={8} order={order} />
                      </td>
                      <td className="py-3 pr-4 text-slate-600">{order.stakeCount} 注</td>
                      <td className="py-3 pr-4 text-slate-600">{formatMoney(order.amountMinor)}</td>
                      <td className="py-3 pr-4 text-slate-600">
                        {formatOdds(order.oddsBasisPoints)}
                      </td>
                      <td className="py-3 pr-4">
                        {order.drawNumber ? (
                          <>
                            <div className="font-mono font-semibold text-ink">
                              {order.drawNumber}
                            </div>
                            <div className="mt-1 text-xs text-slate-400">
                              {order.settledAt ?? '已结算'}
                            </div>
                          </>
                        ) : (
                          <span className="text-slate-400">未开奖</span>
                        )}
                      </td>
                      <td className="py-3 pr-4">
                        {order.payoutMinor > 0 ? (
                          <>
                            <div className="font-semibold text-ink">
                              {formatMoney(order.payoutMinor)}
                            </div>
                            <div className="mt-1 flex max-w-[180px] flex-wrap gap-1">
                              {order.matchedBets.map((bet) => (
                                <Tag key={bet} color="green">
                                  {bet}
                                </Tag>
                              ))}
                            </div>
                          </>
                        ) : (
                          <span className="text-slate-400">
                            {order.status === 'lost' ? '未中奖' : '-'}
                          </span>
                        )}
                      </td>
                      <td className="py-3 pr-4">
                        <Tag color={statusColor(order.status)}>{statusText(order.status)}</Tag>
                      </td>
                      <td className="py-3 pr-4">
                        <Button
                          disabled={saving || order.status !== 'pendingDraw'}
                          icon={<Ban size={14} />}
                          size="small"
                          onClick={() => void cancelPendingOrder(order.id)}
                        >
                          取消
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="rounded-md border border-line p-4 text-sm text-slate-500">
              暂无订单，点击上方“创建投注单”创建一笔测试投注单。
            </div>
          )}
        </Card>

        <SideSheet
          aria-label="创建投注单"
          title="创建投注单"
          visible={createSheetVisible}
          width={560}
          onCancel={() => setCreateSheetVisible(false)}
        >
          <div className="mb-4">
            <h2 className="text-base font-semibold text-ink">创建投注单</h2>
            <p className="mt-1 text-sm text-slate-500">
              后端会先校验期号销售状态，再按玩法规则计算注数和订单金额。
            </p>
          </div>

          <form
            className="space-y-4"
            onSubmit={(event) => {
              event.preventDefault();
            }}
          >
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="用户 ID">
                <Input
                  className="form-input"
                  value={form.userId}
                  onChange={(value) => setFormValue(setForm, 'userId', value)}
                />
              </Field>
              <Field label="期号">
                <Select
                  className="form-input"
                  disabled={availableDrawIssues.length === 0}
                  value={form.issue}
                  onChange={(value) => setFormValue(setForm, 'issue', String(value ?? ''))}
                >
                  {availableDrawIssues.length > 0 ? (
                    availableDrawIssues.map((issue) => (
                      <Select.Option key={issue.id} value={issue.issue}>
                        {issue.issue}（封盘 {issue.saleClosedAt}）
                      </Select.Option>
                    ))
                  ) : (
                    <Select.Option value="">暂无可投注期号</Select.Option>
                  )}
                </Select>
              </Field>
            </div>

            <Field label="彩种">
              <Select
                className="form-input"
                value={selectedLottery?.id ?? ''}
                onChange={(value) => {
                  const lotteryId = String(value ?? '');
                  setForm((current) => ({
                    ...current,
                    issue: '',
                    lotteryId,
                    ruleCode: '',
                  }));
                }}
              >
                {lotteries.map((lottery) => (
                  <Select.Option key={lottery.id} value={lottery.id}>
                    {lottery.name}（{lottery.saleEnabled ? '销售中' : '停售'}）
                  </Select.Option>
                ))}
              </Select>
            </Field>

            <Field label="玩法">
              <Select
                className="form-input"
                value={selectedRule?.code ?? ''}
                onChange={(value) => {
                  const ruleCode = String(value ?? '') as PlayRuleCode;
                  setForm((current) => ({
                    ...current,
                    ...defaultSelectionFields(ruleCode),
                    ruleCode,
                  }));
                }}
              >
                {availableRules.map((rule) => (
                  <Select.Option key={rule.code} value={rule.code}>
                    {rule.label}
                  </Select.Option>
                ))}
              </Select>
            </Field>

            {selectedRule ? renderSelectionFields(selectedRule.code, form, setForm) : null}

            <Field label="单注金额（元）">
              <Input
                className="form-input"
                inputMode="decimal"
                placeholder="例如 2 或 2.00"
                value={form.unitAmountYuan}
                onChange={(value) => setFormValue(setForm, 'unitAmountYuan', value)}
              />
            </Field>

            <Button
              disabled={!selectedLottery || !selectedRule || !selectedDrawIssue || saving}
              icon={<Plus size={16} />}
              theme="solid"
              onClick={() => void submit()}
            >
              {saving ? '创建中' : '创建订单'}
            </Button>
            {!selectedDrawIssue ? (
              <div className="rounded-md bg-amber-50 p-3 text-sm text-amber-700">
                当前彩种没有 open 状态期号，请先到“开奖模式”或“开奖时间”页面创建期号。
              </div>
            ) : null}
          </form>

          {createdOrder ? (
            <div className="mt-4 rounded-md bg-slate-50 p-3 text-sm text-slate-600">
              <div className="font-semibold text-ink">最近创建：{createdOrder.id}</div>
              <div className="mt-1">
                {createdOrder.stakeCount} 注，金额 {formatMoney(createdOrder.amountMinor)}
              </div>
              <div className="mt-3 max-h-[180px] overflow-auto">
                <OrderBetInfo expandedLimit={24} order={createdOrder} />
              </div>
            </div>
          ) : null}
        </SideSheet>
      </section>
    </div>
  );
}

interface FieldProps {
  children: ReactNode;
  label: string;
}

function Field({ children, label }: FieldProps) {
  return (
    <label className="block space-y-1">
      <span className="text-xs font-medium text-slate-500">{label}</span>
      {children}
    </label>
  );
}

function renderSelectionFields(
  code: PlayRuleCode,
  form: OrderFormState,
  setForm: Dispatch<SetStateAction<OrderFormState>>,
) {
  if (isDirectRule(code)) {
    return (
      <div className="grid gap-3 sm:grid-cols-3">
        <Field label="第 1 位">
          <Input
            className="form-input"
            value={form.positionA}
            onChange={(value) => setFormValue(setForm, 'positionA', value)}
          />
        </Field>
        <Field label="第 2 位">
          <Input
            className="form-input"
            value={form.positionB}
            onChange={(value) => setFormValue(setForm, 'positionB', value)}
          />
        </Field>
        <Field label="第 3 位">
          <Input
            className="form-input"
            value={form.positionC}
            onChange={(value) => setFormValue(setForm, 'positionC', value)}
          />
        </Field>
      </div>
    );
  }

  if (isBigSmallOddEvenRule(code)) {
    return (
      <div className="space-y-3">
        <AttributePicker
          label="十位属性"
          selected={form.tensAttributes}
          onToggle={(value) => toggleAttribute(setForm, 'tensAttributes', value)}
        />
        <AttributePicker
          label="个位属性"
          selected={form.onesAttributes}
          onToggle={(value) => toggleAttribute(setForm, 'onesAttributes', value)}
        />
      </div>
    );
  }

  if (isBankerRule(code)) {
    return (
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="胆码">
          <Input
            className="form-input"
            value={form.bankerNumbers}
            onChange={(value) => setFormValue(setForm, 'bankerNumbers', value)}
          />
        </Field>
        <Field label="拖码">
          <Input
            className="form-input"
            value={form.dragNumbers}
            onChange={(value) => setFormValue(setForm, 'dragNumbers', value)}
          />
        </Field>
      </div>
    );
  }

  return (
    <Field label="选号">
      <Input
        className="form-input"
        value={form.numbers}
        onChange={(value) => setFormValue(setForm, 'numbers', value)}
      />
    </Field>
  );
}

interface AttributePickerProps {
  label: string;
  onToggle: (value: DigitAttribute) => void;
  selected: DigitAttribute[];
}

function AttributePicker({ label, onToggle, selected }: AttributePickerProps) {
  return (
    <Field label={label}>
      <div className="grid grid-cols-4 gap-2">
        {digitAttributeOptions.map((option) => (
          <label
            key={option.value}
            className="flex h-10 items-center justify-center gap-2 rounded-md border border-line text-sm"
          >
            <input
              checked={selected.includes(option.value)}
              type="checkbox"
              onChange={() => onToggle(option.value)}
            />
            {option.label}
          </label>
        ))}
      </div>
    </Field>
  );
}

function emptyForm(): OrderFormState {
  return {
    ...defaultSelectionFields('threeDirect'),
    issue: '2026155',
    lotteryId: '',
    ruleCode: '',
    unitAmountYuan: '2.00',
    userId: 'U10001',
  };
}

function defaultSelectionFields(code: PlayRuleCode): Pick<
  OrderFormState,
  | 'bankerNumbers'
  | 'dragNumbers'
  | 'numbers'
  | 'onesAttributes'
  | 'positionA'
  | 'positionB'
  | 'positionC'
  | 'tensAttributes'
> {
  if (isBigSmallOddEvenRule(code)) {
    return {
      bankerNumbers: '',
      dragNumbers: '',
      numbers: '',
      onesAttributes: ['even'],
      positionA: '',
      positionB: '',
      positionC: '',
      tensAttributes: ['small'],
    };
  }

  if (isDirectRule(code)) {
    const digits = defaultDirectDigits(code);
    return {
      bankerNumbers: '',
      dragNumbers: '',
      numbers: '',
      onesAttributes: [],
      positionA: digits[0],
      positionB: digits[1],
      positionC: digits[2],
      tensAttributes: [],
    };
  }

  if (isBankerRule(code)) {
    return {
      bankerNumbers: isGroupSixRule(code) ? '2,4' : '2',
      dragNumbers: isGroupSixRule(code) ? '1,7,9' : '4,7',
      numbers: '',
      onesAttributes: [],
      positionA: '',
      positionB: '',
      positionC: '',
      tensAttributes: [],
    };
  }

  return {
    bankerNumbers: '',
    dragNumbers: '',
    numbers: isGroupSixRule(code) ? '1,2,4,7' : '2,4,7',
    onesAttributes: [],
    positionA: '',
    positionB: '',
    positionC: '',
    tensAttributes: [],
  };
}

function selectionFromForm(code: PlayRuleCode, form: OrderFormState): PlaySelection {
  if (isDirectRule(code)) {
    return {
      positions: [
        parseDigitList(form.positionA),
        parseDigitList(form.positionB),
        parseDigitList(form.positionC),
      ],
    };
  }

  if (isBigSmallOddEvenRule(code)) {
    const picks: BigSmallOddEvenPick[] = [];
    if (form.tensAttributes.length > 0) {
      picks.push({ attributes: form.tensAttributes, position: 'tens' });
    }
    if (form.onesAttributes.length > 0) {
      picks.push({ attributes: form.onesAttributes, position: 'ones' });
    }
    return { bigSmallOddEven: picks };
  }

  if (isBankerRule(code)) {
    return {
      bankerNumbers: parseDigitList(form.bankerNumbers),
      dragNumbers: parseDigitList(form.dragNumbers),
    };
  }

  return {
    numbers: parseDigitList(form.numbers),
  };
}

function parseDigitList(value: string) {
  return value
    .split(/[,\s，]+/)
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => Number(item));
}

function defaultDirectDigits(code: PlayRuleCode) {
  if (code === 'fiveMiddleDirect') {
    return ['8', '9', '4'];
  }
  if (code === 'fiveBackDirect') {
    return ['9', '4', '2'];
  }
  if (code === 'fiveFrontDirect') {
    return ['7', '8', '9'];
  }
  return ['2', '4', '7'];
}

function setFormValue<K extends keyof OrderFormState>(
  setForm: Dispatch<SetStateAction<OrderFormState>>,
  key: K,
  value: OrderFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function toggleAttribute(
  setForm: Dispatch<SetStateAction<OrderFormState>>,
  key: 'onesAttributes' | 'tensAttributes',
  value: DigitAttribute,
) {
  setForm((current) => {
    const selected = current[key].includes(value)
      ? current[key].filter((item) => item !== value)
      : [...current[key], value];
    return { ...current, [key]: selected };
  });
}

function numberField(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function ruleLabel(code: PlayRuleCode, rules: Array<{ code: PlayRuleCode; label: string }>) {
  return rules.find((rule) => rule.code === code)?.label ?? code;
}

function statusText(status: OrderStatus) {
  const labels: Record<OrderStatus, string> = {
    cancelled: '已取消',
    lost: '未中奖',
    pendingDraw: '待开奖',
    won: '已中奖',
  };
  return labels[status];
}

function orderSourceText(source: OrderDetail['orderSource']) {
  return source === 'groupBuy' ? '合买下单' : '独立下单';
}

function statusColor(status: OrderStatus) {
  const colors: Record<OrderStatus, 'blue' | 'green' | 'grey' | 'red'> = {
    cancelled: 'grey',
    lost: 'red',
    pendingDraw: 'blue',
    won: 'green',
  };
  return colors[status];
}
