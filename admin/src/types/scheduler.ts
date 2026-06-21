import type { DrawAutomationSkippedIssue } from './draws';

export type DrawSchedulerRunStatus = 'success' | 'failed';
export type DrawSchedulerRunTrigger = 'automatic';

export interface DrawSchedulerConfig {
  enabled: boolean;
  intervalSeconds: number;
  futureIssueCount: number;
  saleCloseLeadSeconds: number;
  localIssueGenerationConcurrency: number;
  apiIssueGenerationConcurrency: number;
}

export interface DrawSchedulerRunRecord {
  id: string;
  trigger: DrawSchedulerRunTrigger;
  status: DrawSchedulerRunStatus;
  startedAt: string;
  finishedAt: string;
  now: string;
  error: string | null;
  closedIssueCount: number;
  drawnIssueCount: number;
  settlementRunCount: number;
  ledgerEntryCount: number;
  generatedIssueCount: number;
  skippedIssueCount: number;
  skippedLotteryCount: number;
  skippedIssues: DrawAutomationSkippedIssue[];
  skippedLotteries: DrawSchedulerSkippedLottery[];
}

export interface DrawSchedulerSkippedLottery {
  lotteryId: string;
  reason: string;
}

export interface DrawSchedulerStatus {
  enabled: boolean;
  config: DrawSchedulerConfig;
  runCount: number;
  lastRun: DrawSchedulerRunRecord | null;
  recentRuns: DrawSchedulerRunRecord[];
}
