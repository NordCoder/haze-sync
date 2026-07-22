import { Plugin, TAbstractFile, TFile } from "obsidian";

import type { ConflictResolutionAction } from "./api-client";
import { HazeSyncApiClient, createConfigurationError } from "./api-client";
import {
  BaseRevisionState,
  createDefaultBaseRevisionState,
} from "./base-revision-store";
import {
  ConflictActionKeyState,
  clearConflictActionKey,
  conflictServerContext,
  createDefaultConflictActionKeyState,
  prepareConflictActionKey,
  retainOpenConflictActionKeys,
} from "./conflict-action-keys";
import type { ConflictCenterItem, ConflictResolutionResult } from "./conflict-center";
import { loadOpenConflictItems, resolveConflictThroughServer } from "./conflict-center";
import { ConflictCenterModal } from "./conflict-center-modal";
import { factFromEventPath } from "./local-file-facts";
import {
  LocalSyncState,
  PendingChangeKind,
  createDefaultLocalSyncState,
  pendingQueueSummaryText,
  recordEventHint,
  summarizePendingQueue,
} from "./pending-queue";
import { parsePluginData, serializePluginData } from "./plugin-data";
import { RemoteEchoSuppressor } from "./remote-materializer";
import { RemoteSyncState, createDefaultRemoteSyncState } from "./remote-sync-state";
import {
  PluginSettings,
  createDefaultPluginSettings,
  settingsAreReady,
  syncAutomationEnabled,
  syncModeLabel,
  validatePluginSettings,
} from "./settings";
import { HazeSyncSettingsTab } from "./settings-tab";
import {
  PullOperationResult,
  PushOperationResult,
  clearPendingChangeIfCurrent,
  pullRemoteChanges,
  pushPendingChanges,
  scanLocalVault,
} from "./sync-operations";
import { SyncRunScope, SyncRunner } from "./sync-runner";
import {
  SyncRuntimeState,
  SyncTrigger,
  createDefaultSyncRuntimeState,
  syncRuntimeSummary,
} from "./sync-runtime-state";
import { SafeStatusReporter } from "./status";
import { VaultScanner } from "./vault-scanner";

export default class HazeSyncPlugin extends Plugin {
  settings: PluginSettings = createDefaultPluginSettings();

  private localState: LocalSyncState = createDefaultLocalSyncState();
  private baseRevisionState: BaseRevisionState = createDefaultBaseRevisionState();
  private remoteSyncState: RemoteSyncState = createDefaultRemoteSyncState();
  private conflictActionKeyState: ConflictActionKeyState = createDefaultConflictActionKeyState();
  private syncRuntimeState: SyncRuntimeState = createDefaultSyncRuntimeState();
  private readonly remoteEchoSuppressor = new RemoteEchoSuppressor();
  private pendingDataSave: Promise<void> = Promise.resolve();
  private serverOpenConflictCount?: number;
  private scanner?: VaultScanner;
  private syncRunner?: SyncRunner;
  private statusReporter?: SafeStatusReporter;
  private settingsTab?: HazeSyncSettingsTab;

  async onload(): Promise<void> {
    await this.loadSettings();

    this.scanner = new VaultScanner(this.app.vault);
    this.statusReporter = new SafeStatusReporter({
      statusItem: this.addStatusBarItem(),
      getSecrets: () => [this.settings.authToken],
    });
    this.syncRunner = new SyncRunner({
      initialState: this.syncRuntimeState,
      execute: (trigger, scope, signal) => this.executeSyncRun(trigger, scope, signal),
      onStateChange: (state) => this.onSyncRuntimeStateChanged(state),
    });

    this.settingsTab = new HazeSyncSettingsTab(this.app, this);
    this.addSettingTab(this.settingsTab);
    this.addManualSyncCommand();
    this.addScanCommand();
    this.addRemotePullCommand();
    this.addConflictCenterCommand();
    this.registerVaultEventHints();
    this.configureSyncAutomation();

    this.refreshSettingsStatus();
    this.statusReporter.notice("Plugin loaded. Use Sync now for an explicit guarded sync run.");
    this.showAutomationLimitationNotice();
  }

  onunload(): void {
    this.syncRunner?.dispose();
    this.syncRunner = undefined;
    this.remoteEchoSuppressor.dispose();
    this.statusReporter?.dispose();
    this.statusReporter = undefined;
    this.settingsTab = undefined;
    this.scanner = undefined;
  }

