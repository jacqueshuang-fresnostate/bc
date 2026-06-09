import {
  Avatar,
  Banner,
  Button,
  Card,
  Chat,
  Popover,
  Select,
  Spin,
  Tabs,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import {
  Image as ImageIcon,
  MessageCircle,
  RefreshCcw,
  Save,
  Send,
  Smile,
  X,
} from 'lucide-react';
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentType,
  type KeyboardEvent,
  type ReactNode,
} from 'react';
import { uploadImageBedFile } from '../api/client';
import { extractImageUrlFromUploadResult } from '../components/ImageUploadAvatar';
import { MetricCard } from '../components/MetricCard';
import { useSupportConversations } from '../hooks/useSupportConversations';
import type {
  SupportConversation,
  SupportConversationStatus,
  SupportMessage,
  SupportMessageAuthor,
  SupportMessageType,
  SupportPriority,
  UpdateSupportConversationRequest,
} from '../types/support';

interface SupportManagementPageProps {
  onDashboardRefresh: () => void;
}

interface UpdateFormState {
  assignedAdminId: string;
  priority: SupportPriority;
  status: SupportConversationStatus;
}

type SupportChatRole = 'assistant' | 'system' | 'user';
type SupportStatusFilter = 'all' | SupportConversationStatus;

interface SupportChatMessage {
  authorName: string;
  authorText: string;
  content: string;
  createAt?: number;
  createdAtLabel: string;
  id: string;
  imageUrl?: string;
  messageType: SupportMessageType;
  role: SupportChatRole;
  status: 'complete';
}

interface EmojiMartPickerProps {
  data: unknown;
  i18n: unknown;
  locale: string;
  navPosition: string;
  onEmojiSelect: (emoji: unknown) => void;
  previewPosition: string;
  searchPosition: string;
  set: string;
  skinTonePosition: string;
  theme: string;
}

interface EmojiPickerRuntime {
  Picker: ComponentType<EmojiMartPickerProps>;
  data: unknown;
  i18n: unknown;
}

const SUPPORT_STATUS_FILTERS: Array<{
  key: SupportStatusFilter;
  label: string;
}> = [
  { key: 'all', label: '全部' },
  { key: 'open', label: '处理中' },
  { key: 'pending', label: '等待用户' },
  { key: 'resolved', label: '已解决' },
  { key: 'closed', label: '已关闭' },
];

