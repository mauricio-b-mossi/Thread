import { invoke } from "@tauri-apps/api/core";
import type {
  ArchiveTaskInput,
  ArchiveTaskResult,
  CompleteSessionInput,
  CompleteSessionResult,
  CreateTaskInput,
  CreateTaskResult,
  ExportDatabaseInput,
  ExportDatabaseResult,
  GetActiveSessionResult,
  GetSettingsResult,
  ListBacklogResult,
  ListRecentThreadsInput,
  ListRecentThreadsResult,
  ListTodayResult,
  OpenDataFolderResult,
  ResolveSessionRecoveryInput,
  ResolveSessionRecoveryResult,
  SaveFloatingWindowPositionInput,
  SaveFloatingWindowPositionResult,
  StartSessionInput,
  StartSessionResult,
  StopSessionInput,
  StopSessionResult,
  SwitchTaskInput,
  SwitchTaskResult,
  UpdateSettingsInput,
  UpdateSettingsResult,
  UpdateTaskInput,
  UpdateTaskResult
} from "./types";

export const createTask = (input: CreateTaskInput): Promise<CreateTaskResult> =>
  invoke("createTask", { input });

export const updateTask = (input: UpdateTaskInput): Promise<UpdateTaskResult> =>
  invoke("updateTask", { input });

export const archiveTask = (input: ArchiveTaskInput): Promise<ArchiveTaskResult> =>
  invoke("archiveTask", { input });

export const listToday = (): Promise<ListTodayResult> => invoke("listToday");

export const listBacklog = (): Promise<ListBacklogResult> => invoke("listBacklog");

export const listRecentThreads = (
  input?: ListRecentThreadsInput
): Promise<ListRecentThreadsResult> => invoke("listRecentThreads", { input: input ?? null });

export const startSession = (input: StartSessionInput): Promise<StartSessionResult> =>
  invoke("startSession", { input });

export const getActiveSession = (): Promise<GetActiveSessionResult> =>
  invoke("getActiveSession");

export const completeSession = (
  input: CompleteSessionInput = {}
): Promise<CompleteSessionResult> => invoke("completeSession", { input });

export const stopSession = (input: StopSessionInput = {}): Promise<StopSessionResult> =>
  invoke("stopSession", { input });

export const switchTask = (input: SwitchTaskInput): Promise<SwitchTaskResult> =>
  invoke("switchTask", { input });

export const resolveSessionRecovery = (
  input: ResolveSessionRecoveryInput
): Promise<ResolveSessionRecoveryResult> => invoke("resolveSessionRecovery", { input });

export const getSettings = (): Promise<GetSettingsResult> => invoke("getSettings");

export const updateSettings = (input: UpdateSettingsInput): Promise<UpdateSettingsResult> =>
  invoke("updateSettings", { input });

export const saveFloatingWindowPosition = (
  input: SaveFloatingWindowPositionInput
): Promise<SaveFloatingWindowPositionResult> =>
  invoke("saveFloatingWindowPosition", { input });

export const exportDatabase = (
  input?: ExportDatabaseInput
): Promise<ExportDatabaseResult> => invoke("exportDatabase", { input: input ?? null });

export const openDataFolder = (): Promise<OpenDataFolderResult> =>
  invoke("openDataFolder");
