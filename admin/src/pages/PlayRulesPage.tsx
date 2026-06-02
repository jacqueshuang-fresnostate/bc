import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { Calculator, CheckCircle2, RefreshCcw, Save, XCircle } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { useLotteries } from '../hooks/useLotteries';
import { usePlayRules } from '../hooks/usePlayRules';
import type {
  LotteryKind,
  LotteryNumberType,
  LotteryPlayConfig,
  PlayCategory,
} from '../types/dashboard';
import type {
  BigSmallOddEvenPick,
  DigitAttribute,
  PlayRuleCode,
  PlayRuleSummary,
  PlaySelection,
} from '../types/playRules';
import {
  formatOdds,
  isBankerRule,
  isBigSmallOddEvenRule,
  isDirectRule,
  isGroupSixRule,
  oddsBasisPointsToInput,
  oddsInputToBasisPoints,
} from '../utils/playRules';

interface PlayRulesPageProps {
  onDashboardRefresh: () => void;
}

interface FormState {
  bankerNumbers: string;
  dragNumbers: string;
  drawNumber: string;
  numbers: string;
  onesAttributes: DigitAttribute[];
  positionA: string;
  positionB: string;
  positionC: string;
  tensAttributes: DigitAttribute[];
}

interface OddsDraft {
  enabled: boolean;
  oddsInput: string;
}

const digitAttributeOptions: Array<{ label: string; value: DigitAttribute }> = [
  { label: '大', value: 'big' },
  { label: '小', value: 'small' },
  { label: '单', value: 'odd' },
  { label: '双', value: 'even' },
];

