export type TaskKind = "pickup" | "long_term";

export type TaskStatus = "pickup" | "backlog" | "active" | "completed" | "archived";

export type EndReason = "completed" | "stopped" | "switched" | "app_closed" | "discarded";

export type RecoveryAction = "resume" | "stop" | "discard";

export type CommandErrorCode = "validation" | "not_found" | "conflict" | "internal";

export interface CommandError {
  code: CommandErrorCode;
  message: string;
}

export interface Task {
  id: string;
  title: string;
  description: string;
  kind: TaskKind;
  status: TaskStatus;
  priority: number;
  pickupDate: string | null;
  nextAction: string | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  archivedAt: string | null;
}

export interface Session {
  id: string;
  taskId: string;
  startedAt: string;
  endedAt: string | null;
  durationSeconds: number | null;
  endReason: EndReason | null;
  progressNote: string | null;
  nextAction: string | null;
  lapDurationSeconds: number;
  recoveredFromCrash: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ActiveSession {
  session: Session;
  task: Task;
}

export interface TodayPayload {
  activeSession: ActiveSession | null;
  pickup: Task[];
  backlog: Task[];
  recentThreads: RecentThread[];
}

export interface RecentThread {
  task: Task;
  session: Session;
  lastWorkedAt: string;
  progressNote: string | null;
  nextAction: string | null;
  durationSeconds: number | null;
}

export interface FloatingWindowPosition {
  x: number;
  y: number;
}

export interface Settings {
  launchOnStartup: boolean;
  todayOnStartup: boolean;
  donutLapDurationSeconds: number;
  theme: string;
  floatingWindowAlwaysOnTop: boolean;
  floatingWindowPosition: FloatingWindowPosition;
  longTermStopRequirements: string;
  todayReturnBehavior: string;
}

export interface CreateTaskInput {
  title: string;
  description?: string;
  kind: TaskKind;
  status?: Extract<TaskStatus, "pickup" | "backlog">;
  priority?: number;
  pickupDate?: string;
  nextAction?: string;
  sortOrder?: number;
}

export type CreateTaskResult = Task;

export interface UpdateTaskInput {
  id: string;
  title?: string;
  description?: string;
  kind?: TaskKind;
  status?: TaskStatus;
  priority?: number;
  pickupDate?: string;
  nextAction?: string;
  sortOrder?: number;
}

export type UpdateTaskResult = Task;

export interface ArchiveTaskInput {
  taskId: string;
}

export type ArchiveTaskResult = Task;

export type ListTodayResult = TodayPayload;

export type ListBacklogResult = Task[];

export interface ListRecentThreadsInput {
  limit?: number;
}

export type ListRecentThreadsResult = RecentThread[];

export interface StartSessionInput {
  taskId: string;
  lapDurationSeconds?: number;
}

export type StartSessionResult = ActiveSession;

export type GetActiveSessionResult = ActiveSession | null;

export interface GetPendingSessionRecoveryResult {
  activeSession: ActiveSession | null;
}

export interface CompleteSessionInput {
  sessionId?: string;
  progressNote?: string;
  nextAction?: string;
  confirmLongTermCompletion?: boolean;
}

export type CompleteSessionResult = ActiveSession;

export interface StopSessionInput {
  sessionId?: string;
  progressNote?: string;
  nextAction?: string;
  destinationStatus?: Exclude<TaskStatus, "active">;
}

export type StopSessionResult = ActiveSession;

export interface SwitchTaskInput {
  taskId: string;
  progressNote?: string;
  nextAction?: string;
  destinationStatus?: Exclude<TaskStatus, "active">;
  lapDurationSeconds?: number;
}

export type SwitchTaskResult = ActiveSession;

export interface ResolveSessionRecoveryInput {
  action: RecoveryAction;
  sessionId?: string;
  progressNote?: string;
  nextAction?: string;
}

export interface ResolveSessionRecoveryResult {
  activeSession: ActiveSession | null;
  session: Session | null;
  task: Task | null;
}

export type GetSettingsResult = Settings;

export interface UpdateSettingsInput {
  launchOnStartup?: boolean;
  todayOnStartup?: boolean;
  donutLapDurationSeconds?: number;
  theme?: string;
  floatingWindowAlwaysOnTop?: boolean;
  floatingWindowPosition?: FloatingWindowPosition;
  longTermStopRequirements?: string;
  todayReturnBehavior?: string;
}

export type UpdateSettingsResult = Settings;

export interface SaveFloatingWindowPositionInput {
  position: FloatingWindowPosition;
}

export type SaveFloatingWindowPositionResult = FloatingWindowPosition;

export interface ExportDatabaseInput {
  fileName?: string;
}

export interface ExportDatabaseResult {
  path: string;
}

export interface OpenDataFolderResult {
  path: string;
}

export type OpenTodayWindowResult = null;
