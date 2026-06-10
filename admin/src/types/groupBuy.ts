export type GroupBuyPlanStatus =
  | 'draft'
  | 'open'
  | 'filled'
  | 'cancelled'
  | 'settled';

export interface GroupBuyParticipant {
  id: string;
  userId: string;
  username: string;
  amountMinor: number;
  shareCount: number;
  note: string;
  createdAt: string;
}

export interface GroupBuyPlanSummary {
  id: string;
  lotteryId: string;
  lotteryName: string;
  orderId?: string | null;
  issue: string;
  ruleCode: string;
  title: string;
  initiatorUserId: string;
  initiatorUsername: string;
  totalAmountMinor: number;
  filledAmountMinor: number;
  shareCount: number;
  status: GroupBuyPlanStatus;
  createdAt: string;
}

export interface GroupBuyPlan extends GroupBuyPlanSummary {
  minShareAmountMinor: number;
  participantMinAmountMinor: number;
  numbers: string;
  participants: GroupBuyParticipant[];
  note: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateGroupBuyPlanRequest {
  id: string;
  lotteryId: string;
  issue: string;
  ruleCode: string;
  title: string;
  numbers: string;
  initiatorUserId: string;
  totalAmountMinor: number;
  initiatorAmountMinor: number;
  note: string;
}

export interface UpdateGroupBuyPlanRequest {
  status: GroupBuyPlanStatus;
  note: string;
}

export interface AddGroupBuyParticipantRequest {
  id: string;
  userId: string;
  amountMinor: number;
  note: string;
}
