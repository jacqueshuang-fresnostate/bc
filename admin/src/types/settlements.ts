import type { FinancePage } from './finance';
import type { OrderStatus } from './orders';
import type { PlayRuleCode } from './playRules';

export interface OrderSettlement {
  orderId: string;
  userId: string;
  username?: string | null;
  ruleCode: PlayRuleCode;
  stakeCount: number;
  amountMinor: number;
  isWinning: boolean;
  matchedBets: string[];
  oddsBasisPoints: number;
  payoutMinor: number;
  status: OrderStatus;
}

export interface SettlementRun {
  id: string;
  drawIssueId: string;
  lotteryId: string;
  lotteryName: string;
  issue: string;
  drawNumber: string;
  settledOrderCount: number;
  winningOrderCount: number;
  totalStakeAmountMinor: number;
  totalPayoutMinor: number;
  createdAt: string;
  orders: OrderSettlement[];
}

export type SettlementPage = FinancePage<SettlementRun>;
