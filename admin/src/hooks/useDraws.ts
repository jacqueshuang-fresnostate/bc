import { useCallback, useEffect, useState } from 'react';
import {
  cancelDrawIssue,
  clearApiDrawSourceSnapshotRecords,
  clearDrawnIssueRecords,
  createDrawSource,
  closeDrawIssue,
  createDrawIssue,
  deleteDrawSource,
  drawIssueResult,
  fetchApiDrawSourceSnapshots,
  fetchDrawIssues,
  fetchDrawSources,
  generateDrawIssueBatch,
  generateNextDrawIssue,
  previewDrawIssueGeneration,
  runDrawAutomation,
  updateDrawSource,
} from '../api/client';
import type { DrawSource, SaveDrawSourceRequest } from '../types/dashboard';
import type {
  ApiDrawSourceCrawlSnapshot,
  ApiDrawSourceCrawlSnapshotQuery,
  CreateDrawIssueRequest,
  DrawAutomationRunRequest,
  DrawIssue,
  DrawIssueQuery,
  DrawIssueGenerationPreview,
  DrawIssueResultRequest,
  GenerateDrawIssueRequest,
  GenerateDrawIssuesRequest,
} from '../types/draws';

export function useDraws() {
  const [drawSources, setDrawSources] = useState<DrawSource[]>([]);
  const [issues, setIssues] = useState<DrawIssue[]>([]);
  const [query, setQuery] = useState<DrawIssueQuery>({});
  const [issuePage, setIssuePage] = useState(1);
  const [pageSize, setPageSize] = useState(0);
  const [totalCount, setTotalCount] = useState(0);
  const [totalPages, setTotalPages] = useState(0);
  const [snapshots, setSnapshots] = useState<ApiDrawSourceCrawlSnapshot[]>([]);
  const [snapshotQuery, setSnapshotQuery] =
    useState<ApiDrawSourceCrawlSnapshotQuery>({ page: 1, pageSize: 20 });
  const [snapshotPage, setSnapshotPage] = useState(1);
  const [snapshotPageSize, setSnapshotPageSize] = useState(20);
  const [snapshotTotalCount, setSnapshotTotalCount] = useState(0);
  const [snapshotTotalPages, setSnapshotTotalPages] = useState(0);
  const [loading, setLoading] = useState(true);
  const [snapshotLoading, setSnapshotLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [snapshotError, setSnapshotError] = useState<string | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);
  const [snapshotRefreshToken, setSnapshotRefreshToken] = useState(0);

  const refresh = useCallback(() => {
    setRefreshToken((current) => current + 1);
    setSnapshotRefreshToken((current) => current + 1);
  }, []);

  const refreshWithFilter = useCallback((nextQuery?: DrawIssueQuery) => {
    setQuery((current) => ({ ...current, ...(nextQuery ?? {}) }));
    setRefreshToken((current) => current + 1);
  }, []);

  const refreshSnapshots = useCallback(() => {
    setSnapshotRefreshToken((current) => current + 1);
  }, []);

  const refreshSnapshotsWithFilter = useCallback(
    (nextQuery?: ApiDrawSourceCrawlSnapshotQuery) => {
      setSnapshotQuery((current) => ({ ...current, ...(nextQuery ?? {}) }));
      setSnapshotRefreshToken((current) => current + 1);
    },
    [],
  );

  const clearSnapshots = useCallback(async () => {
    setSaving(true);
    setSnapshotError(null);
    try {
      const result = await clearApiDrawSourceSnapshotRecords();
      setSnapshotQuery((current) => ({ ...current, page: 1 }));
      setSnapshotRefreshToken((current) => current + 1);
      return result;
    } catch (requestError) {
      setSnapshotError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const clearDrawnIssues = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      const result = await clearDrawnIssueRecords();
      setQuery((current) => ({ ...current, page: 1 }));
      setRefreshToken((current) => current + 1);
      return result;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  useEffect(() => {
    const controller = new AbortController();

    setLoading(true);
    setError(null);

    Promise.all([
      fetchDrawSources(controller.signal),
      fetchDrawIssues(controller.signal, query),
    ])
      .then(([sources, drawIssuePage]) => {
        setDrawSources(sources);
        setIssuePage(drawIssuePage.page);
        setPageSize(drawIssuePage.pageSize);
        setTotalCount(drawIssuePage.totalCount);
        setTotalPages(drawIssuePage.totalPages);
        setIssues(drawIssuePage.items);
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
  }, [query, refreshToken]);

  useEffect(() => {
    const controller = new AbortController();

    setSnapshotLoading(true);
    setSnapshotError(null);

    fetchApiDrawSourceSnapshots(controller.signal, snapshotQuery)
      .then((snapshotPage) => {
        setSnapshotPage(snapshotPage.page);
        setSnapshotPageSize(snapshotPage.pageSize);
        setSnapshotTotalCount(snapshotPage.totalCount);
        setSnapshotTotalPages(snapshotPage.totalPages);
        setSnapshots(snapshotPage.items);
      })
      .catch((requestError: unknown) => {
        if (!controller.signal.aborted) {
          setSnapshotError(errorMessage(requestError));
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setSnapshotLoading(false);
        }
      });

    return () => {
      controller.abort();
    };
  }, [snapshotQuery, snapshotRefreshToken]);

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

  const createSource = useCallback(async (payload: SaveDrawSourceRequest) => {
    setSaving(true);
    setError(null);
    try {
      const created = await createDrawSource(payload);
      setDrawSources((current) => [created, ...current]);
      return created;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  const updateSource = useCallback(
    async (id: string, payload: SaveDrawSourceRequest) => {
      setSaving(true);
      setError(null);
      try {
        const updated = await updateDrawSource(id, payload);
        setDrawSources((current) =>
          current.map((source) => (source.id === id ? updated : source)),
        );
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

  const deleteSource = useCallback(async (id: string) => {
    setSaving(true);
    setError(null);
    try {
      const deleted = await deleteDrawSource(id);
      setDrawSources((current) => current.filter((source) => source.id !== id));
      return deleted;
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

  const previewGeneration = useCallback(
    async (payload: GenerateDrawIssuesRequest): Promise<DrawIssueGenerationPreview[]> => {
      setSaving(true);
      setError(null);
      try {
        return await previewDrawIssueGeneration(payload);
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  const generateBatch = useCallback(async (payload: GenerateDrawIssuesRequest) => {
    setSaving(true);
    setError(null);
    try {
      const created = await generateDrawIssueBatch(payload);
      setIssues((current) => [...created, ...current]);
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

  const runAutomation = useCallback(
    async (payload: DrawAutomationRunRequest, issueQuery?: DrawIssueQuery) => {
      setSaving(true);
      setError(null);
      try {
        const run = await runDrawAutomation(payload);
        const latestIssues = await fetchDrawIssues(undefined, issueQuery ?? query);
        setIssuePage(latestIssues.page);
        setPageSize(latestIssues.pageSize);
        setTotalCount(latestIssues.totalCount);
        setTotalPages(latestIssues.totalPages);
        setIssues(latestIssues.items);
        return run;
      } catch (requestError) {
        setError(errorMessage(requestError));
        throw requestError;
      } finally {
        setSaving(false);
      }
    },
    [query],
  );

  return {
    cancel,
    clearDrawnIssues,
    clearSnapshots,
    close,
    create,
    createSource,
    deleteSource,
    draw,
    drawSources,
    error,
    generateBatch,
    generateNext,
    issues,
    loading,
    previewGeneration,
    refresh,
    refreshSnapshots,
    refreshSnapshotsWithFilter,
    refreshWithFilter,
    issuePage,
    pageSize,
    snapshotError,
    snapshotLoading,
    snapshotPage,
    snapshotPageSize,
    snapshotTotalCount,
    snapshotTotalPages,
    snapshots,
    totalCount,
    totalPages,
    runAutomation,
    saving,
    updateSource,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
