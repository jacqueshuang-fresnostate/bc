import { Input, Banner, Button, Card, Select, Spin, Tabs, Tag } from '@douyinfe/semi-ui';
import { CheckCircle2, Plus, RefreshCcw, WalletCards, XCircle } from 'lucide-react';
import {
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { MetricCard } from '../components/MetricCard';
import { useFinance } from '../hooks/useFinance';
import type {
  FinancePage,
  LedgerEntry,
  LedgerEntryKind,
  ManualBalanceAdjustmentRequest,
  RechargeChannel,
  RechargeOrderStatus,
  WithdrawalMethodType,
  WithdrawalOrderStatus,
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

const PAGE_SIZE_OPTIONS = [10, 20, 50, 100];

export function FinanceManagementPage({ onDashboardRefresh }: FinanceManagementPageProps) {
  const [activeFinanceTab, setActiveFinanceTab] = useState('accounts');
  const [accountPage, setAccountPage] = useState(1);
  const [accountPageSize, setAccountPageSize] = useState(10);
  const [rechargePage, setRechargePage] = useState(1);
  const [rechargePageSize, setRechargePageSize] = useState(10);
  const [withdrawalPage, setWithdrawalPage] = useState(1);
  const [withdrawalPageSize, setWithdrawalPageSize] = useState(10);
  const [ledgerPage, setLedgerPage] = useState(1);
  const [ledgerPageSize, setLedgerPageSize] = useState(20);
  const {
    accounts,
    adjustBalance,
    approveWithdrawal,
    confirmRecharge,
    error,
    ledgerEntries,
    loading,
    overview,
    rechargeOrders,
    refresh,
    rejectWithdrawal,
    saving,
    withdrawalOrders,
  } = useFinance({
    accountQuery: { page: accountPage, pageSize: accountPageSize },
    ledgerQuery: { page: ledgerPage, pageSize: ledgerPageSize },
    rechargeQuery: { page: rechargePage, pageSize: rechargePageSize },
    withdrawalQuery: { page: withdrawalPage, pageSize: withdrawalPageSize },
  });
  const [form, setForm] = useState<AdjustmentFormState>({
    amountMinor: '1000',
    description: '后台手动补款',
    userId: 'U10001',
  });

  const availableBalanceMinor = overview
    ? overview.totalBalanceMinor - overview.pendingWithdrawMinor
    : 0;

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

  const approvePendingWithdrawal = async (id: string) => {
    await approveWithdrawal(id);
    onDashboardRefresh();
  };

  const rejectPendingWithdrawal = async (id: string) => {
    await rejectWithdrawal(id);
    onDashboardRefresh();
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">财务管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            查看资金账户、充值订单、提现申请和资金流水，执行后台手动调账。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="财务接口错误" description={error} /> : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
        <MetricCard
          label="可用余额"
          trend={`${accounts.totalCount} 个账户`}
          value={formatMoney(availableBalanceMinor)}
        />
        <MetricCard
          label="冻结余额"
          trend="提现申请冻结"
          value={formatMoney(overview?.pendingWithdrawMinor ?? 0)}
        />
        <MetricCard
          label="今日充值"
          trend="充值入账流水"
          value={formatMoney(overview?.todayRechargeMinor ?? 0)}
        />
        <MetricCard
          label="今日派奖"
          trend="派奖入账流水"
          value={formatMoney(overview?.todayPayoutMinor ?? 0)}
        />
        <MetricCard
          label="提现申请"
          trend={`当前页 ${pendingWithdrawalCount(withdrawalOrders)} 笔待审核`}
          value={`${withdrawalOrders.totalCount}`}
        />
      </section>

      <Tabs
        activeKey={activeFinanceTab}
        collapsible
        onChange={(key) => setActiveFinanceTab(String(key))}
      >
        <Tabs.TabPane
          itemKey="accounts"
          tab={
            <span className="inline-flex items-center gap-2">
              <span>账户与调账</span>
              <Tag color="cyan">{accounts.totalCount}</Tag>
            </span>
          }
        >
          <section className="grid gap-4 pt-3 xl:grid-cols-[1fr_420px]">
            <Card className="rounded-md border border-line">
          <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex items-center gap-2">
              <h2 className="text-base font-semibold text-ink">资金账户</h2>
              <Tag color="cyan">{accounts.totalCount} 个账户</Tag>
            </div>
            <PageControls
              loading={loading}
              page={accounts.page}
              pageSize={accountPageSize}
              totalCount={accounts.totalCount}
              totalPages={accounts.totalPages}
              onPageChange={setAccountPage}
              onPageSizeChange={(nextPageSize) => {
                setAccountPage(1);
                setAccountPageSize(nextPageSize);
              }}
            />
          </div>

          {loading ? (
            <div className="grid min-h-[260px] place-items-center">
              <Spin tip="正在加载资金账户" />
            </div>
          ) : accounts.items.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[760px] text-left text-sm">
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
                  {accounts.items.map((account) => (
                    <tr key={account.userId} className="border-b border-slate-100">
                      <td className="py-3 pr-4">
                        <div className="font-semibold text-ink">
                          {account.username ?? '未知用户'}
                        </div>
                        <div className="mt-1 text-xs text-slate-400">{account.userId}</div>
                      </td>
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
                onChange={(value) => setFormValue(setForm, 'amountMinor', value)}
              />
            </Field>

            <Field label="说明">
              <Input
                className="form-input"
                value={form.description}
                onChange={(value) => setFormValue(setForm, 'description', value)}
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
        </Tabs.TabPane>

        <Tabs.TabPane
          itemKey="recharge"
          tab={
            <span className="inline-flex items-center gap-2">
              <span>充值订单</span>
              <Tag color="green">{rechargeOrders.totalCount}</Tag>
            </span>
          }
        >
          <Card className="mt-3 rounded-md border border-line">
        <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div className="flex items-center gap-2">
            <h2 className="text-base font-semibold text-ink">充值订单</h2>
            <Tag color="green">{rechargeOrders.totalCount} 笔</Tag>
          </div>
          <PageControls
            loading={loading}
            page={rechargeOrders.page}
            pageSize={rechargePageSize}
            totalCount={rechargeOrders.totalCount}
            totalPages={rechargeOrders.totalPages}
            onPageChange={setRechargePage}
            onPageSizeChange={(nextPageSize) => {
              setRechargePage(1);
              setRechargePageSize(nextPageSize);
            }}
          />
        </div>

        {loading ? (
          <div className="grid min-h-[240px] place-items-center">
            <Spin tip="正在加载充值订单" />
          </div>
        ) : rechargeOrders.items.length > 0 ? (
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
                {rechargeOrders.items.map((order) => (
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
                          onClick={() => void confirmCustomerServiceRecharge(order.id)}
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
        </Tabs.TabPane>

        <Tabs.TabPane
          itemKey="withdrawals"
          tab={
            <span className="inline-flex items-center gap-2">
              <span>提现管理</span>
              <Tag color="red">{withdrawalOrders.totalCount}</Tag>
            </span>
          }
        >
          <Card className="mt-3 rounded-md border border-line">
        <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div className="flex items-center gap-2">
            <h2 className="text-base font-semibold text-ink">提现管理</h2>
            <Tag color="red">{withdrawalOrders.totalCount} 笔</Tag>
          </div>
          <PageControls
            loading={loading}
            page={withdrawalOrders.page}
            pageSize={withdrawalPageSize}
            totalCount={withdrawalOrders.totalCount}
            totalPages={withdrawalOrders.totalPages}
            onPageChange={setWithdrawalPage}
            onPageSizeChange={(nextPageSize) => {
              setWithdrawalPage(1);
              setWithdrawalPageSize(nextPageSize);
            }}
          />
        </div>

        {loading ? (
          <div className="grid min-h-[240px] place-items-center">
            <Spin tip="正在加载提现申请" />
          </div>
        ) : withdrawalOrders.items.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full min-w-[1180px] text-left text-sm">
              <thead className="border-b border-line text-xs text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">申请</th>
                  <th className="py-2 pr-4 font-medium">用户</th>
                  <th className="py-2 pr-4 font-medium">金额</th>
                  <th className="py-2 pr-4 font-medium">状态</th>
                  <th className="py-2 pr-4 font-medium">提现方式</th>
                  <th className="py-2 pr-4 font-medium">收款账户</th>
                  <th className="py-2 pr-4 font-medium">审核时间</th>
                  <th className="py-2 pr-4 font-medium">操作</th>
                </tr>
              </thead>
              <tbody>
                {withdrawalOrders.items.map((order) => (
                  <tr key={order.id} className="border-b border-slate-100">
                    <td className="py-3 pr-4">
                      <div className="font-semibold text-ink">{order.id}</div>
                      <div className="mt-1 text-xs text-slate-400">{order.createdAt}</div>
                    </td>
                    <td className="py-3 pr-4">
                      <div className="font-medium text-slate-700">{order.username}</div>
                      <div className="mt-1 text-xs text-slate-400">{order.userId}</div>
                    </td>
                    <td className="py-3 pr-4 font-semibold text-rose-700">
                      {formatMoney(order.amountMinor)}
                    </td>
                    <td className="py-3 pr-4">
                      <Tag color={withdrawalStatusColor(order.status)}>
                        {withdrawalStatusText(order.status)}
                      </Tag>
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      {withdrawalMethodText(order.methodType)}
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      <div>{order.accountHolder}</div>
                      <div className="mt-1 text-xs text-slate-400">
                        {order.bankName ? `${order.bankName} · ` : ''}
                        {order.accountNumber}
                      </div>
                    </td>
                    <td className="py-3 pr-4 text-slate-600">
                      {order.reviewedAt ?? '-'}
                    </td>
                    <td className="py-3 pr-4">
                      {order.status === 'pending' ? (
                        <div className="flex items-center gap-2">
                          <Button
                            disabled={saving}
                            icon={<CheckCircle2 size={14} />}
                            size="small"
                            theme="solid"
                            onClick={() => void approvePendingWithdrawal(order.id)}
                          >
                            通过
                          </Button>
                          <Button
                            disabled={saving}
                            icon={<XCircle size={14} />}
                            size="small"
                            type="danger"
                            onClick={() => void rejectPendingWithdrawal(order.id)}
                          >
                            驳回
                          </Button>
                        </div>
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
            暂无提现申请。用户提交提现后会显示在这里。
          </div>
        )}
          </Card>
        </Tabs.TabPane>

        <Tabs.TabPane
          itemKey="ledger"
          tab={
            <span className="inline-flex items-center gap-2">
              <span>资金流水</span>
              <Tag color="blue">{ledgerEntries.totalCount}</Tag>
            </span>
          }
        >
          <Card className="mt-3 rounded-md border border-line">
        <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div className="flex items-center gap-2">
            <h2 className="text-base font-semibold text-ink">资金流水</h2>
            <Tag color="blue">{ledgerEntries.totalCount} 笔</Tag>
          </div>
          <PageControls
            loading={loading}
            page={ledgerEntries.page}
            pageSize={ledgerPageSize}
            totalCount={ledgerEntries.totalCount}
            totalPages={ledgerEntries.totalPages}
            onPageChange={setLedgerPage}
            onPageSizeChange={(nextPageSize) => {
              setLedgerPage(1);
              setLedgerPageSize(nextPageSize);
            }}
          />
        </div>

        {loading ? (
          <div className="grid min-h-[300px] place-items-center">
            <Spin tip="正在加载资金流水" />
          </div>
        ) : ledgerEntries.items.length > 0 ? (
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
                {ledgerEntries.items.map((entry) => (
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
        </Tabs.TabPane>
      </Tabs>
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

interface PageControlsProps {
  loading: boolean;
  page: number;
  pageSize: number;
  totalCount: number;
  totalPages: number;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
}

function PageControls({
  loading,
  page,
  pageSize,
  totalCount,
  totalPages,
  onPageChange,
  onPageSizeChange,
}: PageControlsProps) {
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs text-slate-500">
      <span>共 {totalCount} 条</span>
      <label className="flex items-center gap-1">
        每页
        <Select
          className="form-input min-w-[86px]"
          value={pageSize}
          onChange={(value) => onPageSizeChange(Number(value ?? 10))}
        >
          {PAGE_SIZE_OPTIONS.map((size) => (
            <Select.Option key={size} value={size}>
              {size}
            </Select.Option>
          ))}
        </Select>
        条
      </label>
      <Button
        disabled={loading || page <= 1 || totalPages === 0}
        size="small"
        onClick={() => onPageChange(page - 1)}
      >
        上一页
      </Button>
      <span>
        第 {totalPages === 0 ? 0 : page} / {totalPages} 页
      </span>
      <Button
        disabled={loading || page >= totalPages || totalPages === 0}
        size="small"
        onClick={() => onPageChange(page + 1)}
      >
        下一页
      </Button>
    </div>
  );
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
    groupBuyDebit: '合买认购',
    groupBuyRefund: '合买退款',
    manualAdjustment: '手动调账',
    orderDebit: '投注扣款',
    orderRefund: '取消退款',
    payoutCredit: '派奖入账',
    rechargeCredit: '充值入账',
    withdrawalFreeze: '提现冻结',
    withdrawalPayout: '提现打款',
    withdrawalReject: '提现驳回解冻',
  };
  return labels[kind];
}

function ledgerKindColor(kind: LedgerEntryKind) {
  const colors: Record<LedgerEntryKind, 'blue' | 'green' | 'orange' | 'red'> = {
    groupBuyDebit: 'red',
    groupBuyRefund: 'blue',
    manualAdjustment: 'orange',
    orderDebit: 'red',
    orderRefund: 'blue',
    payoutCredit: 'green',
    rechargeCredit: 'green',
    withdrawalFreeze: 'red',
    withdrawalPayout: 'red',
    withdrawalReject: 'blue',
  };
  return colors[kind];
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

function withdrawalStatusText(status: WithdrawalOrderStatus) {
  const labels: Record<WithdrawalOrderStatus, string> = {
    approved: '已通过',
    cancelled: '已取消',
    pending: '待审核',
    rejected: '已驳回',
  };
  return labels[status];
}

function withdrawalStatusColor(
  status: WithdrawalOrderStatus,
): 'blue' | 'green' | 'orange' | 'red' {
  const colors: Record<WithdrawalOrderStatus, 'blue' | 'green' | 'orange' | 'red'> = {
    approved: 'green',
    cancelled: 'red',
    pending: 'orange',
    rejected: 'red',
  };
  return colors[status];
}

function withdrawalMethodText(methodType: WithdrawalMethodType) {
  const labels: Record<WithdrawalMethodType, string> = {
    alipay: '支付宝',
    bankCard: '银行卡',
    wechat: '微信',
  };
  return labels[methodType];
}

function pendingWithdrawalCount(page: FinancePage<{ status: WithdrawalOrderStatus }>) {
  return page.items.filter((order) => order.status === 'pending').length;
}
