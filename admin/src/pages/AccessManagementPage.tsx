import {
  Input,
  Banner,
  Button,
  Card,
  Select,
  SideSheet,
  Spin,
  Tabs,
  Tag,
  Toast,
} from '@douyinfe/semi-ui';
import {
  Image as ImageIcon,
  CreditCard,
  RefreshCcw,
  Save,
  Settings,
  Smartphone,
  Upload as UploadIcon,
  ShieldCheck,
  Trash2,
  UserPlus,
  Users,
} from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { fetchLotteries } from '../api/client';
import { ImageUploadAvatar } from '../components/ImageUploadAvatar';
import { PageControls } from '../components/PageControls';
import { useAccessManagement } from '../hooks/useAccessManagement';
import type {
  AdminUserSummary,
  AdminSaveRequest,
  MemoryCacheReloadResult,
  UserListSortBy,
  UserListSortDirection,
} from '../types/access';
import type {
  AdminRole,
  AdminSummary,
  LotteryKind,
  PermissionScope,
  RegistrationConfig,
  UserKind,
  UserStatus,
  UserSummary,
} from '../types/dashboard';
import { formatMoney } from '../utils/format';
import { minorToYuanInput, yuanInputToMinor } from '../utils/moneyInput';

type AccessSection = 'admins' | 'roles' | 'settings' | 'users';

interface AccessManagementPageProps {
  activeModuleKey: string;
  onDashboardRefresh: () => void;
  onOpenUserLedger: (user: AdminUserSummary) => void;
  onOpenUserOrders: (user: AdminUserSummary) => void;
}

interface UserFormState {
  agentId: string;
  balanceMinor: string;
  email: string;
  id: string;
  inviteCode: string;
  kind: UserKind;
  status: UserStatus;
  username: string;
}

interface SystemSettingItem {
  description: string;
  key: string;
  value: string;
}

interface SettingSelectOption {
  label: string;
  value: string;
}

interface AdminFormState {
  id: string;
  password: string;
  roleId: string;
  status: UserStatus;
  username: string;
}

interface RoleFormState {
  id: string;
  name: string;
  scopes: PermissionScope[];
}

const ACCESS_SECTIONS: Array<{ key: AccessSection; label: string }> = [
  { key: 'users', label: '用户管理' },
  { key: 'admins', label: '管理员管理' },
  { key: 'roles', label: '角色权限' },
];

const MOBILE_PLATFORM_NAME_SETTING_KEY = 'mobile_platform_name';
const MOBILE_LOGO_SETTING_KEY = 'mobile_logo_image_url';
const MOBILE_INTRO_SETTING_KEY = 'mobile_site_intro';
const MOBILE_HOME_FEATURED_ENABLED_SETTING_KEY = 'mobile_home_featured_enabled';
const MOBILE_HOME_FEATURED_TITLE_SETTING_KEY = 'mobile_home_featured_title';
const MOBILE_HOME_FEATURED_LOTTERY_CODES_SETTING_KEY = 'mobile_home_featured_lottery_codes';
const RECHARGE_RAINBOW_ENABLED_SETTING_KEY = 'recharge_rainbow_epay_enabled';
const RECHARGE_RAINBOW_PAY_TYPES_SETTING_KEY = 'recharge_rainbow_epay_pay_types';
const RECHARGE_CUSTOMER_SERVICE_ENABLED_SETTING_KEY =
  'recharge_customer_service_enabled';
const RECHARGE_MIN_AMOUNT_SETTING_KEY = 'recharge_min_amount_minor';
const RECHARGE_MAX_AMOUNT_SETTING_KEY = 'recharge_max_amount_minor';
const SUPPORT_TELEGRAM_ENABLED_SETTING_KEY =
  'support_telegram_notification_enabled';
const UNCONFIGURED_SETTING_VALUE = '未配置';
const MOBILE_CUSTOM_SETTING_KEYS = new Set([
  MOBILE_PLATFORM_NAME_SETTING_KEY,
  MOBILE_LOGO_SETTING_KEY,
  MOBILE_INTRO_SETTING_KEY,
  MOBILE_HOME_FEATURED_ENABLED_SETTING_KEY,
  MOBILE_HOME_FEATURED_TITLE_SETTING_KEY,
  MOBILE_HOME_FEATURED_LOTTERY_CODES_SETTING_KEY,
]);
const RECHARGE_PAYMENT_SETTING_KEYS = new Set([
  RECHARGE_RAINBOW_ENABLED_SETTING_KEY,
  RECHARGE_RAINBOW_PAY_TYPES_SETTING_KEY,
  RECHARGE_CUSTOMER_SERVICE_ENABLED_SETTING_KEY,
]);
const MINOR_MONEY_SETTING_KEYS = new Set([
  RECHARGE_MIN_AMOUNT_SETTING_KEY,
  RECHARGE_MAX_AMOUNT_SETTING_KEY,
]);
const RECHARGE_PAY_TYPE_OPTIONS: SettingSelectOption[] = [
  { label: '支付宝充值', value: 'alipay' },
  { label: '微信充值', value: 'wxpay' },
];

const PERMISSION_SCOPE_OPTIONS: Array<{ label: string; value: PermissionScope }> = [
  { label: '用户', value: 'users' },
  { label: '订单', value: 'orders' },
  { label: '财务', value: 'finance' },
  { label: '客服', value: 'customerService' },
  { label: '管理员', value: 'admins' },
  { label: '角色', value: 'roles' },
  { label: '系统设置', value: 'systemSettings' },
  { label: '彩种', value: 'lotteries' },
  { label: '机器人', value: 'robots' },
  { label: '返利', value: 'rebates' },
];

const USER_SORT_OPTIONS: Array<{ label: string; value: UserListSortBy }> = [
  { label: '用户 ID', value: 'id' },
  { label: '用户名', value: 'username' },
  { label: '邮箱', value: 'email' },
  { label: '用户类型', value: 'kind' },
  { label: '账户状态', value: 'status' },
  { label: '账户余额', value: 'balanceMinor' },
  { label: '上级代理', value: 'agentId' },
  { label: '邀请码', value: 'inviteCode' },
];

const USER_SORT_DIRECTION_OPTIONS: Array<{
  label: string;
  value: UserListSortDirection;
}> = [
  { label: '降序', value: 'desc' },
  { label: '升序', value: 'asc' },
];

