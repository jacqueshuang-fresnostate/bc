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
  directInviteeRechargeMinor: number;
  directInviteeWithdrawalMinor: number;
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
  inviteeTotalRechargeMinor: number;
  inviteeTotalWithdrawalMinor: number;
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

export type AgentApplicationStatus = 'pending' | 'approved' | 'rejected';

export interface AgentApplication {
  id: string;
  userId: string;
  username: string;
  inviteCode: string;
  status: AgentApplicationStatus;
  reason: string;
  reviewNote: string | null;
  reviewedByAdminId: string | null;
  reviewedByAdminUsername: string | null;
  reviewedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ReviewAgentApplicationRequest {
  approved: boolean;
  note?: string | null;
}

export type AgentApplicationPage = FinancePage<AgentApplication>;
export type AgentApplicationQuery = Pick<FinancePageQuery, 'page' | 'pageSize'> & {
  status?: AgentApplicationStatus;
};
