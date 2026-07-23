import { normalizeStoredContentHash } from "./content-hash";
import type { LocalMutationKind } from "./idempotency-keys";
import { generateLocalIdempotencyKey } from "./idempotency-keys";
import type { LocalFileFact } from "./local-file-facts";

export type PendingChangeKind = "created" | "modified" | "deleted";
export type PendingChangeSource = "event_hint" | "scan";

export interface PendingQueueEntry {
  path: string;
  kind: PendingChangeKind;
  source: PendingChangeSource;
  firstSeenAt: string;
  lastSeenAt: string;
  operationIdempotencyKey: string;
  file?: LocalFileFact;
  previousFile?: LocalFileFact;
}

export interface LocalSyncState {
  knownFiles: Record<string, LocalFileFact>;
  pendingQueue: Record<string, PendingQueueEntry>;
  lastFullScanAt?: string;
  lastEventHintAt?: string;
}

export interface PendingQueueSummary {
  total: number;
  created: number;
  modified: number;
  deleted: number;
  eventHints: number;
  scanFindings: number;
}

export interface ScanReconcileResult {
  state: LocalSyncState;
  summary: PendingQueueSummary;
}

export function createDefaultLocalSyncState(): LocalSyncState {
  return {
    knownFiles: {},
    pendingQueue: {},
  };
}

export function mergeLocalSyncState(rawState: unknown): LocalSyncState {
  if (!isRecord(rawState)) {
    return createDefaultLocalSyncState();
  }

  return {
    knownFiles: readLocalFileFactRecord(rawState.knownFiles),
    pendingQueue: readPendingQueue(rawState.pendingQueue),
    lastFullScanAt: readOptionalString(rawState.lastFullScanAt),
    lastEventHintAt: readOptionalString(rawState.lastEventHintAt),
  };
}

export function reconcileFullScan(
  currentState: LocalSyncState,
  facts: LocalFileFact[],
  scannedAt: string,
): ScanReconcileResult {
  const nextKnownFiles = indexFactsByPath(facts);
  const pendingQueue = { ...currentState.pendingQueue };

  for (const fact of facts) {
    const previousFact = currentState.knownFiles[fact.path];
    const currentEntry = pendingQueue[fact.path];
    if (previousFact === undefined) {
      pendingQueue[fact.path] = upsertPendingEntry(fact.path, currentEntry, "created", "scan", scannedAt, fact);
    } else if (fileFactChanged(previousFact, fact)) {
      pendingQueue[fact.path] = upsertPendingEntry(
        fact.path,
        currentEntry,
        "modified",
        "scan",
        scannedAt,
        fact,
        previousFact,
      );
    } else if (currentEntry?.kind === "deleted") {
      pendingQueue[fact.path] = upsertPendingEntry(
        fact.path,
        currentEntry,
        "modified",
        "scan",
        scannedAt,
        fact,
        currentEntry.previousFile ?? previousFact,
      );
    } else if (currentEntry !== undefined && currentEntry.file === undefined) {
      pendingQueue[fact.path] = upsertPendingEntry(
        fact.path,
        currentEntry,
        currentEntry.kind,
        "scan",
        scannedAt,
        fact,
        currentEntry.previousFile,
      );
    }
  }

  for (const [path, previousFact] of Object.entries(currentState.knownFiles)) {
    if (!(path in nextKnownFiles)) {
      pendingQueue[path] = upsertPendingEntry(path, pendingQueue[path], "deleted", "scan", scannedAt, undefined, previousFact);
    }
  }

  const state: LocalSyncState = {
    ...currentState,
    knownFiles: nextKnownFiles,
    pendingQueue,
    lastFullScanAt: scannedAt,
  };

  return {
    state,
    summary: summarizePendingQueue(state.pendingQueue),
  };
}

export function recordEventHint(
  currentState: LocalSyncState,
  path: string,
  kind: PendingChangeKind,
  observedAt: string,
): LocalSyncState {
  return {
    ...currentState,
    pendingQueue: {
      ...currentState.pendingQueue,
      [path]: upsertPendingEntry(path, currentState.pendingQueue[path], kind, "event_hint", observedAt),
    },
    lastEventHintAt: observedAt,
  };
}

export function summarizePendingQueue(pendingQueue: Record<string, PendingQueueEntry>): PendingQueueSummary {
  const summary: PendingQueueSummary = {
    total: 0,
    created: 0,
    modified: 0,
    deleted: 0,
    eventHints: 0,
    scanFindings: 0,
  };

  for (const entry of Object.values(pendingQueue)) {
    summary.total += 1;

    switch (entry.kind) {
      case "created":
        summary.created += 1;
        break;
      case "modified":
        summary.modified += 1;
        break;
      case "deleted":
        summary.deleted += 1;
        break;
    }

    if (entry.source === "event_hint") {
      summary.eventHints += 1;
    } else {
      summary.scanFindings += 1;
    }
  }

  return summary;
}

export function pendingQueueSummaryText(summary: PendingQueueSummary): string {
  if (summary.total === 0) {
    return "no pending local changes";
  }

  return `${summary.total} pending local change(s): ${summary.created} created, ${summary.modified} modified, ${summary.deleted} deleted`;
}

