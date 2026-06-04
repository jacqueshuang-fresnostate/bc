export type { FinanceOverview, FinancialAccountSummary } from './dashboard';

export type LedgerEntryKind =
  | 'manualAdjustment'
  | 'orderDebit'
  | 'orderRefund'
  | 'payoutCredit'
  | 'rechargeCredit';

export type RechargeChannel = 'rainbowEpay' | 'customerService';

export type RechargeOrderStatus =
  | 'pending'
  | 'waitingCustomerService'
  | 'paid'
  | 'cancelled';

export interface LedgerEntry {
  id: string;
  userId: string;
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
  createdAt: string;
  paidAt: string | null;
}
