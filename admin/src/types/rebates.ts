import type { InvitePolicySummary, RebateMode } from './dashboard';
import type { FinancePage, FinancePageQuery, LedgerEntry } from './finance';

export type { InvitePolicySummary, RebateMode };

export interface InvitePolicyUpdateRequest {
  agentsCanInvite: boolean;
  regularUsersCanInvite: boolean;
  rebateMode: RebateMode;
  defaultRechargeRebateBasisPoints: number;
}

export interface AgentRebateSummary {
  accountAvailableBalanceMinor: number;
  agentUserId: string;
  agentUsername: string;
  directInviteeCount: number;
  inviteCode: string;
  lastRebateAt: string | null;
  pendingRebateMinor: number;
  rebateRecordCount: number;
  totalRebateMinor: number;
  withdrawableRebateMinor: number;
  withdrawnRebateMinor: number;
}

export interface AgentRebateRecord {
  agentUserId: string;
  agentUsername: string;
  createdAt: string;
  inviteeUserId: string | null;
  inviteeUsername: string | null;
  ledgerEntryId: string;
  rebateAmountMinor: number;
  rechargeAmountMinor: number | null;
  rechargeOrderId: string | null;
}

export interface AgentRebateWithdrawalRequest {
  amountMinor: number;
  description: string;
}

export type AgentRebatePage = FinancePage<AgentRebateSummary>;
export type AgentRebateRecordPage = FinancePage<AgentRebateRecord>;
export type AgentRebateQuery = Pick<FinancePageQuery, 'page' | 'pageSize'>;
export type AgentRebateWithdrawalResult = LedgerEntry;
