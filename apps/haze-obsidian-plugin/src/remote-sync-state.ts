import type { ChangeDto, ContentHash, RevisionId, VaultPath } from "./api-client";

export type RemoteConflictReason =
  | "dirty_local_file"
  | "download_missing"
  | "hash_mismatch"
  | "missing_content_hash"
  | "missing_revision"
  | "non_file_path"
  | "revision_mismatch"
  | "unsupported_change"
  | "unsupported_path"
  | "write_failed";

export interface RemoteConflictRecord {
  id: string;
  path: VaultPath;
  sequence: number;
  kind: ChangeDto["kind"];
  reason: RemoteConflictReason;
  detectedAt: string;
  remoteRevisionId?: RevisionId;
  remoteContentHash?: ContentHash;
}

export interface RemoteTombstoneRecord {
  path: VaultPath;
  sequence: number;
  revisionId?: RevisionId | null;
  observedAt: string;
}

export interface RemoteSyncState {
  changeCursor?: number;
  lastPullAt?: string;
  conflicts: Record<string, RemoteConflictRecord>;
  tombstones: Record<VaultPath, RemoteTombstoneRecord>;
}

export function createDefaultRemoteSyncState(): RemoteSyncState {
  return {
    conflicts: {},
    tombstones: {},
  };
}

export function mergeRemoteSyncState(rawState: unknown): RemoteSyncState {
  if (!isRecord(rawState)) {
    return createDefaultRemoteSyncState();
  }

  return {
    changeCursor: readOptionalNumber(rawState.changeCursor),
    lastPullAt: readOptionalString(rawState.lastPullAt),
    conflicts: readRecord(rawState.conflicts, isRemoteConflictRecord),
    tombstones: readRecord(rawState.tombstones, isRemoteTombstoneRecord),
  };
}

export function markRemotePullAttempt(state: RemoteSyncState, observedAt: string): RemoteSyncState {
  return {
    ...state,
    lastPullAt: observedAt,
  };
}

export function advanceRemoteCursor(state: RemoteSyncState, sequence: number, observedAt: string): RemoteSyncState {
  return {
    ...state,
    changeCursor: Math.max(state.changeCursor ?? 0, sequence),
    lastPullAt: observedAt,
  };
}

export function recordRemoteConflict(
  state: RemoteSyncState,
  change: ChangeDto,
  reason: RemoteConflictReason,
  observedAt: string,
): RemoteSyncState {
  const id = conflictIdForChange(change);

  return {
    ...state,
    conflicts: {
      ...state.conflicts,
      [id]: {
        id,
        path: change.path,
        sequence: change.seq,
        kind: change.kind,
        reason,
        detectedAt: observedAt,
        remoteRevisionId: change.revision_id,
        remoteContentHash: change.content_sha256,
      },
    },
    lastPullAt: observedAt,
  };
}

export function recordRemoteTombstone(
  state: RemoteSyncState,
  change: ChangeDto,
  revisionId: RevisionId | null | undefined,
  observedAt: string,
): RemoteSyncState {
  return {
    ...state,
    tombstones: {
      ...state.tombstones,
      [change.path]: {
        path: change.path,
        sequence: change.seq,
        revisionId,
        observedAt,
      },
    },
    lastPullAt: observedAt,
  };
}

function conflictIdForChange(change: ChangeDto): string {
  return `${change.seq}:${change.path}`;
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

function readOptionalNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : undefined;
}

function readOptionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function isRemoteConflictRecord(value: unknown): value is RemoteConflictRecord {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.id === "string" &&
    typeof value.path === "string" &&
    typeof value.sequence === "number" &&
    typeof value.kind === "string" &&
    isRemoteConflictReason(value.reason) &&
    typeof value.detectedAt === "string" &&
    (value.remoteRevisionId === undefined || typeof value.remoteRevisionId === "string") &&
    (value.remoteContentHash === undefined || typeof value.remoteContentHash === "string")
  );
}

function isRemoteTombstoneRecord(value: unknown): value is RemoteTombstoneRecord {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.path === "string" &&
    typeof value.sequence === "number" &&
    (value.revisionId === undefined || typeof value.revisionId === "string" || value.revisionId === null) &&
    typeof value.observedAt === "string"
  );
}

function isRemoteConflictReason(value: unknown): value is RemoteConflictReason {
  return (
    value === "dirty_local_file" ||
    value === "download_missing" ||
    value === "hash_mismatch" ||
    value === "missing_content_hash" ||
    value === "missing_revision" ||
    value === "non_file_path" ||
    value === "revision_mismatch" ||
    value === "unsupported_change" ||
    value === "unsupported_path" ||
    value === "write_failed"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
