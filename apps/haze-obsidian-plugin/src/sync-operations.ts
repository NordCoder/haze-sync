import { TFile, Vault } from "obsidian";

import {
  HazeSyncApiClient,
  createInvalidResponseError,
} from "./api-client";
import {
  BaseRevisionState,
  applyDeleteOutcome,
  applyUploadOutcome,
} from "./base-revision-store";
import type { LocalFileFact } from "./local-file-facts";
import { sha256Hex } from "./local-file-facts";
import { planPendingMutations } from "./mutation-planner";
import type { LocalSyncState, PendingQueueEntry } from "./pending-queue";
import { reconcileFullScan } from "./pending-queue";
import {
  RemoteEchoSuppressor,
  fetchRemoteChangePage,
  markRemotePullStarted,
  materializeRemoteChange,
  remoteChangeNeedsDownload,
} from "./remote-materializer";
import type { RemoteSyncState } from "./remote-sync-state";
import type { VaultScanResult, VaultScanner } from "./vault-scanner";

export interface LocalScanOperationResult {
  localState: LocalSyncState;
  unreadableFiles: number;
}

export interface PushOperationProgress {
  path: string;
  operationIdempotencyKey: string;
  baseRevisionState: BaseRevisionState;
}

export interface PushOperationResult {
  uploaded: number;
  deleted: number;
  sameContent: number;
  conflicts: number;
  rejected: number;
  skipped: number;
}

export interface PullOperationProgress {
  baseRevisionState: BaseRevisionState;
  remoteSyncState: RemoteSyncState;
}

export interface PullOperationResult {
  applied: number;
  noOps: number;
  tombstones: number;
  conflicts: number;
  pages: number;
  hasMore: boolean;
}

export async function scanLocalVault(
  scanner: VaultScanner,
  currentState: LocalSyncState,
  signal: AbortSignal,
): Promise<LocalScanOperationResult> {
  assertNotAborted(signal);
  const scanResult = await scanner.scan();
  assertNotAborted(signal);
  const safeFacts = preserveUnreadableKnownFiles(scanResult, currentState.knownFiles);
  const reconciled = reconcileFullScan(currentState, safeFacts, scanResult.scannedAt);

  return {
    localState: reconciled.state,
    unreadableFiles: scanResult.errors.length,
  };
}

export async function pushPendingChanges(input: {
  vault: Vault;
  client: HazeSyncApiClient;
  localState: LocalSyncState;
  baseRevisionState: BaseRevisionState;
  signal: AbortSignal;
  allowDeletes: boolean;
  onProgress(progress: PushOperationProgress): Promise<void>;
}): Promise<PushOperationResult> {
  const uploadBodies = await readStableUploadBodies(input.vault, input.localState.pendingQueue, input.signal);
  const plan = planPendingMutations(input.localState.pendingQueue, input.baseRevisionState, {
    makeUploadBody: (entry) => uploadBodies.get(entry.path),
  });
  const result: PushOperationResult = {
    uploaded: 0,
    deleted: 0,
    sameContent: 0,
    conflicts: 0,
    rejected: 0,
    skipped: 0,
  };
  let workingBaseState = input.baseRevisionState;

  for (const skipped of plan.skipped) {
    assertNotAborted(input.signal);
    if (skipped.reason === "same_content") {
      result.sameContent += 1;
      await input.onProgress({
        path: skipped.path,
        operationIdempotencyKey: skipped.queueEntry.operationIdempotencyKey,
        baseRevisionState: workingBaseState,
      });
    } else {
      result.skipped += 1;
    }
  }

  for (const mutation of plan.planned) {
    assertNotAborted(input.signal);
    if (mutation.kind === "delete" && !input.allowDeletes) {
      result.skipped += 1;
      continue;
    }

    const observedAt = new Date().toISOString();
    if (mutation.kind === "upload") {
      const response = await input.client.putFile(mutation.request);
      assertMatchingPath(response.path, mutation.path);
      const update = applyUploadOutcome(
        workingBaseState,
        response,
        observedAt,
        mutation.request.contentHash,
      );

      if (update.baseUpdated) {
        workingBaseState = update.state;
        result.uploaded += update.outcome === "accepted" ? 1 : 0;
        result.sameContent += update.outcome === "same_content" ? 1 : 0;
        await input.onProgress({
          path: mutation.path,
          operationIdempotencyKey: mutation.queueEntry.operationIdempotencyKey,
          baseRevisionState: workingBaseState,
        });
      } else if (update.outcome === "conflict_saved") {
        result.conflicts += 1;
      } else {
        result.rejected += 1;
      }
      continue;
    }

    const response = await input.client.deleteFile(mutation.request);
    assertMatchingPath(response.path, mutation.path);
    const update = applyDeleteOutcome(workingBaseState, response, observedAt);
    if (update.baseUpdated) {
      workingBaseState = update.state;
      result.deleted += 1;
      await input.onProgress({
        path: mutation.path,
        operationIdempotencyKey: mutation.queueEntry.operationIdempotencyKey,
        baseRevisionState: workingBaseState,
      });
    } else if (update.outcome === "conflict_saved") {
      result.conflicts += 1;
    } else {
      result.rejected += 1;
    }
  }

  return result;
}