  async loadSettings(): Promise<void> {
    const data = parsePluginData(await this.loadData());
    this.settings = data.settings;
    this.localState = data.localState;
    this.baseRevisionState = data.baseRevisionState;
    this.remoteSyncState = data.remoteSyncState;
    this.conflictActionKeyState = data.conflictActionKeyState;
    this.syncRuntimeState = data.syncRuntimeState;
  }

  async saveSettings(): Promise<void> {
    const validation = validatePluginSettings(this.settings);
    this.settings = validation.normalized;
    await this.persistPluginData();
    this.configureSyncAutomation();
    this.refreshSettingsStatus(validation);
  }

  refreshSettingsStatus(validation = validatePluginSettings(this.settings)): void {
    if (this.statusReporter === undefined) {
      return;
    }

    if (validation.errors.length > 0) {
      this.statusReporter.setStatus("Settings need attention. Open plugin settings to fix validation errors.", "warning");
      return;
    }

    if (!settingsAreReady(validation.normalized)) {
      this.statusReporter.setStatus("Settings incomplete. Configure server URL, adapter identity, and token.", "warning");
      return;
    }

    const queueText = pendingQueueSummaryText(summarizePendingQueue(this.localState.pendingQueue));
    const baseCount = Object.keys(this.baseRevisionState.byPath).length;
    const localRemoteConflictCount = Object.keys(this.remoteSyncState.conflicts).length;
    const remoteCursor = this.remoteSyncState.changeCursor ?? "none";
    const serverConflictText =
      this.serverOpenConflictCount === undefined
        ? "server conflicts not loaded"
        : `${this.serverOpenConflictCount} open server conflict(s)`;
    const automationText = syncAutomationEnabled(validation.normalized)
      ? "automatic triggers are best-effort while Obsidian is active"
      : "automatic triggers are off";
    const runtimeText = syncRuntimeSummary(this.syncRuntimeState);

    if (validation.normalized.syncMode === "disabled") {
      this.statusReporter.setStatus(
        `Configured; sync mode is disabled; ${runtimeText}; ${automationText}; ${queueText}; ${baseCount} tracked base revision(s); remote cursor ${remoteCursor}; ${localRemoteConflictCount} local pull conflict(s); ${serverConflictText}.`,
      );
      return;
    }

    this.statusReporter.setStatus(
      `Configured in ${syncModeLabel(validation.normalized.syncMode)} mode; ${runtimeText}; ${automationText}; ${queueText}; ${baseCount} tracked base revision(s); remote cursor ${remoteCursor}; ${localRemoteConflictCount} local pull conflict(s); ${serverConflictText}.`,
    );
  }

  private addManualSyncCommand(): void {
    this.addCommand({
      id: "haze-sync-sync-now",
      name: "Sync now",
      callback: () => {
        void this.requestManualSync("full");
      },
    });
  }

  private addScanCommand(): void {
    this.addCommand({
      id: "haze-sync-scan-vault-local-queue",
      name: "Scan vault for local changes",
      callback: () => {
        void this.requestManualSync("scan_only");
      },
    });
  }

  private addRemotePullCommand(): void {
    this.addCommand({
      id: "haze-sync-pull-remote-changes",
      name: "Pull remote changes safely",
      callback: () => {
        void this.requestManualSync("pull_only");
      },
    });
  }

  private addConflictCenterCommand(): void {
    this.addCommand({
      id: "haze-sync-open-conflict-center",
      name: "Open conflict center",
      callback: () => {
        this.openConflictCenter();
      },
    });
  }

  private registerVaultEventHints(): void {
    this.registerEvent(
      this.app.vault.on("create", (file) => {
        this.recordVaultEventHint(file, "created");
      }),
    );

    this.registerEvent(
      this.app.vault.on("modify", (file) => {
        this.recordVaultEventHint(file, "modified");
      }),
    );

    this.registerEvent(
      this.app.vault.on("delete", (file) => {
        this.recordVaultEventHint(file, "deleted");
      }),
    );

    this.registerEvent(
      this.app.vault.on("rename", (file, oldPath) => {
        this.recordVaultEventHint(file, "deleted", oldPath);
        this.recordVaultEventHint(file, "created");
      }),
    );
  }

