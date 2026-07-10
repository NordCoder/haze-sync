import { Plugin, TAbstractFile, TFile } from "obsidian";

import type { ConflictResolutionAction } from "./api-client";
import { HazeSyncApiClient } from "./api-client";
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
  reconcileFullScan,
  recordEventHint,
  summarizePendingQueue,
} from "./pending-queue";
import { parsePluginData, serializePluginData } from "./plugin-data";
import {
  RemoteEchoSuppressor,
  fetchRemoteChangePage,
  markRemotePullStarted,
  materializeRemoteChange,
  remoteChangeNeedsDownload,
} from "./remote-materializer";
import { RemoteSyncState, createDefaultRemoteSyncState } from "./remote-sync-state";
import {
  PluginSettings,
  createDefaultPluginSettings,
  settingsAreReady,
  syncModeLabel,
  validatePluginSettings,
} from "./settings";
import { HazeSyncSettingsTab } from "./settings-tab";
import { SafeStatusReporter } from "./status";
import { VaultScanner } from "./vault-scanner";

export default class HazeSyncPlugin extends Plugin {
  settings: PluginSettings = createDefaultPluginSettings();

  private localState: LocalSyncState = createDefaultLocalSyncState();
  private baseRevisionState: BaseRevisionState = createDefaultBaseRevisionState();
  private remoteSyncState: RemoteSyncState = createDefaultRemoteSyncState();
  private conflictActionKeyState: ConflictActionKeyState = createDefaultConflictActionKeyState();
  private readonly remoteEchoSuppressor = new RemoteEchoSuppressor();
  private pendingDataSave: Promise<void> = Promise.resolve();
  private serverOpenConflictCount?: number;
  private scanner?: VaultScanner;
  private statusReporter?: SafeStatusReporter;
  private settingsTab?: HazeSyncSettingsTab;

  async onload(): Promise<void> {
    await this.loadSettings();

    this.scanner = new VaultScanner(this.app.vault);
    this.statusReporter = new SafeStatusReporter({
      statusItem: this.addStatusBarItem(),
      getSecrets: () => [this.settings.authToken],
    });

    this.settingsTab = new HazeSyncSettingsTab(this.app, this);
    this.addSettingTab(this.settingsTab);
    this.addScanCommand();
    this.addRemotePullCommand();
    this.addConflictCenterCommand();
    this.registerVaultEventHints();

    this.refreshSettingsStatus();
    this.statusReporter.notice("Plugin loaded. Configure settings before enabling sync.");
  }

  onunload(): void {
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
  }

  async saveSettings(): Promise<void> {
    const validation = validatePluginSettings(this.settings);
    this.settings = validation.normalized;
    await this.persistPluginData();
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

    if (validation.normalized.syncMode === "disabled") {
      this.statusReporter.setStatus(
        `Configured; sync mode is disabled; ${queueText}; ${baseCount} tracked base revision(s); remote cursor ${remoteCursor}; ${localRemoteConflictCount} local pull conflict(s); ${serverConflictText}.`,
      );
      return;
    }

    this.statusReporter.setStatus(
      `Configured in ${syncModeLabel(validation.normalized.syncMode)} mode; ${queueText}; ${baseCount} tracked base revision(s); remote cursor ${remoteCursor}; ${localRemoteConflictCount} local pull conflict(s); ${serverConflictText}.`,
    );
  }

  private addScanCommand(): void {
    this.addCommand({
      id: "haze-sync-scan-vault-local-queue",
      name: "Scan vault for local changes",
      callback: () => {
        void this.scanVaultForLocalChanges();
      },
    });
  }