export function SupportManagementPage({
  onDashboardRefresh,
}: SupportManagementPageProps) {
  const {
    admins,
    conversations,
    error,
    loading,
    refresh,
    reply,
    saving,
    update,
  } = useSupportConversations();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [replyContent, setReplyContent] = useState('');
  const [emojiPickerVisible, setEmojiPickerVisible] = useState(false);
  const [emojiPickerLoading, setEmojiPickerLoading] = useState(false);
  const [emojiPickerError, setEmojiPickerError] = useState('');
  const [emojiPickerRuntime, setEmojiPickerRuntime] =
    useState<EmojiPickerRuntime | null>(null);
  const replyTextAreaRef = useRef<HTMLTextAreaElement | null>(null);
  const replyImageInputRef = useRef<HTMLInputElement | null>(null);
  const [statusFilter, setStatusFilter] = useState<SupportStatusFilter>('all');
  const [replyImageUrl, setReplyImageUrl] = useState('');
  const [replyImageName, setReplyImageName] = useState('');
  const [replyImageUploading, setReplyImageUploading] = useState(false);
  const [updateForm, setUpdateForm] = useState<UpdateFormState>(() =>
    emptyUpdateForm(),
  );
  const visibleConversations = useMemo(
    () =>
      statusFilter === 'all'
        ? conversations
        : conversations.filter((conversation) => conversation.status === statusFilter),
    [conversations, statusFilter],
  );
  const selectedConversation = useMemo(
    () =>
      visibleConversations.find((conversation) => conversation.id === selectedId) ??
      visibleConversations[0] ??
      null,
    [selectedId, visibleConversations],
  );
  const totals = useMemo(() => supportTotals(conversations), [conversations]);
  const selectedChatMessages = useMemo(
    () =>
      selectedConversation
        ? selectedConversation.messages.map(supportMessageToChatMessage)
        : [],
    [selectedConversation],
  );
  const canSubmitReply =
    Boolean(selectedConversation) &&
    !saving &&
    !replyImageUploading &&
    Boolean(replyContent.trim() || replyImageUrl);
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

  useEffect(() => {
    if (!emojiPickerVisible || emojiPickerRuntime) {
      return;
    }

    let cancelled = false;
    setEmojiPickerLoading(true);
    setEmojiPickerError('');

    Promise.all([
      import('@emoji-mart/react'),
      import('@emoji-mart/data'),
      import('@emoji-mart/data/i18n/zh.json'),
    ])
      .then(([pickerModule, dataModule, i18nModule]) => {
        if (cancelled) {
          return;
        }
        setEmojiPickerRuntime({
          Picker: pickerModule.default as ComponentType<EmojiMartPickerProps>,
          data: dataModule.default,
          i18n: i18nModule.default,
        });
      })
      .catch(() => {
        if (!cancelled) {
          setEmojiPickerError('表情面板加载失败，请稍后重试。');
        }
      })
      .finally(() => {
        if (!cancelled) {
          setEmojiPickerLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [emojiPickerRuntime, emojiPickerVisible]);

  const refreshAll = () => {
    refresh();
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
    const trimmedContent = replyContent.trim();
    if (!trimmedContent && !replyImageUrl) {
      Toast.warning('请输入回复内容或上传图片');
      return;
    }
    const adminId = updateForm.assignedAdminId || admins[0]?.id || '';
    const updated = await reply(selectedConversation.id, {
      adminId,
      content: trimmedContent,
      imageUrl: replyImageUrl || null,
      messageType: replyImageUrl ? 'image' : 'text',
    });
    setSelectedId(updated.id);
    setReplyContent('');
    setReplyImageUrl('');
    setReplyImageName('');
    setEmojiPickerVisible(false);
    onDashboardRefresh();
  };

  const submitReplyByEnter = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (
      event.key !== 'Enter' ||
      event.shiftKey ||
      event.ctrlKey ||
      event.metaKey ||
      event.altKey ||
      event.nativeEvent.isComposing
    ) {
      return;
    }

    event.preventDefault();
    if (canSubmitReply || (selectedConversation && !saving && !replyImageUploading)) {
      void submitReply();
    }
  };

  const selectReplyImage = () => {
    replyImageInputRef.current?.click();
  };

  const uploadReplyImage = async (file: File) => {
    if (!file.type.startsWith('image/')) {
      Toast.warning('请选择图片文件');
      return;
    }

    setReplyImageUploading(true);
    try {
      const response = await uploadImageBedFile(file);
      const uploadedUrl = extractImageUrlFromUploadResult(response);
      if (!uploadedUrl) {
        throw new Error('图床返回未提供可用图片链接');
      }
      setReplyImageUrl(uploadedUrl);
      setReplyImageName(file.name);
      Toast.success('客服图片上传成功');
    } catch (uploadError: unknown) {
      Toast.error(uploadError instanceof Error ? uploadError.message : '客服图片上传失败');
    } finally {
      setReplyImageUploading(false);
    }
  };

  const insertEmoji = (emoji: unknown) => {
    const nativeEmoji = nativeEmojiFromSelection(emoji);
    if (!nativeEmoji) {
      return;
    }

    setReplyContent((current) => {
      const textarea = replyTextAreaRef.current;
      const selectionStart = textarea?.selectionStart ?? current.length;
      const selectionEnd = textarea?.selectionEnd ?? selectionStart;
      const nextContent = `${current.slice(0, selectionStart)}${nativeEmoji}${current.slice(selectionEnd)}`;
      const nextCursor = selectionStart + nativeEmoji.length;

      window.requestAnimationFrame(() => {
        const nextTextarea = replyTextAreaRef.current;
        nextTextarea?.focus();
        nextTextarea?.setSelectionRange(nextCursor, nextCursor);
      });

      return nextContent;
    });
    setEmojiPickerVisible(false);
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">在线客服</h1>
          <p className="mt-1 text-sm text-slate-500">
            处理用户发起的客服会话、工单状态、分配客服和后台回复记录。
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
          <Card className="rounded-md border border-line">
            <div className="mb-4 flex flex-col gap-3">
              <div className="flex items-center justify-between">
                <h2 className="text-base font-semibold text-ink">用户会话</h2>
                <Tag color="teal">{visibleConversations.length} 条</Tag>
              </div>
              <Tabs
                activeKey={statusFilter}
                collapsible
                onChange={(key) => setStatusFilter(String(key) as SupportStatusFilter)}
              >
                {SUPPORT_STATUS_FILTERS.map((filter) => (
                  <Tabs.TabPane
                    key={filter.key}
                    itemKey={filter.key}
                    tab={
                      <span className="inline-flex items-center gap-2">
                        <span>{filter.label}</span>
                        <Tag color={filter.key === statusFilter ? 'teal' : 'grey'}>
                          {statusFilterCount(conversations, filter.key)}
                        </Tag>
                      </span>
                    }
                  />
                ))}
              </Tabs>
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
                  {visibleConversations.map((conversation) => (
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
                        <div className="font-medium text-slate-700">
                          {conversation.username}
                        </div>
                        <div className="mt-1 font-mono text-xs text-slate-400">
                          {conversation.userId}
                        </div>
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
                  {visibleConversations.length === 0 ? (
                    <tr>
                      <td className="py-8 text-center text-sm text-slate-500" colSpan={5}>
                        当前状态下暂无客服会话。
                      </td>
                    </tr>
                  ) : null}
                </tbody>
              </table>
            </div>
          </Card>

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
                      <Select
                        className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                        value={updateForm.status}
                        onChange={(value) =>
                          setUpdateFormValue(
                            setUpdateForm,
                            'status',
                            (value as SupportConversationStatus) || 'open',
                          )
                        }
                      >
                        <Select.Option value="open">处理中</Select.Option>
                        <Select.Option value="pending">等待用户</Select.Option>
                        <Select.Option value="resolved">已解决</Select.Option>
                        <Select.Option value="closed">已关闭</Select.Option>
                      </Select>
                    </Field>
                    <Field label="优先级">
                      <Select
                        className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                        value={updateForm.priority}
                        onChange={(value) =>
                          setUpdateFormValue(
                            setUpdateForm,
                            'priority',
                            (value as SupportPriority) || 'normal',
                          )
                        }
                      >
                        <Select.Option value="normal">普通</Select.Option>
                        <Select.Option value="urgent">紧急</Select.Option>
                      </Select>
                    </Field>
                    <Field label="分配客服">
                      <Select
                        className="h-10 w-full rounded-md border border-line bg-white px-3 text-sm outline-none focus:border-teal-500"
                        value={updateForm.assignedAdminId}
                        onChange={(value) =>
                          setUpdateFormValue(
                            setUpdateForm,
                            'assignedAdminId',
                            String(value ?? ''),
                          )
                        }
                      >
                        <Select.Option value="">未分配</Select.Option>
                        {admins.map((admin) => (
                          <Select.Option key={admin.id} value={admin.id}>
                            {admin.username} ({admin.id})
                          </Select.Option>
                        ))}
                      </Select>
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
                  <Chat
                    align="leftRight"
                    chats={selectedChatMessages}
                    chatBoxRenderConfig={{
                      renderChatBoxAction: () => null,
                      renderChatBoxAvatar: ({ message }) => (
                        <Avatar
                          color={chatAvatarColor(message as SupportChatMessage | undefined)}
                          shape="square"
                          size="extra-small"
                        >
                          {chatAvatarText(message as SupportChatMessage | undefined)}
                        </Avatar>
                      ),
                      renderChatBoxTitle: ({ message, role }) => {
                        const chatMessage = message as SupportChatMessage | undefined;
                        return (
                          <span className="text-xs font-medium text-slate-500">
                            {chatMessage?.authorText ?? role?.name}
                            {chatMessage?.authorName
                              ? ` · ${chatMessage.authorName}`
                              : ''}
                            {chatMessage?.createdAtLabel
                              ? ` · ${chatMessage.createdAtLabel}`
                              : ''}
                          </span>
                        );
                      },
                      renderChatBoxContent: ({ defaultContent, message }) => {
                        const chatMessage = message as SupportChatMessage | undefined;
                        if (
                          chatMessage?.messageType !== 'image' ||
                          !chatMessage.imageUrl
                        ) {
                          return defaultContent;
                        }

                        return (
                          <div className="space-y-2">
                            <a
                              className="block"
                              href={chatMessage.imageUrl}
                              rel="noreferrer"
                              target="_blank"
                            >
                              <img
                                alt="客服图片消息"
                                className="max-h-60 max-w-[280px] rounded-md border border-slate-200 object-contain"
                                src={chatMessage.imageUrl}
                              />
                            </a>
                            {chatMessage.content ? (
                              <p className="whitespace-pre-wrap text-sm leading-6">
                                {chatMessage.content}
                              </p>
                            ) : null}
                          </div>
                        );
                      },
                    }}
                    className="rounded-md border border-line bg-white"
                    enableUpload={false}
                    mode="bubble"
                    renderInputArea={() => null}
                    roleConfig={{
                      assistant: { name: '客服' },
                      system: { name: '系统' },
                      user: { name: '用户' },
                    }}
                    style={{ height: 360 }}
                  />
                </div>

                <div className="border-t border-line pt-4">
                  <Field label="后台回复">
                    <textarea
                      ref={replyTextAreaRef}
                      className="min-h-28 w-full rounded-md border border-line px-3 py-2 text-sm outline-none focus:border-teal-500"
                      placeholder={replyImageUrl ? '可选填写图片说明' : '输入回复内容'}
                      value={replyContent}
                      onChange={(event) => setReplyContent(event.target.value)}
                      onKeyDown={submitReplyByEnter}
                    />
                  </Field>
                  <input
                    ref={replyImageInputRef}
                    accept="image/*"
                    className="hidden"
                    type="file"
                    onChange={(event) => {
                      const file = event.currentTarget.files?.[0];
                      event.currentTarget.value = '';
                      if (file) {
                        void uploadReplyImage(file);
                      }
                    }}
                  />
                  {replyImageUrl ? (
                    <div className="mt-3 flex items-start gap-3 rounded-md border border-teal-100 bg-teal-50 p-3">
                      <img
                        alt="待发送客服图片"
                        className="h-24 w-24 rounded-md border border-teal-100 bg-white object-cover"
                        src={replyImageUrl}
                      />
                      <div className="min-w-0 flex-1">
                        <p className="truncate text-sm font-medium text-teal-800">
                          {replyImageName || '客服图片'}
                        </p>
                        <a
                          className="mt-1 block break-all text-xs text-teal-700 hover:text-teal-800"
                          href={replyImageUrl}
                          rel="noreferrer"
                          target="_blank"
                        >
                          {replyImageUrl}
                        </a>
                        <Button
                          className="mt-2"
                          icon={<X size={14} />}
                          size="small"
                          onClick={() => {
                            setReplyImageUrl('');
                            setReplyImageName('');
                          }}
                        >
                          移除图片
                        </Button>
                      </div>
                    </div>
                  ) : null}
                  <div className="mt-3 flex flex-wrap items-center gap-2">
                    <Button
                      disabled={saving || !selectedConversation || replyImageUploading}
                      icon={<ImageIcon size={16} />}
                      loading={replyImageUploading}
                      onClick={selectReplyImage}
                    >
                      图片
                    </Button>
                    <Popover
                      content={
                        <div className="max-w-[min(352px,calc(100vw-48px))] overflow-hidden rounded-md bg-white">
                          {emojiPickerRuntime ? (
                            <emojiPickerRuntime.Picker
                              data={emojiPickerRuntime.data}
                              i18n={emojiPickerRuntime.i18n}
                              locale="zh"
                              navPosition="bottom"
                              onEmojiSelect={insertEmoji}
                              previewPosition="none"
                              searchPosition="top"
                              set="native"
                              skinTonePosition="none"
                              theme="light"
                            />
                          ) : (
                            <div className="grid h-[300px] w-[320px] place-items-center px-4 text-sm text-slate-500">
                              {emojiPickerLoading
                                ? '正在加载表情面板...'
                                : emojiPickerError || '打开后加载表情面板'}
                            </div>
                          )}
                        </div>
                      }
                      position="topLeft"
                      showArrow
                      trigger="custom"
                      visible={emojiPickerVisible}
                      keepDOM
                      onClickOutSide={() => setEmojiPickerVisible(false)}
                      onVisibleChange={setEmojiPickerVisible}
                    >
                      <Button
                        aria-label="选择表情"
                        disabled={saving || !selectedConversation}
                        icon={<Smile size={16} />}
                        onClick={() => setEmojiPickerVisible((visible) => !visible)}
                      >
                        表情
                      </Button>
                    </Popover>
                    <Button
                      disabled={!canSubmitReply}
                      icon={<Send size={16} />}
                      loading={saving || replyImageUploading}
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

interface EmojiSelection {
  native?: unknown;
  skins?: unknown;
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

function statusFilterCount(
  conversations: SupportConversation[],
  filter: SupportStatusFilter,
) {
  if (filter === 'all') {
    return conversations.length;
  }
  return conversations.filter((conversation) => conversation.status === filter).length;
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

function supportMessageToChatMessage(message: SupportMessage): SupportChatMessage {
  const createAt = Date.parse(message.createdAt);
  const messageType = message.messageType ?? 'text';
  return {
    authorName: message.authorName,
    authorText: authorText(message.author),
    content: message.content,
    createAt: Number.isNaN(createAt) ? undefined : createAt,
    createdAtLabel: message.createdAt,
    id: message.id,
    imageUrl: message.imageUrl ?? undefined,
    messageType,
    role: chatRoleForAuthor(message.author),
    status: 'complete',
  };
}

function chatRoleForAuthor(author: SupportMessageAuthor): SupportChatRole {
  if (author === 'admin') {
    return 'user';
  }
  if (author === 'system') {
    return 'system';
  }
  return 'assistant';
}

function chatAvatarText(message?: SupportChatMessage) {
  if (message?.authorText === '客服') {
    return '客';
  }
  if (message?.authorText === '系统') {
    return '系';
  }
  return '用';
}

function chatAvatarColor(message?: SupportChatMessage): 'blue' | 'grey' | 'teal' {
  if (message?.authorText === '客服') {
    return 'teal';
  }
  if (message?.authorText === '系统') {
    return 'grey';
  }
  return 'blue';
}

function authorText(author: SupportMessageAuthor) {
  if (author === 'admin') {
    return '客服';
  }
  if (author === 'system') {
    return '系统';
  }
  return '用户';
}

function nativeEmojiFromSelection(selection: unknown) {
  if (!isRecord(selection)) {
    return '';
  }
  const emoji = selection as EmojiSelection;
  if (typeof emoji.native === 'string') {
    return emoji.native;
  }
  if (Array.isArray(emoji.skins)) {
    for (const skin of emoji.skins) {
      if (isRecord(skin) && typeof skin.native === 'string') {
        return skin.native;
      }
    }
  }
  return '';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function setUpdateFormValue<K extends keyof UpdateFormState>(
  setForm: (updater: (current: UpdateFormState) => UpdateFormState) => void,
  key: K,
  value: UpdateFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}