export function PlayRulesPage({ onDashboardRefresh }: PlayRulesPageProps) {
  const { error, evaluate, evaluation, loading, rules, saving } = usePlayRules();
  const {
    error: lotteryError,
    loading: lotteriesLoading,
    lotteries,
    refresh: refreshLotteries,
    saving: lotterySaving,
    update,
  } = useLotteries();
  const [numberType, setNumberType] = useState<LotteryNumberType>('threeDigit');
  const [selectedCode, setSelectedCode] = useState<PlayRuleCode | null>(null);
  const [selectedLotteryId, setSelectedLotteryId] = useState<string | null>(null);
  const [form, setForm] = useState<FormState>(() => defaultForm(null));
  const [oddsDrafts, setOddsDrafts] = useState<Record<string, OddsDraft>>({});
  const [localError, setLocalError] = useState<string | null>(null);
  const [saveMessage, setSaveMessage] = useState<string | null>(null);

  const filteredRules = useMemo(
    () => rules.filter((rule) => rule.numberType === numberType),
    [numberType, rules],
  );
  const filteredLotteries = useMemo(
    () => lotteries.filter((lottery) => lottery.numberType === numberType),
    [lotteries, numberType],
  );
  const selectedRule = useMemo(
    () => filteredRules.find((rule) => rule.code === selectedCode) ?? filteredRules[0] ?? null,
    [filteredRules, selectedCode],
  );
  const selectedLottery = useMemo(
    () =>
      filteredLotteries.find((lottery) => lottery.id === selectedLotteryId) ??
      filteredLotteries[0] ??
      null,
    [filteredLotteries, selectedLotteryId],
  );
  const visibleEvaluation =
    evaluation && selectedRule && evaluation.ruleCode === selectedRule.code ? evaluation : null;
  const combinedError = localError ?? error ?? lotteryError;

  useEffect(() => {
    if (selectedRule && selectedCode !== selectedRule.code) {
      setSelectedCode(selectedRule.code);
      setForm(defaultForm(selectedRule));
    }
  }, [selectedCode, selectedRule]);

  useEffect(() => {
    if (selectedLottery && selectedLotteryId !== selectedLottery.id) {
      setSelectedLotteryId(selectedLottery.id);
    }
  }, [selectedLottery, selectedLotteryId]);

  useEffect(() => {
    if (!selectedLottery) {
      setOddsDrafts({});
      return;
    }

    setOddsDrafts(
      Object.fromEntries(
        filteredRules.map((rule) => [rule.code, defaultDraftForRule(rule, selectedLottery)]),
      ),
    );
  }, [filteredRules, selectedLottery]);

  const changeNumberType = (nextType: LotteryNumberType) => {
    const nextRule = rules.find((rule) => rule.numberType === nextType) ?? null;
    const nextLottery = lotteries.find((lottery) => lottery.numberType === nextType) ?? null;
    setNumberType(nextType);
    setSelectedCode(nextRule?.code ?? null);
    setSelectedLotteryId(nextLottery?.id ?? null);
    setForm(defaultForm(nextRule));
    setLocalError(null);
    setSaveMessage(null);
  };

  const changeRule = (code: PlayRuleCode) => {
    const nextRule = filteredRules.find((rule) => rule.code === code) ?? null;
    setSelectedCode(code);
    setForm(defaultForm(nextRule));
    setLocalError(null);
  };

  const submit = async () => {
    if (!selectedRule) {
      return;
    }
    setLocalError(null);
    await evaluate({
      drawNumber: form.drawNumber.trim(),
      numberType: selectedRule.numberType,
      ruleCode: selectedRule.code,
      selection: selectionFromForm(selectedRule.code, form),
    });
  };

  const saveLotteryOdds = async () => {
    if (!selectedLottery) {
      setLocalError('请先选择一个彩种');
      return;
    }

    const playConfigs = filteredRules.map((rule) => {
      const draft = oddsDrafts[rule.code] ?? defaultDraftForRule(rule, selectedLottery);
      return {
        enabled: draft.enabled,
        oddsBasisPoints: oddsInputToBasisPoints(draft.oddsInput),
        ruleCode: rule.code,
      };
    });
    const invalidConfig = playConfigs.find((config) => config.oddsBasisPoints <= 0);
    if (invalidConfig) {
      setLocalError('赔率必须大于 0');
      return;
    }

    const playCategories = enabledPlayCategories(filteredRules, playConfigs);
    if (playCategories.length === 0) {
      setLocalError('至少需要启用一个玩法');
      return;
    }

    setLocalError(null);
    setSaveMessage(null);
    await update(selectedLottery.id, {
      ...selectedLottery,
      playCategories,
      playConfigs,
    });
    refreshLotteries();
    onDashboardRefresh();
    setSaveMessage('玩法赔率已保存');
  };

  const pageLoading = loading || lotteriesLoading;

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">玩法规则与赔率</h1>
          <p className="mt-1 text-sm text-slate-500">
            按 3 位和 5 位号码区分玩法，支持查看规则、试算注数并维护每个彩种的独立赔率。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            theme={numberType === 'threeDigit' ? 'solid' : 'light'}
            onClick={() => changeNumberType('threeDigit')}
          >
            3 位玩法
          </Button>
          <Button
            theme={numberType === 'fiveDigit' ? 'solid' : 'light'}
            onClick={() => changeNumberType('fiveDigit')}
          >
            5 位玩法
          </Button>
          <Button
            icon={<RefreshCcw size={16} />}
            onClick={() => {
              refreshLotteries();
              setSaveMessage(null);
              setLocalError(null);
            }}
          >
            刷新
          </Button>
        </div>
      </section>

      {combinedError ? (
        <Banner type="danger" title="玩法配置接口错误" description={combinedError} />
      ) : null}
      {saveMessage ? <Banner type="success" title={saveMessage} /> : null}

      {pageLoading ? (
        <div className="grid min-h-[360px] place-items-center">
          <Spin tip="正在加载玩法配置" />
        </div>
      ) : (
        <section className="grid min-w-0 gap-4 xl:grid-cols-[420px_minmax(0,1fr)]">
          <Card className="min-w-0 rounded-md border border-line">
            <div className="space-y-4">
              <div className="flex items-center justify-between gap-3">
                <h2 className="text-base font-semibold text-ink">玩法试算</h2>
                <Tag color={numberType === 'threeDigit' ? 'cyan' : 'blue'}>
                  {numberTypeText(numberType)}
                </Tag>
              </div>

              <Field label="玩法">
                <select
                  className="form-input"
                  value={selectedRule?.code ?? ''}
                  onChange={(event) => changeRule(event.target.value as PlayRuleCode)}
                >
                  {filteredRules.map((rule) => (
                    <option key={rule.code} value={rule.code}>
                      {rule.label}
                    </option>
                  ))}
                </select>
              </Field>

              {selectedRule ? (
                <div className="rounded-md bg-slate-50 p-3 text-sm leading-6 text-slate-600">
                  <div className="mb-2 flex flex-wrap gap-2">
                    <Tag color="grey">{categoryLabel(selectedRule.category)}</Tag>
                    <Tag color="blue">{windowLabel(selectedRule.window)}</Tag>
                  </div>
                  {selectedRule.description}
                </div>
              ) : null}

              <Field label="开奖号码">
                <input
                  className="form-input"
                  value={form.drawNumber}
                  onChange={(event) => setFormValue(setForm, 'drawNumber', event.target.value)}
                />
              </Field>

              {selectedRule ? renderSelectionFields(selectedRule.code, form, setForm) : null}

              <Button
                disabled={!selectedRule || saving}
                icon={<Calculator size={16} />}
                theme="solid"
                onClick={() => void submit()}
              >
                {saving ? '计算中' : '计算规则'}
              </Button>
            </div>
          </Card>

          <div className="min-w-0 space-y-4">
            <Card className="min-w-0 rounded-md border border-line">
              <div className="mb-3 flex items-center justify-between gap-3">
                <div>
                  <h2 className="text-base font-semibold text-ink">彩种玩法赔率</h2>
                  <p className="mt-1 text-sm text-slate-500">
                    当前只展示 {numberTypeText(numberType)}，每个彩种可单独设置玩法启用状态和赔率。
                  </p>
                </div>
                <Tag color="cyan">{filteredRules.length} 个玩法</Tag>
              </div>

              <div className="mb-4 grid gap-3 md:grid-cols-[260px_1fr_auto] md:items-end">
                <Field label="彩种">
                  <select
                    className="form-input"
                    value={selectedLottery?.id ?? ''}
                    onChange={(event) => {
                      setSelectedLotteryId(event.target.value);
                      setSaveMessage(null);
                      setLocalError(null);
                    }}
                  >
                    {filteredLotteries.map((lottery) => (
                      <option key={lottery.id} value={lottery.id}>
                        {lottery.name}
                      </option>
                    ))}
                  </select>
                </Field>
                {selectedLottery ? (
                  <div className="rounded-md border border-line px-3 py-2 text-sm text-slate-600">
                    <div className="font-semibold text-ink">{selectedLottery.name}</div>
                    <div className="mt-1">
                      {selectedLottery.saleEnabled ? '销售中' : '已停售'} ·{' '}
                      {selectedLottery.playConfigs.filter((config) => config.enabled).length}{' '}
                      个玩法已启用
                    </div>
                  </div>
                ) : (
                  <div className="rounded-md border border-line px-3 py-2 text-sm text-slate-500">
                    暂无可配置彩种
                  </div>
                )}
                <Button
                  disabled={!selectedLottery || lotterySaving}
                  icon={<Save size={16} />}
                  theme="solid"
                  onClick={() => void saveLotteryOdds()}
                >
                  {lotterySaving ? '保存中' : '保存赔率'}
                </Button>
              </div>

              <div className="min-w-0 overflow-x-auto">
                <table className="w-full min-w-[860px] text-left text-sm">
                  <thead className="border-b border-line text-xs text-slate-500">
                    <tr>
                      <th className="py-2 pr-4 font-medium">启用</th>
                      <th className="py-2 pr-4 font-medium">玩法</th>
                      <th className="py-2 pr-4 font-medium">分类</th>
                      <th className="py-2 pr-4 font-medium">号码段</th>
                      <th className="py-2 pr-4 font-medium">赔率</th>
                      <th className="py-2 pr-4 font-medium">预览</th>
                    </tr>
                  </thead>
                  <tbody>
                    {filteredRules.map((rule) => {
                      const draft =
                        oddsDrafts[rule.code] ??
                        defaultDraftForRule(rule, selectedLottery ?? undefined);
                      const oddsBasisPoints = oddsInputToBasisPoints(draft.oddsInput);
                      return (
                        <tr key={rule.code} className="border-b border-slate-100">
                          <td className="py-3 pr-4">
                            <input
                              checked={draft.enabled}
                              type="checkbox"
                              onChange={(event) =>
                                setOddsDraft(rule, { enabled: event.target.checked }, setOddsDrafts)
                              }
                            />
                          </td>
                          <td className="py-3 pr-4">
                            <button
                              className="text-left font-semibold text-accent"
                              type="button"
                              onClick={() => changeRule(rule.code)}
                            >
                              {rule.label}
                            </button>
                            <div className="mt-1 max-w-[360px] text-xs text-slate-500">
                              {rule.description}
                            </div>
                          </td>
                          <td className="py-3 pr-4">
                            <Tag color={categoryColor(rule.category)}>
                              {categoryLabel(rule.category)}
                            </Tag>
                          </td>
                          <td className="py-3 pr-4 text-slate-600">
                            {windowLabel(rule.window)}
                          </td>
                          <td className="py-3 pr-4">
                            <input
                              className="form-input w-28"
                              min="0.01"
                              step="0.01"
                              type="number"
                              value={draft.oddsInput}
                              onChange={(event) =>
                                setOddsDraft(rule, { oddsInput: event.target.value }, setOddsDrafts)
                              }
                            />
                          </td>
                          <td className="py-3 pr-4 text-slate-600">
                            {oddsBasisPoints > 0 ? formatOdds(oddsBasisPoints) : '未设置'}
                          </td>
                        </tr>
                      );
                    })}
                  </tbody>
                </table>
              </div>
            </Card>

            <Card className="min-w-0 rounded-md border border-line">
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-base font-semibold text-ink">评估结果</h2>
                {visibleEvaluation ? (
                  <Tag color={visibleEvaluation.isWinning ? 'green' : 'red'}>
                    {visibleEvaluation.isWinning ? '命中' : '未命中'}
                  </Tag>
                ) : null}
              </div>

              {visibleEvaluation ? (
                <div className="grid gap-3 md:grid-cols-3">
                  <ResultBlock label="注数" value={`${visibleEvaluation.stakeCount} 注`} />
                  <ResultBlock
                    label="中奖状态"
                    value={visibleEvaluation.isWinning ? '已命中' : '未命中'}
                    tone={visibleEvaluation.isWinning ? 'success' : 'danger'}
                  />
                  <ResultBlock
                    label="命中投注"
                    value={`${visibleEvaluation.matchedBets.length} 个`}
                  />
                </div>
              ) : (
                <div className="text-sm text-slate-500">填写参数后点击计算，结果会显示在这里。</div>
              )}
            </Card>

            {visibleEvaluation ? (
              <Card className="min-w-0 rounded-md border border-line">
                <div className="grid gap-4 lg:grid-cols-2">
                  <ResultList
                    title="命中投注"
                    items={visibleEvaluation.matchedBets}
                    emptyText="暂无命中"
                  />
                  <ResultList
                    title="展开投注"
                    items={visibleEvaluation.expandedBets.slice(0, 80)}
                    emptyText="暂无投注"
                    suffix={
                      visibleEvaluation.expandedBets.length > 80
                        ? `仅展示前 80 个，共 ${visibleEvaluation.expandedBets.length} 个`
                        : undefined
                    }
                  />
                </div>
              </Card>
            ) : null}

            <Card className="min-w-0 rounded-md border border-line">
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-base font-semibold text-ink">玩法目录</h2>
                <Tag color="grey">{filteredRules.length} 个规则</Tag>
              </div>
              <div className="grid gap-2 md:grid-cols-2">
                {filteredRules.map((rule) => (
                  <button
                    key={rule.code}
                    className={`rounded-md border px-3 py-2 text-left text-sm ${
                      rule.code === selectedRule?.code
                        ? 'border-teal-500 bg-teal-50 text-teal-700'
                        : 'border-line text-slate-600 hover:bg-slate-50'
                    }`}
                    type="button"
                    onClick={() => changeRule(rule.code)}
                  >
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold">{rule.label}</span>
                      <Tag color={categoryColor(rule.category)}>
                        {categoryLabel(rule.category)}
                      </Tag>
                    </div>
                    <div className="mt-1 text-xs text-slate-500">{rule.description}</div>
                  </button>
                ))}
              </div>
            </Card>
          </div>
        </section>
      )}
    </div>
  );
}

