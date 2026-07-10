import type {
  ApiErrorCategory,
  ContentHash,
  DeleteFileResponseDto,
  PutFileResponseDto,
  RevisionId,
  VaultPath,
} from "./api-client";
import { normalizeStoredContentHash } from "./content-hash";

export interface BaseRevisionEntry {
  path: VaultPath;
  revisionId: RevisionId | null;
  contentHash: ContentHash | null;
  serverDeleted: boolean;
  updatedAt: string;
}

export interface BaseRevisionState {
  byPath: Record<VaultPath, BaseRevisionEntry>;
}

export type ServerMutationOutcome =
  | "accepted"
  | "same_content"
  | "not_found"
  | "conflict_saved"
  | "rejected"
  | "unauthorized"
  | "server_unavailable";

export interface BaseStateUpdateResult {
  state: BaseRevisionState;
  outcome: ServerMutationOutcome;
  baseUpdated: boolean;
  conflictId?: string;
  reason?: string;
}

export function createDefaultBaseRevisionState(): BaseRevisionState {
  return { byPath: {} };
}

export function mergeBaseRevisionState(rawState: unknown): BaseRevisionState {
  if (!isRecord(rawState)) {
    return createDefaultBaseRevisionState();
  }

  return {
    byPath: readBaseRevisionEntries(rawState.byPath),
  };
}

export function getBaseRevisionId(state: BaseRevisionState, path: VaultPath): RevisionId | null {
  return state.byPath[path]?.revisionId ?? null;
}

export function getBaseContentHash(state: BaseRevisionState, path: VaultPath): ContentHash | null {
  return state.byPath[path]?.contentHash ?? null;
}

export function applyUploadOutcome(
  state: BaseRevisionState,
  response: PutFileResponseDto,
  observedAt: string,
  confirmedContentHash?: ContentHash | null,
): BaseStateUpdateResult {
  switch (response.status) {
    case "accepted":
      return {
        state: upsertBaseRevision(state, {
          path: response.path,
          revisionId: response.revision_id,
          contentHash: confirmedContentHash ?? state.byPath[response.path]?.contentHash ?? null,
          serverDeleted: false,
          updatedAt: observedAt,
        }),
        outcome: "accepted",
        baseUpdated: true,
      };
    case "ignored":
      return skippedUpdate(
        state,
        "same_content",
        undefined,
        "Server confirmed identical content without returning current revision metadata.",
      );
    case "conflict_saved":
      return skippedUpdate(state, "conflict_saved", response.conflict_id);
    case "rejected":
      return skippedUpdate(state, "rejected", undefined, response.reason);
  }
}

export function applyDeleteOutcome(
  state: BaseRevisionState,
  response: DeleteFileResponseDto,
  observedAt: string,
): BaseStateUpdateResult {
  switch (response.status) {
    case "tombstoned":
      return confirmedDelete(state, response.path, "accepted", observedAt);
    case "not_found":
      return confirmedDelete(state, response.path, "not_found", observedAt);
    case "rejected":
      return skippedUpdate(state, "rejected", undefined, response.reason);
  }
}

export function skippedUpdateForApiError(
  state: BaseRevisionState,
  category: ApiErrorCategory,
  reason?: string,
): BaseStateUpdateResult {
  return skippedUpdate(state, mutationOutcomeFromApiErrorCategory(category), undefined, reason);
}

export function markRemoteBaseRevision(
  state: BaseRevisionState,
  path: VaultPath,
  revisionId: RevisionId | null,
  contentHash: ContentHash | null,
  observedAt: string,
): BaseRevisionState {
  return upsertBaseRevision(state, {
    path,
    revisionId,
    contentHash,
    serverDeleted: false,
    updatedAt: observedAt,
  });
}

export function markRemoteDelete(
  state: BaseRevisionState,
  path: VaultPath,
  revisionId: RevisionId | null,
  observedAt: string,
): BaseRevisionState {
  return upsertBaseRevision(state, {
    path,
    revisionId,
    contentHash: null,
    serverDeleted: true,
    updatedAt: observedAt,
  });
}

function confirmedDelete(
  state: BaseRevisionState,
  path: VaultPath,
  outcome: "accepted" | "not_found",
  observedAt: string,
): BaseStateUpdateResult {
  return {
    state: upsertBaseRevision(state, {
      path,
      revisionId: state.byPath[path]?.revisionId ?? null,
      contentHash: null,
      serverDeleted: true,
      updatedAt: observedAt,
    }),
    outcome,
    baseUpdated: true,
  };
}

function upsertBaseRevision(state: BaseRevisionState, entry: BaseRevisionEntry): BaseRevisionState {
  return {
    byPath: {
      ...state.byPath,
      [entry.path]: entry,
    },
  };
}

function skippedUpdate(
  state: BaseRevisionState,
  outcome: ServerMutationOutcome,
  conflictId?: string,
  reason?: string,
): BaseStateUpdateResult {
  return {
    state,
    outcome,
    baseUpdated: false,
    conflictId,
    reason,
  };
}

function mutationOutcomeFromApiErrorCategory(category: ApiErrorCategory): ServerMutationOutcome {
  switch (category) {
    case "unauthorized":
    case "forbidden":
      return "unauthorized";
    case "offline":
    case "rate_limited":
    case "server_unavailable":
      return "server_unavailable";
    case "conflict":
      return "conflict_saved";
    case "not_found":
      return "not_found";
    case "configuration":
    case "invalid_response":
    case "rejected":
    case "internal":
      return "rejected";
  }
}

function readBaseRevisionEntries(value: unknown): Record<VaultPath, BaseRevisionEntry> {
  if (!isRecord(value)) {
    return {};
  }

  const result: Record<VaultPath, BaseRevisionEntry> = {};
  for (const [key, item] of Object.entries(value)) {
    const entry = readBaseRevisionEntry(item);
    if (entry !== undefined) {
      result[key] = entry;
    }
  }
  return result;
}

function readBaseRevisionEntry(value: unknown): BaseRevisionEntry | undefined {
  if (
    !isRecord(value) ||
    typeof value.path !== "string" ||
    !(typeof value.revisionId === "string" || value.revisionId === null) ||
    typeof value.serverDeleted !== "boolean" ||
    typeof value.updatedAt !== "string"
  ) {
    return undefined;
  }

  const contentHash = value.contentHash === null ? null : normalizeStoredContentHash(value.contentHash);
  if (contentHash === undefined) {
    return undefined;
  }

  return {
    path: value.path,
    revisionId: value.revisionId,
    contentHash,
    serverDeleted: value.serverDeleted,
    updatedAt: value.updatedAt,
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
