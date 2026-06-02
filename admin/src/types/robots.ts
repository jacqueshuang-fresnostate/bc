import type { RobotConfigSummary, RobotKind, RobotStatus } from './dashboard';

export type { RobotConfigSummary, RobotKind, RobotStatus };

export interface RobotStatusUpdateRequest {
  status: RobotStatus;
}
