import { Banner, Button, Card, SideSheet, Spin, Tag } from '@douyinfe/semi-ui';
import {
  Activity,
  CalendarPlus,
  Clock3,
  Edit3,
  History,
  ListPlus,
  Lock,
  Play,
  Plus,
  RefreshCcw,
  Save,
  Search,
  Trash2,
  XCircle,
} from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { MetricCard } from '../components/MetricCard';
import { useDrawScheduler } from '../hooks/useDrawScheduler';
import { useDraws } from '../hooks/useDraws';
import { useLotteries } from '../hooks/useLotteries';
import type {
  DrawMode,
  DrawSource,
  DrawSourceProvider,
  LotteryKind,
  LotteryNumberType,
  SaveDrawSourceRequest,
} from '../types/dashboard';
import type {
  CreateDrawIssueRequest,
  DrawAutomationRun,
  DrawIssue,
  DrawIssueGenerationPreview,
  DrawIssueStatus,
} from '../types/draws';
import type {
  DrawSchedulerRunRecord,
  DrawSchedulerRunStatus,
  DrawSchedulerStatus,
  DrawSchedulerConfig,
} from '../types/scheduler';

interface DrawManagementPageProps {
  onDashboardRefresh: () => void;
}

type DrawManagementSection = 'automation' | 'issues' | 'sources';

interface DrawIssueFormState {
  drawNumber: string;
  issue: string;
  lotteryId: string;
  saleClosedAt: string;
  scheduledAt: string;
}

interface DrawSourceFormState {
  endpoint: string;
  id: string;
  lotCode: string;
  name: string;
  provider: DrawSourceProvider;
  reusableForLotteryIds: string[];
}

interface SchedulerConfigFormState {
  enabled: boolean;
  futureIssueCount: string;
  intervalSeconds: string;
  saleCloseLeadSeconds: string;
}

