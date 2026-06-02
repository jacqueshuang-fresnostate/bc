import { useCallback, useEffect, useState } from 'react';
import {
  fetchInvitePolicy,
  fetchRegistrationConfig,
  updateInvitePolicy,
} from '../api/client';
import type { RegistrationConfig } from '../types/dashboard';
import type {
  InvitePolicySummary,
  InvitePolicyUpdateRequest,
} from '../types/rebates';

export function useRebatePolicy() {
  const [policy, setPolicy] = useState<InvitePolicySummary | null>(null);
  const [registration, setRegistration] = useState<RegistrationConfig | null>(null);
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
      fetchInvitePolicy(controller.signal),
      fetchRegistrationConfig(controller.signal),
    ])
      .then(([nextPolicy, nextRegistration]) => {
        setPolicy(nextPolicy);
        setRegistration(nextRegistration);
      })
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

  const save = useCallback(async (payload: InvitePolicyUpdateRequest) => {
    setSaving(true);
    setError(null);
    try {
      const saved = await updateInvitePolicy(payload);
      setPolicy(saved);
      return saved;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    error,
    loading,
    policy,
    refresh,
    registration,
    save,
    saving,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
