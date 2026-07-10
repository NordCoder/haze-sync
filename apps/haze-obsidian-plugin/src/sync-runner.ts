import { ApiClientError, ApiErrorCategory } from "./api-client";
import {
  SyncRuntimeState,
  SyncTrigger,
  isRetryableSyncCategory,
  markSyncFailed,
  markSyncStarted,
  markSyncStopped,
  markSyncSucceeded,
  syncBackoffRemainingMs,
} from "./sync-runtime-state";

export type SyncRunScope = "full" | "scan_only" | "pull_only";

export interface SyncAutomationConfig {
  intervalMs: number | null;
  eventDelayMs: number | null;
}

export interface SyncExecutionResult {
  summary: string;
}

export type SyncRequestStatus = "completed" | "failed" | "already_running" | "backoff" | "stopped";

export interface SyncRequestResult {
  status: SyncRequestStatus;
  state: SyncRuntimeState;
}

export interface SyncRunnerOptions {
  initialState: SyncRuntimeState;
  execute(trigger: SyncTrigger, scope: SyncRunScope, signal: AbortSignal): Promise<SyncExecutionResult>;
  onStateChange(state: SyncRuntimeState): void;
}

export class SyncRunner {
  private state: SyncRuntimeState;
  private readonly execute: SyncRunnerOptions["execute"];
  private readonly onStateChange: SyncRunnerOptions["onStateChange"];
  private automation: SyncAutomationConfig = { intervalMs: null, eventDelayMs: null };
  private runningPromise?: Promise<SyncRequestResult>;
  private abortController?: AbortController;
  private intervalTimer?: number;
  private eventTimer?: number;
  private retryTimer?: number;
  private followUpTimer?: number;
  private pendingAutoTrigger = false;
  private disposed = false;

  constructor(options: SyncRunnerOptions) {
    this.state = options.initialState;
    this.execute = options.execute;
    this.onStateChange = options.onStateChange;
  }

  getState(): SyncRuntimeState {
    return this.state;
  }

  configureAutomation(config: SyncAutomationConfig): void {
    this.automation = normalizeAutomationConfig(config);
    this.clearIntervalTimer();
    this.clearEventTimer();
    this.clearRetryTimer();
    this.clearFollowUpTimer();

    if (this.disposed) {
      return;
    }

    if (!automationEnabled(this.automation)) {
      this.pendingAutoTrigger = false;
      return;
    }

    if (this.automation.intervalMs !== null) {
      this.intervalTimer = window.setInterval(() => {
        void this.request("interval");
      }, this.automation.intervalMs);
    }

    this.scheduleRetryIfNeeded();
  }

  scheduleEventTrigger(): void {
    if (this.disposed || this.automation.eventDelayMs === null) {
      return;
    }

    this.clearEventTimer();
    this.eventTimer = window.setTimeout(() => {
      this.eventTimer = undefined;
      void this.request("event");
    }, this.automation.eventDelayMs);
  }

  async request(trigger: SyncTrigger, scope: SyncRunScope = "full"): Promise<SyncRequestResult> {
    if (this.disposed) {
      return { status: "stopped", state: this.state };
    }

    if (this.runningPromise !== undefined) {
      if (trigger !== "manual" && automationEnabled(this.automation)) {
        this.pendingAutoTrigger = true;
      }
      return { status: "already_running", state: this.state };
    }

    if (trigger !== "manual" && syncBackoffRemainingMs(this.state) > 0) {
      this.scheduleRetryIfNeeded();
      return { status: "backoff", state: this.state };
    }

    this.clearRetryTimer();
    this.clearEventTimer();
    const controller = new AbortController();
    this.abortController = controller;
    this.updateState(markSyncStarted(this.state, trigger, new Date().toISOString()));

    const run = this.executeRun(trigger, scope, controller.signal);
    this.runningPromise = run;

    try {
      return await run;
    } finally {
      if (this.runningPromise === run) {
        this.runningPromise = undefined;
      }
      if (this.abortController === controller) {
        this.abortController = undefined;
      }

      if (!this.disposed && this.pendingAutoTrigger && automationEnabled(this.automation)) {
        this.pendingAutoTrigger = false;
        this.clearFollowUpTimer();
        this.followUpTimer = window.setTimeout(() => {
          this.followUpTimer = undefined;
          void this.request("event");
        }, 0);
      }
    }
  }

