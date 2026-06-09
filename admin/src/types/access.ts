import type {
  AdminRole,
  AdminSummary,
  PermissionScope,
  RegistrationConfig,
  SystemSetting,
  UserStatus,
  UserSummary,
} from './dashboard';
import type { FinancePage } from './finance';

export type {
  AdminRole,
  AdminSummary,
  PermissionScope,
  RegistrationConfig,
  SystemSetting,
  UserStatus,
  UserSummary,
};

export interface StatusUpdateRequest {
  status: UserStatus;
}

export interface AdminSaveRequest extends AdminSummary {
  password?: string;
}

export interface AdminPasswordResetRequest {
  password: string;
}

export interface UpdateSystemSettingRequest {
  value: string;
}

export type UserListSortBy =
  | 'agentId'
  | 'balanceMinor'
  | 'email'
  | 'id'
  | 'inviteCode'
  | 'kind'
  | 'status'
  | 'username';

export type UserListSortDirection = 'asc' | 'desc';

export interface UserListQuery {
  page?: number;
  pageSize?: number;
  sortBy?: UserListSortBy;
  sortDirection?: UserListSortDirection;
}

export type UserPage = FinancePage<UserSummary>;
