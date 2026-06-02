import type {
  AdminRole,
  AdminSummary,
  PermissionScope,
  RegistrationConfig,
  SystemSetting,
  UserStatus,
  UserSummary,
} from './dashboard';

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

export interface UpdateSystemSettingRequest {
  value: string;
}
