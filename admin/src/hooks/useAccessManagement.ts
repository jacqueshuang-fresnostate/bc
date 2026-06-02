import { useCallback, useEffect, useState } from 'react';
import {
  createAdmin,
  createRole,
  createUser,
  deleteRole,
  fetchAdmins,
  fetchRegistrationConfig,
  fetchRoles,
  fetchSystemSettings,
  fetchUsers,
  setAdminStatus,
  setUserStatus,
  updateAdmin,
  updateRegistrationConfig,
  updateRole,
  updateSystemSetting,
  updateUser,
} from '../api/client';
import type {
  AdminRole,
  AdminSummary,
  RegistrationConfig,
  SystemSetting,
  UserStatus,
  UserSummary,
} from '../types/access';

export function useAccessManagement() {
  const [admins, setAdmins] = useState<AdminSummary[]>([]);
  const [registration, setRegistration] = useState<RegistrationConfig | null>(null);
  const [roles, setRoles] = useState<AdminRole[]>([]);
  const [settings, setSettings] = useState<SystemSetting[]>([]);
  const [users, setUsers] = useState<UserSummary[]>([]);
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
      fetchUsers(controller.signal),
      fetchAdmins(controller.signal),
      fetchRoles(controller.signal),
      fetchSystemSettings(controller.signal),
      fetchRegistrationConfig(controller.signal),
    ])
      .then(
        ([
          nextUsers,
          nextAdmins,
          nextRoles,
          nextSettings,
          nextRegistration,
        ]) => {
          setUsers(nextUsers);
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
  }, [refreshToken]);

  const saveUser = useCallback(async (payload: UserSummary, existingId?: string) => {
    setSaving(true);
    setError(null);
    try {
      const saved = existingId
        ? await updateUser(existingId, payload)
        : await createUser(payload);
      setUsers((current) => upsertById(current, saved));
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const changeUserStatus = useCallback(async (id: string, status: UserStatus) => {
    setSaving(true);
    setError(null);
    try {
      const saved = await setUserStatus(id, { status });
      setUsers((current) => upsertById(current, saved));
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const saveAdmin = useCallback(
    async (payload: AdminSummary, existingId?: string) => {
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

  return {
    admins,
    changeAdminStatus,
    changeUserStatus,
    error,
    loading,
    refresh,
    registration,
    removeRole,
    roles,
    saveAdmin,
    saveRegistration,
    saveRole,
    saveSetting,
    saveUser,
    saving,
    settings,
    users,
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
