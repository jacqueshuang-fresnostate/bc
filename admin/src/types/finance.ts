export type { FinanceOverview } from './dashboard';

export type LedgerEntryKind =
  | 'agentRebateWithdrawal'
  | 'manualAdjustment'
  | 'groupBuyDebit'
  | 'groupBuyRefund'
  | 'orderDebit'
  | 'orderRefund'
  | 'payoutCredit'
  | 'rechargeBonusCredit'
  | 'rechargeCredit'
  | 'rechargeRebateCredit'
  | 'redPacketCredit'
  | 'redPacketDebit'
  | 'withdrawalFreeze'
  | 'withdrawalPayout'
  | 'withdrawalReject';

export type RechargeChannel = 'rainbowEpay' | 'customerService';

export type RechargeOrderStatus =
  | 'pending'
  | 'waitingCustomerService'
  | 'paid'
  | 'cancelled';

export type WithdrawalOrderStatus =
  | 'pending'
  | 'approved'
  | 'rejected'
  | 'cancelled';

export type WithdrawalMethodType = 'alipay' | 'wechat' | 'bankCard';

export interface FinancePage<T> {
  items: T[];
  totalCount: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface FinancePageQuery {
  page?: number;
  pageSize?: number;
  includeRobotData?: boolean;
  userId?: string;
  username?: string;
}

export interface ClearRecordsResult {
  deletedCount: number;
}

export interface ClearRobotGroupBuyRecordsResult extends ClearRecordsResult {
  deletedOrderCount: number;
}

export interface AdminFinancialAccountSummary {
  userId: string;
  username: string | null;
  agentId?: string | null;
  agentUsername?: string | null;
  availableBalanceMinor: number;
  frozenBalanceMinor: number;
}

export interface LedgerEntry {
  id: string;
  userId: string;
  username?: string | null;
  kind: LedgerEntryKind;
  amountMinor: number;
  balanceAfterMinor: number;
  referenceId: string | null;
  description: string;
  createdAt: string;
}

export interface ManualBalanceAdjustmentRequest {
  userId: string;
  amountMinor: number;
  description: string;
}

export interface ConfirmRechargeOrderRequest {
  providerTradeNo?: string | null;
  remark?: string | null;
}

export interface RechargeOrderSummary {
  id: string;
  userId: string;
  username: string;
  channel: RechargeChannel;
  amountMinor: number;
  status: RechargeOrderStatus;
  payType: string | null;
  providerTradeNo: string | null;
  paymentUrl: string | null;
  supportConversationId: string | null;
  remark: string;
  createdAt: string;
  paidAt: string | null;
}

export interface AdminRechargeOrderSummary extends RechargeOrderSummary {
  agentId?: string | null;
  agentUsername?: string | null;
}

export interface WithdrawalOrderSummary {
  id: string;
  userId: string;
  username: string;
  methodId: string;
  methodType: WithdrawalMethodType;
  accountHolder: string;
  accountNumber: string;
  bankName: string | null;
  amountMinor: number;
  status: WithdrawalOrderStatus;
  createdAt: string;
  reviewedAt: string | null;
}

export interface AdminWithdrawalOrderSummary extends WithdrawalOrderSummary {
  agentId?: string | null;
  agentUsername?: string | null;
}
