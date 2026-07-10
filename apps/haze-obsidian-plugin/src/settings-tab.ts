import { App, Plugin, PluginSettingTab, Setting } from "obsidian";

import {
  EVENT_DEBOUNCE_SECONDS,
  EventDebounceSeconds,
  PluginSettings,
  SYNC_INTERVAL_MINUTES,
  SYNC_MODES,
  SettingsValidationResult,
  SyncIntervalMinutes,
  SyncMode,
  redactToken,
  syncModeLabel,
  tokenInputPlaceholder,
  validatePluginSettings,
} from "./settings";

export interface SettingsController extends Plugin {
  settings: PluginSettings;
  saveSettings(): Promise<void>;
  refreshSettingsStatus(validation?: SettingsValidationResult): void;
}

export class HazeSyncSettingsTab extends PluginSettingTab {
  constructor(app: App, private readonly controller: SettingsController) {
    super(app, controller);
  }

  display(): void {
    const { containerEl } = this;
    containerEl.empty();

    containerEl.createEl("h2", { text: "Haze Sync" });
    containerEl.createEl("p", {
      text: "Configure this vault as a Haze Sync Server client. Manual sync is explicit. Optional automatic triggers run only while Obsidian keeps this plugin active.",
    });

    this.renderValidation(validatePluginSettings(this.controller.settings));
    this.renderConnectionSettings();
    this.renderModeSettings();
    this.renderAutomationSettings();
    this.renderSafetySettings();
  }

  private renderConnectionSettings(): void {
    new Setting(this.containerEl)
      .setName("Server URL")
      .setDesc("Base URL for Haze Sync Server. Tokens are sent only to this configured origin.")
      .addText((text) => {
        text
          .setPlaceholder("https://sync.example.com")
          .setValue(this.controller.settings.serverUrl)
          .onChange(async (value) => {
            this.controller.settings.serverUrl = value;
            await this.saveAndRefreshStatus();
          });
      });

    new Setting(this.containerEl)
      .setName("Adapter identity")
      .setDesc("Stable adapter id used by the server to identify this Obsidian client.")
      .addText((text) => {
        text
          .setPlaceholder("obsidian-plugin")
          .setValue(this.controller.settings.adapterId)
          .onChange(async (value) => {
            this.controller.settings.adapterId = value;
            await this.saveAndRefreshStatus();
          });
      });

    new Setting(this.containerEl)
      .setName("Adapter token")
      .setDesc(`Current value: ${redactToken(this.controller.settings.authToken)}. Token text is never shown after it is saved.`)
      .addText((text) => {
        text.inputEl.type = "password";
        text
          .setPlaceholder(tokenInputPlaceholder(this.controller.settings.authToken))
          .setValue("")
          .onChange(async (value) => {
            if (value.length === 0) {
              return;
            }

            this.controller.settings.authToken = value;
            await this.saveAndRefreshStatus();
            text.setValue("");
          });
      })
      .addButton((button) => {
        button
          .setButtonText("Clear token")
          .setTooltip("Remove the stored adapter token")
          .onClick(async () => {
            this.controller.settings.authToken = "";
            await this.saveAndRefreshStatus();
            this.display();
          });
      });
  }

  private renderModeSettings(): void {
    new Setting(this.containerEl)
      .setName("Sync mode")
      .setDesc("Controls which server operations the explicit sync runner may perform.")
      .addDropdown((dropdown) => {
        for (const mode of SYNC_MODES) {
          dropdown.addOption(mode, syncModeLabel(mode));
        }

        dropdown.setValue(this.controller.settings.syncMode).onChange(async (value) => {
          this.controller.settings.syncMode = value as SyncMode;
          await this.saveAndRefreshStatus();
        });
      });
  }

