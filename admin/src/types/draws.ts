import type { DrawMode, LotteryNumberType } from './dashboard';

export type DrawIssueStatus = 'open' | 'closed' | 'drawn' | 'cancelled';

export interface CreateDrawIssueRequest {
  lotteryId: string;
  issue: string;
  scheduledAt: string;
  saleClosedAt: string;
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
