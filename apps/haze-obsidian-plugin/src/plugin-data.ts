import {
  BaseRevisionState,
  createDefaultBaseRevisionState,
  mergeBaseRevisionState,
} from "./base-revision-store";
import { LocalSyncState, createDefaultLocalSyncState, mergeLocalSyncState } from "./pending-queue";
import { PluginSettings, mergePluginSettings } from "./settings";

export interface HazeSyncPluginData {
  settings: PluginSettings;
  localState: LocalSyncState;
  baseRevisionState: BaseRevisionState;
}

export function parsePluginData(rawData: unknown): HazeSyncPluginData {
  if (isRecord(rawData) && ("settings" in rawData || "localState" in rawData || "baseRevisionState" in rawData)) {
    return {
      settings: mergePluginSettings(rawData.settings),
      localState: mergeLocalSyncState(rawData.localState),
      baseRevisionState: mergeBaseRevisionState(rawData.baseRevisionState),
    };
  }

  return {
    settings: mergePluginSettings(rawData),
    localState: createDefaultLocalSyncState(),
    baseRevisionState: createDefaultBaseRevisionState(),
  };
}

export function serializePluginData(
  settings: PluginSettings,
  localState: LocalSyncState,
  baseRevisionState: BaseRevisionState,
): HazeSyncPluginData {
  return {
    settings,
    localState,
    baseRevisionState,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
