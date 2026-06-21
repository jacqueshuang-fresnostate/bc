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
  Search,
  Settings,
  Smartphone,
  Upload as UploadIcon,
  ShieldCheck,
  Trash2,
  UserPlus,
  Users,
  X,
} from 'lucide-react';
import {
  useEffect,
  useMemo,
  useState,
  type ChangeEvent,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react';
import { fetchLotteries, uploadAppPackageFile } from '../api/client';
import {
  ImageUploadAvatar,
  extractImageUrlFromUploadResult,
} from '../components/ImageUploadAvatar';
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
  PermissionKey,
  LotteryKind,
  PermissionScope,
  RegistrationConfig,
  UserKind,
  UserRegistrationLocation,
  UserStatus,
  UserSummary,
} from '../types/dashboard';
import type { ClearRecordsResult } from '../types/finance';
import { formatMoney } from '../utils/format';
import { minorToYuanInput, yuanInputToMinor } from '../utils/moneyInput';

type AccessSection = 'admins' | 'roles' | 'settings' | 'users';

interface AccessManagementPageProps {
  activeModuleKey: string;
  onDashboardRefresh: () => void;
  onOpenUserLedger: (user: AdminUserSummary) => void;
  onOpenUserOrders: (user: AdminUserSummary) => void;
  onOpenRebateSettings?: () => void;
}

