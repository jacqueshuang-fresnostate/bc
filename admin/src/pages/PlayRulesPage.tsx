import { Banner, Button, Card, Spin, Tag } from '@douyinfe/semi-ui';
import { Calculator, CheckCircle2, XCircle } from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { usePlayRules } from '../hooks/usePlayRules';
import type {
  BigSmallOddEvenPick,
  DigitAttribute,
  PlayRuleCode,
  PlayRuleSummary,
  PlaySelection,
} from '../types/playRules';

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

const digitAttributeOptions: Array<{ label: string; value: DigitAttribute }> = [
  { label: '大', value: 'big' },
  { label: '小', value: 'small' },
  { label: '单', value: 'odd' },
  { label: '双', value: 'even' },
];

export function PlayRulesPage() {
  const { error, evaluate, evaluation, loading, rules, saving } = usePlayRules();
  const [selectedCode, setSelectedCode] = useState<PlayRuleCode | null>(null);
  const [form, setForm] = useState<FormState>(() => defaultForm(null));
  const selectedRule = useMemo(
    () => rules.find((rule) => rule.code === selectedCode) ?? rules[0] ?? null,
    [rules, selectedCode],
  );

  useEffect(() => {
    if (!selectedCode && rules[0]) {
      setSelectedCode(rules[0].code);
      setForm(defaultForm(rules[0]));
    }
  }, [rules, selectedCode]);

  const changeRule = (code: PlayRuleCode) => {
    const nextRule = rules.find((rule) => rule.code === code) ?? null;
    setSelectedCode(code);
    setForm(defaultForm(nextRule));
  };

  const submit = async () => {
    if (!selectedRule) {
      return;
    }
    await evaluate({
      drawNumber: form.drawNumber.trim(),
      numberType: selectedRule.numberType,
      ruleCode: selectedRule.code,
      selection: selectionFromForm(selectedRule.code, form),
    });
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">玩法规则</h1>
          <p className="mt-1 text-sm text-slate-500">
            验证 3 位和 5 位玩法的注数计算、投注展开和中奖判断。
          </p>
        </div>
        {selectedRule ? (
          <Tag color={selectedRule.numberType === 'threeDigit' ? 'cyan' : 'blue'}>
            {selectedRule.numberType === 'threeDigit' ? '3 位玩法' : '5 位玩法'}
          </Tag>
        ) : null}
      </section>

      {error ? <Banner type="danger" title="玩法规则接口错误" description={error} /> : null}

      {loading ? (
        <div className="grid min-h-[360px] place-items-center">
          <Spin tip="正在加载玩法规则" />
        </div>
      ) : (
        <section className="grid gap-4 xl:grid-cols-[420px_1fr]">
          <Card className="rounded-md border border-line">
            <div className="space-y-4">
              <Field label="玩法">
                <select
                  className="form-input"
                  value={selectedRule?.code ?? ''}
                  onChange={(event) => changeRule(event.target.value as PlayRuleCode)}
                >
                  {rules.map((rule) => (
                    <option key={rule.code} value={rule.code}>
                      {rule.label}
                    </option>
                  ))}
                </select>
              </Field>

              {selectedRule ? (
                <div className="rounded-md bg-slate-50 p-3 text-sm leading-6 text-slate-600">
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

          <div className="space-y-4">
            <Card className="rounded-md border border-line">
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-base font-semibold text-ink">评估结果</h2>
                {evaluation ? (
                  <Tag color={evaluation.isWinning ? 'green' : 'red'}>
                    {evaluation.isWinning ? '命中' : '未命中'}
                  </Tag>
                ) : null}
              </div>

              {evaluation ? (
                <div className="grid gap-3 md:grid-cols-3">
                  <ResultBlock label="注数" value={`${evaluation.stakeCount} 注`} />
                  <ResultBlock
                    label="中奖状态"
                    value={evaluation.isWinning ? '已命中' : '未命中'}
                    tone={evaluation.isWinning ? 'success' : 'danger'}
                  />
                  <ResultBlock label="命中投注" value={`${evaluation.matchedBets.length} 个`} />
                </div>
              ) : (
                <div className="text-sm text-slate-500">填写参数后点击计算，结果会显示在这里。</div>
              )}
            </Card>

            {evaluation ? (
              <Card className="rounded-md border border-line">
                <div className="grid gap-4 lg:grid-cols-2">
                  <ResultList title="命中投注" items={evaluation.matchedBets} emptyText="暂无命中" />
                  <ResultList
                    title="展开投注"
                    items={evaluation.expandedBets.slice(0, 80)}
                    emptyText="暂无投注"
                    suffix={
                      evaluation.expandedBets.length > 80
                        ? `仅展示前 80 个，共 ${evaluation.expandedBets.length} 个`
                        : undefined
                    }
                  />
                </div>
              </Card>
            ) : null}

            <Card className="rounded-md border border-line">
              <div className="mb-3 flex items-center justify-between">
                <h2 className="text-base font-semibold text-ink">规则目录</h2>
                <Tag color="grey">{rules.length} 个规则</Tag>
              </div>
              <div className="grid gap-2 md:grid-cols-2">
                {rules.map((rule) => (
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
                    <div className="font-semibold">{rule.label}</div>
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

function isDirectRule(code: PlayRuleCode) {
  return code.endsWith('Direct') && !code.endsWith('DirectCombination');
}

function isBigSmallOddEvenRule(code: PlayRuleCode) {
  return code === 'fiveBigSmallOddEven';
}

function isBankerRule(code: PlayRuleCode) {
  return code.endsWith('Banker');
}

function isGroupSixRule(code: PlayRuleCode) {
  return code.includes('GroupSix');
}