  private async requestManualSync(scope: SyncRunScope): Promise<void> {
    if (this.syncRunner === undefined) {
      return;
    }

    if (scope !== "scan_only") {
      const validation = validatePluginSettings(this.settings);
      this.settings = validation.normalized;
      if (!settingsAreReady(this.settings)) {
        this.refreshSettingsStatus(validation);
        this.statusReporter?.notice("Configure Haze Sync settings before running server sync.", "warning");
        return;
      }
      if (scope === "full" && this.settings.syncMode === "disabled") {
        this.statusReporter?.notice("Sync now is disabled by the current sync mode.", "warning");
        return;
      }
      if (scope === "pull_only" && !modeAllowsPull(this.settings.syncMode)) {
        this.statusReporter?.notice("Remote pull is disabled by the current sync mode.", "warning");
        return;
      }
    }

    const result = await this.syncRunner.request("manual", scope);
    switch (result.status) {
      case "already_running":
        this.statusReporter?.notice("A sync run is already active. The new manual request was not started.", "warning");
        break;
      case "completed":
        this.statusReporter?.notice(result.state.summary ?? "Sync completed.", "success");
        break;
      case "failed":
        this.statusReporter?.notice(result.state.summary ?? "Sync failed safely.", "warning");
        break;
      case "backoff":
        this.statusReporter?.notice("Automatic retry backoff is active. Manual Sync now may be used to retry immediately.", "warning");
        break;
      case "stopped":
        break;
    }
  }

  private async executeSyncRun(
    trigger: SyncTrigger,
    scope: SyncRunScope,
    signal: AbortSignal,
  ): Promise<{ summary: string }> {
    if (this.scanner === undefined) {
      throw new Error("Vault scanner is unavailable.");
    }

    const scanResult = await scanLocalVault(this.scanner, this.localState, signal);
    this.localState = scanResult.localState;
    await this.persistPluginData();

    if (scope === "scan_only") {
      return {
        summary: scanSummary(scanResult.unreadableFiles, this.localState),
      };
    }

    const validation = validatePluginSettings(this.settings);
    this.settings = validation.normalized;
    if (!settingsAreReady(this.settings)) {
      throw createConfigurationError("Haze Sync settings are incomplete.");
    }
    if (scope === "full" && this.settings.syncMode === "disabled") {
      throw createConfigurationError("Sync mode is disabled.");
    }
    if (scope === "pull_only" && !modeAllowsPull(this.settings.syncMode)) {
      throw createConfigurationError("Remote pull is disabled by the current sync mode.");
    }

    const client = HazeSyncApiClient.fromSettings(this.settings, undefined, signal);
    let pushResult: PushOperationResult | undefined;
    let pullResult: PullOperationResult | undefined;

    if (scope === "full" && modeAllowsPush(this.settings.syncMode)) {
      pushResult = await pushPendingChanges({
        vault: this.app.vault,
        client,
        localState: this.localState,
        baseRevisionState: this.baseRevisionState,
        signal,
        allowDeletes: !this.settings.safety.confirmBeforeDelete,
        onProgress: async (progress) => {
          this.baseRevisionState = progress.baseRevisionState;
          this.localState = clearPendingChangeIfCurrent(
            this.localState,
            progress.path,
            progress.operationIdempotencyKey,
          );
          await this.persistPluginData();
        },
      });
    }

    if (
      scope === "pull_only" ||
      (scope === "full" && modeAllowsPull(this.settings.syncMode))
    ) {
      pullResult = await pullRemoteChanges({
        vault: this.app.vault,
        client,
        baseRevisionState: this.baseRevisionState,
        remoteSyncState: this.remoteSyncState,
        signal,
        echoSuppressor: this.remoteEchoSuppressor,
        onProgress: async (progress) => {
          this.baseRevisionState = progress.baseRevisionState;
          this.remoteSyncState = progress.remoteSyncState;
          await this.persistPluginData();
        },
      });
    }

    const conflicts = await this.refreshOpenConflicts(client);
    await this.persistPluginData();
    this.refreshSettingsStatus();

    return {
      summary: fullSyncSummary(
        trigger,
        this.settings.syncMode,
        scanResult.unreadableFiles,
        this.localState,
        pushResult,
        pullResult,
        conflicts.length,
      ),
    };
  }

