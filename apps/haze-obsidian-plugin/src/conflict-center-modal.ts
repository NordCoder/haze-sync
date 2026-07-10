import { App, Modal, Setting } from "obsidian";

import type { ConflictResolutionAction } from "./api-client";
import {
  CONFLICT_ACTION_DEFINITIONS,
  ConflictActionDefinition,
  ConflictCenterItem,
  ConflictResolutionResult,
} from "./conflict-center";
import { generateConflictResolutionIdempotencyKey } from "./idempotency-keys";

export interface ConflictCenterController {
  loadOpenConflicts(): Promise<ConflictCenterItem[]>;
  resolveConflict(
    item: ConflictCenterItem,
    action: ConflictResolutionAction,
    idempotencyKey: string,
  ): Promise<ConflictResolutionResult>;
  canResolveConflicts(): boolean;
}

interface PendingConfirmation {
  item: ConflictCenterItem;
  definition: ConflictActionDefinition;
  idempotencyKey: string;
}

export class ConflictCenterModal extends Modal {
  private items: ConflictCenterItem[] = [];
  private loading = false;
  private busyKey?: string;
  private statusMessage?: string;
  private pendingConfirmation?: PendingConfirmation;
  private readonly actionKeys = new Map<string, string>();

  constructor(app: App, private readonly controller: ConflictCenterController) {
    super(app);
  }

  onOpen(): void {
    void this.reload();
  }

  onClose(): void {
    this.contentEl.empty();
    this.actionKeys.clear();
  }

  private async reload(preserveMessage = false): Promise<void> {
    this.loading = true;
    this.pendingConfirmation = undefined;
    if (!preserveMessage) {
      this.statusMessage = undefined;
    }
    this.render();

    try {
      this.items = await this.controller.loadOpenConflicts();
    } catch {
      this.items = [];
      this.statusMessage = "Could not load open conflicts. Check the server connection and try again.";
    } finally {
      this.loading = false;
      this.render();
    }
  }

  private render(): void {
    this.contentEl.empty();
    this.contentEl.createEl("h2", { text: "Haze Sync conflict center" });
    this.contentEl.createEl("p", {
      text: "Conflicts are resolved only through Haze Sync Server. This client does not merge or discard note content locally on its own.",
    });

    if (!this.controller.canResolveConflicts()) {
      this.contentEl.createEl("p", {
        text: "Conflict actions are disabled in the current sync mode. Open conflicts remain available for inspection.",
      });
    }

    if (this.statusMessage !== undefined) {
      this.contentEl.createEl("p", { text: this.statusMessage });
    }

    new Setting(this.contentEl)
      .setName("Refresh conflicts")
      .setDesc("Reload the current list of open conflicts from Haze Sync Server.")
      .addButton((button) => {
        button
          .setButtonText("Refresh")
          .setDisabled(this.loading || this.busyKey !== undefined)
          .onClick(() => {
            void this.reload();
          });
      });

    if (this.loading) {
      this.contentEl.createEl("p", { text: "Loading open conflicts…" });
      return;
    }

    if (this.items.length === 0) {
      this.contentEl.createEl("p", { text: "No open conflicts were returned by the server." });
      return;
    }

    this.contentEl.createEl("p", { text: `${this.items.length} open conflict(s).` });
    for (const item of this.items) {
      this.renderConflict(item);
    }
  }

  private renderConflict(item: ConflictCenterItem): void {
    const card = this.contentEl.createDiv({ cls: "haze-sync-conflict-card" });
    card.createEl("h3", { text: item.originalPath });
    this.renderField(card, "Conflict copy", item.conflictPath);
    this.renderField(card, "Status", item.status);
    this.renderField(card, "Source adapter", item.sourceAdapter);
    this.renderField(card, "Created", item.createdAt);
    this.renderField(card, "Resolved", item.resolvedAt);

    const confirmation = this.pendingConfirmation;
    if (confirmation !== undefined && confirmation.item.conflict.id === item.conflict.id) {
      this.renderConfirmation(card, confirmation);
      return;
    }

    for (const definition of CONFLICT_ACTION_DEFINITIONS) {
      new Setting(card)
        .setName(definition.label)
        .setDesc(definition.description)
        .addButton((button) => {
          button
            .setButtonText(definition.label)
            .setDisabled(!this.controller.canResolveConflicts() || this.busyKey !== undefined)
            .onClick(() => {
              void this.requestAction(item, definition);
            });
        });
    }
  }

  private renderConfirmation(container: HTMLElement, confirmation: PendingConfirmation): void {
    container.createEl("p", {
      text: confirmation.definition.confirmation ?? "Confirm this server conflict action.",
    });

    new Setting(container)
      .setName(`Confirm ${confirmation.definition.label}`)
      .setDesc("The request will be sent to Haze Sync Server. Local conflict policy is not applied independently.")
      .addButton((button) => {
        button
          .setButtonText("Cancel")
          .setDisabled(this.busyKey !== undefined)
          .onClick(() => {
            this.pendingConfirmation = undefined;
            this.render();
          });
      })
      .addButton((button) => {
        button
          .setButtonText("Confirm")
          .setCta()
          .setDisabled(this.busyKey !== undefined)
          .onClick(() => {
            void this.executeAction(confirmation);
          });
      });
  }

  private async requestAction(item: ConflictCenterItem, definition: ConflictActionDefinition): Promise<void> {
    if (!this.controller.canResolveConflicts() || this.busyKey !== undefined) {
      return;
    }

    const idempotencyKey = this.idempotencyKeyFor(item, definition.action);
    if (definition.confirmation !== null) {
      this.pendingConfirmation = {
        item,
        definition,
        idempotencyKey,
      };
      this.render();
      return;
    }

    await this.executeAction({
      item,
      definition,
      idempotencyKey,
    });
  }

  private async executeAction(confirmation: PendingConfirmation): Promise<void> {
    const actionKey = this.actionMapKey(confirmation.item, confirmation.definition.action);
    this.busyKey = actionKey;
    this.pendingConfirmation = undefined;
    this.statusMessage = `Submitting ${confirmation.definition.label.toLowerCase()} to Haze Sync Server…`;
    this.render();

    try {
      const result = await this.controller.resolveConflict(
        confirmation.item,
        confirmation.definition.action,
        confirmation.idempotencyKey,
      );
      this.statusMessage = result.message;

      if (result.serverConfirmed) {
        this.actionKeys.delete(actionKey);
        await this.reload(true);
        return;
      }
    } catch {
      this.statusMessage = "The conflict action could not be confirmed. No local conflict resolution was applied.";
    } finally {
      this.busyKey = undefined;
      this.render();
    }
  }

  private idempotencyKeyFor(item: ConflictCenterItem, action: ConflictResolutionAction): string {
    const mapKey = this.actionMapKey(item, action);
    const existing = this.actionKeys.get(mapKey);
    if (existing !== undefined) {
      return existing;
    }

    const created = generateConflictResolutionIdempotencyKey(action);
    this.actionKeys.set(mapKey, created);
    return created;
  }

  private actionMapKey(item: ConflictCenterItem, action: ConflictResolutionAction): string {
    return `${item.conflict.id}:${action}`;
  }

  private renderField(container: HTMLElement, label: string, value: string): void {
    container.createEl("p", { text: `${label}: ${value}` });
  }
}
