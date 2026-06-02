import type { InvitePolicySummary, RebateMode } from './dashboard';

export type { InvitePolicySummary, RebateMode };

export interface InvitePolicyUpdateRequest {
  agentsCanInvite: boolean;
  regularUsersCanInvite: boolean;
  rebateMode: RebateMode;
  defaultRechargeRebateBasisPoints: number;
}
