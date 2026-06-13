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

export interface UserPasswordResetRequest {
  password: string;
}

export interface UpdateSystemSettingRequest {
  value: string;
}

export interface AdminUserSummary extends UserSummary {
  agentUsername?: string | null;
}

export interface MemoryCacheReloadResult {
  reloadedModules: string[];
  databaseDirectModules: string[];
  skippedModules: string[];
  refreshedAt: string;
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
  status?: UserStatus;
}

export type UserPage = FinancePage<AdminUserSummary>;
