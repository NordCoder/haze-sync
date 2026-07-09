import type {
  ApiErrorCategory,
  ContentHash,
  DeleteFileResponseDto,
  PutFileResponseDto,
  RevisionId,
  VaultPath,
} from "./api-client";

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
  return {
    byPath: {},
  };
}

export function mergeBaseRevisionState(rawState: unknown): BaseRevisionState {
  if (!isRecord(rawState)) {
    return createDefaultBaseRevisionState();
  }

  return {
    byPath: readRecord(rawState.byPath, isBaseRevisionEntry),
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
  const outcome = classifyServerStatus(response.status);
  if (outcome !== "accepted" && outcome !== "same_content") {
    return skippedUpdate(state, outcome, response.conflict_id ?? undefined);
  }

  if (response.revision_id === undefined || response.revision_id === null) {
    return skippedUpdate(state, outcome, undefined, "Server did not return a revision for accepted upload outcome.");
  }

  return {
    state: upsertBaseRevision(state, {
      path: response.path,
      revisionId: response.revision_id,
      contentHash: response.content_hash ?? confirmedContentHash ?? state.byPath[response.path]?.contentHash ?? null,
      serverDeleted: false,
      updatedAt: observedAt,
    }),
    outcome,
    baseUpdated: true,
  };
}

export function applyDeleteOutcome(
  state: BaseRevisionState,
  response: DeleteFileResponseDto,
  observedAt: string,
): BaseStateUpdateResult {
  const outcome = classifyServerStatus(response.status);
  if (outcome !== "accepted" && outcome !== "same_content") {
    return skippedUpdate(state, outcome, response.conflict_id ?? undefined);
  }

  return {
    state: upsertBaseRevision(state, {
      path: response.path,
      revisionId: response.revision_id ?? state.byPath[response.path]?.revisionId ?? null,
      contentHash: null,
      serverDeleted: true,
      updatedAt: observedAt,
    }),
    outcome,
    baseUpdated: true,
  };
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

function classifyServerStatus(status: string): ServerMutationOutcome {
  switch (status) {
    case "accepted":
    case "tombstoned":
    case "resolved":
      return "accepted";
    case "same_content":
      return "same_content";
    case "conflict_saved":
      return "conflict_saved";
    case "unauthorized":
      return "unauthorized";
    case "server_unavailable":
      return "server_unavailable";
    case "rejected":
    default:
      return "rejected";
  }
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
    case "configuration":
    case "invalid_response":
    case "not_found":
    case "rejected":
    case "internal":
      return "rejected";
  }
}

function readRecord<T>(value: unknown, predicate: (item: unknown) => item is T): Record<string, T> {
  if (!isRecord(value)) {
    return {};
  }

  const result: Record<string, T> = {};
  for (const [key, item] of Object.entries(value)) {
    if (predicate(item)) {
      result[key] = item;
    }
  }

  return result;
}

function isBaseRevisionEntry(value: unknown): value is BaseRevisionEntry {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.path === "string" &&
    (typeof value.revisionId === "string" || value.revisionId === null) &&
    (typeof value.contentHash === "string" || value.contentHash === null) &&
    typeof value.serverDeleted === "boolean" &&
    typeof value.updatedAt === "string"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
