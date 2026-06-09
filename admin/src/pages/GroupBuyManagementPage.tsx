import { Input, Banner, Button, Card, Select, Spin, Tag, TextArea, Toast } from '@douyinfe/semi-ui';
import { Plus, RefreshCcw, Save, Share2, Users } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { MetricCard } from '../components/MetricCard';
import { PageControls } from '../components/PageControls';
import { useGroupBuyPlans } from '../hooks/useGroupBuyPlans';
import type {
  AddGroupBuyParticipantRequest,
  CreateGroupBuyPlanRequest,
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

export function GroupBuyManagementPage({
  onDashboardRefresh,
}: GroupBuyManagementPageProps) {
  const [planPageNumber, setPlanPageNumber] = useState(1);
  const [planPageSize, setPlanPageSize] = useState(10);
  const {
    addParticipant,
    create,
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
    planQuery: { page: planPageNumber, pageSize: planPageSize },
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
  const totals = useMemo(() => groupBuyTotals(plans), [plans]);
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

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">合买配置</h1>
          <p className="mt-1 text-sm text-slate-500">
            维护合买计划、认购进度、参与记录和计划状态。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="合买接口错误" description={error} /> : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard label="合买计划" trend="全部计划" value={`${planPage.totalCount}`} />
        <MetricCard label="进行中" trend="当前页 open/draft" value={`${totals.openCount}`} />
        <MetricCard label="已满单" trend="当前页自动满额" value={`${totals.filledCount}`} />
        <MetricCard
          label="已认购"
          trend={`当前页 ${formatMoney(totals.totalAmountMinor)}`}
          value={formatMoney(totals.filledAmountMinor)}
        />
      </section>

      {loading ? (
        <Card className="rounded-md border border-line">
          <div className="grid min-h-[320px] place-items-center">
            <Spin tip="正在加载合买计划" />
          </div>
        </Card>
      ) : (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(380px,0.95fr)]">
          <div className="space-y-4">
            <Card className="rounded-md border border-line">
              <div className="mb-4 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
                <div className="flex items-center gap-2">
                  <h2 className="text-base font-semibold text-ink">合买计划列表</h2>
                  <Tag color="teal">{planPage.totalCount} 个计划</Tag>
                </div>
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
              {plans.length > 0 ? (
                <div className="overflow-x-auto">
                  <table className="w-full min-w-[860px] text-left text-sm">
                    <thead className="border-b border-line text-xs text-slate-500">
                      <tr>
                        <th className="py-2 pr-4 font-medium">计划</th>
                        <th className="py-2 pr-4 font-medium">彩种/期号</th>
                        <th className="py-2 pr-4 font-medium">成单</th>
                        <th className="py-2 pr-4 font-medium">进度</th>
                        <th className="py-2 pr-4 font-medium">状态</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-line">
                      {plans.map((plan) => (
                        <tr
                          key={plan.id}
                          className={selectedPlan?.id === plan.id ? 'bg-teal-50/60' : ''}
                        >
                          <td className="py-3 pr-4">
                            <button
                              className="text-left font-semibold text-ink hover:text-teal-700"
                              type="button"
                              onClick={() => void loadPlan(plan.id)}
                            >
                              {plan.id}
                            </button>
                            <div className="mt-1 text-xs text-slate-400">
                              发起人：{plan.initiatorUsername}
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

            <Card className="rounded-md border border-line">
              <div className="mb-4 flex items-center gap-2">
                <Share2 size={17} />
                <h2 className="text-base font-semibold text-ink">新增合买计划</h2>
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
            </Card>
          </div>

          <div className="space-y-4">
            <Card className="rounded-md border border-line">
              <div className="mb-4 flex items-center gap-2">
                <Users size={17} />
                <h2 className="text-base font-semibold text-ink">参与记录</h2>
              </div>
              {selectedPlan ? (
                <div className="space-y-4">
                  <div className="overflow-x-auto">
                    <table className="w-full min-w-[520px] text-left text-sm">
                      <thead className="border-b border-line text-xs text-slate-500">
                        <tr>
                          <th className="py-2 pr-4 font-medium">用户</th>
                          <th className="py-2 pr-4 font-medium">金额</th>
                          <th className="py-2 pr-4 font-medium">份数</th>
                          <th className="py-2 pr-4 font-medium">备注</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-line">
                        {selectedPlan.participants.map((participant) => (
                          <tr key={participant.id}>
                            <td className="py-3 pr-4">
                              <div className="font-medium text-ink">
                                {participant.username}
                              </div>
                              <div className="mt-1 text-xs text-slate-400">
                                {participant.id}
                              </div>
                            </td>
                            <td className="py-3 pr-4 text-slate-600">
                              {formatMoney(participant.amountMinor)}
                            </td>
                            <td className="py-3 pr-4 text-slate-600">
                              {participant.shareCount}
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
                        !['draft', 'open'].includes(selectedPlan.status) ||
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
              ) : (
                <div className="rounded-md border border-line p-4 text-sm text-slate-500">
                  暂无参与记录。
                </div>
              )}
            </Card>

            <Card className="rounded-md border border-line">
              <div className="mb-4 flex items-center justify-between gap-3">
                <div>
                  <h2 className="text-base font-semibold text-ink">计划详情</h2>
                  <p className="mt-1 text-sm text-slate-500">
                    {selectedPlan ? selectedPlan.id : '请选择一个合买计划'}
                  </p>
                </div>
                {selectedPlan ? (
                  <Tag color={statusColor(selectedPlan.status)}>
                    {statusText(selectedPlan.status)}
                  </Tag>
                ) : null}
              </div>

              {selectedPlan ? (
                <div className="space-y-4">
                  <div className="grid gap-3 sm:grid-cols-2">
                    <InfoLine label="彩种" value={selectedPlan.lotteryName} />
                    <InfoLine label="期号" value={selectedPlan.issue || '-'} />
                    <InfoLine
                      label="玩法"
                      value={formatRuleCode(selectedPlan.ruleCode as PlayRuleCode)}
                    />
                    <InfoLine label="订单号" value={selectedPlan.orderId ?? '未成单'} />
                    <InfoLine label="发起人" value={selectedPlan.initiatorUsername} />
                    <InfoLine
                      label="计划总额"
                      value={formatMoney(selectedPlan.totalAmountMinor)}
                    />
                    <InfoLine
                      label="已认购"
                      value={formatMoney(selectedPlan.filledAmountMinor)}
                    />
                    <InfoLine label="总份数" value={`${selectedPlan.shareCount} 份`} />
                    <InfoLine
                      label="每份金额"
                      value={formatMoney(selectedPlan.minShareAmountMinor)}
                    />
                  </div>
                  <div className="rounded-md border border-line px-3 py-2">
                    <div className="text-xs text-slate-500">投注内容</div>
                    <div className="mt-1 whitespace-pre-wrap break-all text-sm font-medium text-ink">
                      {selectedPlan.numbers || '-'}
                    </div>
                  </div>

                  <div className="grid gap-3 sm:grid-cols-2">
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
                  <div className="flex justify-end">
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
                </div>
              ) : (
                <div className="rounded-md border border-line p-4 text-sm text-slate-500">
                  暂无可维护计划。
                </div>
              )}
            </Card>
          </div>
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

function groupBuyTotals(plans: GroupBuyPlanSummary[]) {
  return {
    filledAmountMinor: plans.reduce(
      (total, plan) => total + plan.filledAmountMinor,
      0,
    ),
    filledCount: plans.filter((plan) => plan.status === 'filled').length,
    openCount: plans.filter((plan) => ['draft', 'open'].includes(plan.status)).length,
    totalAmountMinor: plans.reduce(
      (total, plan) => total + plan.totalAmountMinor,
      0,
    ),
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
