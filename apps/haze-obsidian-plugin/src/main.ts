import { Plugin } from "obsidian";

import {
  PluginSettings,
  SettingsValidationResult,
  createDefaultPluginSettings,
  mergePluginSettings,
  settingsAreReady,
  syncModeLabel,
  validatePluginSettings,
} from "./settings";
import { HazeSyncSettingsTab } from "./settings-tab";
import { SafeStatusReporter } from "./status";

export default class HazeSyncPlugin extends Plugin {
  settings: PluginSettings = createDefaultPluginSettings();

  private statusReporter?: SafeStatusReporter;
  private settingsTab?: HazeSyncSettingsTab;

  async onload(): Promise<void> {
    await this.loadSettings();

    this.statusReporter = new SafeStatusReporter({
      statusItem: this.addStatusBarItem(),
      getSecrets: () => [this.settings.authToken],
    });

    this.settingsTab = new HazeSyncSettingsTab(this.app, this);
    this.addSettingTab(this.settingsTab);

    this.refreshSettingsStatus();
    this.statusReporter.notice("Plugin loaded. Configure settings before enabling sync.");
  }

  onunload(): void {
    this.statusReporter?.dispose();
    this.statusReporter = undefined;
    this.settingsTab = undefined;
  }

  async loadSettings(): Promise<void> {
    this.settings = mergePluginSettings(await this.loadData());
  }

  async saveSettings(): Promise<void> {
    const validation = validatePluginSettings(this.settings);
    this.settings = validation.normalized;
    await this.saveData(this.settings);
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

    if (validation.normalized.syncMode === "disabled") {
      this.statusReporter.setStatus("Configured; sync mode is disabled.");
      return;
    }

    this.statusReporter.setStatus(`Configured in ${syncModeLabel(validation.normalized.syncMode)} mode.`);
  }
}
