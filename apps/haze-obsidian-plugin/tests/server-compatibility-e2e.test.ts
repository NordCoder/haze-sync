import assert from "node:assert/strict";
import test from "node:test";

import type { Vault } from "obsidian";

import {
  ApiClientError,
  HazeSyncApiClient,
  type HttpTransport,
} from "../src/api-client";
import { createDefaultBaseRevisionState } from "../src/base-revision-store";
import { materializeRemoteChange } from "../src/remote-materializer";
import { createDefaultRemoteSyncState } from "../src/remote-sync-state";

const TOKEN = "synthetic-e2e-token";
const PUT_KEY = "synthetic-put-key";
const DELETE_KEY = "synthetic-delete-key";
const RESOLVE_KEY = "synthetic-resolve-key";
const HASH = "sha256:76ebc8ee2673d4c79bf5d9809c02a163d6b57ab0f35194433b7d90205b5c19bd";
const MISMATCH_HASH = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const CONTENT = new TextEncoder().encode("synthetic note\n");

interface RecordedRequest {
  url: URL;
  init: RequestInit;
}

interface SyntheticVaultMutation {
  path: string;
  data: string;
}

class DeterministicFakeServer {
  readonly requests: RecordedRequest[] = [];

  readonly transport: HttpTransport = async (url, init) => {
    const request = { url: new URL(url), init };
    this.requests.push(request);
    return this.dispatch(request);
  };

  private dispatch({ url, init }: RecordedRequest): Response {
    const method = init.method ?? "GET";
    const headers = new Headers(init.headers);

    assert.equal(headers.get("Authorization"), `Bearer ${TOKEN}`);
    assert.equal(init.credentials, "omit");
    assert.equal(init.redirect, "error");

    if (method === "GET" && url.pathname === "/v1/server-info") {
      return jsonResponse({
        server_id: "synthetic-server",
        protocol_version: 1,
        max_upload_bytes: 52428800,
        capabilities: ["sha256", "operation_log", "tombstones", "conflicts", "conflict_center", "batch_changes"],
      });
    }

    if (method === "GET" && url.pathname === "/v1/changes") {
      assert.equal(url.searchParams.get("since"), "40");
      assert.equal(url.searchParams.get("limit"), "10");
      return jsonResponse({
        from_seq: 40,
        to_seq: 41,
        has_more: false,
        changes: [
          {
            seq: 41,
            kind: "upsert_file",
            path: "Synthetic/note.md",
            revision_id: "rev_synthetic_0002",
            content_sha256: HASH,
            size_bytes: CONTENT.byteLength,
            updated_by: "synthetic-adapter",
            updated_at: "2026-01-01T00:01:00Z",
          },
        ],
      });
    }

    if (method === "PUT" && url.pathname === "/v1/files/Synthetic/note.md") {
      assert.equal(headers.get("Idempotency-Key"), PUT_KEY);
      assert.equal(headers.get("X-Content-SHA256"), HASH);
      assert.equal(headers.get("X-Base-Revision-Id"), "null");
      assert.equal(init.body, "synthetic note\n");
      return jsonResponse({
        status: "accepted",
        path: "Synthetic/note.md",
        revision_id: "rev_synthetic_0002",
        seq: 41,
      });
    }

    if (method === "GET" && url.pathname === "/v1/files/Synthetic/note.md") {
      return new Response(CONTENT, {
        headers: {
          "Content-Type": "application/octet-stream",
          "X-Revision-Id": "rev_synthetic_0002",
          "X-Content-SHA256": HASH,
          "X-Size-Bytes": String(CONTENT.byteLength),
        },
      });
    }

    if (method === "DELETE" && url.pathname === "/v1/files/Synthetic/note.md") {
      assert.equal(headers.get("Idempotency-Key"), DELETE_KEY);
      assert.equal(headers.get("X-Base-Revision-Id"), "rev_synthetic_0002");
      return jsonResponse({
        status: "tombstoned",
        path: "Synthetic/note.md",
        tombstone_id: "tmb_synthetic_0001",
        seq: 42,
        retention_until: "2026-02-01T00:00:00Z",
      });
    }

    if (method === "GET" && url.pathname === "/v1/conflicts") {
      assert.equal(url.searchParams.get("status"), "open");
      return jsonResponse({
        conflicts: [
          {
            conflict_id: "conf_synthetic_0001",
            original_path: "Synthetic/note.md",
            conflict_path: "_haze_conflicts/open/Synthetic/note.conflict.md",
            current_revision_id: "rev_synthetic_0002",
            conflict_revision_id: "rev_synthetic_conflict",
            incoming_revision_id: "rev_synthetic_incoming",
            source_adapter_id: "synthetic-adapter",
            policy_applied: "preserve_both",
            status: "open",
            created_at: "2026-01-01T00:03:00Z",
            updated_at: "2026-01-01T00:04:00Z",
          },
        ],
      });
    }

    if (method === "POST" && url.pathname === "/v1/conflicts/conf_synthetic_0001/resolve") {
      assert.equal(headers.get("Idempotency-Key"), RESOLVE_KEY);
      assert.deepEqual(JSON.parse(String(init.body)), { resolution: "keep_both" });
      return jsonResponse({
        status: "resolved",
        conflict_id: "conf_synthetic_0001",
        resolution: "keep_both",
        seq: 43,
      });
    }

    return jsonResponse({ error: { code: "not_found", message: "Synthetic route not found" } }, 404);
  }
}

