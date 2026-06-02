export type { FinanceOverview, FinancialAccountSummary } from './dashboard';

export type LedgerEntryKind =
  | 'manualAdjustment'
  | 'orderDebit'
  | 'orderRefund'
  | 'payoutCredit';

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
