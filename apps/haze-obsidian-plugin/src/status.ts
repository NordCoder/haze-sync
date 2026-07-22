import { Notice } from "obsidian";

import { sanitizeStatusMessage } from "./safe-text";

export { sanitizeStatusMessage } from "./safe-text";

export type StatusLevel = "info" | "success" | "warning" | "error";

const DEFAULT_NOTICE_TIMEOUT_MS = 6_000;

export interface StatusReporterOptions {
  statusItem?: HTMLElement;
  getSecrets?: () => string[];
}

export class SafeStatusReporter {
  private readonly statusItem?: HTMLElement;
  private readonly getSecrets: () => string[];

  constructor(options: StatusReporterOptions = {}) {
    this.statusItem = options.statusItem;
    this.getSecrets = options.getSecrets ?? (() => []);
  }

  setStatus(message: string, level: StatusLevel = "info"): void {
    const safeMessage = sanitizeStatusMessage(message, this.getSecrets());
    this.statusItem?.setText(`Haze Sync: ${prefixForLevel(level)}${safeMessage}`);
  }

  notice(message: string, level: StatusLevel = "info", timeout = DEFAULT_NOTICE_TIMEOUT_MS): void {
    const safeMessage = sanitizeStatusMessage(message, this.getSecrets());
    new Notice(`Haze Sync: ${prefixForLevel(level)}${safeMessage}`, timeout);
  }

  dispose(): void {
    this.statusItem?.remove();
  }
}

function prefixForLevel(level: StatusLevel): string {
  switch (level) {
    case "info":
      return "";
    case "success":
      return "OK — ";
    case "warning":
      return "Warning — ";
    case "error":
      return "Error — ";
  }
}