export function AccessManagementPage({
  activeModuleKey,
  onDashboardRefresh,
  onOpenUserLedger,
  onOpenUserOrders,
}: AccessManagementPageProps) {
  const [userPageNumber, setUserPageNumber] = useState(1);
  const [userPageSize, setUserPageSize] = useState(20);
  const [userSortBy, setUserSortBy] = useState<UserListSortBy>('id');
  const [userSortDirection, setUserSortDirection] =
    useState<UserListSortDirection>('desc');
  const userQuery = useMemo(
    () => ({
      page: userPageNumber,
      pageSize: userPageSize,
      sortBy: userSortBy,
      sortDirection: userSortDirection,
    }),
    [userPageNumber, userPageSize, userSortBy, userSortDirection],
  );
  const {
    admins,
    changeAdminStatus,
    changeUserStatus,
    error,
    loading,
    refresh,
    registration,
    removeRole,
    reloadMemoryCache,
    resetPassword,
    roles,
    saveAdmin,
    saveRegistration,
    saveRole,
    saveSetting,
    saveUser,
    saving,
    settings,
    userPage,
    users,
  } = useAccessManagement({ userQuery });
  const [section, setSection] = useState<AccessSection>(
    sectionForModule(activeModuleKey),
  );
  const [editingAdminId, setEditingAdminId] = useState<string | null>(null);
  const [editingRoleId, setEditingRoleId] = useState<string | null>(null);
  const [editingUserId, setEditingUserId] = useState<string | null>(null);
  const [adminSheetVisible, setAdminSheetVisible] = useState(false);
  const [roleSheetVisible, setRoleSheetVisible] = useState(false);
  const [userSheetVisible, setUserSheetVisible] = useState(false);
  const [adminForm, setAdminForm] = useState<AdminFormState>(() =>
    emptyAdminForm('role-ops'),
  );
  const [registrationForm, setRegistrationForm] =
    useState<RegistrationConfig | null>(null);
  const [lotteries, setLotteries] = useState<LotteryKind[]>([]);
  const [roleForm, setRoleForm] = useState<RoleFormState>(() => emptyRoleForm());
  const [settingDrafts, setSettingDrafts] = useState<Record<string, string>>({});
  const [userForm, setUserForm] = useState<UserFormState>(() => emptyUserForm());

  useEffect(() => {
    setSection(sectionForModule(activeModuleKey));
  }, [activeModuleKey]);

  useEffect(() => {
    setAdminSheetVisible(false);
    setRoleSheetVisible(false);
    setUserSheetVisible(false);
  }, [section]);

  useEffect(() => {
    if (!adminForm.roleId && roles[0]) {
      setAdminForm((current) => ({ ...current, roleId: roles[0].id }));
    }
  }, [adminForm.roleId, roles]);

  useEffect(() => {
    setSettingDrafts(
      settings.reduce<Record<string, string>>((drafts, setting) => {
        drafts[setting.key] = settingDraftValue(setting);
        return drafts;
      }, {}),
    );
  }, [settings]);

  useEffect(() => {
    if (registration) {
      setRegistrationForm(registration);
    }
  }, [registration]);

  useEffect(() => {
    const controller = new AbortController();
    fetchLotteries(controller.signal)
      .then(setLotteries)
      .catch(() => {
        if (!controller.signal.aborted) {
          setLotteries([]);
        }
      });
    return () => {
      controller.abort();
    };
  }, []);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };
  const isSettingsPage = section === 'settings';

  const submitUser = async () => {
    const saved = await saveUser(userPayload(userForm), editingUserId ?? undefined);
    setUserForm(userFormFromSummary(saved));
    setEditingUserId(saved.id);
    setUserSheetVisible(false);
    onDashboardRefresh();
  };

  const submitAdmin = async () => {
    const isEditing = Boolean(editingAdminId);
    let saved = await saveAdmin(
      adminPayload(adminForm, roles, !isEditing),
      editingAdminId ?? undefined,
    );
    if (editingAdminId && adminForm.password.trim()) {
      saved = await resetPassword(editingAdminId, {
        password: adminForm.password.trim(),
      });
    }
    setAdminForm(adminFormFromSummary(saved));
    setEditingAdminId(saved.id);
    setAdminSheetVisible(false);
    onDashboardRefresh();
  };

  const submitRole = async () => {
    const saved = await saveRole(rolePayload(roleForm), editingRoleId ?? undefined);
    setRoleForm(roleFormFromSummary(saved));
    setEditingRoleId(saved.id);
    setRoleSheetVisible(false);
    onDashboardRefresh();
  };

  const deleteCurrentRole = async () => {
    if (!editingRoleId) {
      return;
    }
    await removeRole(editingRoleId);
    setEditingRoleId(null);
    setRoleForm(emptyRoleForm());
    setRoleSheetVisible(false);
    onDashboardRefresh();
  };

  const submitRegistration = async () => {
    if (!registrationForm) {
      return;
    }
    await saveRegistration(registrationForm);
    onDashboardRefresh();
  };

  return (
    <div className="space-y-5">
      <section className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-xl font-semibold text-ink">
            {isSettingsPage ? '系统设置' : '用户权限管理'}
          </h1>
          <p className="mt-1 text-sm text-slate-500">
            {isSettingsPage
              ? '按功能分类维护后台运行配置、充值参数、图床和注册安全。'
              : '维护用户、后台账号和角色权限范围。'}
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="用户权限接口错误" description={error} /> : null}

      {!isSettingsPage ? (
        <>
          <section className="flex flex-wrap gap-2">
            {ACCESS_SECTIONS.map((item) => (
              <Button
                key={item.key}
                theme={section === item.key ? 'solid' : 'light'}
                onClick={() => setSection(item.key)}
              >
                {item.label}
              </Button>
            ))}
          </section>
        </>
      ) : null}

      {loading ? (
        <Card className="rounded-md border border-line">
          <div className="grid min-h-[320px] place-items-center">
            <Spin tip="正在加载用户权限数据" />
          </div>
        </Card>
      ) : section === 'users' ? (
        <UserSection
          editingId={editingUserId}
          form={userForm}
          loading={loading}
          page={userPage.page}
          pageSize={userPageSize}
          saving={saving}
          sheetVisible={userSheetVisible}
          sortBy={userSortBy}
          sortDirection={userSortDirection}
          totalCount={userPage.totalCount}
          totalPages={userPage.totalPages}
          users={users}
          onClose={() => setUserSheetVisible(false)}
          onEdit={(user) => {
            setEditingUserId(user.id);
            setUserForm(userFormFromSummary(user));
            setUserSheetVisible(true);
          }}
          onNew={() => {
            setEditingUserId(null);
            setUserForm(emptyUserForm());
            setUserSheetVisible(true);
          }}
          onOpenLedger={onOpenUserLedger}
          onOpenOrders={onOpenUserOrders}
          onSetForm={setUserForm}
          onPageChange={setUserPageNumber}
          onPageSizeChange={(pageSize) => {
            setUserPageSize(pageSize);
            setUserPageNumber(1);
          }}
          onSortByChange={(sortBy) => {
            setUserSortBy(sortBy);
            setUserPageNumber(1);
          }}
          onSortDirectionChange={(sortDirection) => {
            setUserSortDirection(sortDirection);
            setUserPageNumber(1);
          }}
          onStatus={(id, status) => {
            void changeUserStatus(id, status).then(onDashboardRefresh);
          }}
          onSubmit={() => void submitUser()}
        />
      ) : section === 'admins' ? (
        <AdminSection
          admins={admins}
          editingId={editingAdminId}
          form={adminForm}
          roles={roles}
          saving={saving}
          sheetVisible={adminSheetVisible}
          onClose={() => setAdminSheetVisible(false)}
          onEdit={(admin) => {
            setEditingAdminId(admin.id);
            setAdminForm(adminFormFromSummary(admin));
            setAdminSheetVisible(true);
          }}
          onNew={() => {
            setEditingAdminId(null);
            setAdminForm(emptyAdminForm(roles[0]?.id ?? ''));
            setAdminSheetVisible(true);
          }}
          onSetForm={setAdminForm}
          onStatus={(id, status) => {
            void changeAdminStatus(id, status).then(onDashboardRefresh);
          }}
          onSubmit={() => void submitAdmin()}
        />
      ) : section === 'roles' ? (
        <RoleSection
          editingId={editingRoleId}
          form={roleForm}
          roles={roles}
          saving={saving}
          sheetVisible={roleSheetVisible}
          onClose={() => setRoleSheetVisible(false)}
          onDelete={() => void deleteCurrentRole()}
          onEdit={(role) => {
            setEditingRoleId(role.id);
            setRoleForm(roleFormFromSummary(role));
            setRoleSheetVisible(true);
          }}
          onNew={() => {
            setEditingRoleId(null);
            setRoleForm(emptyRoleForm());
            setRoleSheetVisible(true);
          }}
          onSetForm={setRoleForm}
          onSubmit={() => void submitRole()}
        />
      ) : (
        <SettingsSection
          drafts={settingDrafts}
          lotteries={lotteries}
          registration={registrationForm}
          saving={saving}
          settings={settings}
          onDraftChange={(key, value) =>
            setSettingDrafts((current) => ({ ...current, [key]: value }))
          }
          onRegistrationChange={setRegistrationForm}
          onReloadMemoryCache={async () => {
            const result = await reloadMemoryCache();
            onDashboardRefresh();
            return result;
          }}
          onSaveRegistration={() => void submitRegistration()}
          onSaveSetting={(key) => {
            const submitValue = settingSubmitValue(key, settingDrafts[key] ?? '');
            if (submitValue === null) {
              return;
            }
            void saveSetting(key, submitValue).then(onDashboardRefresh);
          }}
        />
      )}
    </div>
  );
}

