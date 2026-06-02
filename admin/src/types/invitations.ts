export type InviteStatus = 'pending' | 'active' | 'disabled';

export interface InviteRecord {
  id: string;
  inviterUserId: string;
  inviterUsername: string;
  inviteeUserId: string;
  inviteeUsername: string;
  inviteCode: string;
  status: InviteStatus;
  rebateEnabled: boolean;
  note: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateInviteRecordRequest {
  id: string;
  inviterUserId: string;
  inviteeUserId: string;
  inviteCode: string;
  rebateEnabled: boolean;
  note: string;
}

export interface UpdateInviteRecordRequest {
  status: InviteStatus;
  rebateEnabled: boolean;
  note: string;
}
