import { Plugin, TAbstractFile, TFile } from "obsidian";

import {
  PluginSettings,
  SettingsValidationResult,
  createDefaultPluginSettings,
  settingsAreReady,
  syncModeLabel,
  validatePluginSettings,
} from "./settings";
import { HazeSyncSettingsTab } from "./settings-tab";
import { SafeStatusReporter } from "./status";
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
import { VaultScanner } from "./vault-scanner";

export default class HazeSyncPlugin extends Plugin {
  settings: PluginSettings = createDefaultPluginSettings();

  private localState: LocalSyncState = createDefaultLocalSyncState();
  private pendingDataSave: Promise<void> = Promise.resolve();
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
    this.registerVaultEventHints();

    this.refreshSettingsStatus();
    this.statusReporter.notice("Plugin loaded. Configure settings before enabling sync.");
  }

  onunload(): void {
    this.statusReporter?.dispose();
    this.statusReporter = undefined;
    this.settingsTab = undefined;
    this.scanner = undefined;
  }

  async loadSettings(): Promise<void> {
    const data = parsePluginData(await this.loadData());
    this.settings = data.settings;
    this.localState = data.localState;
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

    if (validation.normalized.syncMode === "disabled") {
      this.statusReporter.setStatus(`Configured; sync mode is disabled; ${queueText}.`);
      return;
    }

    this.statusReporter.setStatus(`Configured in ${syncModeLabel(validation.normalized.syncMode)} mode; ${queueText}.`);
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

  private recordVaultEventHint(file: TAbstractFile, kind: PendingChangeKind, pathOverride?: string): void {
    if (!(file instanceof TFile) && pathOverride === undefined) {
      return;
    }

    const classification = factFromEventPath(pathOverride ?? file.path);
    if (!classification.included) {
      return;
    }

    this.localState = recordEventHint(this.localState, classification.path, kind, new Date().toISOString());
    void this.persistPluginData().catch(() => {
      this.statusReporter?.setStatus("Could not save local pending queue state.", "warning");
    });
    this.refreshSettingsStatus();
  }

  private persistPluginData(): Promise<void> {
    const data = serializePluginData(this.settings, this.localState);
    const write = this.pendingDataSave.then(
      () => this.saveData(data),
      () => this.saveData(data),
    );
    this.pendingDataSave = write.catch(() => undefined);

    return write;
  }
}
