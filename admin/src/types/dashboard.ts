import type { PlayRuleCode } from './playRules';

export interface ApiEnvelope<T> {
  success: boolean;
  data: T | null;
  message: string;
}

export type ModuleStatus = 'scaffolded' | 'planned';

export interface Metric {
  key: string;
  label: string;
  value: string;
  trend: string;
}

export interface AdminModule {
  key: string;
  name: string;
  description: string;
  status: ModuleStatus;
}

export interface ModuleGroup {
  key: string;
  title: string;
  description: string;
  modules: AdminModule[];
}

export type LotteryNumberType = 'threeDigit' | 'fiveDigit';
export type DrawMode = 'platform' | 'api' | 'manual';

export type DrawSchedule =
  | { periodic: { intervalSeconds: number } }
  | { daily: { time: string } }
  | { weekly: { weekdays: string[]; time: string } };

export type PlayCategory =
  | 'direct'
  | 'groupThree'
  | 'groupSix'
  | 'directCombination'
  | 'bigSmallOddEven';

export interface GroupBuyConfig {
  enabled: boolean;
  minShareAmountMinor: number;
  initiatorMinPercent: number;
  participantMinAmountMinor: number;
}

export interface LotteryPlayConfig {
  enabled: boolean;
  oddsBasisPoints: number;
  ruleCode: PlayRuleCode;
}

export interface LotteryKind {
  id: string;
  name: string;
  numberType: LotteryNumberType;
  drawMode: DrawMode;
  schedule: DrawSchedule;
  saleEnabled: boolean;
  groupBuy: GroupBuyConfig;
  playCategories: PlayCategory[];
  playConfigs: LotteryPlayConfig[];
}

export interface DrawSource {
  id: string;
  name: string;
  mode: DrawMode;
  reusableForLotteryIds: string[];
}

export interface OrderSummary {
  createdAt: string;
  id: string;
  userId: string;
  lotteryId: string;
  lotteryName: string;
  issue: string;
  ruleCode: string;
  stakeCount: number;
  amountMinor: number;
  oddsBasisPoints: number;
  drawNumber: string | null;
  matchedBets: string[];
  payoutMinor: number;
  status: 'pendingDraw' | 'won' | 'lost' | 'cancelled';
  settledAt: string | null;
}

export interface GroupBuyPlanSummary {
  id: string;
  lotteryId: string;
  initiatorUserId: string;
  totalAmountMinor: number;
  filledAmountMinor: number;
  shareCount: number;
  status: string;
}

export interface FinanceOverview {
  totalBalanceMinor: number;
  pendingWithdrawMinor: number;
  todayRechargeMinor: number;
  todayPayoutMinor: number;
}

export type RobotKind = 'groupBuy' | 'purchase';
export type RobotStatus = 'enabled' | 'paused' | 'disabled';

export interface RobotConfigSummary {
  id: string;
  name: string;
  kind: RobotKind;
  lotteryIds: string[];
  status: RobotStatus;
  description: string;
}

export interface UserSummary {
  id: string;
  username: string;
  email: string | null;
  kind: UserKind;
  status: UserStatus;
  balanceMinor: number;
  agentId: string | null;
}

export type UserStatus = 'active' | 'suspended' | 'locked';
export type UserKind = 'regular' | 'agent';

export interface AdminSummary {
  id: string;
  username: string;
  roleId: string;
  roleName: string;
  status: UserStatus;
}

export type PermissionScope =
  | 'users'
  | 'orders'
  | 'finance'
  | 'customerService'
  | 'admins'
  | 'roles'
  | 'systemSettings'
  | 'lotteries'
  | 'robots'
  | 'rebates';

export interface AdminRole {
  id: string;
  name: string;
  scopes: PermissionScope[];
}

export interface SystemSetting {
  key: string;
  value: string;
  description: string;
}

export interface RegistrationConfig {
  usernameEnabled: boolean;
  emailEnabled: boolean;
  agentInviteRequired: boolean;
}

export type RebateMode = 'immediate' | 'rechargeTiered';

export interface InvitePolicySummary {
  agentsCanInvite: boolean;
  regularUsersCanInvite: boolean;
  rebateMode: RebateMode;
  supportedRebateModes: RebateMode[];
  defaultRechargeRebateBasisPoints: number;
}

export interface FinancialAccountSummary {
  userId: string;
  availableBalanceMinor: number;
  frozenBalanceMinor: number;
}

export interface DashboardSummary {
  metrics: Metric[];
  moduleGroups: ModuleGroup[];
  lotteries: LotteryKind[];
  drawSources: DrawSource[];
  recentOrders: OrderSummary[];
  groupBuyPlans: GroupBuyPlanSummary[];
  finance: FinanceOverview;
  financialAccounts: FinancialAccountSummary[];
  robots: RobotConfigSummary[];
  users: UserSummary[];
  admins: AdminSummary[];
  roles: AdminRole[];
  settings: SystemSetting[];
  registration: RegistrationConfig;
  invitePolicy: InvitePolicySummary;
}
