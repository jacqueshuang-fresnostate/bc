import { useCallback, useEffect, useState } from 'react';
import {
  createInvitation,
  fetchInvitations,
  fetchInvitePolicy,
  fetchUsers,
  updateInvitation,
} from '../api/client';
import type { InvitePolicySummary, UserSummary } from '../types/dashboard';
import type {
  CreateInviteRecordRequest,
  InviteRecord,
  UpdateInviteRecordRequest,
} from '../types/invitations';

export function useInvitations() {
  const [invitations, setInvitations] = useState<InviteRecord[]>([]);
  const [invitePolicy, setInvitePolicy] = useState<InvitePolicySummary | null>(null);
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
      fetchInvitations(controller.signal),
      fetchUsers(controller.signal),
      fetchInvitePolicy(controller.signal),
    ])
      .then(([nextInvitations, nextUsers, nextInvitePolicy]) => {
        setInvitations(nextInvitations);
        setUsers(nextUsers);
        setInvitePolicy(nextInvitePolicy);
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

  const create = useCallback(async (payload: CreateInviteRecordRequest) => {
    setSaving(true);
    setError(null);
    try {
      const created = await createInvitation(payload);
      setInvitations((current) => upsertById(current, created));
      return created;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const update = useCallback(
    async (id: string, payload: UpdateInviteRecordRequest) => {
      setSaving(true);
      setError(null);
      try {
        const updated = await updateInvitation(id, payload);
        setInvitations((current) => upsertById(current, updated));
        return updated;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  return {
    create,
    error,
    invitations,
    invitePolicy,
    loading,
    refresh,
    saving,
    update,
    users,
  };
}

function upsertById<T extends { id: string }>(items: T[], item: T) {
  return items.some((current) => current.id === item.id)
    ? items.map((current) => (current.id === item.id ? item : current))
    : [...items, item];
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