function upsertPendingEntry(
  path: string,
  currentEntry: PendingQueueEntry | undefined,
  kind: PendingChangeKind,
  source: PendingChangeSource,
  observedAt: string,
  file?: LocalFileFact,
  previousFile?: LocalFileFact,
): PendingQueueEntry {
  const operationKind = operationKindForPendingChange(kind);
  const currentOperationKind = currentEntry === undefined ? undefined : operationKindForPendingChange(currentEntry.kind);
  const sameOperationKind = currentEntry !== undefined && currentOperationKind === operationKind;
  const reuseIdempotencyKey = shouldReuseIdempotencyKey(currentEntry, operationKind, source, file);

  return {
    path,
    kind,
    source,
    firstSeenAt: sameOperationKind ? currentEntry.firstSeenAt : observedAt,
    lastSeenAt: observedAt,
    operationIdempotencyKey: reuseIdempotencyKey
      ? currentEntry.operationIdempotencyKey
      : generateLocalIdempotencyKey(operationKind),
    file: operationKind === "upload" ? file ?? currentEntry?.file : undefined,
    previousFile: previousFile ?? currentEntry?.previousFile ?? currentEntry?.file,
  };
}

function shouldReuseIdempotencyKey(
  currentEntry: PendingQueueEntry | undefined,
  operationKind: LocalMutationKind,
  source: PendingChangeSource,
  nextFile: LocalFileFact | undefined,
): currentEntry is PendingQueueEntry {
  if (
    currentEntry === undefined ||
    operationKindForPendingChange(currentEntry.kind) !== operationKind ||
    source === "event_hint"
  ) {
    return false;
  }

  if (operationKind === "delete" || nextFile === undefined) {
    return true;
  }

  return currentEntry.file?.contentHash === nextFile.contentHash;
}

function operationKindForPendingChange(kind: PendingChangeKind): LocalMutationKind {
  return kind === "deleted" ? "delete" : "upload";
}

function indexFactsByPath(facts: LocalFileFact[]): Record<string, LocalFileFact> {
  const indexed: Record<string, LocalFileFact> = {};
  for (const fact of facts) {
    indexed[fact.path] = fact;
  }

  return indexed;
}

function fileFactChanged(left: LocalFileFact, right: LocalFileFact): boolean {
  return (
    left.contentHash !== right.contentHash ||
    left.sizeBytes !== right.sizeBytes ||
    left.mtime !== right.mtime ||
    left.extension !== right.extension
  );
}

function readPendingQueue(value: unknown): Record<string, PendingQueueEntry> {
  if (!isRecord(value)) {
    return {};
  }

  const result: Record<string, PendingQueueEntry> = {};
  for (const [key, item] of Object.entries(value)) {
    const entry = readPendingQueueEntry(item);
    if (entry !== undefined) {
      result[key] = entry;
    }
  }

  return result;
}

function readPendingQueueEntry(value: unknown): PendingQueueEntry | undefined {
  if (
    !isRecord(value) ||
    typeof value.path !== "string" ||
    !isPendingChangeKind(value.kind) ||
    !isPendingChangeSource(value.source) ||
    typeof value.firstSeenAt !== "string" ||
    typeof value.lastSeenAt !== "string" ||
    !(value.operationIdempotencyKey === undefined || typeof value.operationIdempotencyKey === "string")
  ) {
    return undefined;
  }

  const file = value.file === undefined ? undefined : readLocalFileFact(value.file);
  const previousFile = value.previousFile === undefined ? undefined : readLocalFileFact(value.previousFile);
  if ((value.file !== undefined && file === undefined) || (value.previousFile !== undefined && previousFile === undefined)) {
    return undefined;
  }

  return {
    path: value.path,
    kind: value.kind,
    source: value.source,
    firstSeenAt: value.firstSeenAt,
    lastSeenAt: value.lastSeenAt,
    operationIdempotencyKey:
      value.operationIdempotencyKey ?? generateLocalIdempotencyKey(operationKindForPendingChange(value.kind)),
    file,
    previousFile,
  };
}

function readLocalFileFactRecord(value: unknown): Record<string, LocalFileFact> {
  if (!isRecord(value)) {
    return {};
  }

  const result: Record<string, LocalFileFact> = {};
  for (const [key, item] of Object.entries(value)) {
    const fact = readLocalFileFact(item);
    if (fact !== undefined) {
      result[key] = fact;
    }
  }
  return result;
}

function readLocalFileFact(value: unknown): LocalFileFact | undefined {
  if (
    !isRecord(value) ||
    typeof value.path !== "string" ||
    typeof value.extension !== "string" ||
    typeof value.sizeBytes !== "number" ||
    typeof value.mtime !== "number"
  ) {
    return undefined;
  }

  const contentHash = normalizeStoredContentHash(value.contentHash);
  if (contentHash === undefined) {
    return undefined;
  }

  return {
    path: value.path,
    extension: value.extension,
    contentHash,
    sizeBytes: value.sizeBytes,
    mtime: value.mtime,
  };
}

function readOptionalString(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function isPendingChangeKind(value: unknown): value is PendingChangeKind {
  return value === "created" || value === "modified" || value === "deleted";
}

function isPendingChangeSource(value: unknown): value is PendingChangeSource {
  return value === "event_hint" || value === "scan";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
