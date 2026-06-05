import type { LotteryNumberType } from './dashboard';
import type { FinancePage } from './finance';
import type { PlayRuleCode, PlaySelection } from './playRules';

export type OrderStatus = 'pendingDraw' | 'won' | 'lost' | 'cancelled';
export type OrderSource = 'direct' | 'groupBuy';

export interface CreateOrderRequest {
  issue: string;
  lotteryId: string;
  ruleCode: PlayRuleCode;
  selection: PlaySelection;
  unitAmountMinor: number;
  userId: string;
}

export interface OrderListQuery {
  includeRobotData?: boolean;
  page?: number;
  pageSize?: number;
}

export type OrderPage = FinancePage<OrderDetail>;

export interface OrderDetail {
  amountMinor: number;
  createdAt: string;
  expandedBets: string[];
  id: string;
  issue: string;
  lotteryId: string;
  lotteryName: string;
  numberType: LotteryNumberType;
  ruleCode: PlayRuleCode;
  selection: PlaySelection;
  stakeCount: number;
  oddsBasisPoints: number;
  orderSource: OrderSource;
  drawNumber: string | null;
  matchedBets: string[];
  payoutMinor: number;
  status: OrderStatus;
  settledAt: string | null;
  unitAmountMinor: number;
  userId: string;
}
