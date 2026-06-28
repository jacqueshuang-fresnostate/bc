import { useCallback, useEffect, useState } from 'react';
import {
  createAdmin,
  createRole,
  createUser,
  clearChatHallMessages,
  deleteRole,
  deleteUser,
  fetchAdmins,
  fetchRegistrationConfig,
  fetchRoles,
  fetchSystemSettings,
  fetchUserPage,
  reloadBackendMemoryCache,
  resetAdminPassword,
  resetUserPassword,
  setAdminStatus,
  setUserStatus,
  updateAdmin,
  updateRegistrationConfig,
  updateRole,
  updateSystemSetting,
  updateUser,
} from '../api/client';
import type {
  AdminUserSummary,
  AdminSaveRequest,
  AdminRole,
  AdminSummary,
  AdminPasswordResetRequest,
  MemoryCacheReloadResult,
  RegistrationConfig,
  SystemSetting,
  UserListQuery,
  UserPage,
  UserPasswordResetRequest,
  UserStatus,
  UserSummary,
} from '../types/access';
import type { ClearRecordsResult } from '../types/finance';

interface UseAccessManagementOptions {
  userQuery: UserListQuery;
}

export function useAccessManagement({ userQuery }: UseAccessManagementOptions) {
  const [admins, setAdmins] = useState<AdminSummary[]>([]);
  const [registration, setRegistration] = useState<RegistrationConfig | null>(null);
  const [roles, setRoles] = useState<AdminRole[]>([]);
  const [settings, setSettings] = useState<SystemSetting[]>([]);
  const [userPage, setUserPage] = useState<UserPage>(emptyUserPage);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);

  const refresh = useCallback(() => {
    setRefreshToken((current) => current + 1);
  }, []);

  useEffect(() => {
    const controller = new AbortController();

    setLoading(true);
    setError(null);

    Promise.all([
      fetchUserPage(controller.signal, userQuery),
      fetchAdmins(controller.signal),
      fetchRoles(controller.signal),
      fetchSystemSettings(controller.signal),
      fetchRegistrationConfig(controller.signal),
    ])
      .then(
        ([
          nextUserPage,
          nextAdmins,
          nextRoles,
          nextSettings,
          nextRegistration,
        ]) => {
          setUserPage(nextUserPage);
          setAdmins(nextAdmins);
          setRoles(nextRoles);
          setSettings(nextSettings);
          setRegistration(nextRegistration);
        },
      )
      .catch((requestError: unknown) => {
        if (!controller.signal.aborted) {
          setError(errorMessage(requestError));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => {
      controller.abort();
    };
  }, [
    refreshToken,
    userQuery.page,
    userQuery.pageSize,
    userQuery.includeRobotData,
    userQuery.sortBy,
    userQuery.sortDirection,
    userQuery.status,
    userQuery.username,
  ]);

  const saveUser = useCallback(async (payload: UserSummary, existingId?: string) => {
    setSaving(true);
    setError(null);
    try {
      const saved = existingId
        ? await updateUser(existingId, payload)
        : await createUser(payload);
      setUserPage((current) => replacePageUser(current, saved));
      refresh();
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, [refresh]);

  const changeUserStatus = useCallback(async (id: string, status: UserStatus) => {
    setSaving(true);
    setError(null);
    try {
      const saved = await setUserStatus(id, { status });
      setUserPage((current) => replacePageUser(current, saved));
      refresh();
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, [refresh]);

  const removeUser = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const deleted = await deleteUser(id);
      setUserPage((current) => removePageUser(current, id));
      refresh();
      return deleted;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, [refresh]);

  const resetUserLoginPassword = useCallback(
    async (id: string, payload: UserPasswordResetRequest) => {
      setSaving(true);
      setError(null);
      try {
        const saved = await resetUserPassword(id, payload);
        setUserPage((current) => replacePageUser(current, saved));
        refresh();
        return saved;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [refresh],
  );

  const saveAdmin = useCallback(
    async (payload: AdminSaveRequest, existingId?: string) => {
      setSaving(true);
      setError(null);
      try {
        const saved = existingId
          ? await updateAdmin(existingId, payload)
          : await createAdmin(payload);
        setAdmins((current) => upsertById(current, saved));
        return saved;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  const resetPassword = useCallback(
    async (id: string, payload: AdminPasswordResetRequest) => {
      setSaving(true);
      setError(null);
      try {
        const saved = await resetAdminPassword(id, payload);
        setAdmins((current) => upsertById(current, saved));
        return saved;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  const changeAdminStatus = useCallback(async (id: string, status: UserStatus) => {
    setSaving(true);
    setError(null);
    try {
      const saved = await setAdminStatus(id, { status });
      setAdmins((current) => upsertById(current, saved));
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const saveRole = useCallback(async (payload: AdminRole, existingId?: string) => {
    setSaving(true);
    setError(null);
    try {
      const saved = existingId
        ? await updateRole(existingId, payload)
        : await createRole(payload);
      setRoles((current) => upsertById(current, saved));
      const nextAdmins = await fetchAdmins();
      setAdmins(nextAdmins);
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const removeRole = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const deleted = await deleteRole(id);
      setRoles((current) => current.filter((role) => role.id !== id));
      return deleted;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const saveSetting = useCallback(async (key: string, value: string) => {
    setSaving(true);
    setError(null);
    try {
      const saved = await updateSystemSetting(key, { value });
      setSettings((current) => upsertByKey(current, saved));
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const saveRegistration = useCallback(async (payload: RegistrationConfig) => {
    setSaving(true);
    setError(null);
    try {
      const saved = await updateRegistrationConfig(payload);
      setRegistration(saved);
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const reloadMemoryCache = useCallback(async (): Promise<MemoryCacheReloadResult> => {
    setSaving(true);
    setError(null);
    try {
      const result = await reloadBackendMemoryCache();
      refresh();
      return result;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, [refresh]);

  const clearChatHallHistory = useCallback(async (): Promise<ClearRecordsResult> => {
    setSaving(true);
    setError(null);
    try {
      return await clearChatHallMessages();
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    admins,
    changeAdminStatus,
    changeUserStatus,
    clearChatHallHistory,
    error,
    loading,
    refresh,
    registration,
    removeUser,
    removeRole,
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
    users: userPage.items,
  };
}

const emptyUserPage: UserPage = {
  items: [],
  page: 1,
  pageSize: 20,
  totalCount: 0,
  totalPages: 0,
};

function replacePageUser(page: UserPage, user: AdminUserSummary): UserPage {
  return {
    ...page,
    items: page.items.some((current) => current.id === user.id)
      ? page.items.map((current) => (current.id === user.id ? user : current))
      : page.items,
  };
}

function removePageUser(page: UserPage, userId: string): UserPage {
  return {
    ...page,
    items: page.items.filter((user) => user.id !== userId),
    totalCount: Math.max(0, page.totalCount - 1),
  };
}

function upsertById<T extends { id: string }>(items: T[], item: T) {
  return items.some((current) => current.id === item.id)
    ? items.map((current) => (current.id === item.id ? item : current))
    : [...items, item];
}

function upsertByKey<T extends { key: string }>(items: T[], item: T) {
  return items.map((current) => (current.key === item.key ? item : current));
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
