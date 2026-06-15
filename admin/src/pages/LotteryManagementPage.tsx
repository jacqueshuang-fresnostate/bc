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
  Tabs,
  Toast,
} from '@douyinfe/semi-ui';
import {
  Calculator,
  FolderTree,
  Plus,
  RefreshCcw,
  Save,
  Trash2,
} from 'lucide-react';
import { useMemo, useState } from 'react';
import { ImageUploadAvatar } from '../components/ImageUploadAvatar';
import { PageControls } from '../components/PageControls';
import { useLotteries } from '../hooks/useLotteries';
import { useLotteryCategories } from '../hooks/useLotteryCategories';
import type {
  DrawMode,
  DrawSchedule,
  LotteryCategory,
  LotteryKind,
  LotteryCategoryConfig,
  LotteryNumberType,
  LotteryPlayConfig,
  PlayCategory,
  SystemSetting,
} from '../types/dashboard';
import {
  lotteryNumberTypeOptions,
  lotteryNumberTypeSupportsPlayRules,
  lotteryNumberTypeText,
} from '../utils/lotteries';
import { minorToYuanInput, yuanInputToMinor } from '../utils/moneyInput';
import { playCategoryForRule } from '../utils/playRules';

interface LotteryManagementPageProps {
  settings: SystemSetting[];
  onDashboardRefresh: () => void;
  onOpenPlayConfig: () => void;
}

type ScheduleKind = 'periodic' | 'timeNode' | 'daily' | 'weekly';
type LotterySaleFilter = 'all' | 'selling' | 'stopped';
const DEFAULT_ISSUE_FORMAT = '{date}{seq4}';

interface LotteryFormState {
  apiDrawDelaySeconds: string;
  category: LotteryCategory;
  drawMode: DrawMode;
  drawControlEnabled: boolean;
  groupBuyEnabled: boolean;
  id: string;
  issueFormat: string;
  logoUrl: string;
  initiatorMinPercent: string;
  intervalSeconds: string;
  minShareAmountYuan: string;
  name: string;
  numberType: LotteryNumberType;
  participantMinAmountYuan: string;
  playCategories: PlayCategory[];
  playConfigs: LotteryPlayConfig[];
  saleEnabled: boolean;
  scheduleKind: ScheduleKind;
  time: string;
  weekdays: string;
}

const playCategoryOptions: Array<{ label: string; value: PlayCategory }> = [
  { label: '直选', value: 'direct' },
  { label: '直选组合', value: 'directCombination' },
  { label: '组三', value: 'groupThree' },
  { label: '组六', value: 'groupSix' },
  { label: '大小单双', value: 'bigSmallOddEven' },
];

