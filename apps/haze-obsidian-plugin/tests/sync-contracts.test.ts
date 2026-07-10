import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

import {
  applyDeleteOutcome,
  applyUploadOutcome,
  createDefaultBaseRevisionState,
  getBaseRevisionId,
  mergeBaseRevisionState,
} from "../src/base-revision-store";
import { CONFLICT_ACTION_DEFINITIONS } from "../src/conflict-center";
import { normalizeStoredContentHash } from "../src/content-hash";
import { planPendingMutations } from "../src/mutation-planner";
import {
  createDefaultLocalSyncState,
  mergeLocalSyncState,
  reconcileFullScan,
  recordEventHint,
} from "../src/pending-queue";
import {
  applyRemoteMetadataChange,
  createDefaultRemoteSyncState,
} from "../src/remote-sync-state";
import { sanitizeStatusMessage } from "../src/safe-text";
import { markSyncFailed, syncBackoffRemainingMs } from "../src/sync-runtime-state";
import { parseApiContractFixture } from "../src/api-client";

const HASH_A = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

function fact(pathValue: string, contentHash: string) {
  return {
    path: pathValue,
    extension: "md",
    contentHash,
    sizeBytes: 10,
    mtime: 1,
  };
}

test("scan reconciliation rotates mutation identity only when payload changes", () => {
  const first = reconcileFullScan(createDefaultLocalSyncState(), [fact("Notes/a.md", HASH_A)], "2026-01-01T00:00:00Z").state;
  const firstKey = first.pendingQueue["Notes/a.md"].operationIdempotencyKey;

  const unchanged = reconcileFullScan(first, [fact("Notes/a.md", HASH_A)], "2026-01-01T00:01:00Z").state;
  assert.equal(unchanged.pendingQueue["Notes/a.md"].operationIdempotencyKey, firstKey);

  const changed = reconcileFullScan(unchanged, [fact("Notes/a.md", HASH_B)], "2026-01-01T00:02:00Z").state;
  assert.notEqual(changed.pendingQueue["Notes/a.md"].operationIdempotencyKey, firstKey);
});

test("new event hint cannot reuse an in-flight mutation identity", () => {
  const scanned = reconcileFullScan(createDefaultLocalSyncState(), [fact("Notes/a.md", HASH_A)], "2026-01-01T00:00:00Z").state;
  const firstKey = scanned.pendingQueue["Notes/a.md"].operationIdempotencyKey;
  const hinted = recordEventHint(scanned, "Notes/a.md", "modified", "2026-01-01T00:00:01Z");

  assert.notEqual(hinted.pendingQueue["Notes/a.md"].operationIdempotencyKey, firstKey);
  assert.equal(hinted.pendingQueue["Notes/a.md"].file?.contentHash, HASH_A);
});

test("canonical base outcomes do not invent missing revision metadata", () => {
  const initial = createDefaultBaseRevisionState();
  const accepted = applyUploadOutcome(
    initial,
    { status: "accepted", path: "Notes/a.md", revision_id: "rev_2", seq: 2 },
    "2026-01-01T00:00:00Z",
    HASH_A,
  );
  assert.equal(accepted.baseUpdated, true);
  assert.equal(accepted.state.byPath["Notes/a.md"].revisionId, "rev_2");

  const ignored = applyUploadOutcome(
    accepted.state,
    { status: "ignored", reason: "same_content", path: "Notes/a.md" },
    "2026-01-01T00:01:00Z",
    HASH_A,
  );
  assert.equal(ignored.outcome, "same_content");
  assert.equal(ignored.baseUpdated, false);
  assert.strictEqual(ignored.state, accepted.state);

  const missing = applyDeleteOutcome(
    accepted.state,
    { status: "not_found", path: "Notes/a.md" },
    "2026-01-01T00:02:00Z",
  );
  assert.equal(missing.outcome, "not_found");
  assert.equal(missing.state.byPath["Notes/a.md"].serverDeleted, true);
  assert.equal(missing.state.byPath["Notes/a.md"].revisionId, null);
  assert.equal(getBaseRevisionId(missing.state, "Notes/a.md"), null);

  const recreated = reconcileFullScan(
    createDefaultLocalSyncState(),
    [fact("Notes/a.md", HASH_B)],
    "2026-01-01T00:03:00Z",
  ).state;
  const plan = planPendingMutations(recreated.pendingQueue, missing.state, {
    makeUploadBody: () => "fixture-body",
  });
  assert.equal(plan.planned.length, 1);
  const upload = plan.planned[0];
  assert.equal(upload.kind, "upload");
  if (upload.kind === "upload") {
    assert.equal(upload.request.baseRevisionId, null);
  }
});

