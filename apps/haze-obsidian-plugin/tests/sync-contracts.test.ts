import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import test from "node:test";

import {
  applyDeleteOutcome,
  applyUploadOutcome,
  createDefaultBaseRevisionState,
  mergeBaseRevisionState,
} from "../src/base-revision-store";
import { CONFLICT_ACTION_DEFINITIONS } from "../src/conflict-center";
import { normalizeStoredContentHash } from "../src/content-hash";
import {
  createDefaultLocalSyncState,
  mergeLocalSyncState,
  reconcileFullScan,
  recordEventHint,
} from "../src/pending-queue";
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
    },
  });
  assert.equal(base.byPath["Notes/a.md"].contentHash, `sha256:${legacy}`);

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
