import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { Link2, RefreshCcw, Save, UserPlus } from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { MetricCard } from '../components/MetricCard';
import { useInvitations } from '../hooks/useInvitations';
import type { InvitePolicySummary, UserKind, UserSummary } from '../types/dashboard';
import type {
  CreateInviteRecordRequest,
  InviteRecord,
  InviteStatus,
  UpdateInviteRecordRequest,
} from '../types/invitations';

interface InviteManagementPageProps {
  onDashboardRefresh: () => void;
}

interface CreateFormState {
  id: string;
  inviteCode: string;
  inviteeUserId: string;
  inviterUserId: string;
  note: string;
  rebateEnabled: boolean;
}

interface UpdateFormState {
  note: string;
  rebateEnabled: boolean;
  status: InviteStatus;
}

export function InviteManagementPage({
  onDashboardRefresh,
}: InviteManagementPageProps) {
  const {
    create,
    error,
    invitations,
    invitePolicy,
    loading,
    refresh,
    saving,
    update,
    users,
  } = useInvitations();
  const [createForm, setCreateForm] = useState<CreateFormState>(() =>
    emptyCreateForm(),
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [updateForm, setUpdateForm] = useState<UpdateFormState>(() =>
    emptyUpdateForm(),
  );
  const inviterCandidates = useMemo(
    () => permittedInviters(users, invitePolicy),
    [invitePolicy, users],
  );
  const inviteeCandidates = useMemo(
    () => users.filter((user) => user.id !== createForm.inviterUserId),
    [createForm.inviterUserId, users],
  );
  const selectedInvitation = useMemo(
    () =>
      invitations.find((invitation) => invitation.id === selectedId) ??
      invitations[0] ??
      null,
    [invitations, selectedId],
  );
  const totals = useMemo(() => inviteTotals(invitations), [invitations]);

  useEffect(() => {
    if (!createForm.inviterUserId && inviterCandidates[0]) {
      setCreateForm((current) => ({
        ...current,
        inviterUserId: inviterCandidates[0].id,
      }));
    }
  }, [createForm.inviterUserId, inviterCandidates]);

  useEffect(() => {
    if (!createForm.inviteeUserId && inviteeCandidates[0]) {
      setCreateForm((current) => ({
        ...current,
        inviteeUserId: inviteeCandidates[0].id,
      }));
    }
  }, [createForm.inviteeUserId, inviteeCandidates]);

  useEffect(() => {
    if (
      createForm.inviteeUserId &&
      createForm.inviteeUserId === createForm.inviterUserId &&
      inviteeCandidates[0]
    ) {
      setCreateForm((current) => ({
        ...current,
        inviteeUserId: inviteeCandidates[0].id,
      }));
    }
  }, [createForm.inviteeUserId, createForm.inviterUserId, inviteeCandidates]);

  useEffect(() => {
    if (selectedInvitation && selectedInvitation.id !== selectedId) {
      setSelectedId(selectedInvitation.id);
    }
  }, [selectedId, selectedInvitation]);

  useEffect(() => {
    if (selectedInvitation) {
      setUpdateForm(formFromInvitation(selectedInvitation));
    }
  }, [selectedInvitation]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const submitCreate = async () => {
    const created = await create(createPayload(createForm));
    setSelectedId(created.id);
    setCreateForm(
      emptyCreateForm(inviterCandidates[0]?.id, inviteeCandidates[0]?.id),
    );
    onDashboardRefresh();
  };

  const submitUpdate = async () => {
    if (!selectedInvitation) {
      return;
    }
    const updated = await update(selectedInvitation.id, updatePayload(updateForm));
    setSelectedId(updated.id);
    onDashboardRefresh();
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">邀请管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            维护代理与下级用户的邀请关系、返利资格和关系状态。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="邀请接口错误" description={error} /> : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="邀请关系"
          trend={`${totals.activeCount} 条有效`}
          value={`${invitations.length}`}
        />
        <MetricCard label="待确认" trend="等待处理" value={`${totals.pendingCount}`} />
        <MetricCard label="可返利" trend="返利资格开启" value={`${totals.rebateCount}`} />
        <MetricCard
          label="允许邀请人"
          trend="当前策略"
          value={`${inviterCandidates.length}`}
        />
      </section>

      {loading ? (
        <Card className="rounded-md border border-line">
          <div className="grid min-h-[320px] place-items-center">
            <Spin tip="正在加载邀请关系" />
          </div>
        </Card>
      ) : (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(360px,0.95fr)]">
          <div className="space-y-4">
            <Card className="rounded-md border border-line">
              <div className="mb-4 flex items-center justify-between">
                <h2 className="text-base font-semibold text-ink">邀请关系列表</h2>
                <Tag color="teal">{invitations.length} 条</Tag>
              </div>
              <div className="overflow-x-auto">
                <table className="min-w-full text-left text-sm">
                  <thead className="border-b border-line text-xs text-slate-500">
                    <tr>
                      <th className="py-2 pr-4 font-medium">邀请关系</th>
                      <th className="py-2 pr-4 font-medium">邀请码</th>
                      <th className="py-2 pr-4 font-medium">状态</th>
                      <th className="py-2 pr-4 font-medium">返利</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-line">
                    {invitations.map((invitation) => (
                      <tr
                        key={invitation.id}
                        className={
                          selectedInvitation?.id === invitation.id
                            ? 'bg-teal-50/60'
                            : ''
                        }
                      >
                        <td className="py-3 pr-4">
                          <button
                            className="text-left font-medium text-ink hover:text-teal-700"
                            type="button"
                            onClick={() => setSelectedId(invitation.id)}
                          >
                            {invitation.inviterUsername} → {invitation.inviteeUsername}
                          </button>
                          <div className="mt-1 text-xs text-slate-400">
                            {invitation.id}
                          </div>
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {invitation.inviteCode}
                        </td>
                        <td className="py-3 pr-4">
                          <Tag color={statusColor(invitation.status)}>
                            {statusText(invitation.status)}
                          </Tag>
                        </td>
                        <td className="py-3 pr-4">
                          <Tag color={invitation.rebateEnabled ? 'green' : 'grey'}>
                            {invitation.rebateEnabled ? '开启' : '关闭'}
                          </Tag>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>

            <Card className="rounded-md border border-line">
              <div className="mb-4 flex items-center gap-2">
                <UserPlus size={17} />
                <h2 className="text-base font-semibold text-ink">新增邀请关系</h2>
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <Field label="关系 ID">
                  <input
                    className="h-10 w-full rounded-md border border-line px-3 text-sm outline-none focus:border-teal-500"
                    value={createForm.id}
                    onChange={(event) =>
                      setCreateFormValue(setCreateForm, 'id', event.target.value)
                    }
                  />
                </Field>
                <Field label="邀请码">
                  <input
                    className="h-10 w-full rounded-md border border-line px-3 text-sm outline-none focus:border-teal-500"
                    value={createForm.inviteCode}
                    onChange={(event) =>
                      setCreateFormValue(
                        setCreateForm,
                        'inviteCode',
                        event.target.value,
                      )
                    }
                  />
                </Field>
                <Field label="邀请人">
                  <select
                    className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                    value={createForm.inviterUserId}
                    onChange={(event) =>
                      setCreateFormValue(
                        setCreateForm,
                        'inviterUserId',
                        event.target.value,
                      )
                    }
                  >
                    {inviterCandidates.map((user) => (
                      <option key={user.id} value={user.id}>
                        {user.username} ({userKindText(user.kind)})
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="被邀请人">
                  <select
                    className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                    value={createForm.inviteeUserId}
                    onChange={(event) =>
                      setCreateFormValue(
                        setCreateForm,
                        'inviteeUserId',
                        event.target.value,
                      )
                    }
                  >
                    {inviteeCandidates.map((user) => (
                      <option key={user.id} value={user.id}>
                        {user.username} ({user.id})
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="备注">
                  <textarea
                    className="min-h-24 w-full rounded-md border border-line px-3 py-2 text-sm outline-none focus:border-teal-500"
                    value={createForm.note}
                    onChange={(event) =>
                      setCreateFormValue(setCreateForm, 'note', event.target.value)
                    }
                  />
                </Field>
                <Field label="返利资格">
                  <label className="inline-flex items-center gap-2 text-sm text-slate-700">
                    <input
                      checked={createForm.rebateEnabled}
                      className="h-4 w-4 rounded border-line text-teal-600"
                      type="checkbox"
                      onChange={(event) =>
                        setCreateFormValue(
                          setCreateForm,
                          'rebateEnabled',
                          event.currentTarget.checked,
                        )
                      }
                    />
                    {createForm.rebateEnabled ? '开启返利' : '关闭返利'}
                  </label>
                </Field>
              </div>
              <div className="mt-4">
                <Button
                  disabled={saving || !inviterCandidates.length || !inviteeCandidates.length}
                  icon={<Save size={16} />}
                  loading={saving}
                  onClick={() => void submitCreate()}
                  theme="solid"
                >
                  创建邀请关系
                </Button>
              </div>
            </Card>
          </div>

          <Card className="rounded-md border border-line">
            {selectedInvitation ? (
              <div className="space-y-5">
                <div className="flex items-start justify-between gap-3 border-b border-line pb-4">
                  <div>
                    <h2 className="text-base font-semibold text-ink">
                      {selectedInvitation.inviterUsername} →{' '}
                      {selectedInvitation.inviteeUsername}
                    </h2>
                    <p className="mt-1 text-sm text-slate-500">
                      {selectedInvitation.inviteCode} · {selectedInvitation.id}
                    </p>
                  </div>
                  <Tag color={statusColor(selectedInvitation.status)}>
                    {statusText(selectedInvitation.status)}
                  </Tag>
                </div>

                <div className="grid gap-3 sm:grid-cols-2">
                  <InfoRow label="邀请人 ID" value={selectedInvitation.inviterUserId} />
                  <InfoRow label="被邀请人 ID" value={selectedInvitation.inviteeUserId} />
                  <InfoRow label="创建时间" value={selectedInvitation.createdAt} />
                  <InfoRow label="更新时间" value={selectedInvitation.updatedAt} />
                </div>

                <div className="grid gap-3 md:grid-cols-2">
                  <Field label="状态">
                    <select
                      className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                      value={updateForm.status}
                      onChange={(event) =>
                        setUpdateFormValue(
                          setUpdateForm,
                          'status',
                          event.target.value as InviteStatus,
                        )
                      }
                    >
                      <option value="pending">待确认</option>
                      <option value="active">有效</option>
                      <option value="disabled">停用</option>
                    </select>
                  </Field>
                  <Field label="返利资格">
                    <label className="inline-flex items-center gap-2 text-sm text-slate-700">
                      <input
                        checked={updateForm.rebateEnabled}
                        className="h-4 w-4 rounded border-line text-teal-600"
                        type="checkbox"
                        onChange={(event) =>
                          setUpdateFormValue(
                            setUpdateForm,
                            'rebateEnabled',
                            event.currentTarget.checked,
                          )
                        }
                      />
                      {updateForm.rebateEnabled ? '开启返利' : '关闭返利'}
                    </label>
                  </Field>
                  <Field label="备注">
                    <textarea
                      className="min-h-28 w-full rounded-md border border-line px-3 py-2 text-sm outline-none focus:border-teal-500"
                      value={updateForm.note}
                      onChange={(event) =>
                        setUpdateFormValue(setUpdateForm, 'note', event.target.value)
                      }
                    />
                  </Field>
                </div>

                <Button
                  disabled={saving}
                  icon={<Link2 size={16} />}
                  loading={saving}
                  onClick={() => void submitUpdate()}
                  theme="solid"
                >
                  保存邀请关系
                </Button>
              </div>
            ) : (
              <div className="grid min-h-[320px] place-items-center text-sm text-slate-500">
                暂无邀请关系。
              </div>
            )}
          </Card>
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
    <label className="block">
      <span className="mb-1 block text-xs font-medium text-slate-500">{label}</span>
      {children}
    </label>
  );
}

interface InfoRowProps {
  label: string;
  value: string;
}

function InfoRow({ label, value }: InfoRowProps) {
  return (
    <div className="rounded-md border border-line bg-slate-50 px-3 py-2 text-sm">
      <div className="text-xs text-slate-500">{label}</div>
      <div className="mt-1 font-medium text-ink">{value}</div>
    </div>
  );
}

function emptyCreateForm(inviterUserId = '', inviteeUserId = ''): CreateFormState {
  return {
    id: 'INV-NEW',
    inviteCode: 'INVITE-NEW',
    inviteeUserId,
    inviterUserId,
    note: '',
    rebateEnabled: true,
  };
}

function emptyUpdateForm(): UpdateFormState {
  return {
    note: '',
    rebateEnabled: true,
    status: 'active',
  };
}

function formFromInvitation(invitation: InviteRecord): UpdateFormState {
  return {
    note: invitation.note,
    rebateEnabled: invitation.rebateEnabled,
    status: invitation.status,
  };
}

function createPayload(form: CreateFormState): CreateInviteRecordRequest {
  return {
    id: form.id,
    inviteCode: form.inviteCode,
    inviteeUserId: form.inviteeUserId,
    inviterUserId: form.inviterUserId,
    note: form.note,
    rebateEnabled: form.rebateEnabled,
  };
}

function updatePayload(form: UpdateFormState): UpdateInviteRecordRequest {
  return {
    note: form.note,
    rebateEnabled: form.rebateEnabled,
    status: form.status,
  };
}

function permittedInviters(
  users: UserSummary[],
  invitePolicy: InvitePolicySummary | null,
) {
  return users.filter((user) => {
    if (user.kind === 'agent') {
      return invitePolicy?.agentsCanInvite ?? true;
    }
    return invitePolicy?.regularUsersCanInvite ?? false;
  });
}

function inviteTotals(invitations: InviteRecord[]) {
  return {
    activeCount: invitations.filter((invitation) => invitation.status === 'active')
      .length,
    pendingCount: invitations.filter((invitation) => invitation.status === 'pending')
      .length,
    rebateCount: invitations.filter((invitation) => invitation.rebateEnabled).length,
  };
}

function statusText(status: InviteStatus) {
  const labels: Record<InviteStatus, string> = {
    active: '有效',
    disabled: '停用',
    pending: '待确认',
  };
  return labels[status];
}

function statusColor(status: InviteStatus) {
  const colors: Record<InviteStatus, 'blue' | 'green' | 'grey'> = {
    active: 'green',
    disabled: 'grey',
    pending: 'blue',
  };
  return colors[status];
}

function userKindText(kind: UserKind) {
  return kind === 'agent' ? '代理' : '普通用户';
}

function setCreateFormValue<K extends keyof CreateFormState>(
  setForm: (updater: (current: CreateFormState) => CreateFormState) => void,
  key: K,
  value: CreateFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function setUpdateFormValue<K extends keyof UpdateFormState>(
  setForm: (updater: (current: UpdateFormState) => UpdateFormState) => void,
  key: K,
  value: UpdateFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}
