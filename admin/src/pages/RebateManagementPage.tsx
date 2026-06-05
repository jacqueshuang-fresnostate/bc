import { Input, Banner, Button, Card, Select, Spin, Tag } from '@douyinfe/semi-ui';
import { Percent, RefreshCcw, Save, UserPlus, Users } from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { MetricCard } from '../components/MetricCard';
import { useRebatePolicy } from '../hooks/useRebatePolicy';
import type {
  InvitePolicySummary,
  InvitePolicyUpdateRequest,
  RebateMode,
} from '../types/rebates';

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
  const { error, loading, policy, refresh, registration, save, saving } =
    useRebatePolicy();
  const [form, setForm] = useState<RebateFormState>(() => emptyForm());
  const currentMode = policy?.rebateMode ?? form.rebateMode;
  const totals = useMemo(() => policyTotals(policy), [policy]);

  useEffect(() => {
    if (policy) {
      setForm(formFromPolicy(policy));
    }
  }, [policy]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const submit = async () => {
    const saved = await save(policyPayload(form));
    setForm(formFromPolicy(saved));
    onDashboardRefresh();
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">返利配置</h1>
          <p className="mt-1 text-sm text-slate-500">
            维护代理邀请入口、普通用户邀请入口和默认充值返利策略。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="返利接口错误" description={error} /> : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
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
        <Card className="rounded-md border border-line">
          <div className="grid min-h-[320px] place-items-center">
            <Spin tip="正在加载返利配置" />
          </div>
        </Card>
      ) : (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
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
                  className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
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
                    className="h-10 w-full rounded-md border border-line px-3 text-sm outline-none focus:border-teal-500"
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

function rebateModeText(mode: RebateMode) {
  return mode === 'immediate' ? '立即返利' : '充值阶梯返利';
}

function percentText(basisPoints: number) {
  return `${(basisPoints / 100).toFixed(2)}%`;
}
