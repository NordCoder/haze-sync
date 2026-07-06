import { Notice } from "obsidian";

export type StatusLevel = "info" | "success" | "warning" | "error";

const DEFAULT_NOTICE_TIMEOUT_MS = 6_000;
const MAX_STATUS_MESSAGE_LENGTH = 220;
const SECRET_PATTERNS: RegExp[] = [
  /\bBearer\s+[A-Za-z0-9._~+/=-]+/giu,
  /\bAuthorization\s*[:=]\s*[^\s,;]+/giu,
  /\btoken\s*[:=]\s*[^\s,;]+/giu,
  /https?:\/\/[^\s/:@]+:[^\s/@]+@/giu,
];

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

export function sanitizeStatusMessage(message: string, secrets: string[] = []): string {
  let sanitized = message.replace(/[\r\n\t]+/gu, " ").trim();

  for (const pattern of SECRET_PATTERNS) {
    sanitized = sanitized.replace(pattern, "[redacted]");
  }

  for (const secret of secrets) {
    const normalizedSecret = secret.trim();
    if (normalizedSecret.length < 4) {
      continue;
    }

    sanitized = sanitized.replace(new RegExp(escapeRegExp(normalizedSecret), "gu"), "[redacted]");
  }

  if (sanitized.length > MAX_STATUS_MESSAGE_LENGTH) {
    return `${sanitized.slice(0, MAX_STATUS_MESSAGE_LENGTH - 1)}…`;
  }

  return sanitized;
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

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}
