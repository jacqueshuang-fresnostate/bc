import {
  Input,
  Banner,
  Button,
  Card,
  Select,
  SideSheet,
  Spin,
  Tag,
} from '@douyinfe/semi-ui';
import {
  ExternalLink,
  Image as ImageIcon,
  Plus,
  RefreshCcw,
  Save,
  Trash2,
} from 'lucide-react';
import {
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { ImageUploadAvatar } from '../components/ImageUploadAvatar';
import { MetricCard } from '../components/MetricCard';
import { useAdvertisements } from '../hooks/useAdvertisements';
import type {
  AdvertisementPlacement,
  AdvertisementStatus,
  AdvertisementSummary,
  SaveAdvertisementRequest,
} from '../types/advertisements';
import type { SystemSetting } from '../types/dashboard';

interface AdvertisementManagementPageProps {
  onDashboardRefresh: () => void;
  settings: SystemSetting[];
}

interface AdvertisementFormState {
  endAt: string;
  id: string;
  imageUrl: string;
  linkUrl: string;
  placement: AdvertisementPlacement;
  sortOrder: string;
  startAt: string;
  status: AdvertisementStatus;
  title: string;
}

export function AdvertisementManagementPage({
  onDashboardRefresh,
  settings,
}: AdvertisementManagementPageProps) {
  const {
    advertisements,
    error,
    loading,
    refresh,
    remove,
    save,
    saving,
  } = useAdvertisements();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [sheetVisible, setSheetVisible] = useState(false);
  const [form, setForm] = useState<AdvertisementFormState>(() =>
    emptyAdvertisementForm(),
  );
  const sortedAdvertisements = useMemo(
    () =>
      [...advertisements].sort((left, right) =>
        left.sortOrder === right.sortOrder
          ? left.id.localeCompare(right.id)
          : left.sortOrder - right.sortOrder,
      ),
    [advertisements],
  );
  const totals = useMemo(
    () => advertisementTotals(advertisements),
    [advertisements],
  );
  const uploadFieldName = readSettingValue(settings, 'image_bed_upload_field').trim() || 'file';
  const missingImageConfigLabels = [
    readSettingValue(settings, 'image_bed_upload_url').trim()
      ? null
      : '图床上传接口地址',
    readSettingValue(settings, 'image_bed_authorization_token').trim()
      ? null
      : '图床上传 Token',
  ].filter((item): item is string => Boolean(item));

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

  const openNewAdvertisement = () => {
    setEditingId(null);
    setForm(emptyAdvertisementForm());
    setSheetVisible(true);
  };

  const openEditAdvertisement = (advertisement: AdvertisementSummary) => {
    setEditingId(advertisement.id);
    setForm(formFromAdvertisement(advertisement));
    setSheetVisible(true);
  };

  const submit = async () => {
    const saved = await save(
      advertisementPayload(form, Boolean(editingId)),
      editingId ?? undefined,
    );
    setEditingId(saved.id);
    setForm(formFromAdvertisement(saved));
    setSheetVisible(false);
    onDashboardRefresh();
  };

  const deleteCurrent = async () => {
    if (!editingId) {
      return;
    }
    await remove(editingId);
    setEditingId(null);
    setForm(emptyAdvertisementForm());
    setSheetVisible(false);
    onDashboardRefresh();
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">广告管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            配置手机端首页轮播广告、跳转链接、排序和展示时间。
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            theme="solid"
            icon={<Plus size={16} />}
            onClick={openNewAdvertisement}
          >
            新增广告
          </Button>
          <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
            刷新
          </Button>
        </div>
      </section>

      {error ? <Banner type="danger" title="广告接口错误" description={error} /> : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard label="广告总数" trend="后台配置" value={`${advertisements.length}`} />
        <MetricCard label="启用广告" trend="用户端可见候选" value={`${totals.enabledCount}`} />
        <MetricCard label="停用广告" trend="不会展示" value={`${totals.disabledCount}`} />
        <MetricCard label="手机轮播" trend="当前广告位" value={`${totals.mobileCarouselCount}`} />
      </section>

      {loading ? (
        <Card className="rounded-md border border-line">
          <div className="grid min-h-[320px] place-items-center">
            <Spin tip="正在加载广告配置" />
          </div>
        </Card>
      ) : (
        <Card className="rounded-md border border-line">
          <div className="mb-3 flex items-center justify-between">
            <h2 className="text-base font-semibold text-ink">手机端轮播</h2>
            <Tag color="cyan">{sortedAdvertisements.length} 条广告</Tag>
          </div>

          {sortedAdvertisements.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full min-w-[960px] text-left text-sm">
                <thead className="border-b border-line text-xs text-slate-500">
                  <tr>
                    <th className="py-2 pr-4 font-medium">广告</th>
                    <th className="py-2 pr-4 font-medium">位置</th>
                    <th className="py-2 pr-4 font-medium">状态</th>
                    <th className="py-2 pr-4 font-medium">排序</th>
                    <th className="py-2 pr-4 font-medium">展示时间</th>
                    <th className="py-2 pr-4 font-medium">操作</th>
                  </tr>
                </thead>
                <tbody>
                  {sortedAdvertisements.map((advertisement) => (
                    <tr
                      key={advertisement.id}
                      className={`border-b border-slate-100 ${
                        editingId === advertisement.id ? 'bg-teal-50/60' : ''
                      }`}
                    >
                      <td className="py-3 pr-4">
                        <div className="flex items-center gap-3">
                          <div className="grid h-14 w-24 shrink-0 place-items-center overflow-hidden rounded border border-slate-200 bg-slate-50">
                            {advertisement.imageUrl ? (
                              <img
                                alt={advertisement.title}
                                className="h-full w-full object-cover"
                                src={advertisement.imageUrl}
                              />
                            ) : (
                              <ImageIcon size={20} className="text-slate-400" />
                            )}
                          </div>
                          <div className="min-w-0">
                            <button
                              className="truncate text-left font-semibold text-accent"
                              type="button"
                              onClick={() => openEditAdvertisement(advertisement)}
                            >
                              {advertisement.title}
                            </button>
                            <div className="mt-1 text-xs text-slate-400">
                              {advertisement.id}
                            </div>
                            {advertisement.linkUrl ? (
                              <a
                                className="mt-1 inline-flex max-w-[260px] items-center gap-1 truncate text-xs text-teal-700"
                                href={advertisement.linkUrl}
                                rel="noreferrer"
                                target="_blank"
                              >
                                <ExternalLink size={12} />
                                {advertisement.linkUrl}
                              </a>
                            ) : null}
                          </div>
                        </div>
                      </td>
                      <td className="py-3 pr-4">
                        {advertisementPlacementText(advertisement.placement)}
                      </td>
                      <td className="py-3 pr-4">
                        <Tag color={advertisement.status === 'enabled' ? 'green' : 'grey'}>
                          {advertisementStatusText(advertisement.status)}
                        </Tag>
                      </td>
                      <td className="py-3 pr-4">{advertisement.sortOrder}</td>
                      <td className="py-3 pr-4 text-slate-600">
                        <div>{advertisement.startAt || '立即开始'}</div>
                        <div>{advertisement.endAt || '长期有效'}</div>
                      </td>
                      <td className="py-3 pr-4">
                        <Button
                          size="small"
                          onClick={() => openEditAdvertisement(advertisement)}
                        >
                          编辑
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="rounded border border-dashed border-slate-200 py-10 text-center text-sm text-slate-500">
              暂无广告配置
            </div>
          )}
        </Card>
      )}

      <SideSheet
        title={editingId ? '编辑广告' : '新增广告'}
        visible={sheetVisible}
        width={560}
        onCancel={() => setSheetVisible(false)}
      >
        <div className="space-y-5 pb-6">
          <FormField label="广告 ID">
            <Input
              disabled={Boolean(editingId)}
              placeholder="留空自动生成，例如 AD000001"
              value={form.id}
              onChange={(value) => setFormValue(setForm, 'id', value)}
            />
          </FormField>

          <FormField label="广告标题">
            <Input
              placeholder="例如 春季加奖活动"
              value={form.title}
              onChange={(value) => setFormValue(setForm, 'title', value)}
            />
          </FormField>

          <FormField label="广告图片">
            <ImageUploadAvatar
              description="建议上传长方形横幅图片，适合手机首页轮播展示。"
              imageUrl={form.imageUrl}
              missingConfigLabels={missingImageConfigLabels}
              previewShape="banner"
              requireImageUrl
              showResultPanel={false}
              successMessage="广告图片上传成功"
              title="上传广告轮播图"
              uploadFieldName={uploadFieldName}
              warningTitle="图床配置不完整"
              onClear={() => setFormValue(setForm, 'imageUrl', '')}
              onUploaded={(url) => setFormValue(setForm, 'imageUrl', url)}
            />
          </FormField>

          <FormField label="图片链接">
            <Input
              placeholder="也可以直接粘贴图片 URL"
              value={form.imageUrl}
              onChange={(value) => setFormValue(setForm, 'imageUrl', value)}
            />
          </FormField>

          <FormField label="跳转链接">
            <Input
              placeholder="可为空，例如 https://example.com/activity"
              value={form.linkUrl}
              onChange={(value) => setFormValue(setForm, 'linkUrl', value)}
            />
          </FormField>

          <div className="grid gap-4 sm:grid-cols-2">
            <FormField label="广告位置">
              <Select
                className="w-full"
                value={form.placement}
                onChange={(value) =>
                  setFormValue(
                    setForm,
                    'placement',
                    value as AdvertisementPlacement,
                  )
                }
              >
                <Select.Option value="mobileCarousel">手机端轮播</Select.Option>
              </Select>
            </FormField>
            <FormField label="广告状态">
              <Select
                className="w-full"
                value={form.status}
                onChange={(value) =>
                  setFormValue(setForm, 'status', value as AdvertisementStatus)
                }
              >
                <Select.Option value="enabled">启用</Select.Option>
                <Select.Option value="disabled">停用</Select.Option>
              </Select>
            </FormField>
          </div>

          <FormField label="排序">
            <Input
              placeholder="数字越小越靠前"
              value={form.sortOrder}
              onChange={(value) => setFormValue(setForm, 'sortOrder', value)}
            />
          </FormField>

          <div className="grid gap-4 sm:grid-cols-2">
            <FormField label="开始时间">
              <Input
                placeholder="可为空，格式 2026-06-04 12:00:00"
                value={form.startAt}
                onChange={(value) => setFormValue(setForm, 'startAt', value)}
              />
            </FormField>
            <FormField label="结束时间">
              <Input
                placeholder="可为空，格式 2026-06-04 23:59:59"
                value={form.endAt}
                onChange={(value) => setFormValue(setForm, 'endAt', value)}
              />
            </FormField>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button
              theme="solid"
              icon={<Save size={16} />}
              loading={saving}
              onClick={() => {
                void submit();
              }}
            >
              保存广告
            </Button>
            {editingId ? (
              <Button
                type="danger"
                icon={<Trash2 size={16} />}
                loading={saving}
                onClick={() => {
                  void deleteCurrent();
                }}
              >
                删除广告
              </Button>
            ) : null}
          </div>
        </div>
      </SideSheet>
    </div>
  );
}

interface FormFieldProps {
  children: ReactNode;
  label: string;
}

function FormField({ children, label }: FormFieldProps) {
  return (
    <label className="block space-y-1 text-sm">
      <span className="font-medium text-slate-700">{label}</span>
      {children}
    </label>
  );
}

function emptyAdvertisementForm(): AdvertisementFormState {
  return {
    endAt: '',
    id: '',
    imageUrl: '',
    linkUrl: '',
    placement: 'mobileCarousel',
    sortOrder: '100',
    startAt: '',
    status: 'enabled',
    title: '',
  };
}

function formFromAdvertisement(
  advertisement: AdvertisementSummary,
): AdvertisementFormState {
  return {
    endAt: advertisement.endAt ?? '',
    id: advertisement.id,
    imageUrl: advertisement.imageUrl,
    linkUrl: advertisement.linkUrl ?? '',
    placement: advertisement.placement,
    sortOrder: String(advertisement.sortOrder),
    startAt: advertisement.startAt ?? '',
    status: advertisement.status,
    title: advertisement.title,
  };
}

function advertisementPayload(
  form: AdvertisementFormState,
  editing: boolean,
): SaveAdvertisementRequest {
  const sortOrder = Number.parseInt(form.sortOrder, 10);
  return {
    id: editing || form.id.trim() ? form.id.trim() : undefined,
    title: form.title.trim(),
    imageUrl: form.imageUrl.trim(),
    linkUrl: optionalText(form.linkUrl),
    placement: form.placement,
    status: form.status,
    sortOrder: Number.isNaN(sortOrder) ? 0 : sortOrder,
    startAt: optionalText(form.startAt),
    endAt: optionalText(form.endAt),
  };
}

function optionalText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function advertisementTotals(advertisements: AdvertisementSummary[]) {
  return {
    disabledCount: advertisements.filter(
      (advertisement) => advertisement.status === 'disabled',
    ).length,
    enabledCount: advertisements.filter(
      (advertisement) => advertisement.status === 'enabled',
    ).length,
    mobileCarouselCount: advertisements.filter(
      (advertisement) => advertisement.placement === 'mobileCarousel',
    ).length,
  };
}

function advertisementPlacementText(placement: AdvertisementPlacement) {
  return placement === 'mobileCarousel' ? '手机端轮播' : placement;
}

function advertisementStatusText(status: AdvertisementStatus) {
  return status === 'enabled' ? '启用' : '停用';
}

function readSettingValue(settings: SystemSetting[], key: string) {
  return settings.find((setting) => setting.key === key)?.value ?? '';
}

function setFormValue<K extends keyof AdvertisementFormState>(
  setter: Dispatch<SetStateAction<AdvertisementFormState>>,
  key: K,
  value: AdvertisementFormState[K],
) {
  setter((current) => ({ ...current, [key]: value }));
}
