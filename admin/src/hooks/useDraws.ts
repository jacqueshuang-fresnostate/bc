import { useCallback, useEffect, useState } from 'react';
import {
  cancelDrawIssue,
  closeDrawIssue,
  createDrawIssue,
  drawIssueResult,
  fetchDrawIssues,
  fetchDrawSources,
  generateNextDrawIssue,
  runDrawAutomation,
} from '../api/client';
import type { DrawSource } from '../types/dashboard';
import type {
  CreateDrawIssueRequest,
  DrawAutomationRunRequest,
  DrawIssue,
  DrawIssueResultRequest,
  GenerateDrawIssueRequest,
} from '../types/draws';

export function useDraws() {
  const [drawSources, setDrawSources] = useState<DrawSource[]>([]);
  const [issues, setIssues] = useState<DrawIssue[]>([]);
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
      fetchDrawSources(controller.signal),
      fetchDrawIssues(controller.signal),
    ])
      .then(([sources, drawIssues]) => {
        setDrawSources(sources);
        setIssues(drawIssues);
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

  const create = useCallback(async (payload: CreateDrawIssueRequest) => {
    setSaving(true);
    setError(null);
    try {
      const created = await createDrawIssue(payload);
      setIssues((current) => [created, ...current]);
      return created;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const generateNext = useCallback(async (payload: GenerateDrawIssueRequest) => {
    setSaving(true);
    setError(null);
    try {
      const created = await generateNextDrawIssue(payload);
      setIssues((current) => [created, ...current]);
      return created;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const close = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const closed = await closeDrawIssue(id);
      setIssues((current) =>
        current.map((issue) => (issue.id === id ? closed : issue)),
      );
      return closed;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const draw = useCallback(async (id: string, payload: DrawIssueResultRequest) => {
    setSaving(true);
    setError(null);
    try {
      const drawn = await drawIssueResult(id, payload);
      setIssues((current) =>
        current.map((issue) => (issue.id === id ? drawn : issue)),
      );
      return drawn;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const cancel = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const cancelled = await cancelDrawIssue(id);
      setIssues((current) =>
        current.map((issue) => (issue.id === id ? cancelled : issue)),
      );
      return cancelled;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const runAutomation = useCallback(async (payload: DrawAutomationRunRequest) => {
    setSaving(true);
    setError(null);
    try {
      const run = await runDrawAutomation(payload);
      const latestIssues = await fetchDrawIssues();
      setIssues(latestIssues);
      return run;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    cancel,
    close,
    create,
    draw,
    drawSources,
    error,
    generateNext,
    issues,
    loading,
    refresh,
    runAutomation,
    saving,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
