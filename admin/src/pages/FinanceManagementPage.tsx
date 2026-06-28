import {
  Input,
  Banner,
  Button,
  Card,
  Modal,
  Select,
  SideSheet,
  Spin,
  Switch,
  Tabs,
  Tag,
  TextArea,
  Toast,
} from '@douyinfe/semi-ui';
import {
  CheckCircle2,
  Download,
  Eye,
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
import { fetchGroupBuyPlan, fetchOrder, fetchOrderGroupBuyPlan } from '../api/client';
import { MetricCard } from '../components/MetricCard';
import { OrderBetInfo } from '../components/OrderBetInfo';
import { PageControls } from '../components/PageControls';
import { useFinance } from '../hooks/useFinance';
import { usePlayRules } from '../hooks/usePlayRules';
import type { GroupBuyPlan, GroupBuyPlanStatus } from '../types/groupBuy';
import type {
  AdminFinancialAccountSummary,
  LedgerEntry,
  LedgerEntryKind,
  ManualBalanceAdjustmentRequest,
  RechargeChannel,
  RechargeOrderSummary,
  RechargeOrderStatus,
  WithdrawalMethodType,
  WithdrawalOrderStatus,
} from '../types/finance';
import type { OrderDetail, OrderStatus } from '../types/orders';
import type { PlayRuleCode, PlayRuleSummary } from '../types/playRules';
import { formatDateTime, formatMoney, formatSignedMoney } from '../utils/format';
import { yuanInputToMinor } from '../utils/moneyInput';
import { formatPlayRuleLabel } from '../utils/playRules';

interface FinanceManagementPageProps {
  onDashboardRefresh: () => void;
  ledgerUserFilter?: UserRecordFilter | null;
  onClearLedgerUserFilter?: () => void;
}

interface UserRecordFilter {
  userId: string;
  username?: string | null;
}

type LedgerKindFilter = LedgerEntryKind | 'all';

interface AdjustmentFormState {
  amountYuan: string;
  description: string;
  userId: string;
}

interface RechargeConfirmFormState {
  providerTradeNo: string;
  remark: string;
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
  const [ledgerKindFilter, setLedgerKindFilter] =
    useState<LedgerKindFilter>('all');
  const [includeRobotData, setIncludeRobotData] = useState(false);
  const [accountUsernameSearch, setAccountUsernameSearch] = useState('');
  const { rules: playRules } = usePlayRules();
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
      kind: ledgerKindFilter === 'all' ? undefined : ledgerKindFilter,
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
  const [rechargeConfirmOrder, setRechargeConfirmOrder] =
    useState<RechargeOrderSummary | null>(null);
  const [rechargeConfirmForm, setRechargeConfirmForm] =
    useState<RechargeConfirmFormState>({
      providerTradeNo: '',
      remark: '',
    });
  const [adjustmentSheetVisible, setAdjustmentSheetVisible] = useState(false);
  const [adjustmentAccount, setAdjustmentAccount] =
    useState<AdminFinancialAccountSummary | null>(null);
  const [ledgerDetailEntry, setLedgerDetailEntry] = useState<LedgerEntry | null>(null);
  const [ledgerDetailLoading, setLedgerDetailLoading] = useState(false);
  const [ledgerDetailError, setLedgerDetailError] = useState<string | null>(null);
  const [ledgerDetailOrder, setLedgerDetailOrder] = useState<OrderDetail | null>(null);
  const [ledgerDetailPlan, setLedgerDetailPlan] = useState<GroupBuyPlan | null>(null);

  useEffect(() => {
    if (!ledgerUserFilter?.userId) {
      return;
    }
    setActiveFinanceTab('ledger');
    setLedgerPage(1);
  }, [ledgerUserFilter?.userId]);

  const changeLedgerKindFilter = (value: unknown) => {
    setLedgerPage(1);
    setLedgerKindFilter(value === 'all' ? 'all' : (value as LedgerEntryKind));
  };

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

  const openLedgerDetail = async (entry: LedgerEntry) => {
    setLedgerDetailEntry(entry);
    setLedgerDetailError(null);
    setLedgerDetailOrder(null);
    setLedgerDetailPlan(null);
    setLedgerDetailLoading(true);
    try {
      const orderId = orderIdFromLedgerEntry(entry);
      const planId = groupBuyPlanIdFromLedgerEntry(entry);
      const nextOrder = orderId ? await fetchOrder(orderId) : null;
      setLedgerDetailOrder(nextOrder);

      if (nextOrder?.orderSource === 'groupBuy') {
        const plan = await fetchOrderGroupBuyPlan(nextOrder.id);
        setLedgerDetailPlan(plan);
      } else if (planId) {
        const plan = await fetchGroupBuyPlan(planId);
        setLedgerDetailPlan(plan);
      }
    } catch (requestError) {
      setLedgerDetailError(errorMessage(requestError));
    } finally {
      setLedgerDetailLoading(false);
    }
  };

  const closeLedgerDetail = () => {
    setLedgerDetailEntry(null);
    setLedgerDetailError(null);
    setLedgerDetailOrder(null);
    setLedgerDetailPlan(null);
  };

  const openRechargeConfirmModal = (order: RechargeOrderSummary) => {
    setRechargeConfirmOrder(order);
    setRechargeConfirmForm({
      providerTradeNo: order.providerTradeNo ?? '',
      remark: order.remark ?? '',
    });
  };

  const closeRechargeConfirmModal = () => {
    if (saving) {
      return;
    }
    setRechargeConfirmOrder(null);
    setRechargeConfirmForm({ providerTradeNo: '', remark: '' });
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

  const submitRechargeConfirmation = async () => {
    if (!rechargeConfirmOrder) {
      return;
    }
    await confirmRecharge(rechargeConfirmOrder.id, {
      providerTradeNo: rechargeConfirmForm.providerTradeNo.trim() || null,
      remark: rechargeConfirmForm.remark.trim() || null,
    });
    setRechargeConfirmOrder(null);
    setRechargeConfirmForm({ providerTradeNo: '', remark: '' });
    onDashboardRefresh();
    Toast.success('充值订单已确认入账');
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

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="今日充值"
          trend="充值入账流水"
          value={formatMoney(overview?.todayRechargeMinor ?? 0)}
        />
        <MetricCard
          label="今日提现"
          trend="提现打款流水"
          value={formatMoney(overview?.todayWithdrawMinor ?? 0)}
        />
        <MetricCard
          label="总充值"
          trend="累计充值入账"
          value={formatMoney(overview?.totalRechargeMinor ?? 0)}
        />
        <MetricCard
          label="总提现"
          trend="累计提现打款"
          value={formatMoney(overview?.totalWithdrawMinor ?? 0)}
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
              <table className="w-full min-w-[980px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">用户</th>
                    <th className="py-2 pr-4 font-medium">上级代理</th>
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
                      <td className="py-3 pr-4">
                        <AgentCell
                          agentId={account.agentId}
                          agentUsername={account.agentUsername}
                        />
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
            <table className="w-full min-w-[1400px] text-left text-sm">
              <thead className="border-b border-line text-xs text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">订单</th>
                  <th className="py-2 pr-4 font-medium">用户</th>
                  <th className="py-2 pr-4 font-medium">上级代理</th>
                  <th className="py-2 pr-4 font-medium">渠道</th>
                  <th className="py-2 pr-4 font-medium">金额</th>
                  <th className="py-2 pr-4 font-medium">状态</th>
                  <th className="py-2 pr-4 font-medium">支付方式</th>
                  <th className="py-2 pr-4 font-medium">外部交易号</th>
                  <th className="py-2 pr-4 font-medium">客服会话</th>
                  <th className="py-2 pr-4 font-medium">备注</th>
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
                      <AgentCell
                        agentId={order.agentId}
                        agentUsername={order.agentUsername}
                      />
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
                    <td className="py-3 pr-4 text-slate-600">
                      {order.remark ? (
                        <div className="max-w-[220px] whitespace-pre-wrap break-words">
                          {order.remark}
                        </div>
                      ) : (
                        '-'
                      )}
                    </td>
                    <td className="py-3 pr-4">
                      {order.channel === 'customerService' &&
                      order.status === 'waitingCustomerService' ? (
                        <Button
                          disabled={saving}
                          size="small"
                          theme="solid"
                          onClick={() => openRechargeConfirmModal(order)}
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
            <table className="w-full min-w-[1300px] text-left text-sm">
              <thead className="border-b border-line text-xs text-slate-500">
                <tr>
                  <th className="py-2 pr-4 font-medium">申请</th>
                  <th className="py-2 pr-4 font-medium">用户</th>
                  <th className="py-2 pr-4 font-medium">上级代理</th>
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
                    <td className="py-3 pr-4">
                      <AgentCell
                        agentId={order.agentId}
                        agentUsername={order.agentUsername}
                      />
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
            <label className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-2 text-sm text-slate-600">
              <Switch
                checked={includeRobotData}
                onChange={(checked) => {
                  setIncludeRobotData(checked);
                  setAccountPage(1);
                  setLedgerPage(1);
                }}
              />
              <span>显示机器人流水</span>
            </label>
            <div className="flex items-center gap-2 text-sm text-slate-600">
              <span className="text-xs font-medium text-slate-500">类型</span>
              <Select
                className="form-input min-w-[148px]"
                value={ledgerKindFilter}
                onChange={changeLedgerKindFilter}
              >
                <Select.Option value="all">全部类型</Select.Option>
                {LEDGER_KIND_OPTIONS.map((kind) => (
                  <Select.Option key={kind} value={kind}>
                    {ledgerKindText(kind)}
                  </Select.Option>
                ))}
              </Select>
            </div>
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
                  <th className="py-2 pr-4 font-medium">说明</th>
                  <th className="py-2 pr-4 font-medium">操作</th>
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
                    <td className="py-3 pr-4 text-slate-600">{entry.description}</td>
                    <td className="py-3 pr-4">
                      <Button
                        icon={<Eye size={14} />}
                        loading={ledgerDetailLoading && ledgerDetailEntry?.id === entry.id}
                        size="small"
                        onClick={() => void openLedgerDetail(entry)}
                      >
                        查看详情
                      </Button>
                    </td>
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

      <Modal
        cancelText="取消"
        closeOnEsc={!saving}
        confirmLoading={saving}
        maskClosable={!saving}
        okText="确认入账"
        title="确认充值入账"
        visible={Boolean(rechargeConfirmOrder)}
        onCancel={closeRechargeConfirmModal}
        onOk={() => void submitRechargeConfirmation()}
      >
        {rechargeConfirmOrder ? (
          <div className="space-y-4">
            <div className="rounded-md bg-slate-50 p-3 text-sm text-slate-600">
              <div className="font-semibold text-ink">
                {rechargeConfirmOrder.username}（{rechargeConfirmOrder.userId}）
              </div>
              <div className="mt-1 text-xs text-slate-400">
                订单 {rechargeConfirmOrder.id} · 金额{' '}
                {formatMoney(rechargeConfirmOrder.amountMinor)}
              </div>
            </div>

            <Field label="外部交易号（可选）">
              <Input
                className="form-input"
                placeholder="可填写线下收款凭证号或第三方交易号"
                value={rechargeConfirmForm.providerTradeNo}
                onChange={(value) =>
                  setRechargeConfirmForm((current) => ({
                    ...current,
                    providerTradeNo: value,
                  }))
                }
              />
            </Field>

            <Field label="入账备注">
              <TextArea
                autosize={{ maxRows: 5, minRows: 3 }}
                className="form-input"
                placeholder="例如：已核对付款截图，客服确认收款"
                value={rechargeConfirmForm.remark}
                onChange={(value) =>
                  setRechargeConfirmForm((current) => ({
                    ...current,
                    remark: value,
                  }))
                }
              />
            </Field>
          </div>
        ) : null}
      </Modal>

      <SideSheet
        aria-label="资金流水详情"
        title="资金流水详情"
        visible={Boolean(ledgerDetailEntry)}
        width="80%"
        onCancel={closeLedgerDetail}
      >
        {ledgerDetailEntry ? (
          <div className="space-y-4">
            <section className="rounded-md border border-line bg-slate-50 p-4">
              <div className="mb-4 flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                <div>
                  <h2 className="text-base font-semibold text-ink">流水信息</h2>
                  <p className="mt-1 break-all text-sm text-slate-500">
                    流水号：{ledgerDetailEntry.id}
                  </p>
                </div>
                <Tag color={ledgerKindColor(ledgerDetailEntry.kind)}>
                  {ledgerKindText(ledgerDetailEntry.kind)}
                </Tag>
              </div>
              <div className="grid gap-3 md:grid-cols-3">
                <InfoLine
                  label="用户"
                  value={`${ledgerDetailEntry.username ?? '未知用户'}（${ledgerDetailEntry.userId}）`}
                />
                <InfoLine
                  label="变动金额"
                  value={formatSignedMoney(ledgerDetailEntry.amountMinor)}
                />
                <InfoLine
                  label="变更后余额"
                  value={formatMoney(ledgerDetailEntry.balanceAfterMinor)}
                />
                <InfoLine
                  label="关联单据"
                  value={ledgerDetailEntry.referenceId ?? '-'}
                />
                <InfoLine
                  label="创建时间"
                  value={formatDateTime(
                    ledgerDetailEntry.createdAt,
                    ledgerDetailEntry.createdAt || '-',
                  )}
                />
                <InfoLine label="说明" value={ledgerDetailEntry.description || '-'} />
              </div>
            </section>

            <section className="rounded-md border border-line p-4">
              <div className="mb-4 flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                <div>
                  <h2 className="text-base font-semibold text-ink">
                    {ledgerBusinessTitle(ledgerDetailEntry, ledgerDetailOrder)}
                  </h2>
                </div>
                {ledgerDetailPlan ? (
                  <Tag color={groupBuyStatusColor(ledgerDetailPlan.status)}>
                    {groupBuyStatusText(ledgerDetailPlan.status)}
                  </Tag>
                ) : ledgerDetailOrder ? (
                  <Tag color={orderStatusColor(ledgerDetailOrder.status)}>
                    {orderStatusText(ledgerDetailOrder.status)}
                  </Tag>
                ) : null}
              </div>

              {ledgerDetailLoading ? (
                <div className="grid min-h-[240px] place-items-center">
                  <Spin tip="正在加载关联业务详情" />
                </div>
              ) : ledgerDetailError ? (
                <Banner
                  type="danger"
                  title="关联业务详情加载失败"
                  description={ledgerDetailError}
                />
              ) : (
                <LedgerBusinessDetail
                  entry={ledgerDetailEntry}
                  order={ledgerDetailOrder}
                  playRules={playRules}
                  plan={ledgerDetailPlan}
                />
              )}
            </section>
          </div>
        ) : null}
      </SideSheet>

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

interface InfoLineProps {
  label: string;
  value: ReactNode;
}

function InfoLine({ label, value }: InfoLineProps) {
  return (
    <div className="rounded-md border border-line bg-white px-3 py-2">
      <div className="text-xs text-slate-500">{label}</div>
      <div className="mt-1 break-all text-sm font-medium text-ink">{value}</div>
    </div>
  );
}

interface LedgerBusinessDetailProps {
  entry: LedgerEntry;
  order: OrderDetail | null;
  playRules: Array<Pick<PlayRuleSummary, 'code' | 'label'>>;
  plan: GroupBuyPlan | null;
}

function LedgerBusinessDetail({
  entry,
  order,
  playRules,
  plan,
}: LedgerBusinessDetailProps) {
  const participant = plan ? groupBuyParticipantFromLedgerEntry(plan, entry) : null;

  if (!order && !plan) {
    return (
      <div className="rounded-md border border-dashed border-line py-10 text-center text-sm text-slate-500">
        当前流水没有可直接查看的投注或合买业务详情。
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {order ? (
        <section className="rounded-md border border-line bg-slate-50 p-4">
          <div className="mb-4 flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <h3 className="text-sm font-semibold text-ink">投注订单</h3>
              <p className="mt-1 break-all text-xs text-slate-500">订单号：{order.id}</p>
            </div>
            <Tag color={orderStatusColor(order.status)}>{orderStatusText(order.status)}</Tag>
          </div>
          <div className="grid gap-3 md:grid-cols-3">
            <InfoLine label="下单类型" value={orderSourceText(order.orderSource)} />
            <InfoLine label="用户" value={`${order.username ?? '未知用户'}（${order.userId}）`} />
            <InfoLine label="彩种" value={`${order.lotteryName}（${order.lotteryId}）`} />
            <InfoLine label="期号" value={order.issue} />
            <InfoLine
              label="玩法"
              value={formatPlayRuleLabel(order.ruleCode, playRules)}
            />
            <InfoLine label="投注金额" value={formatMoney(order.amountMinor)} />
            <InfoLine label="注数" value={`${order.stakeCount} 注`} />
            <InfoLine label="开奖号码" value={order.drawNumber ?? '未开奖'} />
            <InfoLine label="派奖金额" value={formatMoney(order.payoutMinor)} />
            <InfoLine
              label="结算时间"
              value={formatDateTime(order.settledAt, order.settledAt ?? '-')}
            />
          </div>
          <div className="mt-4 rounded-md border border-line bg-white px-3 py-3">
            <div className="mb-2 text-xs text-slate-500">投注内容</div>
            <OrderBetInfo order={order} />
          </div>
          {order.matchedBets.length > 0 ? (
            <div className="mt-4 rounded-md border border-line bg-white px-3 py-3">
              <div className="mb-2 text-xs text-slate-500">命中注码</div>
              <div className="flex flex-wrap gap-1">
                {order.matchedBets.map((bet) => (
                  <Tag key={bet} color="green">
                    {bet}
                  </Tag>
                ))}
              </div>
            </div>
          ) : null}
        </section>
      ) : null}

      {plan ? (
        <section className="rounded-md border border-line bg-slate-50 p-4">
          <div className="mb-4 flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <h3 className="text-sm font-semibold text-ink">合买计划</h3>
              <p className="mt-1 break-all text-xs text-slate-500">计划编号：{plan.id}</p>
            </div>
            <Tag color={groupBuyStatusColor(plan.status)}>{groupBuyStatusText(plan.status)}</Tag>
          </div>
          <div className="grid gap-3 md:grid-cols-3">
            <InfoLine label="彩种" value={`${plan.lotteryName}（${plan.lotteryId}）`} />
            <InfoLine label="期号" value={plan.issue || '-'} />
            <InfoLine label="玩法" value={financePlayRuleLabel(plan.ruleCode, playRules)} />
            <InfoLine label="关联订单" value={plan.orderId ?? '未成单'} />
            <InfoLine
              label="中奖状态"
              value={groupBuyOrderStatusText(plan)}
            />
            <InfoLine label="开奖号码" value={plan.orderDrawNumber ?? '-'} />
            <InfoLine
              label="整单派奖"
              value={plan.orderPayoutMinor == null ? '-' : formatMoney(plan.orderPayoutMinor)}
            />
            <InfoLine
              label="发起人"
              value={`${plan.initiatorUsername}（${plan.initiatorUserId}）`}
            />
            <InfoLine label="计划总额" value={formatMoney(plan.totalAmountMinor)} />
            <InfoLine label="已认购" value={formatMoney(plan.filledAmountMinor)} />
            <InfoLine label="进度" value={`${groupBuyProgressPercent(plan)}%`} />
            <InfoLine label="总份数" value={`${plan.shareCount} 份`} />
          </div>
          <div className="mt-4">
            <div className="mb-1 flex items-center justify-between text-xs text-slate-500">
              <span>合买进度</span>
              <span>{groupBuyProgressPercent(plan)}%</span>
            </div>
            <div className="h-2 overflow-hidden rounded-full bg-white ring-1 ring-line">
              <div
                className="h-full rounded-full bg-teal-500"
                style={{ width: `${groupBuyProgressPercent(plan)}%` }}
              />
            </div>
          </div>
          <div className="mt-4 rounded-md border border-line bg-white px-3 py-2">
            <div className="text-xs text-slate-500">投注内容</div>
            <div className="mt-1 whitespace-pre-wrap break-all text-sm font-medium text-ink">
              {plan.numbers || '-'}
            </div>
          </div>
        </section>
      ) : null}

      {plan ? (
        <section className="rounded-md border border-line p-4">
          <div className="mb-4 flex items-center justify-between gap-2">
            <h3 className="text-sm font-semibold text-ink">
              {participant ? '本流水认购记录' : '参与记录'}
            </h3>
            <Tag color="orange">{plan.participants.length} 条</Tag>
          </div>
          {participant ? (
            <div className="mb-4 grid gap-3 md:grid-cols-4">
              <InfoLine
                label="认购用户"
                value={`${participant.username}（${participant.userId}）`}
              />
              <InfoLine label="认购金额" value={formatMoney(participant.amountMinor)} />
              <InfoLine label="认购份数" value={`${participant.shareCount} 份`} />
              <InfoLine
                label="占比"
                value={`${participantPercent(participant.amountMinor, plan)}%`}
              />
              <InfoLine
                label="个人派奖"
                value={
                  entry.kind === 'payoutCredit'
                    ? formatMoney(entry.amountMinor)
                    : '-'
                }
              />
              <InfoLine
                label="认购时间"
                value={formatDateTime(participant.createdAt, participant.createdAt || '-')}
              />
              <InfoLine label="参与编号" value={participant.id} />
              <InfoLine label="备注" value={participant.note || '-'} />
            </div>
          ) : null}
          {plan.participants.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[680px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">用户</th>
                    <th className="py-2 pr-4 font-medium">金额</th>
                    <th className="py-2 pr-4 font-medium">份数</th>
                    <th className="py-2 pr-4 font-medium">占比</th>
                    <th className="py-2 pr-4 font-medium">创建时间</th>
                    <th className="py-2 pr-4 font-medium">备注</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-line">
                  {plan.participants.map((item) => (
                    <tr key={item.id}>
                      <td className="py-3 pr-4">
                        <div className="font-medium text-ink">{item.username}</div>
                        <div className="mt-1 text-xs text-slate-400">{item.userId}</div>
                      </td>
                      <td className="py-3 pr-4 font-semibold text-ink">
                        {formatMoney(item.amountMinor)}
                      </td>
                      <td className="py-3 pr-4 text-slate-600">{item.shareCount} 份</td>
                      <td className="py-3 pr-4 text-slate-600">
                        {participantPercent(item.amountMinor, plan)}%
                      </td>
                      <td className="py-3 pr-4 text-slate-500">
                        {formatDateTime(item.createdAt, item.createdAt || '-')}
                      </td>
                      <td className="py-3 pr-4 text-slate-500">{item.note || '-'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="rounded-md border border-dashed border-line py-8 text-center text-sm text-slate-500">
              暂无参与记录
            </div>
          )}
        </section>
      ) : null}
    </div>
  );
}

interface AgentCellProps {
  agentId?: string | null;
  agentUsername?: string | null;
}

function AgentCell({ agentId, agentUsername }: AgentCellProps) {
  if (!agentId) {
    return <span className="text-slate-400">无</span>;
  }

  return (
    <div className="min-w-0">
      <div className="font-medium text-slate-700">{agentUsername ?? '未知代理'}</div>
      <div className="mt-1 text-xs text-slate-400">{agentId}</div>
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

function financePlayRuleLabel(
  ruleCode: string,
  rules: Array<Pick<PlayRuleSummary, 'code' | 'label'>>,
) {
  return formatPlayRuleLabel(ruleCode as PlayRuleCode, rules);
}

function orderIdFromLedgerEntry(entry: LedgerEntry) {
  const referenceId = entry.referenceId?.trim();
  if (!referenceId) {
    return null;
  }
  if (entry.kind === 'orderDebit' || entry.kind === 'orderRefund') {
    return referenceId;
  }
  if (entry.kind === 'payoutCredit') {
    return referenceId.split(':')[1]?.trim() || null;
  }
  return null;
}

function participantIdFromLedgerEntry(entry: LedgerEntry) {
  const referenceId = entry.referenceId?.trim();
  if (!referenceId) {
    return null;
  }
  if (entry.kind === 'groupBuyDebit' || entry.kind === 'groupBuyRefund') {
    return referenceId;
  }
  if (entry.kind === 'payoutCredit') {
    return referenceId.split(':')[2]?.trim() || null;
  }
  return null;
}

function groupBuyPlanIdFromLedgerEntry(entry: LedgerEntry) {
  const participantId = participantIdFromLedgerEntry(entry);
  if (!participantId) {
    return null;
  }
  return groupBuyPlanIdFromParticipantId(participantId);
}

function groupBuyPlanIdFromParticipantId(participantId: string) {
  const match = participantId.match(/^(.+)-P\d+$/i);
  return match?.[1] ?? null;
}

function groupBuyParticipantFromLedgerEntry(plan: GroupBuyPlan, entry: LedgerEntry) {
  const participantId = participantIdFromLedgerEntry(entry);
  if (participantId) {
    const byId = plan.participants.find((participant) => participant.id === participantId);
    if (byId) {
      return byId;
    }
  }
  return plan.participants.find((participant) => participant.userId === entry.userId) ?? null;
}

function ledgerBusinessTitle(entry: LedgerEntry, order: OrderDetail | null) {
  if (entry.kind === 'groupBuyDebit') {
    return '合买认购详情';
  }
  if (entry.kind === 'orderDebit') {
    return '投注扣款详情';
  }
  if (entry.kind === 'payoutCredit') {
    return order?.orderSource === 'groupBuy' ? '合买派奖详情' : '独立下单派奖详情';
  }
  if (entry.kind === 'groupBuyRefund') {
    return '合买退款详情';
  }
  if (entry.kind === 'orderRefund') {
    return '投注退款详情';
  }
  return '关联业务详情';
}

function groupBuyProgressPercent(plan: GroupBuyPlan) {
  if (plan.totalAmountMinor <= 0) {
    return 0;
  }

  return Math.min(
    100,
    Math.round((plan.filledAmountMinor / plan.totalAmountMinor) * 100),
  );
}

function participantPercent(amountMinor: number, plan: GroupBuyPlan) {
  if (plan.totalAmountMinor <= 0) {
    return 0;
  }

  return Math.round((amountMinor / plan.totalAmountMinor) * 10000) / 100;
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
  const colors = {
    cancelled: 'grey',
    lost: 'red',
    pendingDraw: 'blue',
    won: 'green',
  } as const;
  return colors[status];
}

function orderSourceText(source: OrderDetail['orderSource']) {
  return source === 'groupBuy' ? '合买下单' : '独立下单';
}

function groupBuyStatusText(status: GroupBuyPlanStatus) {
  const labels: Record<GroupBuyPlanStatus, string> = {
    cancelled: '已取消',
    draft: '草稿',
    filled: '已满单',
    open: '进行中',
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
  } as const;
  return colors[status];
}

function groupBuyOrderStatusText(plan: GroupBuyPlan) {
  if (!plan.orderId) {
    return '未成单';
  }
  if (!plan.orderStatus) {
    return '订单缺失';
  }
  const labels = {
    cancelled: '已取消',
    lost: '未中奖',
    pendingDraw: '待开奖',
    won: '已中奖',
  } as const;
  return labels[plan.orderStatus];
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}

const LEDGER_KIND_OPTIONS: LedgerEntryKind[] = [
  'agentRebateWithdrawal',
  'groupBuyDebit',
  'groupBuyRefund',
  'manualAdjustment',
  'orderDebit',
  'orderRefund',
  'payoutCredit',
  'rechargeBonusCredit',
  'rechargeCredit',
  'rechargeRebateCredit',
  'redPacketCredit',
  'redPacketDebit',
  'withdrawalFreeze',
  'withdrawalPayout',
  'withdrawalReject',
];

function ledgerKindText(kind: LedgerEntryKind) {
  const labels: Record<LedgerEntryKind, string> = {
    agentRebateWithdrawal: '代理返利提现',
    groupBuyDebit: '合买认购',
    groupBuyRefund: '合买退款',
    manualAdjustment: '手动调账',
    orderDebit: '投注扣款',
    orderRefund: '取消退款',
    payoutCredit: '派奖入账',
    rechargeBonusCredit: '充值赠送',
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
    rechargeBonusCredit: 'green',
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
