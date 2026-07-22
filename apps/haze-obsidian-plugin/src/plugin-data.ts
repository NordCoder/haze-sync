import {
  BaseRevisionState,
  createDefaultBaseRevisionState,
  mergeBaseRevisionState,
} from "./base-revision-store";
import {
  ConflictActionKeyState,
  createDefaultConflictActionKeyState,
  mergeConflictActionKeyState,
} from "./conflict-action-keys";
import { LocalSyncState, createDefaultLocalSyncState, mergeLocalSyncState } from "./pending-queue";
import { RemoteSyncState, createDefaultRemoteSyncState, mergeRemoteSyncState } from "./remote-sync-state";
import { PluginSettings, mergePluginSettings } from "./settings";
import {
  SyncRuntimeState,
  createDefaultSyncRuntimeState,
  mergeSyncRuntimeState,
} from "./sync-runtime-state";

export interface HazeSyncPluginData {
  settings: PluginSettings;
  localState: LocalSyncState;
  baseRevisionState: BaseRevisionState;
  remoteSyncState: RemoteSyncState;
  conflictActionKeyState: ConflictActionKeyState;
  syncRuntimeState: SyncRuntimeState;
}

export function parsePluginData(rawData: unknown): HazeSyncPluginData {
  if (
    isRecord(rawData) &&
    (
      "settings" in rawData ||
      "localState" in rawData ||
      "baseRevisionState" in rawData ||
      "remoteSyncState" in rawData ||
      "conflictActionKeyState" in rawData ||
      "syncRuntimeState" in rawData
    )
  ) {
    return {
      settings: mergePluginSettings(rawData.settings),
      localState: mergeLocalSyncState(rawData.localState),
      baseRevisionState: mergeBaseRevisionState(rawData.baseRevisionState),
      remoteSyncState: mergeRemoteSyncState(rawData.remoteSyncState),
      conflictActionKeyState: mergeConflictActionKeyState(rawData.conflictActionKeyState),
      syncRuntimeState: mergeSyncRuntimeState(rawData.syncRuntimeState),
    };
  }

  return {
    settings: mergePluginSettings(rawData),
    localState: createDefaultLocalSyncState(),
    baseRevisionState: createDefaultBaseRevisionState(),
    remoteSyncState: createDefaultRemoteSyncState(),
    conflictActionKeyState: createDefaultConflictActionKeyState(),
    syncRuntimeState: createDefaultSyncRuntimeState(),
  };
}

export function serializePluginData(
  settings: PluginSettings,
  localState: LocalSyncState,
  baseRevisionState: BaseRevisionState,
  remoteSyncState: RemoteSyncState,
  conflictActionKeyState: ConflictActionKeyState,
  syncRuntimeState: SyncRuntimeState,
): HazeSyncPluginData {
  return {
    settings,
    localState,
    baseRevisionState,
    remoteSyncState,
    conflictActionKeyState,
    syncRuntimeState,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
