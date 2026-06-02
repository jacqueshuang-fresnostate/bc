import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { Lock, Play, Plus, RefreshCcw, XCircle } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { useDraws } from '../hooks/useDraws';
import { useLotteries } from '../hooks/useLotteries';
import type { DrawMode, LotteryKind, LotteryNumberType } from '../types/dashboard';
import type {
  CreateDrawIssueRequest,
  DrawIssue,
  DrawIssueStatus,
} from '../types/draws';

interface DrawManagementPageProps {
  onDashboardRefresh: () => void;
}

interface DrawIssueFormState {
  drawNumber: string;
  issue: string;
  lotteryId: string;
  saleClosedAt: string;
  scheduledAt: string;
}

export function DrawManagementPage({ onDashboardRefresh }: DrawManagementPageProps) {
  const {
    cancel,
    close,
    create,
    draw,
    drawSources,
    error: drawError,
    issues,
    loading: drawsLoading,
    refresh: refreshDraws,
    saving,
  } = useDraws();
  const {
    error: lotteryError,
    loading: lotteriesLoading,
    lotteries,
    refresh: refreshLotteries,
  } = useLotteries();
  const [selectedIssueId, setSelectedIssueId] = useState<string | null>(null);
  const [form, setForm] = useState<DrawIssueFormState>(() => emptyForm());

  const selectedLottery = useMemo(
    () => lotteries.find((lottery) => lottery.id === form.lotteryId) ?? lotteries[0] ?? null,
    [form.lotteryId, lotteries],
  );
  const selectedIssue = useMemo(
    () => issues.find((issue) => issue.id === selectedIssueId) ?? issues[0] ?? null,
    [issues, selectedIssueId],
  );

  useEffect(() => {
    if (!form.lotteryId && lotteries[0]) {
      setForm((current) => ({ ...current, lotteryId: lotteries[0].id }));
    }
  }, [form.lotteryId, lotteries]);

  useEffect(() => {
    if (selectedIssueId && !issues.some((issue) => issue.id === selectedIssueId)) {
      setSelectedIssueId(null);
    }
  }, [issues, selectedIssueId]);

  const refreshAll = () => {
    refreshDraws();
    refreshLotteries();
  };

  const createIssue = async () => {
    if (!selectedLottery) {
      return;
    }
    const payload: CreateDrawIssueRequest = {
      issue: form.issue.trim(),
      lotteryId: selectedLottery.id,
      saleClosedAt: form.saleClosedAt.trim(),
      scheduledAt: form.scheduledAt.trim(),
    };
    const created = await create(payload);
    setSelectedIssueId(created.id);
    onDashboardRefresh();
  };

  const closeIssue = async (issue: DrawIssue) => {
    const closed = await close(issue.id);
    setSelectedIssueId(closed.id);
    onDashboardRefresh();
  };

  const drawIssue = async (issue: DrawIssue) => {
    const payload =
      issue.drawMode === 'manual'
        ? { drawNumber: form.drawNumber.trim() }
        : {};
    const drawn = await draw(issue.id, payload);
    setSelectedIssueId(drawn.id);
    setForm((current) => ({ ...current, drawNumber: '' }));
    onDashboardRefresh();
  };

  const cancelIssue = async (issue: DrawIssue) => {
    const cancelled = await cancel(issue.id);
    setSelectedIssueId(cancelled.id);
    onDashboardRefresh();
  };

  const loading = drawsLoading || lotteriesLoading;
  const error = drawError ?? lotteryError;

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">开奖期号与开奖源</h1>
          <p className="mt-1 text-sm text-slate-500">
            维护期号、封盘状态和开奖结果，开奖后结果会保留在后端内存仓储。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="开奖接口错误" description={error} /> : null}

      <section className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        {drawSources.map((source) => (
          <Card key={source.id} className="rounded-md border border-line">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0">
                <h2 className="truncate text-base font-semibold text-ink">{source.name}</h2>
                <div className="mt-1 text-xs text-slate-400">{source.id}</div>
              </div>
              <Tag color={drawModeColor(source.mode)}>{drawModeText(source.mode)}</Tag>
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              {source.reusableForLotteryIds.map((lotteryId) => (
                <Tag key={lotteryId} color="grey">
                  {lotteryName(lotteryId, lotteries)}
                </Tag>
              ))}
            </div>
          </Card>
        ))}
      </section>

      <section className="grid gap-4 xl:grid-cols-[1fr_420px]">
        <Card className="rounded-md border border-line">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold text-ink">期号列表</h2>
            <Tag color="cyan">{issues.length} 个期号</Tag>
          </div>
          {loading ? (
            <div className="grid min-h-[300px] place-items-center">
              <Spin tip="正在加载期开奖数据" />
            </div>
          ) : issues.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[980px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">期号</th>
                    <th className="py-2 pr-4 font-medium">彩种</th>
                    <th className="py-2 pr-4 font-medium">号码类型</th>
                    <th className="py-2 pr-4 font-medium">开奖模式</th>
                    <th className="py-2 pr-4 font-medium">封盘/开奖</th>
                    <th className="py-2 pr-4 font-medium">结果</th>
                    <th className="py-2 pr-4 font-medium">状态</th>
                    <th className="py-2 pr-4 font-medium">操作</th>
                  </tr>
                </thead>
                <tbody>
                  {issues.map((issue) => (
                    <tr
                      key={issue.id}
                      className={`border-b border-slate-100 ${
                        selectedIssue?.id === issue.id ? 'bg-teal-50/60' : ''
                      }`}
                    >
                      <td className="py-3 pr-4">
                        <button
                          className="text-left font-semibold text-accent"
                          type="button"
                          onClick={() => setSelectedIssueId(issue.id)}
                        >
                          {issue.issue}
                        </button>
                        <div className="mt-1 text-xs text-slate-400">{issue.id}</div>
                      </td>
                      <td className="py-3 pr-4">
                        <div className="font-medium text-ink">{issue.lotteryName}</div>
                        <div className="mt-1 text-xs text-slate-400">{issue.lotteryId}</div>
                      </td>
                      <td className="py-3 pr-4 text-slate-600">
                        {numberTypeText(issue.numberType)}
                      </td>
                      <td className="py-3 pr-4">
                        <Tag color={drawModeColor(issue.drawMode)}>
                          {drawModeText(issue.drawMode)}
                        </Tag>
                      </td>
                      <td className="py-3 pr-4 text-slate-600">
                        <div>{issue.saleClosedAt}</div>
                        <div className="mt-1 text-xs text-slate-400">{issue.scheduledAt}</div>
                      </td>
                      <td className="py-3 pr-4">
                        {issue.drawNumber ? (
                          <span className="font-mono text-base font-semibold text-ink">
                            {issue.drawNumber}
                          </span>
                        ) : (
                          <span className="text-slate-400">未开奖</span>
                        )}
                      </td>
                      <td className="py-3 pr-4">
                        <Tag color={statusColor(issue.status)}>
                          {statusText(issue.status)}
                        </Tag>
                      </td>
                      <td className="py-3 pr-4">
                        <div className="flex flex-wrap gap-2">
                          <Button
                            disabled={saving || issue.status !== 'open'}
                            icon={<Lock size={14} />}
                            size="small"
                            onClick={() => void closeIssue(issue)}
                          >
                            封盘
                          </Button>
                          <Button
                            disabled={
                              saving ||
                              issue.status === 'drawn' ||
                              issue.status === 'cancelled'
                            }
                            icon={<Play size={14} />}
                            size="small"
                            theme={selectedIssue?.id === issue.id ? 'solid' : 'light'}
                            onClick={() => setSelectedIssueId(issue.id)}
                          >
                            开奖
                          </Button>
                          <Button
                            disabled={saving || !canCancel(issue.status)}
                            icon={<XCircle size={14} />}
                            size="small"
                            onClick={() => void cancelIssue(issue)}
                          >
                            取消
                          </Button>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="rounded-md border border-line p-4 text-sm text-slate-500">
              暂无期号，先在右侧创建一期开奖结果期号。
            </div>
          )}
        </Card>

        <div className="space-y-4">
          <Card className="rounded-md border border-line">
            <div className="mb-4">
              <h2 className="text-base font-semibold text-ink">创建期号</h2>
              <p className="mt-1 text-sm text-slate-500">
                创建后可在列表中封盘、开奖或取消。
              </p>
            </div>

            <form
              className="space-y-4"
              onSubmit={(event) => {
                event.preventDefault();
              }}
            >
              <Field label="彩种">
                <select
                  className="form-input"
                  value={selectedLottery?.id ?? ''}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      lotteryId: event.target.value,
                    }))
                  }
                >
                  {lotteries.map((lottery) => (
                    <option key={lottery.id} value={lottery.id}>
                      {lottery.name}（{drawModeText(lottery.drawMode)}）
                    </option>
                  ))}
                </select>
              </Field>

              <Field label="期号">
                <input
                  className="form-input"
                  value={form.issue}
                  onChange={(event) => setFormValue(setForm, 'issue', event.target.value)}
                />
              </Field>

              <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-1">
                <Field label="封盘时间">
                  <input
                    className="form-input"
                    value={form.saleClosedAt}
                    onChange={(event) =>
                      setFormValue(setForm, 'saleClosedAt', event.target.value)
                    }
                  />
                </Field>
                <Field label="开奖时间">
                  <input
                    className="form-input"
                    value={form.scheduledAt}
                    onChange={(event) =>
                      setFormValue(setForm, 'scheduledAt', event.target.value)
                    }
                  />
                </Field>
              </div>

              <Button
                disabled={!selectedLottery || saving}
                icon={<Plus size={16} />}
                theme="solid"
                onClick={() => void createIssue()}
              >
                {saving ? '处理中' : '创建期号'}
              </Button>
            </form>
          </Card>

          <Card className="rounded-md border border-line">
            <div className="mb-4 flex items-start justify-between gap-3">
              <div>
                <h2 className="text-base font-semibold text-ink">执行开奖</h2>
                <p className="mt-1 text-sm text-slate-500">
                  手动开奖需要录入号码，平台/API 开奖由后端生成。
                </p>
              </div>
              {selectedIssue ? (
                <Tag color={statusColor(selectedIssue.status)}>
                  {statusText(selectedIssue.status)}
                </Tag>
              ) : null}
            </div>

            {selectedIssue ? (
              <div className="space-y-4">
                <IssueSummary issue={selectedIssue} />

                {selectedIssue.drawMode === 'manual' ? (
                  <Field label={`开奖号码（${numberTypeText(selectedIssue.numberType)}）`}>
                    <input
                      className="form-input font-mono"
                      maxLength={selectedIssue.numberType === 'threeDigit' ? 3 : 5}
                      value={form.drawNumber}
                      onChange={(event) =>
                        setFormValue(setForm, 'drawNumber', event.target.value)
                      }
                    />
                  </Field>
                ) : (
                  <div className="rounded-md bg-slate-50 p-3 text-sm text-slate-600">
                    当前期号将使用{drawModeText(selectedIssue.drawMode)}生成开奖号码。
                  </div>
                )}

                <div className="flex flex-wrap gap-2">
                  <Button
                    disabled={saving || selectedIssue.status !== 'open'}
                    icon={<Lock size={14} />}
                    onClick={() => void closeIssue(selectedIssue)}
                  >
                    封盘
                  </Button>
                  <Button
                    disabled={
                      saving ||
                      selectedIssue.status === 'drawn' ||
                      selectedIssue.status === 'cancelled'
                    }
                    icon={<Play size={14} />}
                    theme="solid"
                    onClick={() => void drawIssue(selectedIssue)}
                  >
                    开奖
                  </Button>
                  <Button
                    disabled={saving || !canCancel(selectedIssue.status)}
                    icon={<XCircle size={14} />}
                    onClick={() => void cancelIssue(selectedIssue)}
                  >
                    取消
                  </Button>
                </div>
              </div>
            ) : (
              <div className="rounded-md border border-line p-4 text-sm text-slate-500">
                暂无可操作期号。
              </div>
            )}
          </Card>
        </div>
      </section>
    </div>
  );
}