export function DrawManagementPage({ onDashboardRefresh }: DrawManagementPageProps) {
  const {
    cancel,
    close,
    create,
    createSource,
    deleteSource,
    draw,
    drawSources,
    error: drawError,
    generateBatch,
    generateNext,
    issues,
    loading: drawsLoading,
    previewGeneration,
    refresh: refreshDraws,
    runAutomation,
    saving,
    updateSource,
  } = useDraws();
  const {
    error: lotteryError,
    loading: lotteriesLoading,
    lotteries,
    refresh: refreshLotteries,
  } = useLotteries();
  const {
    error: schedulerError,
    loading: schedulerLoading,
    refresh: refreshScheduler,
    saveConfig: saveSchedulerConfigRequest,
    saving: schedulerSaving,
    status: schedulerStatus,
  } = useDrawScheduler();
  const [section, setSection] = useState<DrawManagementSection>('issues');
  const [selectedIssueId, setSelectedIssueId] = useState<string | null>(null);
  const [automationNow, setAutomationNow] = useState(() => currentDateTimeLabel());
  const [automationResult, setAutomationResult] =
    useState<DrawAutomationRun | null>(null);
  const [generationCount, setGenerationCount] = useState('5');
  const [generationPreview, setGenerationPreview] = useState<
    DrawIssueGenerationPreview[]
  >([]);
  const [form, setForm] = useState<DrawIssueFormState>(() => emptyForm());
  const [selectedSourceId, setSelectedSourceId] = useState<string | null>(null);
  const [sourceForm, setSourceForm] =
    useState<DrawSourceFormState>(() => emptySourceForm());
  const [schedulerConfigForm, setSchedulerConfigForm] =
    useState<SchedulerConfigFormState>(() => emptySchedulerConfigForm());
  const [createIssueSheetVisible, setCreateIssueSheetVisible] = useState(false);
  const [drawIssueSheetVisible, setDrawIssueSheetVisible] = useState(false);
  const [sourceSheetVisible, setSourceSheetVisible] = useState(false);
  const [schedulerSheetVisible, setSchedulerSheetVisible] = useState(false);

  const selectedLottery = useMemo(
    () => lotteries.find((lottery) => lottery.id === form.lotteryId) ?? lotteries[0] ?? null,
    [form.lotteryId, lotteries],
  );
  const selectedIssue = useMemo(
    () => issues.find((issue) => issue.id === selectedIssueId) ?? issues[0] ?? null,
    [issues, selectedIssueId],
  );
  const selectedSource = useMemo(
    () =>
      selectedSourceId
        ? drawSources.find((source) => source.id === selectedSourceId) ?? null
        : null,
    [drawSources, selectedSourceId],
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

  useEffect(() => {
    if (schedulerStatus) {
      setSchedulerConfigForm(configFormFromStatus(schedulerStatus));
    }
  }, [schedulerStatus]);

  useEffect(() => {
    if (selectedSource?.editable) {
      setSelectedSourceId(selectedSource.id);
      setSourceForm(sourceFormFromSource(selectedSource));
    } else if (selectedSource) {
      setSourceForm(emptySourceForm());
    } else if (!selectedSourceId) {
      setSourceForm(emptySourceForm());
    }
  }, [selectedSource, selectedSourceId]);

  useEffect(() => {
    setCreateIssueSheetVisible(false);
    setDrawIssueSheetVisible(false);
    setSourceSheetVisible(false);
    setSchedulerSheetVisible(false);
  }, [section]);

  const overview = useMemo(
    () => drawOverview(issues, drawSources, schedulerStatus),
    [drawSources, issues, schedulerStatus],
  );

  const refreshAll = () => {
    refreshDraws();
    refreshLotteries();
    refreshScheduler();
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
    setCreateIssueSheetVisible(false);
    onDashboardRefresh();
  };

  const startCreateSource = () => {
    setSelectedSourceId(null);
    setSourceForm(emptySourceForm());
    setSourceSheetVisible(true);
  };

  const editDrawSourceConfig = (source: DrawSource) => {
    setSelectedSourceId(source.id);
    setSourceForm(sourceFormFromSource(source));
    setSourceSheetVisible(true);
  };

  const openDrawIssueSheet = (issue: DrawIssue) => {
    setSelectedIssueId(issue.id);
    setDrawIssueSheetVisible(true);
  };

  const saveDrawSourceConfig = async () => {
    const payload = sourcePayload(sourceForm);
    const saved =
      selectedSource?.editable && selectedSource.id === payload.id
        ? await updateSource(selectedSource.id, payload)
        : await createSource(payload);
    setSelectedSourceId(saved.id);
    setSourceSheetVisible(false);
    refreshDraws();
    onDashboardRefresh();
  };

  const deleteDrawSourceConfig = async () => {
    if (!selectedSource?.editable) {
      return;
    }
    await deleteSource(selectedSource.id);
    setSelectedSourceId(null);
    setSourceForm(emptySourceForm());
    setSourceSheetVisible(false);
    refreshDraws();
    onDashboardRefresh();
  };

  const generateNextIssue = async () => {
    if (!selectedLottery) {
      return;
    }
    const created = await generateNext({
      lotteryId: selectedLottery.id,
      now: automationNow.trim(),
    });
    setGenerationPreview([]);
    setSelectedIssueId(created.id);
    setCreateIssueSheetVisible(false);
    refreshScheduler();
    onDashboardRefresh();
  };

  const previewIssueGeneration = async () => {
    if (!selectedLottery) {
      return;
    }
    const count = parseGenerationCount(generationCount);
    if (!count) {
      return;
    }

    const plans = await previewGeneration({
      count,
      lotteryId: selectedLottery.id,
      now: automationNow.trim(),
    });
    setGenerationPreview(plans);
  };

  const generateIssueBatch = async () => {
    if (!selectedLottery) {
      return;
    }
    const count = parseGenerationCount(generationCount);
    if (!count) {
      return;
    }

    const created = await generateBatch({
      count,
      lotteryId: selectedLottery.id,
      now: automationNow.trim(),
    });
    setGenerationPreview([]);
    if (created[0]) {
      setSelectedIssueId(created[0].id);
    }
    setCreateIssueSheetVisible(false);
    refreshScheduler();
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
    setDrawIssueSheetVisible(false);
    onDashboardRefresh();
  };

  const cancelIssue = async (issue: DrawIssue) => {
    const cancelled = await cancel(issue.id);
    setSelectedIssueId(cancelled.id);
    setDrawIssueSheetVisible(false);
    onDashboardRefresh();
  };

  const runDueAutomation = async () => {
    const result = await runAutomation({ now: automationNow.trim() });
    setAutomationResult(result);
    const focusIssue = result.drawnIssues[0] ?? result.closedIssues[0] ?? null;
    if (focusIssue) {
      setSelectedIssueId(focusIssue.id);
    }
    refreshScheduler();
    onDashboardRefresh();
  };

  const saveSchedulerConfig = async () => {
    await saveSchedulerConfigRequest(schedulerConfigPayload(schedulerConfigForm));
    setSchedulerSheetVisible(false);
    refreshScheduler();
    onDashboardRefresh();
  };

  const loading = drawsLoading || lotteriesLoading;
  const error = drawError ?? lotteryError ?? schedulerError;
  const generationCountValue = parseGenerationCount(generationCount);
  const generationActionDisabled =
    !selectedLottery || saving || !automationNow.trim() || !generationCountValue;

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

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="期号总数"
          trend={`${overview.openCount} 个销售中，${overview.closedCount} 个已封盘`}
          value={`${issues.length}`}
        />
        <MetricCard
          label="待开奖"
          trend="销售中或已封盘"
          value={`${overview.pendingCount}`}
        />
        <MetricCard
          label="已开奖"
          trend={`取消 ${overview.cancelledCount} 期`}
          value={`${overview.drawnCount}`}
        />
        <MetricCard
          label="开奖源"
          trend={`${overview.apiSourceCount} 个 API，调度${
            overview.schedulerEnabled ? '已启用' : '未启用'
          }`}
          value={`${drawSources.length}`}
        />
      </section>

      <SectionTabs active={section} onChange={setSection} />

      {section === 'issues' ? (
        <IssueManagementSection
          issues={issues}
          loading={loading}
          saving={saving}
          selectedIssue={selectedIssue}
          onCancelIssue={(issue) => void cancelIssue(issue)}
          onCloseIssue={(issue) => void closeIssue(issue)}
          onCreateIssue={() => setCreateIssueSheetVisible(true)}
          onOpenDraw={openDrawIssueSheet}
          onSelectIssue={setSelectedIssueId}
        />
      ) : null}

      {section === 'sources' ? (
        <SourceManagementSection
          drawSources={drawSources}
          lotteries={lotteries}
          selectedSource={selectedSource}
          onCreate={startCreateSource}
          onEdit={editDrawSourceConfig}
          onSelect={setSelectedSourceId}
        />
      ) : null}

      {section === 'automation' ? (
        <AutomationManagementSection
          automationNow={automationNow}
          automationResult={automationResult}
          saving={saving}
          schedulerLoading={schedulerLoading}
          schedulerStatus={schedulerStatus}
          onAutomationNowChange={setAutomationNow}
          onConfigureScheduler={() => setSchedulerSheetVisible(true)}
          onRunAutomation={() => void runDueAutomation()}
        />
      ) : null}

      <CreateIssueSideSheet
        automationNow={automationNow}
        form={form}
        generationActionDisabled={generationActionDisabled}
        generationCount={generationCount}
        generationCountValue={generationCountValue}
        generationPreview={generationPreview}
        lotteries={lotteries}
        saving={saving}
        selectedLottery={selectedLottery}
        visible={createIssueSheetVisible}
        onAutomationNowChange={setAutomationNow}
        onClose={() => setCreateIssueSheetVisible(false)}
        onCreateIssue={() => void createIssue()}
        onFormChange={setForm}
        onGenerateBatch={() => void generateIssueBatch()}
        onGenerateNext={() => void generateNextIssue()}
        onGenerationCountChange={setGenerationCount}
        onPreview={() => void previewIssueGeneration()}
      />

      <DrawIssueSideSheet
        form={form}
        issue={selectedIssue}
        saving={saving}
        visible={drawIssueSheetVisible}
        onCancelIssue={(issue) => void cancelIssue(issue)}
        onClose={() => setDrawIssueSheetVisible(false)}
        onCloseIssue={(issue) => void closeIssue(issue)}
        onDrawIssue={(issue) => void drawIssue(issue)}
        onFormChange={setForm}
      />

      <DrawSourceSideSheet
        form={sourceForm}
        lotteries={lotteries}
        saving={saving}
        selectedSource={selectedSource}
        visible={sourceSheetVisible}
        onClose={() => setSourceSheetVisible(false)}
        onDelete={() => void deleteDrawSourceConfig()}
        onFormChange={setSourceForm}
        onReset={() => {
          if (selectedSource?.editable) {
            setSourceForm(sourceFormFromSource(selectedSource));
          }
        }}
        onSave={() => void saveDrawSourceConfig()}
      />

      <SchedulerConfigSideSheet
        form={schedulerConfigForm}
        saving={schedulerSaving}
        visible={schedulerSheetVisible}
        onChange={setSchedulerConfigForm}
        onClose={() => setSchedulerSheetVisible(false)}
        onSubmit={() => void saveSchedulerConfig()}
      />
    </div>
  );
}

