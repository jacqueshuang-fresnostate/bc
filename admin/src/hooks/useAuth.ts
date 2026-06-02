import { useCallback, useEffect, useState } from 'react';
import {
  clearStoredAuthToken,
  fetchCurrentAdmin,
  getStoredAuthToken,
  loginAdmin,
  logoutAdmin,
  setStoredAuthToken,
} from '../api/client';
import type { AdminAuthSession, AdminLoginRequest } from '../types/auth';

export function useAuth() {
  const [session, setSession] = useState<AdminAuthSession | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const token = getStoredAuthToken();
    if (!token) {
      setLoading(false);
      return undefined;
    }

    const controller = new AbortController();
    setLoading(true);
    setError(null);

    fetchCurrentAdmin(controller.signal)
      .then((profile) => {
        setSession({ ...profile, token });
      })
      .catch((requestError: unknown) => {
        if (!controller.signal.aborted) {
          clearStoredAuthToken();
          setSession(null);
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
  }, []);

  const login = useCallback(async (payload: AdminLoginRequest) => {
    setSaving(true);
    setError(null);
    try {
      const nextSession = await loginAdmin(payload);
      setStoredAuthToken(nextSession.token);
      setSession(nextSession);
      return nextSession;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const logout = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      await logoutAdmin();
    } catch (requestError) {
      setError(errorMessage(requestError));
    } finally {
      clearStoredAuthToken();
      setSession(null);
      setSaving(false);
    }
  }, []);

  return {
    error,
    loading,
    login,
    logout,
    saving,
    session,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
