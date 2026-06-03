import { Banner, Button, Card, SideSheet, Spin, Tag } from '@douyinfe/semi-ui';
import {
  RefreshCcw,
  Save,
  Settings,
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
            <input
              className="form-input"
              value={form.id}
              onChange={(event) => setFormValue(onSetForm, 'id', event.target.value)}
            />
          </Field>
          <Field label="用户名">
            <input
              className="form-input"
              value={form.username}
              onChange={(event) =>
                setFormValue(onSetForm, 'username', event.target.value)
              }
            />
          </Field>
          <Field label="邮箱">
            <input
              className="form-input"
              value={form.email}
              onChange={(event) =>
                setFormValue(onSetForm, 'email', event.target.value)
              }
            />
          </Field>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-1">
            <Field label="用户类型">
              <select
                className="form-input"
                value={form.kind}
                onChange={(event) =>
                  setFormValue(onSetForm, 'kind', event.target.value as UserKind)
                }
              >
                <option value="regular">普通用户</option>
                <option value="agent">代理</option>
              </select>
            </Field>
            <Field label="状态">
              <select
                className="form-input"
                value={form.status}
                onChange={(event) =>
                  setFormValue(onSetForm, 'status', event.target.value as UserStatus)
                }
              >
                <option value="active">启用</option>
                <option value="suspended">停用</option>
                <option value="locked">锁定</option>
              </select>
            </Field>
          </div>
          <Field label="余额（分）">
            <input
              className="form-input"
              type="number"
              value={form.balanceMinor}
              onChange={(event) =>
                setFormValue(onSetForm, 'balanceMinor', event.target.value)
              }
            />
          </Field>
          <Field label="上级代理 ID">
            <input
              className="form-input"
              value={form.agentId}
              onChange={(event) =>
                setFormValue(onSetForm, 'agentId', event.target.value)
              }
            />
          </Field>
          <Field label="邀请码">
            <input
              className="form-input"
              placeholder="留空时后端自动生成"
              value={form.inviteCode}
              onChange={(event) =>
                setFormValue(onSetForm, 'inviteCode', event.target.value)
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
            <input
              className="form-input"
              value={form.id}
              onChange={(event) => setFormValue(onSetForm, 'id', event.target.value)}
            />
          </Field>
          <Field label="用户名">
            <input
              className="form-input"
              value={form.username}
              onChange={(event) =>
                setFormValue(onSetForm, 'username', event.target.value)
              }
            />
          </Field>
          <Field label={editingId ? '重置密码' : '初始密码'}>
            <input
              autoComplete="new-password"
              className="form-input"
              placeholder={editingId ? '留空则不修改密码' : '至少 8 位'}
              type="password"
              value={form.password}
              onChange={(event) =>
                setFormValue(onSetForm, 'password', event.target.value)
              }
            />
          </Field>
          <Field label="角色">
            <select
              className="form-input"
              value={form.roleId}
              onChange={(event) =>
                setFormValue(onSetForm, 'roleId', event.target.value)
              }
            >
              {roles.map((role) => (
                <option key={role.id} value={role.id}>
                  {role.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label="状态">
            <select
              className="form-input"
              value={form.status}
              onChange={(event) =>
                setFormValue(onSetForm, 'status', event.target.value as UserStatus)
              }
            >
              <option value="active">启用</option>
              <option value="suspended">停用</option>
              <option value="locked">锁定</option>
            </select>
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
            <input
              className="form-input"
              value={form.id}
              onChange={(event) => setFormValue(onSetForm, 'id', event.target.value)}
            />
          </Field>
          <Field label="角色名称">
            <input
              className="form-input"
              value={form.name}
              onChange={(event) => setFormValue(onSetForm, 'name', event.target.value)}
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
  return (
    <section className="grid gap-4 xl:grid-cols-[1fr_420px]">
      <Card className="rounded-md border border-line">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-base font-semibold text-ink">系统设置</h2>
          <Tag color="cyan">{settings.length} 项</Tag>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full min-w-[760px] text-left text-sm">
            <thead className="border-b border-line text-xs text-slate-500">
              <tr>
                <th className="py-2 pr-4 font-medium">配置</th>
                <th className="py-2 pr-4 font-medium">值</th>
                <th className="py-2 pr-4 font-medium">说明</th>
                <th className="py-2 pr-4 font-medium">操作</th>
              </tr>
            </thead>
            <tbody>
              {settings.map((setting) => (
                <tr key={setting.key} className="border-b border-slate-100">
                  <td className="py-3 pr-4 font-semibold text-ink">{setting.key}</td>
                  <td className="py-3 pr-4">
                    <input
                      className="form-input min-w-40"
                      value={drafts[setting.key] ?? setting.value}
                      onChange={(event) =>
                        onDraftChange(setting.key, event.target.value)
                      }
                    />
                  </td>
                  <td className="py-3 pr-4 text-slate-600">{setting.description}</td>
                  <td className="py-3 pr-4">
                    <Button
                      disabled={saving}
                      size="small"
                      onClick={() => onSaveSetting(setting.key)}
                    >
                      保存
                    </Button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Card>

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