export async function pullRemoteChanges(input: {
  vault: Vault;
  client: HazeSyncApiClient;
  baseRevisionState: BaseRevisionState;
  remoteSyncState: RemoteSyncState;
  signal: AbortSignal;
  echoSuppressor: RemoteEchoSuppressor;
  onProgress(progress: PullOperationProgress): Promise<void>;
  maxPages?: number;
}): Promise<PullOperationResult> {
  const maxPages = Math.max(1, Math.min(input.maxPages ?? 10, 50));
  const result: PullOperationResult = {
    applied: 0,
    noOps: 0,
    tombstones: 0,
    conflicts: 0,
    pages: 0,
    hasMore: false,
  };
  let baseRevisionState = input.baseRevisionState;
  let remoteSyncState = markRemotePullStarted(input.remoteSyncState, new Date().toISOString());
  await input.onProgress({ baseRevisionState, remoteSyncState });

  for (let page = 0; page < maxPages; page += 1) {
    assertNotAborted(input.signal);
    const response = await fetchRemoteChangePage(input.client, remoteSyncState, { limit: 50 });
    result.pages += 1;
    result.hasMore = Boolean(response.has_more);

    for (const change of response.changes) {
      assertNotAborted(input.signal);
      const download = remoteChangeNeedsDownload(change)
        ? await input.client.getFile(change.path)
        : undefined;
      const materialized = await materializeRemoteChange({
        vault: input.vault,
        change,
        download,
        baseRevisionState,
        remoteSyncState,
        observedAt: new Date().toISOString(),
        echoSuppressor: input.echoSuppressor,
      });
      baseRevisionState = materialized.baseRevisionState;
      remoteSyncState = materialized.remoteSyncState;
      await input.onProgress({ baseRevisionState, remoteSyncState });

      switch (materialized.status) {
        case "applied":
          result.applied += 1;
          break;
        case "no_op":
          result.noOps += 1;
          break;
        case "tombstone_recorded":
          result.tombstones += 1;
          break;
        case "conflict_queued":
          result.conflicts += 1;
          return result;
      }
    }

    if (!response.has_more || response.changes.length === 0) {
      return result;
    }
  }

  return result;
}

export function clearPendingChangeIfCurrent(
  state: LocalSyncState,
  path: string,
  operationIdempotencyKey: string,
): LocalSyncState {
  const current = state.pendingQueue[path];
  if (current === undefined || current.operationIdempotencyKey !== operationIdempotencyKey) {
    return state;
  }

  const pendingQueue = { ...state.pendingQueue };
  delete pendingQueue[path];
  return {
    ...state,
    pendingQueue,
  };
}

function preserveUnreadableKnownFiles(
  scanResult: VaultScanResult,
  knownFiles: Record<string, LocalFileFact>,
): LocalFileFact[] {
  const factsByPath = new Map<string, LocalFileFact>();
  for (const fact of scanResult.facts) {
    factsByPath.set(fact.path, fact);
  }
  for (const error of scanResult.errors) {
    const known = knownFiles[error.path];
    if (known !== undefined) {
      factsByPath.set(error.path, known);
    }
  }

  return Array.from(factsByPath.values()).sort((left, right) => left.path.localeCompare(right.path));
}

function assertMatchingPath(actual: string, expected: string): void {
  if (actual !== expected) {
    throw createInvalidResponseError("/v1/files/{path}");
  }
}

async function readStableUploadBodies(
  vault: Vault,
  pendingQueue: Record<string, PendingQueueEntry>,
  signal: AbortSignal,
): Promise<Map<string, ArrayBuffer>> {
  const bodies = new Map<string, ArrayBuffer>();
  const entries = Object.values(pendingQueue)
    .filter((entry) => entry.kind !== "deleted" && entry.file !== undefined)
    .sort((left, right) => left.path.localeCompare(right.path));

  for (const entry of entries) {
    assertNotAborted(signal);
    const file = vault.getAbstractFileByPath(entry.path);
    if (!(file instanceof TFile) || entry.file === undefined) {
      continue;
    }

    const body = await vault.readBinary(file);
    assertNotAborted(signal);
    if (await sha256Hex(body) === entry.file.contentHash) {
      bodies.set(entry.path, body);
    }
  }

  return bodies;
}

function assertNotAborted(signal: AbortSignal): void {
  if (signal.aborted) {
    throw new DOMException("Sync operation aborted.", "AbortError");
  }
}
