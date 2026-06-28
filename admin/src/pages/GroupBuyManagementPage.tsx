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
  TextArea,
  Toast,
} from '@douyinfe/semi-ui';
import { Eye, Plus, RefreshCcw, Save, Search, Trash2, Users, X } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { PageControls } from '../components/PageControls';
import { useGroupBuyPlans } from '../hooks/useGroupBuyPlans';
import type {
  AddGroupBuyParticipantRequest,
  CreateGroupBuyPlanRequest,
  GroupBuyFormationStatus,
  GroupBuyPlan,
  GroupBuyPlanStatus,
  GroupBuyPlanSummary,
  UpdateGroupBuyPlanRequest,
} from '../types/groupBuy';
import type { PlayRuleCode } from '../types/playRules';
import { formatMoney } from '../utils/format';
import { yuanInputToMinor } from '../utils/moneyInput';

interface GroupBuyManagementPageProps {
  onDashboardRefresh: () => void;
}

interface CreateFormState {
  id: string;
  initiatorAmountYuan: string;
  initiatorUserId: string;
  issue: string;
  lotteryId: string;
  note: string;
  numbers: string;
  ruleCode: string;
  title: string;
  totalAmountYuan: string;
}

interface UpdateFormState {
  note: string;
  status: GroupBuyPlanStatus;
}

interface ParticipantFormState {
  amountYuan: string;
  id: string;
  note: string;
  userId: string;
}

const ROBOT_GROUP_BUY_USER_ID = 'U90001';
type GroupBuyFormationFilter = 'all' | GroupBuyFormationStatus;

