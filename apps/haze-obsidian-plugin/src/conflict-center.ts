import type {
  ConflictDto,
  ConflictResolutionAction,
  HazeSyncApiClient,
  ResolveConflictResponseDto,
  RevisionId,
} from "./api-client";
import type { BaseRevisionState } from "./base-revision-store";
import { markRemoteBaseRevision } from "./base-revision-store";
import { classifyVaultPath } from "./vault-paths";

export interface ConflictActionDefinition {
  action: ConflictResolutionAction;
  label: string;
  description: string;
  confirmation: string | null;
}

export const CONFLICT_ACTION_DEFINITIONS: readonly ConflictActionDefinition[] = [
  {
    action: "accept_current",
    label: "Accept current",
    description: "Keep the current authoritative server version and close the competing conflict revision.",
    confirmation: "The competing conflict revision may stop being available as an open conflict after the server confirms this action.",
  },
  {
    action: "accept_conflict",
    label: "Accept conflict",
    description: "Promote the conflicting revision through the server conflict-resolution endpoint.",
    confirmation: "The current authoritative server content may be replaced after the server confirms this action.",
  },
  {
    action: "keep_both",
    label: "Keep both",
    description: "Ask the server to preserve both versions using its supported conflict policy.",
    confirmation: null,
  },
  {
    action: "mark_resolved",
    label: "Mark resolved",
    description: "Close the conflict without choosing a version in this client.",
    confirmation: "Use this only when the conflict has already been handled elsewhere. The conflict will no longer appear as open after server confirmation.",
  },
] as const;

export interface ConflictCenterItem {
  conflict: ConflictDto;
  originalPath: string;
  conflictPath: string;
  status: string;
  sourceAdapter: string;
  createdAt: string;
  resolvedAt: string;
}

export interface ConflictResolutionResult {
  serverConfirmed: boolean;
  baseUpdated: boolean;
  baseRevisionState: BaseRevisionState;
  status: "resolved" | "kept_both" | "rejected";
  message: string;
}

export async function loadOpenConflictItems(client: HazeSyncApiClient): Promise<ConflictCenterItem[]> {
  const response = await client.getConflicts({ status: "open" });

  return response.conflicts
    .filter((conflict) => conflict.status === "open")
    .map(toConflictCenterItem)
    .sort(compareConflictItems);
}

export async function resolveConflictThroughServer(
  client: HazeSyncApiClient,
  item: ConflictCenterItem,
  action: ConflictResolutionAction,
  idempotencyKey: string,
  baseRevisionState: BaseRevisionState,
  observedAt: string,
): Promise<ConflictResolutionResult> {
  const response = await client.resolveConflict({
    conflictId: item.conflict.id,
    action,
    idempotencyKey,
  });

  if (response.conflict_id !== item.conflict.id) {
    throw new Error("Conflict resolution response did not match the requested conflict.");
  }

  if (!isConfirmedResolution(response)) {
    return {
      serverConfirmed: false,
      baseUpdated: false,
      baseRevisionState,
      status: "rejected",
      message: "The server did not confirm the conflict action.",
    };
  }

  const revisionId = resolvedRevisionId(response, item.conflict, action);
  const path = classifyVaultPath(item.conflict.original_path);
  if (!path.included || revisionId === null) {
    return {
      serverConfirmed: true,
      baseUpdated: false,
      baseRevisionState,
      status: response.status,
      message: "The server confirmed the conflict action. Base metadata will refresh during a later sync pass.",
    };
  }

  return {
    serverConfirmed: true,
    baseUpdated: true,
    baseRevisionState: markRemoteBaseRevision(
      baseRevisionState,
      path.path,
      revisionId,
      null,
      observedAt,
    ),
    status: response.status,
    message: "The server confirmed the conflict action and local base revision metadata was refreshed.",
  };
}

function toConflictCenterItem(conflict: ConflictDto): ConflictCenterItem {
  return {
    conflict,
    originalPath: safeDisplayText(conflict.original_path, "Path unavailable", 240),
    conflictPath: safeDisplayText(conflict.conflict_path, "Server-managed conflict copy (path not provided)", 240),
    status: safeDisplayText(conflict.status, "Unknown", 48),
    sourceAdapter: safeDisplayText(conflict.source_adapter_id, "Not provided", 128),
    createdAt: safeTimestamp(conflict.created_at),
    resolvedAt: safeTimestamp(conflict.resolved_at),
  };
}

function compareConflictItems(left: ConflictCenterItem, right: ConflictCenterItem): number {
  return left.originalPath.localeCompare(right.originalPath) || left.conflict.id.localeCompare(right.conflict.id);
}

function isConfirmedResolution(
  response: ResolveConflictResponseDto,
): response is ResolveConflictResponseDto & { status: "resolved" | "kept_both" } {
  return response.status === "resolved" || response.status === "kept_both";
}

function resolvedRevisionId(
  response: ResolveConflictResponseDto,
  conflict: ConflictDto,
  action: ConflictResolutionAction,
): RevisionId | null {
  if (response.revision_id !== undefined && response.revision_id !== null) {
    return response.revision_id;
  }

  if (action === "accept_conflict") {
    return conflict.conflict_revision_id ?? null;
  }

  return conflict.current_revision_id ?? null;
}

function safeTimestamp(value: string | null | undefined): string {
  if (value === undefined || value === null || value.trim().length === 0) {
    return "Not provided";
  }

  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) {
    return "Unavailable";
  }

  return new Date(timestamp).toLocaleString();
}

function safeDisplayText(value: string | null | undefined, fallback: string, maxLength: number): string {
  if (value === undefined || value === null) {
    return fallback;
  }

  const normalized = value.replace(/[\r\n\t\u0000-\u001f\u007f]+/gu, " ").trim();
  if (normalized.length === 0) {
    return fallback;
  }

  return normalized.length <= maxLength ? normalized : `${normalized.slice(0, maxLength - 1)}…`;
}
