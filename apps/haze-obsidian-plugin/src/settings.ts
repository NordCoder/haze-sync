export const SYNC_MODES = [
  "disabled",
  "pull_only",
  "push_only",
  "bidirectional",
  "dry_run",
] as const;

export type SyncMode = (typeof SYNC_MODES)[number];

export interface SafetyToggles {
  protectLocalChanges: boolean;
  confirmBeforeDelete: boolean;
  showMobileBackgroundWarning: boolean;
}

export interface PluginSettings {
  serverUrl: string;
  adapterId: string;
  authToken: string;
  syncMode: SyncMode;
  safety: SafetyToggles;
}

export interface SettingsValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
  normalized: PluginSettings;
}

const ADAPTER_ID_PATTERN = /^[A-Za-z0-9._:-]{1,128}$/;

export function createDefaultPluginSettings(): PluginSettings {
  return {
    serverUrl: "",
    adapterId: "obsidian-plugin",
    authToken: "",
    syncMode: "disabled",
    safety: {
      protectLocalChanges: true,
      confirmBeforeDelete: true,
      showMobileBackgroundWarning: true,
    },
  };
}

export const DEFAULT_PLUGIN_SETTINGS = createDefaultPluginSettings();

export function isSyncMode(value: unknown): value is SyncMode {
  return typeof value === "string" && SYNC_MODES.includes(value as SyncMode);
}

export function mergePluginSettings(rawSettings: unknown): PluginSettings {
  const defaults = createDefaultPluginSettings();

  if (!isRecord(rawSettings)) {
    return defaults;
  }

  const rawSafety = isRecord(rawSettings.safety) ? rawSettings.safety : {};

  const merged: PluginSettings = {
    serverUrl: readString(rawSettings.serverUrl, defaults.serverUrl),
    adapterId: readString(rawSettings.adapterId, defaults.adapterId),
    authToken: normalizeAuthToken(readString(rawSettings.authToken, defaults.authToken)),
    syncMode: isSyncMode(rawSettings.syncMode) ? rawSettings.syncMode : defaults.syncMode,
    safety: {
      protectLocalChanges: readBoolean(
        rawSafety.protectLocalChanges,
        defaults.safety.protectLocalChanges,
      ),
      confirmBeforeDelete: readBoolean(
        rawSafety.confirmBeforeDelete,
        defaults.safety.confirmBeforeDelete,
      ),
      showMobileBackgroundWarning: readBoolean(
        rawSafety.showMobileBackgroundWarning,
        defaults.safety.showMobileBackgroundWarning,
      ),
    },
  };

  return validatePluginSettings(merged).normalized;
}

export function validatePluginSettings(settings: PluginSettings): SettingsValidationResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  const serverUrlResult = normalizeServerUrl(settings.serverUrl);
  if (serverUrlResult.error !== undefined) {
    errors.push(serverUrlResult.error);
  }
  if (serverUrlResult.warning !== undefined) {
    warnings.push(serverUrlResult.warning);
  }

  const adapterId = settings.adapterId.trim();
  if (adapterId.length === 0) {
    errors.push("Adapter identity is required.");
  } else if (!ADAPTER_ID_PATTERN.test(adapterId)) {
    errors.push(
      "Adapter identity may contain only letters, numbers, dots, underscores, colons, and hyphens.",
    );
  }

  if (!isSyncMode(settings.syncMode)) {
    errors.push("Sync mode is invalid.");
  }

  const normalized: PluginSettings = {
    serverUrl: serverUrlResult.url,
    adapterId,
    authToken: normalizeAuthToken(settings.authToken),
    syncMode: isSyncMode(settings.syncMode) ? settings.syncMode : "disabled",
    safety: {
      protectLocalChanges: Boolean(settings.safety.protectLocalChanges),
      confirmBeforeDelete: Boolean(settings.safety.confirmBeforeDelete),
      showMobileBackgroundWarning: Boolean(settings.safety.showMobileBackgroundWarning),
    },
  };

  return {
    valid: errors.length === 0,
    errors,
    warnings,
    normalized,
  };
}

export function normalizeServerUrl(value: string): {
  url: string;
  error?: string;
  warning?: string;
} {
  const trimmed = value.trim();

  if (trimmed.length === 0) {
    return {
      url: "",
      warning: "Server URL is not configured.",
    };
  }

  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    return {
      url: trimmed,
      error: "Server URL must be a valid http:// or https:// URL.",
    };
  }

  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    return {
      url: trimmed,
      error: "Server URL must use http:// or https://.",
    };
  }

  if (parsed.username.length > 0 || parsed.password.length > 0) {
    return {
      url: trimmed,
      error: "Server URL must not include embedded credentials.",
    };
  }

  if (parsed.search.length > 0 || parsed.hash.length > 0) {
    return {
      url: trimmed,
      error: "Server URL must not include query strings or fragments.",
    };
  }

  const pathname = parsed.pathname.replace(/\/+$/u, "");
  const normalizedPath = pathname === "" ? "" : pathname;

  return {
    url: `${parsed.origin}${normalizedPath}`,
  };
}

export function normalizeAuthToken(value: string): string {
  return value.trim();
}

export function redactToken(value: string): string {
  return normalizeAuthToken(value).length === 0 ? "not configured" : "••••••••";
}

export function tokenInputPlaceholder(value: string): string {
  return normalizeAuthToken(value).length === 0
    ? "Paste adapter token"
    : "Token saved; paste a new token to replace it";
}

export function syncModeLabel(mode: SyncMode): string {
  switch (mode) {
    case "disabled":
      return "Disabled";
    case "pull_only":
      return "Pull only";
    case "push_only":
      return "Push only";
    case "bidirectional":
      return "Bidirectional";
    case "dry_run":
      return "Dry run";
  }
}

export function settingsAreReady(settings: PluginSettings): boolean {
  const validation = validatePluginSettings(settings);

  return (
    validation.valid &&
    validation.normalized.serverUrl.length > 0 &&
    validation.normalized.authToken.length > 0 &&
    validation.normalized.adapterId.length > 0
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readString(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

function readBoolean(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}
