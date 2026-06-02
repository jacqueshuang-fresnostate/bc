import { useCallback, useEffect, useState } from 'react';
import { evaluatePlayRule, fetchPlayRules } from '../api/client';
import type {
  PlayRuleEvaluateRequest,
  PlayRuleEvaluation,
  PlayRuleSummary,
} from '../types/playRules';

export function usePlayRules() {
  const [rules, setRules] = useState<PlayRuleSummary[]>([]);
  const [evaluation, setEvaluation] = useState<PlayRuleEvaluation | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const controller = new AbortController();

    setLoading(true);
    setError(null);

    fetchPlayRules(controller.signal)
      .then(setRules)
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
  }, []);

  const evaluate = useCallback(async (payload: PlayRuleEvaluateRequest) => {
    setSaving(true);
    setError(null);
    try {
      const result = await evaluatePlayRule(payload);
      setEvaluation(result);
      return result;
    } catch (requestError) {
      setError(errorMessage(requestError));
      throw requestError;
    } finally {
      setSaving(false);
    }
  }, []);

  return {
    error,
    evaluate,
    evaluation,
    loading,
    rules,
    saving,
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : '接口请求失败';
}
