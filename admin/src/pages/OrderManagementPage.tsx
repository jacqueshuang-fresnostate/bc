import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { Ban, Plus, RefreshCcw } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { useLotteries } from '../hooks/useLotteries';
import { useOrders } from '../hooks/useOrders';
import { usePlayRules } from '../hooks/usePlayRules';
import type { LotteryKind, PlayCategory } from '../types/dashboard';
import type { CreateOrderRequest, OrderDetail, OrderStatus } from '../types/orders';
import type {
  BigSmallOddEvenPick,
  DigitAttribute,
  PlayRuleCode,
  PlaySelection,
} from '../types/playRules';

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
  unitAmountMinor: string;
  userId: string;
}

const digitAttributeOptions: Array<{ label: string; value: DigitAttribute }> = [
  { label: '大', value: 'big' },
  { label: '小', value: 'small' },
  { label: '单', value: 'odd' },
  { label: '双', value: 'even' },
];

export function OrderManagementPage({ onDashboardRefresh }: OrderManagementPageProps) {
  const {
    cancel,
    create,
    error: orderError,
    loading: ordersLoading,
    orders,
    refresh: refreshOrders,
    saving,
  } = useOrders();
  const {
    error: lotteryError,
    loading: lotteriesLoading,
    lotteries,
    refresh: refreshLotteries,
  } = useLotteries();
  const { error: rulesError, loading: rulesLoading, rules } = usePlayRules();
  const [createdOrder, setCreatedOrder] = useState<OrderDetail | null>(null);
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
        unitAmountMinor: current.unitAmountMinor,
        userId: current.userId,
      }));
    }
  }, [form.ruleCode, selectedRule]);

  const refreshAll = () => {
    refreshOrders();
    refreshLotteries();
  };

  const submit = async () => {
    if (!selectedLottery || !selectedRule) {
      return;
    }
    const payload: CreateOrderRequest = {
      issue: form.issue.trim(),
      lotteryId: selectedLottery.id,
      ruleCode: selectedRule.code,
      selection: selectionFromForm(selectedRule.code, form),
      unitAmountMinor: numberField(form.unitAmountMinor),
      userId: form.userId.trim(),
    };
    const order = await create(payload);
    setCreatedOrder(order);
    onDashboardRefresh();
  };

  const cancelPendingOrder = async (id: string) => {
    await cancel(id);
    onDashboardRefresh();
  };

  const loading = ordersLoading || lotteriesLoading || rulesLoading;
  const error = orderError ?? lotteryError ?? rulesError;

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">订单管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            创建测试投注单，查看后端计算的注数、金额和投注展开。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="订单接口错误" description={error} /> : null}

      <section className="grid gap-4 xl:grid-cols-[1fr_420px]">
        <Card className="rounded-md border border-line">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold text-ink">订单列表</h2>
            <Tag color="cyan">{orders.length} 个订单</Tag>
          </div>
          {loading ? (
            <div className="grid min-h-[280px] place-items-center">
              <Spin tip="正在加载订单" />
            </div>
          ) : orders.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[920px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">订单</th>
                    <th className="py-2 pr-4 font-medium">用户</th>
                    <th className="py-2 pr-4 font-medium">彩种</th>
                    <th className="py-2 pr-4 font-medium">玩法</th>
                    <th className="py-2 pr-4 font-medium">注数</th>
                    <th className="py-2 pr-4 font-medium">金额</th>
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
                        <div className="font-medium text-ink">{order.lotteryName}</div>
                        <div className="mt-1 text-xs text-slate-400">{order.lotteryId}</div>
                      </td>
                      <td className="py-3 pr-4 text-slate-600">{ruleLabel(order.ruleCode, rules)}</td>
                      <td className="py-3 pr-4 text-slate-600">{order.stakeCount} 注</td>
                      <td className="py-3 pr-4 text-slate-600">{formatMoney(order.amountMinor)}</td>
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
              暂无订单，先在右侧创建一笔测试投注单。
            </div>
          )}
        </Card>

        <Card className="rounded-md border border-line">
          <div className="mb-4">
            <h2 className="text-base font-semibold text-ink">创建投注单</h2>
            <p className="mt-1 text-sm text-slate-500">
              后端会按玩法规则重新计算注数和订单金额。
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
                <input
                  className="form-input"
                  value={form.userId}
                  onChange={(event) => setFormValue(setForm, 'userId', event.target.value)}
                />
              </Field>
              <Field label="期号">
                <input
                  className="form-input"
                  value={form.issue}
                  onChange={(event) => setFormValue(setForm, 'issue', event.target.value)}
                />
              </Field>
            </div>

            <Field label="彩种">
              <select
                className="form-input"
                value={selectedLottery?.id ?? ''}
                onChange={(event) =>
                  setForm((current) => ({
                    ...current,
                    lotteryId: event.target.value,
                    ruleCode: '',
                  }))
                }
              >
                {lotteries.map((lottery) => (
                  <option key={lottery.id} value={lottery.id}>
                    {lottery.name}（{lottery.saleEnabled ? '销售中' : '停售'}）
                  </option>
                ))}
              </select>
            </Field>

            <Field label="玩法">
              <select
                className="form-input"
                value={selectedRule?.code ?? ''}
                onChange={(event) =>
                  setForm((current) => ({
                    ...current,
                    ...defaultSelectionFields(event.target.value as PlayRuleCode),
                    ruleCode: event.target.value as PlayRuleCode,
                  }))
                }
              >
                {availableRules.map((rule) => (
                  <option key={rule.code} value={rule.code}>
                    {rule.label}
                  </option>
                ))}
              </select>
            </Field>

            {selectedRule ? renderSelectionFields(selectedRule.code, form, setForm) : null}

            <Field label="单注金额（分）">
              <input
                className="form-input"
                min="1"
                type="number"
                value={form.unitAmountMinor}
                onChange={(event) => setFormValue(setForm, 'unitAmountMinor', event.target.value)}
              />
            </Field>

            <Button
              disabled={!selectedLottery || !selectedRule || saving}
              icon={<Plus size={16} />}
              theme="solid"
              onClick={() => void submit()}
            >
              {saving ? '创建中' : '创建订单'}
            </Button>
          </form>

          {createdOrder ? (
            <div className="mt-4 rounded-md bg-slate-50 p-3 text-sm text-slate-600">
              <div className="font-semibold text-ink">最近创建：{createdOrder.id}</div>
              <div className="mt-1">
                {createdOrder.stakeCount} 注，金额 {formatMoney(createdOrder.amountMinor)}
              </div>
              <div className="mt-2 flex max-h-[120px] flex-wrap gap-2 overflow-auto">
                {createdOrder.expandedBets.slice(0, 24).map((bet) => (
                  <Tag key={bet} color="blue">
                    {bet}
                  </Tag>
                ))}
              </div>
            </div>
          ) : null}
        </Card>
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
          <input
            className="form-input"
            value={form.positionA}
            onChange={(event) => setFormValue(setForm, 'positionA', event.target.value)}
          />
        </Field>
        <Field label="第 2 位">
          <input
            className="form-input"
            value={form.positionB}
            onChange={(event) => setFormValue(setForm, 'positionB', event.target.value)}
          />
        </Field>
        <Field label="第 3 位">
          <input
            className="form-input"
            value={form.positionC}
            onChange={(event) => setFormValue(setForm, 'positionC', event.target.value)}
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
          <input
            className="form-input"
            value={form.bankerNumbers}
            onChange={(event) => setFormValue(setForm, 'bankerNumbers', event.target.value)}
          />
        </Field>
        <Field label="拖码">
          <input
            className="form-input"
            value={form.dragNumbers}
            onChange={(event) => setFormValue(setForm, 'dragNumbers', event.target.value)}
          />
        </Field>
      </div>
    );
  }

  return (
    <Field label="选号">
      <input
        className="form-input"
        value={form.numbers}
        onChange={(event) => setFormValue(setForm, 'numbers', event.target.value)}
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
    unitAmountMinor: '200',
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

function playCategoryForRule(code: PlayRuleCode): PlayCategory {
  if (isDirectRule(code)) {
    return 'direct';
  }
  if (code.endsWith('DirectCombination')) {
    return 'directCombination';
  }
  if (code.includes('GroupThree')) {
    return 'groupThree';
  }
  if (code.includes('GroupSix')) {
    return 'groupSix';
  }
  return 'bigSmallOddEven';
}

function isDirectRule(code: PlayRuleCode) {
  return code.endsWith('Direct') && !code.endsWith('DirectCombination');
}

function isBigSmallOddEvenRule(code: PlayRuleCode) {
  return code === 'fiveBigSmallOddEven';
}

function isBankerRule(code: PlayRuleCode) {
  return code.endsWith('Banker');
}

function isGroupSixRule(code: PlayRuleCode) {
  return code.includes('GroupSix');
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

function statusColor(status: OrderStatus) {
  const colors: Record<OrderStatus, 'blue' | 'green' | 'grey' | 'red'> = {
    cancelled: 'grey',
    lost: 'red',
    pendingDraw: 'blue',
    won: 'green',
  };
  return colors[status];
}

function formatMoney(amountMinor: number) {
  return new Intl.NumberFormat('zh-CN', {
    style: 'currency',
    currency: 'CNY',
  }).format(amountMinor / 100);
}
