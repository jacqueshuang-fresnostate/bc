import type { PlayRuleCode } from './playRules';
import type { GroupBuyPlanSummary } from './groupBuy';

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

export type LotteryNumberType =
  | 'threeDigit'
  | 'fiveDigit'
  | 'pk10'
  | 'elevenFive'
  | 'fastThree'
  | 'luckTwenty';
export type DrawMode = 'platform' | 'api' | 'manual';
export type DrawSourceProvider =
  | 'api68'
  | 'kjApi'
  | 'bbKaijiang'
  | 'indonesiaLottery';
export type LotteryCategory = string;

export interface LotteryCategoryConfig {
  code: string;
  name: string;
}

export type DrawSchedule =
  | { periodic: { intervalSeconds: number } }
  | { timeNode: { intervalSeconds: number; startTime: string } }
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

export interface LotteryPlayPositionSelectLimit {
  positionKey: string;
  maxSelectCount: number;
}

export interface LotteryPlayConfig {
  enabled: boolean;
  oddsBasisPoints: number;
  positionSelectLimits?: LotteryPlayPositionSelectLimit[];
  ruleCode: PlayRuleCode;
}

export interface LotteryKind {
  id: string;
  name: string;
  category: LotteryCategory;
  logoUrl: string;
  numberType: LotteryNumberType;
  drawMode: DrawMode;
  apiDrawDelaySeconds: number;
  drawControlEnabled: boolean;
  issueFormat: string;
  saleCloseLeadSeconds: number;
  schedule: DrawSchedule;
  saleEnabled: boolean;
  groupBuy: GroupBuyConfig;
  playCategories: PlayCategory[];
  playConfigs: LotteryPlayConfig[];
}

export interface DrawSource {
  editable: boolean;
  endpoint: string | null;
  id: string;
  lotCode: string | null;
  name: string;
  mode: DrawMode;
  provider: DrawSourceProvider | null;
  reusableForLotteryIds: string[];
}

export interface SaveDrawSourceRequest {
  endpoint?: string | null;
  id: string;
  lotCode: string;
  name: string;
  provider: DrawSourceProvider;
  reusableForLotteryIds: string[];
}

export interface OrderSummary {
  createdAt: string;
  id: string;
  orderSource: 'direct' | 'groupBuy';
  userId: string;
  username?: string | null;
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

export interface FinanceOverview {
  totalBalanceMinor: number;
  pendingWithdrawMinor: number;
  todayRechargeMinor: number;
  todayPayoutMinor: number;
}

export type RobotKind = 'groupBuy' | 'purchase';
export type RobotStatus = 'enabled' | 'paused' | 'disabled';
export type GroupBuyRobotFillStrategy = 'rhythm' | 'beforeDraw';

export interface RobotConfigSummary {
  id: string;
  name: string;
  kind: RobotKind;
  lotteryIds: string[];
  status: RobotStatus;
  description: string;
  groupBuyFillStrategy: GroupBuyRobotFillStrategy;
  groupBuyFillBeforeDrawSeconds: number;
  deletable: boolean;
}

export interface UserSummary {
  id: string;
  username: string;
  email: string | null;
  avatarUrl?: string;
  contactQq: string;
  kind: UserKind;
  status: UserStatus;
  balanceMinor: number;
  agentId: string | null;
  inviteCode: string;
  registrationLocation?: UserRegistrationLocation;
}

export interface UserRegistrationLocation {
  registeredIp: string;
  country: string;
  region: string;
  city: string;
  source: 'client' | 'gps' | 'ip' | 'unknown' | string;
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

export type PermissionKey = string;

export interface AdminRole {
  id: string;
  name: string;
  permissions?: PermissionKey[];
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
