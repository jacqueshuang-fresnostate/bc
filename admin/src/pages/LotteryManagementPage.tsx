import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { Calculator, Plus, RefreshCcw, Save, Trash2 } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useLotteries } from '../hooks/useLotteries';
import type {
  DrawMode,
  DrawSchedule,
  LotteryKind,
  LotteryNumberType,
  LotteryPlayConfig,
  PlayCategory,
} from '../types/dashboard';
import { playCategoryForRule } from '../utils/playRules';

interface LotteryManagementPageProps {
  onDashboardRefresh: () => void;
  onOpenPlayConfig: () => void;
}

type ScheduleKind = 'periodic' | 'daily' | 'weekly';

interface LotteryFormState {
  drawMode: DrawMode;
  groupBuyEnabled: boolean;
  id: string;
  initiatorMinPercent: string;
  intervalSeconds: string;
  minShareAmountMinor: string;
  name: string;
  numberType: LotteryNumberType;
  participantMinAmountMinor: string;
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
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [form, setForm] = useState<LotteryFormState>(() => emptyForm());
  const selectedLottery = useMemo(
    () => lotteries.find((lottery) => lottery.id === selectedId) ?? null,
    [lotteries, selectedId],
  );

  const selectLottery = (lottery: LotteryKind) => {
    setSelectedId(lottery.id);
    setForm(formFromLottery(lottery));
  };

  const newLottery = () => {
    setSelectedId(null);
    setForm(emptyForm());
  };

  const saveLottery = async () => {
    const payload = lotteryFromForm(form);
    if (selectedId) {
      await update(selectedId, payload);
    } else {
      await create(payload);
      setSelectedId(payload.id);
    }
    onDashboardRefresh();
  };

  const deleteSelected = async () => {
    if (!selectedId) {
      return;
    }
    await remove(selectedId);
    newLottery();
    onDashboardRefresh();
  };