function UserSection({
  editingId,
  form,
  loading,
  onClose,
  onEdit,
  onNew,
  onOpenLedger,
  onOpenOrders,
  onPageChange,
  onPageSizeChange,
  onSetForm,
  onSortByChange,
  onSortDirectionChange,
  onStatus,
  onSubmit,
  page,
  pageSize,
  saving,
  sheetVisible,
  sortBy,
  sortDirection,
  totalCount,
  totalPages,
  users,
}: {
  editingId: string | null;
  form: UserFormState;
  loading: boolean;
  onClose: () => void;
  onEdit: (user: AdminUserSummary) => void;
  onNew: () => void;
  onOpenLedger: (user: AdminUserSummary) => void;
  onOpenOrders: (user: AdminUserSummary) => void;
  onPageChange: (page: number) => void;
  onPageSizeChange: (pageSize: number) => void;
  onSetForm: Dispatch<SetStateAction<UserFormState>>;
  onSortByChange: (sortBy: UserListSortBy) => void;
  onSortDirectionChange: (sortDirection: UserListSortDirection) => void;
  onStatus: (id: string, status: UserStatus) => void;
  onSubmit: () => void;
  page: number;
  pageSize: number;
  saving: boolean;
  sheetVisible: boolean;
  sortBy: UserListSortBy;
  sortDirection: UserListSortDirection;
  totalCount: number;
  totalPages: number;
  users: AdminUserSummary[];
}) {
  return (
    <section className="space-y-4">
      <Card className="rounded-md border border-line">
        <div className="mb-3 flex flex-col gap-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h2 className="text-base font-semibold text-ink">用户列表</h2>
            <div className="flex items-center gap-2">
              <Tag color="cyan">当前页 {users.length} 个用户</Tag>
              <Button
                icon={<UserPlus size={15} />}
                size="small"
                theme="solid"
                onClick={onNew}
              >
                新建用户
              </Button>
            </div>
          </div>

          <div className="flex flex-wrap items-center justify-between gap-3 rounded-md border border-slate-100 bg-slate-50 px-3 py-2">
            <div className="flex flex-nowrap items-center gap-2 overflow-x-auto whitespace-nowrap text-sm text-slate-600">
              <span className="text-xs font-medium text-slate-500">排序</span>
              <Select
                className="form-input min-w-[132px]"
                value={sortBy}
                onChange={(value) =>
                  onSortByChange((value as UserListSortBy) || 'id')
                }
              >
                {USER_SORT_OPTIONS.map((option) => (
                  <Select.Option key={option.value} value={option.value}>
                    {option.label}
                  </Select.Option>
                ))}
              </Select>
              <Select
                className="form-input min-w-[104px]"
                value={sortDirection}
                onChange={(value) =>
                  onSortDirectionChange(
                    (value as UserListSortDirection) || 'desc',
                  )
                }
              >
                {USER_SORT_DIRECTION_OPTIONS.map((option) => (
                  <Select.Option key={option.value} value={option.value}>
                    {option.label}
                  </Select.Option>
                ))}
              </Select>
            </div>
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
        <div className="overflow-x-auto">
          <table className="w-full min-w-[960px] text-left text-sm">
            <thead className="border-b border-line text-xs text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">用户</th>
                <th className="py-2 pr-4 font-medium">类型</th>
                <th className="py-2 pr-4 font-medium">余额</th>
                <th className="py-2 pr-4 font-medium">上级代理</th>
                <th className="py-2 pr-4 font-medium">邀请码</th>
                <th className="py-2 pr-4 font-medium">状态</th>
                <th className="py-2 pr-4 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {users.map((user) => (
                <tr
                  key={user.id}
                  className={`border-b border-slate-100 ${
                    editingId === user.id ? 'bg-teal-50/60' : ''
                  }`}
                >
                  <td className="py-3 pr-4">
                    <button
                      className="text-left font-semibold text-accent"
                      type="button"
                      onClick={() => onEdit(user)}
                    >
                      {user.username}
                    </button>
                    <div className="mt-1 text-xs text-slate-400">
                      {user.id}
                      {user.email ? ` · ${user.email}` : ''}
                    </div>
                  </td>
                  <td className="py-3 pr-4">
                    <Tag color={user.kind === 'agent' ? 'purple' : 'blue'}>
                      {userKindText(user.kind)}
                    </Tag>
                  </td>
                  <td className="py-3 pr-4 text-slate-600">
                    {formatMoney(user.balanceMinor)}
                  </td>
                  <td className="py-3 pr-4 text-slate-600">
                    {user.agentId ? (
                      <div>
                        <div className="font-medium text-ink">
                          {user.agentUsername ?? '未知代理'}
                        </div>
                        <div className="mt-1 text-xs text-slate-400">
                          {user.agentId}
                        </div>
                      </div>
                    ) : (
                      <span className="text-slate-400">无</span>
                    )}
                  </td>
                  <td className="py-3 pr-4">
                    {user.inviteCode ? (
                      <div className="flex max-w-[240px] flex-wrap gap-1">
                        <Tag color={user.kind === 'agent' ? 'teal' : 'grey'}>
                          {user.inviteCode}
                        </Tag>
                        <Tag color={user.kind === 'agent' ? 'green' : 'orange'}>
                          {user.kind === 'agent' ? '可邀请' : '无邀请功能'}
                        </Tag>
                      </div>
                    ) : (
                      <span className="text-slate-400">未配置</span>
                    )}
                  </td>
                  <td className="py-3 pr-4">
                    <Tag color={userStatusColor(user.status)}>
                      {userStatusText(user.status)}
                    </Tag>
                  </td>
                  <td className="py-3 pr-4">
                    <div className="flex flex-wrap gap-2">
                      <Button size="small" onClick={() => onEdit(user)}>
                        编辑
                      </Button>
                      <Button size="small" onClick={() => onOpenOrders(user)}>
                        注单
                      </Button>
                      <Button size="small" onClick={() => onOpenLedger(user)}>
                        流水
                      </Button>
                      <Button
                        disabled={user.status === 'active'}
                        size="small"
                        onClick={() => onStatus(user.id, 'active')}
                      >
                        启用
                      </Button>
                      <Button
                        disabled={user.status === 'suspended'}
                        size="small"
                        onClick={() => onStatus(user.id, 'suspended')}
                      >
                        停用
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <SideSheet
        aria-label="用户维护"
        title="用户维护"
        visible={sheetVisible}
        width={460}
        onCancel={() => onClose()}
      >
        <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
          <Field label="用户 ID">
            <Input
              className="form-input"
              disabled
              value={form.id}
            />
            <p className="text-xs text-slate-400">用户 ID 由系统生成，创建后不可编辑。</p>
          </Field>
          <Field label="用户名">
            <Input
              className="form-input"
              value={form.username}
              onChange={(value) =>
                setFormValue(onSetForm, 'username', value)
              }
            />
          </Field>
          <Field label="邮箱">
            <Input
              className="form-input"
              value={form.email}
              onChange={(value) =>
                setFormValue(onSetForm, 'email', value)
              }
            />
          </Field>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-1">
          <Field label="用户类型">
            <Select
              className="form-input"
              value={form.kind}
              onChange={(value) =>
                setFormValue(onSetForm, 'kind', (value as UserKind) || 'regular')
              }
            >
              <Select.Option value="regular">普通用户</Select.Option>
              <Select.Option value="agent">代理</Select.Option>
            </Select>
          </Field>
          <Field label="状态">
            <Select
              className="form-input"
              value={form.status}
              onChange={(value) =>
                setFormValue(onSetForm, 'status', (value as UserStatus) || 'active')
              }
            >
              <Select.Option value="active">启用</Select.Option>
              <Select.Option value="suspended">停用</Select.Option>
              <Select.Option value="locked">锁定</Select.Option>
            </Select>
          </Field>
          </div>
          <Field label="账户余额">
            <Input
              className="form-input"
              disabled
              value={formatMoney(numberField(form.balanceMinor))}
            />
            <p className="text-xs text-slate-400">余额只能通过财务管理的手动调账入口调整。</p>
          </Field>
          <Field label="上级代理 ID">
            <Input
              className="form-input"
              value={form.agentId}
              onChange={(value) =>
                setFormValue(onSetForm, 'agentId', value)
              }
            />
          </Field>
          <Field label="邀请码">
            <Input
              className="form-input"
              disabled
              placeholder="创建用户后由后端自动生成"
              value={form.inviteCode || '创建后自动生成'}
            />
            <p className="text-xs text-slate-400">邀请码用于邀请关系识别，不允许在用户维护中修改。</p>
          </Field>
          <div className="flex flex-wrap gap-2">
            <Button
              disabled={saving}
              icon={<Save size={16} />}
              theme="solid"
              onClick={onSubmit}
            >
              {editingId ? '保存用户' : '新增用户'}
            </Button>
            <Button onClick={onNew}>新建</Button>
          </div>
        </form>
      </SideSheet>
    </section>
  );
}

