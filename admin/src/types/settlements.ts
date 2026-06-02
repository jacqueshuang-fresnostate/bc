import type { OrderStatus } from './orders';
import type { PlayRuleCode } from './playRules';

export interface OrderSettlement {
  orderId: string;
  userId: string;
  ruleCode: PlayRuleCode;
  stakeCount: number;
  amountMinor: number;
  isWinning: boolean;
  matchedBets: string[];
  payoutMultiplier: number;
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
