import type { DrawMode, LotteryNumberType } from './dashboard';
import type { LedgerEntry } from './finance';
import type { SettlementRun } from './settlements';

export type DrawIssueStatus = 'open' | 'closed' | 'drawn' | 'cancelled';

export interface CreateDrawIssueRequest {
  lotteryId: string;
  issue: string;
  scheduledAt: string;
  saleClosedAt: string;
}

export interface GenerateDrawIssueRequest {
  lotteryId: string;
  now: string;
  saleCloseLeadSeconds?: number;
}

export interface DrawIssueResultRequest {
  drawNumber?: string;
}

export interface DrawIssue {
  id: string;
  lotteryId: string;
  lotteryName: string;
  issue: string;
  numberType: LotteryNumberType;
  drawMode: DrawMode;
  scheduledAt: string;
  saleClosedAt: string;
  status: DrawIssueStatus;
  drawNumber: string | null;
  drawnAt: string | null;
  createdAt: string;
}

export interface DrawAutomationRunRequest {
  now: string;
}

export interface DrawAutomationSkippedIssue {
  drawIssueId: string;
  lotteryId: string;
  issue: string;
  reason: string;
}

export interface DrawAutomationRun {
  now: string;
  closedIssues: DrawIssue[];
  drawnIssues: DrawIssue[];
  settlementRuns: SettlementRun[];
  ledgerEntries: LedgerEntry[];
  skippedIssues: DrawAutomationSkippedIssue[];
}
