import {
  Input,
  Banner,
  Button,
  Card,
  Select,
  SideSheet,
  Spin,
  Switch,
  Tabs,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import {
  CheckCircle2,
  Download,
  Plus,
  RefreshCcw,
  Trash2,
  WalletCards,
  XCircle,
} from 'lucide-react';
import {
  useEffect,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { MetricCard } from '../components/MetricCard';
import { PageControls } from '../components/PageControls';
import { useFinance } from '../hooks/useFinance';
import type {
  AdminFinancialAccountSummary,
  FinancePage,
  LedgerEntry,
  LedgerEntryKind,
  ManualBalanceAdjustmentRequest,
  RechargeChannel,
  RechargeOrderStatus,
  WithdrawalMethodType,
  WithdrawalOrderStatus,
} from '../types/finance';
import { formatDateTime, formatMoney, formatSignedMoney } from '../utils/format';
import { yuanInputToMinor } from '../utils/moneyInput';

interface FinanceManagementPageProps {
  onDashboardRefresh: () => void;
  ledgerUserFilter?: UserRecordFilter | null;
  onClearLedgerUserFilter?: () => void;
}

interface UserRecordFilter {
  userId: string;
  username?: string | null;
}

interface AdjustmentFormState {
  amountYuan: string;
  description: string;
  userId: string;
}

export function FinanceManagementPage({
  ledgerUserFilter,
  onClearLedgerUserFilter,
  onDashboardRefresh,
}: FinanceManagementPageProps) {
  const [activeFinanceTab, setActiveFinanceTab] = useState('accounts');
  const [accountPage, setAccountPage] = useState(1);
  const [accountPageSize, setAccountPageSize] = useState(10);
  const [rechargePage, setRechargePage] = useState(1);
  const [rechargePageSize, setRechargePageSize] = useState(10);
  const [withdrawalPage, setWithdrawalPage] = useState(1);
  const [withdrawalPageSize, setWithdrawalPageSize] = useState(10);
  const [ledgerPage, setLedgerPage] = useState(1);
  const [ledgerPageSize, setLedgerPageSize] = useState(20);
  const [includeRobotData, setIncludeRobotData] = useState(false);
  const [accountUsernameSearch, setAccountUsernameSearch] = useState('');
  const {
    accounts,
    adjustBalance,
    approveWithdrawal,
    clearLedgerRecords,
    clearRechargeRecords,
    clearWithdrawalRecords,
    confirmRecharge,
    error,
    exportRechargeRecords,
    ledgerEntries,
    loading,
    overview,
    rechargeOrders,
    refresh,
    rejectWithdrawal,
    saving,
    withdrawalOrders,
  } = useFinance({
    accountQuery: {
      includeRobotData,
      page: accountPage,
      pageSize: accountPageSize,
      username: accountUsernameSearch,
    },
    includeRobotData,
    ledgerQuery: {
      includeRobotData,
      page: ledgerPage,
      pageSize: ledgerPageSize,
      userId: ledgerUserFilter?.userId,
    },
    rechargeQuery: { page: rechargePage, pageSize: rechargePageSize },
    withdrawalQuery: { page: withdrawalPage, pageSize: withdrawalPageSize },
  });
  const [form, setForm] = useState<AdjustmentFormState>({
    amountYuan: '10.00',
    description: '后台手动补款',
    userId: 'U10001',
  });
  const [adjustmentSheetVisible, setAdjustmentSheetVisible] = useState(false);
  const [adjustmentAccount, setAdjustmentAccount] =
    useState<AdminFinancialAccountSummary | null>(null);

  const availableBalanceMinor = overview
    ? overview.totalBalanceMinor - overview.pendingWithdrawMinor
    : 0;

  useEffect(() => {
    if (!ledgerUserFilter?.userId) {
      return;
    }
    setActiveFinanceTab('ledger');
    setLedgerPage(1);
  }, [ledgerUserFilter?.userId]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const openAdjustmentSheet = (account: AdminFinancialAccountSummary) => {
    setAdjustmentAccount(account);
    setForm({
      amountYuan: '10.00',
      description: `后台手动调账：${account.username ?? account.userId}`,
      userId: account.userId,
    });
    setAdjustmentSheetVisible(true);
  };

  const closeAdjustmentSheet = () => {
    setAdjustmentSheetVisible(false);
    setAdjustmentAccount(null);
  };

  const submit = async () => {
    const amountMinor = yuanInputToMinor(form.amountYuan);
    if (amountMinor === null) {
      Toast.warning('请输入正确的调账金额，最多保留两位小数');
      return;
    }
    if (amountMinor === 0) {
      Toast.warning('调账金额不能为 0 元');
      return;
    }
    const userId = form.userId.trim();
    if (!userId) {
      Toast.warning('请选择需要调账的资金账户');
      return;
    }
    const payload: ManualBalanceAdjustmentRequest = {
      amountMinor,
      description: form.description.trim() || '后台手动调账',
      userId,
    };
    await adjustBalance(payload);
    closeAdjustmentSheet();
    onDashboardRefresh();
    Toast.success('调账已提交');
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

  const downloadRechargeOrders = async () => {
    try {
      const blob = await exportRechargeRecords();
      downloadBlob(blob, `充值订单-${dateFileLabel()}.csv`);
      Toast.success('充值记录已开始下载');
    } catch {
      Toast.error('充值记录导出失败，请查看接口错误提示');
    }
  };

  const clearRechargeOrderRecords = async () => {
    if (!window.confirm('确定一键清除全部用户充值记录吗？已入账余额和资金流水不会回滚。')) {
      return;
    }
    try {
      const result = await clearRechargeRecords();
      setRechargePage(1);
      onDashboardRefresh();
      Toast.success(`已清除 ${result.deletedCount} 笔充值记录`);
    } catch {
      Toast.error('充值记录清除失败，请查看接口错误提示');
    }
  };

  const clearWithdrawalOrderRecords = async () => {
    if (!window.confirm('确定一键清除全部提现记录吗？存在待审核申请时系统会拒绝清理。')) {
      return;
    }
    try {
      const result = await clearWithdrawalRecords();
      setWithdrawalPage(1);
      onDashboardRefresh();
      Toast.success(`已清除 ${result.deletedCount} 笔提现记录`);
    } catch {
      Toast.error('提现记录清除失败，请查看接口错误提示');
    }
  };

  const clearLedgerEntryRecords = async () => {
    if (
      !window.confirm(
        '确定一键清除全部资金流水吗？该操作只清除流水审计列表，不会回滚或调整用户余额，后续新流水编号不会重置。',
      )
    ) {
      return;
    }
    try {
      const result = await clearLedgerRecords();
      setLedgerPage(1);
      onDashboardRefresh();
      Toast.success(`已清除 ${result.deletedCount} 笔资金流水`);
    } catch {
      Toast.error('资金流水清除失败，请查看接口错误提示');
    }
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
        <div className="flex flex-wrap items-center gap-3">
          <label className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-2 text-sm text-slate-600">
            <Switch
              checked={includeRobotData}
              onChange={(checked) => {
                setIncludeRobotData(checked);
                setAccountPage(1);
                setLedgerPage(1);
              }}
            />
            <span>显示机器人数据</span>
          </label>
          <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
            刷新
          </Button>
          {ledgerUserFilter ? (
            <Tag color="purple" closable onClose={onClearLedgerUserFilter}>
              流水用户：{ledgerUserFilter.username || '未知用户'}（{ledgerUserFilter.userId}）
            </Tag>
          ) : null}
        </div>
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
          <section className="pt-3">
            <Card className="rounded-md border border-line">
              <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                <div className="flex items-center gap-2">
                  <h2 className="text-base font-semibold text-ink">资金账户</h2>
                  <Tag color="cyan">{accounts.totalCount} 个账户</Tag>
                </div>
                <div className="flex flex-col gap-2 lg:flex-row lg:items-center">
                  <Input
                    className="form-input min-w-[220px]"
                    placeholder="按用户名搜索"
                    value={accountUsernameSearch}
                    onChange={(value) => {
                      setAccountUsernameSearch(value);
                      setAccountPage(1);
                    }}
                  />
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
              </div>

          {loading ? (
            <div className="grid min-h-[260px] place-items-center">
              <Spin tip="正在加载资金账户" />
            </div>
          ) : accounts.items.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[860px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">用户</th>
                    <th className="py-2 pr-4 font-medium">可用余额</th>
                    <th className="py-2 pr-4 font-medium">冻结余额</th>
                    <th className="py-2 pr-4 font-medium">账户总额</th>
                    <th className="py-2 pr-4 font-medium">状态</th>
                    <th className="py-2 pr-4 font-medium">操作</th>
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
                      <td className="py-3 pr-4">
                        <Button
                          icon={<WalletCards size={14} />}
                          size="small"
                          theme="solid"
                          onClick={() => openAdjustmentSheet(account)}
                        >
                          调账
                        </Button>
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
          <div className="flex flex-wrap items-center gap-2">
            <Button
              disabled={saving || loading || rechargeOrders.totalCount === 0}
              icon={<Download size={16} />}
              onClick={() => void downloadRechargeOrders()}
            >
              导出记录
            </Button>
            <Button
              disabled={saving || loading || rechargeOrders.totalCount === 0}
              icon={<Trash2 size={16} />}
              theme="solid"
              type="danger"
              onClick={() => void clearRechargeOrderRecords()}
            >
              一键清除
            </Button>
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
                        {formatDateTime(order.createdAt, order.createdAt || '-')}
                        {order.paidAt
                          ? ` · ${formatDateTime(order.paidAt, order.paidAt)}`
                          : ''}
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
          <div className="flex flex-wrap items-center gap-2">
            <Button
              disabled={saving || loading || withdrawalOrders.totalCount === 0}
              icon={<Trash2 size={16} />}
              theme="solid"
              type="danger"
              onClick={() => void clearWithdrawalOrderRecords()}
            >
              一键清除
            </Button>
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
                      <div className="mt-1 text-xs text-slate-400">
                        {formatDateTime(order.createdAt, order.createdAt || '-')}
                      </div>
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
                      {order.reviewedAt
                        ? formatDateTime(order.reviewedAt, order.reviewedAt)
                        : '-'}
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
            {ledgerUserFilter ? (
              <Tag color="purple" closable onClose={onClearLedgerUserFilter}>
                {ledgerUserFilter.username || '未知用户'}（{ledgerUserFilter.userId}）
              </Tag>
            ) : null}
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              disabled={saving || loading || ledgerEntries.totalCount === 0}
              icon={<Trash2 size={16} />}
              theme="solid"
              type="danger"
              onClick={() => void clearLedgerEntryRecords()}
            >
              一键清除
            </Button>
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
                      <div className="mt-1 text-xs text-slate-400">
                        {formatDateTime(entry.createdAt, entry.createdAt || '-')}
                      </div>
                    </td>
                    <td className="py-3 pr-4">
                      <div className="font-medium text-slate-700">
                        {entry.username ?? '未知用户'}
                      </div>
                      <div className="mt-1 text-xs text-slate-400">{entry.userId}</div>
                    </td>
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

      <SideSheet
        aria-label="手动调账"
        title="手动调账"
        visible={adjustmentSheetVisible}
        width={460}
        onCancel={closeAdjustmentSheet}
      >
        <div className="mb-4">
          <h2 className="text-base font-semibold text-ink">资金账户调账</h2>
          <p className="mt-1 text-sm text-slate-500">
            当前调账对象来自资金账户列表，金额使用元，负数表示扣减可用余额。
          </p>
        </div>

        {adjustmentAccount ? (
          <div className="mb-4 rounded-md bg-slate-50 p-3 text-sm text-slate-600">
            <div className="font-semibold text-ink">
              {adjustmentAccount.username ?? '未知用户'}
            </div>
            <div className="mt-1 text-xs text-slate-400">{adjustmentAccount.userId}</div>
            <div className="mt-3 grid grid-cols-2 gap-3">
              <div>
                <div className="text-xs text-slate-400">可用余额</div>
                <div className="mt-1 font-semibold text-emerald-700">
                  {formatMoney(adjustmentAccount.availableBalanceMinor)}
                </div>
              </div>
              <div>
                <div className="text-xs text-slate-400">冻结余额</div>
                <div className="mt-1 font-semibold text-slate-700">
                  {formatMoney(adjustmentAccount.frozenBalanceMinor)}
                </div>
              </div>
            </div>
          </div>
        ) : null}

        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
          }}
        >
          <Field label="用户 ID">
            <Input className="form-input" disabled value={form.userId} />
          </Field>

          <Field label="调账金额（元）">
            <Input
              className="form-input"
              inputMode="decimal"
              placeholder="例如 10 或 -5.50"
              value={form.amountYuan}
              onChange={(value) => setFormValue(setForm, 'amountYuan', value)}
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
            disabled={saving || !adjustmentAccount}
            icon={<Plus size={16} />}
            theme="solid"
            onClick={() => void submit()}
          >
            {saving ? '提交中' : '提交调账'}
          </Button>
        </form>
      </SideSheet>
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

function setFormValue<K extends keyof AdjustmentFormState>(
  setForm: Dispatch<SetStateAction<AdjustmentFormState>>,
  key: K,
  value: AdjustmentFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function downloadBlob(blob: Blob, fileName: string) {
  const url = window.URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = fileName;
  link.click();
  window.URL.revokeObjectURL(url);
}

function dateFileLabel() {
  return new Date().toISOString().slice(0, 10);
}

function ledgerKindText(kind: LedgerEntryKind) {
  const labels: Record<LedgerEntryKind, string> = {
    agentRebateWithdrawal: '代理返利提现',
    groupBuyDebit: '合买认购',
    groupBuyRefund: '合买退款',
    manualAdjustment: '手动调账',
    orderDebit: '投注扣款',
    orderRefund: '取消退款',
    payoutCredit: '派奖入账',
    rechargeCredit: '充值入账',
    rechargeRebateCredit: '充值返利',
    redPacketCredit: '红包入账',
    redPacketDebit: '红包支出',
    withdrawalFreeze: '提现冻结',
    withdrawalPayout: '提现打款',
    withdrawalReject: '提现驳回解冻',
  };
  return labels[kind];
}

function ledgerKindColor(kind: LedgerEntryKind) {
  const colors: Record<LedgerEntryKind, 'blue' | 'green' | 'orange' | 'red'> = {
    agentRebateWithdrawal: 'red',
    groupBuyDebit: 'red',
    groupBuyRefund: 'blue',
    manualAdjustment: 'orange',
    orderDebit: 'red',
    orderRefund: 'blue',
    payoutCredit: 'green',
    rechargeCredit: 'green',
    rechargeRebateCredit: 'green',
    redPacketCredit: 'green',
    redPacketDebit: 'red',
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
