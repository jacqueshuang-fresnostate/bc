import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { MessageCircle, RefreshCcw, Save, Send, UserPlus } from 'lucide-react';
import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { MetricCard } from '../components/MetricCard';
import { useSupportConversations } from '../hooks/useSupportConversations';
import type {
  CreateSupportConversationRequest,
  SupportConversation,
  SupportConversationStatus,
  SupportPriority,
  UpdateSupportConversationRequest,
} from '../types/support';

interface SupportManagementPageProps {
  onDashboardRefresh: () => void;
}

interface CreateFormState {
  content: string;
  id: string;
  priority: SupportPriority;
  subject: string;
  userId: string;
}

interface UpdateFormState {
  assignedAdminId: string;
  priority: SupportPriority;
  status: SupportConversationStatus;
}

export function SupportManagementPage({
  onDashboardRefresh,
}: SupportManagementPageProps) {
  const {
    admins,
    conversations,
    create,
    error,
    loading,
    refresh,
    reply,
    saving,
    update,
    users,
  } = useSupportConversations();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [createForm, setCreateForm] = useState<CreateFormState>(() =>
    emptyCreateForm(),
  );
  const [replyContent, setReplyContent] = useState('');
  const [updateForm, setUpdateForm] = useState<UpdateFormState>(() =>
    emptyUpdateForm(),
  );
  const selectedConversation = useMemo(
    () =>
      conversations.find((conversation) => conversation.id === selectedId) ??
      conversations[0] ??
      null,
    [conversations, selectedId],
  );
  const totals = useMemo(() => supportTotals(conversations), [conversations]);

  useEffect(() => {
    if (!createForm.userId && users[0]) {
      setCreateForm((current) => ({ ...current, userId: users[0].id }));
    }
  }, [createForm.userId, users]);

  useEffect(() => {
    if (selectedConversation && selectedConversation.id !== selectedId) {
      setSelectedId(selectedConversation.id);
    }
  }, [selectedConversation, selectedId]);

  useEffect(() => {
    if (selectedConversation) {
      setUpdateForm(formFromConversation(selectedConversation));
    }
  }, [selectedConversation]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const submitCreate = async () => {
    const created = await create(createPayload(createForm));
    setSelectedId(created.id);
    setCreateForm(emptyCreateForm(users[0]?.id));
    onDashboardRefresh();
  };

  const submitUpdate = async () => {
    if (!selectedConversation) {
      return;
    }
    const updated = await update(selectedConversation.id, updatePayload(updateForm));
    setSelectedId(updated.id);
    onDashboardRefresh();
  };

  const submitReply = async () => {
    if (!selectedConversation) {
      return;
    }
    const adminId = updateForm.assignedAdminId || admins[0]?.id || '';
    const updated = await reply(selectedConversation.id, {
      adminId,
      content: replyContent,
    });
    setSelectedId(updated.id);
    setReplyContent('');
    onDashboardRefresh();
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">在线客服</h1>
          <p className="mt-1 text-sm text-slate-500">
            维护客服会话、工单状态、分配客服和后台回复记录。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="客服接口错误" description={error} /> : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="会话总数"
          trend={`${totals.openCount} 个处理中`}
          value={`${conversations.length}`}
        />
        <MetricCard label="未读消息" trend="待客服处理" value={`${totals.unread}`} />
        <MetricCard label="紧急会话" trend="优先处理" value={`${totals.urgent}`} />
        <MetricCard label="已解决" trend="已完成工单" value={`${totals.resolved}`} />
      </section>

      {loading ? (
        <Card className="rounded-md border border-line">
          <div className="grid min-h-[320px] place-items-center">
            <Spin tip="正在加载客服会话" />
          </div>
        </Card>
      ) : (
        <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(360px,0.95fr)]">
          <div className="space-y-4">
            <Card className="rounded-md border border-line">
              <div className="mb-4 flex items-center justify-between">
                <h2 className="text-base font-semibold text-ink">会话列表</h2>
                <Tag color="teal">{conversations.length} 条</Tag>
              </div>
              <div className="overflow-x-auto">
                <table className="min-w-full text-left text-sm">
                  <thead className="border-b border-line text-xs text-slate-500">
                    <tr>
                      <th className="py-2 pr-4 font-medium">主题</th>
                      <th className="py-2 pr-4 font-medium">用户</th>
                      <th className="py-2 pr-4 font-medium">状态</th>
                      <th className="py-2 pr-4 font-medium">优先级</th>
                      <th className="py-2 pr-4 font-medium">未读</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-line">
                    {conversations.map((conversation) => (
                      <tr
                        key={conversation.id}
                        className={
                          selectedConversation?.id === conversation.id
                            ? 'bg-teal-50/60'
                            : ''
                        }
                      >
                        <td className="py-3 pr-4">
                          <button
                            className="text-left font-medium text-ink hover:text-teal-700"
                            type="button"
                            onClick={() => setSelectedId(conversation.id)}
                          >
                            {conversation.subject}
                          </button>
                          <div className="mt-1 text-xs text-slate-400">
                            {conversation.id}
                          </div>
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {conversation.username}
                        </td>
                        <td className="py-3 pr-4">
                          <Tag color={statusColor(conversation.status)}>
                            {statusText(conversation.status)}
                          </Tag>
                        </td>
                        <td className="py-3 pr-4">
                          <Tag color={priorityColor(conversation.priority)}>
                            {priorityText(conversation.priority)}
                          </Tag>
                        </td>
                        <td className="py-3 pr-4 text-slate-600">
                          {conversation.unreadCount}
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
                <h2 className="text-base font-semibold text-ink">新建会话</h2>
              </div>
              <div className="grid gap-3 md:grid-cols-2">
                <Field label="会话 ID">
                  <input
                    className="h-10 w-full rounded-md border border-line px-3 text-sm outline-none focus:border-teal-500"
                    value={createForm.id}
                    onChange={(event) =>
                      setCreateFormValue(setCreateForm, 'id', event.target.value)
                    }
                  />
                </Field>
                <Field label="绑定用户">
                  <select
                    className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                    value={createForm.userId}
                    onChange={(event) =>
                      setCreateFormValue(setCreateForm, 'userId', event.target.value)
                    }
                  >
                    {users.map((user) => (
                      <option key={user.id} value={user.id}>
                        {user.username} ({user.id})
                      </option>
                    ))}
                  </select>
                </Field>
                <Field label="主题">
                  <input
                    className="h-10 w-full rounded-md border border-line px-3 text-sm outline-none focus:border-teal-500"
                    value={createForm.subject}
                    onChange={(event) =>
                      setCreateFormValue(setCreateForm, 'subject', event.target.value)
                    }
                  />
                </Field>
                <Field label="优先级">
                  <select
                    className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                    value={createForm.priority}
                    onChange={(event) =>
                      setCreateFormValue(
                        setCreateForm,
                        'priority',
                        event.target.value as SupportPriority,
                      )
                    }
                  >
                    <option value="normal">普通</option>
                    <option value="urgent">紧急</option>
                  </select>
                </Field>
                <Field label="首条消息">
                  <textarea
                    className="min-h-24 w-full rounded-md border border-line px-3 py-2 text-sm outline-none focus:border-teal-500"
                    value={createForm.content}
                    onChange={(event) =>
                      setCreateFormValue(setCreateForm, 'content', event.target.value)
                    }
                  />
                </Field>
              </div>
              <div className="mt-4">
                <Button
                  disabled={saving}
                  icon={<Save size={16} />}
                  loading={saving}
                  onClick={() => void submitCreate()}
                  theme="solid"
                >
                  创建会话
                </Button>
              </div>
            </Card>
          </div>

          <Card className="rounded-md border border-line">
            {selectedConversation ? (
              <div className="space-y-5">
                <div className="flex flex-col gap-3 border-b border-line pb-4">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <h2 className="text-base font-semibold text-ink">
                        {selectedConversation.subject}
                      </h2>
                      <p className="mt-1 text-sm text-slate-500">
                        {selectedConversation.username} · {selectedConversation.id}
                      </p>
                    </div>
                    <Tag color={statusColor(selectedConversation.status)}>
                      {statusText(selectedConversation.status)}
                    </Tag>
                  </div>
                  <div className="grid gap-3 md:grid-cols-3">
                    <Field label="状态">
                      <select
                        className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                        value={updateForm.status}
                        onChange={(event) =>
                          setUpdateFormValue(
                            setUpdateForm,
                            'status',
                            event.target.value as SupportConversationStatus,
                          )
                        }
                      >
                        <option value="open">处理中</option>
                        <option value="pending">等待用户</option>
                        <option value="resolved">已解决</option>
                        <option value="closed">已关闭</option>
                      </select>
                    </Field>
                    <Field label="优先级">
                      <select
                        className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                        value={updateForm.priority}
                        onChange={(event) =>
                          setUpdateFormValue(
                            setUpdateForm,
                            'priority',
                            event.target.value as SupportPriority,
                          )
                        }
                      >
                        <option value="normal">普通</option>
                        <option value="urgent">紧急</option>
                      </select>
                    </Field>
                    <Field label="分配客服">
                      <select
                        className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                        value={updateForm.assignedAdminId}
                        onChange={(event) =>
                          setUpdateFormValue(
                            setUpdateForm,
                            'assignedAdminId',
                            event.target.value,
                          )
                        }
                      >
                        <option value="">未分配</option>
                        {admins.map((admin) => (
                          <option key={admin.id} value={admin.id}>
                            {admin.username} ({admin.id})
                          </option>
                        ))}
                      </select>
                    </Field>
                  </div>
                  <Button
                    disabled={saving}
                    icon={<Save size={16} />}
                    loading={saving}
                    onClick={() => void submitUpdate()}
                    theme="solid"
                  >
                    保存状态
                  </Button>
                </div>

                <div>
                  <div className="mb-3 flex items-center gap-2">
                    <MessageCircle size={17} />
                    <h3 className="text-sm font-semibold text-ink">消息记录</h3>
                  </div>
                  <div className="max-h-[360px] space-y-3 overflow-y-auto pr-1">
                    {selectedConversation.messages.map((message) => (
                      <div
                        key={message.id}
                        className="rounded-md border border-line bg-slate-50 px-3 py-2"
                      >
                        <div className="flex items-center justify-between gap-3 text-xs text-slate-500">
                          <span>
                            {authorText(message.author)} · {message.authorName}
                          </span>
                          <span>{message.createdAt}</span>
                        </div>
                        <div className="mt-2 text-sm text-ink">{message.content}</div>
                      </div>
                    ))}
                  </div>
                </div>

                <div className="border-t border-line pt-4">
                  <Field label="后台回复">
                    <textarea
                      className="min-h-28 w-full rounded-md border border-line px-3 py-2 text-sm outline-none focus:border-teal-500"
                      value={replyContent}
                      onChange={(event) => setReplyContent(event.target.value)}
                    />
                  </Field>
                  <div className="mt-3">
                    <Button
                      disabled={saving || !selectedConversation || !replyContent.trim()}
                      icon={<Send size={16} />}
                      loading={saving}
                      onClick={() => void submitReply()}
                      theme="solid"
                    >
                      发送回复
                    </Button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="grid min-h-[320px] place-items-center text-sm text-slate-500">
                暂无客服会话。
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

function emptyCreateForm(userId = ''): CreateFormState {
  return {
    content: '',
    id: 'CS-NEW',
    priority: 'normal',
    subject: '',
    userId,
  };
}

function emptyUpdateForm(): UpdateFormState {
  return {
    assignedAdminId: '',
    priority: 'normal',
    status: 'open',
  };
}

function formFromConversation(conversation: SupportConversation): UpdateFormState {
  return {
    assignedAdminId: conversation.assignedAdminId ?? '',
    priority: conversation.priority,
    status: conversation.status,
  };
}

function createPayload(form: CreateFormState): CreateSupportConversationRequest {
  return {
    content: form.content,
    id: form.id,
    priority: form.priority,
    subject: form.subject,
    userId: form.userId,
  };
}

function updatePayload(form: UpdateFormState): UpdateSupportConversationRequest {
  return {
    assignedAdminId: form.assignedAdminId || null,
    priority: form.priority,
    status: form.status,
  };
}

function supportTotals(conversations: SupportConversation[]) {
  return {
    openCount: conversations.filter(
      (conversation) =>
        conversation.status === 'open' || conversation.status === 'pending',
    ).length,
    resolved: conversations.filter((conversation) => conversation.status === 'resolved')
      .length,
    unread: conversations.reduce(
      (total, conversation) => total + conversation.unreadCount,
      0,
    ),
    urgent: conversations.filter((conversation) => conversation.priority === 'urgent')
      .length,
  };
}

function statusText(status: SupportConversationStatus) {
  const labels: Record<SupportConversationStatus, string> = {
    closed: '已关闭',
    open: '处理中',
    pending: '等待用户',
    resolved: '已解决',
  };
  return labels[status];
}

function statusColor(status: SupportConversationStatus) {
  const colors: Record<SupportConversationStatus, 'blue' | 'green' | 'grey' | 'orange'> = {
    closed: 'grey',
    open: 'blue',
    pending: 'orange',
    resolved: 'green',
  };
  return colors[status];
}

function priorityText(priority: SupportPriority) {
  return priority === 'urgent' ? '紧急' : '普通';
}

function priorityColor(priority: SupportPriority) {
  const colors: Record<SupportPriority, 'grey' | 'red'> = {
    normal: 'grey',
    urgent: 'red',
  };
  return colors[priority];
}

function authorText(author: string) {
  if (author === 'admin') {
    return '客服';
  }
  if (author === 'system') {
    return '系统';
  }
  return '用户';
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
