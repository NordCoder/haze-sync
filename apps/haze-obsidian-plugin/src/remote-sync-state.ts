import type { ChangeDto, ContentHash, RevisionId, VaultPath } from "./api-client";

export type RemoteConflictReason =
  | "dirty_local_file"
  | "download_missing"
  | "hash_mismatch"
  | "missing_content_hash"
  | "missing_revision"
  | "non_file_path"
  | "revision_mismatch"
  | "server_conflict"
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
  remoteConflictId?: string;
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

export interface RemoteMetadataChangeResult {
  state: RemoteSyncState;
  conflictRecorded: boolean;
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

export function applyRemoteMetadataChange(
  state: RemoteSyncState,
  change: ChangeDto,
  observedAt: string,
): RemoteMetadataChangeResult {
  switch (change.kind) {
    case "conflict_created":
      return {
        state: advanceRemoteCursor(
          recordRemoteConflict(state, change, "server_conflict", observedAt),
          change.seq,
          observedAt,
        ),
        conflictRecorded: true,
      };
    case "conflict_resolved":
      return {
        state: advanceRemoteCursor(
          clearRemoteConflictForChange(state, change, observedAt),
          change.seq,
          observedAt,
        ),
        conflictRecorded: false,
      };
    case "backup_created":
      return {
        state: advanceRemoteCursor(state, change.seq, observedAt),
        conflictRecorded: false,
      };
    case "upsert_file":
    case "restore_file":
    case "delete_file":
      throw new Error("Remote metadata transition requires a metadata-only change kind.");
  }
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
        remoteConflictId: change.conflict_id,
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

function clearRemoteConflictForChange(
  state: RemoteSyncState,
  change: ChangeDto,
  observedAt: string,
): RemoteSyncState {
  const conflicts = { ...state.conflicts };
  let directMatchFound = false;

  if (change.conflict_id !== undefined) {
    for (const [key, record] of Object.entries(conflicts)) {
      if (key === change.conflict_id || record.remoteConflictId === change.conflict_id) {
        delete conflicts[key];
        directMatchFound = true;
      }
    }
  }

  if (!directMatchFound) {
    const legacyMatches = Object.entries(conflicts).filter(
      ([, record]) =>
        record.remoteConflictId === undefined &&
        record.kind === "conflict_created" &&
        record.path === change.path,
    );
    if (legacyMatches.length === 1) {
      delete conflicts[legacyMatches[0][0]];
    }
  }

  return {
    ...state,
    conflicts,
    lastPullAt: observedAt,
  };
}

function conflictIdForChange(change: ChangeDto): string {
  return change.conflict_id ?? `${change.seq}:${change.path}`;
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
    isChangeKind(value.kind) &&
    isRemoteConflictReason(value.reason) &&
    typeof value.detectedAt === "string" &&
    (value.remoteConflictId === undefined || typeof value.remoteConflictId === "string") &&
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
    value === "server_conflict" ||
    value === "unsupported_change" ||
    value === "unsupported_path" ||
    value === "write_failed"
  );
}

function isChangeKind(value: unknown): value is ChangeDto["kind"] {
  return (
    value === "upsert_file" ||
    value === "delete_file" ||
    value === "restore_file" ||
    value === "conflict_created" ||
    value === "conflict_resolved" ||
    value === "backup_created"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