export function LotteryManagementPage({
  settings,
  onDashboardRefresh,
  onOpenPlayConfig,
}: LotteryManagementPageProps) {
  const {
    create,
    error,
    loading,
    lotteries,
    refresh,
    remove,
    saving,
    setSaleStatus,
    update,
  } = useLotteries();
  const {
    categories,
    create: createCategory,
    error: categoryError,
    loading: categoriesLoading,
    refresh: refreshCategoryList,
    remove: removeCategory,
    saving: categorySaving,
    update: updateCategory,
  } = useLotteryCategories();
  const [categoryDraftCode, setCategoryDraftCode] = useState('');
  const [categoryDraftName, setCategoryDraftName] = useState('');
  const [editingCategoryCode, setEditingCategoryCode] = useState<string | null>(null);
  const [editingCategoryName, setEditingCategoryName] = useState('');
  const [categorySheetVisible, setCategorySheetVisible] = useState(false);
  const [lotterySheetVisible, setLotterySheetVisible] = useState(false);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [form, setForm] = useState<LotteryFormState>(() => emptyForm());
  const [saleUpdatingId, setSaleUpdatingId] = useState<string | null>(null);
  const [saleFilter, setSaleFilter] = useState<LotterySaleFilter>('all');
  const [lotteryPage, setLotteryPage] = useState(1);
  const [lotteryPageSize, setLotteryPageSize] = useState(10);
  const imageBedUploadField =
    readSettingValue(settings, 'image_bed_upload_field').trim() || 'file';
  const selectedLottery = useMemo(
    () => lotteries.find((lottery) => lottery.id === selectedId) ?? null,
    [lotteries, selectedId],
  );
  const lotteryCategoryOptions = useMemo(
    () =>
      categories
        .map((item) => ({ label: item.name, value: item.code }))
        .sort((a, b) => a.label.localeCompare(b.label)),
    [categories],
  );
  const saleCounts = useMemo(() => {
    const selling = lotteries.filter((lottery) => lottery.saleEnabled).length;
    const stopped = lotteries.length - selling;

    return {
      all: lotteries.length,
      selling,
      stopped,
    };
  }, [lotteries]);
  const filteredLotteries = useMemo(() => {
    if (saleFilter === 'selling') {
      return lotteries.filter((lottery) => lottery.saleEnabled);
    }
    if (saleFilter === 'stopped') {
      return lotteries.filter((lottery) => !lottery.saleEnabled);
    }
    return lotteries;
  }, [lotteries, saleFilter]);
  const lotteryTotalPages = Math.ceil(filteredLotteries.length / lotteryPageSize);
  const normalizedLotteryPage =
    lotteryTotalPages === 0 ? 0 : Math.min(lotteryPage, lotteryTotalPages);
  const paginatedLotteries = useMemo(() => {
    if (normalizedLotteryPage === 0) {
      return [];
    }

    const start = (normalizedLotteryPage - 1) * lotteryPageSize;
    return filteredLotteries.slice(start, start + lotteryPageSize);
  }, [filteredLotteries, lotteryPageSize, normalizedLotteryPage]);

  const allError = categoryError || error;

  const selectLottery = (lottery: LotteryKind) => {
    setSelectedId(lottery.id);
    setForm(formFromLottery(lottery));
    setLotterySheetVisible(true);
  };

  const refreshAll = () => {
    refresh();
    refreshCategoryList();
  };

  const addLotteryCategory = async () => {
    const code = categoryDraftCode.trim();
    const name = categoryDraftName.trim();
    if (!code || !name) {
      return;
    }

    const created = await createCategory({ code, name });
    setCategoryDraftCode('');
    setCategoryDraftName('');
    onDashboardRefresh();
    return created;
  };

  const saveCategory = async () => {
    if (!editingCategoryCode) {
      return;
    }

    const category = editingCategoryCode;
    const name = editingCategoryName.trim();
    if (!name) {
      return;
    }

    await updateCategory(category, {
      code: category,
      name,
    });
    setEditingCategoryCode(null);
    onDashboardRefresh();
  };

  const removeLotteryCategory = async (code: string) => {
    await removeCategory(code);
    onDashboardRefresh();
  };

  const newLottery = () => {
    setSelectedId(null);
    setForm(emptyForm());
    setLotterySheetVisible(true);
  };

  const saveLottery = async () => {
    const minShareAmountMinor = positiveYuanToMinor(
      form.minShareAmountYuan,
      '每份最低金额',
    );
    const participantMinAmountMinor = positiveYuanToMinor(
      form.participantMinAmountYuan,
      '参与最低金额',
    );
    if (minShareAmountMinor === null || participantMinAmountMinor === null) {
      return;
    }
    const payload = lotteryFromForm(
      form,
      minShareAmountMinor,
      participantMinAmountMinor,
    );
    if (selectedId) {
      await update(selectedId, payload);
    } else {
      await create(payload);
      setSelectedId(payload.id);
    }
    setLotterySheetVisible(false);
    onDashboardRefresh();
  };

  const updateLotteryCategory = async (
    lottery: LotteryKind,
    category: LotteryCategory,
  ) => {
    const payload = {
      ...lottery,
      category,
    };

    await update(lottery.id, payload);

    if (selectedId === lottery.id) {
      setForm((current) => ({
        ...current,
        category,
      }));
    }

    onDashboardRefresh();
  };

  const deleteSelected = async () => {
    if (!selectedId) {
      return;
    }
    await remove(selectedId);
    setSelectedId(null);
    setForm(emptyForm());
    setLotterySheetVisible(false);
    onDashboardRefresh();
  };

  const toggleSale = async (lottery: LotteryKind, saleEnabled: boolean) => {
    setSaleUpdatingId(lottery.id);
    try {
      await setSaleStatus(lottery.id, saleEnabled);
      if (selectedId === lottery.id) {
        setForm((current) => ({
          ...current,
          saleEnabled,
        }));
      }
      onDashboardRefresh();
    } finally {
      setSaleUpdatingId((current) => (current === lottery.id ? null : current));
    }
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">彩种管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            配置彩种基础信息、开奖模式、开奖时间、玩法分类和合买规则。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            icon={<FolderTree size={16} />}
            onClick={() => setCategorySheetVisible(true)}
          >
            分类管理
          </Button>
          <Button icon={<Calculator size={16} />} onClick={onOpenPlayConfig}>
            玩法配置
          </Button>
          <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
            刷新
          </Button>
          <Button theme="solid" icon={<Plus size={16} />} onClick={newLottery}>
            新增彩种
          </Button>
        </div>
      </section>

      {allError ? (
        <Banner type="danger" title="接口错误" description={allError} />
      ) : null}

      <section>
        <Card className="rounded-md border border-line">
          <div className="mb-3 flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
            <div className="flex items-center gap-2">
              <h2 className="text-base font-semibold text-ink">彩种列表</h2>
              <Tag color="cyan">{filteredLotteries.length} 个彩种</Tag>
            </div>
            <PageControls
              loading={loading}
              page={normalizedLotteryPage}
              pageSize={lotteryPageSize}
              totalCount={filteredLotteries.length}
              totalPages={lotteryTotalPages}
              onPageChange={setLotteryPage}
              onPageSizeChange={(nextPageSize) => {
                setLotteryPage(1);
                setLotteryPageSize(nextPageSize);
              }}
            />
          </div>
          <Tabs
            activeKey={saleFilter}
            collapsible
            onChange={(key) => {
              setSaleFilter(key as LotterySaleFilter);
              setLotteryPage(1);
            }}
          >
            <Tabs.TabPane
              itemKey="all"
              tab={
                <span className="inline-flex items-center gap-2">
                  <span>全部</span>
                  <Tag color="cyan">{saleCounts.all}</Tag>
                </span>
              }
            />
            <Tabs.TabPane
              itemKey="selling"
              tab={
                <span className="inline-flex items-center gap-2">
                  <span>销售中</span>
                  <Tag color="green">{saleCounts.selling}</Tag>
                </span>
              }
            />
            <Tabs.TabPane
              itemKey="stopped"
              tab={
                <span className="inline-flex items-center gap-2">
                  <span>已停售</span>
                  <Tag color="grey">{saleCounts.stopped}</Tag>
                </span>
              }
            />
          </Tabs>
          {loading ? (
            <div className="grid min-h-[260px] place-items-center">
              <Spin tip="正在加载彩种" />
            </div>
          ) : paginatedLotteries.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[860px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">彩种</th>
                    <th className="py-2 pr-4 font-medium">LOGO</th>
                    <th className="py-2 pr-4 font-medium">分类</th>
                    <th className="py-2 pr-4 font-medium">快速改分类</th>
                    <th className="py-2 pr-4 font-medium">类型</th>
                    <th className="py-2 pr-4 font-medium">开奖</th>
                    <th className="py-2 pr-4 font-medium">时间</th>
                    <th className="py-2 pr-4 font-medium">销售</th>
                    <th className="py-2 pr-4 font-medium">控制</th>
                    <th className="py-2 pr-4 font-medium">操作</th>
                  </tr>
                </thead>
                <tbody>
                  {paginatedLotteries.map((lottery) => (
                    <tr
                      key={lottery.id}
                      className={`border-b border-slate-100 ${
                        selectedId === lottery.id ? 'bg-teal-50/60' : ''
                      }`}
                    >
                      <td className="py-3 pr-4">
                        <button
                          className="text-left font-semibold text-accent"
                          type="button"
                          onClick={() => selectLottery(lottery)}
                        >
                          {lottery.name}
                        </button>
                        <div className="mt-1 text-xs text-slate-400">{lottery.id}</div>
                      </td>
                      <td className="py-3 pr-4">
                        {lottery.logoUrl ? (
                          <img
                            alt="彩种 logo"
                            className="h-8 w-8 rounded border border-line object-cover"
                            src={lottery.logoUrl}
                          />
                        ) : (
                          <span className="text-xs text-slate-400">未设置</span>
                        )}
                      </td>
                      <td className="py-3 pr-4 text-slate-600">
                        {lotteryCategoryText(lottery.category, categories)}
                      </td>
                      <td className="py-3 pr-4">
                        <Select
                          className="form-input"
                          disabled={saving}
                          value={lottery.category}
                          onChange={(value) =>
                            void updateLotteryCategory(
                              lottery,
                              value as LotteryCategory,
                            )
                          }
                        >
                          {lotteryCategoryOptions
                            .concat(
                              categoryMissingOption(
                                lottery.category,
                                lotteryCategoryOptions,
                              ),
                            )
                            .map((option) => (
                              <Select.Option
                                key={option.value}
                                value={option.value}
                              >
                                {option.label}
                              </Select.Option>
                            ))}
                        </Select>
                      </td>
                      <td className="py-3 pr-4 text-slate-600">
                        {lotteryNumberTypeText(lottery.numberType)}
                      </td>
                      <td className="py-3 pr-4">
                        <Tag color={drawModeColor(lottery.drawMode)}>
                          {drawModeText(lottery.drawMode)}
                        </Tag>
                      </td>
                      <td className="py-3 pr-4 text-slate-600">
                        {scheduleText(lottery.schedule)}
                      </td>
                      <td className="py-3 pr-4">
                        <div className="flex items-center gap-2">
                          <Switch
                            checked={lottery.saleEnabled}
                            disabled={saving && saleUpdatingId !== lottery.id}
                            loading={saleUpdatingId === lottery.id}
                            onChange={(checked) => void toggleSale(lottery, checked)}
                          />
                          <Tag color={lottery.saleEnabled ? 'green' : 'grey'}>
                            {lottery.saleEnabled ? '销售中' : '已停售'}
                          </Tag>
                        </div>
                      </td>
                      <td className="py-3 pr-4">
                        <Tag color={lottery.drawControlEnabled ? 'red' : 'grey'}>
                          {lottery.drawControlEnabled ? '允许控制' : '不控制'}
                        </Tag>
                      </td>
                      <td className="py-3 pr-4">
                        <Button size="small" onClick={() => selectLottery(lottery)}>
                          编辑
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="rounded-md border border-line p-4 text-sm text-slate-500">
              {saleFilter === 'selling'
                ? '暂无销售中的彩种。'
                : saleFilter === 'stopped'
                  ? '暂无停售彩种。'
                  : '暂无彩种。'}
            </div>
          )}
        </Card>
      </section>

      <SideSheet
        aria-label={selectedLottery ? '编辑彩种' : '新增彩种'}
        title={selectedLottery ? '编辑彩种' : '新增彩种'}
        visible={lotterySheetVisible}
        width={720}
        onCancel={() => setLotterySheetVisible(false)}
      >
        <div className="mb-4 flex items-start justify-between gap-3 rounded border border-slate-200 bg-slate-50 p-3">
          <p className="text-sm text-slate-500">
            保存后会立即更新后端仓储和工作台概览。
          </p>
          {selectedLottery ? (
            <Tag color="green">已选择</Tag>
          ) : (
            <Tag color="blue">新建</Tag>
          )}
        </div>

        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
          }}
        >
            <Field label="彩种 ID">
              <Input
                className="form-input"
                disabled={Boolean(selectedId)}
                value={form.id}
                onChange={(value) => setFormValue(setForm, 'id', value)}
              />
            </Field>

            <Field label="彩种名称">
              <Input
                className="form-input"
                value={form.name}
                onChange={(value) => setFormValue(setForm, 'name', value)}
              />
            </Field>

            <Field label="LOGO">
              <ImageUploadAvatar
                errorTitle="LOGO 上传失败"
                failureMessage="LOGO 上传失败"
                imageUrl={form.logoUrl}
                requireImageUrl
                showResultPanel={false}
                successMessage="LOGO 上传成功"
                uploadFieldName={imageBedUploadField || 'file'}
                uploadingText="正在上传 LOGO..."
                variant="uploadAdd"
                onUploaded={(url) => setFormValue(setForm, 'logoUrl', url)}
              />
            </Field>

            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="号码类型">
                <Select
                  className="form-input"
                  value={form.numberType}
                  onChange={(value) => {
                    const numberType = value as LotteryNumberType;
                    const supportsPlayRules =
                      lotteryNumberTypeSupportsPlayRules(numberType);
                    setForm((current) => ({
                      ...current,
                      numberType,
                      playCategories: supportsPlayRules ? ['direct'] : [],
                      playConfigs: [],
                    }));
                  }}
                >
                  {lotteryNumberTypeOptions.map((option) => (
                    <Select.Option key={option.value} value={option.value}>
                      {option.label}
                    </Select.Option>
                  ))}
                </Select>
              </Field>
              <Field label="开奖模式">
                <Select
                  className="form-input"
                  value={form.drawMode}
                  onChange={(value) =>
                    setFormValue(setForm, 'drawMode', value as DrawMode)
                  }
                >
                  <Select.Option value="platform">平台开奖</Select.Option>
                  <Select.Option value="api">API 接口</Select.Option>
                  <Select.Option value="manual">指定号码</Select.Option>
                </Select>
              </Field>
              {form.drawMode === 'api' ? (
                <Field label="API开奖延迟秒数">
                  <Input
                    className="form-input"
                    min="0"
                    type="number"
                    value={form.apiDrawDelaySeconds}
                    onChange={(value) =>
                      setFormValue(setForm, 'apiDrawDelaySeconds', value)
                    }
                  />
                </Field>
              ) : null}
              {form.drawMode === 'platform' ? (
                <Field label="平台期号格式">
                  <Input
                    className="form-input"
                    placeholder={DEFAULT_ISSUE_FORMAT}
                    value={form.issueFormat}
                    onChange={(value) =>
                      setFormValue(setForm, 'issueFormat', value)
                    }
                  />
                  <p className="text-xs text-slate-400">
                    支持 {'{yyyy}'}、{'{yy}'}、{'{MM}'}、{'{dd}'}、{'{HH}'}、{'{mm}'}、{'{ss}'}、{'{date}'}、{'{time}'}、{'{timestamp}'}、{'{seq4}'}；留空默认生成如 202606130001 的日期加 4 位序号。
                  </p>
                </Field>
              ) : null}
              <Field label="彩种分类">
                <Select
                  className="form-input"
                  value={form.category}
                  onChange={(value) =>
                    setFormValue(
                      setForm,
                      'category',
                      value as LotteryCategory,
                    )
                  }
                >
                  {lotteryCategoryOptions
                    .concat(
                      categoryMissingOption(form.category, lotteryCategoryOptions),
                    )
                    .map((option) => (
                      <Select.Option key={option.value} value={option.value}>
                        {option.label}
                      </Select.Option>
                    ))}
                </Select>
              </Field>
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="开奖时间类型">
                <Select
                  className="form-input"
                  value={form.scheduleKind}
                  onChange={(value) => {
                    const scheduleKind = value as ScheduleKind;
                    setForm((current) => ({
                      ...current,
                      scheduleKind,
                      time:
                        scheduleKind === 'timeNode' && current.time === '21:00:15'
                          ? '00:00:00'
                          : current.time,
                    }));
                  }}
                >
                  <Select.Option value="periodic">周期开奖</Select.Option>
                  <Select.Option value="timeNode">时间节点周期</Select.Option>
                  <Select.Option value="daily">每日固定</Select.Option>
                  <Select.Option value="weekly">周开奖</Select.Option>
                </Select>
              </Field>
              {form.scheduleKind === 'periodic' || form.scheduleKind === 'timeNode' ? (
                <Field label="周期秒数">
                  <Input
                    className="form-input"
                    min="1"
                    type="number"
                    value={form.intervalSeconds}
                    onChange={(value) =>
                      setFormValue(setForm, 'intervalSeconds', value)
                    }
                  />
                </Field>
              ) : (
                <Field label="开奖时间">
                  <Input
                    className="form-input"
                    placeholder="21:00:15"
                    value={form.time}
                    onChange={(value) => setFormValue(setForm, 'time', value)}
                  />
                </Field>
              )}
            </div>

            {form.scheduleKind === 'timeNode' ? (
              <Field label="起始时间节点">
                <Input
                  className="form-input"
                  placeholder="00:00:00"
                  value={form.time}
                  onChange={(value) => setFormValue(setForm, 'time', value)}
                />
                <div className="text-xs text-slate-400">
                  例如 00:00:00 + 300 秒，会按 00:05、00:10、00:15 开奖。
                </div>
              </Field>
            ) : null}

            {form.scheduleKind === 'weekly' ? (
              <Field label="开奖星期">
                <Input
                  className="form-input"
                  placeholder="Tuesday,Thursday"
                  value={form.weekdays}
                  onChange={(value) => setFormValue(setForm, 'weekdays', value)}
                />
              </Field>
            ) : null}

            {lotteryNumberTypeSupportsPlayRules(form.numberType) ? (
              <Field label="玩法分类">
                <div className="grid gap-2 sm:grid-cols-2">
                  {playCategoryOptions.map((option) => (
                    <label key={option.value} className="flex items-center gap-2 text-sm">
                      <input
                        checked={form.playCategories.includes(option.value)}
                        type="checkbox"
                        onChange={() => togglePlayCategory(setForm, option.value)}
                      />
                      {option.label}
                    </label>
                  ))}
                </div>
              </Field>
            ) : (
              <Banner
                type="info"
                title="玩法暂未接入"
                description="该号码类型当前用于开奖采集、期号调度和开奖号码控制，投注玩法后续可单独扩展。"
              />
            )}

            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="销售状态">
                <div className="flex h-10 items-center gap-2 text-sm text-slate-700">
                  <Switch
                    checked={form.saleEnabled}
                    onChange={(checked) => setFormValue(setForm, 'saleEnabled', checked)}
                  />
                  <span>{form.saleEnabled ? '销售中' : '停售'}</span>
                </div>
              </Field>
              <Field label="开奖号码控制">
                <div className="flex h-10 items-center gap-2 text-sm text-slate-700">
                  <Switch
                    checked={form.drawControlEnabled}
                    onChange={(checked) =>
                      setFormValue(setForm, 'drawControlEnabled', checked)
                    }
                  />
                  <span>{form.drawControlEnabled ? '允许控制' : '不需要控制'}</span>
                </div>
              </Field>
              <Field label="合买状态">
                <label className="flex h-10 items-center gap-2 text-sm">
                  <input
                    checked={form.groupBuyEnabled}
                    type="checkbox"
                    onChange={(event) =>
                      setFormValue(setForm, 'groupBuyEnabled', event.target.checked)
                    }
                  />
                  {form.groupBuyEnabled ? '开启合买' : '关闭合买'}
                </label>
              </Field>
            </div>

            <div className="grid gap-3 sm:grid-cols-3">
              <Field label="每份最低金额（元）">
                <Input
                  className="form-input"
                  inputMode="decimal"
                  placeholder="例如 1 或 1.00"
                  value={form.minShareAmountYuan}
                  onChange={(value) =>
                    setFormValue(setForm, 'minShareAmountYuan', value)
                  }
                />
              </Field>
              <Field label="发起人比例">
                <Input
                  className="form-input"
                  max="100"
                  min="0"
                  type="number"
                  value={form.initiatorMinPercent}
                  onChange={(value) =>
                    setFormValue(setForm, 'initiatorMinPercent', value)
                  }
                />
              </Field>
              <Field label="参与最低金额（元）">
                <Input
                  className="form-input"
                  inputMode="decimal"
                  placeholder="例如 10 或 10.00"
                  value={form.participantMinAmountYuan}
                  onChange={(value) =>
                    setFormValue(setForm, 'participantMinAmountYuan', value)
                  }
                />
              </Field>
            </div>

            <div className="flex flex-wrap gap-2 pt-2">
              <Button
                disabled={saving}
                theme="solid"
                icon={<Save size={16} />}
                onClick={() => void saveLottery()}
              >
                {saving ? '保存中' : '保存'}
              </Button>
              {selectedId ? (
                <Button
                  disabled={saving}
                  icon={<Trash2 size={16} />}
                  onClick={() => void deleteSelected()}
                >
                  删除
                </Button>
              ) : null}
              <Button onClick={() => setLotterySheetVisible(false)}>取消</Button>
            </div>
        </form>
      </SideSheet>

      <SideSheet
        aria-label="彩种分类管理"
        title="彩种分类管理"
        visible={categorySheetVisible}
        width={560}
        onCancel={() => setCategorySheetVisible(false)}
      >
        <div className="space-y-4">
          <div className="rounded border border-slate-200 bg-slate-50 p-3">
            <div className="flex items-start justify-between gap-3">
              <div>
                <p className="text-sm font-medium text-ink">分类配置</p>
                <p className="mt-1 text-xs text-slate-500">
                  新建、编辑与删除分类后，彩种表单和列表的分类下拉会立即使用最新数据。
                </p>
              </div>
              <Tag color="blue">{categories.length} 个分类</Tag>
            </div>
          </div>

          {categoriesLoading ? (
            <div className="grid min-h-[180px] place-items-center">
              <Spin tip="正在加载分类" />
            </div>
          ) : (
            <div className="space-y-3">
              <div className="grid gap-2 rounded border border-slate-200 bg-white p-3">
                <p className="text-sm font-medium text-ink">新增分类</p>
                <div className="grid gap-2 sm:grid-cols-[1fr_1fr_auto]">
                  <Input
                    className="form-input"
                    placeholder="编码，如 overseas"
                    value={categoryDraftCode}
                    onChange={(value) => setCategoryDraftCode(value)}
                  />
                  <Input
                    className="form-input"
                    placeholder="名称，如 海外彩种"
                    value={categoryDraftName}
                    onChange={(value) => setCategoryDraftName(value)}
                  />
                  <Button
                    disabled={!categoryDraftCode.trim() || !categoryDraftName.trim()}
                    loading={categorySaving}
                    theme="solid"
                    onClick={() => {
                      void addLotteryCategory();
                    }}
                  >
                    新增
                  </Button>
                </div>
              </div>

              <div className="overflow-x-auto rounded border border-slate-200">
                <table className="w-full min-w-[460px] text-left text-sm">
                  <thead className="border-b border-line bg-slate-50 text-xs text-slate-500">
                    <tr>
                      <th className="py-2 pl-3 pr-4 font-medium">分类编码</th>
                      <th className="py-2 pr-4 font-medium">分类名称</th>
                      <th className="py-2 pr-3 font-medium">操作</th>
                    </tr>
                  </thead>
                  <tbody>
                    {categories.map((category) => (
                      <tr key={category.code} className="border-b border-slate-100">
                        <td className="py-2 pl-3 pr-4 font-mono text-xs text-slate-600">
                          {category.code}
                        </td>
                        <td className="py-2 pr-4">
                          {editingCategoryCode === category.code ? (
                            <Input
                              className="form-input"
                              value={editingCategoryName}
                              onChange={(value) =>
                                setEditingCategoryName(value)
                              }
                            />
                          ) : (
                            <span className="text-ink">{category.name}</span>
                          )}
                        </td>
                        <td className="py-2 pr-3">
                          {editingCategoryCode === category.code ? (
                            <div className="flex items-center gap-2">
                              <Button
                                loading={categorySaving}
                                size="small"
                                onClick={() => {
                                  void saveCategory();
                                }}
                              >
                                保存
                              </Button>
                              <Button
                                size="small"
                                onClick={() => setEditingCategoryCode(null)}
                              >
                                取消
                              </Button>
                            </div>
                          ) : (
                            <div className="flex items-center gap-2">
                              <Button
                                size="small"
                                onClick={() => {
                                  setEditingCategoryCode(category.code);
                                  setEditingCategoryName(category.name);
                                }}
                              >
                                编辑
                              </Button>
                              <Button
                                size="small"
                                onClick={() => {
                                  if (!window.confirm(`确定删除分类【${category.name}】吗？`)) {
                                    return;
                                  }
                                  void removeLotteryCategory(category.code);
                                }}
                              >
                                删除
                              </Button>
                            </div>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          )}
        </div>
      </SideSheet>
    </div>
  );
}

interface FieldProps {
  children: React.ReactNode;
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

function emptyForm(): LotteryFormState {
  return {
    apiDrawDelaySeconds: '0',
    category: 'regional',
    drawMode: 'platform',
    drawControlEnabled: true,
    groupBuyEnabled: true,
    id: '',
    initiatorMinPercent: '10',
    intervalSeconds: '60',
    issueFormat: DEFAULT_ISSUE_FORMAT,
    minShareAmountYuan: '1.00',
    name: '',
    logoUrl: '',
    numberType: 'threeDigit',
    participantMinAmountYuan: '10.00',
    playCategories: ['direct'],
    playConfigs: [],
    saleEnabled: true,
    scheduleKind: 'periodic',
    time: '21:00:15',
    weekdays: 'Tuesday,Thursday',
  };
}

function formFromLottery(lottery: LotteryKind): LotteryFormState {
  const schedule = scheduleFormFields(lottery.schedule);

  return {
    apiDrawDelaySeconds: String(lottery.apiDrawDelaySeconds ?? 0),
    category: lottery.category,
    logoUrl: lottery.logoUrl,
    drawMode: lottery.drawMode,
    drawControlEnabled: lottery.drawControlEnabled,
    groupBuyEnabled: lottery.groupBuy.enabled,
    id: lottery.id,
    initiatorMinPercent: String(lottery.groupBuy.initiatorMinPercent),
    intervalSeconds: schedule.intervalSeconds,
    issueFormat: lottery.issueFormat || DEFAULT_ISSUE_FORMAT,
    minShareAmountYuan: minorToYuanInput(lottery.groupBuy.minShareAmountMinor),
    name: lottery.name,
    numberType: lottery.numberType,
    participantMinAmountYuan: minorToYuanInput(
      lottery.groupBuy.participantMinAmountMinor,
    ),
    playCategories: lottery.playCategories,
    playConfigs: lottery.playConfigs,
    saleEnabled: lottery.saleEnabled,
    scheduleKind: schedule.scheduleKind,
    time: schedule.time,
    weekdays: schedule.weekdays,
  };
}

function lotteryFromForm(
  form: LotteryFormState,
  minShareAmountMinor: number,
  participantMinAmountMinor: number,
): LotteryKind {
  return {
    apiDrawDelaySeconds:
      form.drawMode === 'api'
        ? nonNegativeIntegerField(form.apiDrawDelaySeconds)
        : 0,
    category: form.category,
    drawMode: form.drawMode,
    drawControlEnabled: form.drawControlEnabled,
    groupBuy: {
      enabled: form.groupBuyEnabled,
      initiatorMinPercent: numberField(form.initiatorMinPercent),
      minShareAmountMinor,
      participantMinAmountMinor,
    },
    id: form.id.trim(),
    issueFormat:
      form.drawMode === 'platform'
        ? form.issueFormat.trim() || DEFAULT_ISSUE_FORMAT
        : DEFAULT_ISSUE_FORMAT,
    name: form.name.trim(),
    logoUrl: form.logoUrl.trim(),
    numberType: form.numberType,
    playCategories: lotteryNumberTypeSupportsPlayRules(form.numberType)
      ? form.playCategories
      : [],
    playConfigs: playConfigsForForm(form),
    saleEnabled: form.saleEnabled,
    schedule: scheduleFromForm(form),
  };
}

function positiveYuanToMinor(value: string, label: string) {
  const amountMinor = yuanInputToMinor(value);
  if (amountMinor === null || amountMinor <= 0) {
    Toast.warning(`${label}必须大于 0 元且最多保留两位小数`);
    return null;
  }
  return amountMinor;
}

function playConfigsForForm(form: LotteryFormState): LotteryPlayConfig[] {
  if (!lotteryNumberTypeSupportsPlayRules(form.numberType)) {
    return [];
  }

  return form.playConfigs
    .map((config) => ({
      ...config,
      enabled: config.enabled && form.playCategories.includes(playCategoryForRule(config.ruleCode)),
    }));
}

function scheduleFormFields(schedule: DrawSchedule) {
  if ('periodic' in schedule) {
    return {
      intervalSeconds: String(schedule.periodic.intervalSeconds),
      scheduleKind: 'periodic' as const,
      time: '21:00:15',
      weekdays: 'Tuesday,Thursday',
    };
  }

  if ('timeNode' in schedule) {
    return {
      intervalSeconds: String(schedule.timeNode.intervalSeconds),
      scheduleKind: 'timeNode' as const,
      time: schedule.timeNode.startTime,
      weekdays: 'Tuesday,Thursday',
    };
  }

  if ('daily' in schedule) {
    return {
      intervalSeconds: '60',
      scheduleKind: 'daily' as const,
      time: schedule.daily.time,
      weekdays: 'Tuesday,Thursday',
    };
  }

  return {
    intervalSeconds: '60',
    scheduleKind: 'weekly' as const,
    time: schedule.weekly.time,
    weekdays: schedule.weekly.weekdays.join(','),
  };
}

function scheduleFromForm(form: LotteryFormState): DrawSchedule {
  if (form.scheduleKind === 'periodic') {
    return {
      periodic: {
        intervalSeconds: numberField(form.intervalSeconds),
      },
    };
  }

  if (form.scheduleKind === 'timeNode') {
    return {
      timeNode: {
        intervalSeconds: numberField(form.intervalSeconds),
        startTime: form.time.trim() || '00:00:00',
      },
    };
  }

  if (form.scheduleKind === 'daily') {
    return {
      daily: {
        time: form.time.trim(),
      },
    };
  }

  return {
    weekly: {
      time: form.time.trim(),
      weekdays: form.weekdays
        .split(',')
        .map((weekday) => weekday.trim())
        .filter(Boolean),
    },
  };
}

function numberField(value: string) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function nonNegativeIntegerField(value: string) {
  return Math.floor(Math.max(0, numberField(value)));
}

function readSettingValue(settings: SystemSetting[], key: string) {
  return settings.find((item) => item.key === key)?.value ?? '';
}

function setFormValue<K extends keyof LotteryFormState>(
  setForm: React.Dispatch<React.SetStateAction<LotteryFormState>>,
  key: K,
  value: LotteryFormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function togglePlayCategory(
  setForm: React.Dispatch<React.SetStateAction<LotteryFormState>>,
  value: PlayCategory,
) {
  setForm((current) => {
    const selected = current.playCategories.includes(value)
      ? current.playCategories.filter((category) => category !== value)
      : [...current.playCategories, value];
    return { ...current, playCategories: selected };
  });
}

function drawModeText(mode: string) {
  const labels: Record<string, string> = {
    api: 'API 接口',
    manual: '指定号码',
    platform: '平台开奖',
  };
  return labels[mode] ?? mode;
}

function lotteryCategoryText(
  category: LotteryCategory,
  categories: LotteryCategoryConfig[],
) {
  const match = categories.find((item) => item.code === category);
  return match?.name ?? category;
}

function categoryMissingOption(
  category: LotteryCategory,
  options: Array<{ label: string; value: LotteryCategory }>,
) {
  const exists = options.some((option) => option.value === category);
  return exists ? [] : [{ label: category, value: category }];
}

function drawModeColor(mode: string) {
  const colors: Record<string, 'green' | 'blue' | 'orange'> = {
    api: 'blue',
    manual: 'orange',
    platform: 'green',
  };
  return colors[mode] ?? 'blue';
}

function scheduleText(schedule: DrawSchedule) {
  if ('periodic' in schedule) {
    return `${schedule.periodic.intervalSeconds} 秒一期`;
  }
  if ('timeNode' in schedule) {
    return `时间节点 ${schedule.timeNode.startTime} 起，每 ${schedule.timeNode.intervalSeconds} 秒一期`;
  }
  if ('daily' in schedule) {
    return `每日 ${schedule.daily.time}`;
  }
  return `${schedule.weekly.weekdays.join('、')} ${schedule.weekly.time}`;
}
