import { TFile, Vault } from "obsidian";

import type { ChangeDto, ChangesResponseDto, ContentHash, FileDownload, HazeSyncApiClient, RevisionId } from "./api-client";
import {
  BaseRevisionState,
  getBaseContentHash,
  markRemoteBaseRevision,
  markRemoteDelete,
} from "./base-revision-store";
import { readLocalFileFact, sha256Hex } from "./local-file-facts";
import {
  RemoteConflictReason,
  RemoteSyncState,
  advanceRemoteCursor,
  markRemotePullAttempt,
  recordRemoteConflict,
  recordRemoteTombstone,
} from "./remote-sync-state";
import { classifyVaultPath } from "./vault-paths";

export type RemoteMaterializationStatus =
  | "applied"
  | "conflict_queued"
  | "no_op"
  | "tombstone_recorded";

export interface RemoteMaterializationInput {
  vault: Vault;
  change: ChangeDto;
  download?: FileDownload;
  baseRevisionState: BaseRevisionState;
  remoteSyncState: RemoteSyncState;
  observedAt: string;
  echoSuppressor?: RemoteEchoSuppressor;
}

export interface RemoteMaterializationResult {
  status: RemoteMaterializationStatus;
  baseRevisionState: BaseRevisionState;
  remoteSyncState: RemoteSyncState;
  path: string;
  reason?: RemoteConflictReason;
}

export interface RemotePullOptions {
  limit?: number;
}

export class RemoteEchoSuppressor {
  private readonly suppressedPaths = new Set<string>();

  isSuppressed(path: string): boolean {
    return this.suppressedPaths.has(path);
  }

  async suppressWhile<T>(path: string, action: () => Promise<T>): Promise<T> {
    this.suppressedPaths.add(path);
    try {
      return await action();
    } finally {
      setTimeout(() => {
        this.suppressedPaths.delete(path);
      }, 0);
    }
  }
}

export async function fetchRemoteChangePage(
  client: HazeSyncApiClient,
  state: RemoteSyncState,
  options: RemotePullOptions = {},
): Promise<ChangesResponseDto> {
  return client.getChanges({
    since: state.changeCursor,
    limit: options.limit,
  });
}

export function markRemotePullStarted(state: RemoteSyncState, observedAt: string): RemoteSyncState {
  return markRemotePullAttempt(state, observedAt);
}

export async function materializeRemoteChange(input: RemoteMaterializationInput): Promise<RemoteMaterializationResult> {
  const classification = classifyVaultPath(input.change.path);
  if (!classification.included) {
    return queueConflict(input, "unsupported_path");
  }

  switch (input.change.kind) {
    case "upsert":
      return materializeUpsert(input, classification.path);
    case "delete":
      return materializeDelete(input, classification.path);
    case "conflict":
      return queueConflict(input, "unsupported_change");
    default:
      return queueConflict(input, "unsupported_change");
  }
}

async function materializeUpsert(
  input: RemoteMaterializationInput,
  path: string,
): Promise<RemoteMaterializationResult> {
  if (input.download === undefined) {
    return queueConflict(input, "download_missing");
  }

  const expectedHash = input.change.content_hash ?? input.download.metadata.content_hash ?? undefined;
  if (expectedHash === undefined || expectedHash === null) {
    return queueConflict(input, "missing_content_hash");
  }

  const revisionId = input.change.revision_id ?? input.download.metadata.revision_id ?? undefined;
  if (revisionId === undefined || revisionId === null) {
    return queueConflict(input, "missing_revision");
  }

  const downloadedRevisionId = input.download.metadata.revision_id;
  if (
    input.change.revision_id !== undefined &&
    input.change.revision_id !== null &&
    downloadedRevisionId !== undefined &&
    downloadedRevisionId !== input.change.revision_id
  ) {
    return queueConflict(input, "revision_mismatch");
  }

  const actualHash = await sha256Hex(input.download.body);
  if (actualHash !== expectedHash) {
    return queueConflict(input, "hash_mismatch");
  }

  const local = await inspectLocalPath(input.vault, path, input.baseRevisionState, expectedHash);
  if (local.status === "dirty" || local.status === "blocked") {
    return queueConflict(input, local.reason);
  }

  if (local.status === "clean" && local.contentHash === expectedHash) {
    return remoteApplied(input, path, revisionId, expectedHash, false);
  }

  const text = new TextDecoder("utf-8").decode(input.download.body);
  const apply = async (): Promise<void> => {
    if (local.status === "missing") {
      await input.vault.create(path, text);
    } else {
      await input.vault.modify(local.file, text);
    }
  };

  try {
    if (input.echoSuppressor === undefined) {
      await apply();
    } else {
      await input.echoSuppressor.suppressWhile(path, apply);
    }
  } catch {
    return queueConflict(input, "write_failed");
  }

  return remoteApplied(input, path, revisionId, expectedHash, true);
}

