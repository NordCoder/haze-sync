import { DeleteFileRequest, PutFileRequest, RevisionId, VaultPath } from "./api-client";
import { BaseRevisionState, getBaseContentHash, getBaseRevisionId } from "./base-revision-store";
import { generateLocalIdempotencyKey } from "./idempotency-keys";
import { PendingQueueEntry } from "./pending-queue";

export type PlannedMutationKind = "upload" | "delete";
export type PlannerSkipReason = "missing_file_fact" | "missing_upload_body" | "same_content";

export interface PlannedUploadMutation {
  kind: "upload";
  path: VaultPath;
  queueEntry: PendingQueueEntry;
  request: PutFileRequest;
}

export interface PlannedDeleteMutation {
  kind: "delete";
  path: VaultPath;
  queueEntry: PendingQueueEntry;
  request: DeleteFileRequest;
}

export type PlannedMutation = PlannedUploadMutation | PlannedDeleteMutation;

export interface SkippedPendingMutation {
  path: VaultPath;
  kind: PlannedMutationKind;
  queueEntry: PendingQueueEntry;
  reason: PlannerSkipReason;
}

export interface MutationPlan {
  planned: PlannedMutation[];
  skipped: SkippedPendingMutation[];
}

export interface MutationPlanOptions {
  makeUploadBody?: (entry: PendingQueueEntry) => BodyInit | undefined;
}

export function planPendingMutations(
  pendingQueue: Record<string, PendingQueueEntry>,
  baseState: BaseRevisionState,
  options: MutationPlanOptions = {},
): MutationPlan {
  const planned: PlannedMutation[] = [];
  const skipped: SkippedPendingMutation[] = [];

  const entries = Object.values(pendingQueue).sort((left, right) => left.path.localeCompare(right.path));
  for (const entry of entries) {
    if (entry.kind === "deleted") {
      planned.push(planDelete(entry, baseState));
      continue;
    }

    const upload = planUpload(entry, baseState, options);
    if ("skipped" in upload) {
      skipped.push(upload.skipped);
    } else {
      planned.push(upload.planned);
    }
  }

  return {
    planned,
    skipped,
  };
}

function planUpload(
  entry: PendingQueueEntry,
  baseState: BaseRevisionState,
  options: MutationPlanOptions,
): { planned: PlannedUploadMutation } | { skipped: SkippedPendingMutation } {
  if (entry.file === undefined) {
    return skippedUpload(entry, "missing_file_fact");
  }

  if (entry.file.contentHash === getBaseContentHash(baseState, entry.path)) {
    return skippedUpload(entry, "same_content");
  }

  const body = options.makeUploadBody?.(entry);
  if (body === undefined) {
    return skippedUpload(entry, "missing_upload_body");
  }

  return {
    planned: {
      kind: "upload",
      path: entry.path,
      queueEntry: entry,
      request: {
        path: entry.path,
        body,
        contentHash: entry.file.contentHash,
        baseRevisionId: baseRevisionForRequest(baseState, entry.path),
        idempotencyKey: generateLocalIdempotencyKey("upload"),
        contentType: contentTypeForExtension(entry.file.extension),
      },
    },
  };
}

function planDelete(entry: PendingQueueEntry, baseState: BaseRevisionState): PlannedDeleteMutation {
  return {
    kind: "delete",
    path: entry.path,
    queueEntry: entry,
    request: {
      path: entry.path,
      baseRevisionId: baseRevisionForRequest(baseState, entry.path),
      idempotencyKey: generateLocalIdempotencyKey("delete"),
    },
  };
}

function skippedUpload(
  entry: PendingQueueEntry,
  reason: PlannerSkipReason,
): { skipped: SkippedPendingMutation } {
  return {
    skipped: {
      path: entry.path,
      kind: "upload",
      queueEntry: entry,
      reason,
    },
  };
}

function baseRevisionForRequest(state: BaseRevisionState, path: VaultPath): RevisionId | null {
  return getBaseRevisionId(state, path);
}

function contentTypeForExtension(extension: string): string {
  switch (extension.toLowerCase()) {
    case "md":
    case "txt":
      return "text/plain; charset=utf-8";
    case "canvas":
      return "application/json";
    default:
      return "application/octet-stream";
  }
}
