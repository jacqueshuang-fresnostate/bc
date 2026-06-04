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
  RefreshCcw,
  Save,
  Settings,
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
import { ImageUploadAvatar } from '../components/ImageUploadAvatar';
import { MetricCard } from '../components/MetricCard';
import { useAccessManagement } from '../hooks/useAccessManagement';
import type { AdminSaveRequest } from '../types/access';
import type {
  AdminRole,
  AdminSummary,
  PermissionScope,
  RegistrationConfig,
  UserKind,
  UserStatus,
  UserSummary,
} from '../types/dashboard';
import { formatMoney } from '../utils/format';

type AccessSection = 'admins' | 'roles' | 'settings' | 'users';

interface AccessManagementPageProps {
  activeModuleKey: string;
  onDashboardRefresh: () => void;
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
  { key: 'settings', label: '系统设置' },
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

export function AccessManagementPage({
  activeModuleKey,
  onDashboardRefresh,
}: AccessManagementPageProps) {
  const {
    admins,
    changeAdminStatus,
    changeUserStatus,
    error,
    loading,
    refresh,
    registration,
    removeRole,
    resetPassword,
    roles,
    saveAdmin,
    saveRegistration,
    saveRole,
    saveSetting,
    saveUser,
    saving,
    settings,
    users,
  } = useAccessManagement();
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
  const [roleForm, setRoleForm] = useState<RoleFormState>(() => emptyRoleForm());
  const [settingDrafts, setSettingDrafts] = useState<Record<string, string>>({});
  const [userForm, setUserForm] = useState<UserFormState>(() => emptyUserForm());
  const totals = useMemo(() => accessTotals(users, admins), [admins, users]);

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
        drafts[setting.key] = setting.value;
        return drafts;
      }, {}),
    );
  }, [settings]);

  useEffect(() => {
    if (registration) {
      setRegistrationForm(registration);
    }
  }, [registration]);

  const refreshAll = () => {
    refresh();
    onDashboardRefresh();
  };

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
          <h1 className="text-xl font-semibold text-ink">用户权限管理</h1>
          <p className="mt-1 text-sm text-slate-500">
            维护用户、后台账号、角色范围和注册配置。
          </p>
        </div>
        <Button icon={<RefreshCcw size={16} />} onClick={refreshAll}>
          刷新
        </Button>
      </section>

      {error ? <Banner type="danger" title="用户权限接口错误" description={error} /> : null}

      <section className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          label="用户总数"
          trend={`${totals.agentCount} 个代理`}
          value={`${users.length}`}
        />
        <MetricCard
          label="活跃用户"
          trend="可参与投注"
          value={`${totals.activeUserCount}`}
        />
        <MetricCard
          label="后台账号"
          trend={`${totals.lockedAdminCount} 个锁定`}
          value={`${admins.length}`}
        />
        <MetricCard
          label="角色数量"
          trend="权限范围绑定"
          value={`${roles.length}`}
        />
      </section>

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
          saving={saving}
          sheetVisible={userSheetVisible}
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
          onSetForm={setUserForm}
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
          registration={registrationForm}
          saving={saving}
          settings={settings}
          onDraftChange={(key, value) =>
            setSettingDrafts((current) => ({ ...current, [key]: value }))
          }
          onRegistrationChange={setRegistrationForm}
          onSaveRegistration={() => void submitRegistration()}
          onSaveSetting={(key) => {
            const value = settingDrafts[key] ?? '';
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
  onClose,
  onEdit,
  onNew,
  onSetForm,
  onStatus,
  onSubmit,
  saving,
  sheetVisible,
  users,
}: {
  editingId: string | null;
  form: UserFormState;
  onClose: () => void;
  onEdit: (user: UserSummary) => void;
  onNew: () => void;
  onSetForm: Dispatch<SetStateAction<UserFormState>>;
  onStatus: (id: string, status: UserStatus) => void;
  onSubmit: () => void;
  saving: boolean;
  sheetVisible: boolean;
  users: UserSummary[];
}) {
  return (
    <section className="space-y-4">
      <Card className="rounded-md border border-line">
        <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
          <h2 className="text-base font-semibold text-ink">用户列表</h2>
          <div className="flex items-center gap-2">
            <Tag color="cyan">{users.length} 个用户</Tag>
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
                    {user.agentId ?? '无'}
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
          <Field label="余额（分）">
            <Input
              className="form-input"
              type="number"
              value={form.balanceMinor}
              onChange={(value) =>
                setFormValue(onSetForm, 'balanceMinor', value)
              }
            />
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
              placeholder="留空时后端自动生成"
              value={form.inviteCode}
              onChange={(value) =>
                setFormValue(onSetForm, 'inviteCode', value)
              }
            />
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
  onDraftChange,
  onRegistrationChange,
  onSaveRegistration,
  onSaveSetting,
  registration,
  saving,
  settings,
}: {
  drafts: Record<string, string>;
  onDraftChange: (key: string, value: string) => void;
  onRegistrationChange: Dispatch<SetStateAction<RegistrationConfig | null>>;
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
  const imageBedMissingConfigs = [
    imageBedUploadUrl.trim() ? null : '上传地址',
    imageBedUploadToken.trim() ? null : 'Token',
    imageBedUploadField.trim() ? null : '上传字段名',
  ].filter(Boolean) as string[];

  const filteredSettings = settings.filter((setting) => {
    const keyword = settingKeyword.trim().toLowerCase();
    if (!keyword) {
      return true;
    }
    return (
      setting.key.toLowerCase().includes(keyword) ||
      setting.description.toLowerCase().includes(keyword)
    );
  });
  const groupedSettings = settingsGroups(filteredSettings);

  return (
    <section className="grid gap-4 xl:grid-cols-[1fr_420px]">
      <Card className="rounded-md border border-line">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-base font-semibold text-ink">系统设置</h2>
          <Tag color="cyan">{settings.length} 项</Tag>
        </div>
        <div className="mb-3">
          <Input
            className="form-input"
            placeholder="搜索配置项 / 说明"
            value={settingKeyword}
            onChange={(value) => setSettingKeyword(value)}
          />
        </div>
        <div className="space-y-3">
          {groupedSettings.length === 0 ? (
            <div className="rounded border border-slate-200 bg-slate-50 p-4 text-sm text-slate-500">
              未找到匹配的系统配置项，可清空关键字后重试。
            </div>
          ) : (
            groupedSettings.map(([groupName, items]) => (
              <div
                key={groupName}
                className="rounded-lg border border-slate-200 bg-white p-3"
              >
                <div className="mb-2 flex items-center justify-between">
                  <h3 className="text-sm font-medium text-slate-700">
                    {groupName}
                  </h3>
                  <Tag color="grey">{items.length} 项</Tag>
                </div>
                <div className="space-y-3">
                  {items.map((setting) => {
                    const draftValue = drafts[setting.key] ?? setting.value;
                    const selectOptions = settingSelectOptions(
                      setting.key,
                      draftValue,
                    );

                    return (
                      <div
                        key={setting.key}
                        className="grid gap-2 rounded border border-slate-100 p-2"
                      >
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <div className="min-w-0">
                            <p className="truncate text-sm font-medium text-ink">
                              {setting.key}
                            </p>
                            <p className="mt-1 text-xs text-slate-500">
                              {setting.description}
                            </p>
                          </div>
                          <div>
                            <Button
                              disabled={saving}
                              size="small"
                              onClick={() => onSaveSetting(setting.key)}
                            >
                              保存
                            </Button>
                          </div>
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
                              <Select.Option
                                key={option.value}
                                value={option.value}
                              >
                                {option.label}
                              </Select.Option>
                            ))}
                          </Select>
                        ) : (
                          <Input
                            className="form-input"
                            value={draftValue}
                            onChange={(value) =>
                              onDraftChange(setting.key, value)
                            }
                          />
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            ))
          )}
        </div>
      </Card>

      <div className="space-y-4">
        <Card className="rounded-md border border-line">
          <PanelTitle icon={<Settings size={18} />} title="注册配置" />
          {registration ? (
            <div className="space-y-4">
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
        </Card>
        <Card className="rounded-md border border-line">
          <PanelTitle icon={<UploadIcon size={18} />} title="图床上传测试" />
          <div className="space-y-4">
            <div className="grid gap-2 text-sm text-slate-600">
              <div className="flex items-center justify-between gap-3 rounded border border-slate-200 bg-slate-50 px-3 py-2">
                <span>上传地址</span>
                <span className="min-w-0 truncate font-mono text-xs text-slate-700">
                  {imageBedUploadUrl || '未配置'}
                </span>
              </div>
              <div className="grid grid-cols-2 gap-2">
                <div className="rounded border border-slate-200 bg-slate-50 px-3 py-2">
                  <p className="text-xs text-slate-500">上传字段名</p>
                  <p className="mt-1 truncate font-mono text-sm text-slate-700">
                    {imageBedUploadField || 'file'}
                  </p>
                </div>
                <div className="rounded border border-slate-200 bg-slate-50 px-3 py-2">
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
        </Card>
      </div>
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

function Field({ children, label }: { children: ReactNode; label: string }) {
  return (
    <label className="block text-sm font-medium text-slate-600">
      <span className="mb-1 block">{label}</span>
      {children}
    </label>
  );
}

function settingGroupName(key: string): string {
  if (key.startsWith('image_bed_')) {
    return '图床设置';
  }
  if (
    key.startsWith('recharge_rainbow_epay_') ||
    key.startsWith('recharge_customer_service_') ||
    key === 'recharge_min_amount_minor' ||
    key === 'recharge_max_amount_minor'
  ) {
    return '充值设置';
  }
  if (key.includes('email') || key.includes('registration')) {
    return '注册与安全';
  }
  if (key.includes('recharge') || key.includes('rebate')) {
    return '返利设置';
  }
  return '基础设置';
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

  const priority = ['图床设置', '充值设置', '注册与安全', '返利设置', '基础设置'];
  return priority
    .filter((name) => (groups.get(name)?.length ?? 0) > 0)
    .map((name) => [name, groups.get(name) ?? []]);
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

function accessTotals(users: UserSummary[], admins: AdminSummary[]) {
  return {
    activeUserCount: users.filter((user) => user.status === 'active').length,
    agentCount: users.filter((user) => user.kind === 'agent').length,
    lockedAdminCount: admins.filter((admin) => admin.status === 'locked').length,
  };
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
