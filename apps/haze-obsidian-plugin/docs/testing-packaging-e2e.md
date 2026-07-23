# Testing, packaging, and local E2E readiness

## Supported validation commands

Run commands from the repository root unless noted otherwise.

```bash
npm ci
npm test --workspace haze-obsidian-plugin
npm run typecheck --workspace haze-obsidian-plugin
npm run build --workspace haze-obsidian-plugin
```

The current `build` command is a source/test type-validation gate. It intentionally does not
produce an installable Obsidian bundle. The test command compiles TypeScript into the ignored
`.test-dist/` directory, runs Node's built-in test runner, and compares the vendored API fixture
with the canonical workspace fixture when that accepted fixture is present.

## Server compatibility integration harness

Normal CI uses `tests/server-compatibility-e2e.test.ts`, which supplies a deterministic fake HTTP
transport to the production API client. It uses only synthetic paths, content, adapter identifiers,
tokens, idempotency keys, revisions, conflicts, and timestamps. It requires no public network,
database, provider, credentials, or real vault.

The harness covers:

- server-info protocol/capability negotiation;
- canonical changes pagination and ordered sequence values used by cursor persistence;
- upload headers for idempotency, content hash, and explicit null base;
- download metadata/body boundary for revision, content hash, size, and bytes;
- guarded delete construction with an explicit base revision;
- canonical conflict listing and a currently supported Server-routed `keep_both` action;
- authorization, conflict, unavailable, and validation error categories;
- redaction of configured tokens, mutation keys, Windows/Unix absolute paths, and raw content.

API fixture compatibility remains separately enforced by `tests/api-compatibility.test.ts`.
Neither harness invents routes, DTO fields, conflict actions, or Server policy.

## Optional loopback Server smoke

The smoke command is opt-in and read-only:

```bash
HAZE_OBSIDIAN_SMOKE_SERVER_URL=http://127.0.0.1:3000 \
HAZE_OBSIDIAN_SMOKE_AUTH_TOKEN='<synthetic-test-token>' \
npm run test:server-smoke --workspace haze-obsidian-plugin
```

It accepts only `localhost`, `127.0.0.1`, or `[::1]`, then validates `/v1/server-info` and a bounded
`/v1/changes?since=0&limit=1` response. When either variable is absent it exits successfully with
an explicit `SKIP` message. Do not use production URLs, real adapter tokens, real vault data, or
provider credentials.

## Generated artifact policy

Generated files are not source-of-truth and are not tracked in this phase:

- `main.js` and source maps;
- `.test-dist/`;
- release zip archives;
- temporary test vaults, plugin data, logs, and server state.

A future accepted packaging phase may add a bundler and produce `main.js` for installation.
Until then, the source tree must not be presented as a marketplace-ready or directly
installable release. Generated artifacts must never be committed merely to make a local test
pass.

## Synthetic local test environment

Use only disposable synthetic data.

1. Start a local Haze Sync test server using its documented development configuration.
2. Create a new empty Obsidian vault outside any real notes directory.
3. Use a synthetic adapter identity such as `obsidian-e2e` and a dedicated test token.
4. Add only deterministic files such as:
   - `Notes/alpha.md` containing `alpha-v1`;
   - `Notes/delete-me.md` containing `delete-v1`;
   - `Canvas/sample.canvas` containing a minimal synthetic JSON document.
5. Keep automatic triggers disabled for deterministic manual scenarios.
6. Reset the test server and delete the disposable vault after the run.

Never reuse a production server, real token, real vault, personal note, provider credential,
or external provider cursor in tests.

## Deterministic E2E scenarios

### 1. Scan and upload

- Run `Scan vault for local changes`.
- Verify the pending queue reports the synthetic files.
- Run `Sync now` in `push_only` mode.
- Verify accepted responses clear only the matching pending operations and store returned base
  revisions.

### 2. Same-content retry

- Re-run sync without changing a file.
- Verify an API `ignored/same_content` outcome clears the matching pending operation without
  inventing revision metadata.

### 3. Pull and materialization

- Add a canonical `upsert_file` operation through the test server.
- Run `Sync now` in `pull_only` mode.
- Verify downloaded bytes match `content_sha256`, the local file is materialized through the
  Obsidian Vault API, and the cursor advances only after successful handling.

### 4. Dirty-local conflict stop

- Modify the local file after its base is recorded and publish a different remote revision.
- Run pull.
- Verify the local file is preserved, a local pull conflict is recorded, and the cursor does
  not advance past the blocked operation.

### 5. Server conflict center

- Create a synthetic open conflict on the test server.
- Verify the conflict center displays canonical route fields and safe timestamps.
- Verify `accept_current`, `keep_both`, and `mark_resolved` use the Server route.
- Verify `accept_conflict` is shown as reserved/unavailable because API vocabulary presence
  does not imply current Server execution support.

### 6. Delete/tombstone safety

- Delete `Notes/delete-me.md` locally and scan.
- With `Confirm delete actions` enabled, verify the pending delete remains queued.
- In a disposable environment only, disable the guard and run sync.
- Verify a fresh Vault absence check occurs before the DELETE request and no local hard-delete
  operation is performed by the plugin.

### 7. Offline and backoff

- Stop the test server and trigger sync.
- Verify the user sees only sanitized offline/backoff status.
- Verify manual sync may retry immediately and automatic retries remain bounded and owned by
  the active plugin lifecycle.

### 8. Status and doctor fixture compatibility

- Run the TypeScript fixture tests.
- Verify canonical status/doctor vocabularies, safe integer sequences, admin adapter counts,
  and omitted-versus-null fields remain accepted exactly.

## Local installation status

The manifest remains suitable for compatibility validation, but this phase does not create a
release bundle. Installing into `.obsidian/plugins/haze-sync/` requires a future accepted
bundle containing at least `manifest.json` and generated `main.js`. Do not claim local install
success until that packaging output exists and has been tested in a disposable vault.