  dispose(): void {
    if (this.disposed) {
      return;
    }

    this.disposed = true;
    this.pendingAutoTrigger = false;
    this.clearIntervalTimer();
    this.clearEventTimer();
    this.clearRetryTimer();
    this.clearFollowUpTimer();
    this.abortController?.abort();
    this.abortController = undefined;
    this.updateState(markSyncStopped(this.state, new Date().toISOString()));
  }

  private async executeRun(
    trigger: SyncTrigger,
    scope: SyncRunScope,
    signal: AbortSignal,
  ): Promise<SyncRequestResult> {
    try {
      const result = await this.execute(trigger, scope, signal);
      if (signal.aborted || this.disposed) {
        return { status: "stopped", state: this.state };
      }

      this.updateState(markSyncSucceeded(this.state, new Date().toISOString(), result.summary));
      return { status: "completed", state: this.state };
    } catch (error) {
      if (signal.aborted || this.disposed || isAbortError(error)) {
        return { status: "stopped", state: this.state };
      }

      const category = errorCategory(error);
      this.updateState(
        markSyncFailed(
          this.state,
          category,
          new Date().toISOString(),
          safeFailureSummary(category, trigger),
        ),
      );
      this.scheduleRetryIfNeeded();
      return { status: "failed", state: this.state };
    }
  }

  private updateState(state: SyncRuntimeState): void {
    this.state = state;
    this.onStateChange(state);
  }

  private scheduleRetryIfNeeded(): void {
    this.clearRetryTimer();
    if (
      this.disposed ||
      !automationEnabled(this.automation) ||
      this.state.nextRetryAt === undefined ||
      this.state.lastErrorCategory === undefined ||
      !isRetryableSyncCategory(this.state.lastErrorCategory)
    ) {
      return;
    }

    this.retryTimer = window.setTimeout(() => {
      this.retryTimer = undefined;
      void this.request("retry");
    }, syncBackoffRemainingMs(this.state));
  }

  private clearIntervalTimer(): void {
    if (this.intervalTimer !== undefined) {
      window.clearInterval(this.intervalTimer);
      this.intervalTimer = undefined;
    }
  }

  private clearEventTimer(): void {
    if (this.eventTimer !== undefined) {
      window.clearTimeout(this.eventTimer);
      this.eventTimer = undefined;
    }
  }

  private clearRetryTimer(): void {
    if (this.retryTimer !== undefined) {
      window.clearTimeout(this.retryTimer);
      this.retryTimer = undefined;
    }
  }

  private clearFollowUpTimer(): void {
    if (this.followUpTimer !== undefined) {
      window.clearTimeout(this.followUpTimer);
      this.followUpTimer = undefined;
    }
  }
}

function normalizeAutomationConfig(config: SyncAutomationConfig): SyncAutomationConfig {
  return {
    intervalMs: normalizeDelay(config.intervalMs, 60_000),
    eventDelayMs: normalizeDelay(config.eventDelayMs, 1_000),
  };
}

function normalizeDelay(value: number | null, minimum: number): number | null {
  return value !== null && Number.isFinite(value) && value >= minimum ? Math.floor(value) : null;
}

function automationEnabled(config: SyncAutomationConfig): boolean {
  return config.intervalMs !== null || config.eventDelayMs !== null;
}

function errorCategory(error: unknown): ApiErrorCategory {
  return error instanceof ApiClientError ? error.category : "internal";
}

function safeFailureSummary(category: ApiErrorCategory, trigger: SyncTrigger): string {
  switch (category) {
    case "offline":
      return `Sync started by ${trigger} could not reach Haze Sync Server.`;
    case "rate_limited":
      return `Sync started by ${trigger} was rate limited and entered backoff.`;
    case "server_unavailable":
      return `Sync started by ${trigger} found Haze Sync Server unavailable and entered backoff.`;
    case "unauthorized":
    case "forbidden":
      return `Sync started by ${trigger} was not authorized. Check plugin credentials.`;
    case "configuration":
      return `Sync started by ${trigger} could not run because settings are invalid.`;
    case "conflict":
      return `Sync started by ${trigger} stopped on a server conflict.`;
    case "not_found":
    case "rejected":
    case "invalid_response":
    case "internal":
      return `Sync started by ${trigger} failed safely.`;
  }
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}
