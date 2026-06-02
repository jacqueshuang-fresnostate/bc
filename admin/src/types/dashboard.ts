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

export interface LotteryKind {
  id: string;
  name: string;
  numberType: LotteryNumberType;
  drawMode: DrawMode;
  schedule: DrawSchedule;
  saleEnabled: boolean;
  groupBuy: GroupBuyConfig;
  playCategories: PlayCategory[];
}

export interface DrawSource {
  id: string;
  name: string;
  mode: DrawMode;
  reusableForLotteryIds: string[];
}

export interface OrderSummary {
  id: string;
  userId: string;
  lotteryId: string;
  issue: string;
  amountMinor: number;
  status: 'pendingDraw' | 'won' | 'lost' | 'cancelled';
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

export interface RobotConfigSummary {
  id: string;
  name: string;
  kind: 'groupBuy' | 'purchase';
  lotteryIds: string[];
  status: 'enabled' | 'paused' | 'disabled';
  description: string;
}

export interface UserSummary {
  id: string;
  username: string;
  email: string | null;
  kind: 'regular' | 'agent';
  status: 'active' | 'suspended' | 'locked';
  balanceMinor: number;
  agentId: string | null;
}

export interface AdminSummary {
  id: string;
  username: string;
  roleName: string;
  status: 'active' | 'suspended' | 'locked';
}

export interface AdminRole {
  id: string;
  name: string;
  scopes: string[];
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

export interface InvitePolicySummary {
  agentsCanInvite: boolean;
  regularUsersCanInvite: boolean;
  rebateMode: 'immediate' | 'rechargeTiered';
  supportedRebateModes: Array<'immediate' | 'rechargeTiered'>;
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