  const toggleSale = async (lottery: LotteryKind) => {
    await setSaleStatus(lottery.id, !lottery.saleEnabled);
    if (selectedId === lottery.id) {
      setForm((current) => ({
        ...current,
        saleEnabled: !lottery.saleEnabled,
      }));
    }
    onDashboardRefresh();
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
          <Button icon={<Calculator size={16} />} onClick={onOpenPlayConfig}>
            玩法配置
          </Button>
          <Button icon={<RefreshCcw size={16} />} onClick={refresh}>
            刷新
          </Button>
          <Button theme="solid" icon={<Plus size={16} />} onClick={newLottery}>
            新增彩种
          </Button>
        </div>
      </section>

      {error ? <Banner type="danger" title="彩种接口错误" description={error} /> : null}

      <section className="grid gap-4 xl:grid-cols-[1fr_420px]">
        <Card className="rounded-md border border-line">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold text-ink">彩种列表</h2>
            <Tag color="cyan">{lotteries.length} 个彩种</Tag>
          </div>
          {loading ? (
            <div className="grid min-h-[260px] place-items-center">
              <Spin tip="正在加载彩种" />
            </div>
          ) : (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[760px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">彩种</th>
                    <th className="py-2 pr-4 font-medium">类型</th>
                    <th className="py-2 pr-4 font-medium">开奖</th>
                    <th className="py-2 pr-4 font-medium">时间</th>
                    <th className="py-2 pr-4 font-medium">销售</th>
                    <th className="py-2 pr-4 font-medium">操作</th>
                  </tr>
                </thead>
                <tbody>
                  {lotteries.map((lottery) => (
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
                      <td className="py-3 pr-4 text-slate-600">
                        {lottery.numberType === 'threeDigit' ? '3 位号码' : '5 位号码'}
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
                        <button
                          className="rounded-md border border-line px-2 py-1 text-xs text-slate-600 hover:bg-slate-50"
                          type="button"
                          onClick={() => toggleSale(lottery)}
                        >
                          {lottery.saleEnabled ? '销售中' : '已停售'}
                        </button>
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
          )}
        </Card>

        <Card className="rounded-md border border-line">
          <div className="mb-4 flex items-start justify-between gap-3">
            <div>
              <h2 className="text-base font-semibold text-ink">
                {selectedLottery ? '编辑彩种' : '新增彩种'}
              </h2>
              <p className="mt-1 text-sm text-slate-500">
                保存后会立即更新后端内存仓储和工作台概览。
              </p>
            </div>
            {selectedLottery ? <Tag color="green">已选择</Tag> : <Tag color="blue">新建</Tag>}
          </div>

          <form
            className="space-y-4"
            onSubmit={(event) => {
              event.preventDefault();
            }}
          >
            <Field label="彩种 ID">
              <input
                className="form-input"
                disabled={Boolean(selectedId)}
                value={form.id}
                onChange={(event) => setFormValue(setForm, 'id', event.target.value)}
              />
            </Field>

            <Field label="彩种名称">
              <input
                className="form-input"
                value={form.name}
                onChange={(event) => setFormValue(setForm, 'name', event.target.value)}
              />
            </Field>

            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="号码类型">
                <select
                  className="form-input"
                  value={form.numberType}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      numberType: event.target.value as LotteryNumberType,
                      playCategories: ['direct'],
                      playConfigs: [],
                    }))
                  }
                >
                  <option value="threeDigit">3 位号码</option>
                  <option value="fiveDigit">5 位号码</option>
                </select>
              </Field>
              <Field label="开奖模式">
                <select
                  className="form-input"
                  value={form.drawMode}
                  onChange={(event) =>
                    setFormValue(setForm, 'drawMode', event.target.value as DrawMode)
                  }
                >
                  <option value="platform">平台开奖</option>
                  <option value="api">API 接口</option>
                  <option value="manual">指定号码</option>
                </select>
              </Field>
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="开奖时间类型">
                <select
                  className="form-input"
                  value={form.scheduleKind}
                  onChange={(event) =>
                    setFormValue(setForm, 'scheduleKind', event.target.value as ScheduleKind)
                  }
                >
                  <option value="periodic">周期开奖</option>
                  <option value="daily">每日固定</option>
                  <option value="weekly">周开奖</option>
                </select>
              </Field>
              {form.scheduleKind === 'periodic' ? (
                <Field label="周期秒数">
                  <input
                    className="form-input"
                    min="1"
                    type="number"
                    value={form.intervalSeconds}
                    onChange={(event) =>
                      setFormValue(setForm, 'intervalSeconds', event.target.value)
                    }
                  />
                </Field>
              ) : (
                <Field label="开奖时间">
                  <input
                    className="form-input"
                    placeholder="21:00:15"
                    value={form.time}
                    onChange={(event) => setFormValue(setForm, 'time', event.target.value)}
                  />
                </Field>
              )}
            </div>

            {form.scheduleKind === 'weekly' ? (
              <Field label="开奖星期">
                <input
                  className="form-input"
                  placeholder="Tuesday,Thursday"
                  value={form.weekdays}
                  onChange={(event) => setFormValue(setForm, 'weekdays', event.target.value)}
                />
              </Field>
            ) : null}

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

            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="销售状态">
                <label className="flex h-10 items-center gap-2 text-sm">
                  <input
                    checked={form.saleEnabled}
                    type="checkbox"
                    onChange={(event) =>
                      setFormValue(setForm, 'saleEnabled', event.target.checked)
                    }
                  />
                  {form.saleEnabled ? '销售中' : '停售'}
                </label>
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
              <Field label="每份最低金额">
                <input
                  className="form-input"
                  min="1"
                  type="number"
                  value={form.minShareAmountMinor}
                  onChange={(event) =>
                    setFormValue(setForm, 'minShareAmountMinor', event.target.value)
                  }
                />
              </Field>
              <Field label="发起人比例">
                <input
                  className="form-input"
                  max="100"
                  min="0"
                  type="number"
                  value={form.initiatorMinPercent}
                  onChange={(event) =>
                    setFormValue(setForm, 'initiatorMinPercent', event.target.value)
                  }
                />
              </Field>
              <Field label="参与最低金额">
                <input
                  className="form-input"
                  min="1"
                  type="number"
                  value={form.participantMinAmountMinor}
                  onChange={(event) =>
                    setFormValue(setForm, 'participantMinAmountMinor', event.target.value)
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
            </div>
          </form>
        </Card>
      </section>
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
    drawMode: 'platform',
    groupBuyEnabled: true,
    id: '',
    initiatorMinPercent: '10',
    intervalSeconds: '60',
    minShareAmountMinor: '100',
    name: '',
    numberType: 'threeDigit',
    participantMinAmountMinor: '1000',
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
    drawMode: lottery.drawMode,
    groupBuyEnabled: lottery.groupBuy.enabled,
    id: lottery.id,
    initiatorMinPercent: String(lottery.groupBuy.initiatorMinPercent),
    intervalSeconds: schedule.intervalSeconds,
    minShareAmountMinor: String(lottery.groupBuy.minShareAmountMinor),
    name: lottery.name,
    numberType: lottery.numberType,
    participantMinAmountMinor: String(lottery.groupBuy.participantMinAmountMinor),
    playCategories: lottery.playCategories,
    playConfigs: lottery.playConfigs,
    saleEnabled: lottery.saleEnabled,
    scheduleKind: schedule.scheduleKind,
    time: schedule.time,
    weekdays: schedule.weekdays,
  };
}

function lotteryFromForm(form: LotteryFormState): LotteryKind {
  return {
    drawMode: form.drawMode,
    groupBuy: {
      enabled: form.groupBuyEnabled,
      initiatorMinPercent: numberField(form.initiatorMinPercent),
      minShareAmountMinor: numberField(form.minShareAmountMinor),
      participantMinAmountMinor: numberField(form.participantMinAmountMinor),
    },
    id: form.id.trim(),
    name: form.name.trim(),
    numberType: form.numberType,
    playCategories: form.playCategories,
    playConfigs: playConfigsForForm(form),
    saleEnabled: form.saleEnabled,
    schedule: scheduleFromForm(form),
  };
}

function playConfigsForForm(form: LotteryFormState): LotteryPlayConfig[] {
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
  if ('daily' in schedule) {
    return `每日 ${schedule.daily.time}`;
  }
  return `${schedule.weekly.weekdays.join('、')} ${schedule.weekly.time}`;
}