  private configureSyncAutomation(): void {
    if (this.syncRunner === undefined) {
      return;
    }

    const active = settingsAreReady(this.settings) && this.settings.syncMode !== "disabled";
    this.syncRunner.configureAutomation({
      intervalMs:
        active && this.settings.automation.intervalMinutes > 0
          ? this.settings.automation.intervalMinutes * 60_000
          : null,
      eventDelayMs:
        active && this.settings.automation.syncOnFileEvents
          ? this.settings.automation.eventDebounceSeconds * 1_000
          : null,
    });
  }

  private onSyncRuntimeStateChanged(state: SyncRuntimeState): void {
    const previous = this.syncRuntimeState;
    this.syncRuntimeState = state;
    this.refreshSettingsStatus();
    void this.persistPluginData().catch(() => {
      this.statusReporter?.setStatus("Could not save sync runtime status.", "warning");
    });

    if (
      state.lastTrigger !== "manual" &&
      previous.phase === "running" &&
      (state.phase === "offline" || state.phase === "backoff" || state.phase === "error")
    ) {
      this.statusReporter?.notice(state.summary ?? "Automatic sync failed safely.", "warning");
    }
  }

  private showAutomationLimitationNotice(): void {
    if (!syncAutomationEnabled(this.settings) || !this.settings.safety.showMobileBackgroundWarning) {
      return;
    }

    this.statusReporter?.notice(
      "Automatic sync is best-effort only while Obsidian keeps the plugin active. Mobile suspension may delay or skip runs.",
      "warning",
    );
  }

  private openConflictCenter(): void {
    const validation = validatePluginSettings(this.settings);
    this.settings = validation.normalized;
    if (!settingsAreReady(this.settings)) {
      this.refreshSettingsStatus(validation);
      this.statusReporter?.notice("Configure Haze Sync settings before opening the conflict center.", "warning");
      return;
    }

    new ConflictCenterModal(this.app, {
      loadOpenConflicts: () => this.loadOpenConflicts(),
      prepareConflictAction: (item, action) => this.prepareOpenConflictAction(item, action),
      resolveConflict: (item, action, idempotencyKey) =>
        this.resolveOpenConflict(item, action, idempotencyKey),
      canResolveConflicts: () => this.canResolveConflicts(),
    }).open();
  }

  private async loadOpenConflicts(): Promise<ConflictCenterItem[]> {
    return this.refreshOpenConflicts(HazeSyncApiClient.fromSettings(this.settings));
  }

  private async refreshOpenConflicts(client: HazeSyncApiClient): Promise<ConflictCenterItem[]> {
    const items = await loadOpenConflictItems(client, [this.settings.authToken]);
    const retainedKeys = retainOpenConflictActionKeys(
      this.conflictActionKeyState,
      this.currentConflictServerContext(),
      new Set(items.map((item) => item.conflict.id)),
    );

    if (retainedKeys !== this.conflictActionKeyState) {
      this.conflictActionKeyState = retainedKeys;
      await this.persistPluginData();
    }

    this.serverOpenConflictCount = items.length;
    this.refreshSettingsStatus();
    return items;
  }

  private async prepareOpenConflictAction(
    item: ConflictCenterItem,
    action: ConflictResolutionAction,
  ): Promise<string> {
    if (!this.canResolveConflicts()) {
      throw new Error("Conflict resolution is disabled while sync is running or by the current sync mode.");
    }

    const previousState = this.conflictActionKeyState;
    const prepared = prepareConflictActionKey(
      previousState,
      this.currentConflictServerContext(),
      item.conflict.id,
      action,
      new Date().toISOString(),
    );
    this.conflictActionKeyState = prepared.state;

    if (prepared.state !== previousState) {
      try {
        await this.persistPluginData();
      } catch (error) {
        if (this.conflictActionKeyState === prepared.state) {
          this.conflictActionKeyState = previousState;
        }
        throw error;
      }
    }

    return prepared.idempotencyKey;
  }

