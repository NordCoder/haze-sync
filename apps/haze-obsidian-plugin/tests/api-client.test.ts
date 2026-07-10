import assert from "node:assert/strict";
import test from "node:test";

import {
  ApiClientError,
  HazeSyncApiClient,
  type HttpTransport,
} from "../src/api-client";

interface CapturedRequest {
  url: URL;
  init: RequestInit;
}

function jsonResponse(payload: unknown, status = 200): Response {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function clientWithTransport(handler: (request: CapturedRequest) => Response | Promise<Response>): HazeSyncApiClient {
  const transport: HttpTransport = (url, init) => handler({ url: new URL(url), init });
  return new HazeSyncApiClient({
    config: {
      serverUrl: "https://sync.example.test",
      authToken: "synthetic-test-token",
    },
    transport,
  });
}

test("changes request uses canonical cursor query and validates response shape", async () => {
  const client = clientWithTransport(({ url, init }) => {
    assert.equal(init.method, "GET");
    assert.equal(url.pathname, "/v1/changes");
    assert.equal(url.searchParams.get("since"), "40");
    assert.equal(url.searchParams.get("limit"), "10");
    return jsonResponse({ from_seq: 40, to_seq: 40, has_more: false, changes: [] });
  });

  const response = await client.getChanges({ since: 40, limit: 10 });
  assert.equal(response.from_seq, 40);
  assert.equal(response.to_seq, 40);
});

test("PUT and DELETE requests preserve idempotency and base revision headers", async () => {
  let requestCount = 0;
  const client = clientWithTransport(({ url, init }) => {
    requestCount += 1;
    const headers = new Headers(init.headers);
    assert.equal(headers.get("Authorization"), "Bearer synthetic-test-token");

    if (requestCount === 1) {
      assert.equal(init.method, "PUT");
      assert.equal(url.pathname, "/v1/files/Notes/fixture.md");
      assert.equal(headers.get("Idempotency-Key"), "put-key");
      assert.equal(headers.get("X-Base-Revision-Id"), "rev_fixture_0001");
      assert.equal(
        headers.get("X-Content-SHA256"),
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      );
      return jsonResponse({
        status: "accepted",
        path: "Notes/fixture.md",
        revision_id: "rev_fixture_0002",
        seq: 41,
      });
    }

    assert.equal(init.method, "DELETE");
    assert.equal(headers.get("Idempotency-Key"), "delete-key");
    assert.equal(headers.get("X-Base-Revision-Id"), "null");
    return jsonResponse({
      status: "not_found",
      path: "Notes/missing.md",
    });
  });

  const put = await client.putFile({
    path: "Notes/fixture.md",
    body: new Uint8Array([1, 2, 3]),
    contentHash: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    baseRevisionId: "rev_fixture_0001",
    idempotencyKey: "put-key",
  });
  assert.equal(put.status, "accepted");

  const deleted = await client.deleteFile({
    path: "Notes/missing.md",
    baseRevisionId: null,
    idempotencyKey: "delete-key",
  });
  assert.equal(deleted.status, "not_found");
});

test("conflict resolution uses canonical resolution body and route path id", async () => {
  const client = clientWithTransport(async ({ url, init }) => {
    assert.equal(init.method, "POST");
    assert.equal(url.pathname, "/v1/conflicts/conf_fixture_0001/resolve");
    assert.deepEqual(JSON.parse(String(init.body)), { resolution: "keep_both" });
    assert.equal(new Headers(init.headers).get("Idempotency-Key"), "resolve-key");
    return jsonResponse({
      status: "resolved",
      conflict_id: "conf_fixture_0001",
      resolution: "keep_both",
      seq: 46,
    });
  });

  const response = await client.resolveConflict({
    conflictId: "conf_fixture_0001",
    resolution: "keep_both",
    idempotencyKey: "resolve-key",
  });
  assert.equal(response.resolution, "keep_both");
});

test("invalid or permissive legacy payloads fail as invalid responses", async () => {
  const client = clientWithTransport(() =>
    jsonResponse({
      changes: [{ sequence: 1, kind: "upsert", path: "Notes/legacy.md" }],
      next_since: 1,
    }),
  );

  await assert.rejects(
    () => client.getChanges({ since: 0, limit: 50 }),
    (error: unknown) => error instanceof ApiClientError && error.category === "invalid_response",
  );
});
