import { Input, Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { Plus, RefreshCcw, WalletCards } from 'lucide-react';
import {
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { MetricCard } from '../components/MetricCard';
import { useFinance } from '../hooks/useFinance';
import type {
  LedgerEntryKind,
  ManualBalanceAdjustmentRequest,
  RechargeChannel,
  RechargeOrderStatus,
} from '../types/finance';
import { formatMoney, formatSignedMoney } from '../utils/format';

interface FinanceManagementPageProps {
  onDashboardRefresh: () => void;
}

interface AdjustmentFormState {
  amountMinor: string;
  description: string;
  userId: string;
}

export function FinanceManagementPage({ onDashboardRefresh }: FinanceManagementPageProps) {
  const {
    accounts,
    adjustBalance,
    confirmRecharge,
    error,
    ledgerEntries,
    loading,
    rechargeOrders,
    refresh,
    saving,
  } = useFinance();
  const [form, setForm] = useState<AdjustmentFormState>({
    amountMinor: '1000',
    description: '后台手动补款',
    userId: 'U10001',
  });
  const totals = useMemo(() => financeTotals(accounts, ledgerEntries), [
    accounts,
    ledgerEntries,
  ]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const submit = async () => {
    const payload: ManualBalanceAdjustmentRequest = {
      amountMinor: numberField(form.amountMinor),
      description: form.description.trim(),
      userId: form.userId.trim(),
    };
    await adjustBalance(payload);
    onDashboardRefresh();
  };

  const confirmCustomerServiceRecharge = async (id: string) => {
    await confirmRecharge(id);
    onDashboardRefresh();
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">财务管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            查看用户资金账户和资金流水，执行后台手动调账。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="财务接口错误" description={error} /> : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="可用余额"
          trend={`${accounts.length} 个账户`}
          value={formatMoney(totals.availableBalanceMinor)}
        />
        <MetricCard
          label="冻结余额"
          trend="提现冻结后续接入"
          value={formatMoney(totals.frozenBalanceMinor)}
        />
        <MetricCard
          label="今日派奖"
          trend="来自派奖流水"
          value={formatMoney(totals.payoutMinor)}
        />
        <MetricCard
          label="充值订单"
          trend={`${paidRechargeCount(rechargeOrders)} 笔已入账`}
          value={`${rechargeOrders.length}`}
        />
      </section>

      <section className="grid gap-4 xl:grid-cols-[1fr_420px]">
        <Card className="rounded-md border border-line">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold text-ink">资金账户</h2>
            <Tag color="cyan">{accounts.length} 个账户</Tag>
          </div>

          {loading ? (
            <div className="grid min-h-[260px] place-items-center">
              <Spin tip="正在加载资金账户" />
            </div>
          ) : accounts.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[680px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">用户</th>
                    <th className="py-2 pr-4 font-medium">可用余额</th>
                    <th className="py-2 pr-4 font-medium">冻结余额</th>
                    <th className="py-2 pr-4 font-medium">账户总额</th>
                    <th className="py-2 pr-4 font-medium">状态</th>
                  </tr>
                </thead>
                <tbody>
                  {accounts.map((account) => (
                    <tr key={account.userId} className="border-b border-slate-100">
                      <td className="py-3 pr-4 font-semibold text-ink">{account.userId}</td>
                      <td className="py-3 pr-4 text-slate-600">
                        {formatMoney(account.availableBalanceMinor)}
                      </td>
                      <td className="py-3 pr-4 text-slate-600">
                        {formatMoney(account.frozenBalanceMinor)}
                      </td>
                      <td className="py-3 pr-4 text-slate-600">
                        {formatMoney(
                          account.availableBalanceMinor + account.frozenBalanceMinor,
                        )}
                      </td>
                      <td className="py-3 pr-4">
                        <Tag color={account.availableBalanceMinor > 0 ? 'green' : 'grey'}>
                          {account.availableBalanceMinor > 0 ? '可投注' : '无可用余额'}
                        </Tag>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="rounded-md border border-line p-4 text-sm text-slate-500">
              暂无资金账户。
            </div>
          )}
        </Card>

        <Card className="rounded-md border border-line">
          <div className="mb-4 flex items-start gap-3">
            <div className="grid h-10 w-10 place-items-center rounded-md bg-teal-50 text-teal-700">
              <WalletCards size={18} />
            </div>
            <div>
              <h2 className="text-base font-semibold text-ink">手动调账</h2>
              <p className="mt-1 text-sm text-slate-500">
                金额使用分，负数表示扣减可用余额。
              </p>
            </div>
          </div>

          <form
            className="space-y-4"
            onSubmit={(event) => {
              event.preventDefault();
            }}
          >
            <Field label="用户 ID">
              <Input
                className="form-input"
                value={form.userId}
                onChange={(value) => setFormValue(setForm, 'userId', value)}
              />
            </Field>

            <Field label="调账金额（分）">
              <Input
                className="form-input"
                type="number"
                value={form.amountMinor}
                onChange={(value) =>
                  setFormValue(setForm, 'amountMinor', value)
                }
              />
            </Field>

            <Field label="说明">
              <Input
                className="form-input"
                value={form.description}
                onChange={(value) =>
                  setFormValue(setForm, 'description', value)
                }
              />
            </Field>

            <Button
              disabled={saving}
              icon={<Plus size={16} />}
              theme="solid"
              onClick={() => void submit()}
            >
              {saving ? '提交中' : '提交调账'}
            </Button>
          </form>
        </Card>
      </section>

      <Card className="rounded-md border border-line">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-base font-semibold text-ink">充值订单</h2>
          <Tag color="green">{rechargeOrders.length} 笔</Tag>
        </div>

        {loading ? (
          <div className="grid min-h-[240px] place-items-center">
            <Spin tip="正在加载充值订单" />
          </div>
        ) : rechargeOrders.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full min-w-[1160px] text-left text-sm">
              <thead className="border-b border-line text-xs text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">订单</th>
                  <th className="py-2 pr-4 font-medium">用户</th>
                  <th className="py-2 pr-4 font-medium">渠道</th>
                  <th className="py-2 pr-4 font-medium">金额</th>
                  <th className="py-2 pr-4 font-medium">状态</th>
                  <th className="py-2 pr-4 font-medium">支付方式</th>
                  <th className="py-2 pr-4 font-medium">外部交易号</th>
                  <th className="py-2 pr-4 font-medium">客服会话</th>
                  <th className="py-2 pr-4 font-medium">操作</th>
                </tr>
              </thead>
              <tbody>
                {rechargeOrders.map((order) => (
                  <tr key={order.id} className="border-b border-slate-100">
                    <td className="py-3 pr-4">
                      <div className="font-semibold text-ink">{order.id}</div>
                      <div className="mt-1 text-xs text-slate-400">
                        {order.createdAt}
                        {order.paidAt ? ` · ${order.paidAt}` : ''}
                      </div>
                    </td>
                    <td className="py-3 pr-4">
                      <div className="font-medium text-slate-700">{order.username}</div>
                      <div className="mt-1 text-xs text-slate-400">{order.userId}</div>
                    </td>
                    <td className="py-3 pr-4">
                      <Tag color={order.channel === 'rainbowEpay' ? 'blue' : 'teal'}>
                        {rechargeChannelText(order.channel)}
                      </Tag>
                    </td>
                    <td className="py-3 pr-4 font-semibold text-emerald-700">
                      {formatMoney(order.amountMinor)}
                    </td>
                    <td className="py-3 pr-4">
                      <Tag color={rechargeStatusColor(order.status)}>
                        {rechargeStatusText(order.status)}
                      </Tag>
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      {order.payType ?? '-'}
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      {order.providerTradeNo ?? '-'}
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      {order.supportConversationId ?? '-'}
                    </td>
                    <td className="py-3 pr-4">
                      {order.channel === 'customerService' &&
                      order.status === 'waitingCustomerService' ? (
                        <Button
                          disabled={saving}
                          size="small"
                          theme="solid"
                          onClick={() =>
                            void confirmCustomerServiceRecharge(order.id)
                          }
                        >
                          确认入账
                        </Button>
                      ) : (
                        <span className="text-slate-400">-</span>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="rounded-md border border-line p-4 text-sm text-slate-500">
            暂无充值订单。用户发起彩虹易支付或客服直充后会显示在这里。
          </div>
        )}
      </Card>

      <Card className="rounded-md border border-line">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-base font-semibold text-ink">资金流水</h2>
          <Tag color="blue">{ledgerEntries.length} 笔</Tag>
        </div>

        {loading ? (
          <div className="grid min-h-[300px] place-items-center">
            <Spin tip="正在加载资金流水" />
          </div>
        ) : ledgerEntries.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full min-w-[920px] text-left text-sm">
              <thead className="border-b border-line text-xs text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">流水</th>
                  <th className="py-2 pr-4 font-medium">用户</th>
                  <th className="py-2 pr-4 font-medium">类型</th>
                  <th className="py-2 pr-4 font-medium">金额</th>
                  <th className="py-2 pr-4 font-medium">变更后余额</th>
                  <th className="py-2 pr-4 font-medium">关联单据</th>
                  <th className="py-2 pr-4 font-medium">说明</th>
                </tr>
              </thead>
              <tbody>
                {ledgerEntries.map((entry) => (
                  <tr key={entry.id} className="border-b border-slate-100">
                    <td className="py-3 pr-4">
                      <div className="font-semibold text-ink">{entry.id}</div>
                      <div className="mt-1 text-xs text-slate-400">{entry.createdAt}</div>
                    </td>
                    <td className="py-3 pr-4 text-slate-600">{entry.userId}</td>
                    <td className="py-3 pr-4">
                      <Tag color={ledgerKindColor(entry.kind)}>
                        {ledgerKindText(entry.kind)}
                      </Tag>
                    </td>
                    <td
                      className={`py-3 pr-4 font-semibold ${
                        entry.amountMinor >= 0 ? 'text-emerald-700' : 'text-rose-700'
                      }`}
                    >
                      {formatSignedMoney(entry.amountMinor)}
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      {formatMoney(entry.balanceAfterMinor)}
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      {entry.referenceId ?? '-'}
                    </td>
                    <td className="py-3 pr-4 text-slate-600">{entry.description}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="rounded-md border border-line p-4 text-sm text-slate-500">
            暂无资金流水。创建订单、取消订单、执行派奖或手动调账后会生成记录。
          </div>
        )}
      </Card>
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

function financeTotals(
  accounts: Array<{ availableBalanceMinor: number; frozenBalanceMinor: number }>,
  ledgerEntries: Array<{ amountMinor: number; kind: LedgerEntryKind }>,
) {
  return {
    availableBalanceMinor: accounts.reduce(
      (total, account) => total + account.availableBalanceMinor,
      0,
    ),
    frozenBalanceMinor: accounts.reduce(
      (total, account) => total + account.frozenBalanceMinor,
      0,
    ),
    payoutMinor: ledgerEntries
      .filter((entry) => entry.kind === 'payoutCredit')
      .reduce((total, entry) => total + entry.amountMinor, 0),
  };
}

function setFormValue<K extends keyof AdjustmentFormState>(
  setForm: Dispatch<SetStateAction<AdjustmentFormState>>,
  key: K,
  value: AdjustmentFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function numberField(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function ledgerKindText(kind: LedgerEntryKind) {
  const labels: Record<LedgerEntryKind, string> = {
    manualAdjustment: '手动调账',
    orderDebit: '投注扣款',
    orderRefund: '取消退款',
    payoutCredit: '派奖入账',
    rechargeCredit: '充值入账',
  };
  return labels[kind];
}

function ledgerKindColor(kind: LedgerEntryKind) {
  const colors: Record<LedgerEntryKind, 'blue' | 'green' | 'orange' | 'red'> = {
    manualAdjustment: 'orange',
    orderDebit: 'red',
    orderRefund: 'blue',
    payoutCredit: 'green',
    rechargeCredit: 'green',
  };
  return colors[kind];
}

function paidRechargeCount(
  rechargeOrders: Array<{ status: RechargeOrderStatus }>,
) {
  return rechargeOrders.filter((order) => order.status === 'paid').length;
}

function rechargeChannelText(channel: RechargeChannel) {
  const labels: Record<RechargeChannel, string> = {
    customerService: '客服直充',
    rainbowEpay: '彩虹易支付',
  };
  return labels[channel];
}

function rechargeStatusText(status: RechargeOrderStatus) {
  const labels: Record<RechargeOrderStatus, string> = {
    cancelled: '已取消',
    paid: '已入账',
    pending: '待支付',
    waitingCustomerService: '等待客服',
  };
  return labels[status];
}

function rechargeStatusColor(
  status: RechargeOrderStatus,
): 'blue' | 'green' | 'orange' | 'red' {
  const colors: Record<RechargeOrderStatus, 'blue' | 'green' | 'orange' | 'red'> = {
    cancelled: 'red',
    paid: 'green',
    pending: 'blue',
    waitingCustomerService: 'orange',
  };
  return colors[status];
}
