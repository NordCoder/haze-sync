import { PluginSettings, validatePluginSettings } from "../settings";

export interface ApiClientConfig {
  serverUrl: string;
  authToken: string;
}

export interface RequestHeadersInput {
  idempotencyKey?: string;
  contentHash?: string;
  baseRevisionId?: string | null;
  contentType?: string;
  accept?: string;
}

export function apiClientConfigFromSettings(settings: PluginSettings): ApiClientConfig {
  const validation = validatePluginSettings(settings);

  return {
    serverUrl: validation.normalized.serverUrl,
    authToken: validation.normalized.authToken,
  };
}

export function buildApiUrl(serverUrl: string, path: string, query?: Record<string, string | number | undefined>): URL {
  const normalizedServerUrl = serverUrl.replace(/\/+$/u, "");
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  const url = new URL(`${normalizedServerUrl}${normalizedPath}`);

  for (const [key, value] of Object.entries(query ?? {})) {
    if (value !== undefined) {
      url.searchParams.set(key, String(value));
    }
  }

  return url;
}

export function buildFilePathUrl(serverUrl: string, pathPrefix: string, vaultPath: string): URL {
  const encodedPath = encodeVaultPath(vaultPath);
  const normalizedPrefix = pathPrefix.replace(/\/+$/u, "");

  return buildApiUrl(serverUrl, `${normalizedPrefix}/${encodedPath}`);
}

export function encodeVaultPath(vaultPath: string): string {
  return vaultPath
    .split("/")
    .filter((segment) => segment.length > 0)
    .map((segment) => encodeURIComponent(segment))
    .join("/");
}

export function buildApiHeaders(authToken: string, input: RequestHeadersInput = {}): Headers {
  const headers = new Headers();
  headers.set("Accept", input.accept ?? "application/json");

  if (input.contentType !== undefined) {
    headers.set("Content-Type", input.contentType);
  }

  if (authToken.trim().length > 0) {
    headers.set("Authorization", `Bearer ${authToken.trim()}`);
  }

  if (input.idempotencyKey !== undefined) {
    headers.set("Idempotency-Key", input.idempotencyKey);
  }

  if (input.contentHash !== undefined) {
    headers.set("X-Content-SHA256", input.contentHash);
  }

  if ("baseRevisionId" in input) {
    headers.set("X-Base-Revision-Id", input.baseRevisionId ?? "null");
  }

  return headers;
}

export function assertConfiguredServer(config: ApiClientConfig): void {
  if (config.serverUrl.trim().length === 0) {
    throw new Error("Haze Sync Server URL is not configured.");
  }
}

export function assertSameOrigin(requestUrl: URL, serverUrl: string): void {
  const configuredOrigin = new URL(serverUrl).origin;
  if (requestUrl.origin !== configuredOrigin) {
    throw new Error("Refusing to send auth token to a non-configured origin.");
  }
}
