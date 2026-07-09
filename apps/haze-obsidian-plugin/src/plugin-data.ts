import { PluginSettings, mergePluginSettings } from "./settings";
import { LocalSyncState, createDefaultLocalSyncState, mergeLocalSyncState } from "./pending-queue";

export interface HazeSyncPluginData {
  settings: PluginSettings;
  localState: LocalSyncState;
}

export function parsePluginData(rawData: unknown): HazeSyncPluginData {
  if (isRecord(rawData) && ("settings" in rawData || "localState" in rawData)) {
    return {
      settings: mergePluginSettings(rawData.settings),
      localState: mergeLocalSyncState(rawData.localState),
    };
  }

  return {
    settings: mergePluginSettings(rawData),
    localState: createDefaultLocalSyncState(),
  };
}

export function serializePluginData(settings: PluginSettings, localState: LocalSyncState): HazeSyncPluginData {
  return {
    settings,
    localState,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