test("server conflict metadata advances cursor and clears after resolution", () => {
  const created = applyRemoteMetadataChange(
    createDefaultRemoteSyncState(),
    {
      seq: 43,
      kind: "conflict_created",
      path: "Notes/a.md",
      conflict_id: "conf_fixture_0001",
      updated_by: "gdrive-adapter",
      updated_at: "2026-01-01T00:03:00Z",
    },
    "2026-01-01T00:03:01Z",
  );

  assert.equal(created.conflictRecorded, true);
  assert.equal(created.state.changeCursor, 43);
  assert.equal(created.state.conflicts.conf_fixture_0001.reason, "server_conflict");

  const resolved = applyRemoteMetadataChange(
    created.state,
    {
      seq: 44,
      kind: "conflict_resolved",
      path: "Notes/a.md",
      conflict_id: "conf_fixture_0001",
      updated_by: "obsidian-plugin",
      updated_at: "2026-01-01T00:04:00Z",
    },
    "2026-01-01T00:04:01Z",
  );

  assert.equal(resolved.conflictRecorded, false);
  assert.equal(resolved.state.changeCursor, 44);
  assert.deepEqual(resolved.state.conflicts, {});
});

test("legacy persisted hex hashes migrate in base and pending state readers", () => {
  const legacy = "a".repeat(64);
  assert.equal(normalizeStoredContentHash(legacy), `sha256:${legacy}`);

  const base = mergeBaseRevisionState({
    byPath: {
      "Notes/a.md": {
        path: "Notes/a.md",
        revisionId: "rev_1",
        contentHash: legacy,
        serverDeleted: false,
        updatedAt: "2026-01-01T00:00:00Z",
      },
      "Notes/deleted.md": {
        path: "Notes/deleted.md",
        revisionId: "rev_stale",
        contentHash: legacy,
        serverDeleted: true,
        updatedAt: "2026-01-01T00:00:00Z",
      },
    },
  });
  assert.equal(base.byPath["Notes/a.md"].contentHash, `sha256:${legacy}`);
  assert.equal(base.byPath["Notes/deleted.md"].revisionId, null);
  assert.equal(base.byPath["Notes/deleted.md"].contentHash, null);

  const local = mergeLocalSyncState({
    knownFiles: {
      "Notes/a.md": fact("Notes/a.md", legacy),
    },
    pendingQueue: {},
  });
  assert.equal(local.knownFiles["Notes/a.md"].contentHash, `sha256:${legacy}`);
  assert.equal(normalizeStoredContentHash("not-a-hash"), undefined);
});

test("retry state is bounded and status output redacts secrets", () => {
  const failed = markSyncFailed(
    { phase: "idle", consecutiveFailures: 20 },
    "offline",
    "2026-01-01T00:00:00Z",
    "offline",
  );
  assert.equal(syncBackoffRemainingMs(failed, Date.parse("2026-01-01T00:00:00Z")), 5 * 60_000);

  const safe = sanitizeStatusMessage(
    "Authorization: secret-value\nBearer token-value exact-secret",
    ["exact-secret"],
  );
  assert.equal(safe.includes("secret-value"), false);
  assert.equal(safe.includes("token-value"), false);
  assert.equal(safe.includes("exact-secret"), false);
});

test("reserved conflict vocabulary is visible but not executable in current UI", () => {
  const reserved = CONFLICT_ACTION_DEFINITIONS.find((definition) => definition.action === "accept_conflict");
  assert.equal(reserved?.available, false);
  assert.match(reserved?.unavailableReason ?? "", /current Server/u);
});

test("synthetic doctor and status fixture stays internally coherent", () => {
  const fixturePath = path.join(process.cwd(), "tests/fixtures/api-contract-v1.json");
  const fixture = parseApiContractFixture(JSON.parse(readFileSync(fixturePath, "utf8")) as unknown);

  assert.equal(fixture.admin.status_summary.adapter_count, fixture.admin.adapter_list.total_count);
  assert.equal(fixture.admin.doctor_statuses[0].checks[0].check, "database");
  assert.equal(fixture.admin.doctor_statuses[0].checks[1].status, "failed");
});