interface FieldProps {
  children: ReactNode;
  label: string;
}

function Field({ children, label }: FieldProps) {
  return (
    <label className="block text-sm font-medium text-slate-600">
      <span className="mb-1 block">{label}</span>
      {children}
    </label>
  );
}

function IssueSummary({ issue }: { issue: DrawIssue }) {
  return (
    <div className="rounded-md bg-slate-50 p-3 text-sm text-slate-600">
      <div className="font-semibold text-ink">{issue.lotteryName}</div>
      <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1">
        <span>{issue.issue}</span>
        <span>{numberTypeText(issue.numberType)}</span>
        <span>{drawModeText(issue.drawMode)}</span>
      </div>
      {issue.drawNumber ? (
        <div className="mt-2 font-mono text-lg font-semibold text-ink">
          {issue.drawNumber}
        </div>
      ) : null}
    </div>
  );
}

function emptyForm(): DrawIssueFormState {
  return {
    drawNumber: '',
    issue: '20260602001',
    lotteryId: '',
    saleClosedAt: '2026-06-02 20:59:45',
    scheduledAt: '2026-06-02 21:00:15',
  };
}

function setFormValue<K extends keyof DrawIssueFormState>(
  setForm: Dispatch<SetStateAction<DrawIssueFormState>>,
  key: K,
  value: DrawIssueFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function lotteryName(id: string, lotteries: LotteryKind[]) {
  return lotteries.find((lottery) => lottery.id === id)?.name ?? id;
}

function drawModeText(mode: DrawMode) {
  const labels: Record<DrawMode, string> = {
    api: 'API 开奖',
    manual: '手动开奖',
    platform: '平台开奖',
  };
  return labels[mode];
}

function drawModeColor(mode: DrawMode) {
  if (mode === 'api') {
    return 'blue';
  }
  if (mode === 'manual') {
    return 'orange';
  }
  return 'green';
}

function numberTypeText(numberType: LotteryNumberType) {
  return numberType === 'threeDigit' ? '3 位号码' : '5 位号码';
}

function statusText(status: DrawIssueStatus) {
  const labels: Record<DrawIssueStatus, string> = {
    cancelled: '已取消',
    closed: '已封盘',
    drawn: '已开奖',
    open: '销售中',
  };
  return labels[status];
}

function statusColor(status: DrawIssueStatus) {
  if (status === 'cancelled') {
    return 'grey';
  }
  if (status === 'closed') {
    return 'orange';
  }
  if (status === 'drawn') {
    return 'green';
  }
  return 'blue';
}

function canCancel(status: DrawIssueStatus) {
  return status === 'open' || status === 'closed';
}