function AdminSection({
  admins,
  editingId,
  form,
  onClose,
  onEdit,
  onNew,
  onSetForm,
  onStatus,
  onSubmit,
  roles,
  saving,
  sheetVisible,
}: {
  admins: AdminSummary[];
  editingId: string | null;
  form: AdminFormState;
  onClose: () => void;
  onEdit: (admin: AdminSummary) => void;
  onNew: () => void;
  onSetForm: Dispatch<SetStateAction<AdminFormState>>;
  onStatus: (id: string, status: UserStatus) => void;
  onSubmit: () => void;
  roles: AdminRole[];
  saving: boolean;
  sheetVisible: boolean;
}) {
  return (
    <section className="space-y-4">
      <Card className="rounded-md border border-line">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-base font-semibold text-ink">后台账号</h2>
          <div className="flex items-center gap-2">
            <Tag color="cyan">{admins.length} 个账号</Tag>
            <Button
              icon={<ShieldCheck size={15} />}
              size="small"
              theme="solid"
              onClick={onNew}
            >
              新建账号
            </Button>
          </div>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[720px] text-left text-sm">
            <thead className="border-b border-line text-xs text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">账号</th>
                <th className="py-2 pr-4 font-medium">角色</th>
                <th className="py-2 pr-4 font-medium">状态</th>
                <th className="py-2 pr-4 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {admins.map((admin) => (
                <tr
                  key={admin.id}
                  className={`border-b border-slate-100 ${
                    editingId === admin.id ? 'bg-teal-50/60' : ''
                  }`}
                >
                  <td className="py-3 pr-4">
                    <button
                      className="text-left font-semibold text-accent"
                      type="button"
                      onClick={() => onEdit(admin)}
                    >
                      {admin.username}
                    </button>
                    <div className="mt-1 text-xs text-slate-400">{admin.id}</div>
                  </td>
                  <td className="py-3 pr-4 text-slate-600">{admin.roleName}</td>
                  <td className="py-3 pr-4">
                    <Tag color={userStatusColor(admin.status)}>
                      {userStatusText(admin.status)}
                    </Tag>
                  </td>
                  <td className="py-3 pr-4">
                    <div className="flex flex-wrap gap-2">
                      <Button size="small" onClick={() => onEdit(admin)}>
                        编辑
                      </Button>
                      <Button
                        disabled={admin.status === 'active'}
                        size="small"
                        onClick={() => onStatus(admin.id, 'active')}
                      >
                        启用
                      </Button>
                      <Button
                        disabled={admin.status === 'locked'}
                        size="small"
                        onClick={() => onStatus(admin.id, 'locked')}
                      >
                        锁定
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <SideSheet
        aria-label="账号维护"
        title="账号维护"
        visible={sheetVisible}
        width={460}
        onCancel={() => onClose()}
      >
        <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
          <Field label="管理员 ID">
            <Input
              className="form-input"
              value={form.id}
              onChange={(value) => setFormValue(onSetForm, 'id', value)}
            />
          </Field>
          <Field label="用户名">
            <Input
              className="form-input"
              value={form.username}
              onChange={(value) =>
                setFormValue(onSetForm, 'username', value)
              }
            />
          </Field>
          <Field label={editingId ? '重置密码' : '初始密码'}>
            <Input
              autoComplete="new-password"
              className="form-input"
              placeholder={editingId ? '留空则不修改密码' : '至少 8 位'}
              type="password"
              value={form.password}
              onChange={(value) =>
                setFormValue(onSetForm, 'password', value)
              }
            />
          </Field>
              <Field label="角色">
                <Select
                  className="form-input"
                  value={form.roleId}
                  onChange={(value) =>
                    setFormValue(onSetForm, 'roleId', (value as string) || '')
                  }
                >
                  {roles.map((role) => (
                    <Select.Option key={role.id} value={role.id}>
                      {role.name}
                    </Select.Option>
                  ))}
                </Select>
              </Field>
              <Field label="状态">
                <Select
                  className="form-input"
                  value={form.status}
                  onChange={(value) =>
                    setFormValue(onSetForm, 'status', (value as UserStatus) || 'active')
                  }
                >
                  <Select.Option value="active">启用</Select.Option>
                  <Select.Option value="suspended">停用</Select.Option>
                <Select.Option value="locked">锁定</Select.Option>
              </Select>
            </Field>
          <div className="flex flex-wrap gap-2">
            <Button
              disabled={saving || !form.roleId || (!editingId && form.password.trim().length < 8)}
              icon={<Save size={16} />}
              theme="solid"
              onClick={onSubmit}
            >
              {editingId ? '保存账号' : '新增账号'}
            </Button>
            <Button onClick={onNew}>新建</Button>
          </div>
        </form>
      </SideSheet>
    </section>
  );
}

function RoleSection({
  editingId,
  form,
  onClose,
  onDelete,
  onEdit,
  onNew,
  onSetForm,
  onSubmit,
  roles,
  saving,
  sheetVisible,
}: {
  editingId: string | null;
  form: RoleFormState;
  onClose: () => void;
  onDelete: () => void;
  onEdit: (role: AdminRole) => void;
  onNew: () => void;
  onSetForm: Dispatch<SetStateAction<RoleFormState>>;
  onSubmit: () => void;
  roles: AdminRole[];
  saving: boolean;
  sheetVisible: boolean;
}) {
  return (
    <section className="space-y-4">
      <Card className="rounded-md border border-line">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-base font-semibold text-ink">角色列表</h2>
          <div className="flex items-center gap-2">
            <Tag color="cyan">{roles.length} 个角色</Tag>
            <Button
              icon={<ShieldCheck size={15} />}
              size="small"
              theme="solid"
              onClick={onNew}
            >
              新建角色
            </Button>
          </div>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[760px] text-left text-sm">
            <thead className="border-b border-line text-xs text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">角色</th>
                <th className="py-2 pr-4 font-medium">权限范围</th>
                <th className="py-2 pr-4 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {roles.map((role) => (
                <tr
                  key={role.id}
                  className={`border-b border-slate-100 ${
                    editingId === role.id ? 'bg-teal-50/60' : ''
                  }`}
                >
                  <td className="py-3 pr-4">
                    <button
                      className="text-left font-semibold text-accent"
                      type="button"
                      onClick={() => onEdit(role)}
                    >
                      {role.name}
                    </button>
                    <div className="mt-1 text-xs text-slate-400">{role.id}</div>
                  </td>
                  <td className="py-3 pr-4">
                    <div className="flex flex-wrap gap-2">
                      {role.scopes.map((scope) => (
                        <Tag key={scope} color="grey">
                          {permissionScopeText(scope)}
                        </Tag>
                      ))}
                    </div>
                  </td>
                  <td className="py-3 pr-4">
                    <Button size="small" onClick={() => onEdit(role)}>
                      编辑
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

      <SideSheet
        aria-label="角色维护"
        title="角色维护"
        visible={sheetVisible}
        width={480}
        onCancel={() => onClose()}
      >
        <form className="space-y-4" onSubmit={(event) => event.preventDefault()}>
          <Field label="角色 ID">
            <Input
              className="form-input"
              value={form.id}
              onChange={(value) => setFormValue(onSetForm, 'id', value)}
            />
          </Field>
          <Field label="角色名称">
            <Input
              className="form-input"
              value={form.name}
              onChange={(value) => setFormValue(onSetForm, 'name', value)}
            />
          </Field>
          <div className="space-y-2">
            <div className="text-sm font-medium text-slate-600">权限范围</div>
            <div className="grid grid-cols-2 gap-2">
              {PERMISSION_SCOPE_OPTIONS.map((scope) => (
                <label
                  key={scope.value}
                  className="flex items-center gap-2 rounded border border-slate-200 bg-white px-2 py-2 text-sm text-slate-600"
                >
                  <input
                    checked={form.scopes.includes(scope.value)}
                    type="checkbox"
                    onChange={(event) =>
                      toggleScope(onSetForm, scope.value, event.target.checked)
                    }
                  />
                  {scope.label}
                </label>
              ))}
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              disabled={saving || form.scopes.length === 0}
              icon={<Save size={16} />}
              theme="solid"
              onClick={onSubmit}
            >
              {editingId ? '保存角色' : '新增角色'}
            </Button>
            <Button onClick={onNew}>新建</Button>
            <Button
              disabled={!editingId || saving}
              icon={<Trash2 size={16} />}
              onClick={onDelete}
            >
              删除
            </Button>
          </div>
        </form>
      </SideSheet>
    </section>
  );
}

