import { sanitizeStatusMessage } from "../safe-text";

import { isErrorResponseDto } from "./validators";

export type ApiErrorCategory =
  | "configuration"
  | "offline"
  | "unauthorized"
  | "forbidden"
  | "not_found"
  | "conflict"
  | "rejected"
  | "rate_limited"
  | "server_unavailable"
  | "invalid_response"
  | "internal";

export interface SafeApiErrorSummary {
  category: ApiErrorCategory;
  message: string;
  status?: number;
  endpoint?: string;
}

export class ApiClientError extends Error {
  readonly category: ApiErrorCategory;
  readonly status?: number;
  readonly endpoint?: string;

  constructor(summary: SafeApiErrorSummary) {
    super(summary.message);
    this.name = "ApiClientError";
    this.category = summary.category;
    this.status = summary.status;
    this.endpoint = summary.endpoint;
  }

  toSafeSummary(): SafeApiErrorSummary {
    return {
      category: this.category,
      message: this.message,
      status: this.status,
      endpoint: this.endpoint,
    };
  }
}

export function mapHttpStatusToCategory(status: number): ApiErrorCategory {
  if (status === 401) {
    return "unauthorized";
  }
  if (status === 403) {
    return "forbidden";
  }
  if (status === 404) {
    return "not_found";
  }
  if (status === 409) {
    return "conflict";
  }
  if (status === 422) {
    return "rejected";
  }
  if (status === 429) {
    return "rate_limited";
  }
  if (status >= 500) {
    return "server_unavailable";
  }

  return "internal";
}

export function createHttpError(
  status: number,
  endpoint: string,
  payload: unknown,
  secrets: readonly string[] = [],
): ApiClientError {
  return new ApiClientError({
    category: mapHttpStatusToCategory(status),
    status,
    endpoint,
    message: safeErrorMessage(status, payload, secrets),
  });
}

export function createConfigurationError(message: string, endpoint?: string): ApiClientError {
  return new ApiClientError({
    category: "configuration",
    endpoint,
    message: sanitizeApiErrorMessage(message),
  });
}

export function createInvalidResponseError(endpoint: string): ApiClientError {
  return new ApiClientError({
    category: "invalid_response",
    endpoint,
    message: "Server returned an invalid response.",
  });
}

export function createOfflineError(endpoint: string): ApiClientError {
  return new ApiClientError({
    category: "offline",
    endpoint,
    message: "Could not reach Haze Sync Server.",
  });
}

function safeErrorMessage(status: number, payload: unknown, secrets: readonly string[]): string {
  if (isErrorResponseDto(payload)) {
    return sanitizeApiErrorMessage(payload.error.message, secrets);
  }

  switch (mapHttpStatusToCategory(status)) {
    case "configuration":
      return "Haze Sync API client is not configured correctly.";
    case "unauthorized":
      return "Authentication failed. Check the adapter token.";
    case "forbidden":
      return "This adapter is not allowed to perform the requested operation.";
    case "not_found":
      return "The requested file or resource was not found.";
    case "conflict":
      return "The server rejected the request because it conflicts with current state.";
    case "rejected":
      return "The server rejected the request.";
    case "rate_limited":
      return "The server asked the plugin to slow down.";
    case "server_unavailable":
      return "Haze Sync Server is unavailable.";
    case "offline":
    case "invalid_response":
    case "internal":
      return "Haze Sync request failed.";
  }
}

function sanitizeApiErrorMessage(message: string, secrets: readonly string[] = []): string {
  return sanitizeStatusMessage(message, secrets)
    .replace(/\b[A-Za-z]:\\(?:[^\\\s]+\\)*[^\\\s]*/gu, "[local path redacted]")
    .replace(/(^|[\s(])\/(?:Users|home|var|tmp|private|srv)\/(?:[^\s),;]+\/?)+/gu, "$1[local path redacted]");
}
