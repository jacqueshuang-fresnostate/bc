import type { DrawMode, DrawSourceProvider, LotteryNumberType } from './dashboard';
import type { LedgerEntry } from './finance';
import type { SettlementRun } from './settlements';

export type DrawIssueStatus = 'open' | 'closed' | 'drawn' | 'cancelled';

export interface CreateDrawIssueRequest {
  lotteryId: string;
  issue: string;
  scheduledAt: string;
  saleClosedAt: string;
}

export interface DrawIssueQuery {
  lotteryId?: string;
  status?: DrawIssueStatus;
  page?: number;
  pageSize?: number;
}

export interface GenerateDrawIssueRequest {
  lotteryId: string;
  now: string;
  saleCloseLeadSeconds?: number;
}

export interface GenerateDrawIssuesRequest {
  lotteryId: string;
  now: string;
  count: number;
  saleCloseLeadSeconds?: number;
}

export interface DrawIssueGenerationPreview {
  lotteryId: string;
  lotteryName: string;
  issue: string;
  numberType: LotteryNumberType;
  drawMode: DrawMode;
  scheduledAt: string;
  saleClosedAt: string;
}

export interface ApiDrawSourceIssueSnapshot {
  latestIssue: string;
  latestDrawTime: string | null;
  nextIssue: string | null;
  nextDrawTime: string | null;
}

export type ApiDrawSourceSnapshotRequestKind = 'latestIssue' | 'drawNumber';

export interface ApiDrawSourceCrawlSnapshot {
  id: string;
  sourceId: string;
  sourceName: string;
  provider: DrawSourceProvider | string;
  lotteryId: string;
  requestKind: ApiDrawSourceSnapshotRequestKind | string;
  requestedIssue: string | null;
  latestIssue: string | null;
  latestDrawTime: string | null;
  nextIssue: string | null;
  nextDrawTime: string | null;
  drawNumber: string | null;
  endpoint: string;
  lotCode: string;
  httpStatus: number | null;
  success: boolean;
  errorMessage: string | null;
  rawResponse: unknown | null;
  rawResponseText: string;
  crawledAt: string;
}

export interface ApiDrawSourceCrawlSnapshotPage {
  items: ApiDrawSourceCrawlSnapshot[];
  totalCount: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface ApiDrawSourceCrawlSnapshotQuery {
  lotteryId?: string;
  sourceId?: string;
  requestKind?: ApiDrawSourceSnapshotRequestKind;
  success?: boolean;
  issue?: string;
  page?: number;
  pageSize?: number;
}

export interface DrawSourceSyncResult {
  lotteryId: string;
  lotteryName: string;
  apiSnapshot: ApiDrawSourceIssueSnapshot;
  targetIssue: DrawIssue;
  generatedIssues: DrawIssue[];
  updatedIssues: DrawIssue[];
  cancelledIssues: DrawIssue[];
  keptIssues: DrawIssue[];
  message: string;
}

export interface DrawIssueResultRequest {
  drawNumber?: string;
}

export type DrawControlTargetScope = 'lottery' | 'issue' | 'order';

export interface SaveLotteryDrawControlRequest {
  enabled: boolean;
  drawNumber?: string | null;
  targetScope?: DrawControlTargetScope;
  targetIssue?: string | null;
  targetOrderId?: string | null;
}

export interface LotteryDrawControl {
  lotteryId: string;
  lotteryName: string;
  numberType: LotteryNumberType;
  enabled: boolean;
  drawNumber: string | null;
  targetScope: DrawControlTargetScope;
  targetIssue: string | null;
  targetOrderId: string | null;
  updatedAt: string | null;
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

export interface DrawIssuePage {
  items: DrawIssue[];
  totalCount: number;
  page: number;
  pageSize: number;
  totalPages: number;
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