function renderSelectionFields(
  code: PlayRuleCode,
  form: FormState,
  setForm: Dispatch<SetStateAction<FormState>>,
) {
  if (isDirectRule(code)) {
    return (
      <div className="grid gap-3 sm:grid-cols-3">
        <Field label="第 1 位">
          <input
            className="form-input"
            value={form.positionA}
            onChange={(event) => setFormValue(setForm, 'positionA', event.target.value)}
          />
        </Field>
        <Field label="第 2 位">
          <input
            className="form-input"
            value={form.positionB}
            onChange={(event) => setFormValue(setForm, 'positionB', event.target.value)}
          />
        </Field>
        <Field label="第 3 位">
          <input
            className="form-input"
            value={form.positionC}
            onChange={(event) => setFormValue(setForm, 'positionC', event.target.value)}
          />
        </Field>
      </div>
    );
  }

  if (isBigSmallOddEvenRule(code)) {
    return (
      <div className="space-y-3">
        <AttributePicker
          label="十位属性"
          selected={form.tensAttributes}
          onToggle={(value) => toggleAttribute(setForm, 'tensAttributes', value)}
        />
        <AttributePicker
          label="个位属性"
          selected={form.onesAttributes}
          onToggle={(value) => toggleAttribute(setForm, 'onesAttributes', value)}
        />
      </div>
    );
  }

  if (isBankerRule(code)) {
    return (
      <div className="grid gap-3 sm:grid-cols-2">
        <Field label="胆码">
          <input
            className="form-input"
            value={form.bankerNumbers}
            onChange={(event) => setFormValue(setForm, 'bankerNumbers', event.target.value)}
          />
        </Field>
        <Field label="拖码">
          <input
            className="form-input"
            value={form.dragNumbers}
            onChange={(event) => setFormValue(setForm, 'dragNumbers', event.target.value)}
          />
        </Field>
      </div>
    );
  }

  return (
    <Field label="选号">
      <input
        className="form-input"
        value={form.numbers}
        onChange={(event) => setFormValue(setForm, 'numbers', event.target.value)}
      />
    </Field>
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

interface AttributePickerProps {
  label: string;
  onToggle: (value: DigitAttribute) => void;
  selected: DigitAttribute[];
}

function AttributePicker({ label, onToggle, selected }: AttributePickerProps) {
  return (
    <Field label={label}>
      <div className="grid grid-cols-4 gap-2">
        {digitAttributeOptions.map((option) => (
          <label
            key={option.value}
            className="flex h-10 items-center justify-center gap-2 rounded-md border border-line text-sm"
          >
            <input
              checked={selected.includes(option.value)}
              type="checkbox"
              onChange={() => onToggle(option.value)}
            />
            {option.label}
          </label>
        ))}
      </div>
    </Field>
  );
}

interface ResultBlockProps {
  label: string;
  tone?: 'danger' | 'success';
  value: string;
}

function ResultBlock({ label, tone, value }: ResultBlockProps) {
  const icon =
    tone === 'success' ? (
      <CheckCircle2 size={18} />
    ) : tone === 'danger' ? (
      <XCircle size={18} />
    ) : null;

  return (
    <div className="rounded-md border border-line p-3">
      <div className="text-xs text-slate-500">{label}</div>
      <div
        className={`mt-2 flex items-center gap-2 text-lg font-semibold ${
          tone === 'success' ? 'text-teal-700' : tone === 'danger' ? 'text-rose-600' : 'text-ink'
        }`}
      >
        {icon}
        {value}
      </div>
    </div>
  );
}

interface ResultListProps {
  emptyText: string;
  items: string[];
  suffix?: string;
  title: string;
}

function ResultList({ emptyText, items, suffix, title }: ResultListProps) {
  return (
    <div>
      <div className="mb-2 flex items-center justify-between gap-2">
        <h3 className="text-sm font-semibold text-ink">{title}</h3>
        {suffix ? <span className="text-xs text-slate-500">{suffix}</span> : null}
      </div>
      {items.length > 0 ? (
        <div className="flex max-h-[220px] flex-wrap gap-2 overflow-auto rounded-md border border-line p-2">
          {items.map((item) => (
            <Tag key={item} color="blue">
              {item}
            </Tag>
          ))}
        </div>
      ) : (
        <div className="rounded-md border border-line p-3 text-sm text-slate-500">{emptyText}</div>
      )}
    </div>
  );
}

function defaultForm(rule: PlayRuleSummary | null): FormState {
  const code = rule?.code ?? 'threeDirect';
  const drawNumber = rule?.numberType === 'fiveDigit' ? '78942' : '247';

  if (isBigSmallOddEvenRule(code)) {
    return {
      bankerNumbers: '',
      dragNumbers: '',
      drawNumber: '78942',
      numbers: '',
      onesAttributes: ['even'],
      positionA: '',
      positionB: '',
      positionC: '',
      tensAttributes: ['small'],
    };
  }

  if (isDirectRule(code)) {
    const digits = defaultDirectDigits(code);
    return {
      bankerNumbers: '',
      dragNumbers: '',
      drawNumber,
      numbers: '',
      onesAttributes: [],
      positionA: digits[0],
      positionB: digits[1],
      positionC: digits[2],
      tensAttributes: [],
    };
  }

  if (isBankerRule(code)) {
    return {
      bankerNumbers: isGroupSixRule(code) ? '2,4' : '2',
      dragNumbers: isGroupSixRule(code) ? '1,7,9' : '4,7',
      drawNumber,
      numbers: '',
      onesAttributes: [],
      positionA: '',
      positionB: '',
      positionC: '',
      tensAttributes: [],
    };
  }

  return {
    bankerNumbers: '',
    dragNumbers: '',
    drawNumber,
    numbers: isGroupSixRule(code) ? '1,2,4,7' : '2,4,7',
    onesAttributes: [],
    positionA: '',
    positionB: '',
    positionC: '',
    tensAttributes: [],
  };
}

function selectionFromForm(code: PlayRuleCode, form: FormState): PlaySelection {
  if (isDirectRule(code)) {
    return {
      positions: [
        parseDigitList(form.positionA),
        parseDigitList(form.positionB),
        parseDigitList(form.positionC),
      ],
    };
  }

  if (isBigSmallOddEvenRule(code)) {
    const picks: BigSmallOddEvenPick[] = [];
    if (form.tensAttributes.length > 0) {
      picks.push({ attributes: form.tensAttributes, position: 'tens' });
    }
    if (form.onesAttributes.length > 0) {
      picks.push({ attributes: form.onesAttributes, position: 'ones' });
    }
    return { bigSmallOddEven: picks };
  }

  if (isBankerRule(code)) {
    return {
      bankerNumbers: parseDigitList(form.bankerNumbers),
      dragNumbers: parseDigitList(form.dragNumbers),
    };
  }

  return {
    numbers: parseDigitList(form.numbers),
  };
}

function parseDigitList(value: string) {
  return value
    .split(/[,\s，]+/)
    .map((item) => item.trim())
    .filter(Boolean)
    .map((item) => Number(item));
}

function defaultDirectDigits(code: PlayRuleCode) {
  if (code === 'fiveMiddleDirect') {
    return ['8', '9', '4'];
  }
  if (code === 'fiveBackDirect') {
    return ['9', '4', '2'];
  }
  if (code === 'fiveFrontDirect') {
    return ['7', '8', '9'];
  }
  return ['2', '4', '7'];
}

function setFormValue<K extends keyof FormState>(
  setForm: Dispatch<SetStateAction<FormState>>,
  key: K,
  value: FormState[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function toggleAttribute(
  setForm: Dispatch<SetStateAction<FormState>>,
  key: 'onesAttributes' | 'tensAttributes',
  value: DigitAttribute,
) {
  setForm((current) => {
    const selected = current[key].includes(value)
      ? current[key].filter((item) => item !== value)
      : [...current[key], value];
    return { ...current, [key]: selected };
  });
}

function setOddsDraft(
  rule: PlayRuleSummary,
  patch: Partial<OddsDraft>,
  setOddsDrafts: Dispatch<SetStateAction<Record<string, OddsDraft>>>,
) {
  setOddsDrafts((current) => ({
    ...current,
    [rule.code]: {
      ...defaultDraftForRule(rule),
      ...current[rule.code],
      ...patch,
    },
  }));
}

function defaultDraftForRule(rule: PlayRuleSummary, lottery?: LotteryKind): OddsDraft {
  const config = lottery?.playConfigs.find((item) => item.ruleCode === rule.code);
  return {
    enabled: config?.enabled ?? true,
    oddsInput: oddsBasisPointsToInput(config?.oddsBasisPoints ?? defaultOddsForRule(rule)),
  };
}

function defaultOddsForRule(rule: PlayRuleSummary) {
  if (rule.category === 'bigSmallOddEven') {
    return 20_000;
  }
  if (rule.category === 'groupThree' || rule.category === 'groupSix') {
    return 50_000;
  }
  return 100_000;
}

function enabledPlayCategories(
  rules: PlayRuleSummary[],
  configs: LotteryPlayConfig[],
): PlayCategory[] {
  const enabledCodes = new Set(
    configs.filter((config) => config.enabled).map((config) => config.ruleCode),
  );
  const categories: PlayCategory[] = [];

  for (const rule of rules) {
    if (enabledCodes.has(rule.code) && !categories.includes(rule.category)) {
      categories.push(rule.category);
    }
  }

  return categories;
}

function numberTypeText(numberType: LotteryNumberType) {
  return numberType === 'threeDigit' ? '3 位玩法' : '5 位玩法';
}

function categoryLabel(category: PlayCategory) {
  const labels: Record<PlayCategory, string> = {
    bigSmallOddEven: '大小单双',
    direct: '直选',
    directCombination: '直选组合',
    groupSix: '组六',
    groupThree: '组三',
  };
  return labels[category];
}

function categoryColor(category: PlayCategory) {
  const colors: Record<PlayCategory, 'blue' | 'cyan' | 'green' | 'orange' | 'purple'> = {
    bigSmallOddEven: 'orange',
    direct: 'blue',
    directCombination: 'cyan',
    groupSix: 'purple',
    groupThree: 'green',
  };
  return colors[category];
}

function windowLabel(window: PlayRuleSummary['window']) {
  const labels: Record<PlayRuleSummary['window'], string> = {
    back: '后三',
    front: '前三',
    full: '全位',
    middle: '中三',
  };
  return labels[window];
}