function SettingsSection({
  drafts,
  lotteries,
  onDraftChange,
  onRegistrationChange,
  onReloadMemoryCache,
  onSaveRegistration,
  onSaveSetting,
  registration,
  saving,
  settings,
}: {
  drafts: Record<string, string>;
  lotteries: LotteryKind[];
  onDraftChange: (key: string, value: string) => void;
  onRegistrationChange: Dispatch<SetStateAction<RegistrationConfig | null>>;
  onReloadMemoryCache: () => Promise<MemoryCacheReloadResult>;
  onSaveRegistration: () => void;
  onSaveSetting: (key: string) => void;
  registration: RegistrationConfig | null;
  saving: boolean;
  settings: Array<{ description: string; key: string; value: string }>;
}) {
  const imageBedUploadUrl = readSettingValue(settings, 'image_bed_upload_url');
  const imageBedUploadToken = readSettingValue(settings, 'image_bed_authorization_token');
  const imageBedUploadField =
    (drafts['image_bed_upload_field'] ?? '').trim() ||
    readSettingValue(settings, 'image_bed_upload_field') ||
    'file';
  const imageBedResultUrlField =
    (drafts['image_bed_result_url_field'] ?? '').trim() ||
    readSettingValue(settings, 'image_bed_result_url_field') ||
    'links.download';
  const [settingKeyword, setSettingKeyword] = useState('');
  const [cacheRefreshing, setCacheRefreshing] = useState(false);
  const [lastCacheReloadResult, setLastCacheReloadResult] =
    useState<MemoryCacheReloadResult | null>(null);
  const imageBedMissingConfigs = [
    imageBedUploadUrl.trim() ? null : '上传地址',
    imageBedUploadToken.trim() ? null : 'Token',
    imageBedUploadField.trim() ? null : '上传字段名',
  ].filter(Boolean) as string[];

  const filteredSettings = useMemo(() => settings.filter((setting) => {
    const keyword = settingKeyword.trim().toLowerCase();
    if (!keyword) {
      return true;
    }
    return (
      setting.key.toLowerCase().includes(keyword) ||
      setting.description.toLowerCase().includes(keyword)
    );
  }), [settingKeyword, settings]);
  const groupedSettings = useMemo(
    () => settingsGroups(filteredSettings),
    [filteredSettings],
  );
  const [activeSettingGroup, setActiveSettingGroup] = useState('手机端设置');

  useEffect(() => {
    if (groupedSettings.length === 0) {
      return;
    }
    if (!groupedSettings.some(([groupName]) => groupName === activeSettingGroup)) {
      setActiveSettingGroup(groupedSettings[0][0]);
    }
  }, [activeSettingGroup, groupedSettings]);

  const handleReloadMemoryCache = async () => {
    const confirmed = window.confirm(
      '确定从数据库重新刷新后端内存缓存吗？如果你刚刚手动清表或改库，刷新后内存会以数据库当前内容为准。',
    );
    if (!confirmed) {
      return;
    }

    setCacheRefreshing(true);
    try {
      const result = await onReloadMemoryCache();
      setLastCacheReloadResult(result);
      Toast.success(`内存缓存已刷新：${result.reloadedModules.length} 个模块`);
    } catch (error) {
      Toast.error(error instanceof Error ? error.message : '内存缓存刷新失败');
    } finally {
      setCacheRefreshing(false);
    }
  };

  return (
    <section className="space-y-4">
      <Card className="rounded-md border border-line">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <div>
            <h2 className="text-base font-semibold text-ink">系统设置</h2>
            <p className="mt-1 text-xs text-slate-500">
              手动清表或直接改库后，可用维护按钮让后端重新读取数据库快照。
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Tag color="cyan">{settings.length} 项</Tag>
            <Button
              disabled={saving || cacheRefreshing}
              icon={<RefreshCcw size={15} />}
              loading={cacheRefreshing}
              size="small"
              onClick={() => void handleReloadMemoryCache()}
            >
              刷新内存缓存
            </Button>
          </div>
        </div>
        {lastCacheReloadResult ? (
          <div className="mb-3 rounded border border-emerald-100 bg-emerald-50 p-3 text-xs text-emerald-800">
            <div className="flex flex-wrap items-center gap-2">
              <Tag color="green">已刷新 {lastCacheReloadResult.reloadedModules.length}</Tag>
              <Tag color="blue">
                数据库直读 {lastCacheReloadResult.databaseDirectModules.length}
              </Tag>
              <Tag color="grey">跳过 {lastCacheReloadResult.skippedModules.length}</Tag>
              <span>时间：{lastCacheReloadResult.refreshedAt}</span>
            </div>
            {lastCacheReloadResult.reloadedModules.length > 0 ? (
              <p className="mt-2 leading-5">
                已刷新模块：{lastCacheReloadResult.reloadedModules.join('、')}
              </p>
            ) : null}
          </div>
        ) : null}
        <div className="mb-3">
          <Input
            className="form-input"
            placeholder="搜索配置项 / 说明"
            value={settingKeyword}
            onChange={(value) => setSettingKeyword(value)}
          />
        </div>
        {groupedSettings.length === 0 ? (
          <div className="rounded border border-slate-200 bg-slate-50 p-4 text-sm text-slate-500">
            未找到匹配的系统配置项，可清空关键字后重试。
          </div>
        ) : (
          <Tabs
            activeKey={activeSettingGroup}
            collapsible
            onChange={(key) => setActiveSettingGroup(String(key))}
          >
            {groupedSettings.map(([groupName, items]) => {
              const fieldItems =
                groupName === '手机端设置'
                  ? items.filter(
                      (setting) => !MOBILE_CUSTOM_SETTING_KEYS.has(setting.key),
                    )
                  : groupName === '充值设置'
                    ? items.filter(
                        (setting) =>
                          !RECHARGE_PAYMENT_SETTING_KEYS.has(setting.key),
                      )
                  : items;

              return (
                <Tabs.TabPane
                  key={groupName}
                  itemKey={groupName}
                  tab={
                    <span className="inline-flex items-center gap-2">
                      <span>{groupName}</span>
                      <Tag color="grey">{items.length}</Tag>
                    </span>
                  }
                >
                  <div className="space-y-4 pt-3">
                    {fieldItems.length > 0 ? (
                      <SettingFields
                        drafts={drafts}
                        items={fieldItems}
                        saving={saving}
                        onDraftChange={onDraftChange}
                        onSaveSetting={onSaveSetting}
                      />
                    ) : null}

                    {groupName === '手机端设置' ? (
                      <MobileSettingsPanel
                        drafts={drafts}
                        imageBedMissingConfigs={imageBedMissingConfigs}
                        imageBedUploadField={imageBedUploadField}
                        lotteries={lotteries}
                        saving={saving}
                        settings={settings}
                        onDraftChange={onDraftChange}
                        onSaveSetting={onSaveSetting}
                      />
                    ) : null}

                    {groupName === '充值设置' ? (
                      <RechargePaymentPanel
                        drafts={drafts}
                        saving={saving}
                        settings={settings}
                        onDraftChange={onDraftChange}
                        onSaveSetting={onSaveSetting}
                      />
                    ) : null}

                    {groupName === '注册与安全' ? (
                      <RegistrationSettingsPanel
                        registration={registration}
                        saving={saving}
                        onRegistrationChange={onRegistrationChange}
                        onSaveRegistration={onSaveRegistration}
                      />
                    ) : null}

                    {groupName === '图床设置' ? (
                      <ImageBedTestPanel
                        imageBedMissingConfigs={imageBedMissingConfigs}
                        imageBedResultUrlField={imageBedResultUrlField}
                        imageBedUploadField={imageBedUploadField}
                        imageBedUploadUrl={imageBedUploadUrl}
                        saving={saving}
                      />
                    ) : null}
                  </div>
                </Tabs.TabPane>
              );
            })}
          </Tabs>
        )}
      </Card>
    </section>
  );
}

function PanelTitle({ icon, title }: { icon: ReactNode; title: string }) {
  return (
    <div className="mb-4 flex items-center gap-3">
      <div className="grid h-10 w-10 place-items-center rounded-md bg-teal-50 text-teal-700">
        {icon}
      </div>
      <h2 className="text-base font-semibold text-ink">{title}</h2>
    </div>
  );
}

