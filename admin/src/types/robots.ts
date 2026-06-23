import type {
  GroupBuyRobotFillStrategy,
  RobotConfigSummary,
  RobotKind,
  RobotStatus,
} from './dashboard';
import type { LedgerEntry } from './finance';
import type { GroupBuyPlan } from './groupBuy';
import type { OrderDetail } from './orders';

export type {
  GroupBuyRobotFillStrategy,
  RobotConfigSummary,
  RobotKind,
  RobotStatus,
};

export type RobotConfigPayload = Omit<RobotConfigSummary, 'deletable'>;

export interface RobotStatusUpdateRequest {
  status: RobotStatus;
}

export interface GroupBuyRobotSkippedItem {
  robotId: string;
  robotName: string;
  lotteryId: string;
  issue: string | null;
  reason: string;
}

export interface GroupBuyRobotRun {
  now: string;
  createdPlans: GroupBuyPlan[];
  filledPlans: GroupBuyPlan[];
  createdOrders: OrderDetail[];
  ledgerEntries: LedgerEntry[];
  skippedItems: GroupBuyRobotSkippedItem[];
}

export type RobotSchedulerRunStatus = 'success' | 'failed';
export type RobotSchedulerRunTrigger = 'automatic';

export interface RobotSchedulerConfig {
  enabled: boolean;
  intervalSeconds: number;
}

export interface RobotSchedulerRunRecord {
  id: string;
  trigger: RobotSchedulerRunTrigger;
  status: RobotSchedulerRunStatus;
  startedAt: string;
  finishedAt: string;
  now: string;
  error: string | null;
  createdPlanCount: number;
  filledPlanCount: number;
  createdOrderCount: number;
  ledgerEntryCount: number;
  skippedItemCount: number;
  skippedItems: GroupBuyRobotSkippedItem[];
}

export interface RobotSchedulerStatus {
  enabled: boolean;
  config: RobotSchedulerConfig;
  runCount: number;
  lastRun: RobotSchedulerRunRecord | null;
  recentRuns: RobotSchedulerRunRecord[];
}
