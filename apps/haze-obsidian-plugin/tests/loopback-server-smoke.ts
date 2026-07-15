import assert from "node:assert/strict";

import { HazeSyncApiClient } from "../src/api-client";

async function main(): Promise<void> {
  const serverUrl = process.env.HAZE_OBSIDIAN_SMOKE_SERVER_URL?.trim();
  const authToken = process.env.HAZE_OBSIDIAN_SMOKE_AUTH_TOKEN?.trim();

  if (!serverUrl || !authToken) {
    console.log(
      "SKIP loopback Server smoke: set HAZE_OBSIDIAN_SMOKE_SERVER_URL and HAZE_OBSIDIAN_SMOKE_AUTH_TOKEN.",
    );
    return;
  }

  const parsedUrl = new URL(serverUrl);
  assert.ok(
    parsedUrl.hostname === "localhost" || parsedUrl.hostname === "127.0.0.1" || parsedUrl.hostname === "[::1]",
    "Loopback smoke accepts only localhost/127.0.0.1/[::1] Server URLs.",
  );

  const client = new HazeSyncApiClient({
    config: { serverUrl: parsedUrl.toString(), authToken },
  });

  const info = await client.getServerInfo();
  assert.equal(info.protocol_version, 1);
  assert.ok(info.capabilities.includes("sha256"));

  const changes = await client.getChanges({ since: 0, limit: 1 });
  assert.ok(changes.from_seq <= changes.to_seq);
  assert.ok(changes.changes.length <= 1);

  console.log("PASS loopback Server smoke: server-info and bounded changes request validated.");
}

void main().catch((error: unknown) => {
  console.error(error instanceof Error ? error.message : "Loopback Server smoke failed.");
  process.exitCode = 1;
});
