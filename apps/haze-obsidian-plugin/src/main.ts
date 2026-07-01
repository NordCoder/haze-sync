import { Notice, Plugin } from "obsidian";

export default class HazeSyncPlugin extends Plugin {
  async onload(): Promise<void> {
    new Notice("Haze Sync skeleton loaded");
  }

  onunload(): void {
    // Wave 0 skeleton only. Runtime sync cleanup will be added in later phases.
  }
}