export function GroupBuyManagementPage({
  onDashboardRefresh,
}: GroupBuyManagementPageProps) {
  const [includeRobotData, setIncludeRobotData] = useState(false);
  const [formationFilter, setFormationFilter] =
    useState<GroupBuyFormationFilter>('all');
  const [planIdInput, setPlanIdInput] = useState('');
  const [planIdFilter, setPlanIdFilter] = useState('');
  const [planPageNumber, setPlanPageNumber] = useState(1);
  const [planPageSize, setPlanPageSize] = useState(10);
  const [createSheetVisible, setCreateSheetVisible] = useState(false);
  const [detailSheetVisible, setDetailSheetVisible] = useState(false);
  const [detailPlanId, setDetailPlanId] = useState('');
  const {
    addParticipant,
    clearRecords,
    clearRobotRecords,
    create,
    deleteRobotPlan,
    drawIssues,
    error,
    loadPlan,
    loading,
    lotteries,
    planPage,
    plans,
    refresh,
    saving,
    selectedPlan,
    update,
    users,
  } = useGroupBuyPlans({
    planQuery: {
      formationStatus: formationFilter === 'all' ? undefined : formationFilter,
      includeRobotData,
      page: planPageNumber,
      pageSize: planPageSize,
      planId: planIdFilter || undefined,
    },
  });
  const eligibleLotteries = useMemo(
    () => lotteries.filter((lottery) => lottery.groupBuy.enabled),
    [lotteries],
  );
  const [createForm, setCreateForm] = useState<CreateFormState>(() =>
    emptyCreateForm(),
  );
  const [updateForm, setUpdateForm] = useState<UpdateFormState>(() =>
    emptyUpdateForm(),
  );
  const [participantForm, setParticipantForm] = useState<ParticipantFormState>(
    () => emptyParticipantForm(),
  );
  const selectedLottery = useMemo(
    () => eligibleLotteries.find((lottery) => lottery.id === createForm.lotteryId),
    [createForm.lotteryId, eligibleLotteries],
  );
  const availableIssues = useMemo(
    () =>
      drawIssues.filter(
        (issue) => issue.lotteryId === createForm.lotteryId && issue.status === 'open',
      ),
    [createForm.lotteryId, drawIssues],
  );
  const availablePlayConfigs = useMemo(
    () => selectedLottery?.playConfigs.filter((config) => config.enabled) ?? [],
    [selectedLottery],
  );
  const detailPlan = selectedPlan?.id === detailPlanId ? selectedPlan : null;

  useEffect(() => {
    if (!createForm.lotteryId && eligibleLotteries[0]) {
      setCreateForm((current) => ({
        ...current,
        lotteryId: eligibleLotteries[0].id,
      }));
    }
  }, [createForm.lotteryId, eligibleLotteries]);

  useEffect(() => {
    if (availableIssues.length === 0) {
      if (createForm.issue) {
        setCreateForm((current) => ({ ...current, issue: '' }));
      }
      return;
    }
    if (!availableIssues.some((issue) => issue.issue === createForm.issue)) {
      setCreateForm((current) => ({
        ...current,
        issue: availableIssues[0].issue,
      }));
    }
  }, [availableIssues, createForm.issue]);

  useEffect(() => {
    if (availablePlayConfigs.length === 0) {
      if (createForm.ruleCode) {
        setCreateForm((current) => ({ ...current, ruleCode: '' }));
      }
      return;
    }
    if (!availablePlayConfigs.some((config) => config.ruleCode === createForm.ruleCode)) {
      setCreateForm((current) => ({
        ...current,
        ruleCode: availablePlayConfigs[0].ruleCode,
      }));
    }
  }, [availablePlayConfigs, createForm.ruleCode]);

  useEffect(() => {
    if (!createForm.initiatorUserId && users[0]) {
      setCreateForm((current) => ({
        ...current,
        initiatorUserId: users[0].id,
      }));
    }
    if (!participantForm.userId && users[0]) {
      setParticipantForm((current) => ({
        ...current,
        userId: users[0].id,
      }));
    }
  }, [createForm.initiatorUserId, participantForm.userId, users]);

  useEffect(() => {
    if (selectedPlan) {
      setUpdateForm(formFromPlan(selectedPlan));
      setParticipantForm((current) => ({
        ...current,
        id: nextParticipantId(selectedPlan),
      }));
    }
  }, [selectedPlan]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const applyPlanIdFilter = () => {
    setPlanIdFilter(planIdInput.trim());
    setPlanPageNumber(1);
  };

  const clearPlanIdFilter = () => {
    setPlanIdInput('');
    setPlanIdFilter('');
    setPlanPageNumber(1);
  };

  const openDetailSheet = async (planId: string) => {
    setDetailPlanId(planId);
    setDetailSheetVisible(true);
    try {
      await loadPlan(planId);
    } catch {
      // 错误文案已由 hook 写入 error，这里只避免异步点击产生未处理异常。
    }
  };

  const submitCreate = async () => {
    const totalAmountMinor = positiveYuanToMinor(
      createForm.totalAmountYuan,
      '计划总金额',
    );
    const initiatorAmountMinor = positiveYuanToMinor(
      createForm.initiatorAmountYuan,
      '发起人认购',
    );
    if (totalAmountMinor === null || initiatorAmountMinor === null) {
      return;
    }
    const created = await create(
      createPayload(createForm, totalAmountMinor, initiatorAmountMinor),
    );
    setCreateForm(
      emptyCreateForm(eligibleLotteries[0]?.id, users[0]?.id),
    );
    setParticipantForm(emptyParticipantForm(created.id, users[0]?.id));
    setCreateSheetVisible(false);
    onDashboardRefresh();
  };

  const submitUpdate = async () => {
    if (!selectedPlan) {
      return;
    }
    await update(selectedPlan.id, updatePayload(updateForm));
    onDashboardRefresh();
  };

  const submitParticipant = async () => {
    if (!selectedPlan) {
      return;
    }
    const amountMinor = positiveYuanToMinor(participantForm.amountYuan, '参与金额');
    if (amountMinor === null) {
      return;
    }
    const updated = await addParticipant(
      selectedPlan.id,
      participantPayload(participantForm, amountMinor),
    );
    setParticipantForm(emptyParticipantForm(updated.id, users[0]?.id));
    onDashboardRefresh();
  };

  const deleteRobotPlanRecord = async (plan: GroupBuyPlanSummary) => {
    if (!isRobotGroupBuyPlan(plan)) {
      Toast.warning('只能删除机器人发起的合买计划');
      return;
    }
    if (
      !window.confirm(
        `确定删除机器人合买计划【${plan.id}】吗？仅机器人发起且没有真实用户认购的待开奖单据会被删除。`,
      )
    ) {
      return;
    }
    try {
      await deleteRobotPlan(plan.id);
      if (detailPlanId === plan.id) {
        setDetailPlanId('');
        setDetailSheetVisible(false);
      }
      Toast.success('机器人合买计划已删除');
      onDashboardRefresh();
    } catch {
      Toast.error('机器人合买计划删除失败，请查看接口错误提示');
    }
  };

  const clearGroupBuyPlanRecords = async () => {
    if (
      !window.confirm(
        '确定一键清除合买计划列表吗？系统只会删除已取消或已结算的历史计划；草稿、进行中或已满单未结算计划会自动保留，且不会回滚资金流水或订单。',
      )
    ) {
      return;
    }
    try {
      const result = await clearRecords();
      setDetailPlanId('');
      setDetailSheetVisible(false);
      setPlanPageNumber(1);
      onDashboardRefresh();
      Toast.success(
        result.deletedCount > 0
          ? `已清除 ${result.deletedCount} 条已结束合买计划，未结算计划已保留`
          : '没有可清理的已结束合买计划，未结算计划已保留',
      );
    } catch {
      Toast.error('合买计划列表清除失败，请查看接口错误提示');
    }
  };

  const clearRobotGroupBuyPlanRecords = async () => {
    if (
      !window.confirm(
        '确定一键清理所有机器人合买订单吗？系统会删除纯机器人合买计划，以及这些计划关联的机器人投注订单；未成单、待开奖和已结算机器人记录都会清理，包含真实用户认购的计划会保留。',
      )
    ) {
      return;
    }
    try {
      const result = await clearRobotRecords();
      setDetailPlanId('');
      setDetailSheetVisible(false);
      setPlanPageNumber(1);
      onDashboardRefresh();
      Toast.success(
        result.deletedCount > 0 || result.deletedOrderCount > 0
          ? `已清理 ${result.deletedCount} 条机器人合买计划，关联机器人订单 ${result.deletedOrderCount} 笔`
          : '没有可清理的纯机器人合买订单',
      );
    } catch {
      Toast.error('机器人合买订单清理失败，请查看接口错误提示');
    }
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">合买管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            维护合买计划、认购进度、参与记录和计划状态。
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <label className="inline-flex items-center gap-2 rounded-md border border-line px-3 py-2 text-sm text-slate-600">
            <Switch
              checked={includeRobotData}
              onChange={(checked) => {
                setIncludeRobotData(checked);
                setPlanPageNumber(1);
              }}
            />
            <span>显示机器人数据</span>
          </label>
          <Button
            icon={<Plus size={16} />}
            theme="solid"
            onClick={() => setCreateSheetVisible(true)}
          >
            新增合买计划
          </Button>
          <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
            刷新
          </Button>
          <Button
            disabled={saving || loading || planPage.totalCount === 0}
            icon={<Trash2 size={16} />}
            theme="solid"
            type="danger"
            onClick={() => void clearGroupBuyPlanRecords()}
          >
            一键清除合买计划列表
          </Button>
          <Button
            disabled={saving || loading}
            icon={<Trash2 size={16} />}
            theme="light"
            type="danger"
            onClick={() => void clearRobotGroupBuyPlanRecords()}
          >
            清理机器人订单
          </Button>
        </div>
      </section>

      {error ? <Banner type="danger" title="合买接口错误" description={error} /> : null}

      {loading ? (
        <Card className="rounded-md border border-line">
          <div className="grid min-h-[320px] place-items-center">
            <Spin tip="正在加载合买计划" />
          </div>
        </Card>
      ) : (
        <section className="space-y-4">
          <Card className="rounded-md border border-line">
              <div className="mb-4 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                <div className="flex items-center gap-2">
                  <h2 className="text-base font-semibold text-ink">合买计划列表</h2>
                  <Tag color="teal">{planPage.totalCount} 个计划</Tag>
                </div>
                <div className="flex flex-col gap-2 md:flex-row md:items-center">
                  <label className="flex items-center gap-2 text-sm text-slate-600">
                    <span className="shrink-0">计划 ID</span>
                    <Input
                      className="w-48"
                      placeholder="输入完整计划 ID"
                      value={planIdInput}
                      onChange={setPlanIdInput}
                      onEnterPress={applyPlanIdFilter}
                    />
                  </label>
                  <div className="flex items-center gap-2">
                    <Button
                      icon={<Search size={14} />}
                      size="small"
                      theme="solid"
                      onClick={applyPlanIdFilter}
                    >
                      查询
                    </Button>
                    <Button
                      disabled={!planIdFilter && !planIdInput}
                      icon={<X size={14} />}
                      size="small"
                      onClick={clearPlanIdFilter}
                    >
                      清空
                    </Button>
                  </div>
                  <label className="flex items-center gap-2 text-sm text-slate-600">
                    <span className="shrink-0">成单状态</span>
                    <Select
                      className="w-36"
                      value={formationFilter}
                      onChange={(value) => {
                        setFormationFilter(value as GroupBuyFormationFilter);
                        setPlanPageNumber(1);
                      }}
                    >
                      <Select.Option value="all">全部</Select.Option>
                      <Select.Option value="formed">已成单</Select.Option>
                      <Select.Option value="unformed">未成单</Select.Option>
                    </Select>
                  </label>
                  <PageControls
                    loading={loading}
                    page={planPage.page}
                    pageSize={planPageSize}
                    totalCount={planPage.totalCount}
                    totalPages={planPage.totalPages}
                    onPageChange={setPlanPageNumber}
                    onPageSizeChange={(nextPageSize) => {
                      setPlanPageNumber(1);
                      setPlanPageSize(nextPageSize);
                    }}
                  />
                </div>
              </div>
              {plans.length > 0 ? (
                <div className="overflow-x-auto">
                  <table className="w-full min-w-[1180px] text-left text-sm">
                    <thead className="border-b border-line text-xs text-slate-500">
                      <tr>
                        <th className="py-2 pr-4 font-medium">计划</th>
                        <th className="py-2 pr-4 font-medium">彩种/期号</th>
                        <th className="py-2 pr-4 font-medium">成单</th>
                        <th className="py-2 pr-4 font-medium">中奖状态</th>
                        <th className="py-2 pr-4 font-medium">创建时间</th>
                        <th className="py-2 pr-4 font-medium">进度</th>
                        <th className="py-2 pr-4 font-medium">状态</th>
                        <th className="py-2 pr-4 font-medium">操作</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-line">
                      {plans.map((plan) => (
                        <tr
                          key={plan.id}
                          className={detailPlanId === plan.id ? 'bg-teal-50/60' : ''}
                        >
                          <td className="py-3 pr-4">
                            <div className="font-semibold text-ink">{plan.id}</div>
                            <div className="mt-1 text-xs text-slate-400">
                              发起人：{plan.initiatorUsername}
                              {isRobotGroupBuyPlan(plan) ? (
                                <Tag className="ml-2" color="purple">
                                  机器人
                                </Tag>
                              ) : null}
                            </div>
                          </td>
                          <td className="py-3 pr-4 text-slate-600">
                            <div>{plan.lotteryName}</div>
                            <div className="mt-1 text-xs text-slate-400">
                              第 {plan.issue || '-'} 期
                            </div>
                          </td>
                          <td className="py-3 pr-4 text-slate-600">
                            {plan.orderId ? (
                              <Tag color="green">{plan.orderId}</Tag>
                            ) : (
                              <Tag color="grey">未成单</Tag>
                            )}
                          </td>
                          <td className="py-3 pr-4 text-slate-600">
                            <GroupBuyOrderStatusView plan={plan} />
                          </td>
                          <td className="py-3 pr-4 text-xs text-slate-500">
                            {plan.createdAt || '-'}
                          </td>
                          <td className="py-3 pr-4">
                            <div className="font-medium text-ink">
                              {progressPercent(plan)}%
                            </div>
                            <div className="mt-1 text-xs text-slate-500">
                              {formatMoney(plan.filledAmountMinor)} /{' '}
                              {formatMoney(plan.totalAmountMinor)}
                            </div>
                          </td>
                          <td className="py-3 pr-4">
                            <Tag color={statusColor(plan.status)}>
                              {statusText(plan.status)}
                            </Tag>
                          </td>
                          <td className="py-3 pr-4">
                            <div className="flex flex-wrap gap-2">
                              <Button
                                disabled={saving}
                                icon={<Eye size={14} />}
                                loading={saving && detailPlanId === plan.id}
                                size="small"
                                onClick={() => void openDetailSheet(plan.id)}
                              >
                                查看详情
                              </Button>
                              {includeRobotData && isRobotGroupBuyPlan(plan) ? (
                                <Button
                                  disabled={saving}
                                  icon={<Trash2 size={14} />}
                                  size="small"
                                  theme="light"
                                  type="danger"
                                  onClick={() => void deleteRobotPlanRecord(plan)}
                                >
                                  删除
                                </Button>
                              ) : null}
                            </div>
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <div className="rounded-md border border-line p-4 text-sm text-slate-500">
                  暂无合买计划。
                </div>
              )}
          </Card>

          <SideSheet
              aria-label="新增合买计划"
              title="新增合买计划"
              visible={createSheetVisible}
              width={600}
              onCancel={() => setCreateSheetVisible(false)}
          >
              <div className="mb-4">
                <h2 className="text-base font-semibold text-ink">新增合买计划</h2>
                <p className="mt-1 text-sm text-slate-500">
                  选择开放期号和玩法后创建合买计划，创建成功后会自动关闭抽屉并刷新列表。
                </p>
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <Field label="计划 ID">
                  <Input
                    className="form-input"
                    value={createForm.id}
                    onChange={(value) =>
                      setCreateFormValue(setCreateForm, 'id', value)
                    }
                  />
                </Field>
                <Field label="彩种">
                  <Select
                    className="form-input"
                    value={createForm.lotteryId}
                    onChange={(value) =>
                      setCreateFormValue(setCreateForm, 'lotteryId', String(value ?? ''))
                    }
                  >
                    {eligibleLotteries.map((lottery) => (
                      <Select.Option key={lottery.id} value={lottery.id}>
                        {lottery.name}
                      </Select.Option>
                    ))}
                  </Select>
                </Field>
                <Field label="期号">
                  <Select
                    className="form-input"
                    disabled={availableIssues.length === 0}
                    value={createForm.issue}
                    onChange={(value) =>
                      setCreateFormValue(setCreateForm, 'issue', String(value ?? ''))
                    }
                  >
                    {availableIssues.map((issue) => (
                      <Select.Option key={issue.id} value={issue.issue}>
                        第 {issue.issue} 期
                      </Select.Option>
                    ))}
                  </Select>
                </Field>
                <Field label="玩法">
                  <Select
                    className="form-input"
                    disabled={availablePlayConfigs.length === 0}
                    value={createForm.ruleCode}
                    onChange={(value) =>
                      setCreateFormValue(setCreateForm, 'ruleCode', String(value ?? ''))
                    }
                  >
                    {availablePlayConfigs.map((config) => (
                      <Select.Option key={config.ruleCode} value={config.ruleCode}>
                        {formatRuleCode(config.ruleCode)}
                      </Select.Option>
                    ))}
                  </Select>
                </Field>
                <Field label="发起人">
                  <Select
                    className="form-input"
                    value={createForm.initiatorUserId}
                    onChange={(value) =>
                      setCreateFormValue(
                        setCreateForm,
                        'initiatorUserId',
                        String(value ?? ''),
                      )
                    }
                  >
                    {users.map((user) => (
                      <Select.Option key={user.id} value={user.id}>
                        {user.username}（{user.id}）
                      </Select.Option>
                    ))}
                  </Select>
                </Field>
                <Field label="计划总金额（元）">
                  <Input
                    className="form-input"
                    inputMode="decimal"
                    placeholder="例如 1000 或 1000.00"
                    value={createForm.totalAmountYuan}
                    onChange={(value) =>
                      setCreateFormValue(
                        setCreateForm,
                        'totalAmountYuan',
                        value,
                      )
                    }
                  />
                </Field>
                <Field label="发起人认购（元）">
                  <Input
                    className="form-input"
                    inputMode="decimal"
                    placeholder="例如 100 或 100.00"
                    value={createForm.initiatorAmountYuan}
                    onChange={(value) =>
                      setCreateFormValue(
                        setCreateForm,
                        'initiatorAmountYuan',
                        value,
                      )
                    }
                  />
                </Field>
                <Field label="标题">
                  <Input
                    className="form-input"
                    value={createForm.title}
                    onChange={(value) =>
                      setCreateFormValue(setCreateForm, 'title', value)
                    }
                  />
                </Field>
                <Field label="备注">
                  <Input
                    className="form-input"
                    value={createForm.note}
                    onChange={(value) =>
                      setCreateFormValue(setCreateForm, 'note', value)
                    }
                  />
                </Field>
                <div className="md:col-span-2">
                  <Field label="投注内容">
                    <TextArea
                      autosize
                      className="form-input"
                      placeholder="直选：1|2|3；组合：1,2,3；胆拖：1|2,3,4；大小单双：tens:big|ones:odd"
                      value={createForm.numbers}
                      onChange={(value) =>
                        setCreateFormValue(setCreateForm, 'numbers', value)
                      }
                    />
                  </Field>
                </div>
              </div>
              {selectedLottery ? (
                <div className="mt-3 flex flex-wrap gap-2 text-xs">
                  <Tag color="cyan">
                    每份 {formatMoney(selectedLottery.groupBuy.minShareAmountMinor)}
                  </Tag>
                  <Tag color="green">
                    发起人最低 {selectedLottery.groupBuy.initiatorMinPercent}%
                  </Tag>
                  <Tag color="blue">
                    参与最低{' '}
                    {formatMoney(selectedLottery.groupBuy.participantMinAmountMinor)}
                  </Tag>
                </div>
              ) : null}
              <div className="mt-4 flex justify-end">
                <Button
                  disabled={saving || eligibleLotteries.length === 0 || users.length === 0}
                  icon={<Plus size={16} />}
                  loading={saving}
                  theme="solid"
                  onClick={() => void submitCreate()}
                >
                  创建合买计划
                </Button>
              </div>
          </SideSheet>

          <SideSheet
              aria-label="合买计划详情"
              title="合买计划详情"
              visible={detailSheetVisible}
              width="80%"
              onCancel={() => setDetailSheetVisible(false)}
          >
              {detailPlan ? (
                <div className="space-y-5">
                  <section className="rounded-md border border-line bg-slate-50 p-4">
                    <div className="mb-4 flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
                      <div>
                        <h2 className="text-base font-semibold text-ink">计划详情</h2>
                        <p className="mt-1 break-all text-sm text-slate-500">
                          计划编号：{detailPlan.id}
                        </p>
                      </div>
                      <Tag color={statusColor(detailPlan.status)}>
                        {statusText(detailPlan.status)}
                      </Tag>
                    </div>

                    <div className="grid gap-3 md:grid-cols-3">
                      <InfoLine label="彩种" value={detailPlan.lotteryName} />
                      <InfoLine label="期号" value={detailPlan.issue || '-'} />
                      <InfoLine
                        label="玩法"
                        value={formatRuleCode(detailPlan.ruleCode as PlayRuleCode)}
                      />
                      <InfoLine label="订单号" value={detailPlan.orderId ?? '未成单'} />
                      <InfoLine
                        label="中奖状态"
                        value={groupBuyOrderStatusText(detailPlan)}
                      />
                      <InfoLine
                        label="开奖号码"
                        value={detailPlan.orderDrawNumber ?? '-'}
                      />
                      <InfoLine
                        label="派奖金额"
                        value={
                          detailPlan.orderPayoutMinor == null
                            ? '-'
                            : formatMoney(detailPlan.orderPayoutMinor)
                        }
                      />
                      <InfoLine label="发起人" value={detailPlan.initiatorUsername} />
                      <InfoLine
                        label="计划总额"
                        value={formatMoney(detailPlan.totalAmountMinor)}
                      />
                      <InfoLine
                        label="已认购"
                        value={formatMoney(detailPlan.filledAmountMinor)}
                      />
                      <InfoLine label="总份数" value={`${detailPlan.shareCount} 份`} />
                      <InfoLine
                        label="每份金额"
                        value={formatMoney(detailPlan.minShareAmountMinor)}
                      />
                    </div>

                    <div className="mt-4">
                      <div className="mb-1 flex items-center justify-between text-xs text-slate-500">
                        <span>合买进度</span>
                        <span>{progressPercent(detailPlan)}%</span>
                      </div>
                      <div className="h-2 overflow-hidden rounded-full bg-white ring-1 ring-line">
                        <div
                          className="h-full rounded-full bg-teal-500"
                          style={{ width: `${progressPercent(detailPlan)}%` }}
                        />
                      </div>
                    </div>

                    <div className="mt-4 rounded-md border border-line bg-white px-3 py-2">
                      <div className="text-xs text-slate-500">投注内容</div>
                      <div className="mt-1 whitespace-pre-wrap break-all text-sm font-medium text-ink">
                        {detailPlan.numbers || '-'}
                      </div>
                    </div>

                    <div className="mt-4 grid gap-3 sm:grid-cols-2">
                      <Field label="状态">
                        <Select
                          className="form-input"
                          value={updateForm.status}
                          onChange={(value) =>
                            setUpdateForm((current) => ({
                              ...current,
                              status: (value as GroupBuyPlanStatus) || 'draft',
                            }))
                          }
                        >
                          <Select.Option value="draft">草稿</Select.Option>
                          <Select.Option value="open">进行中</Select.Option>
                          <Select.Option value="filled">已满单</Select.Option>
                          <Select.Option value="cancelled">已取消</Select.Option>
                          <Select.Option value="settled">已结算</Select.Option>
                        </Select>
                      </Field>
                      <Field label="备注">
                        <Input
                          className="form-input"
                          value={updateForm.note}
                          onChange={(value) =>
                            setUpdateForm((current) => ({
                              ...current,
                              note: value,
                            }))
                          }
                        />
                      </Field>
                    </div>
                    <div className="mt-4 flex justify-end">
                      {includeRobotData && isRobotGroupBuyPlan(detailPlan) ? (
                        <Button
                          className="mr-2"
                          disabled={saving}
                          icon={<Trash2 size={16} />}
                          loading={saving}
                          theme="light"
                          type="danger"
                          onClick={() => void deleteRobotPlanRecord(detailPlan)}
                        >
                          删除机器人计划
                        </Button>
                      ) : null}
                      <Button
                        disabled={saving}
                        icon={<Save size={16} />}
                        loading={saving}
                        theme="solid"
                        onClick={() => void submitUpdate()}
                      >
                        保存计划状态
                      </Button>
                    </div>
                  </section>

                  <section className="rounded-md border border-line p-4">
                    <div className="mb-4 flex items-center gap-2">
                      <Users size={17} />
                      <h2 className="text-base font-semibold text-ink">参与记录</h2>
                    </div>
                    <div className="space-y-4">
                      <div className="overflow-x-auto">
                        <table className="w-full min-w-[680px] text-left text-sm">
                          <thead className="border-b border-line text-xs text-slate-500">
                            <tr>
                              <th className="py-2 pr-4 font-medium">用户</th>
                              <th className="py-2 pr-4 font-medium">金额</th>
                              <th className="py-2 pr-4 font-medium">份数</th>
                              <th className="py-2 pr-4 font-medium">创建时间</th>
                              <th className="py-2 pr-4 font-medium">备注</th>
                            </tr>
                          </thead>
                          <tbody className="divide-y divide-line">
                            {detailPlan.participants.map((participant) => (
                              <tr key={participant.id}>
                                <td className="py-3 pr-4">
                                  <div className="font-medium text-ink">
                                    {participant.username}
                                  </div>
                                  <div className="mt-1 text-xs text-slate-400">
                                    {participant.userId}
                                  </div>
                                </td>
                                <td className="py-3 pr-4 text-slate-600">
                                  {formatMoney(participant.amountMinor)}
                                </td>
                                <td className="py-3 pr-4 text-slate-600">
                                  {participant.shareCount}
                                </td>
                                <td className="py-3 pr-4 text-slate-500">
                                  {participant.createdAt || '-'}
                                </td>
                                <td className="py-3 pr-4 text-slate-500">
                                  {participant.note || '-'}
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>

                      <div className="grid gap-3 sm:grid-cols-2">
                        <Field label="参与记录 ID">
                          <Input
                            className="form-input"
                            value={participantForm.id}
                            onChange={(value) =>
                              setParticipantFormValue(
                                setParticipantForm,
                                'id',
                                value,
                              )
                            }
                          />
                        </Field>
                        <Field label="参与用户">
                          <Select
                            className="form-input"
                            value={participantForm.userId}
                            onChange={(value) =>
                              setParticipantFormValue(
                                setParticipantForm,
                                'userId',
                                String(value ?? ''),
                              )
                            }
                          >
                            {users.map((user) => (
                              <Select.Option key={user.id} value={user.id}>
                                {user.username}（{user.id}）
                              </Select.Option>
                            ))}
                          </Select>
                        </Field>
                        <Field label="参与金额（元）">
                          <Input
                            className="form-input"
                            inputMode="decimal"
                            placeholder="例如 10 或 10.00"
                            value={participantForm.amountYuan}
                            onChange={(value) =>
                              setParticipantFormValue(
                                setParticipantForm,
                                'amountYuan',
                                value,
                              )
                            }
                          />
                        </Field>
                        <Field label="备注">
                          <Input
                            className="form-input"
                            value={participantForm.note}
                            onChange={(value) =>
                              setParticipantFormValue(
                                setParticipantForm,
                                'note',
                                value,
                              )
                            }
                          />
                        </Field>
                      </div>
                      <div className="flex justify-end">
                        <Button
                          disabled={
                            saving ||
                            !['draft', 'open'].includes(detailPlan.status) ||
                            users.length === 0
                          }
                          icon={<Plus size={16} />}
                          loading={saving}
                          theme="solid"
                          onClick={() => void submitParticipant()}
                        >
                          添加参与记录
                        </Button>
                      </div>
                    </div>
                  </section>
                </div>
              ) : (
                <div className="grid min-h-[320px] place-items-center">
                  {error ? (
                    <Banner type="danger" title="计划详情加载失败" description={error} />
                  ) : (
                    <Spin tip="正在加载合买计划详情" />
                  )}
                </div>
              )}
          </SideSheet>
        </section>
      )}
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
  value: string;
}

function InfoLine({ label, value }: InfoLineProps) {
  return (
    <div className="rounded-md border border-line px-3 py-2">
      <div className="text-xs text-slate-500">{label}</div>
      <div className="mt-1 text-sm font-medium text-ink">{value}</div>
    </div>
  );
}

function emptyCreateForm(
  lotteryId = '',
  initiatorUserId = '',
): CreateFormState {
  return {
    id: 'G-NEW-001',
    initiatorAmountYuan: '100.00',
    initiatorUserId,
    issue: '',
    lotteryId,
    note: '后台创建合买计划',
    numbers: '1|2|3',
    ruleCode: '',
    title: '后台合买计划',
    totalAmountYuan: '1000.00',
  };
}

function emptyUpdateForm(): UpdateFormState {
  return {
    note: '',
    status: 'open',
  };
}

function emptyParticipantForm(planId = 'G-NEW-001', userId = ''): ParticipantFormState {
  return {
    amountYuan: '10.00',
    id: `${planId}-P002`,
    note: '后台添加参与记录',
    userId,
  };
}

function formFromPlan(plan: GroupBuyPlan): UpdateFormState {
  return {
    note: plan.note,
    status: plan.status,
  };
}

function createPayload(
  form: CreateFormState,
  totalAmountMinor: number,
  initiatorAmountMinor: number,
): CreateGroupBuyPlanRequest {
  return {
    id: form.id.trim(),
    initiatorAmountMinor,
    initiatorUserId: form.initiatorUserId.trim(),
    issue: form.issue.trim(),
    lotteryId: form.lotteryId.trim(),
    note: form.note.trim(),
    numbers: form.numbers.trim(),
    ruleCode: form.ruleCode.trim(),
    title: form.title.trim(),
    totalAmountMinor,
  };
}

function updatePayload(form: UpdateFormState): UpdateGroupBuyPlanRequest {
  return {
    note: form.note.trim(),
    status: form.status,
  };
}

function participantPayload(
  form: ParticipantFormState,
  amountMinor: number,
): AddGroupBuyParticipantRequest {
  return {
    amountMinor,
    id: form.id.trim(),
    note: form.note.trim(),
    userId: form.userId.trim(),
  };
}

function progressPercent(plan: GroupBuyPlanSummary) {
  if (plan.totalAmountMinor <= 0) {
    return 0;
  }

  return Math.min(
    100,
    Math.round((plan.filledAmountMinor / plan.totalAmountMinor) * 100),
  );
}

function GroupBuyOrderStatusView({ plan }: { plan: GroupBuyPlanSummary }) {
  const color = groupBuyOrderStatusColor(plan);
  const payoutText =
    plan.orderPayoutMinor != null && plan.orderPayoutMinor > 0
      ? formatMoney(plan.orderPayoutMinor)
      : '';

  return (
    <div className="space-y-1">
      <Tag color={color}>{groupBuyOrderStatusText(plan)}</Tag>
      {payoutText ? (
        <div className="text-xs font-medium text-emerald-600">派奖 {payoutText}</div>
      ) : null}
      {plan.orderDrawNumber ? (
        <div className="text-xs text-slate-400">开奖号码 {plan.orderDrawNumber}</div>
      ) : null}
    </div>
  );
}

function groupBuyOrderStatusText(plan: GroupBuyPlanSummary) {
  if (!plan.orderId) {
    return '未成单';
  }
  if (!plan.orderStatus) {
    return '订单缺失';
  }

  const mapping = {
    cancelled: '已取消',
    lost: '未中奖',
    pendingDraw: '待开奖',
    won: '已中奖',
  } as const;

  return mapping[plan.orderStatus];
}

function groupBuyOrderStatusColor(plan: GroupBuyPlanSummary) {
  if (!plan.orderId || !plan.orderStatus) {
    return 'grey';
  }

  const mapping = {
    cancelled: 'grey',
    lost: 'red',
    pendingDraw: 'blue',
    won: 'green',
  } as const;

  return mapping[plan.orderStatus];
}

function isRobotGroupBuyPlan(plan: GroupBuyPlanSummary) {
  return plan.initiatorUserId === ROBOT_GROUP_BUY_USER_ID;
}

function nextParticipantId(plan: GroupBuyPlan) {
  return `${plan.id}-P${String(plan.participants.length + 1).padStart(3, '0')}`;
}

function positiveYuanToMinor(value: string, label: string) {
  const amountMinor = yuanInputToMinor(value);
  if (amountMinor === null || amountMinor <= 0) {
    Toast.warning(`${label}必须大于 0 元且最多保留两位小数`);
    return null;
  }
  return amountMinor;
}

function setCreateFormValue<K extends keyof CreateFormState>(
  setter: Dispatch<SetStateAction<CreateFormState>>,
  key: K,
  value: CreateFormState[K],
) {
  setter((current) => ({ ...current, [key]: value }));
}

function setParticipantFormValue<K extends keyof ParticipantFormState>(
  setter: Dispatch<SetStateAction<ParticipantFormState>>,
  key: K,
  value: ParticipantFormState[K],
) {
  setter((current) => ({ ...current, [key]: value }));
}

function statusText(status: GroupBuyPlanStatus) {
  const mapping: Record<GroupBuyPlanStatus, string> = {
    cancelled: '已取消',
    draft: '草稿',
    filled: '已满单',
    open: '进行中',
    settled: '已结算',
  };

  return mapping[status];
}

function statusColor(status: GroupBuyPlanStatus) {
  const mapping = {
    cancelled: 'grey',
    draft: 'blue',
    filled: 'green',
    open: 'cyan',
    settled: 'teal',
  } as const satisfies Record<GroupBuyPlanStatus, string>;

  return mapping[status];
}

function formatRuleCode(code: PlayRuleCode | string) {
  const labels: Partial<Record<PlayRuleCode, string>> = {
    fiveBackDirect: '后三直选',
    fiveBackDirectCombination: '后三直选组合',
    fiveBackGroupSix: '后三组六',
    fiveBackGroupSixBanker: '后三组六胆拖',
    fiveBackGroupThree: '后三组三',
    fiveBackGroupThreeBanker: '后三组三胆拖',
    fiveBigSmallOddEven: '大小单双',
    fiveFrontDirect: '前三直选',
    fiveFrontDirectCombination: '前三直选组合',
    fiveFrontGroupSix: '前三组六',
    fiveFrontGroupSixBanker: '前三组六胆拖',
    fiveFrontGroupThree: '前三组三',
    fiveFrontGroupThreeBanker: '前三组三胆拖',
    fiveMiddleDirect: '中三直选',
    fiveMiddleDirectCombination: '中三直选组合',
    fiveMiddleGroupSix: '中三组六',
    fiveMiddleGroupSixBanker: '中三组六胆拖',
    fiveMiddleGroupThree: '中三组三',
    fiveMiddleGroupThreeBanker: '中三组三胆拖',
    threeDirect: '三位直选',
    threeGroupSix: '三位组六',
    threeGroupSixBanker: '三位组六胆拖',
    threeGroupThree: '三位组三',
    threeGroupThreeBanker: '三位组三胆拖',
  };

  return labels[code as PlayRuleCode] ?? code;
}
