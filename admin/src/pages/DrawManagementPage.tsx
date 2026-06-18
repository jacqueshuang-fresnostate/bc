import {
  Input,
  Banner,
  Button,
  Card,
  Select,
  SideSheet,
  Spin,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import {
  Activity,
  CalendarPlus,
  Clock3,
  Edit3,
  Eye,
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
import { PageControls } from '../components/PageControls';
import { useDrawScheduler } from '../hooks/useDrawScheduler';
import { useDraws } from '../hooks/useDraws';
import { useLotteries } from '../hooks/useLotteries';
import type {
  DrawMode,
  DrawSource,
  DrawSourceProvider,
  LotteryKind,
  SaveDrawSourceRequest,
} from '../types/dashboard';
import type {
  ApiDrawSourceCrawlSnapshot,
  ApiDrawSourceSnapshotRequestKind,
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
import {
  drawNumberInputMeta,
  lotteryNumberTypeText as numberTypeText,
} from '../utils/lotteries';
import { formatDateTime } from '../utils/format';

interface DrawManagementPageProps {
  onDashboardRefresh: () => void;
}

type DrawManagementSection = 'automation' | 'issues' | 'snapshots' | 'sources';

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

const DRAW_SOURCE_PRESETS: Array<{ label: string; form: DrawSourceFormState }> = [
  {
    label: '印尼5分彩采集',
    form: {
      endpoint: 'https://draw.indonesia-lottery.org/others/draw.php',
      id: 'indonesia-id5',
      lotCode: 'IDFFC5',
      name: '印尼开奖 印尼5分彩',
      provider: 'indonesiaLottery',
      reusableForLotteryIds: ['id5'],
    },
  },
  {
    label: '河内5分彩采集',
    form: {
      endpoint:
        'https://www.bbkaijiang.com/api/st-lottery-open/open-result/list-newest-result',
      id: 'bb-hn5',
      lotCode: 'VIFFC5',
      name: 'BB开奖 河内5分彩',
      provider: 'bbKaijiang',
      reusableForLotteryIds: ['hn5'],
    },
  },
  {
    label: '腾讯分分彩采集',
    form: {
      endpoint: 'https://kjapi.net/hall/hallajax/getLotteryInfo',
      id: 'kj-txffc',
      lotCode: 'txffc',
      name: 'KJAPI-腾讯分分彩',
      provider: 'kjApi',
      reusableForLotteryIds: ['txffc'],
    },
  },
  {
    label: '澳洲幸运5采集',
    form: {
      endpoint: 'https://api.api68.com/CQShiCai/getBaseCQShiCai.do',
      id: 'api68-au5',
      lotCode: '10010',
      name: 'API68 澳洲幸运5',
      provider: 'api68',
      reusableForLotteryIds: ['au5'],
    },
  },
  {
    label: '福彩 3D/排列 3 采集',
    form: {
      endpoint: 'https://api.api68.com/QuanGuoCai/getLotteryInfoList.do',
      id: 'api68-fc3d',
      lotCode: '10041',
      name: 'API68 福彩 3D/排列 3',
      provider: 'api68',
      reusableForLotteryIds: ['fc3d', 'pl3'],
    },
  },
  ...[
    ['bjpk10', '北京PK10', '10001', 'https://api.api68.com/pks/getLotteryPksInfo.do'],
    ['tjssc', '天津时时彩', '10003', 'https://api.api68.com/CQShiCai/getBaseCQShiCai.do'],
    ['xjssc', '新疆时时彩', '10004', 'https://api.api68.com/CQShiCai/getBaseCQShiCai.do'],
    ['gd11x5', '广东11选5', '10006', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['au10', '澳洲幸运10', '10012', 'https://api.api68.com/pks/getLotteryPksInfo.do'],
    ['au20', '澳洲幸运20', '10013', 'https://api.api68.com/LuckTwenty/getBaseLuckTewnty.do'],
    ['jx11x5', '江西11选5', '10015', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['js11x5', '江苏11选5', '10016', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['ah11x5', '安徽11选5', '10017', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['sh11x5', '上海11选5', '10018', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['ln11x5', '辽宁11选5', '10019', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['hb11x5', '湖北11选5', '10020', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['gx11x5', '广西11选5', '10022', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['jl11x5', '吉林11选5', '10023', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['nmg11x5', '内蒙古11选5', '10024', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
    ['zj11x5', '浙江11选5', '10025', 'https://api.api68.com/ElevenFive/getElevenFiveInfo.do'],
  ].map(([lotteryId, lotteryName, lotCode, endpoint]) => ({
    label: `${lotteryName}采集`,
    form: {
      endpoint,
      id: `api68-${lotteryId}`,
      lotCode,
      name: `API68 ${lotteryName}`,
      provider: 'api68' as DrawSourceProvider,
      reusableForLotteryIds: [lotteryId],
    },
  })),
];

export function DrawManagementPage({ onDashboardRefresh }: DrawManagementPageProps) {
  const {
    cancel,
    close,
    create,
    clearSnapshots,
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
    refreshSnapshots,
    refreshSnapshotsWithFilter,
    refreshWithFilter,
    issuePage,
    runAutomation,
    snapshotError,
    snapshotLoading,
    snapshotPage,
    snapshotPageSize,
    snapshots,
    snapshotTotalCount,
    snapshotTotalPages,
    totalCount: issueTotalCount,
    totalPages: issueTotalPages,
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
  const [issueLotteryFilter, setIssueLotteryFilter] = useState('');
  const [issueCurrentPage, setIssueCurrentPage] = useState(1);
  const [issueCurrentPageSize, setIssueCurrentPageSize] = useState(20);
  const [snapshotLotteryFilter, setSnapshotLotteryFilter] = useState('');
  const [snapshotSourceFilter, setSnapshotSourceFilter] = useState('');
  const [snapshotRequestKindFilter, setSnapshotRequestKindFilter] =
    useState<ApiDrawSourceSnapshotRequestKind | 'all'>('all');
  const [snapshotSuccessFilter, setSnapshotSuccessFilter] =
    useState<'all' | 'failed' | 'success'>('all');
  const [snapshotIssueFilter, setSnapshotIssueFilter] = useState('');
  const [snapshotCurrentPage, setSnapshotCurrentPage] = useState(1);
  const [snapshotCurrentPageSize, setSnapshotCurrentPageSize] = useState(20);
  const [selectedSnapshot, setSelectedSnapshot] =
    useState<ApiDrawSourceCrawlSnapshot | null>(null);
  const [snapshotDetailVisible, setSnapshotDetailVisible] = useState(false);

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
    if (issueLotteryFilter && !lotteries.some((lottery) => lottery.id === issueLotteryFilter)) {
      handleIssueLotteryFilterChange('');
    }
  }, [issueLotteryFilter, lotteries]);

  useEffect(() => {
    refreshWithFilter({
      lotteryId: issueLotteryFilter || undefined,
      page: issueCurrentPage,
      pageSize: issueCurrentPageSize,
    });
  }, [issueCurrentPage, issueCurrentPageSize, issueLotteryFilter, refreshWithFilter]);

  useEffect(() => {
    if (
      snapshotLotteryFilter &&
      !lotteries.some((lottery) => lottery.id === snapshotLotteryFilter)
    ) {
      handleSnapshotLotteryFilterChange('');
    }
  }, [lotteries, snapshotLotteryFilter]);

  useEffect(() => {
    if (
      snapshotSourceFilter &&
      !drawSources.some((source) => source.id === snapshotSourceFilter)
    ) {
      handleSnapshotSourceFilterChange('');
    }
  }, [drawSources, snapshotSourceFilter]);

  useEffect(() => {
    refreshSnapshotsWithFilter({
      issue: snapshotIssueFilter.trim() || undefined,
      lotteryId: snapshotLotteryFilter || undefined,
      page: snapshotCurrentPage,
      pageSize: snapshotCurrentPageSize,
      requestKind:
        snapshotRequestKindFilter === 'all'
          ? undefined
          : snapshotRequestKindFilter,
      sourceId: snapshotSourceFilter || undefined,
      success:
        snapshotSuccessFilter === 'all'
          ? undefined
          : snapshotSuccessFilter === 'success',
    });
  }, [
    refreshSnapshotsWithFilter,
    snapshotCurrentPage,
    snapshotCurrentPageSize,
    snapshotIssueFilter,
    snapshotLotteryFilter,
    snapshotRequestKindFilter,
    snapshotSourceFilter,
    snapshotSuccessFilter,
  ]);

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
    setSnapshotDetailVisible(false);
  }, [section]);

  const overview = useMemo(
    () => drawOverview(issues, drawSources, schedulerStatus),
    [drawSources, issues, schedulerStatus],
  );

  const issueFilterOptions = useMemo(
    () =>
      lotteries
        .slice()
        .sort((left, right) => left.name.localeCompare(right.name, 'zh-CN'))
        .map((lottery) => ({
          value: lottery.id,
          label: `${lottery.name}（${numberTypeText(lottery.numberType)}）`,
        })),
    [lotteries],
  );
  const snapshotSourceOptions = useMemo(
    () =>
      drawSources
        .filter((source) => source.mode === 'api')
        .slice()
        .sort((left, right) => left.name.localeCompare(right.name, 'zh-CN'))
        .map((source) => ({
          value: source.id,
          label: source.name,
        })),
    [drawSources],
  );

  const handleIssueLotteryFilterChange = (lotteryId: string) => {
    setIssueLotteryFilter(lotteryId);
    setIssueCurrentPage(1);
  };

  const handleIssuePageChange = (nextPage: number) => {
    const safeNextPage = Math.max(1, Math.min(nextPage, issueTotalPages || 1));
    setIssueCurrentPage(safeNextPage);
  };

  const handleIssuePageSizeChange = (nextPageSize: number) => {
    setIssueCurrentPage(1);
    setIssueCurrentPageSize(nextPageSize);
  };

  const handleSnapshotLotteryFilterChange = (lotteryId: string) => {
    setSnapshotLotteryFilter(lotteryId);
    setSnapshotCurrentPage(1);
  };

  const handleSnapshotSourceFilterChange = (sourceId: string) => {
    setSnapshotSourceFilter(sourceId);
    setSnapshotCurrentPage(1);
  };

  const handleSnapshotRequestKindChange = (
    requestKind: ApiDrawSourceSnapshotRequestKind | 'all',
  ) => {
    setSnapshotRequestKindFilter(requestKind);
    setSnapshotCurrentPage(1);
  };

  const handleSnapshotSuccessFilterChange = (
    successFilter: 'all' | 'failed' | 'success',
  ) => {
    setSnapshotSuccessFilter(successFilter);
    setSnapshotCurrentPage(1);
  };

  const handleSnapshotIssueFilterChange = (issue: string) => {
    setSnapshotIssueFilter(issue);
    setSnapshotCurrentPage(1);
  };

  const handleSnapshotPageChange = (nextPage: number) => {
    const safeNextPage = Math.max(1, Math.min(nextPage, snapshotTotalPages || 1));
    setSnapshotCurrentPage(safeNextPage);
  };

  const handleSnapshotPageSizeChange = (nextPageSize: number) => {
    setSnapshotCurrentPage(1);
    setSnapshotCurrentPageSize(nextPageSize);
  };

  const openSnapshotDetail = (snapshot: ApiDrawSourceCrawlSnapshot) => {
    setSelectedSnapshot(snapshot);
    setSnapshotDetailVisible(true);
  };

  const clearSnapshotRecords = async () => {
    if (
      !window.confirm(
        '确定一键清除全部 API 开奖源采集快照吗？该操作只删除第三方接口抓取审计记录，不会影响开奖源配置、开奖期号、调度或开奖结果。',
      )
    ) {
      return;
    }

    try {
      const result = await clearSnapshots();
      setSelectedSnapshot(null);
      setSnapshotDetailVisible(false);
      setSnapshotCurrentPage(1);
      Toast.success(`已清除 ${result.deletedCount} 条 API 采集快照`);
    } catch {
      Toast.error('API 采集快照清除失败，请查看接口错误提示');
    }
  };

  const refreshAll = () => {
    refreshDraws();
    refreshSnapshots();
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

  const toggleSchedulerEnabled = async (enabled: boolean) => {
    if (!schedulerStatus) {
      return;
    }

    const nextStatus = await saveSchedulerConfigRequest({
      ...schedulerStatus.config,
      enabled,
    });
    setSchedulerConfigForm(configFormFromStatus(nextStatus));
    refreshScheduler();
    onDashboardRefresh();
  };

  const loading = drawsLoading || lotteriesLoading;
  const error = drawError ?? lotteryError ?? schedulerError ?? snapshotError;
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
          value={`${issueTotalCount}`}
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
          lotteryFilter={issueLotteryFilter}
          lotteryFilterOptions={issueFilterOptions}
          issues={issues}
          loading={loading}
          saving={saving}
          selectedIssue={selectedIssue}
          onIssueLotteryFilterChange={handleIssueLotteryFilterChange}
          onIssuePageChange={handleIssuePageChange}
          onIssuePageSizeChange={handleIssuePageSizeChange}
          page={issuePage}
          pageSize={issueCurrentPageSize}
          totalCount={issueTotalCount}
          totalPages={issueTotalPages}
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

      {section === 'snapshots' ? (
        <ApiSnapshotSection
          issueFilter={snapshotIssueFilter}
          loading={snapshotLoading}
          lotteryFilter={snapshotLotteryFilter}
          lotteryFilterOptions={issueFilterOptions}
          onClearSnapshots={() => void clearSnapshotRecords()}
          page={snapshotPage}
          pageSize={snapshotPageSize || snapshotCurrentPageSize}
          requestKindFilter={snapshotRequestKindFilter}
          saving={saving}
          snapshots={snapshots}
          sourceFilter={snapshotSourceFilter}
          sourceFilterOptions={snapshotSourceOptions}
          successFilter={snapshotSuccessFilter}
          totalCount={snapshotTotalCount}
          totalPages={snapshotTotalPages}
          lotteries={lotteries}
          onIssueFilterChange={handleSnapshotIssueFilterChange}
          onLotteryFilterChange={handleSnapshotLotteryFilterChange}
          onOpenDetail={openSnapshotDetail}
          onPageChange={handleSnapshotPageChange}
          onPageSizeChange={handleSnapshotPageSizeChange}
          onRequestKindFilterChange={handleSnapshotRequestKindChange}
          onSourceFilterChange={handleSnapshotSourceFilterChange}
          onSuccessFilterChange={handleSnapshotSuccessFilterChange}
        />
      ) : null}

      {section === 'automation' ? (
        <AutomationManagementSection
          automationNow={automationNow}
          automationResult={automationResult}
          saving={saving}
          schedulerLoading={schedulerLoading}
          schedulerSaving={schedulerSaving}
          schedulerStatus={schedulerStatus}
          onAutomationNowChange={setAutomationNow}
          onConfigureScheduler={() => setSchedulerSheetVisible(true)}
          onRunAutomation={() => void runDueAutomation()}
          onStartScheduler={() => void toggleSchedulerEnabled(true)}
          onStopScheduler={() => void toggleSchedulerEnabled(false)}
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

      <ApiSnapshotDetailSideSheet
        snapshot={selectedSnapshot}
        visible={snapshotDetailVisible}
        onClose={() => setSnapshotDetailVisible(false)}
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
  { key: 'snapshots', label: '采集快照', summary: 'API 原始响应比对' },
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
  lotteryFilter,
  lotteryFilterOptions,
  page,
  pageSize,
  totalCount,
  totalPages,
  onCancelIssue,
  onCloseIssue,
  onCreateIssue,
  onIssuePageChange,
  onIssuePageSizeChange,
  onOpenDraw,
  onIssueLotteryFilterChange,
  onSelectIssue,
  saving,
  selectedIssue,
}: {
  issues: DrawIssue[];
  loading: boolean;
  lotteryFilter: string;
  lotteryFilterOptions: Array<{ value: string; label: string }>;
  page: number;
  pageSize: number;
  totalCount: number;
  totalPages: number;
  onCancelIssue: (issue: DrawIssue) => void;
  onCloseIssue: (issue: DrawIssue) => void;
  onCreateIssue: () => void;
  onIssuePageChange: (page: number) => void;
  onIssuePageSizeChange: (pageSize: number) => void;
  onOpenDraw: (issue: DrawIssue) => void;
  onIssueLotteryFilterChange: (lotteryId: string) => void;
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
          <Tag color="cyan">{totalCount} 个期号</Tag>
          <Select
            className="form-input min-w-[220px]"
            value={lotteryFilter}
            onChange={(value) =>
              onIssueLotteryFilterChange(String(value ?? ''))
            }
          >
            <Select.Option value="">全部玩法</Select.Option>
            {lotteryFilterOptions.map((option) => (
              <Select.Option key={option.value} value={option.value}>
                {option.label}
              </Select.Option>
            ))}
          </Select>
          <label className="text-xs text-slate-500">
            每页
            <Select
              className="ml-1 form-input min-w-[90px]"
              value={pageSize}
              onChange={(value) => onIssuePageSizeChange(Number(value ?? 10))}
            >
              {[10, 20, 50, 100].map((size) => (
                <Select.Option key={size} value={size}>
                  {size}
                </Select.Option>
              ))}
            </Select>
            条
          </label>
          <div className="flex items-center gap-2 text-xs">
            <Button
              size="small"
              disabled={loading || page <= 1 || totalPages === 0}
              onClick={() => onIssuePageChange(page - 1)}
            >
              上一页
            </Button>
            <span>
              第 {totalPages === 0 ? 0 : page} / {totalPages} 页
            </span>
            <Button
              size="small"
              disabled={loading || page >= totalPages || totalPages === 0}
              onClick={() => onIssuePageChange(page + 1)}
            >
              下一页
            </Button>
          </div>
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

function ApiSnapshotSection({
  issueFilter,
  loading,
  lotteries,
  lotteryFilter,
  lotteryFilterOptions,
  onClearSnapshots,
  onIssueFilterChange,
  onLotteryFilterChange,
  onOpenDetail,
  onPageChange,
  onPageSizeChange,
  onRequestKindFilterChange,
  onSourceFilterChange,
  onSuccessFilterChange,
  page,
  pageSize,
  requestKindFilter,
  saving,
  snapshots,
  sourceFilter,
  sourceFilterOptions,
  successFilter,
  totalCount,
  totalPages,
}: {
  issueFilter: string;
  loading: boolean;
  lotteries: LotteryKind[];
  lotteryFilter: string;
  lotteryFilterOptions: Array<{ value: string; label: string }>;
  onClearSnapshots: () => void;
  onIssueFilterChange: (value: string) => void;
  onLotteryFilterChange: (value: string) => void;
  onOpenDetail: (snapshot: ApiDrawSourceCrawlSnapshot) => void;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
  onRequestKindFilterChange: (value: ApiDrawSourceSnapshotRequestKind | 'all') => void;
  onSourceFilterChange: (value: string) => void;
  onSuccessFilterChange: (value: 'all' | 'failed' | 'success') => void;
  page: number;
  pageSize: number;
  requestKindFilter: ApiDrawSourceSnapshotRequestKind | 'all';
  saving: boolean;
  snapshots: ApiDrawSourceCrawlSnapshot[];
  sourceFilter: string;
  sourceFilterOptions: Array<{ value: string; label: string }>;
  successFilter: 'all' | 'failed' | 'success';
  totalCount: number;
  totalPages: number;
}) {
  return (
    <Card className="rounded-md border border-line">
      <div className="mb-3 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold text-ink">API 采集快照</h2>
          <p className="mt-1 text-sm text-slate-500">
            查看第三方开奖源每次读取的期号、开奖号码、成功状态和原始响应。
          </p>
        </div>
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            disabled={loading || saving || totalCount === 0}
            icon={<Trash2 size={15} />}
            loading={saving}
            type="danger"
            onClick={onClearSnapshots}
          >
            一键清除
          </Button>
          <PageControls
            loading={loading}
            page={page}
            pageSize={pageSize}
            totalCount={totalCount}
            totalPages={totalPages}
            onPageChange={onPageChange}
            onPageSizeChange={onPageSizeChange}
          />
        </div>
      </div>

      <div className="mb-3 grid gap-2 md:grid-cols-2 xl:grid-cols-5">
        <Select
          className="form-input"
          value={lotteryFilter}
          onChange={(value) => onLotteryFilterChange(String(value ?? ''))}
        >
          <Select.Option value="">全部彩种</Select.Option>
          {lotteryFilterOptions.map((option) => (
            <Select.Option key={option.value} value={option.value}>
              {option.label}
            </Select.Option>
          ))}
        </Select>
        <Select
          className="form-input"
          value={sourceFilter}
          onChange={(value) => onSourceFilterChange(String(value ?? ''))}
        >
          <Select.Option value="">全部来源</Select.Option>
          {sourceFilterOptions.map((option) => (
            <Select.Option key={option.value} value={option.value}>
              {option.label}
            </Select.Option>
          ))}
        </Select>
        <Select
          className="form-input"
          value={requestKindFilter}
          onChange={(value) =>
            onRequestKindFilterChange(
              (value ?? 'all') as ApiDrawSourceSnapshotRequestKind | 'all',
            )
          }
        >
          <Select.Option value="all">全部用途</Select.Option>
          <Select.Option value="latestIssue">最新期号</Select.Option>
          <Select.Option value="drawNumber">开奖号码</Select.Option>
        </Select>
        <Select
          className="form-input"
          value={successFilter}
          onChange={(value) =>
            onSuccessFilterChange((value ?? 'all') as 'all' | 'failed' | 'success')
          }
        >
          <Select.Option value="all">全部结果</Select.Option>
          <Select.Option value="success">成功</Select.Option>
          <Select.Option value="failed">失败</Select.Option>
        </Select>
        <Input
          className="form-input"
          placeholder="按期号筛选"
          prefix={<Search size={14} />}
          value={issueFilter}
          onChange={(value) => onIssueFilterChange(value)}
        />
      </div>

      {loading ? (
        <div className="grid min-h-[300px] place-items-center">
          <Spin tip="正在加载 API 采集快照" />
        </div>
      ) : snapshots.length > 0 ? (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[1180px] text-left text-sm">
            <thead className="border-b border-line text-xs text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">采集时间</th>
                <th className="py-2 pr-4 font-medium">彩种</th>
                <th className="py-2 pr-4 font-medium">开奖源</th>
                <th className="py-2 pr-4 font-medium">用途</th>
                <th className="py-2 pr-4 font-medium">期号对比</th>
                <th className="py-2 pr-4 font-medium">开奖号码</th>
                <th className="py-2 pr-4 font-medium">结果</th>
                <th className="py-2 pr-4 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {snapshots.map((snapshot) => (
                <tr key={snapshot.id} className="border-b border-slate-100">
                  <td className="py-3 pr-4 align-top text-slate-600">
                    <div className="font-medium text-ink">
                      {formatDateTime(snapshot.crawledAt)}
                    </div>
                    <div className="mt-1 font-mono text-xs text-slate-400">
                      {snapshot.id}
                    </div>
                  </td>
                  <td className="py-3 pr-4 align-top">
                    <div className="font-medium text-ink">
                      {lotteryName(snapshot.lotteryId, lotteries)}
                    </div>
                    <div className="mt-1 font-mono text-xs text-slate-400">
                      {snapshot.lotteryId}
                    </div>
                  </td>
                  <td className="py-3 pr-4 align-top">
                    <div className="font-medium text-ink">{snapshot.sourceName}</div>
                    <div className="mt-1 flex flex-wrap gap-1 text-xs text-slate-400">
                      <span className="font-mono">{snapshot.sourceId}</span>
                      <span>{snapshotProviderText(snapshot.provider)}</span>
                    </div>
                  </td>
                  <td className="py-3 pr-4 align-top">
                    <Tag color={snapshot.requestKind === 'drawNumber' ? 'purple' : 'cyan'}>
                      {snapshotRequestKindText(snapshot.requestKind)}
                    </Tag>
                    {snapshot.requestedIssue ? (
                      <div className="mt-1 font-mono text-xs text-slate-500">
                        请求 {snapshot.requestedIssue}
                      </div>
                    ) : null}
                  </td>
                  <td className="py-3 pr-4 align-top text-slate-600">
                    <div>最新：{snapshot.latestIssue ?? '-'}</div>
                    <div className="mt-1">下一期：{snapshot.nextIssue ?? '-'}</div>
                    {snapshot.latestDrawTime || snapshot.nextDrawTime ? (
                      <div className="mt-1 text-xs text-slate-400">
                        {snapshot.latestDrawTime ?? '-'} / {snapshot.nextDrawTime ?? '-'}
                      </div>
                    ) : null}
                  </td>
                  <td className="py-3 pr-4 align-top">
                    {snapshot.drawNumber ? (
                      <span className="font-mono text-base font-semibold text-ink">
                        {snapshot.drawNumber}
                      </span>
                    ) : (
                      <span className="text-slate-400">-</span>
                    )}
                  </td>
                  <td className="py-3 pr-4 align-top">
                    <Tag color={snapshot.success ? 'green' : 'red'}>
                      {snapshot.success ? '成功' : '失败'}
                    </Tag>
                    <div className="mt-1 text-xs text-slate-400">
                      HTTP {snapshot.httpStatus ?? '-'}
                    </div>
                    {snapshot.errorMessage ? (
                      <div className="mt-1 max-w-[220px] truncate text-xs text-red-500">
                        {snapshot.errorMessage}
                      </div>
                    ) : null}
                  </td>
                  <td className="py-3 pr-4 align-top">
                    <Button
                      icon={<Eye size={14} />}
                      size="small"
                      onClick={() => onOpenDetail(snapshot)}
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
          暂无 API 采集快照。只有启用数据库并实际请求 API 开奖源后才会产生记录。
        </div>
      )}
    </Card>
  );
}

function AutomationManagementSection({
  automationNow,
  automationResult,
  onAutomationNowChange,
  onConfigureScheduler,
  onRunAutomation,
  onStartScheduler,
  onStopScheduler,
  saving,
  schedulerLoading,
  schedulerSaving,
  schedulerStatus,
}: {
  automationNow: string;
  automationResult: DrawAutomationRun | null;
  onAutomationNowChange: (value: string) => void;
  onConfigureScheduler: () => void;
  onRunAutomation: () => void;
  onStartScheduler: () => void;
  onStopScheduler: () => void;
  saving: boolean;
  schedulerLoading: boolean;
  schedulerSaving: boolean;
  schedulerStatus: DrawSchedulerStatus | null;
}) {
  const schedulerEnabled = schedulerStatus?.enabled ?? false;
  const schedulerActionDisabled = schedulerLoading || schedulerSaving || !schedulerStatus;

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
            <Input
              className="form-input"
              value={automationNow}
              onChange={(value) => onAutomationNowChange(value)}
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
              disabled={schedulerActionDisabled || schedulerEnabled}
              icon={<Play size={15} />}
              loading={schedulerSaving && !schedulerEnabled}
              size="small"
              theme="solid"
              onClick={onStartScheduler}
            >
              启动调度
            </Button>
            <Button
              disabled={schedulerActionDisabled || !schedulerEnabled}
              icon={<XCircle size={15} />}
              loading={schedulerSaving && schedulerEnabled}
              size="small"
              type="danger"
              onClick={onStopScheduler}
            >
              关闭调度
            </Button>
            <Button
              icon={<Save size={15} />}
              loading={schedulerSaving}
              size="small"
              disabled={schedulerLoading || schedulerSaving || !schedulerStatus}
              onClick={onConfigureScheduler}
            >
              修改配置
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
          <Select
            className="form-input"
            value={selectedLottery?.id ?? ''}
            onChange={(value) =>
              onFormChange((current) => ({
                ...current,
                lotteryId: String(value ?? ''),
              }))
            }
          >
            {lotteries.map((lottery) => (
              <Select.Option key={lottery.id} value={lottery.id}>
                {lottery.name}（{drawModeText(lottery.drawMode)}）
              </Select.Option>
            ))}
          </Select>
        </Field>

        <Field label="期号">
          <Input
            className="form-input"
            value={form.issue}
            onChange={(value) => setFormValue(onFormChange, 'issue', value)}
          />
        </Field>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="封盘时间">
            <Input
              className="form-input"
              value={form.saleClosedAt}
              onChange={(value) =>
                setFormValue(onFormChange, 'saleClosedAt', value)
              }
            />
          </Field>
          <Field label="开奖时间">
            <Input
              className="form-input"
              value={form.scheduledAt}
              onChange={(value) =>
                setFormValue(onFormChange, 'scheduledAt', value)
              }
            />
          </Field>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="计划基准时间">
            <Input
              className="form-input"
              value={automationNow}
              onChange={(value) => onAutomationNowChange(value)}
            />
          </Field>
          <Field label="预生成数量">
            <Input
              className="form-input"
              max={50}
              min={1}
              type="number"
              value={generationCount}
              onChange={(value) => onGenerationCountChange(value)}
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
  const inputMeta = issue ? drawNumberInputMeta(issue.numberType) : null;

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
              <Input
                className="form-input font-mono"
                maxLength={inputMeta?.maxLength}
                placeholder={inputMeta?.placeholder}
                value={form.drawNumber}
                onChange={(value) =>
                  setFormValue(onFormChange, 'drawNumber', value)
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
        <div className="flex flex-wrap gap-2">
          {DRAW_SOURCE_PRESETS.map((preset) => (
            <Button
              key={preset.label}
              disabled={saving}
              onClick={() => onFormChange(preset.form)}
            >
              {preset.label}
            </Button>
          ))}
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="来源 ID">
            <Input
              className="form-input font-mono"
              disabled={Boolean(selectedSource?.editable)}
              value={form.id}
              onChange={(value) =>
                setSourceFormValue(onFormChange, 'id', value)
              }
            />
          </Field>
          <Field label="来源名称">
            <Input
              className="form-input"
              value={form.name}
              onChange={(value) =>
                setSourceFormValue(onFormChange, 'name', value)
              }
            />
          </Field>
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          <Field label="供应商">
            <Select
              className="form-input"
              value={form.provider}
              onChange={(value) =>
                setSourceFormValue(
                  onFormChange,
                  'provider',
                  value as DrawSourceProvider,
                )
              }
            >
              <Select.Option value="api68">API68</Select.Option>
              <Select.Option value="kjApi">KJAPI</Select.Option>
              <Select.Option value="bbKaijiang">BB开奖</Select.Option>
              <Select.Option value="indonesiaLottery">印尼开奖</Select.Option>
            </Select>
          </Field>
          <Field label="lotCode / lotKey / gameCode">
            <Input
              className="form-input font-mono"
              value={form.lotCode}
              onChange={(value) =>
                setSourceFormValue(onFormChange, 'lotCode', value)
              }
            />
          </Field>
        </div>

        <Field label="endpoint">
          <Input
            className="form-input"
            value={form.endpoint}
            onChange={(value) =>
              setSourceFormValue(onFormChange, 'endpoint', value)
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

function ApiSnapshotDetailSideSheet({
  onClose,
  snapshot,
  visible,
}: {
  onClose: () => void;
  snapshot: ApiDrawSourceCrawlSnapshot | null;
  visible: boolean;
}) {
  const rawJsonText = snapshot?.rawResponse
    ? JSON.stringify(snapshot.rawResponse, null, 2)
    : '';

  return (
    <SideSheet
      aria-label="API 采集快照详情"
      title="API 采集快照详情"
      visible={visible}
      width={720}
      onCancel={onClose}
    >
      {snapshot ? (
        <div className="space-y-4">
          <div className="grid gap-3 md:grid-cols-2">
            <SnapshotDetailItem label="采集时间" value={formatDateTime(snapshot.crawledAt)} />
            <SnapshotDetailItem label="采集结果" value={snapshot.success ? '成功' : '失败'} />
            <SnapshotDetailItem label="彩种 ID" value={snapshot.lotteryId} />
            <SnapshotDetailItem label="开奖源" value={`${snapshot.sourceName}（${snapshot.sourceId}）`} />
            <SnapshotDetailItem label="供应商" value={snapshotProviderText(snapshot.provider)} />
            <SnapshotDetailItem label="用途" value={snapshotRequestKindText(snapshot.requestKind)} />
            <SnapshotDetailItem label="请求期号" value={snapshot.requestedIssue ?? '-'} />
            <SnapshotDetailItem label="开奖号码" value={snapshot.drawNumber ?? '-'} />
            <SnapshotDetailItem label="最新期号" value={snapshot.latestIssue ?? '-'} />
            <SnapshotDetailItem label="下一期期号" value={snapshot.nextIssue ?? '-'} />
            <SnapshotDetailItem label="最新开奖时间" value={snapshot.latestDrawTime ?? '-'} />
            <SnapshotDetailItem label="下一期开奖时间" value={snapshot.nextDrawTime ?? '-'} />
            <SnapshotDetailItem label="HTTP 状态" value={String(snapshot.httpStatus ?? '-')} />
            <SnapshotDetailItem label="开奖源编码" value={snapshot.lotCode || '-'} />
          </div>

          <Field label="请求地址">
            <div className="break-all rounded border border-line bg-slate-50 p-3 font-mono text-xs text-slate-600">
              {snapshot.endpoint}
            </div>
          </Field>

          {snapshot.errorMessage ? (
            <Field label="错误信息">
              <div className="rounded border border-red-200 bg-red-50 p-3 text-sm text-red-600">
                {snapshot.errorMessage}
              </div>
            </Field>
          ) : null}

          <Field label="原始 JSON">
            <pre className="max-h-72 overflow-auto rounded border border-line bg-slate-950 p-3 text-xs text-slate-100">
              {rawJsonText || '无可解析 JSON'}
            </pre>
          </Field>

          <Field label="原始文本">
            <pre className="max-h-72 overflow-auto whitespace-pre-wrap break-all rounded border border-line bg-slate-50 p-3 text-xs text-slate-600">
              {snapshot.rawResponseText || '无原始文本'}
            </pre>
          </Field>
        </div>
      ) : (
        <div className="rounded-md border border-line p-4 text-sm text-slate-500">
          请选择一条采集快照。
        </div>
      )}
    </SideSheet>
  );
}

function SnapshotDetailItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded border border-line bg-slate-50 px-3 py-2">
      <div className="text-xs text-slate-400">{label}</div>
      <div className="mt-1 break-all text-sm font-medium text-ink">{value}</div>
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

function AutomationResultSummary({ run }: { run: DrawAutomationRun }) {
  return (
    <div className="rounded-md bg-slate-50 p-3 text-sm text-slate-600">
      <div className="font-medium text-ink">{run.now}</div>
      <div className="mt-3 grid grid-cols-2 gap-2">
        <ResultMetric label="封盘" value={`${run.closedIssues.length} 期`} />
        <ResultMetric label="开奖" value={`${run.drawnIssues.length} 期`} />
        <ResultMetric label="结算" value={`${run.settlementRuns.length} 批`} />
        <ResultMetric label="入账" value={`${run.ledgerEntries.length} 笔`} />
        <ResultMetric label="跳过" value={`${run.skippedIssues.length} 项`} />
      </div>
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
          <Input
            className="form-input"
            min="1"
            type="number"
            value={form.intervalSeconds}
            onChange={(value) =>
              setSchedulerConfigFormValue(
                onChange,
                'intervalSeconds',
                value,
              )
            }
          />
        </Field>
        <Field label="未来期号缓冲">
          <Input
            className="form-input"
            max="50"
            min="1"
            type="number"
            value={form.futureIssueCount}
            onChange={(value) =>
              setSchedulerConfigFormValue(
                onChange,
                'futureIssueCount',
                value,
              )
            }
          />
        </Field>
        <Field label="封盘提前（秒）">
          <Input
            className="form-input"
            min="1"
            type="number"
            value={form.saleCloseLeadSeconds}
            onChange={(value) =>
              setSchedulerConfigFormValue(
                onChange,
                'saleCloseLeadSeconds',
                value,
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
                  {run.ledgerEntryCount}，跳过{' '}
                  {run.skippedIssueCount + run.skippedLotteryCount}
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
  const saleClosedAt = new Date(scheduledAt.getTime() - 1000);

  return {
    drawNumber: '',
    issue: '',
    lotteryId: '',
    saleClosedAt: currentDateTimeLabel(saleClosedAt),
    scheduledAt: currentDateTimeLabel(scheduledAt),
  };
}

function emptySourceForm(): DrawSourceFormState {
  return DRAW_SOURCE_PRESETS[0].form;
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
    saleCloseLeadSeconds: '1',
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
    kjApi: 'KJAPI',
    bbKaijiang: 'BB开奖',
    indonesiaLottery: '印尼开奖',
  };
  return labels[provider];
}

function snapshotProviderText(provider: string) {
  if (provider === 'api68' || provider === 'kjApi' || provider === 'bbKaijiang' || provider === 'indonesiaLottery') {
    return drawSourceProviderText(provider as DrawSourceProvider);
  }
  return provider || '-';
}

function snapshotRequestKindText(requestKind: string) {
  if (requestKind === 'latestIssue') {
    return '最新期号';
  }
  if (requestKind === 'drawNumber') {
    return '开奖号码';
  }
  return requestKind || '-';
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
