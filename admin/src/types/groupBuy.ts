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
  initiatorUserId: string;
  initiatorUsername: string;
  totalAmountMinor: number;
  filledAmountMinor: number;
  shareCount: number;
  status: GroupBuyPlanStatus;
}

export interface GroupBuyPlan extends GroupBuyPlanSummary {
  minShareAmountMinor: number;
  participantMinAmountMinor: number;
  participants: GroupBuyParticipant[];
  note: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateGroupBuyPlanRequest {
  id: string;
  lotteryId: string;
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
