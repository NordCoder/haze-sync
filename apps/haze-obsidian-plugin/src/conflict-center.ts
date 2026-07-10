import type {
  ConflictDto,
  ConflictResolutionAction,
  HazeSyncApiClient,
} from "./api-client";
import type { BaseRevisionState } from "./base-revision-store";
import { sanitizeStatusMessage } from "./safe-text";

export interface ConflictActionDefinition {
  action: ConflictResolutionAction;
  label: string;
  description: string;
  confirmation: string | null;
  available: boolean;
  unavailableReason?: string;
}

export const CONFLICT_ACTION_DEFINITIONS: readonly ConflictActionDefinition[] = [
  {
    action: "accept_current",
    label: "Accept current",
    description: "Keep the current authoritative server version and close the competing conflict revision.",
    confirmation: "The competing conflict revision may stop being available as an open conflict after the server confirms this action.",
    available: true,
  },
  {
    action: "accept_conflict",
    label: "Accept conflict",
    description: "Promote the conflicting revision through the server conflict-resolution endpoint.",
    confirmation: null,
    available: false,
    unavailableReason: "Reserved by the API contract; the current Server does not execute this promotion flow.",
  },
  {
    action: "keep_both",
    label: "Keep both",
    description: "Ask the server to preserve both versions using its supported conflict policy.",
    confirmation: null,
    available: true,
  },
  {
    action: "mark_resolved",
    label: "Mark resolved",
    description: "Close the conflict without choosing a version in this client.",
    confirmation: "Use this only when the conflict has already been handled elsewhere. The conflict will no longer appear as open after server confirmation.",
    available: true,
  },
] as const;

export interface ConflictCenterItem {
  conflict: ConflictDto;
  originalPath: string;
  conflictPath: string;
  status: string;
  sourceAdapter: string;
  policyApplied: string;
  createdAt: string;
  updatedAt: string;
}

export interface ConflictResolutionResult {
  serverConfirmed: boolean;
  baseUpdated: boolean;
  baseRevisionState: BaseRevisionState;
  status: "resolved" | "rejected";
  message: string;
}

export async function loadOpenConflictItems(
  client: HazeSyncApiClient,
  secrets: readonly string[] = [],
): Promise<ConflictCenterItem[]> {
  const response = await client.getConflicts({ status: "open" });
  const redactionSecrets = Array.from(secrets);

  return response.conflicts
    .filter((conflict) => conflict.status === "open")
    .map((conflict) => toConflictCenterItem(conflict, redactionSecrets))
    .sort(compareConflictItems);
}

export async function resolveConflictThroughServer(
  client: HazeSyncApiClient,
  item: ConflictCenterItem,
  action: ConflictResolutionAction,
  idempotencyKey: string,
  baseRevisionState: BaseRevisionState,
): Promise<ConflictResolutionResult> {
  const definition = CONFLICT_ACTION_DEFINITIONS.find((candidate) => candidate.action === action);
  if (definition === undefined || !definition.available) {
    throw new Error("Conflict resolution action is not executable by the current Server contract.");
  }

  const response = await client.resolveConflict({
    conflictId: item.conflict.conflict_id,
    resolution: action,
    idempotencyKey,
  });

  if (
    response.conflict_id !== item.conflict.conflict_id ||
    response.resolution !== action
  ) {
    throw new Error("Conflict resolution response did not match the requested conflict action.");
  }

  return {
    serverConfirmed: true,
    baseUpdated: false,
    baseRevisionState,
    status: "resolved",
    message: "The server confirmed the conflict action. Base metadata will refresh during a later sync pass.",
  };
}

function toConflictCenterItem(conflict: ConflictDto, secrets: string[]): ConflictCenterItem {
  return {
    conflict,
    originalPath: safeDisplayText(conflict.original_path, "Path unavailable", 240, secrets),
    conflictPath: safeDisplayText(conflict.conflict_path, "Conflict path unavailable", 240, secrets),
    status: safeDisplayText(conflict.status, "Unknown", 48, secrets),
    sourceAdapter: safeDisplayText(conflict.source_adapter_id, "Not provided", 128, secrets),
    policyApplied: safeDisplayText(conflict.policy_applied, "Not provided", 128, secrets),
    createdAt: safeTimestamp(conflict.created_at),
    updatedAt: safeTimestamp(conflict.updated_at),
  };
}

function compareConflictItems(left: ConflictCenterItem, right: ConflictCenterItem): number {
  return (
    left.originalPath.localeCompare(right.originalPath) ||
    left.conflict.conflict_id.localeCompare(right.conflict.conflict_id)
  );
}

function safeTimestamp(value: string | undefined): string {
  if (value === undefined || value.trim().length === 0) {
    return "Not provided";
  }

  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) {
    return "Unavailable";
  }

  return new Date(timestamp).toLocaleString();
}

function safeDisplayText(
  value: string | undefined,
  fallback: string,
  maxLength: number,
  secrets: string[],
): string {
  if (value === undefined) {
    return fallback;
  }

  const normalized = sanitizeStatusMessage(value, secrets);
  if (normalized.length === 0) {
    return fallback;
  }

  return normalized.length <= maxLength ? normalized : `${normalized.slice(0, maxLength - 1)}…`;
}