  private renderAutomationSettings(): void {
    this.containerEl.createEl("h3", { text: "Automatic triggers" });
    this.containerEl.createEl("p", {
      text: "Automatic sync is best-effort. It runs only while Obsidian and this plugin remain active. Mobile operating systems may suspend Obsidian and delay or skip timers and file events.",
    });

    new Setting(this.containerEl)
      .setName("Interval sync")
      .setDesc("Run the same guarded sync runner on an interval while the plugin is active. Off by default.")
      .addDropdown((dropdown) => {
        for (const minutes of SYNC_INTERVAL_MINUTES) {
          dropdown.addOption(String(minutes), intervalLabel(minutes));
        }

        dropdown
          .setValue(String(this.controller.settings.automation.intervalMinutes))
          .onChange(async (value) => {
            this.controller.settings.automation.intervalMinutes = Number(value) as SyncIntervalMinutes;
            await this.saveAndRefreshStatus();
          });
      });

    new Setting(this.containerEl)
      .setName("Sync after vault events")
      .setDesc("Debounce supported vault create, modify, delete, and rename events before requesting sync. Events remain hints; each run performs a full scan.")
      .addToggle((toggle) => {
        toggle
          .setValue(this.controller.settings.automation.syncOnFileEvents)
          .onChange(async (value) => {
            this.controller.settings.automation.syncOnFileEvents = value;
            await this.saveAndRefreshStatus();
          });
      });

    new Setting(this.containerEl)
      .setName("Event debounce")
      .setDesc("Wait for a quiet period after vault events before starting an automatic sync run.")
      .addDropdown((dropdown) => {
        for (const seconds of EVENT_DEBOUNCE_SECONDS) {
          dropdown.addOption(String(seconds), `${seconds} seconds`);
        }

        dropdown
          .setValue(String(this.controller.settings.automation.eventDebounceSeconds))
          .onChange(async (value) => {
            this.controller.settings.automation.eventDebounceSeconds = Number(value) as EventDebounceSeconds;
            await this.saveAndRefreshStatus();
          });
      });
  }

  private renderSafetySettings(): void {
    this.containerEl.createEl("h3", { text: "Safety" });

    new Setting(this.containerEl)
      .setName("Protect local changes")
      .setDesc("Never overwrite dirty local files during remote materialization.")
      .addToggle((toggle) => {
        toggle
          .setValue(this.controller.settings.safety.protectLocalChanges)
          .onChange(async (value) => {
            this.controller.settings.safety.protectLocalChanges = value;
            await this.saveAndRefreshStatus();
          });
      });

    new Setting(this.containerEl)
      .setName("Confirm delete actions")
      .setDesc("Require explicit confirmation before future destructive local delete handling.")
      .addToggle((toggle) => {
        toggle
          .setValue(this.controller.settings.safety.confirmBeforeDelete)
          .onChange(async (value) => {
            this.controller.settings.safety.confirmBeforeDelete = value;
            await this.saveAndRefreshStatus();
          });
      });

    new Setting(this.containerEl)
      .setName("Show mobile/background warning")
      .setDesc("Keep UI messaging explicit that reliable mobile background sync is not guaranteed.")
      .addToggle((toggle) => {
        toggle
          .setValue(this.controller.settings.safety.showMobileBackgroundWarning)
          .onChange(async (value) => {
            this.controller.settings.safety.showMobileBackgroundWarning = value;
            await this.saveAndRefreshStatus();
          });
      });
  }

  private renderValidation(validation: SettingsValidationResult): void {
    if (validation.errors.length === 0 && validation.warnings.length === 0) {
      return;
    }

    const validationEl = this.containerEl.createDiv({ cls: "haze-sync-settings-validation" });

    for (const error of validation.errors) {
      validationEl.createEl("p", { text: `Error: ${error}` });
    }

    for (const warning of validation.warnings) {
      validationEl.createEl("p", { text: `Warning: ${warning}` });
    }
  }

  private async saveAndRefreshStatus(): Promise<void> {
    await this.controller.saveSettings();
    this.controller.refreshSettingsStatus();
  }
}

function intervalLabel(minutes: SyncIntervalMinutes): string {
  if (minutes === 0) {
    return "Off";
  }
  return minutes === 1 ? "Every minute" : `Every ${minutes} minutes`;
}
