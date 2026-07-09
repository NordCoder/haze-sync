import {
  BaseRevisionState,
  createDefaultBaseRevisionState,
  mergeBaseRevisionState,
} from "./base-revision-store";
import { LocalSyncState, createDefaultLocalSyncState, mergeLocalSyncState } from "./pending-queue";
import { RemoteSyncState, createDefaultRemoteSyncState, mergeRemoteSyncState } from "./remote-sync-state";
import { PluginSettings, mergePluginSettings } from "./settings";

export interface HazeSyncPluginData {
  settings: PluginSettings;
  localState: LocalSyncState;
  baseRevisionState: BaseRevisionState;
  remoteSyncState: RemoteSyncState;
}

export function parsePluginData(rawData: unknown): HazeSyncPluginData {
  if (
    isRecord(rawData) &&
    ("settings" in rawData || "localState" in rawData || "baseRevisionState" in rawData || "remoteSyncState" in rawData)
  ) {
    return {
      settings: mergePluginSettings(rawData.settings),
      localState: mergeLocalSyncState(rawData.localState),
      baseRevisionState: mergeBaseRevisionState(rawData.baseRevisionState),
      remoteSyncState: mergeRemoteSyncState(rawData.remoteSyncState),
    };
  }

  return {
    settings: mergePluginSettings(rawData),
    localState: createDefaultLocalSyncState(),
    baseRevisionState: createDefaultBaseRevisionState(),
    remoteSyncState: createDefaultRemoteSyncState(),
  };
}

export function serializePluginData(
  settings: PluginSettings,
  localState: LocalSyncState,
  baseRevisionState: BaseRevisionState,
  remoteSyncState: RemoteSyncState,
): HazeSyncPluginData {
  return {
    settings,
    localState,
    baseRevisionState,
    remoteSyncState,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