  private async resolveOpenConflict(
    item: ConflictCenterItem,
    action: ConflictResolutionAction,
    idempotencyKey: string,
  ): Promise<ConflictResolutionResult> {
    if (!this.canResolveConflicts()) {
      throw new Error("Conflict resolution is disabled while sync is running or by the current sync mode.");
    }

    const result = await resolveConflictThroughServer(
      HazeSyncApiClient.fromSettings(this.settings),
      item,
      action,
      idempotencyKey,
      this.baseRevisionState,
      new Date().toISOString(),
    );

    if (!result.serverConfirmed) {
      return result;
    }

    this.baseRevisionState = result.baseRevisionState;
    this.conflictActionKeyState = clearConflictActionKey(
      this.conflictActionKeyState,
      this.currentConflictServerContext(),
      item.conflict.id,
      action,
    );

    try {
      await this.persistPluginData();
      this.refreshSettingsStatus();
      return result;
    } catch {
      this.refreshSettingsStatus();
      return {
        ...result,
        message: "The server confirmed the conflict action, but local metadata could not be saved. Refresh the conflict center before taking another action.",
      };
    }
  }

  private canResolveConflicts(): boolean {
    return (
      settingsAreReady(this.settings) &&
      this.settings.syncMode !== "disabled" &&
      this.settings.syncMode !== "dry_run" &&
      this.syncRuntimeState.phase !== "running"
    );
  }

  private currentConflictServerContext(): string {
    return conflictServerContext(this.settings.serverUrl, this.settings.adapterId);
  }

  private recordVaultEventHint(file: TAbstractFile, kind: PendingChangeKind, pathOverride?: string): void {
    if (!(file instanceof TFile) && pathOverride === undefined) {
      return;
    }

    const classification = factFromEventPath(pathOverride ?? file.path);
    if (!classification.included) {
      return;
    }

    if (this.remoteEchoSuppressor.isSuppressed(classification.path)) {
      return;
    }

    this.localState = recordEventHint(this.localState, classification.path, kind, new Date().toISOString());
    void this.persistPluginData().catch(() => {
      this.statusReporter?.setStatus("Could not save local pending queue state.", "warning");
    });
    this.syncRunner?.scheduleEventTrigger();
    this.refreshSettingsStatus();
  }

  private persistPluginData(): Promise<void> {
    const data = serializePluginData(
      this.settings,
      this.localState,
      this.baseRevisionState,
      this.remoteSyncState,
      this.conflictActionKeyState,
      this.syncRuntimeState,
    );
    const write = this.pendingDataSave.then(
      () => this.saveData(data),
      () => this.saveData(data),
    );
    this.pendingDataSave = write.catch(() => undefined);

    return write;
  }
}

function modeAllowsPush(mode: PluginSettings["syncMode"]): boolean {
  return mode === "push_only" || mode === "bidirectional";
}

function modeAllowsPull(mode: PluginSettings["syncMode"]): boolean {
  return mode === "pull_only" || mode === "bidirectional";
}

function scanSummary(unreadableFiles: number, localState: LocalSyncState): string {
  const queueText = pendingQueueSummaryText(summarizePendingQueue(localState.pendingQueue));
  const unreadableText = unreadableFiles === 0 ? "" : `; ${unreadableFiles} file(s) could not be read`;
  return `Vault scan complete: ${queueText}${unreadableText}.`;
}

function fullSyncSummary(
  trigger: SyncTrigger,
  mode: PluginSettings["syncMode"],
  unreadableFiles: number,
  localState: LocalSyncState,
  push: PushOperationResult | undefined,
  pull: PullOperationResult | undefined,
  openConflicts: number,
): string {
  const parts = [
    `Sync (${trigger}, ${syncModeLabel(mode)}) complete`,
    pendingQueueSummaryText(summarizePendingQueue(localState.pendingQueue)),
  ];

  if (unreadableFiles > 0) {
    parts.push(`${unreadableFiles} unreadable file(s)`);
  }
  if (push !== undefined) {
    parts.push(
      `${push.uploaded} upload(s), ${push.deleted} delete(s), ${push.sameContent} already current, ${push.conflicts} push conflict(s), ${push.rejected} rejected, ${push.skipped} skipped`,
    );
  }
  if (pull !== undefined) {
    parts.push(
      `${pull.applied} remote applied, ${pull.noOps} already current, ${pull.tombstones} tombstone(s), ${pull.conflicts} pull conflict(s)`,
    );
    if (pull.hasMore) {
      parts.push("more remote changes remain after the bounded pull pass");
    }
  }
  if (mode === "dry_run") {
    parts.push("dry run performed no server or vault mutations");
  }
  parts.push(`${openConflicts} open server conflict(s)`);

  return `${parts.join("; ")}.`;
}
