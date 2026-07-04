import { Notice, Plugin } from "obsidian";

export default class HazeSyncPlugin extends Plugin {
  async onload(): Promise<void> {
    new Notice("Haze Sync plugin loaded");
  }

  onunload(): void {
  }
}