interface UserFormState {
  agentId: string;
  balanceMinor: string;
  contactQq: string;
  email: string;
  id: string;
  inviteCode: string;
  kind: UserKind;
  password: string;
  registrationLocation: UserRegistrationLocation;
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

interface RechargeBonusRuleDraft {
  bonusAmountYuan: string;
  thresholdAmountYuan: string;
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
  permissions: PermissionKey[];
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
const MOBILE_APP_UPDATE_PLATFORMS = [
  {
    accept: '.apk,application/vnd.android.package-archive',
    buildKey: 'mobile_app_android_latest_build',
    downloadKey: 'mobile_app_android_package_url',
    enabledKey: 'mobile_app_android_enabled',
    forceKey: 'mobile_app_android_force_update',
    label: 'Android',
    notesKey: 'mobile_app_android_release_notes',
    packageLabel: 'APK',
    versionKey: 'mobile_app_android_latest_version',
  },
  {
    accept: '.ipa,application/octet-stream,application/x-itunes-ipa',
    buildKey: 'mobile_app_ios_latest_build',
    downloadKey: 'mobile_app_ios_package_url',
    enabledKey: 'mobile_app_ios_enabled',
    forceKey: 'mobile_app_ios_force_update',
    label: 'iOS',
    notesKey: 'mobile_app_ios_release_notes',
    packageLabel: 'IPA',
    versionKey: 'mobile_app_ios_latest_version',
  },
] as const;
const MOBILE_APP_UPDATE_SETTING_KEYS = MOBILE_APP_UPDATE_PLATFORMS.flatMap(
  (platform) => [
    platform.enabledKey,
    platform.versionKey,
    platform.buildKey,
    platform.downloadKey,
    platform.forceKey,
    platform.notesKey,
  ],
);
const MOBILE_APP_UPDATE_SETTING_KEY_SET = new Set<string>(
  MOBILE_APP_UPDATE_SETTING_KEYS,
);
const RECHARGE_RAINBOW_ENABLED_SETTING_KEY = 'recharge_rainbow_epay_enabled';
const RECHARGE_RAINBOW_PAY_TYPES_SETTING_KEY = 'recharge_rainbow_epay_pay_types';
const RECHARGE_CUSTOMER_SERVICE_ENABLED_SETTING_KEY =
  'recharge_customer_service_enabled';
const RECHARGE_MIN_AMOUNT_SETTING_KEY = 'recharge_min_amount_minor';
const RECHARGE_MAX_AMOUNT_SETTING_KEY = 'recharge_max_amount_minor';
const RECHARGE_BONUS_ENABLED_SETTING_KEY = 'recharge_bonus_enabled';
const RECHARGE_BONUS_RULES_SETTING_KEY = 'recharge_bonus_rules';
const CHAT_HALL_SPEAKING_MIN_RECHARGE_SETTING_KEY =
  'chat_hall_speaking_min_recharge_minor';
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
  ...MOBILE_APP_UPDATE_SETTING_KEYS,
]);
const RECHARGE_PAYMENT_SETTING_KEYS = new Set([
  RECHARGE_RAINBOW_ENABLED_SETTING_KEY,
  RECHARGE_RAINBOW_PAY_TYPES_SETTING_KEY,
  RECHARGE_CUSTOMER_SERVICE_ENABLED_SETTING_KEY,
  RECHARGE_BONUS_ENABLED_SETTING_KEY,
  RECHARGE_BONUS_RULES_SETTING_KEY,
]);
const REBATE_LEGACY_SETTING_KEYS = new Set(['recharge_rebate_mode']);
const MINOR_MONEY_SETTING_KEYS = new Set([
  RECHARGE_MIN_AMOUNT_SETTING_KEY,
  RECHARGE_MAX_AMOUNT_SETTING_KEY,
  CHAT_HALL_SPEAKING_MIN_RECHARGE_SETTING_KEY,
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

interface PermissionDefinition {
  group: string;
  key: PermissionKey;
  label: string;
  scope: PermissionScope;
  sensitive: boolean;
}

const PERMISSION_DEFINITIONS: PermissionDefinition[] = [
  { group: '用户管理', key: 'user.read', label: '查看用户', scope: 'users', sensitive: false },
  { group: '用户管理', key: 'user.write', label: '新增或编辑用户', scope: 'users', sensitive: false },
  { group: '用户管理', key: 'user.status', label: '启停或锁定用户', scope: 'users', sensitive: true },
  { group: '用户管理', key: 'user.password.reset', label: '重置用户密码', scope: 'users', sensitive: true },
  { group: '用户管理', key: 'user.delete', label: '删除用户', scope: 'users', sensitive: true },
  { group: '管理员管理', key: 'admin.read', label: '查看管理员', scope: 'admins', sensitive: false },
  { group: '管理员管理', key: 'admin.write', label: '新增或编辑管理员', scope: 'admins', sensitive: true },
  { group: '管理员管理', key: 'admin.status', label: '启停或锁定管理员', scope: 'admins', sensitive: true },
  { group: '管理员管理', key: 'admin.password.reset', label: '重置管理员密码', scope: 'admins', sensitive: true },
  { group: '角色权限', key: 'role.read', label: '查看角色', scope: 'roles', sensitive: false },
  { group: '角色权限', key: 'role.write', label: '新增或编辑角色', scope: 'roles', sensitive: true },
  { group: '角色权限', key: 'role.delete', label: '删除角色', scope: 'roles', sensitive: true },
  { group: '财务管理', key: 'finance.read', label: '查看财务', scope: 'finance', sensitive: false },
  { group: '财务管理', key: 'finance.adjust.create', label: '手动调账', scope: 'finance', sensitive: true },
  { group: '财务管理', key: 'finance.ledger.clear', label: '清除资金流水', scope: 'finance', sensitive: true },
  { group: '充值订单', key: 'recharge.confirm', label: '确认充值', scope: 'finance', sensitive: true },
  { group: '充值订单', key: 'recharge.export', label: '导出充值记录', scope: 'finance', sensitive: true },
  { group: '充值订单', key: 'recharge.clear', label: '清除充值记录', scope: 'finance', sensitive: true },
  { group: '提现管理', key: 'withdrawal.review', label: '审核提现', scope: 'finance', sensitive: true },
  { group: '提现管理', key: 'withdrawal.clear', label: '清除提现记录', scope: 'finance', sensitive: true },
  { group: '订单管理', key: 'order.read', label: '查看订单', scope: 'orders', sensitive: false },
  { group: '订单管理', key: 'order.write', label: '创建或处理订单', scope: 'orders', sensitive: true },
  { group: '订单管理', key: 'order.clear', label: '清除投注记录', scope: 'orders', sensitive: true },
  { group: '计奖派奖', key: 'settlement.run', label: '计奖派奖', scope: 'orders', sensitive: true },
  { group: '在线客服', key: 'support.read', label: '查看客服会话', scope: 'customerService', sensitive: false },
  { group: '在线客服', key: 'support.reply', label: '回复客服消息', scope: 'customerService', sensitive: false },
  { group: '在线客服', key: 'support.manage', label: '管理客服会话', scope: 'customerService', sensitive: true },
  { group: '彩种管理', key: 'lottery.read', label: '查看彩种', scope: 'lotteries', sensitive: false },
  { group: '彩种管理', key: 'lottery.write', label: '新增或编辑彩种', scope: 'lotteries', sensitive: true },
  { group: '彩种管理', key: 'lottery.sale.toggle', label: '切换销售状态', scope: 'lotteries', sensitive: true },
  { group: '彩种控制台', key: 'lottery.draw.control', label: '控制开奖号码', scope: 'lotteries', sensitive: true },
  { group: '期号管理', key: 'lottery.issue.write', label: '维护期号', scope: 'lotteries', sensitive: true },
  { group: '开奖源', key: 'lottery.source.manage', label: '维护开奖源', scope: 'lotteries', sensitive: true },
  { group: '开奖源', key: 'lottery.source.sync', label: '同步开奖源', scope: 'lotteries', sensitive: true },
  { group: '玩法规则', key: 'play.rule.manage', label: '玩法配置', scope: 'lotteries', sensitive: true },
  { group: '合买管理', key: 'group.buy.read', label: '查看合买', scope: 'lotteries', sensitive: false },
  { group: '合买管理', key: 'group.buy.manage', label: '维护合买', scope: 'lotteries', sensitive: true },
  { group: '合买管理', key: 'group.buy.clear', label: '清除合买记录', scope: 'lotteries', sensitive: true },
  { group: '机器人配置', key: 'robot.read', label: '查看机器人', scope: 'robots', sensitive: false },
  { group: '机器人配置', key: 'robot.write', label: '维护机器人', scope: 'robots', sensitive: true },
  { group: '机器人配置', key: 'robot.run', label: '执行机器人', scope: 'robots', sensitive: true },
  { group: '机器人配置', key: 'robot.delete', label: '删除机器人', scope: 'robots', sensitive: true },
  { group: '邀请返利', key: 'rebate.read', label: '查看邀请返利', scope: 'rebates', sensitive: false },
  { group: '邀请返利', key: 'rebate.withdraw', label: '处理返利提现', scope: 'rebates', sensitive: true },
  { group: '代理管理', key: 'agent.review', label: '审核代理申请', scope: 'rebates', sensitive: true },
  { group: '邀请管理', key: 'invite.manage', label: '维护邀请配置', scope: 'rebates', sensitive: true },
  { group: '系统设置', key: 'system.read', label: '查看系统设置', scope: 'systemSettings', sensitive: false },
  { group: '系统设置', key: 'system.write', label: '修改系统设置', scope: 'systemSettings', sensitive: true },
  { group: '系统设置', key: 'system.cache.reload', label: '刷新系统缓存', scope: 'systemSettings', sensitive: true },
  { group: '系统设置', key: 'system.chat.clear', label: '清空聊天大厅', scope: 'systemSettings', sensitive: true },
  { group: '系统设置', key: 'system.upload', label: '上传系统文件', scope: 'systemSettings', sensitive: true },
  { group: '广告管理', key: 'advertisement.manage', label: '维护广告', scope: 'systemSettings', sensitive: true },
];

const PERMISSION_GROUPS = Array.from(
  PERMISSION_DEFINITIONS.reduce((groups, definition) => {
    const items = groups.get(definition.group) ?? [];
    items.push(definition);
    groups.set(definition.group, items);
    return groups;
  }, new Map<string, PermissionDefinition[]>()),
).map(([group, items]) => ({ group, items }));

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
type UserStatusFilter = 'all' | UserStatus;

const USER_STATUS_FILTER_OPTIONS: Array<{ label: string; value: UserStatusFilter }> = [
  { label: '全部状态', value: 'all' },
  { label: '启用', value: 'active' },
  { label: '停用', value: 'suspended' },
  { label: '锁定', value: 'locked' },
];

export function AccessManagementPage({
  activeModuleKey,
  onDashboardRefresh,
  onOpenRebateSettings,
  onOpenUserLedger,
  onOpenUserOrders,
}: AccessManagementPageProps) {
  const [userPageNumber, setUserPageNumber] = useState(1);
  const [userPageSize, setUserPageSize] = useState(20);
  const [userSortBy, setUserSortBy] = useState<UserListSortBy>('id');
  const [userSortDirection, setUserSortDirection] =
    useState<UserListSortDirection>('desc');
  const [userStatusFilter, setUserStatusFilter] =
    useState<UserStatusFilter>('all');
  const [userUsernameDraft, setUserUsernameDraft] = useState('');
  const [userUsernameSearch, setUserUsernameSearch] = useState('');
  const userQuery = useMemo(
    () => ({
      page: userPageNumber,
      pageSize: userPageSize,
      sortBy: userSortBy,
      sortDirection: userSortDirection,
      status: userStatusFilter === 'all' ? undefined : userStatusFilter,
      username: userUsernameSearch || undefined,
    }),
    [
      userPageNumber,
      userPageSize,
      userSortBy,
      userSortDirection,
      userStatusFilter,
      userUsernameSearch,
    ],
  );
  const {
    admins,
    changeAdminStatus,
    changeUserStatus,
    clearChatHallHistory,
    error,
    loading,
    refresh,
    registration,
    removeRole,
    removeUser,
    reloadMemoryCache,
    resetPassword,
    resetUserLoginPassword,
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
  const applyUserUsernameSearch = () => {
    setUserUsernameSearch(userUsernameDraft.trim());
    setUserPageNumber(1);
  };
  const clearUserUsernameSearch = () => {
    setUserUsernameDraft('');
    setUserUsernameSearch('');
    setUserPageNumber(1);
  };
  const isSettingsPage = section === 'settings';

  const submitUser = async () => {
    let saved = await saveUser(userPayload(userForm), editingUserId ?? undefined);
    const password = userForm.password.trim();
    if (password) {
      saved = await resetUserLoginPassword(saved.id, { password });
      Toast.success('用户密码已重置');
    }
    setUserForm({ ...userFormFromSummary(saved), password: '' });
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

  const deleteUserFromList = async (user: AdminUserSummary) => {
    const confirmed = window.confirm(
      `确定删除用户「${user.username}」（${user.id}）吗？删除后该账号无法登录，历史订单、资金流水会继续保留用户 ID 作为审计线索。`,
    );
    if (!confirmed) {
      return;
    }

    try {
      await removeUser(user.id);
      if (editingUserId === user.id) {
        setEditingUserId(null);
        setUserForm(emptyUserForm());
        setUserSheetVisible(false);
      }
      Toast.success('用户已删除');
      onDashboardRefresh();
    } catch (error) {
      Toast.error(error instanceof Error ? error.message : '用户删除失败');
    }
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
          statusFilter={userStatusFilter}
          totalCount={userPage.totalCount}
          totalPages={userPage.totalPages}
          usernameDraft={userUsernameDraft}
          usernameSearch={userUsernameSearch}
          users={users}
          onClose={() => setUserSheetVisible(false)}
          onEdit={(user) => {
            setEditingUserId(user.id);
            setUserForm(userFormFromSummary(user));
            setUserSheetVisible(true);
          }}
          onDelete={(user) => void deleteUserFromList(user)}
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
          onStatusFilterChange={(status) => {
            setUserStatusFilter(status);
            setUserPageNumber(1);
          }}
          onUsernameDraftChange={setUserUsernameDraft}
          onUsernameSearchApply={applyUserUsernameSearch}
          onUsernameSearchClear={clearUserUsernameSearch}
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
          onOpenRebateSettings={onOpenRebateSettings}
          onDraftChange={(key, value) =>
            setSettingDrafts((current) => ({ ...current, [key]: value }))
          }
          onRegistrationChange={setRegistrationForm}
          onClearChatHallMessages={async () => {
            const result = await clearChatHallHistory();
            onDashboardRefresh();
            return result;
          }}
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
          onSaveSettingValue={(key, value) => {
            void saveSetting(key, value).then(onDashboardRefresh);
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
  onDelete,
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
  onStatusFilterChange,
  onSubmit,
  onUsernameDraftChange,
  onUsernameSearchApply,
  onUsernameSearchClear,
  page,
  pageSize,
  saving,
  sheetVisible,
  sortBy,
  sortDirection,
  statusFilter,
  totalCount,
  totalPages,
  usernameDraft,
  usernameSearch,
  users,
}: {
  editingId: string | null;
  form: UserFormState;
  loading: boolean;
  onClose: () => void;
  onDelete: (user: AdminUserSummary) => void;
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
  onStatusFilterChange: (status: UserStatusFilter) => void;
  onSubmit: () => void;
  onUsernameDraftChange: (value: string) => void;
  onUsernameSearchApply: () => void;
  onUsernameSearchClear: () => void;
  page: number;
  pageSize: number;
  saving: boolean;
  sheetVisible: boolean;
  sortBy: UserListSortBy;
  sortDirection: UserListSortDirection;
  statusFilter: UserStatusFilter;
  totalCount: number;
  totalPages: number;
  usernameDraft: string;
  usernameSearch: string;
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
              <span className="text-xs font-medium text-slate-500">状态</span>
              <Select
                className="form-input min-w-[112px]"
                value={statusFilter}
                onChange={(value) =>
                  onStatusFilterChange((value as UserStatusFilter) || 'all')
                }
              >
                {USER_STATUS_FILTER_OPTIONS.map((option) => (
                  <Select.Option key={option.value} value={option.value}>
                    {option.label}
                  </Select.Option>
                ))}
              </Select>
              <span className="text-xs font-medium text-slate-500">用户名</span>
              <Input
                className="form-input min-w-[180px]"
                placeholder="输入用户名搜索"
                prefix={<Search size={14} />}
                showClear
                value={usernameDraft}
                onChange={onUsernameDraftChange}
                onClear={onUsernameSearchClear}
                onEnterPress={onUsernameSearchApply}
              />
              <Button
                icon={<Search size={14} />}
                size="small"
                theme="solid"
                onClick={onUsernameSearchApply}
              >
                搜索
              </Button>
              <Button
                disabled={!usernameDraft && !usernameSearch}
                icon={<X size={14} />}
                size="small"
                onClick={onUsernameSearchClear}
              >
                清空
              </Button>
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
          <table className="w-full min-w-[1160px] text-left text-sm">
            <thead className="border-b border-line text-xs text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">用户</th>
                <th className="py-2 pr-4 font-medium">注册地</th>
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
                      {user.contactQq ? ` · QQ ${user.contactQq}` : ''}
                    </div>
                  </td>
                  <td className="py-3 pr-4 text-slate-600">
                    <div className="max-w-[180px] truncate font-medium text-ink">
                      {registrationLocationLabel(user.registrationLocation)}
                    </div>
                    <div className="mt-1 max-w-[180px] truncate text-xs text-slate-400">
                      {registrationLocationMeta(user.registrationLocation)}
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
                        disabled={user.status !== 'active'}
                        size="small"
                        onClick={() => onStatus(user.id, 'suspended')}
                      >
                        {inactiveUserStatusActionLabel(user.status)}
                      </Button>
                      {user.status !== 'active' ? (
                        <Button
                          size="small"
                          onClick={() => onStatus(user.id, 'active')}
                        >
                          {user.status === 'locked' ? '解除锁定' : '启用'}
                        </Button>
                      ) : null}
                      <Button
                        disabled={saving}
                        icon={<Trash2 size={14} />}
                        size="small"
                        type="danger"
                        onClick={() => onDelete(user)}
                      >
                        删除
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
            <Input className="form-input" disabled value={form.id} />
            <p className="text-xs text-slate-400">
              用户 ID 由系统生成，创建后不可编辑。
            </p>
          </Field>
          <Field
            description={editingId ? '用户名创建后不可编辑。' : undefined}
            label="用户名"
          >
            <Input
              className="form-input"
              disabled={Boolean(editingId)}
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
          <Field label="联系方式 QQ">
            <Input
              className="form-input"
              placeholder="选填 QQ 号码"
              value={form.contactQq}
              onChange={(value) =>
                setFormValue(onSetForm, 'contactQq', value)
              }
            />
          </Field>
          <Field label="注册来源">
            <div className="rounded-md border border-slate-100 bg-slate-50 px-3 py-2 text-sm text-slate-600">
              <div className="font-medium text-ink">
                {registrationLocationLabel(form.registrationLocation)}
              </div>
              <div className="mt-1 text-xs text-slate-400">
                {registrationLocationMeta(form.registrationLocation)}
              </div>
            </div>
          </Field>
          <Field label={editingId ? '重置密码' : '初始密码'}>
            <Input
              autoComplete="new-password"
              className="form-input"
              placeholder={editingId ? '留空则不修改密码' : '可选，至少 8 位'}
              type="password"
              value={form.password}
              onChange={(value) =>
                setFormValue(onSetForm, 'password', value)
              }
            />
            <p className="text-xs text-slate-400">
              填写后保存会立即重置该用户的登录密码。
            </p>
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
            <Field
              description="停用用于运营主动禁用账号；锁定用于安全异常冻结账号。两种状态都会禁止登录，列表快捷操作只保留启用/停用。"
              label="状态"
            >
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
            <p className="text-xs text-slate-400">
              余额只能通过财务管理的手动调账入口调整。
            </p>
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
              disabled={saving || passwordNeedsMoreChars(form.password)}
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
          <table className="w-full min-w-[920px] text-left text-sm">
            <thead className="border-b border-line text-xs text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">角色</th>
                <th className="py-2 pr-4 font-medium">权限范围</th>
                <th className="py-2 pr-4 font-medium">操作权限</th>
                <th className="py-2 pr-4 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {roles.map((role) => {
                const permissions = effectiveRolePermissions(role);
                const sensitiveCount = permissions.filter((permission) =>
                  permissionDefinition(permission)?.sensitive,
                ).length;
                return (
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
                      <div className="flex flex-wrap items-center gap-2 text-xs text-slate-500">
                        <Tag color="blue">{permissions.length} 个权限点</Tag>
                        {sensitiveCount > 0 ? (
                          <Tag color="red">{sensitiveCount} 个高风险</Tag>
                        ) : (
                          <Tag color="green">无高风险</Tag>
                        )}
                      </div>
                    </td>
                    <td className="py-3 pr-4">
                      <Button size="small" onClick={() => onEdit(role)}>
                        编辑
                      </Button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </Card>

      <SideSheet
        aria-label="角色维护"
        title="角色维护"
        visible={sheetVisible}
        width={760}
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
            <div className="text-sm font-medium text-slate-600">模块权限</div>
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
          <div className="space-y-3">
            <div className="flex items-center justify-between gap-2">
              <div>
                <div className="text-sm font-medium text-slate-600">操作权限点</div>
                <p className="mt-1 text-xs text-slate-400">
                  旧角色未配置权限点时会兼容模块权限；保存后将按这里的权限点精确控制。
                </p>
              </div>
              <Tag color={form.permissions.length > 0 ? 'blue' : 'red'}>
                已选 {form.permissions.length}
              </Tag>
            </div>
            <div className="max-h-[48vh] space-y-3 overflow-y-auto rounded border border-slate-100 bg-slate-50/70 p-3">
              {PERMISSION_GROUPS.map((group) => (
                <div key={group.group} className="rounded bg-white p-3 shadow-sm">
                  <div className="mb-2 flex items-center justify-between gap-2">
                    <span className="text-sm font-semibold text-ink">{group.group}</span>
                    <Button
                      size="small"
                      theme="borderless"
                      type="tertiary"
                      onClick={() =>
                        togglePermissionGroup(onSetForm, group.items, true)
                      }
                    >
                      全选
                    </Button>
                  </div>
                  <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
                    {group.items.map((permission) => (
                      <label
                        key={permission.key}
                        className="flex min-h-10 items-center justify-between gap-2 rounded border border-slate-100 bg-white px-2 py-2 text-sm text-slate-600"
                      >
                        <span className="flex items-center gap-2">
                          <input
                            checked={form.permissions.includes(permission.key)}
                            type="checkbox"
                            onChange={(event) =>
                              togglePermission(
                                onSetForm,
                                permission,
                                event.target.checked,
                              )
                            }
                          />
                          {permission.label}
                        </span>
                        {permission.sensitive ? (
                          <Tag color="red">高风险</Tag>
                        ) : null}
                      </label>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              disabled={saving || form.scopes.length === 0 || form.permissions.length === 0}
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
  onClearChatHallMessages,
  onDraftChange,
  onOpenRebateSettings,
  onRegistrationChange,
  onReloadMemoryCache,
  onSaveRegistration,
  onSaveSetting,
  onSaveSettingValue,
  registration,
  saving,
  settings,
}: {
  drafts: Record<string, string>;
  lotteries: LotteryKind[];
  onClearChatHallMessages: () => Promise<ClearRecordsResult>;
  onDraftChange: (key: string, value: string) => void;
  onOpenRebateSettings?: () => void;
  onRegistrationChange: Dispatch<SetStateAction<RegistrationConfig | null>>;
  onReloadMemoryCache: () => Promise<MemoryCacheReloadResult>;
  onSaveRegistration: () => void;
  onSaveSetting: (key: string) => void;
  onSaveSettingValue: (key: string, value: string) => void;
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
  const [chatHallClearing, setChatHallClearing] = useState(false);
  const [lastCacheReloadResult, setLastCacheReloadResult] =
    useState<MemoryCacheReloadResult | null>(null);
  const [lastChatHallClearCount, setLastChatHallClearCount] =
    useState<number | null>(null);
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
    () => settingsGroupsWithRebateShortcut(filteredSettings, settingKeyword),
    [filteredSettings, settingKeyword],
  );
  const [activeSettingGroup, setActiveSettingGroup] = useState('APP更新');

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

  const handleClearChatHallMessages = async () => {
    const confirmed = window.confirm(
      '确定一键清除聊天大厅全部历史消息吗？这会清除大厅文本、红包卡片和合买分享展示记录，不会回滚已经产生的资金流水。',
    );
    if (!confirmed) {
      return;
    }

    setChatHallClearing(true);
    try {
      const result = await onClearChatHallMessages();
      setLastChatHallClearCount(result.deletedCount);
      Toast.success(`聊天大厅消息已清除：${result.deletedCount} 条`);
    } catch (error) {
      Toast.error(error instanceof Error ? error.message : '聊天大厅消息清除失败');
    } finally {
      setChatHallClearing(false);
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
              disabled={saving || cacheRefreshing || chatHallClearing}
              icon={<Trash2 size={15} />}
              loading={chatHallClearing}
              size="small"
              type="danger"
              onClick={() => void handleClearChatHallMessages()}
            >
              清除聊天大厅消息
            </Button>
            <Button
              disabled={saving || cacheRefreshing || chatHallClearing}
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
        {lastChatHallClearCount !== null ? (
          <div className="mb-3 rounded border border-orange-100 bg-orange-50 p-3 text-xs text-orange-800">
            <Tag color="orange">已清除聊天大厅消息 {lastChatHallClearCount} 条</Tag>
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
                groupName === 'APP更新'
                  ? items.filter(
                      (setting) => !MOBILE_APP_UPDATE_SETTING_KEY_SET.has(setting.key),
                    )
                : groupName === '手机端设置'
                  ? items.filter(
                      (setting) => !MOBILE_CUSTOM_SETTING_KEYS.has(setting.key),
                    )
                  : groupName === '充值设置'
                    ? items.filter(
                        (setting) =>
                          !RECHARGE_PAYMENT_SETTING_KEYS.has(setting.key),
                      )
                  : groupName === '返利设置'
                    ? items.filter(
                        (setting) => !REBATE_LEGACY_SETTING_KEYS.has(setting.key),
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

                    {groupName === 'APP更新' ? (
                      <AppUpdateSettingsPanel
                        drafts={drafts}
                        imageBedMissingConfigs={imageBedMissingConfigs}
                        imageBedUploadField={imageBedUploadField}
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
                        onSaveSettingValue={onSaveSettingValue}
                      />
                    ) : null}

                    {groupName === '返利设置' ? (
                      <RebateSettingsShortcut
                        onOpenRebateSettings={onOpenRebateSettings}
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

function RebateSettingsShortcut({
  onOpenRebateSettings,
}: {
  onOpenRebateSettings?: () => void;
}) {
  return (
    <div className="rounded border border-slate-200 bg-slate-50 p-3">
      <PanelTitle icon={<UserPlus size={18} />} title="代理返利配置入口" />
      <div className="rounded border border-slate-200 bg-white p-4">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
          <div className="min-w-0">
            <p className="text-sm font-semibold text-ink">代理返利策略配置</p>
            <p className="mt-1 text-sm leading-6 text-slate-500">
              代理邀请开关、普通用户邀请、返利模式和默认充值返利比例使用
              “返利管理 / 策略配置” 维护；这里保留入口，避免和普通系统配置混在一起。
            </p>
            <div className="mt-3 flex flex-wrap gap-2">
              <Tag color="teal">代理邀请</Tag>
              <Tag color="blue">返利模式</Tag>
              <Tag color="purple">默认充值返利比例</Tag>
              <Tag color="orange">代理申请审核</Tag>
            </div>
          </div>
          <Button
            disabled={!onOpenRebateSettings}
            icon={<UserPlus size={16} />}
            theme="solid"
            onClick={onOpenRebateSettings}
          >
            打开策略配置
          </Button>
        </div>
      </div>
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
  onSaveSettingValue,
  saving,
  settings,
}: {
  drafts: Record<string, string>;
  onDraftChange: (key: string, value: string) => void;
  onSaveSetting: (key: string) => void;
  onSaveSettingValue: (key: string, value: string) => void;
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
  const bonusEnabledValue =
    draftSettingValue(settings, drafts, RECHARGE_BONUS_ENABLED_SETTING_KEY) ||
    'false';
  const bonusRuleDrafts = rechargeBonusRuleDraftsFromSetting(
    draftSettingValue(settings, drafts, RECHARGE_BONUS_RULES_SETTING_KEY),
  );
  const bonusSummary =
    bonusEnabledValue === 'true' && bonusRuleDrafts.length > 0
      ? `${bonusRuleDrafts.length} 档活动`
      : '未开启';

  const updateBonusRules = (nextRules: RechargeBonusRuleDraft[]) => {
    onDraftChange(
      RECHARGE_BONUS_RULES_SETTING_KEY,
      JSON.stringify(nextRules),
    );
  };

  const saveBonusRules = () => {
    const submitValue = rechargeBonusRuleSubmitValue(bonusRuleDrafts);
    if (submitValue === null) {
      return;
    }
    onSaveSettingValue(RECHARGE_BONUS_RULES_SETTING_KEY, submitValue);
  };

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

      <div className="mt-3 rounded border border-slate-200 bg-white p-3">
        <div className="mb-3 flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="text-sm font-semibold text-ink">充值赠送活动</p>
            <p className="mt-1 text-xs leading-5 text-slate-500">
              按单笔充值金额匹配最高门槛档位，例如充值 100 元赠送 5 元，充值 500 元赠送 40 元。
            </p>
          </div>
          <Tag color={bonusEnabledValue === 'true' && bonusRuleDrafts.length > 0 ? 'green' : 'grey'}>
            {bonusSummary}
          </Tag>
        </div>

        <div className="grid gap-3 xl:grid-cols-[220px_minmax(0,1fr)]">
          <div className="rounded border border-slate-100 bg-slate-50 p-3">
            <Field
              description="关闭后不会给新确认入账的充值单发放赠送彩金。"
              label="活动开关"
            >
              <Select
                className="form-input"
                value={bonusEnabledValue}
                onChange={(value) =>
                  onDraftChange(
                    RECHARGE_BONUS_ENABLED_SETTING_KEY,
                    String(value ?? 'false'),
                  )
                }
              >
                <Select.Option value="true">开启充值赠送</Select.Option>
                <Select.Option value="false">关闭充值赠送</Select.Option>
              </Select>
            </Field>
            <div className="mt-3 flex justify-end">
              <Button
                disabled={saving}
                icon={<Save size={16} />}
                size="small"
                onClick={() => onSaveSetting(RECHARGE_BONUS_ENABLED_SETTING_KEY)}
              >
                保存开关
              </Button>
            </div>
          </div>

          <div className="rounded border border-slate-100 bg-slate-50 p-3">
            <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
              <div>
                <p className="text-sm font-semibold text-ink">赠送档位</p>
                <p className="mt-1 text-xs text-slate-500">
                  金额按“元”填写，保存后后端按分持久化。
                </p>
              </div>
              <Button
                disabled={saving}
                size="small"
                onClick={() =>
                  updateBonusRules([
                    ...bonusRuleDrafts,
                    { bonusAmountYuan: '', thresholdAmountYuan: '' },
                  ])
                }
              >
                新增档位
              </Button>
            </div>

            {bonusRuleDrafts.length === 0 ? (
              <div className="rounded border border-dashed border-slate-200 bg-white p-4 text-sm text-slate-500">
                暂无赠送档位，新增后可配置类似“充值 100 送 5”的活动。
              </div>
            ) : (
              <div className="space-y-2">
                {bonusRuleDrafts.map((rule, index) => (
                  <div
                    key={`bonus-rule-${index}`}
                    className="grid gap-2 rounded border border-slate-200 bg-white p-3 md:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_auto]"
                  >
                    <Field label="充值满（元）">
                      <Input
                        className="form-input"
                        inputMode="decimal"
                        placeholder="例如 100"
                        value={rule.thresholdAmountYuan}
                        onChange={(value) => {
                          const nextRules = [...bonusRuleDrafts];
                          nextRules[index] = {
                            ...rule,
                            thresholdAmountYuan: value,
                          };
                          updateBonusRules(nextRules);
                        }}
                      />
                    </Field>
                    <Field label="赠送（元）">
                      <Input
                        className="form-input"
                        inputMode="decimal"
                        placeholder="例如 5"
                        value={rule.bonusAmountYuan}
                        onChange={(value) => {
                          const nextRules = [...bonusRuleDrafts];
                          nextRules[index] = {
                            ...rule,
                            bonusAmountYuan: value,
                          };
                          updateBonusRules(nextRules);
                        }}
                      />
                    </Field>
                    <div className="flex items-end justify-end">
                      <Button
                        disabled={saving}
                        size="small"
                        type="danger"
                        onClick={() =>
                          updateBonusRules(
                            bonusRuleDrafts.filter((_, ruleIndex) => ruleIndex !== index),
                          )
                        }
                      >
                        删除
                      </Button>
                    </div>
                  </div>
                ))}
              </div>
            )}

            <div className="mt-3 flex justify-end">
              <Button
                disabled={saving}
                icon={<Save size={16} />}
                size="small"
                type="primary"
                onClick={saveBonusRules}
              >
                保存档位
              </Button>
            </div>
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

function AppUpdateSettingsPanel({
  drafts,
  imageBedMissingConfigs,
  imageBedUploadField,
  onDraftChange,
  onSaveSetting,
  saving,
  settings,
}: {
  drafts: Record<string, string>;
  imageBedMissingConfigs: string[];
  imageBedUploadField: string;
  onDraftChange: (key: string, value: string) => void;
  onSaveSetting: (key: string) => void;
  saving: boolean;
  settings: SystemSettingItem[];
}) {
  const [uploadingPackageKey, setUploadingPackageKey] = useState<string | null>(null);
  const handlePackageUpload = async (
    event: ChangeEvent<HTMLInputElement>,
    downloadKey: string,
    packageLabel: string,
  ) => {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) {
      return;
    }

    setUploadingPackageKey(downloadKey);
    try {
      const response = await uploadAppPackageFile(file, imageBedUploadField || 'file');
      const url = extractImageUrlFromUploadResult(response);
      if (!url) {
        throw new Error('上传成功但未识别到安装包下载链接，请检查图床返回链接字段配置');
      }
      onDraftChange(downloadKey, url);
      Toast.success(`${packageLabel} 上传成功，记得保存配置`);
    } catch (error: unknown) {
      Toast.error(error instanceof Error ? error.message : `${packageLabel} 上传失败`);
    } finally {
      setUploadingPackageKey(null);
    }
  };

  return (
    <div className="rounded border border-slate-200 bg-slate-50 p-3">
      <PanelTitle icon={<Smartphone size={18} />} title="APP 更新配置" />
      <div className="mb-3 rounded border border-blue-100 bg-blue-50 px-3 py-2 text-xs leading-5 text-blue-700">
        后台上传 APK/IPA 后保存下载链接；手机端启动时会自动检查版本，需要更新时弹窗提示用户下载安装。
        普通 iOS/Android 应用不能静默安装，因此这里提供自动检查和引导更新。
      </div>
      <div className="grid gap-3">
        {MOBILE_APP_UPDATE_PLATFORMS.map((platform) => {
          const enabledValue =
            draftSettingValue(settings, drafts, platform.enabledKey) || 'false';
          const forceValue =
            draftSettingValue(settings, drafts, platform.forceKey) || 'false';
          const versionValue =
            draftSettingValue(settings, drafts, platform.versionKey) || '0.1.0';
          const buildValue =
            draftSettingValue(settings, drafts, platform.buildKey) || '1';
          const downloadValue = draftSettingValue(
            settings,
            drafts,
            platform.downloadKey,
          );
          const notesValue = draftSettingValue(settings, drafts, platform.notesKey);
          const isUploading = uploadingPackageKey === platform.downloadKey;

          return (
            <div
              key={platform.label}
              className="grid gap-3 rounded border border-slate-200 bg-white p-3"
            >
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <div className="text-sm font-semibold text-ink">
                    {platform.label} 安装包与更新策略
                  </div>
                  <p className="mt-1 text-xs text-slate-500">
                    构建号优先判断版本，新构建号需要大于客户端当前构建号。
                  </p>
                </div>
                <Tag color={enabledValue === 'true' ? 'green' : 'grey'}>
                  {enabledValue === 'true' ? '检查已开启' : '检查已关闭'}
                </Tag>
              </div>

              <div className="grid gap-3 md:grid-cols-2">
                <div>
                  <span className="mb-1 block text-xs font-medium text-slate-500">
                    更新检查
                  </span>
                  <Select
                    className="form-input"
                    value={enabledValue}
                    onChange={(value) =>
                      onDraftChange(platform.enabledKey, String(value ?? 'false'))
                    }
                  >
                    <Select.Option value="false">关闭检查</Select.Option>
                    <Select.Option value="true">开启检查</Select.Option>
                  </Select>
                </div>
                <div>
                  <span className="mb-1 block text-xs font-medium text-slate-500">
                    更新方式
                  </span>
                  <Select
                    className="form-input"
                    value={forceValue}
                    onChange={(value) =>
                      onDraftChange(platform.forceKey, String(value ?? 'false'))
                    }
                  >
                    <Select.Option value="false">可选更新</Select.Option>
                    <Select.Option value="true">强制更新</Select.Option>
                  </Select>
                </div>
                <div>
                  <span className="mb-1 block text-xs font-medium text-slate-500">
                    最新版本号
                  </span>
                  <Input
                    className="form-input"
                    placeholder="例如：1.2.0"
                    value={versionValue}
                    onChange={(value) => onDraftChange(platform.versionKey, value)}
                  />
                </div>
                <div>
                  <span className="mb-1 block text-xs font-medium text-slate-500">
                    最新构建号
                  </span>
                  <Input
                    className="form-input"
                    placeholder="例如：12"
                    value={buildValue}
                    onChange={(value) => onDraftChange(platform.buildKey, value)}
                  />
                </div>
              </div>

              <div>
                <span className="mb-1 block text-xs font-medium text-slate-500">
                  安装包下载链接
                </span>
                <div className="grid gap-2 md:grid-cols-[minmax(0,1fr)_auto]">
                  <Input
                    className="form-input"
                    placeholder={`上传或粘贴 ${platform.packageLabel} 下载链接`}
                    value={downloadValue}
                    onChange={(value) => onDraftChange(platform.downloadKey, value)}
                  />
                  <span className="relative inline-flex">
                    <Button
                      disabled={
                        saving ||
                        isUploading ||
                        imageBedMissingConfigs.length > 0
                      }
                      icon={<UploadIcon size={16} />}
                      loading={isUploading}
                    >
                      上传 {platform.packageLabel}
                    </Button>
                    <input
                      accept={platform.accept}
                      aria-label={`上传 ${platform.packageLabel} 安装包`}
                      className="absolute inset-0 cursor-pointer opacity-0 disabled:pointer-events-none"
                      disabled={
                        saving ||
                        isUploading ||
                        imageBedMissingConfigs.length > 0
                      }
                      type="file"
                      onChange={(event) =>
                        void handlePackageUpload(
                          event,
                          platform.downloadKey,
                          platform.packageLabel,
                        )
                      }
                    />
                  </span>
                </div>
                {imageBedMissingConfigs.length > 0 ? (
                  <p className="mt-1 text-xs text-amber-600">
                    请先补全图床配置：{imageBedMissingConfigs.join('、')}
                  </p>
                ) : null}
              </div>

              <div>
                <span className="mb-1 block text-xs font-medium text-slate-500">
                  更新说明
                </span>
                <Input
                  className="form-input"
                  placeholder="填写本次更新给用户展示的中文说明"
                  value={notesValue}
                  onChange={(value) => onDraftChange(platform.notesKey, value)}
                />
              </div>

              <div className="flex flex-wrap justify-end gap-2">
                {[
                  platform.enabledKey,
                  platform.versionKey,
                  platform.buildKey,
                  platform.downloadKey,
                  platform.forceKey,
                  platform.notesKey,
                ].map((key) => (
                  <Button
                    key={key}
                    disabled={saving}
                    icon={<Save size={16} />}
                    size="small"
                    onClick={() => onSaveSetting(key)}
                  >
                    保存{settingShortLabel(key)}
                  </Button>
                ))}
              </div>
            </div>
          );
        })}
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

function Field({
  children,
  description,
  label,
}: {
  children: ReactNode;
  description?: string;
  label: string;
}) {
  return (
    <label className="block text-sm font-medium text-slate-600">
      <span className="mb-1 block">{label}</span>
      {children}
      {description ? (
        <span className="mt-1 block text-xs font-normal text-slate-400">
          {description}
        </span>
      ) : null}
    </label>
  );
}

function settingGroupName(key: string): string {
  if (key.startsWith('mobile_app_')) {
    return 'APP更新';
  }
  if (key.startsWith('mobile_')) {
    return '手机端设置';
  }
  if (key.startsWith('image_bed_')) {
    return '图床设置';
  }
  if (
    key.startsWith('recharge_rainbow_epay_') ||
    key.startsWith('recharge_customer_service_') ||
    key.startsWith('recharge_bonus_') ||
    key === RECHARGE_MIN_AMOUNT_SETTING_KEY ||
    key === RECHARGE_MAX_AMOUNT_SETTING_KEY
  ) {
    return '充值设置';
  }
  if (key.startsWith('support_telegram_')) {
    return '通知设置';
  }
  if (key.startsWith('chat_hall_')) {
    return '聊天设置';
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
  const allowZero = key === CHAT_HALL_SPEAKING_MIN_RECHARGE_SETTING_KEY;
  if (amountMinor === null || amountMinor < 0 || (!allowZero && amountMinor <= 0)) {
    Toast.warning(
      allowZero
        ? '聊天大厅发言门槛必须大于等于 0 元且最多保留两位小数'
        : '充值金额设置必须大于 0 元且最多保留两位小数',
    );
    return null;
  }
  return String(amountMinor);
}

function settingDescription(setting: SystemSettingItem) {
  if (setting.key === RECHARGE_BONUS_RULES_SETTING_KEY) {
    return '用户充值赠送活动档位，请在下方“赠送档位”区域按元维护，不需要手写 JSON';
  }
  if (setting.key === CHAT_HALL_SPEAKING_MIN_RECHARGE_SETTING_KEY) {
    return '聊天大厅发言最低累计充值金额（元），0 表示不限制';
  }
  if (!isMinorMoneySetting(setting.key)) {
    return setting.description;
  }
  return setting.description
    .replace('（分）', '（元）')
    .replace('金额单位为分', '金额按元填写');
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
    recharge_bonus_enabled: [
      { label: '开启充值赠送', value: 'true' },
      { label: '关闭充值赠送', value: 'false' },
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

function settingShortLabel(key: string) {
  if (key.endsWith('_enabled')) {
    return '开关';
  }
  if (key.endsWith('_latest_version')) {
    return '版本';
  }
  if (key.endsWith('_latest_build')) {
    return '构建号';
  }
  if (key.endsWith('_package_url')) {
    return '链接';
  }
  if (key.endsWith('_force_update')) {
    return '方式';
  }
  if (key.endsWith('_release_notes')) {
    return '说明';
  }
  return '配置';
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
    'APP更新',
    '手机端设置',
    '图床设置',
    '充值设置',
    '聊天设置',
    '通知设置',
    '注册与安全',
    '返利设置',
    '基础设置',
  ];
  return priority
    .filter((name) => (groups.get(name)?.length ?? 0) > 0)
    .map((name) => [name, groups.get(name) ?? []]);
}

function settingsGroupsWithRebateShortcut(
  settings: SystemSettingItem[],
  keyword: string,
): Array<[string, SystemSettingItem[]]> {
  const groups = settingsGroups(settings);
  if (!rebateShortcutMatchesKeyword(keyword)) {
    return groups;
  }
  if (groups.some(([name]) => name === '返利设置')) {
    return groups;
  }
  const nextGroups = [...groups];
  const basicIndex = nextGroups.findIndex(([name]) => name === '基础设置');
  const insertIndex = basicIndex >= 0 ? basicIndex : nextGroups.length;
  nextGroups.splice(insertIndex, 0, ['返利设置', []]);
  return nextGroups;
}

function rebateShortcutMatchesKeyword(keyword: string) {
  const normalized = keyword.trim().toLowerCase();
  if (!normalized) {
    return true;
  }
  return [
    '代理返利',
    '返利设置',
    '返利管理',
    '策略配置',
    'recharge_rebate',
    'rebate',
  ].some((item) => item.toLowerCase().includes(normalized));
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

function rechargeBonusRuleDraftsFromSetting(value: string): RechargeBonusRuleDraft[] {
  try {
    const parsed = JSON.parse(value || '[]');
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed
      .map((item) => {
        const record = item as Record<string, unknown>;
        const thresholdAmountYuan =
          typeof record.thresholdAmountYuan === 'string'
            ? record.thresholdAmountYuan
            : minorToYuanInput(record.thresholdAmountMinor as number | string | undefined);
        const bonusAmountYuan =
          typeof record.bonusAmountYuan === 'string'
            ? record.bonusAmountYuan
            : minorToYuanInput(record.bonusAmountMinor as number | string | undefined);
        return { bonusAmountYuan, thresholdAmountYuan };
      });
  } catch {
    return [];
  }
}

function rechargeBonusRuleSubmitValue(rules: RechargeBonusRuleDraft[]) {
  const parsed = rules
    .map((rule) => {
      const thresholdAmountMinor = yuanInputToMinor(rule.thresholdAmountYuan);
      const bonusAmountMinor = yuanInputToMinor(rule.bonusAmountYuan);
      return { bonusAmountMinor, thresholdAmountMinor };
    })
    .filter(
      (rule) =>
        rule.thresholdAmountMinor !== null ||
        rule.bonusAmountMinor !== null,
    );

  if (
    parsed.some(
      (rule) => rule.thresholdAmountMinor === null || rule.bonusAmountMinor === null,
    )
  ) {
    Toast.warning('充值赠送档位金额格式无效，请填写大于 0 且最多两位小数的元金额');
    return null;
  }
  const normalized = parsed as Array<{
    bonusAmountMinor: number;
    thresholdAmountMinor: number;
  }>;
  if (
    normalized.some(
      (rule) => rule.thresholdAmountMinor <= 0 || rule.bonusAmountMinor <= 0,
    )
  ) {
    Toast.warning('充值赠送档位的充值门槛和赠送金额都必须大于 0 元');
    return null;
  }

  const sorted = [...normalized].sort(
    (left, right) =>
      left.thresholdAmountMinor - right.thresholdAmountMinor ||
      left.bonusAmountMinor - right.bonusAmountMinor,
  );
  const duplicatedThreshold = sorted.some(
    (rule, index) =>
      index > 0 && rule.thresholdAmountMinor === sorted[index - 1].thresholdAmountMinor,
  );
  if (duplicatedThreshold) {
    Toast.warning('充值赠送档位不能配置重复的充值门槛');
    return null;
  }

  return JSON.stringify(
    sorted.map((rule) => ({
      thresholdAmountMinor: rule.thresholdAmountMinor,
      bonusAmountMinor: rule.bonusAmountMinor,
    })),
  );
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

function emptyRegistrationLocation(): UserRegistrationLocation {
  return {
    city: '',
    country: '',
    registeredIp: '',
    region: '',
    source: 'unknown',
  };
}

function emptyUserForm(): UserFormState {
  return {
    agentId: '',
    balanceMinor: '0',
    contactQq: '',
    email: '',
    id: 'U20001',
    inviteCode: '',
    kind: 'regular',
    password: '',
    registrationLocation: emptyRegistrationLocation(),
    status: 'active',
    username: 'new_user',
  };
}

function userFormFromSummary(user: UserSummary): UserFormState {
  return {
    agentId: user.agentId ?? '',
    balanceMinor: `${user.balanceMinor}`,
    contactQq: user.contactQq ?? '',
    email: user.email ?? '',
    id: user.id,
    inviteCode: user.inviteCode,
    kind: user.kind,
    password: '',
    registrationLocation: user.registrationLocation ?? emptyRegistrationLocation(),
    status: user.status,
    username: user.username,
  };
}

function userPayload(form: UserFormState): UserSummary {
  return {
    agentId: optionalText(form.agentId),
    balanceMinor: numberField(form.balanceMinor),
    contactQq: form.contactQq.trim(),
    email: optionalText(form.email),
    id: form.id.trim(),
    inviteCode: form.inviteCode.trim(),
    kind: form.kind,
    registrationLocation: form.registrationLocation,
    status: form.status,
    username: form.username.trim(),
  };
}

function registrationLocationLabel(location?: UserRegistrationLocation) {
  if (!location) return '未知地区';
  const parts = [location.country, location.region, location.city]
    .map((part) => part?.trim())
    .filter(Boolean);
  if (parts.length > 0) return parts.join(' / ');
  if (location.registeredIp?.trim()) return `IP ${location.registeredIp.trim()}`;
  return '未知地区';
}

function registrationLocationMeta(location?: UserRegistrationLocation) {
  if (!location) return '来源：未知';
  const source = registrationSourceText(location.source);
  const ip = location.registeredIp?.trim();
  return ip ? `来源：${source} · IP ${ip}` : `来源：${source}`;
}

function registrationSourceText(source?: string) {
  switch ((source ?? 'unknown').trim()) {
    case 'gps':
      return '客户端定位';
    case 'ip':
      return '请求 IP';
    case 'client':
      return '客户端上报';
    default:
      return '未知';
  }
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
    permissions: ['user.read'],
    scopes: ['users'],
  };
}

function roleFormFromSummary(role: AdminRole): RoleFormState {
  return {
    id: role.id,
    name: role.name,
    permissions: effectiveRolePermissions(role),
    scopes: role.scopes,
  };
}

function rolePayload(form: RoleFormState): AdminRole {
  return {
    id: form.id.trim(),
    name: form.name.trim(),
    permissions: Array.from(new Set(form.permissions)),
    scopes: Array.from(new Set(form.scopes)),
  };
}

function toggleScope(
  setForm: Dispatch<SetStateAction<RoleFormState>>,
  scope: PermissionScope,
  checked: boolean,
) {
  setForm((current) => {
    const hasScopePermission = current.permissions.some(
      (permission) => permissionDefinition(permission)?.scope === scope,
    );
    return {
      ...current,
      permissions: checked
        ? hasScopePermission
          ? current.permissions
          : Array.from(new Set([...current.permissions, ...defaultPermissionsForScope(scope)]))
        : current.permissions.filter(
            (permission) => permissionDefinition(permission)?.scope !== scope,
          ),
      scopes: checked
        ? Array.from(new Set([...current.scopes, scope]))
        : current.scopes.filter((item) => item !== scope),
    };
  });
}

function togglePermission(
  setForm: Dispatch<SetStateAction<RoleFormState>>,
  permission: PermissionDefinition,
  checked: boolean,
) {
  setForm((current) => ({
    ...current,
    permissions: checked
      ? Array.from(new Set([...current.permissions, permission.key]))
      : current.permissions.filter((item) => item !== permission.key),
    scopes: checked
      ? Array.from(new Set([...current.scopes, permission.scope]))
      : current.scopes,
  }));
}

function togglePermissionGroup(
  setForm: Dispatch<SetStateAction<RoleFormState>>,
  permissions: PermissionDefinition[],
  checked: boolean,
) {
  setForm((current) => {
    const permissionKeys = permissions.map((permission) => permission.key);
    const scopes = permissions.map((permission) => permission.scope);
    return {
      ...current,
      permissions: checked
        ? Array.from(new Set([...current.permissions, ...permissionKeys]))
        : current.permissions.filter((permission) => !permissionKeys.includes(permission)),
      scopes: checked
        ? Array.from(new Set([...current.scopes, ...scopes]))
        : current.scopes,
    };
  });
}

function effectiveRolePermissions(role: AdminRole) {
  if (role.permissions && role.permissions.length > 0) {
    return role.permissions.filter((permission) => Boolean(permissionDefinition(permission)));
  }
  return permissionsForScopes(role.scopes);
}

function permissionsForScopes(scopes: PermissionScope[]) {
  const scopeSet = new Set(scopes);
  return PERMISSION_DEFINITIONS
    .filter((permission) => scopeSet.has(permission.scope))
    .map((permission) => permission.key);
}

function defaultPermissionsForScope(scope: PermissionScope) {
  return PERMISSION_DEFINITIONS
    .filter((permission) => permission.scope === scope && !permission.sensitive)
    .slice(0, 1)
    .map((permission) => permission.key);
}

function permissionDefinition(permissionKey: PermissionKey) {
  return PERMISSION_DEFINITIONS.find((permission) => permission.key === permissionKey);
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

function passwordNeedsMoreChars(password: string) {
  const trimmed = password.trim();
  return trimmed.length > 0 && trimmed.length < 8;
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

function inactiveUserStatusActionLabel(status: UserStatus) {
  if (status === 'active') {
    return '停用';
  }
  return status === 'locked' ? '已锁定' : '已停用';
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