const DRAW_MANAGEMENT_SECTIONS: Array<{
  key: DrawManagementSection;
  label: string;
  summary: string;
}> = [
  { key: 'issues', label: '期号管理', summary: '创建、封盘、开奖和取消' },
  { key: 'sources', label: '开奖源配置', summary: 'API 来源和彩种复用' },
  { key: 'automation', label: '自动任务与调度', summary: '补期、开奖、结算和历史' },
];

function SectionTabs({
  active,
  onChange,
}: {
  active: DrawManagementSection;
  onChange: (section: DrawManagementSection) => void;
}) {
  return (
    <section className="flex flex-wrap gap-2">
      {DRAW_MANAGEMENT_SECTIONS.map((item) => (
        <Button
          key={item.key}
          theme={active === item.key ? 'solid' : 'light'}
          onClick={() => onChange(item.key)}
        >
          <span className="inline-flex flex-col items-start leading-tight">
            <span>{item.label}</span>
            <span className="text-xs opacity-70">{item.summary}</span>
          </span>
        </Button>
      ))}
    </section>
  );
}

function IssueManagementSection({
  issues,
  loading,
  onCancelIssue,
  onCloseIssue,
  onCreateIssue,
  onOpenDraw,
  onSelectIssue,
  saving,
  selectedIssue,
}: {
  issues: DrawIssue[];
  loading: boolean;
  onCancelIssue: (issue: DrawIssue) => void;
  onCloseIssue: (issue: DrawIssue) => void;
  onCreateIssue: () => void;
  onOpenDraw: (issue: DrawIssue) => void;
  onSelectIssue: (id: string) => void;
  saving: boolean;
  selectedIssue: DrawIssue | null;
}) {
  return (
    <Card className="rounded-md border border-line">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-base font-semibold text-ink">期号列表</h2>
          <p className="mt-1 text-sm text-slate-500">
            按期号查看封盘、开奖和取消状态。
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Tag color="cyan">{issues.length} 个期号</Tag>
          <Button icon={<Plus size={16} />} theme="solid" onClick={onCreateIssue}>
            创建期号
          </Button>
        </div>
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
                      onClick={() => onSelectIssue(issue.id)}
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
                    <Tag color={statusColor(issue.status)}>{statusText(issue.status)}</Tag>
                  </td>
                  <td className="py-3 pr-4">
                    <div className="flex flex-wrap gap-2">
                      <Button
                        disabled={saving || issue.status !== 'open'}
                        icon={<Lock size={14} />}
                        size="small"
                        onClick={() => onCloseIssue(issue)}
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
                        onClick={() => onOpenDraw(issue)}
                      >
                        开奖
                      </Button>
                      <Button
                        disabled={saving || !canCancel(issue.status)}
                        icon={<XCircle size={14} />}
                        size="small"
                        onClick={() => onCancelIssue(issue)}
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
          暂无期号，点击“创建期号”新增一期开奖结果期号。
        </div>
      )}
    </Card>
  );
}