  private addRemotePullCommand(): void {
    this.addCommand({
      id: "haze-sync-pull-remote-changes",
      name: "Pull remote changes safely",
      callback: () => {
        void this.pullRemoteChangesSafely();
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

  private async scanVaultForLocalChanges(): Promise<void> {
    if (this.scanner === undefined) {
      return;
    }

    const scanResult = await this.scanner.scan();
    const reconcileResult = reconcileFullScan(this.localState, scanResult.facts, scanResult.scannedAt);
    this.localState = reconcileResult.state;
    await this.persistPluginData();

    const queueText = pendingQueueSummaryText(reconcileResult.summary);
    const unreadableText = scanResult.errors.length === 0 ? "" : ` ${scanResult.errors.length} file(s) could not be read.`;
    const level = scanResult.errors.length === 0 ? "success" : "warning";

    this.statusReporter?.notice(`Vault scan complete: ${queueText}.${unreadableText}`, level);
    this.refreshSettingsStatus();
  }

  private async pullRemoteChangesSafely(): Promise<void> {
    const validation = validatePluginSettings(this.settings);
    this.settings = validation.normalized;
    if (!settingsAreReady(this.settings)) {
      this.refreshSettingsStatus(validation);
      this.statusReporter?.notice("Configure Haze Sync settings before pulling remote changes.", "warning");
      return;
    }

    if (this.settings.syncMode === "disabled" || this.settings.syncMode === "push_only" || this.settings.syncMode === "dry_run") {
      this.refreshSettingsStatus(validation);
      this.statusReporter?.notice("Remote pull is disabled by the current sync mode.", "warning");
      return;
    }

    const observedAt = new Date().toISOString();
    this.remoteSyncState = markRemotePullStarted(this.remoteSyncState, observedAt);

    let applied = 0;
    let conflicts = 0;
    let tombstones = 0;
    let noOps = 0;

    try {
      const client = HazeSyncApiClient.fromSettings(this.settings);
      const response = await fetchRemoteChangePage(client, this.remoteSyncState, { limit: 50 });

      for (const change of response.changes) {
        const download = remoteChangeNeedsDownload(change) ? await client.getFile(change.path) : undefined;
        const result = await materializeRemoteChange({
          vault: this.app.vault,
          change,
          download,
          baseRevisionState: this.baseRevisionState,
          remoteSyncState: this.remoteSyncState,
          observedAt,
          echoSuppressor: this.remoteEchoSuppressor,
        });

        this.baseRevisionState = result.baseRevisionState;
        this.remoteSyncState = result.remoteSyncState;

        switch (result.status) {
          case "applied":
            applied += 1;
            break;
          case "conflict_queued":
            conflicts += 1;
            await this.persistPluginData();
            this.refreshSettingsStatus();
            this.statusReporter?.notice(
              `Remote pull stopped: queued conflict for ${result.path}. Local file was preserved.`,
              "warning",
            );
            return;
          case "no_op":
            noOps += 1;
            break;
          case "tombstone_recorded":
            tombstones += 1;
            break;
        }
      }

      await this.persistPluginData();
      this.refreshSettingsStatus();
      const moreText = response.has_more ? " More remote changes are available; run pull again." : "";
      this.statusReporter?.notice(
        `Remote pull complete: ${applied} applied, ${noOps} already current, ${tombstones} tombstone(s), ${conflicts} conflict(s).${moreText}`,
        conflicts === 0 ? "success" : "warning",
      );
    } catch {
      await this.persistPluginData();
      this.refreshSettingsStatus();
      this.statusReporter?.notice("Remote pull failed safely. Local files were preserved and cursor was not advanced for the failed change.", "warning");
    }
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
    const client = HazeSyncApiClient.fromSettings(this.settings);
    const items = await loadOpenConflictItems(client, [this.settings.authToken]);
    const retainedKeys = retainOpenConflictActionKeys(
      this.conflictActionKeyState,
      this.currentConflictServerContext(),
      new Set(items.map((item) => item.conflict.id)),
    );

    if (retainedKeys !== this.conflictActionKeyState) {
      this.conflictActionKeyState = retainedKeys;
      try {
        await this.persistPluginData();
      } catch {
        // Stale-key cleanup is best effort and must not hide the server conflict list.
      }
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
      throw new Error("Conflict resolution is disabled by the current sync mode.");
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
      throw new Error("Conflict resolution is disabled by the current sync mode.");
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
      this.settings.syncMode !== "dry_run"
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
    this.refreshSettingsStatus();
  }

  private persistPluginData(): Promise<void> {
    const data = serializePluginData(
      this.settings,
      this.localState,
      this.baseRevisionState,
      this.remoteSyncState,
      this.conflictActionKeyState,
    );
    const write = this.pendingDataSave.then(
      () => this.saveData(data),
      () => this.saveData(data),
    );
    this.pendingDataSave = write.catch(() => undefined);

    return write;
  }
}
