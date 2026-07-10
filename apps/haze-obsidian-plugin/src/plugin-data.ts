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

export interface HazeSyncPluginData {
  settings: PluginSettings;
  localState: LocalSyncState;
  baseRevisionState: BaseRevisionState;
  remoteSyncState: RemoteSyncState;
  conflictActionKeyState: ConflictActionKeyState;
}

export function parsePluginData(rawData: unknown): HazeSyncPluginData {
  if (
    isRecord(rawData) &&
    (
      "settings" in rawData ||
      "localState" in rawData ||
      "baseRevisionState" in rawData ||
      "remoteSyncState" in rawData ||
      "conflictActionKeyState" in rawData
    )
  ) {
    return {
      settings: mergePluginSettings(rawData.settings),
      localState: mergeLocalSyncState(rawData.localState),
      baseRevisionState: mergeBaseRevisionState(rawData.baseRevisionState),
      remoteSyncState: mergeRemoteSyncState(rawData.remoteSyncState),
      conflictActionKeyState: mergeConflictActionKeyState(rawData.conflictActionKeyState),
    };
  }

  return {
    settings: mergePluginSettings(rawData),
    localState: createDefaultLocalSyncState(),
    baseRevisionState: createDefaultBaseRevisionState(),
    remoteSyncState: createDefaultRemoteSyncState(),
    conflictActionKeyState: createDefaultConflictActionKeyState(),
  };
}

export function serializePluginData(
  settings: PluginSettings,
  localState: LocalSyncState,
  baseRevisionState: BaseRevisionState,
  remoteSyncState: RemoteSyncState,
  conflictActionKeyState: ConflictActionKeyState,
): HazeSyncPluginData {
  return {
    settings,
    localState,
    baseRevisionState,
    remoteSyncState,
    conflictActionKeyState,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