function SourceManagementSection({
  drawSources,
  lotteries,
  onCreate,
  onEdit,
  onSelect,
  selectedSource,
}: {
  drawSources: DrawSource[];
  lotteries: LotteryKind[];
  onCreate: () => void;
  onEdit: (source: DrawSource) => void;
  onSelect: (id: string) => void;
  selectedSource: DrawSource | null;
}) {
  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <h2 className="text-base font-semibold text-ink">开奖源配置</h2>
          <p className="mt-1 text-sm text-slate-500">
            API 源可绑定多个 API 开奖彩种复用。
          </p>
        </div>
        <Button icon={<Plus size={16} />} theme="solid" onClick={onCreate}>
          新建来源
        </Button>
      </div>
      <div className="grid gap-3 md:grid-cols-2 2xl:grid-cols-3">
        {drawSources.map((source) => (
          <Card
            key={source.id}
            className={`rounded-md border ${
              selectedSource?.id === source.id ? 'border-accent' : 'border-line'
            }`}
          >
            <div className="flex items-start justify-between gap-3">
              <button
                className="min-w-0 text-left"
                type="button"
                onClick={() => onSelect(source.id)}
              >
                <h3 className="truncate text-base font-semibold text-ink">{source.name}</h3>
                <div className="mt-1 text-xs text-slate-400">{source.id}</div>
              </button>
              <Tag color={drawModeColor(source.mode)}>{drawModeText(source.mode)}</Tag>
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              {source.reusableForLotteryIds.map((lotteryId) => (
                <Tag key={lotteryId} color="grey">
                  {lotteryName(lotteryId, lotteries)}
                </Tag>
              ))}
            </div>
            {source.provider ? (
              <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-slate-500">
                <div>
                  <span className="text-slate-400">供应商</span>
                  <div className="mt-1 font-medium text-ink">
                    {drawSourceProviderText(source.provider)}
                  </div>
                </div>
                <div>
                  <span className="text-slate-400">lotCode</span>
                  <div className="mt-1 font-mono font-medium text-ink">
                    {source.lotCode ?? '-'}
                  </div>
                </div>
              </div>
            ) : null}
            <div className="mt-4 flex justify-end">
              <Button
                disabled={!source.editable}
                icon={<Edit3 size={15} />}
                size="small"
                onClick={() => onEdit(source)}
              >
                {source.editable ? '编辑' : '只读'}
              </Button>
            </div>
          </Card>
        ))}
      </div>
    </section>
  );
}

function AutomationManagementSection({
  automationNow,
  automationResult,
  onAutomationNowChange,
  onConfigureScheduler,
  onRunAutomation,
  saving,
  schedulerLoading,
  schedulerStatus,
}: {
  automationNow: string;
  automationResult: DrawAutomationRun | null;
  onAutomationNowChange: (value: string) => void;
  onConfigureScheduler: () => void;
  onRunAutomation: () => void;
  saving: boolean;
  schedulerLoading: boolean;
  schedulerStatus: DrawSchedulerStatus | null;
}) {
  return (
    <section className="grid gap-4 xl:grid-cols-[420px_1fr]">
      <Card className="rounded-md border border-line">
        <div className="mb-4 flex items-start justify-between gap-3">
          <div>
            <h2 className="text-base font-semibold text-ink">自动任务</h2>
            <p className="mt-1 text-sm text-slate-500">
              按时间执行封盘、开奖、结算和派奖入账。
            </p>
          </div>
          <Tag color="blue">执行器</Tag>
        </div>
        <div className="space-y-4">
          <Field label="执行时间">
            <input
              className="form-input"
              value={automationNow}
              onChange={(event) => onAutomationNowChange(event.target.value)}
            />
          </Field>
          <Button
            disabled={saving || !automationNow.trim()}
            icon={<Clock3 size={16} />}
            theme="solid"
            onClick={onRunAutomation}
          >
            {saving ? '处理中' : '运行自动任务'}
          </Button>
          {automationResult ? <AutomationResultSummary run={automationResult} /> : null}
        </div>
      </Card>

      <Card className="rounded-md border border-line">
        <div className="mb-4 flex items-start justify-between gap-3">
          <div>
            <div className="flex items-center gap-2">
              <Activity size={17} className="text-accent" />
              <h2 className="text-base font-semibold text-ink">常驻调度</h2>
            </div>
            <p className="mt-1 text-sm text-slate-500">
              服务启动后按配置自动补期并执行到期任务。
            </p>
          </div>
          <div className="flex flex-wrap justify-end gap-2">
            {schedulerStatus ? (
              <Tag color={schedulerStatus.enabled ? 'green' : 'grey'}>
                {schedulerStatus.enabled ? '已启用' : '未启用'}
              </Tag>
            ) : null}
            <Button
              icon={<Save size={15} />}
              size="small"
              onClick={onConfigureScheduler}
            >
              配置
            </Button>
          </div>
        </div>
        <SchedulerStatusSummary loading={schedulerLoading} status={schedulerStatus} />
      </Card>
    </section>
  );
}