function createClient(server: DeterministicFakeServer): HazeSyncApiClient {
  return new HazeSyncApiClient({
    config: { serverUrl: "https://sync.example.test", authToken: TOKEN },
    transport: server.transport,
  });
}

function createEmptySyntheticVault(mutations: SyntheticVaultMutation[]): Vault {
  return {
    getAbstractFileByPath: () => null,
    create: async (path: string, data: string) => {
      mutations.push({ path, data });
      return undefined;
    },
  } as unknown as Vault;
}

function jsonResponse(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

test("deterministic fake Server exercises the accepted public sync surface", async () => {
  const server = new DeterministicFakeServer();
  const client = createClient(server);

  const info = await client.getServerInfo();
  assert.equal(info.protocol_version, 1);
  assert.ok(info.capabilities.includes("sha256"));
  assert.ok(info.capabilities.includes("conflict_center"));

  const changes = await client.getChanges({ since: 40, limit: 10 });
  assert.equal(changes.from_seq, 40);
  assert.equal(changes.to_seq, 41);
  assert.deepEqual(changes.changes.map((change) => change.seq), [41]);

  const put = await client.putFile({
    path: "Synthetic/note.md",
    body: "synthetic note\n",
    contentHash: HASH,
    baseRevisionId: null,
    idempotencyKey: PUT_KEY,
  });
  assert.equal(put.status, "accepted");

  const file = await client.getFile("Synthetic/note.md");
  assert.equal(file.metadata.revision_id, "rev_synthetic_0002");
  assert.equal(file.metadata.content_sha256, HASH);
  assert.equal(file.metadata.size_bytes, CONTENT.byteLength);
  assert.deepEqual(new Uint8Array(file.body), CONTENT);

  const change = changes.changes[0];
  assert.ok(change);
  const mutations: SyntheticVaultMutation[] = [];
  const materialized = await materializeRemoteChange({
    vault: createEmptySyntheticVault(mutations),
    change,
    download: file,
    baseRevisionState: createDefaultBaseRevisionState(),
    remoteSyncState: createDefaultRemoteSyncState(),
    observedAt: "2026-01-01T00:01:01Z",
  });
  assert.equal(materialized.status, "applied");
  assert.deepEqual(mutations, [{ path: "Synthetic/note.md", data: "synthetic note\n" }]);
  assert.equal(materialized.remoteSyncState.changeCursor, 41);
  assert.equal(materialized.baseRevisionState.byPath["Synthetic/note.md"]?.revisionId, "rev_synthetic_0002");

  const deleted = await client.deleteFile({
    path: "Synthetic/note.md",
    baseRevisionId: "rev_synthetic_0002",
    idempotencyKey: DELETE_KEY,
  });
  assert.equal(deleted.status, "tombstoned");

  const conflicts = await client.getConflicts({ status: "open" });
  assert.equal(conflicts.conflicts[0]?.conflict_id, "conf_synthetic_0001");

  const resolved = await client.resolveConflict({
    conflictId: "conf_synthetic_0001",
    resolution: "keep_both",
    idempotencyKey: RESOLVE_KEY,
  });
  assert.equal(resolved.resolution, "keep_both");
  assert.equal(server.requests.length, 7);
});

test("production materializer rejects a mismatched download before vault mutation or cursor advancement", async () => {
  const mutations: SyntheticVaultMutation[] = [];
  const result = await materializeRemoteChange({
    vault: createEmptySyntheticVault(mutations),
    change: {
      seq: 99,
      kind: "upsert_file",
      path: "Synthetic/mismatch.md",
      revision_id: "rev_synthetic_mismatch",
      content_sha256: MISMATCH_HASH,
      size_bytes: CONTENT.byteLength,
      updated_by: "synthetic-adapter",
      updated_at: "2026-01-01T00:09:00Z",
    },
    download: {
      metadata: {
        path: "Synthetic/mismatch.md",
        revision_id: "rev_synthetic_mismatch",
        content_sha256: MISMATCH_HASH,
        size_bytes: CONTENT.byteLength,
      },
      body: CONTENT.slice().buffer,
      contentType: "application/octet-stream",
    },
    baseRevisionState: createDefaultBaseRevisionState(),
    remoteSyncState: createDefaultRemoteSyncState(),
    observedAt: "2026-01-01T00:09:01Z",
  });

  assert.equal(result.status, "conflict_queued");
  assert.equal(result.reason, "hash_mismatch");
  assert.deepEqual(mutations, []);
  assert.equal(result.remoteSyncState.changeCursor, undefined);
  assert.equal(result.baseRevisionState.byPath["Synthetic/mismatch.md"], undefined);
});

test("public authorization, conflict, unavailable and validation failures stay categorized and redacted", async () => {
  const cases = [
    { status: 401, category: "unauthorized" },
    { status: 409, category: "conflict" },
    { status: 503, category: "server_unavailable" },
    { status: 422, category: "rejected" },
  ] as const;

  for (const item of cases) {
    const transport: HttpTransport = async () =>
      jsonResponse(
        {
          error: {
            code: item.status === 422 ? "validation_error" : "internal_error",
            message: `Bearer ${TOKEN} ${PUT_KEY} C:\\Users\\Synthetic\\vault\\note.md`,
          },
        },
        item.status,
      );
    const client = new HazeSyncApiClient({
      config: { serverUrl: "https://sync.example.test", authToken: TOKEN },
      transport,
    });

    await assert.rejects(
      () =>
        client.putFile({
          path: "Synthetic/note.md",
          body: "synthetic note\n",
          contentHash: HASH,
          baseRevisionId: null,
          idempotencyKey: PUT_KEY,
        }),
      (error: unknown) => {
        assert.ok(error instanceof ApiClientError);
        assert.equal(error.category, item.category);
        assert.ok(error.message.includes("[redacted]"));
        assert.ok(error.message.includes("[local path redacted]"));
        assert.ok(!error.message.includes(TOKEN));
        assert.ok(!error.message.includes(PUT_KEY));
        assert.ok(!error.message.includes("C:\\Users"));
        assert.ok(!error.message.includes("synthetic note"));
        return true;
      },
    );
  }
});

test("absolute local paths are fully redacted while safe public route text remains", async () => {
  const paths = [
    "C:\\Users\\John Doe\\vault\\note.md",
    "C:/Users/John/vault/note.md",
    "\\\\server\\share\\note.md",
    "/home/john doe/vault/note.md",
    "/mnt/data/note.md",
    "/Volumes/My Vault/note.md",
    "/storage/emulated/0/Notes/note.md",
    '"/Volumes/My Vault/folder"',
  ];

  for (const localPath of paths) {
    const transport: HttpTransport = async () =>
      jsonResponse(
        {
          error: {
            code: "validation_error",
            message: `Failed at ${localPath}; public route /v1/files remains.`,
          },
        },
        422,
      );
    const client = new HazeSyncApiClient({
      config: { serverUrl: "https://sync.example.test", authToken: TOKEN },
      transport,
    });

    await assert.rejects(
      () => client.getChanges({ since: 0, limit: 1 }),
      (error: unknown) => {
        assert.ok(error instanceof ApiClientError);
        assert.equal(error.category, "rejected");
        assert.ok(error.message.includes("[local path redacted]"));
        assert.ok(error.message.includes("public route /v1/files remains"));
        assert.ok(!error.message.includes(localPath.replace(/"/gu, "")));
        return true;
      },
    );
  }
});
