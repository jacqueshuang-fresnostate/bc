import type { AdminRole, AdminSummary, PermissionKey, PermissionScope } from './dashboard';

export interface AdminLoginRequest {
  password: string;
  username: string;
}

export interface CurrentAdminProfile {
  admin: AdminSummary;
  permissions: PermissionKey[];
  role: AdminRole;
  scopes: PermissionScope[];
}

export interface AdminAuthSession extends CurrentAdminProfile {
  token: string;
}

export interface AdminLogoutResponse {
  loggedOut: boolean;
}