function CreateIssueSideSheet({
  automationNow,
  form,
  generationActionDisabled,
  generationCount,
  generationCountValue,
  generationPreview,
  lotteries,
  onAutomationNowChange,
  onClose,
  onCreateIssue,
  onFormChange,
  onGenerateBatch,
  onGenerateNext,
  onGenerationCountChange,
  onPreview,
  saving,
  selectedLottery,
  visible,
}: {
  automationNow: string;
  form: DrawIssueFormState;
  generationActionDisabled: boolean;
  generationCount: string;
  generationCountValue: number | null;
  generationPreview: DrawIssueGenerationPreview[];
  lotteries: LotteryKind[];
  onAutomationNowChange: (value: string) => void;
  onClose: () => void;
  onCreateIssue: () => void;
  onFormChange: Dispatch<SetStateAction<DrawIssueFormState>>;
  onGenerateBatch: () => void;
  onGenerateNext: () => void;
  onGenerationCountChange: (value: string) => void;
  onPreview: () => void;
  saving: boolean;
  selectedLottery: LotteryKind | null;
  visible: boolean;
}) {
  return (
    <SideSheet
      aria-label="创建期号"
      title="创建期号"
      visible={visible}
      width={560}
      onCancel={onClose}
    >
      <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
        <Field label="彩种">
          <select
            className="form-input"
            value={selectedLottery?.id ?? ''}
            onChange={(event) =>
              onFormChange((current) => ({
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
            onChange={(event) => setFormValue(onFormChange, 'issue', event.target.value)}
          />
        </Field>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="封盘时间">
            <input
              className="form-input"
              value={form.saleClosedAt}
              onChange={(event) =>
                setFormValue(onFormChange, 'saleClosedAt', event.target.value)
              }
            />
          </Field>
          <Field label="开奖时间">
            <input
              className="form-input"
              value={form.scheduledAt}
              onChange={(event) =>
                setFormValue(onFormChange, 'scheduledAt', event.target.value)
              }
            />
          </Field>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="计划基准时间">
            <input
              className="form-input"
              value={automationNow}
              onChange={(event) => onAutomationNowChange(event.target.value)}
            />
          </Field>
          <Field label="预生成数量">
            <input
              className="form-input"
              max={50}
              min={1}
              type="number"
              value={generationCount}
              onChange={(event) => onGenerationCountChange(event.target.value)}
            />
            {!generationCountValue ? (
              <span className="mt-1 block text-xs text-amber-600">
                数量需要在 1 到 50 之间。
              </span>
            ) : null}
          </Field>
        </div>

        <div className="flex flex-wrap gap-2">
          <Button
            disabled={!selectedLottery || saving}
            icon={<Plus size={16} />}
            theme="solid"
            onClick={onCreateIssue}
          >
            {saving ? '处理中' : '创建期号'}
          </Button>
          <Button
            disabled={!selectedLottery || saving || !automationNow.trim()}
            icon={<CalendarPlus size={16} />}
            onClick={onGenerateNext}
          >
            按计划生成下一期
          </Button>
          <Button
            disabled={generationActionDisabled}
            icon={<Search size={16} />}
            onClick={onPreview}
          >
            预览计划
          </Button>
          <Button
            disabled={generationActionDisabled}
            icon={<ListPlus size={16} />}
            onClick={onGenerateBatch}
          >
            批量生成
          </Button>
        </div>

        {generationPreview.length > 0 ? (
          <GenerationPreviewList plans={generationPreview} />
        ) : null}
      </form>
    </SideSheet>
  );
}

function DrawIssueSideSheet({
  form,
  issue,
  onCancelIssue,
  onClose,
  onCloseIssue,
  onDrawIssue,
  onFormChange,
  saving,
  visible,
}: {
  form: DrawIssueFormState;
  issue: DrawIssue | null;
  onCancelIssue: (issue: DrawIssue) => void;
  onClose: () => void;
  onCloseIssue: (issue: DrawIssue) => void;
  onDrawIssue: (issue: DrawIssue) => void;
  onFormChange: Dispatch<SetStateAction<DrawIssueFormState>>;
  saving: boolean;
  visible: boolean;
}) {
  return (
    <SideSheet
      aria-label="执行开奖"
      title="执行开奖"
      visible={visible}
      width={520}
      onCancel={onClose}
    >
      {issue ? (
        <div className="space-y-4">
          <IssueSummary issue={issue} />

          {issue.drawMode === 'manual' ? (
            <Field label={`开奖号码（${numberTypeText(issue.numberType)}）`}>
              <input
                className="form-input font-mono"
                maxLength={issue.numberType === 'threeDigit' ? 5 : 9}
                placeholder={issue.numberType === 'threeDigit' ? '2,4,7' : '7,8,9,4,2'}
                value={form.drawNumber}
                onChange={(event) =>
                  setFormValue(onFormChange, 'drawNumber', event.target.value)
                }
              />
            </Field>
          ) : (
            <div className="rounded-md bg-slate-50 p-3 text-sm text-slate-600">
              当前期号将使用{drawModeText(issue.drawMode)}
              {issue.drawMode === 'api'
                ? '按开奖源配置拉取开奖号码。'
                : '生成开奖号码。'}
            </div>
          )}

          <div className="flex flex-wrap gap-2">
            <Button
              disabled={saving || issue.status !== 'open'}
              icon={<Lock size={14} />}
              onClick={() => onCloseIssue(issue)}
            >
              封盘
            </Button>
            <Button
              disabled={saving || issue.status === 'drawn' || issue.status === 'cancelled'}
              icon={<Play size={14} />}
              theme="solid"
              onClick={() => onDrawIssue(issue)}
            >
              开奖
            </Button>
            <Button
              disabled={saving || !canCancel(issue.status)}
              icon={<XCircle size={14} />}
              onClick={() => onCancelIssue(issue)}
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
    </SideSheet>
  );
}

function DrawSourceSideSheet({
  form,
  lotteries,
  onClose,
  onDelete,
  onFormChange,
  onReset,
  onSave,
  saving,
  selectedSource,
  visible,
}: {
  form: DrawSourceFormState;
  lotteries: LotteryKind[];
  onClose: () => void;
  onDelete: () => void;
  onFormChange: Dispatch<SetStateAction<DrawSourceFormState>>;
  onReset: () => void;
  onSave: () => void;
  saving: boolean;
  selectedSource: DrawSource | null;
  visible: boolean;
}) {
  return (
    <SideSheet
      aria-label="开奖源维护"
      title={selectedSource?.editable ? '编辑开奖源' : '新建开奖源'}
      visible={visible}
      width={560}
      onCancel={onClose}
    >
      <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="来源 ID">
            <input
              className="form-input font-mono"
              disabled={Boolean(selectedSource?.editable)}
              value={form.id}
              onChange={(event) =>
                setSourceFormValue(onFormChange, 'id', event.target.value)
              }
            />
          </Field>
          <Field label="来源名称">
            <input
              className="form-input"
              value={form.name}
              onChange={(event) =>
                setSourceFormValue(onFormChange, 'name', event.target.value)
              }
            />
          </Field>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="供应商">
            <select
              className="form-input"
              value={form.provider}
              onChange={(event) =>
                setSourceFormValue(
                  onFormChange,
                  'provider',
                  event.target.value as DrawSourceProvider,
                )
              }
            >
              <option value="api68">API68</option>
            </select>
          </Field>
          <Field label="lotCode">
            <input
              className="form-input font-mono"
              value={form.lotCode}
              onChange={(event) =>
                setSourceFormValue(onFormChange, 'lotCode', event.target.value)
              }
            />
          </Field>
        </div>

        <Field label="endpoint">
          <input
            className="form-input"
            value={form.endpoint}
            onChange={(event) =>
              setSourceFormValue(onFormChange, 'endpoint', event.target.value)
            }
          />
        </Field>

        <Field label="复用彩种">
          <div className="grid gap-2 sm:grid-cols-2">
            {lotteries
              .filter((lottery) => lottery.drawMode === 'api')
              .map((lottery) => (
                <label
                  key={lottery.id}
                  className="flex min-h-10 items-center gap-2 rounded border border-line px-3 py-2 text-sm text-slate-700"
                >
                  <input
                    checked={form.reusableForLotteryIds.includes(lottery.id)}
                    className="h-4 w-4 rounded border-line text-teal-600"
                    type="checkbox"
                    onChange={() => toggleSourceLottery(onFormChange, lottery.id)}
                  />
                  <span className="min-w-0 truncate">{lottery.name}</span>
                  <span className="font-mono text-xs text-slate-400">{lottery.id}</span>
                </label>
              ))}
          </div>
        </Field>

        <div className="flex flex-wrap gap-2">
          <Button
            disabled={saving}
            icon={<Save size={15} />}
            loading={saving}
            theme="solid"
            onClick={onSave}
          >
            保存来源
          </Button>
          {selectedSource?.editable ? (
            <>
              <Button disabled={saving} icon={<Edit3 size={15} />} onClick={onReset}>
                还原
              </Button>
              <Button disabled={saving} icon={<Trash2 size={15} />} onClick={onDelete}>
                删除
              </Button>
            </>
          ) : null}
        </div>
      </form>
    </SideSheet>
  );
}

function SchedulerConfigSideSheet({
  form,
  onChange,
  onClose,
  onSubmit,
  saving,
  visible,
}: {
  form: SchedulerConfigFormState;
  onChange: Dispatch<SetStateAction<SchedulerConfigFormState>>;
  onClose: () => void;
  onSubmit: () => void;
  saving: boolean;
  visible: boolean;
}) {
  return (
    <SideSheet
      aria-label="调度配置"
      title="调度配置"
      visible={visible}
      width={520}
      onCancel={onClose}
    >
      <SchedulerConfigForm
        form={form}
        saving={saving}
        onChange={onChange}
        onSubmit={onSubmit}
      />
    </SideSheet>
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

function AutomationResultSummary({ run }: { run: DrawAutomationRun }) {
  return (
    <div className="rounded-md bg-slate-50 p-3 text-sm text-slate-600">
      <div className="font-medium text-ink">{run.now}</div>
      <div className="mt-3 grid grid-cols-2 gap-2">
        <ResultMetric label="封盘" value={`${run.closedIssues.length} 期`} />
        <ResultMetric label="开奖" value={`${run.drawnIssues.length} 期`} />
        <ResultMetric label="结算" value={`${run.settlementRuns.length} 批`} />
        <ResultMetric label="入账" value={`${run.ledgerEntries.length} 笔`} />
      </div>
      {run.skippedIssues.length > 0 ? (
        <div className="mt-3 space-y-2">
          {run.skippedIssues.map((issue) => (
            <div
              key={`${issue.drawIssueId}-${issue.reason}`}
              className="rounded border border-amber-200 bg-amber-50 px-2 py-1 text-xs text-amber-700"
            >
              {issue.issue}：{issue.reason}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function SchedulerStatusSummary({
  loading,
  status,
}: {
  loading: boolean;
  status: DrawSchedulerStatus | null;
}) {
  if (loading) {
    return (
      <div className="grid min-h-36 place-items-center">
        <Spin tip="正在加载调度状态" />
      </div>
    );
  }

  if (!status) {
    return (
      <div className="rounded-md border border-line p-3 text-sm text-slate-500">
        暂无调度状态。
      </div>
    );
  }

  const recentRuns = status.recentRuns.slice(0, 5);

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-2">
        <ResultMetric label="执行周期" value={`${status.config.intervalSeconds} 秒`} />
        <ResultMetric label="未来期号" value={`${status.config.futureIssueCount} 期`} />
        <ResultMetric
          label="封盘提前"
          value={`${status.config.saleCloseLeadSeconds} 秒`}
        />
        <ResultMetric label="保留历史" value={`${status.runCount} 条`} />
      </div>

      {status.lastRun ? (
        <SchedulerLastRunSummary run={status.lastRun} />
      ) : (
        <div className="rounded-md border border-line p-3 text-sm text-slate-500">
          暂无调度运行历史。
        </div>
      )}

      {recentRuns.length > 0 ? <SchedulerRunHistory runs={recentRuns} /> : null}
    </div>
  );
}

function SchedulerConfigForm({
  form,
  onChange,
  onSubmit,
  saving,
}: {
  form: SchedulerConfigFormState;
  onChange: Dispatch<SetStateAction<SchedulerConfigFormState>>;
  onSubmit: () => void;
  saving: boolean;
}) {
  return (
    <div className="border-t border-line pt-4">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="text-sm font-medium text-ink">调度配置</div>
        <Button
          disabled={saving}
          icon={<Save size={15} />}
          loading={saving}
          size="small"
          theme="solid"
          onClick={onSubmit}
        >
          保存配置
        </Button>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="启用调度">
          <label className="inline-flex h-10 items-center gap-2 text-sm text-slate-700">
            <input
              checked={form.enabled}
              className="h-4 w-4 rounded border-line text-teal-600"
              type="checkbox"
              onChange={(event) =>
                setSchedulerConfigFormValue(
                  onChange,
                  'enabled',
                  event.currentTarget.checked,
                )
              }
            />
            {form.enabled ? '已启用' : '已关闭'}
          </label>
        </Field>
        <Field label="执行周期（秒）">
          <input
            className="form-input"
            min="1"
            type="number"
            value={form.intervalSeconds}
            onChange={(event) =>
              setSchedulerConfigFormValue(
                onChange,
                'intervalSeconds',
                event.target.value,
              )
            }
          />
        </Field>
        <Field label="未来期号缓冲">
          <input
            className="form-input"
            max="50"
            min="1"
            type="number"
            value={form.futureIssueCount}
            onChange={(event) =>
              setSchedulerConfigFormValue(
                onChange,
                'futureIssueCount',
                event.target.value,
              )
            }
          />
        </Field>
        <Field label="封盘提前（秒）">
          <input
            className="form-input"
            min="1"
            type="number"
            value={form.saleCloseLeadSeconds}
            onChange={(event) =>
              setSchedulerConfigFormValue(
                onChange,
                'saleCloseLeadSeconds',
                event.target.value,
              )
            }
          />
        </Field>
      </div>
    </div>
  );
}

function SchedulerLastRunSummary({ run }: { run: DrawSchedulerRunRecord }) {
  return (
    <div className="rounded-md bg-slate-50 p-3 text-sm text-slate-600">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="font-medium text-ink">{run.finishedAt}</div>
          <div className="mt-1 truncate text-xs text-slate-400">
            {run.id} · {schedulerTriggerText(run.trigger)} · {run.now}
          </div>
        </div>
        <Tag color={schedulerRunStatusColor(run.status)}>
          {schedulerRunStatusText(run.status)}
        </Tag>
      </div>

      <div className="mt-3 grid grid-cols-2 gap-2">
        <ResultMetric label="补期" value={`${run.generatedIssueCount} 期`} />
        <ResultMetric label="封盘" value={`${run.closedIssueCount} 期`} />
        <ResultMetric label="开奖" value={`${run.drawnIssueCount} 期`} />
        <ResultMetric label="结算" value={`${run.settlementRunCount} 批`} />
        <ResultMetric label="入账" value={`${run.ledgerEntryCount} 笔`} />
        <ResultMetric
          label="跳过"
          value={`${run.skippedIssueCount + run.skippedLotteryCount} 项`}
        />
      </div>

      {run.error ? (
        <div className="mt-3 rounded border border-red-200 bg-red-50 px-2 py-1 text-xs text-red-600">
          {run.error}
        </div>
      ) : null}
    </div>
  );
}

function SchedulerRunHistory({ runs }: { runs: DrawSchedulerRunRecord[] }) {
  return (
    <div className="rounded-md border border-line p-3">
      <div className="mb-2 flex items-center gap-2 text-sm font-medium text-ink">
        <History size={15} className="text-accent" />
        最近运行
      </div>
      <div className="max-h-48 overflow-y-auto">
        <table className="w-full text-left text-xs">
          <thead className="border-b border-slate-200 text-slate-500">
            <tr>
              <th className="py-2 pr-2 font-medium">时间</th>
              <th className="py-2 pr-2 font-medium">结果</th>
              <th className="py-2 font-medium">摘要</th>
            </tr>
          </thead>
          <tbody>
            {runs.map((run) => (
              <tr key={run.id} className="border-b border-slate-200 last:border-0">
                <td className="py-2 pr-2 align-top text-slate-500">
                  <div>{run.finishedAt}</div>
                  <div className="mt-1 text-slate-400">{run.id}</div>
                </td>
                <td className="py-2 pr-2 align-top">
                  <Tag color={schedulerRunStatusColor(run.status)}>
                    {schedulerRunStatusText(run.status)}
                  </Tag>
                </td>
                <td className="py-2 align-top text-slate-500">
                  补期 {run.generatedIssueCount}，开奖 {run.drawnIssueCount}，入账{' '}
                  {run.ledgerEntryCount}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function GenerationPreviewList({
  plans,
}: {
  plans: DrawIssueGenerationPreview[];
}) {
  return (
    <div className="rounded-md border border-line bg-slate-50 p-3">
      <div className="mb-2 flex items-center justify-between gap-2 text-sm">
        <span className="font-medium text-ink">计划预览</span>
        <Tag color="cyan">{plans.length} 期</Tag>
      </div>
      <div className="max-h-56 overflow-y-auto">
        <table className="w-full text-left text-xs">
          <thead className="border-b border-slate-200 text-slate-500">
            <tr>
              <th className="py-2 pr-2 font-medium">期号</th>
              <th className="py-2 pr-2 font-medium">封盘</th>
              <th className="py-2 font-medium">开奖</th>
            </tr>
          </thead>
          <tbody>
            {plans.map((plan) => (
              <tr key={plan.issue} className="border-b border-slate-200 last:border-0">
                <td className="py-2 pr-2 font-mono text-ink">{plan.issue}</td>
                <td className="py-2 pr-2 text-slate-500">{plan.saleClosedAt}</td>
                <td className="py-2 text-slate-500">{plan.scheduledAt}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function ResultMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded border border-slate-200 bg-white px-2 py-2">
      <div className="text-xs text-slate-400">{label}</div>
      <div className="mt-1 font-semibold text-ink">{value}</div>
    </div>
  );
}

function drawOverview(
  issues: DrawIssue[],
  drawSources: DrawSource[],
  schedulerStatus: DrawSchedulerStatus | null,
) {
  const openCount = issues.filter((issue) => issue.status === 'open').length;
  const closedCount = issues.filter((issue) => issue.status === 'closed').length;
  const drawnCount = issues.filter((issue) => issue.status === 'drawn').length;
  const cancelledCount = issues.filter((issue) => issue.status === 'cancelled').length;
  const apiSourceCount = drawSources.filter((source) => source.mode === 'api').length;

  return {
    apiSourceCount,
    cancelledCount,
    closedCount,
    drawnCount,
    openCount,
    pendingCount: openCount + closedCount,
    schedulerEnabled: schedulerStatus?.enabled ?? false,
  };
}

function emptyForm(): DrawIssueFormState {
  const scheduledAt = new Date(Date.now() + 60 * 60 * 1000);
  const saleClosedAt = new Date(scheduledAt.getTime() - 30 * 1000);

  return {
    drawNumber: '',
    issue: '',
    lotteryId: '',
    saleClosedAt: currentDateTimeLabel(saleClosedAt),
    scheduledAt: currentDateTimeLabel(scheduledAt),
  };
}

function emptySourceForm(): DrawSourceFormState {
  return {
    endpoint: 'https://api.api68.com/QuanGuoCai/getLotteryInfoList.do',
    id: 'api68-custom',
    lotCode: '10041',
    name: 'API68 自定义来源',
    provider: 'api68',
    reusableForLotteryIds: [],
  };
}

function sourceFormFromSource(source: DrawSource): DrawSourceFormState {
  return {
    endpoint: source.endpoint ?? '',
    id: source.id,
    lotCode: source.lotCode ?? '',
    name: source.name,
    provider: source.provider ?? 'api68',
    reusableForLotteryIds: source.reusableForLotteryIds,
  };
}

function sourcePayload(form: DrawSourceFormState): SaveDrawSourceRequest {
  return {
    endpoint: form.endpoint.trim() || null,
    id: form.id.trim(),
    lotCode: form.lotCode.trim(),
    name: form.name.trim(),
    provider: form.provider,
    reusableForLotteryIds: form.reusableForLotteryIds,
  };
}

function emptySchedulerConfigForm(): SchedulerConfigFormState {
  return {
    enabled: false,
    futureIssueCount: '1',
    intervalSeconds: '60',
    saleCloseLeadSeconds: '30',
  };
}

function configFormFromStatus(status: DrawSchedulerStatus): SchedulerConfigFormState {
  return {
    enabled: status.config.enabled,
    futureIssueCount: String(status.config.futureIssueCount),
    intervalSeconds: String(status.config.intervalSeconds),
    saleCloseLeadSeconds: String(status.config.saleCloseLeadSeconds),
  };
}

function schedulerConfigPayload(
  form: SchedulerConfigFormState,
): DrawSchedulerConfig {
  return {
    enabled: form.enabled,
    futureIssueCount: numberField(form.futureIssueCount),
    intervalSeconds: numberField(form.intervalSeconds),
    saleCloseLeadSeconds: numberField(form.saleCloseLeadSeconds),
  };
}

function currentDateTimeLabel(value = new Date()) {
  const pad = (value: number) => value.toString().padStart(2, '0');

  return [
    `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())}`,
    `${pad(value.getHours())}:${pad(value.getMinutes())}:${pad(value.getSeconds())}`,
  ].join(' ');
}

function parseGenerationCount(value: string) {
  const count = Number.parseInt(value, 10);
  if (!Number.isFinite(count) || count < 1 || count > 50) {
    return null;
  }
  return count;
}

function numberField(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function setFormValue<K extends keyof DrawIssueFormState>(
  setForm: Dispatch<SetStateAction<DrawIssueFormState>>,
  key: K,
  value: DrawIssueFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function setSourceFormValue<K extends keyof DrawSourceFormState>(
  setForm: Dispatch<SetStateAction<DrawSourceFormState>>,
  key: K,
  value: DrawSourceFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function toggleSourceLottery(
  setForm: Dispatch<SetStateAction<DrawSourceFormState>>,
  lotteryId: string,
) {
  setForm((current) => {
    const exists = current.reusableForLotteryIds.includes(lotteryId);
    return {
      ...current,
      reusableForLotteryIds: exists
        ? current.reusableForLotteryIds.filter((id) => id !== lotteryId)
        : [...current.reusableForLotteryIds, lotteryId],
    };
  });
}

function setSchedulerConfigFormValue<K extends keyof SchedulerConfigFormState>(
  setForm: Dispatch<SetStateAction<SchedulerConfigFormState>>,
  key: K,
  value: SchedulerConfigFormState[K],
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

function drawSourceProviderText(provider: DrawSourceProvider) {
  const labels: Record<DrawSourceProvider, string> = {
    api68: 'API68',
  };
  return labels[provider];
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

function schedulerRunStatusText(status: DrawSchedulerRunStatus) {
  return status === 'success' ? '成功' : '失败';
}

function schedulerRunStatusColor(status: DrawSchedulerRunStatus) {
  return status === 'success' ? 'green' : 'red';
}

function schedulerTriggerText(trigger: DrawSchedulerRunRecord['trigger']) {
  return trigger === 'automatic' ? '自动运行' : trigger;
}
