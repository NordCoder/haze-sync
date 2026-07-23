import type { ApiErrorCategory } from "./api-client";

export type SyncTrigger = "manual" | "event" | "interval" | "retry";
export type SyncRuntimePhase = "idle" | "running" | "offline" | "backoff" | "error" | "stopped";

export interface SyncRuntimeState {
  phase: SyncRuntimePhase;
  consecutiveFailures: number;
  lastTrigger?: SyncTrigger;
  lastStartedAt?: string;
  lastCompletedAt?: string;
  lastSuccessfulAt?: string;
  lastFailureAt?: string;
  lastErrorCategory?: ApiErrorCategory;
  nextRetryAt?: string;
  summary?: string;
}

const BASE_BACKOFF_MS = 5_000;
const RATE_LIMIT_BACKOFF_MS = 30_000;
const MAX_BACKOFF_MS = 5 * 60_000;

export function createDefaultSyncRuntimeState(): SyncRuntimeState {
  return {
    phase: "idle",
    consecutiveFailures: 0,
  };
}

export function mergeSyncRuntimeState(rawState: unknown): SyncRuntimeState {
  if (!isRecord(rawState)) {
    return createDefaultSyncRuntimeState();
  }

  const phase = isSyncRuntimePhase(rawState.phase) ? rawState.phase : "idle";
  return {
    phase: phase === "running" || phase === "stopped" ? "idle" : phase,
    consecutiveFailures: readNonNegativeInteger(rawState.consecutiveFailures),
    lastTrigger: isSyncTrigger(rawState.lastTrigger) ? rawState.lastTrigger : undefined,
    lastStartedAt: readOptionalTimestamp(rawState.lastStartedAt),
    lastCompletedAt: readOptionalTimestamp(rawState.lastCompletedAt),
    lastSuccessfulAt: readOptionalTimestamp(rawState.lastSuccessfulAt),
    lastFailureAt: readOptionalTimestamp(rawState.lastFailureAt),
    lastErrorCategory: isApiErrorCategory(rawState.lastErrorCategory) ? rawState.lastErrorCategory : undefined,
    nextRetryAt: readOptionalTimestamp(rawState.nextRetryAt),
    summary: readOptionalText(rawState.summary),
  };
}

export function markSyncStarted(state: SyncRuntimeState, trigger: SyncTrigger, startedAt: string): SyncRuntimeState {
  return {
    ...state,
    phase: "running",
    lastTrigger: trigger,
    lastStartedAt: startedAt,
    lastCompletedAt: undefined,
    nextRetryAt: undefined,
    summary: `Sync started by ${trigger}.`,
  };
}

export function markSyncSucceeded(state: SyncRuntimeState, completedAt: string, summary: string): SyncRuntimeState {
  return {
    ...state,
    phase: "idle",
    consecutiveFailures: 0,
    lastCompletedAt: completedAt,
    lastSuccessfulAt: completedAt,
    lastErrorCategory: undefined,
    nextRetryAt: undefined,
    summary,
  };
}

export function markSyncFailed(
  state: SyncRuntimeState,
  category: ApiErrorCategory,
  failedAt: string,
  summary: string,
): SyncRuntimeState {
  const consecutiveFailures = state.consecutiveFailures + 1;
  const retryable = isRetryableSyncCategory(category);
  const nextRetryAt = retryable
    ? new Date(Date.parse(failedAt) + backoffDelayMs(category, consecutiveFailures)).toISOString()
    : undefined;

  return {
    ...state,
    phase: retryable ? (category === "offline" ? "offline" : "backoff") : "error",
    consecutiveFailures,
    lastCompletedAt: failedAt,
    lastFailureAt: failedAt,
    lastErrorCategory: category,
    nextRetryAt,
    summary,
  };
}

export function markSyncStopped(state: SyncRuntimeState, stoppedAt: string): SyncRuntimeState {
  return {
    ...state,
    phase: "stopped",
    lastCompletedAt: stoppedAt,
    nextRetryAt: undefined,
    summary: "Sync runner stopped because the plugin unloaded.",
  };
}

export function syncBackoffRemainingMs(state: SyncRuntimeState, now = Date.now()): number {
  if (state.nextRetryAt === undefined) {
    return 0;
  }

  const retryAt = Date.parse(state.nextRetryAt);
  return Number.isFinite(retryAt) ? Math.max(0, retryAt - now) : 0;
}

export function syncRuntimeSummary(state: SyncRuntimeState, now = Date.now()): string {
  switch (state.phase) {
    case "running":
      return "sync is running";
    case "offline":
    case "backoff": {
      const remainingSeconds = Math.ceil(syncBackoffRemainingMs(state, now) / 1_000);
      const prefix = state.phase === "offline" ? "offline" : "retry backoff";
      return remainingSeconds > 0 ? `${prefix}; retry in about ${remainingSeconds}s` : `${prefix}; retry is due`;
    }
    case "error":
      return state.lastErrorCategory === undefined
        ? "last sync failed"
        : `last sync failed (${state.lastErrorCategory.replace(/_/gu, " ")})`;
    case "stopped":
      return "sync runner stopped";
    case "idle":
      return state.lastSuccessfulAt === undefined
        ? "sync has not completed yet"
        : `last sync succeeded ${formatSafeTimestamp(state.lastSuccessfulAt)}`;
  }
}

export function isRetryableSyncCategory(category: ApiErrorCategory): boolean {
  return category === "offline" || category === "rate_limited" || category === "server_unavailable";
}

function backoffDelayMs(category: ApiErrorCategory, consecutiveFailures: number): number {
  const base = category === "rate_limited" ? RATE_LIMIT_BACKOFF_MS : BASE_BACKOFF_MS;
  const exponent = Math.max(0, Math.min(consecutiveFailures - 1, 10));
  return Math.min(MAX_BACKOFF_MS, base * 2 ** exponent);
}

function formatSafeTimestamp(value: string): string {
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? new Date(timestamp).toLocaleString() : "at an unknown time";
}

function readNonNegativeInteger(value: unknown): number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 ? value : 0;
}

function readOptionalTimestamp(value: unknown): string | undefined {
  return typeof value === "string" && Number.isFinite(Date.parse(value)) ? value : undefined;
}

function readOptionalText(value: unknown): string | undefined {
  return typeof value === "string" && value.length <= 500 ? value : undefined;
}

function isSyncTrigger(value: unknown): value is SyncTrigger {
  return value === "manual" || value === "event" || value === "interval" || value === "retry";
}

function isSyncRuntimePhase(value: unknown): value is SyncRuntimePhase {
  return (
    value === "idle" ||
    value === "running" ||
    value === "offline" ||
    value === "backoff" ||
    value === "error" ||
    value === "stopped"
  );
}

function isApiErrorCategory(value: unknown): value is ApiErrorCategory {
  return (
    value === "configuration" ||
    value === "offline" ||
    value === "unauthorized" ||
    value === "forbidden" ||
    value === "not_found" ||
    value === "conflict" ||
    value === "rejected" ||
    value === "rate_limited" ||
    value === "server_unavailable" ||
    value === "invalid_response" ||
    value === "internal"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