function SettingFields({
  drafts,
  items,
  onDraftChange,
  onSaveSetting,
  saving,
}: {
  drafts: Record<string, string>;
  items: SystemSettingItem[];
  onDraftChange: (key: string, value: string) => void;
  onSaveSetting: (key: string) => void;
  saving: boolean;
}) {
  return (
    <div className="grid gap-3 xl:grid-cols-2">
      {items.map((setting) => {
        const draftValue = drafts[setting.key] ?? setting.value;
        const selectOptions = settingSelectOptions(setting.key, draftValue);
        const usesYuanInput = isMinorMoneySetting(setting.key);

        return (
          <div
            key={setting.key}
            className="grid min-h-[112px] gap-2 rounded border border-slate-100 bg-white p-3"
          >
            <div className="flex flex-wrap items-start justify-between gap-2">
              <div className="min-w-0">
                <p className="break-all text-sm font-medium text-ink">
                  {setting.key}
                </p>
                <p className="mt-1 text-xs leading-5 text-slate-500">
                  {settingDescription(setting)}
                </p>
              </div>
              <Button
                disabled={saving}
                size="small"
                onClick={() => onSaveSetting(setting.key)}
              >
                保存
              </Button>
            </div>
            {selectOptions.length > 0 ? (
              <Select
                className="form-input"
                value={draftValue}
                onChange={(value) =>
                  onDraftChange(setting.key, String(value ?? ''))
                }
              >
                {selectOptions.map((option) => (
                  <Select.Option key={option.value} value={option.value}>
                    {option.label}
                  </Select.Option>
                ))}
              </Select>
            ) : (
              <Input
                className="form-input"
                inputMode={usesYuanInput ? 'decimal' : undefined}
                placeholder={usesYuanInput ? '例如 100 或 100.00' : undefined}
                value={draftValue}
                onChange={(value) => onDraftChange(setting.key, value)}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

function RechargePaymentPanel({
  drafts,
  onDraftChange,
  onSaveSetting,
  saving,
  settings,
}: {
  drafts: Record<string, string>;
  onDraftChange: (key: string, value: string) => void;
  onSaveSetting: (key: string) => void;
  saving: boolean;
  settings: SystemSettingItem[];
}) {
  const rainbowEnabledValue =
    draftSettingValue(settings, drafts, RECHARGE_RAINBOW_ENABLED_SETTING_KEY) ||
    'false';
  const customerServiceEnabledValue =
    draftSettingValue(
      settings,
      drafts,
      RECHARGE_CUSTOMER_SERVICE_ENABLED_SETTING_KEY,
    ) || 'false';
  const payTypesValue = draftSettingValue(
    settings,
    drafts,
    RECHARGE_RAINBOW_PAY_TYPES_SETTING_KEY,
  );
  const selectedPayTypes = settingListValue(payTypesValue);
  const rainbowReady =
    rainbowEnabledValue === 'true' && selectedPayTypes.length > 0;

  return (
    <div className="rounded border border-slate-200 bg-slate-50 p-3">
      <PanelTitle icon={<CreditCard size={18} />} title="支付方式开关" />
      <div className="grid gap-3 xl:grid-cols-3">
        <div className="rounded border border-slate-200 bg-white p-3">
          <div className="mb-3 flex items-start justify-between gap-3">
            <div>
              <p className="text-sm font-semibold text-ink">彩虹易支付</p>
              <p className="mt-1 text-xs leading-5 text-slate-500">
                开启后用户端展示在线充值入口，仍需要下方至少开启一种支付方式。
              </p>
            </div>
            <Tag color={rainbowReady ? 'green' : 'grey'}>
              {rainbowReady ? '可用' : '不可用'}
            </Tag>
          </div>
          <Select
            className="form-input"
            value={rainbowEnabledValue}
            onChange={(value) =>
              onDraftChange(
                RECHARGE_RAINBOW_ENABLED_SETTING_KEY,
                String(value ?? 'false'),
              )
            }
          >
            <Select.Option value="true">开启彩虹易支付</Select.Option>
            <Select.Option value="false">关闭彩虹易支付</Select.Option>
          </Select>
          <div className="mt-3 flex justify-end">
            <Button
              disabled={saving}
              icon={<Save size={16} />}
              size="small"
              onClick={() => onSaveSetting(RECHARGE_RAINBOW_ENABLED_SETTING_KEY)}
            >
              保存开关
            </Button>
          </div>
        </div>

        <div className="rounded border border-slate-200 bg-white p-3">
          <div className="mb-3 flex items-start justify-between gap-3">
            <div>
              <p className="text-sm font-semibold text-ink">客服直充</p>
              <p className="mt-1 text-xs leading-5 text-slate-500">
                开启后用户可以提交直充申请，并进入客服会话继续沟通。
              </p>
            </div>
            <Tag color={customerServiceEnabledValue === 'true' ? 'green' : 'grey'}>
              {customerServiceEnabledValue === 'true' ? '已开启' : '已关闭'}
            </Tag>
          </div>
          <Select
            className="form-input"
            value={customerServiceEnabledValue}
            onChange={(value) =>
              onDraftChange(
                RECHARGE_CUSTOMER_SERVICE_ENABLED_SETTING_KEY,
                String(value ?? 'false'),
              )
            }
          >
            <Select.Option value="true">开启客服直充</Select.Option>
            <Select.Option value="false">关闭客服直充</Select.Option>
          </Select>
          <div className="mt-3 flex justify-end">
            <Button
              disabled={saving}
              icon={<Save size={16} />}
              size="small"
              onClick={() =>
                onSaveSetting(RECHARGE_CUSTOMER_SERVICE_ENABLED_SETTING_KEY)
              }
            >
              保存开关
            </Button>
          </div>
        </div>

        <div className="rounded border border-slate-200 bg-white p-3">
          <div className="mb-3 flex items-start justify-between gap-3">
            <div>
              <p className="text-sm font-semibold text-ink">在线支付方式</p>
              <p className="mt-1 text-xs leading-5 text-slate-500">
                支付宝和微信会写入彩虹易支付 `payTypes`，用户端只展示已选方式。
              </p>
            </div>
            <Tag color={selectedPayTypes.length > 0 ? 'blue' : 'orange'}>
              {selectedPayTypes.length > 0
                ? `${selectedPayTypes.length} 个方式`
                : '未开启'}
            </Tag>
          </div>
          <Select
            className="form-input"
            multiple
            placeholder="选择要开启的支付方式"
            value={selectedPayTypes}
            onChange={(value) =>
              onDraftChange(
                RECHARGE_RAINBOW_PAY_TYPES_SETTING_KEY,
                settingListText(value),
              )
            }
          >
            {RECHARGE_PAY_TYPE_OPTIONS.map((option) => (
              <Select.Option key={option.value} value={option.value}>
                {option.label}
              </Select.Option>
            ))}
          </Select>
          <p className="mt-2 text-xs leading-5 text-slate-500">
            如果不选择任何支付方式，用户端不会展示彩虹易支付入口。
          </p>
          <div className="mt-3 flex justify-end">
            <Button
              disabled={saving}
              icon={<Save size={16} />}
              size="small"
              onClick={() => onSaveSetting(RECHARGE_RAINBOW_PAY_TYPES_SETTING_KEY)}
            >
              保存方式
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

function MobileSettingsPanel({
  drafts,
  imageBedMissingConfigs,
  imageBedUploadField,
  lotteries,
  onDraftChange,
  onSaveSetting,
  saving,
  settings,
}: {
  drafts: Record<string, string>;
  imageBedMissingConfigs: string[];
  imageBedUploadField: string;
  lotteries: LotteryKind[];
  onDraftChange: (key: string, value: string) => void;
  onSaveSetting: (key: string) => void;
  saving: boolean;
  settings: SystemSettingItem[];
}) {
  const platformNameValue = draftSettingValue(
    settings,
    drafts,
    MOBILE_PLATFORM_NAME_SETTING_KEY,
  );
  const logoValue = draftSettingValue(settings, drafts, MOBILE_LOGO_SETTING_KEY);
  const introValue = draftSettingValue(settings, drafts, MOBILE_INTRO_SETTING_KEY);
  const featuredEnabledValue =
    draftSettingValue(settings, drafts, MOBILE_HOME_FEATURED_ENABLED_SETTING_KEY) || 'false';
  const featuredTitleValue = draftSettingValue(
    settings,
    drafts,
    MOBILE_HOME_FEATURED_TITLE_SETTING_KEY,
  ) || '高频极速';
  const featuredLotteryCodesValue = draftSettingValue(
    settings,
    drafts,
    MOBILE_HOME_FEATURED_LOTTERY_CODES_SETTING_KEY,
  );
  const selectedFeaturedLotteryCodes = settingListValue(featuredLotteryCodesValue);
  const logoImageUrl =
    logoValue && logoValue !== UNCONFIGURED_SETTING_VALUE ? logoValue : '';

  return (
    <div className="rounded border border-slate-200 bg-slate-50 p-3">
      <PanelTitle icon={<Smartphone size={18} />} title="手机端展示配置" />
      <div className="grid gap-4 lg:grid-cols-[280px_minmax(0,1fr)]">
        <ImageUploadAvatar
          clearLabel="清空 Logo"
          description="建议上传清晰的方形或横向透明底图片，上传后点击右侧保存。"
          disabled={saving}
          errorTitle="手机端 Logo 上传失败"
          failureMessage="上传失败"
          imageUrl={logoImageUrl}
          missingConfigLabels={imageBedMissingConfigs}
          requireImageUrl
          showResultPanel={false}
          successMessage="Logo 上传成功，记得保存配置"
          title="手机端 Logo 图片"
          uploadFieldName={imageBedUploadField || 'file'}
          uploadingText="正在上传手机端 Logo..."
          warningTitle="图床配置不完整"
          onClear={() => onDraftChange(MOBILE_LOGO_SETTING_KEY, UNCONFIGURED_SETTING_VALUE)}
          onUploaded={(url) => onDraftChange(MOBILE_LOGO_SETTING_KEY, url)}
        />

        <div className="grid content-start gap-3">
          <div className="rounded border border-slate-200 bg-white p-3">
            <div className="mb-2 flex items-center gap-2 text-sm font-medium text-ink">
              <Smartphone size={16} />
              平台名称
            </div>
            <Input
              className="form-input"
              placeholder="填写手机端显示的平台名称"
              value={platformNameValue}
              onChange={(value) =>
                onDraftChange(MOBILE_PLATFORM_NAME_SETTING_KEY, value)
              }
            />
            <div className="mt-2 flex justify-end">
              <Button
                disabled={saving}
                icon={<Save size={16} />}
                size="small"
                onClick={() => onSaveSetting(MOBILE_PLATFORM_NAME_SETTING_KEY)}
              >
                保存名称
              </Button>
            </div>
          </div>

          <div className="rounded border border-slate-200 bg-white p-3">
            <div className="mb-2 flex items-center gap-2 text-sm font-medium text-ink">
              <ImageIcon size={16} />
              Logo 图片链接
            </div>
            <Input
              className="form-input"
              placeholder="上传或粘贴手机端 Logo 图片链接"
              value={logoValue}
              onChange={(value) => onDraftChange(MOBILE_LOGO_SETTING_KEY, value)}
            />
            <div className="mt-2 flex justify-end">
              <Button
                disabled={saving}
                icon={<Save size={16} />}
                size="small"
                onClick={() => onSaveSetting(MOBILE_LOGO_SETTING_KEY)}
              >
                保存 Logo
              </Button>
            </div>
          </div>

          <div className="rounded border border-slate-200 bg-white p-3">
            <div className="mb-2 flex items-center gap-2 text-sm font-medium text-ink">
              <Settings size={16} />
              手机端介绍
            </div>
            <Input
              className="form-input"
              placeholder="填写手机端首页或关于页面展示的介绍"
              value={introValue}
              onChange={(value) => onDraftChange(MOBILE_INTRO_SETTING_KEY, value)}
            />
            <div className="mt-2 flex justify-end">
              <Button
                disabled={saving}
                icon={<Save size={16} />}
                size="small"
                onClick={() => onSaveSetting(MOBILE_INTRO_SETTING_KEY)}
              >
              保存介绍
              </Button>
            </div>
          </div>

          <div className="rounded border border-slate-200 bg-white p-3">
            <div className="mb-3 flex items-center justify-between gap-2">
              <div>
                <div className="flex items-center gap-2 text-sm font-medium text-ink">
                  <Settings size={16} />
                  首页高频极速
                </div>
                <p className="mt-1 text-xs text-slate-500">
                  默认关闭；开启后只展示下方选中的销售中彩种，首页不显示合买标签和合买入口。
                </p>
              </div>
              <Tag color={featuredEnabledValue === 'true' ? 'green' : 'grey'}>
                {featuredEnabledValue === 'true' ? '已开启' : '已关闭'}
              </Tag>
            </div>
            <div className="grid gap-3 md:grid-cols-[180px_minmax(0,1fr)]">
              <div>
                <span className="mb-1 block text-xs font-medium text-slate-500">
                  模块开关
                </span>
                <Select
                  className="form-input"
                  value={featuredEnabledValue || 'false'}
                  onChange={(value) =>
                    onDraftChange(
                      MOBILE_HOME_FEATURED_ENABLED_SETTING_KEY,
                      String(value ?? 'false'),
                    )
                  }
                >
                  <Select.Option value="false">关闭高频极速</Select.Option>
                  <Select.Option value="true">开启高频极速</Select.Option>
                </Select>
              </div>
              <div>
                <span className="mb-1 block text-xs font-medium text-slate-500">
                  模块标题
                </span>
                <Input
                  className="form-input"
                  placeholder="例如：高频极速"
                  value={featuredTitleValue}
                  onChange={(value) =>
                    onDraftChange(MOBILE_HOME_FEATURED_TITLE_SETTING_KEY, value)
                  }
                />
              </div>
              <div className="md:col-span-2">
                <span className="mb-1 block text-xs font-medium text-slate-500">
                  展示彩种
                </span>
                <Select
                  className="form-input"
                  filter
                  multiple
                  placeholder="选择需要在首页高频极速模块展示的彩种"
                  value={selectedFeaturedLotteryCodes}
                  onChange={(value) =>
                    onDraftChange(
                      MOBILE_HOME_FEATURED_LOTTERY_CODES_SETTING_KEY,
                      settingListText(value),
                    )
                  }
                >
                  {lotteries.map((lottery) => (
                    <Select.Option key={lottery.id} value={lottery.id}>
                      {lottery.name}（{lottery.id}）
                      {lottery.saleEnabled ? '' : ' - 停售不显示'}
                    </Select.Option>
                  ))}
                </Select>
              </div>
            </div>
            <div className="mt-3 flex flex-wrap justify-end gap-2">
              <Button
                disabled={saving}
                icon={<Save size={16} />}
                size="small"
                onClick={() => onSaveSetting(MOBILE_HOME_FEATURED_ENABLED_SETTING_KEY)}
              >
                保存开关
              </Button>
              <Button
                disabled={saving}
                icon={<Save size={16} />}
                size="small"
                onClick={() => onSaveSetting(MOBILE_HOME_FEATURED_TITLE_SETTING_KEY)}
              >
                保存标题
              </Button>
              <Button
                disabled={saving}
                icon={<Save size={16} />}
                size="small"
                onClick={() => onSaveSetting(MOBILE_HOME_FEATURED_LOTTERY_CODES_SETTING_KEY)}
              >
                保存彩种
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function RegistrationSettingsPanel({
  onRegistrationChange,
  onSaveRegistration,
  registration,
  saving,
}: {
  onRegistrationChange: Dispatch<SetStateAction<RegistrationConfig | null>>;
  onSaveRegistration: () => void;
  registration: RegistrationConfig | null;
  saving: boolean;
}) {
  return (
    <div className="rounded border border-slate-200 bg-slate-50 p-3">
      <PanelTitle icon={<Settings size={18} />} title="注册配置" />
      {registration ? (
        <div className="grid gap-3 lg:grid-cols-3">
          <ToggleRow
            checked={registration.usernameEnabled}
            label="用户名注册"
            onChange={(checked) =>
              onRegistrationChange((current) =>
                current ? { ...current, usernameEnabled: checked } : current,
              )
            }
          />
          <ToggleRow
            checked={registration.emailEnabled}
            label="邮箱注册"
            onChange={(checked) =>
              onRegistrationChange((current) =>
                current ? { ...current, emailEnabled: checked } : current,
              )
            }
          />
          <ToggleRow
            checked={registration.agentInviteRequired}
            label="代理邀请必填"
            onChange={(checked) =>
              onRegistrationChange((current) =>
                current ? { ...current, agentInviteRequired: checked } : current,
              )
            }
          />
          <Button
            disabled={
              saving || (!registration.usernameEnabled && !registration.emailEnabled)
            }
            icon={<Save size={16} />}
            theme="solid"
            onClick={onSaveRegistration}
          >
            保存注册配置
          </Button>
        </div>
      ) : (
        <div className="rounded-md border border-line p-3 text-sm text-slate-500">
          暂无注册配置。
        </div>
      )}
    </div>
  );
}

function ImageBedTestPanel({
  imageBedMissingConfigs,
  imageBedResultUrlField,
  imageBedUploadField,
  imageBedUploadUrl,
  saving,
}: {
  imageBedMissingConfigs: string[];
  imageBedResultUrlField: string;
  imageBedUploadField: string;
  imageBedUploadUrl: string;
  saving: boolean;
}) {
  return (
    <div className="rounded border border-slate-200 bg-slate-50 p-3">
      <PanelTitle icon={<UploadIcon size={18} />} title="图床上传测试" />
      <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_260px]">
        <div className="grid content-start gap-2 text-sm text-slate-600">
          <div className="flex items-center justify-between gap-3 rounded border border-slate-200 bg-white px-3 py-2">
            <span>上传地址</span>
            <span className="min-w-0 truncate font-mono text-xs text-slate-700">
              {imageBedUploadUrl || '未配置'}
            </span>
          </div>
          <div className="grid gap-2 sm:grid-cols-2">
            <div className="rounded border border-slate-200 bg-white px-3 py-2">
              <p className="text-xs text-slate-500">上传字段名</p>
              <p className="mt-1 truncate font-mono text-sm text-slate-700">
                {imageBedUploadField || 'file'}
              </p>
            </div>
            <div className="rounded border border-slate-200 bg-white px-3 py-2">
              <p className="text-xs text-slate-500">返回链接字段</p>
              <p className="mt-1 truncate font-mono text-sm text-slate-700">
                {imageBedResultUrlField}
              </p>
            </div>
          </div>
        </div>

        <ImageUploadAvatar
          description="上传成功后会自动展示图片链接和预览。"
          disabled={saving}
          errorTitle="图床上传失败"
          failureMessage="上传失败"
          missingConfigLabels={imageBedMissingConfigs}
          successMessage="图片上传成功"
          title="点击图片区域选择并测试上传"
          uploadFieldName={imageBedUploadField || 'file'}
          uploadingText="正在请求图床服务..."
          warningTitle="图床配置不完整"
        />
      </div>
    </div>
  );
}

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <label className="block text-sm font-medium text-slate-600">
      <span className="mb-1 block">{label}</span>
      {children}
    </label>
  );
}

function settingGroupName(key: string): string {
  if (key.startsWith('mobile_')) {
    return '手机端设置';
  }
  if (key.startsWith('image_bed_')) {
    return '图床设置';
  }
  if (
    key.startsWith('recharge_rainbow_epay_') ||
    key.startsWith('recharge_customer_service_') ||
    key === RECHARGE_MIN_AMOUNT_SETTING_KEY ||
    key === RECHARGE_MAX_AMOUNT_SETTING_KEY
  ) {
    return '充值设置';
  }
  if (key.startsWith('support_telegram_')) {
    return '通知设置';
  }
  if (key.includes('email') || key.includes('registration')) {
    return '注册与安全';
  }
  if (key.includes('recharge') || key.includes('rebate')) {
    return '返利设置';
  }
  return '基础设置';
}

function isMinorMoneySetting(key: string) {
  return MINOR_MONEY_SETTING_KEYS.has(key);
}

function settingDraftValue(setting: SystemSettingItem) {
  return isMinorMoneySetting(setting.key)
    ? minorToYuanInput(setting.value)
    : setting.value;
}

function settingSubmitValue(key: string, value: string) {
  if (!isMinorMoneySetting(key)) {
    return value;
  }
  const amountMinor = yuanInputToMinor(value);
  if (amountMinor === null || amountMinor <= 0) {
    Toast.warning('充值金额设置必须大于 0 元且最多保留两位小数');
    return null;
  }
  return String(amountMinor);
}

function settingDescription(setting: SystemSettingItem) {
  if (!isMinorMoneySetting(setting.key)) {
    return setting.description;
  }
  return setting.description.replace('（分）', '（元）');
}

function settingSelectOptions(
  key: string,
  currentValue: string,
): SettingSelectOption[] {
  const optionsByKey: Record<string, SettingSelectOption[]> = {
    email_registration_enabled: [
      { label: '开启邮箱注册', value: 'true' },
      { label: '关闭邮箱注册', value: 'false' },
    ],
    recharge_rebate_mode: [
      { label: '立即返利', value: 'immediate' },
      { label: '充值阶梯返利', value: 'rechargeTiered' },
    ],
    recharge_rainbow_epay_enabled: [
      { label: '开启彩虹易支付', value: 'true' },
      { label: '关闭彩虹易支付', value: 'false' },
    ],
    recharge_customer_service_enabled: [
      { label: '开启客服直充', value: 'true' },
      { label: '关闭客服直充', value: 'false' },
    ],
    [SUPPORT_TELEGRAM_ENABLED_SETTING_KEY]: [
      { label: '开启 Telegram 提醒', value: 'true' },
      { label: '关闭 Telegram 提醒', value: 'false' },
    ],
  };
  const options = optionsByKey[key] ?? [];
  const normalizedCurrentValue = currentValue.trim();
  if (
    normalizedCurrentValue &&
    options.length > 0 &&
    !options.some((option) => option.value === normalizedCurrentValue)
  ) {
    return [
      ...options,
      {
        label: `当前值：${normalizedCurrentValue}`,
        value: normalizedCurrentValue,
      },
    ];
  }
  return options;
}

function settingsGroups(
  settings: SystemSettingItem[],
): Array<[string, SystemSettingItem[]]> {
  const groups = new Map<string, SystemSettingItem[]>();
  for (const setting of settings) {
    const name = settingGroupName(setting.key);
    const list = groups.get(name);
    if (list) {
      list.push(setting);
    } else {
      groups.set(name, [setting]);
    }
  }

  const priority = [
    '手机端设置',
    '图床设置',
    '充值设置',
    '通知设置',
    '注册与安全',
    '返利设置',
    '基础设置',
  ];
  return priority
    .filter((name) => (groups.get(name)?.length ?? 0) > 0)
    .map((name) => [name, groups.get(name) ?? []]);
}

function draftSettingValue(
  settings: Array<{ key: string; value: string }>,
  drafts: Record<string, string>,
  key: string,
) {
  return drafts[key] ?? readSettingValue(settings, key);
}

function settingListValue(value: string): string[] {
  return value
    .split(/[,\s，]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function settingListText(value: unknown): string {
  if (!Array.isArray(value)) {
    return String(value ?? '').trim();
  }
  return value
    .map((item) => String(item ?? '').trim())
    .filter(Boolean)
    .join(',');
}

function ToggleRow({
  checked,
  label,
  onChange,
}: {
  checked: boolean;
  label: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-3 rounded border border-slate-200 bg-white px-3 py-2 text-sm text-slate-600">
      <span>{label}</span>
      <input
        checked={checked}
        type="checkbox"
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}

function sectionForModule(moduleKey: string): AccessSection {
  if (moduleKey === 'admins') {
    return 'admins';
  }
  if (moduleKey === 'roles') {
    return 'roles';
  }
  if (moduleKey === 'settings' || moduleKey === 'registration') {
    return 'settings';
  }
  return 'users';
}

function readSettingValue(
  settings: Array<{ key: string; value: string }>,
  key: string,
) {
  return settings.find((item) => item.key === key)?.value ?? '';
}

function emptyUserForm(): UserFormState {
  return {
    agentId: '',
    balanceMinor: '0',
    email: '',
    id: 'U20001',
    inviteCode: '',
    kind: 'regular',
    status: 'active',
    username: 'new_user',
  };
}

function userFormFromSummary(user: UserSummary): UserFormState {
  return {
    agentId: user.agentId ?? '',
    balanceMinor: `${user.balanceMinor}`,
    email: user.email ?? '',
    id: user.id,
    inviteCode: user.inviteCode,
    kind: user.kind,
    status: user.status,
    username: user.username,
  };
}

function userPayload(form: UserFormState): UserSummary {
  return {
    agentId: optionalText(form.agentId),
    balanceMinor: numberField(form.balanceMinor),
    email: optionalText(form.email),
    id: form.id.trim(),
    inviteCode: form.inviteCode.trim(),
    kind: form.kind,
    status: form.status,
    username: form.username.trim(),
  };
}

function emptyAdminForm(roleId: string): AdminFormState {
  return {
    id: 'A20001',
    password: '',
    roleId,
    status: 'active',
    username: 'new_admin',
  };
}

function adminFormFromSummary(admin: AdminSummary): AdminFormState {
  return {
    id: admin.id,
    password: '',
    roleId: admin.roleId,
    status: admin.status,
    username: admin.username,
  };
}

function adminPayload(
  form: AdminFormState,
  roles: AdminRole[],
  includePassword: boolean,
): AdminSaveRequest {
  const role = roles.find((item) => item.id === form.roleId);
  const payload: AdminSaveRequest = {
    id: form.id.trim(),
    roleId: form.roleId,
    roleName: role?.name ?? form.roleId,
    status: form.status,
    username: form.username.trim(),
  };
  const password = form.password.trim();
  if (includePassword && password) {
    payload.password = password;
  }
  return payload;
}

function emptyRoleForm(): RoleFormState {
  return {
    id: 'role-new',
    name: '新角色',
    scopes: ['users'],
  };
}

function roleFormFromSummary(role: AdminRole): RoleFormState {
  return {
    id: role.id,
    name: role.name,
    scopes: role.scopes,
  };
}

function rolePayload(form: RoleFormState): AdminRole {
  return {
    id: form.id.trim(),
    name: form.name.trim(),
    scopes: form.scopes,
  };
}

function toggleScope(
  setForm: Dispatch<SetStateAction<RoleFormState>>,
  scope: PermissionScope,
  checked: boolean,
) {
  setForm((current) => ({
    ...current,
    scopes: checked
      ? Array.from(new Set([...current.scopes, scope]))
      : current.scopes.filter((item) => item !== scope),
  }));
}

function setFormValue<T, K extends keyof T>(
  setForm: Dispatch<SetStateAction<T>>,
  key: K,
  value: T[K],
) {
  setForm((current) => ({ ...current, [key]: value }));
}

function optionalText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function numberField(value: string) {
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

function userKindText(kind: UserKind) {
  return kind === 'agent' ? '代理' : '普通用户';
}

function userStatusText(status: UserStatus) {
  const labels: Record<UserStatus, string> = {
    active: '启用',
    locked: '锁定',
    suspended: '停用',
  };
  return labels[status];
}

function userStatusColor(status: UserStatus) {
  if (status === 'active') {
    return 'green';
  }
  if (status === 'locked') {
    return 'red';
  }
  return 'grey';
}

function permissionScopeText(scope: PermissionScope) {
  return PERMISSION_SCOPE_OPTIONS.find((item) => item.value === scope)?.label ?? scope;
}
