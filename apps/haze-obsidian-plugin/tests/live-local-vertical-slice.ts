import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";

import { HazeSyncApiClient } from "../src/api-client";

interface WorktreeStatus {
  configured_mode: string;
  host_lifecycle: string;
  readiness: string;
  readiness_reason: string;
  cycles_completed: number;
  cycles_failed: number;
  cycle_in_progress: boolean;
  pending_watcher_hints: number;
  manual_availability: string;
}

const VAULT_PATH = "stage8-local-vertical-slice.md";
const INITIAL_CONTENT = "# Stage 8\n\nCreated through the Obsidian production HTTP client.\n";
const WORKTREE_CONTENT =
  "# Stage 8\n\nUpdated through the live Server-hosted Worktree adapter.\n";

async function main(): Promise<void> {
  const serverUrl = requiredEnv("HAZE_OBSIDIAN_SMOKE_SERVER_URL");
  const authToken = requiredEnv("HAZE_OBSIDIAN_SMOKE_AUTH_TOKEN");
  const adminToken = requiredEnv("HAZE_OBSIDIAN_SMOKE_ADMIN_TOKEN");
  const worktreeRoot = requiredEnv("HAZE_OBSIDIAN_SMOKE_WORKTREE_PATH");
  const parsedUrl = validateLoopbackServerUrl(serverUrl);
  const worktreePath = path.resolve(worktreeRoot, VAULT_PATH);

  const client = new HazeSyncApiClient({
    config: { serverUrl: parsedUrl.toString(), authToken },
  });

  const info = await client.getServerInfo();
  assert.equal(info.protocol_version, 1);
  assert.ok(info.capabilities.includes("sha256"));
  assert.ok(info.capabilities.includes("batch_changes"));

  const before = await client.getChanges({ since: 0, limit: 50 });
  const put = await client.putFile({
    path: VAULT_PATH,
    body: INITIAL_CONTENT,
    contentHash: contentHash(INITIAL_CONTENT),
    baseRevisionId: null,
    idempotencyKey: "stage8-obsidian-put-v1",
    contentType: "text/markdown; charset=utf-8",
  });
  assert.equal(put.status, "accepted");
  if (put.status !== "accepted") {
    throw new Error("Stage 8 Obsidian upload was not accepted.");
  }

  await runWorktreeCycle(parsedUrl, adminToken);
  await waitForFileContent(worktreePath, INITIAL_CONTENT, 60_000);

  const replacementPath = path.resolve(worktreeRoot, ".stage8-replacement.tmp");
  await writeFile(replacementPath, WORKTREE_CONTENT, { encoding: "utf8", mode: 0o600 });
  await rename(replacementPath, worktreePath);

  await runWorktreeCycle(parsedUrl, adminToken);
  const imported = await waitForRemoteContent(client, WORKTREE_CONTENT, 60_000);
  assert.notEqual(imported.metadata.revision_id, put.revision_id);
  assert.equal(imported.metadata.content_sha256, contentHash(WORKTREE_CONTENT));

  const changes = await client.getChanges({ since: put.seq, limit: 50 });
  const worktreeChange = changes.changes.find(
    (change) =>
      change.kind === "upsert_file" &&
      change.path === VAULT_PATH &&
      change.updated_by === "worktree",
  );
  assert.ok(worktreeChange, "Worktree import must appear in the live change feed.");
  assert.ok(changes.to_seq > before.to_seq);

  const finalStatus = await getWorktreeStatus(parsedUrl, adminToken);
  assert.equal(finalStatus.configured_mode, "bidirectional");
  assert.equal(finalStatus.host_lifecycle, "running");
  assert.equal(finalStatus.readiness, "ready");
  assert.equal(finalStatus.readiness_reason, "running");
  assert.equal(finalStatus.cycles_failed, 0);
  assert.ok(finalStatus.cycles_completed >= 2);

  console.log(
    "PASS Stage 8 local vertical slice: Obsidian PUT, Worktree export/import, and Obsidian change/readback validated.",
  );
}

function requiredEnv(name: string): string {
  const value = process.env[name]?.trim();
  if (!value) {
    throw new Error(`${name} is required for the Stage 8 local vertical slice.`);
  }
  return value;
}

function validateLoopbackServerUrl(value: string): URL {
  const parsed = new URL(value);
  assert.ok(
    parsed.hostname === "localhost" ||
      parsed.hostname === "127.0.0.1" ||
      parsed.hostname === "[::1]",
    "Stage 8 smoke accepts only loopback Server URLs.",
  );
  return parsed;
}

function contentHash(content: string): string {
  return `sha256:${createHash("sha256").update(content, "utf8").digest("hex")}`;
}

async function getWorktreeStatus(serverUrl: URL, adminToken: string): Promise<WorktreeStatus> {
  const response = await fetch(new URL("/v1/admin/worktree/status", serverUrl), {
    method: "GET",
    headers: { Authorization: `Bearer ${adminToken}` },
    redirect: "error",
    credentials: "omit",
    cache: "no-store",
  });
  assert.equal(response.status, 200, "Worktree status endpoint must be available.");
  return (await response.json()) as WorktreeStatus;
}

async function runWorktreeCycle(serverUrl: URL, adminToken: string): Promise<void> {
  const deadline = Date.now() + 60_000;
  let baseline: WorktreeStatus | undefined;

  while (Date.now() < deadline) {
    const status = await getWorktreeStatus(serverUrl, adminToken);
    assert.notEqual(status.host_lifecycle, "failed");
    assert.equal(status.cycles_failed, 0);

    if (status.manual_availability === "available") {
      baseline = status;
      const response = await fetch(new URL("/v1/admin/worktree/sync-once", serverUrl), {
        method: "POST",
        body: "{}",
        headers: {
          Authorization: `Bearer ${adminToken}`,
          "Content-Type": "application/json",
        },
        redirect: "error",
        credentials: "omit",
        cache: "no-store",
      });
      if (response.status === 202) {
        const payload = (await response.json()) as { status?: string };
        assert.equal(payload.status, "accepted");
        break;
      }
      if (response.status !== 409 && response.status !== 503) {
        throw new Error(`Worktree sync-once returned unexpected status ${response.status}.`);
      }
    }
    await sleep(200);
  }

  if (baseline === undefined) {
    throw new Error("Worktree manual cycle did not become available.");
  }

  while (Date.now() < deadline) {
    const status = await getWorktreeStatus(serverUrl, adminToken);
    assert.equal(status.cycles_failed, baseline.cycles_failed);
    if (!status.cycle_in_progress && status.cycles_completed > baseline.cycles_completed) {
      return;
    }
    await sleep(200);
  }

  throw new Error("Worktree manual cycle did not complete within the bounded timeout.");
}

async function waitForFileContent(
  filePath: string,
  expected: string,
  timeoutMs: number,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const content = await readFile(filePath, "utf8");
      if (content === expected) {
        return;
      }
    } catch {
      // The export may not have materialized yet.
    }
    await sleep(200);
  }
  throw new Error("Worktree export did not materialize the expected file content.");
}

async function waitForRemoteContent(
  client: HazeSyncApiClient,
  expected: string,
  timeoutMs: number,
) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const file = await client.getFile(VAULT_PATH);
      const content = new TextDecoder().decode(file.body);
      if (content === expected) {
        return file;
      }
    } catch {
      // The Worktree import may still be in progress.
    }
    await sleep(200);
  }
  throw new Error("Worktree import did not reach the Server within the bounded timeout.");
}

function sleep(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

void main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : "Stage 8 local vertical slice failed.");
  process.exitCode = 1;
});
