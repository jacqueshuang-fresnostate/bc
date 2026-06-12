import {
  Input,
  Banner,
  Button,
  Card,
  Select,
  SideSheet,
  Spin,
  Tabs,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import {
  Eye,
  Percent,
  RefreshCcw,
  Save,
  UserPlus,
  Users,
  WalletCards,
} from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { MetricCard } from '../components/MetricCard';
import { PageControls } from '../components/PageControls';
import { useRebatePolicy } from '../hooks/useRebatePolicy';
import type {
  AgentRebateSummary,
  InvitePolicySummary,
  InvitePolicyUpdateRequest,
  RebateMode,
} from '../types/rebates';
import { formatDateTime, formatMoney } from '../utils/format';
import { yuanInputToMinor } from '../utils/moneyInput';

interface RebateManagementPageProps {
  onDashboardRefresh: () => void;
}

interface RebateFormState {
  agentsCanInvite: boolean;
  defaultRechargeRebatePercent: string;
  rebateMode: RebateMode;
  regularUsersCanInvite: boolean;
}

export function RebateManagementPage({
  onDashboardRefresh,
}: RebateManagementPageProps) {
  const [activeTab, setActiveTab] = useState('statistics');
  const [statisticsPage, setStatisticsPage] = useState(1);
  const [statisticsPageSize, setStatisticsPageSize] = useState(10);
  const [recordsPage, setRecordsPage] = useState(1);
  const [recordsPageSize, setRecordsPageSize] = useState(10);
  const [selectedAgent, setSelectedAgent] = useState<AgentRebateSummary | null>(null);
  const [withdrawAmountYuan, setWithdrawAmountYuan] = useState('');
  const [withdrawDescription, setWithdrawDescription] = useState('代理返利提现处理');
  const {
    error,
    loadRecords,
    loading,
    policy,
    records,
    recordsLoading,
    refresh,
    registration,
    save,
    saving,
    statistics,
    withdraw,
  } = useRebatePolicy({ page: statisticsPage, pageSize: statisticsPageSize });
  const [form, setForm] = useState<RebateFormState>(() => emptyForm());
  const currentMode = policy?.rebateMode ?? form.rebateMode;
  const totals = useMemo(() => policyTotals(policy), [policy]);
  const statisticTotals = useMemo(
    () => visibleStatisticTotals(statistics.items),
    [statistics.items],
  );
  const currentSelectedAgent =
    statistics.items.find((item) => item.agentUserId === selectedAgent?.agentUserId) ??
    selectedAgent;

  useEffect(() => {
    if (policy) {
      setForm(formFromPolicy(policy));
    }
  }, [policy]);

  useEffect(() => {
    if (!selectedAgent) {
      return;
    }
    void loadRecords(selectedAgent.agentUserId, {
      page: recordsPage,
      pageSize: recordsPageSize,
    }).catch(() => {
      Toast.error('返利明细加载失败');
    });
  }, [loadRecords, recordsPage, recordsPageSize, selectedAgent]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const submit = async () => {
    const saved = await save(policyPayload(form));
    setForm(formFromPolicy(saved));
    onDashboardRefresh();
    Toast.success('返利配置已保存');
  };

  const openAgentDetail = (agent: AgentRebateSummary) => {
    setSelectedAgent(agent);
    setRecordsPage(1);
    setWithdrawAmountYuan(
      agent.withdrawableRebateMinor > 0
        ? (agent.withdrawableRebateMinor / 100).toFixed(2)
        : '',
    );
    setWithdrawDescription(`代理返利提现处理：${agent.agentUsername}`);
  };

  const closeAgentDetail = () => {
    setSelectedAgent(null);
  };

  const submitWithdrawal = async () => {
    if (!currentSelectedAgent) {
      return;
    }
    const amountMinor = yuanInputToMinor(withdrawAmountYuan);
    if (amountMinor === null || amountMinor <= 0) {
      Toast.warning('请输入正确的返利提现金额');
      return;
    }
    if (amountMinor > currentSelectedAgent.withdrawableRebateMinor) {
      Toast.warning('提现金额不能超过当前可处理返利金额');
      return;
    }

    await withdraw(currentSelectedAgent.agentUserId, {
      amountMinor,
      description: withdrawDescription.trim() || '代理返利提现处理',
    });
    Toast.success('代理返利提现已处理');
    onDashboardRefresh();
    await loadRecords(currentSelectedAgent.agentUserId, {
      page: recordsPage,
      pageSize: recordsPageSize,
    });
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">返利管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            统计代理邀请返利、查看下级返利明细，并处理代理返利提现。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="返利接口错误" description={error} /> : null}

      <Tabs
        activeKey={activeTab}
        onChange={(key) => setActiveTab(String(key))}
      >
        <Tabs.TabPane itemKey="statistics" tab="返利统计">
          <section className="grid gap-3 pt-3 sm:grid-cols-2 xl:grid-cols-4">
            <MetricCard
              label="代理数量"
              trend="统计范围"
              value={`${statistics.totalCount}`}
            />
            <MetricCard
              label="本页总返利"
              trend="充值返利入账"
              value={formatMoney(statisticTotals.totalRebateMinor)}
            />
            <MetricCard
              label="本页待处理"
              trend="未提现返利"
              value={formatMoney(statisticTotals.pendingRebateMinor)}
            />
            <MetricCard
              label="本页可处理"
              trend="受账户余额限制"
              value={formatMoney(statisticTotals.withdrawableRebateMinor)}
            />
          </section>

          <Card className="mt-3 rounded-md border border-line">
            <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
              <div className="flex items-center gap-2">
                <h2 className="text-base font-semibold text-ink">代理返利统计</h2>
                <Tag color="purple">{statistics.totalCount} 个代理</Tag>
              </div>
              <PageControls
                loading={loading}
                page={statistics.page}
                pageSize={statisticsPageSize}
                totalCount={statistics.totalCount}
                totalPages={statistics.totalPages}
                onPageChange={setStatisticsPage}
                onPageSizeChange={(nextPageSize) => {
                  setStatisticsPage(1);
                  setStatisticsPageSize(nextPageSize);
                }}
              />
            </div>

            {loading ? (
              <div className="grid min-h-[320px] place-items-center">
                <Spin tip="正在加载返利统计" />
              </div>
            ) : statistics.items.length > 0 ? (
              <div className="overflow-x-auto">
                <table className="w-full min-w-[1180px] text-left text-sm">
                  <thead className="border-b border-line text-xs text-slate-500">
                    <tr>
                      <th className="py-2 pr-4 font-medium">代理</th>
                      <th className="py-2 pr-4 font-medium">邀请码</th>
                      <th className="py-2 pr-4 font-medium">直属下级</th>
                      <th className="py-2 pr-4 font-medium">返利笔数</th>
                      <th className="py-2 pr-4 font-medium">返利总额</th>
                      <th className="py-2 pr-4 font-medium">已提现</th>
                      <th className="py-2 pr-4 font-medium">待处理</th>
                      <th className="py-2 pr-4 font-medium">可处理</th>
                      <th className="py-2 pr-4 font-medium">最近返利</th>
                      <th className="py-2 pr-4 font-medium">操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {statistics.items.map((agent) => (
                      <tr key={agent.agentUserId} className="border-b border-slate-100">
                        <td className="py-3 pr-4">
                          <div className="font-semibold text-ink">{agent.agentUsername}</div>
                          <div className="mt-1 text-xs text-slate-400">
                            {agent.agentUserId}
                          </div>
                        </td>
                        <td className="py-3 pr-4">
                          <Tag color="teal">{agent.inviteCode}</Tag>
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {agent.directInviteeCount}
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {agent.rebateRecordCount}
                        </td>
                        <td className="py-3 pr-4 font-semibold text-emerald-700">
                          {formatMoney(agent.totalRebateMinor)}
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {formatMoney(agent.withdrawnRebateMinor)}
                        </td>
                        <td className="py-3 pr-4 font-semibold text-amber-700">
                          {formatMoney(agent.pendingRebateMinor)}
                        </td>
                        <td className="py-3 pr-4 font-semibold text-rose-700">
                          {formatMoney(agent.withdrawableRebateMinor)}
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {agent.lastRebateAt
                            ? formatDateTime(agent.lastRebateAt, agent.lastRebateAt)
                            : '-'}
                        </td>
                        <td className="py-3 pr-4">
                          <Button
                            icon={<Eye size={14} />}
                            size="small"
                            theme="solid"
                            onClick={() => openAgentDetail(agent)}
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
                暂无代理返利统计。代理下级充值并产生返利后会显示在这里。
              </div>
            )}
          </Card>
        </Tabs.TabPane>

        <Tabs.TabPane itemKey="policy" tab="策略配置">
          <section className="grid gap-3 pt-3 sm:grid-cols-2 xl:grid-cols-4">
            <MetricCard
              label="邀请入口"
              trend="当前开启"
              value={`${totals.enabledInviteEntries}`}
            />
            <MetricCard
              label="返利模式"
              trend="当前策略"
              value={rebateModeText(currentMode)}
            />
            <MetricCard
              label="默认充值返利"
              trend="basis points"
              value={policy ? percentText(policy.defaultRechargeRebateBasisPoints) : '-'}
            />
            <MetricCard
              label="注册邀请"
              trend="注册配置"
              value={registration?.agentInviteRequired ? '必填' : '非必填'}
            />
          </section>

          {loading ? (
            <Card className="mt-3 rounded-md border border-line">
              <div className="grid min-h-[320px] place-items-center">
                <Spin tip="正在加载返利配置" />
              </div>
            </Card>
          ) : (
            <section className="mt-3 grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
              <Card className="rounded-md border border-line">
                <div className="mb-4 flex items-center justify-between gap-3">
                  <div>
                    <h2 className="text-base font-semibold text-ink">策略维护</h2>
                    <p className="mt-1 text-sm text-slate-500">
                      充值成功后会按当前策略给符合条件的上级代理发放返利。
                    </p>
                  </div>
                  <Button
                    disabled={saving}
                    icon={<Save size={16} />}
                    loading={saving}
                    onClick={() => void submit()}
                    theme="solid"
                  >
                    保存配置
                  </Button>
                </div>

                <div className="grid gap-4 md:grid-cols-2">
                  <Field
                    description="代理用户可生成邀请关系。"
                    icon={<UserPlus size={16} />}
                    label="代理邀请"
                  >
                    <label className="inline-flex items-center gap-2 text-sm text-slate-700">
                      <input
                        checked={form.agentsCanInvite}
                        className="h-4 w-4 rounded border-line text-teal-600"
                        type="checkbox"
                        onChange={(event) =>
                          setForm((current) => ({
                            ...current,
                            agentsCanInvite: event.currentTarget.checked,
                          }))
                        }
                      />
                      {form.agentsCanInvite ? '已开启' : '已关闭'}
                    </label>
                  </Field>

                  <Field
                    description="普通用户邀请用于灰度或活动。"
                    icon={<Users size={16} />}
                    label="普通用户邀请"
                  >
                    <label className="inline-flex items-center gap-2 text-sm text-slate-700">
                      <input
                        checked={form.regularUsersCanInvite}
                        className="h-4 w-4 rounded border-line text-teal-600"
                        type="checkbox"
                        onChange={(event) =>
                          setForm((current) => ({
                            ...current,
                            regularUsersCanInvite: event.currentTarget.checked,
                          }))
                        }
                      />
                      {form.regularUsersCanInvite ? '已开启' : '已关闭'}
                    </label>
                  </Field>

                  <Field label="返利模式">
                    <Select
                      className="form-input"
                      value={form.rebateMode}
                      onChange={(value) =>
                        setForm((current) => ({
                          ...current,
                          rebateMode: value as RebateMode,
                        }))
                      }
                    >
                      <Select.Option value="immediate">立即返利</Select.Option>
                      <Select.Option value="rechargeTiered">充值阶梯返利</Select.Option>
                    </Select>
                  </Field>

                  <Field icon={<Percent size={16} />} label="默认充值返利比例">
                    <div className="flex items-center gap-2">
                      <Input
                        className="form-input"
                        min="0"
                        step="0.01"
                        type="number"
                        value={form.defaultRechargeRebatePercent}
                        onChange={(value) =>
                          setForm((current) => ({
                            ...current,
                            defaultRechargeRebatePercent: value,
                          }))
                        }
                      />
                      <span className="text-sm text-slate-500">%</span>
                    </div>
                  </Field>
                </div>
              </Card>

              <Card className="rounded-md border border-line">
                <div className="mb-4">
                  <h2 className="text-base font-semibold text-ink">当前策略</h2>
                  <p className="mt-1 text-sm text-slate-500">
                    保存成功后这里和系统概览使用同一份返利配置。
                  </p>
                </div>

                <div className="space-y-4 text-sm">
                  <PolicyRow
                    label="代理邀请"
                    value={
                      <Tag color={policy?.agentsCanInvite ? 'green' : 'grey'}>
                        {policy?.agentsCanInvite ? '开启' : '关闭'}
                      </Tag>
                    }
                  />
                  <PolicyRow
                    label="普通用户邀请"
                    value={
                      <Tag color={policy?.regularUsersCanInvite ? 'green' : 'grey'}>
                        {policy?.regularUsersCanInvite ? '开启' : '关闭'}
                      </Tag>
                    }
                  />
                  <PolicyRow
                    label="返利模式"
                    value={<Tag color="blue">{rebateModeText(currentMode)}</Tag>}
                  />
                  <PolicyRow
                    label="支持模式"
                    value={
                      <div className="flex flex-wrap justify-end gap-1">
                        {(policy?.supportedRebateModes ?? ['immediate', 'rechargeTiered']).map(
                          (mode) => (
                            <Tag key={mode} color="teal">
                              {rebateModeText(mode)}
                            </Tag>
                          ),
                        )}
                      </div>
                    }
                  />
                  <PolicyRow
                    label="默认比例"
                    value={policy ? percentText(policy.defaultRechargeRebateBasisPoints) : '-'}
                  />
                  <PolicyRow
                    label="代理邀请码"
                    value={registration?.agentInviteRequired ? '注册必填' : '注册非必填'}
                  />
                </div>
              </Card>
            </section>
          )}
        </Tabs.TabPane>
      </Tabs>

      <SideSheet
        aria-label="代理返利详情"
        title="代理返利详情"
        visible={Boolean(selectedAgent)}
        width={820}
        onCancel={closeAgentDetail}
      >
        {currentSelectedAgent ? (
          <div className="space-y-4">
            <section className="rounded-md bg-slate-50 p-4">
              <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                <div>
                  <h2 className="text-base font-semibold text-ink">
                    {currentSelectedAgent.agentUsername}
                  </h2>
                  <div className="mt-1 text-xs text-slate-400">
                    {currentSelectedAgent.agentUserId} · 邀请码 {currentSelectedAgent.inviteCode}
                  </div>
                </div>
                <Tag color={currentSelectedAgent.withdrawableRebateMinor > 0 ? 'red' : 'grey'}>
                  可处理 {formatMoney(currentSelectedAgent.withdrawableRebateMinor)}
                </Tag>
              </div>
              <div className="mt-4 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
                <SummaryAmount label="返利总额" value={currentSelectedAgent.totalRebateMinor} />
                <SummaryAmount label="已提现" value={currentSelectedAgent.withdrawnRebateMinor} />
                <SummaryAmount label="待处理" value={currentSelectedAgent.pendingRebateMinor} />
                <SummaryAmount
                  label="账户可用"
                  value={currentSelectedAgent.accountAvailableBalanceMinor}
                />
              </div>
            </section>

            <Card className="rounded-md border border-line">
              <div className="mb-4 flex items-start gap-3">
                <div className="grid h-10 w-10 place-items-center rounded-md bg-rose-50 text-rose-700">
                  <WalletCards size={18} />
                </div>
                <div>
                  <h2 className="text-base font-semibold text-ink">返利提现处理</h2>
                  <p className="mt-1 text-sm text-slate-500">
                    系统会从代理可用余额中扣减，并生成“代理返利提现”资金流水。
                  </p>
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-[180px_1fr_auto] md:items-end">
                <Field label="提现金额（元）">
                  <Input
                    className="form-input"
                    inputMode="decimal"
                    placeholder="例如 100.00"
                    value={withdrawAmountYuan}
                    onChange={setWithdrawAmountYuan}
                  />
                </Field>
                <Field label="处理说明">
                  <Input
                    className="form-input"
                    value={withdrawDescription}
                    onChange={setWithdrawDescription}
                  />
                </Field>
                <Button
                  disabled={saving || currentSelectedAgent.withdrawableRebateMinor <= 0}
                  loading={saving}
                  theme="solid"
                  type="danger"
                  onClick={() => void submitWithdrawal()}
                >
                  确认处理
                </Button>
              </div>
            </Card>

            <Card className="rounded-md border border-line">
              <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                <div className="flex items-center gap-2">
                  <h2 className="text-base font-semibold text-ink">下级返利记录</h2>
                  <Tag color="blue">{records.totalCount} 笔</Tag>
                </div>
                <PageControls
                  loading={recordsLoading}
                  page={records.page}
                  pageSize={recordsPageSize}
                  totalCount={records.totalCount}
                  totalPages={records.totalPages}
                  onPageChange={setRecordsPage}
                  onPageSizeChange={(nextPageSize) => {
                    setRecordsPage(1);
                    setRecordsPageSize(nextPageSize);
                  }}
                />
              </div>

              {recordsLoading ? (
                <div className="grid min-h-[220px] place-items-center">
                  <Spin tip="正在加载返利明细" />
                </div>
              ) : records.items.length > 0 ? (
                <div className="overflow-x-auto">
                  <table className="w-full min-w-[760px] text-left text-sm">
                    <thead className="border-b border-line text-xs text-slate-500">
                      <tr>
                        <th className="py-2 pr-4 font-medium">下级用户</th>
                        <th className="py-2 pr-4 font-medium">充值订单</th>
                        <th className="py-2 pr-4 font-medium">充值金额</th>
                        <th className="py-2 pr-4 font-medium">返利金额</th>
                        <th className="py-2 pr-4 font-medium">返利时间</th>
                      </tr>
                    </thead>
                    <tbody>
                      {records.items.map((record) => (
                        <tr key={record.ledgerEntryId} className="border-b border-slate-100">
                          <td className="py-3 pr-4">
                            <div className="font-medium text-ink">
                              {record.inviteeUsername ?? '未知下级'}
                            </div>
                            <div className="mt-1 text-xs text-slate-400">
                              {record.inviteeUserId ?? '-'}
                            </div>
                          </td>
                          <td className="py-3 pr-4 text-slate-600">
                            {record.rechargeOrderId ?? '-'}
                          </td>
                          <td className="py-3 pr-4 text-slate-600">
                            {record.rechargeAmountMinor === null
                              ? '-'
                              : formatMoney(record.rechargeAmountMinor)}
                          </td>
                          <td className="py-3 pr-4 font-semibold text-emerald-700">
                            {formatMoney(record.rebateAmountMinor)}
                          </td>
                          <td className="py-3 pr-4 text-slate-600">
                            {formatDateTime(record.createdAt, record.createdAt)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div className="rounded-md border border-line p-4 text-sm text-slate-500">
                  当前代理暂无下级返利记录。
                </div>
              )}
            </Card>
          </div>
        ) : null}
      </SideSheet>
    </div>
  );
}

interface FieldProps {
  children: ReactNode;
  description?: string;
  icon?: ReactNode;
  label: string;
}

function Field({ children, description, icon, label }: FieldProps) {
  return (
    <label className="block">
      <span className="mb-1 flex items-center gap-1 text-xs font-medium text-slate-500">
        {icon}
        {label}
      </span>
      {children}
      {description ? (
        <span className="mt-1 block text-xs text-slate-400">{description}</span>
      ) : null}
    </label>
  );
}

interface PolicyRowProps {
  label: string;
  value: ReactNode;
}

function PolicyRow({ label, value }: PolicyRowProps) {
  return (
    <div className="flex items-start justify-between gap-3 border-b border-line/70 pb-3 last:border-b-0 last:pb-0">
      <span className="text-slate-500">{label}</span>
      <span className="text-right font-medium text-ink">{value}</span>
    </div>
  );
}

interface SummaryAmountProps {
  label: string;
  value: number;
}

function SummaryAmount({ label, value }: SummaryAmountProps) {
  return (
    <div>
      <div className="text-xs text-slate-400">{label}</div>
      <div className="mt-1 font-semibold text-ink">{formatMoney(value)}</div>
    </div>
  );
}

function emptyForm(): RebateFormState {
  return {
    agentsCanInvite: true,
    defaultRechargeRebatePercent: '3.50',
    rebateMode: 'immediate',
    regularUsersCanInvite: false,
  };
}

function formFromPolicy(policy: InvitePolicySummary): RebateFormState {
  return {
    agentsCanInvite: policy.agentsCanInvite,
    defaultRechargeRebatePercent: (
      policy.defaultRechargeRebateBasisPoints / 100
    ).toFixed(2),
    rebateMode: policy.rebateMode,
    regularUsersCanInvite: policy.regularUsersCanInvite,
  };
}

function policyPayload(form: RebateFormState): InvitePolicyUpdateRequest {
  const percent = Number(form.defaultRechargeRebatePercent || '0');
  return {
    agentsCanInvite: form.agentsCanInvite,
    defaultRechargeRebateBasisPoints: Math.round(percent * 100),
    rebateMode: form.rebateMode,
    regularUsersCanInvite: form.regularUsersCanInvite,
  };
}

function policyTotals(policy: InvitePolicySummary | null) {
  return {
    enabledInviteEntries: [
      policy?.agentsCanInvite,
      policy?.regularUsersCanInvite,
    ].filter(Boolean).length,
  };
}

function visibleStatisticTotals(items: AgentRebateSummary[]) {
  return items.reduce(
    (totals, item) => ({
      pendingRebateMinor: totals.pendingRebateMinor + item.pendingRebateMinor,
      totalRebateMinor: totals.totalRebateMinor + item.totalRebateMinor,
      withdrawableRebateMinor:
        totals.withdrawableRebateMinor + item.withdrawableRebateMinor,
    }),
    {
      pendingRebateMinor: 0,
      totalRebateMinor: 0,
      withdrawableRebateMinor: 0,
    },
  );
}

function rebateModeText(mode: RebateMode) {
  return mode === 'immediate' ? '立即返利' : '充值阶梯返利';
}

function percentText(basisPoints: number) {
  return `${(basisPoints / 100).toFixed(2)}%`;
}
