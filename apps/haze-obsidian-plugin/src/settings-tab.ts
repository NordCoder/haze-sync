import { App, Plugin, PluginSettingTab, Setting } from "obsidian";

import {
  PluginSettings,
  SYNC_MODES,
  SettingsValidationResult,
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
      text: "Configure this Obsidian vault as a Haze Sync Server client. Sync execution is intentionally disabled until later phases add API and vault behavior.",
    });

    this.renderValidation(validatePluginSettings(this.controller.settings));
    this.renderConnectionSettings();
    this.renderModeSettings();
    this.renderSafetySettings();
  }

  private renderConnectionSettings(): void {
    new Setting(this.containerEl)
      .setName("Server URL")
      .setDesc("Base URL for the Haze Sync Server. Tokens are only intended for this configured origin.")
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
      .setDesc("Mode is stored locally for future sync phases. This phase does not perform server calls or vault mutations.")
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

  private renderSafetySettings(): void {
    new Setting(this.containerEl)
      .setName("Protect local changes")
      .setDesc("Keep local dirty-file protection enabled for future remote materialization phases.")
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
      .setDesc("Require explicit user confirmation before future local delete handling.")
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
      .setDesc("Keep UI messaging honest about Obsidian mobile and background execution limits.")
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