async function materializeDelete(input: RemoteMaterializationInput, path: string): Promise<RemoteMaterializationResult> {
  const local = await inspectLocalPath(input.vault, path, input.baseRevisionState, undefined);
  if (local.status === "dirty" || local.status === "blocked") {
    return queueConflict(input, local.reason);
  }

  const revisionId = input.change.revision_id ?? null;
  const remoteSyncState = advanceRemoteCursor(
    recordRemoteTombstone(input.remoteSyncState, input.change, revisionId, input.observedAt),
    input.change.sequence,
    input.observedAt,
  );

  return {
    status: "tombstone_recorded",
    baseRevisionState: markRemoteDelete(input.baseRevisionState, path, revisionId, input.observedAt),
    remoteSyncState,
    path,
  };
}

async function inspectLocalPath(
  vault: Vault,
  path: string,
  baseRevisionState: BaseRevisionState,
  expectedHash: ContentHash | undefined,
): Promise<LocalPathInspection> {
  const current = vault.getAbstractFileByPath(path);
  if (current === null) {
    return { status: "missing" };
  }

  if (!(current instanceof TFile)) {
    return { status: "blocked", reason: "non_file_path" };
  }

  const factResult = await readLocalFileFact(vault, current);
  if (factResult.fact === undefined) {
    return { status: "blocked", reason: "unsupported_path" };
  }

  const localHash = factResult.fact.contentHash;
  const baseHash = getBaseContentHash(baseRevisionState, path);
  const isCleanToBase = baseHash !== null && localHash === baseHash;
  const alreadyMatchesRemote = expectedHash !== undefined && localHash === expectedHash;

  if (isCleanToBase || alreadyMatchesRemote) {
    return { status: "clean", file: current, contentHash: localHash };
  }

  return { status: "dirty", reason: "dirty_local_file" };
}

function remoteApplied(
  input: RemoteMaterializationInput,
  path: string,
  revisionId: RevisionId,
  contentHash: ContentHash,
  wroteLocalFile: boolean,
): RemoteMaterializationResult {
  return {
    status: wroteLocalFile ? "applied" : "no_op",
    baseRevisionState: markRemoteBaseRevision(input.baseRevisionState, path, revisionId, contentHash, input.observedAt),
    remoteSyncState: advanceRemoteCursor(input.remoteSyncState, input.change.sequence, input.observedAt),
    path,
  };
}

function queueConflict(input: RemoteMaterializationInput, reason: RemoteConflictReason): RemoteMaterializationResult {
  return {
    status: "conflict_queued",
    baseRevisionState: input.baseRevisionState,
    remoteSyncState: recordRemoteConflict(input.remoteSyncState, input.change, reason, input.observedAt),
    path: input.change.path,
    reason,
  };
}

type LocalPathInspection =
  | { status: "missing" }
  | { status: "clean"; file: TFile; contentHash: ContentHash }
  | { status: "dirty"; reason: "dirty_local_file" }
  | { status: "blocked"; reason: "non_file_path" | "unsupported_path" };
