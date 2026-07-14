# W1-CLI-P6A — Worktree status and explicit sync-once operator commands

Before starting, name this worker chat exactly:

`cli — W1 CLI-P6A Worktree Operator Commands`

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: implementation-worker
Phase: CLI-P6A-WORKTREE-OPERATOR-COMMANDS

Do not merge, change draft state, rewrite history, modify sibling branches, or perform unrelated cleanup.

## Accepted baselines

CLI pre-sync:

- exact post-sync SHA: `8a3012a20440066422e7ad6c4e52d1a859b1bd51`;
- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`;
- pre-sync CI run `29329332012`, number `1941`, success.

Accepted API-P8:

- code-bearing SHA: `56ae94570441d68715f34b5d54381a0fc4d7c231`;
- clean report blob: `401dc1c2fe9d5ce91a1aa248be8ed7d27a8275d7`;
- accepted product blobs:
  - `crates/haze-sync-api/src/dto/worktree.rs` → `7c592184d58c1cda98314fd0e2eae8baed1de1e8`;
  - `crates/haze-sync-api/src/dto/mod.rs` → `5534f79606639fb13857de0729793d9083523e03`;
  - `crates/haze-sync-api/src/dto/public_contract_tests.rs` → `903fea0d7cdc28a8eda8140cff23f521db58e061`;
  - `crates/haze-sync-api/src/routes/worktree.rs` → `032f43a1a513e7fb25d6103de281f7e5d8e08930`;
  - `crates/haze-sync-api/src/routes/mod.rs` → `439bebd09d3aa9252188d2d3eb106e65943c5846`;
  - `crates/haze-sync-api/fixtures/worktree-contract-v1.json` → `da0a197b1e6d5425f05cfd6fe772a4c96f6a830c`;
  - `crates/haze-sync-api/tests/worktree_compatibility_fixture.rs` → `968f1e9825a2c54385615bf75db54a975218fd20`;
  - `crates/haze-sync-api/docs/worktree-status-contract.md` → `f2c8d2d53c09a0e1ac0caf3ac8c6ee8d15754009`.

Accepted Server HTTP surface:

- code-bearing SHA: `50461354c18ddc4d2e47202d9303b4358a27ee45`;
- clean report blob: `e3271abaf3d667f9ffd4f4ff0652e5d26892b9e5`;
- DB-capable CI run `29326558901`, number `1940`, success;
- GET `/v1/admin/worktree/status`;
- POST `/v1/admin/worktree/sync-once`;
- exact HTTP and public body semantics pinned below.

The accepted API-P8 files are not yet present in current `main`. Fan them into the CLI branch exactly as needed; do not invent duplicate DTOs when the accepted API crate can be consumed directly.

## Goal

Add explicit operator commands that consume the accepted public Server/API contract without hosting runtime work locally:

- `haze-sync worktree status`;
- `haze-sync worktree sync-once`.

The CLI is a remote administrative client only.

## Exact API fan-in

1. Copy the accepted API-P8 product files required for workspace compilation/tests exactly from SHA `56ae945...`.
2. Preserve every listed accepted blob SHA exactly.
3. Do not copy API control files.
4. Do not semantically edit accepted API-P8 source or vocabulary.
5. Record any additional exact accepted-source dependency required by module/test references.

## Command model

Add a `worktree` command group:

```text
haze-sync worktree status
haze-sync worktree sync-once
```

Requirements:

- deterministic dependency-free parser;
- no client-controlled path, budget, mode, force, ticket, generation, request id or arbitrary JSON flags;
- reject unknown/trailing arguments with existing safe non-echoing parse errors;
- root usage lists both commands;
- command-specific help may be added if it fits the existing model without broad parser redesign;
- do not overload the existing global `status` command or change its accepted behavior.

## Client boundary

Extend the existing Server client abstraction with minimal Worktree operations:

- fetch one accepted `WorktreeStatusResponse`;
- submit one strict-empty `WorktreeSyncOnceRequest` and receive `WorktreeSyncOnceResponse` plus HTTP classification.

Use accepted API types directly where practical. Do not recreate public enum vocabulary in CLI-local duplicate types unless compilation proves a narrowly justified adapter is required.

A concrete HTTP transport may remain deferred if config/token/network transport is still outside the current accepted CLI boundary. However, the command execution, request construction, response classification, rendering and test client boundary must be complete and ready for a later transport implementation. Do not add a fake success transport.

## Status command behavior

`haze-sync worktree status`:

- requires configured live Server access through the client boundary;
- no offline substitute and no local filesystem/runtime inspection;
- request is exactly GET `/v1/admin/worktree/status`;
- render deterministic human-readable output containing only accepted fields:
  - configured mode;
  - lifecycle;
  - readiness;
  - readiness reason;
  - cycles completed/failed;
  - cycle in progress;
  - pending watcher hints;
  - manual availability;
- Running+Busy must display readiness `ready`, not infer not-ready;
- counters/hints are informational only;
- return success for any valid HTTP 200 accepted status body, including Disabled and Busy;
- map auth, unavailable, internal and invalid-response conditions to safe existing CLI runtime errors without raw response/body/token leakage.

## Sync-once behavior

`haze-sync worktree sync-once`:

- constructs only the accepted strict-empty request `{}`;
- request is exactly POST `/v1/admin/worktree/sync-once`;
- this is explicit operator mutation, so help/output must state that it requests one bounded server-owned DryRun cycle;
- accepted means queued/submitted only, not completed;
- do not poll, retry, wait, or expose tickets/generations;
- render deterministic outcome text for every accepted status:
  - accepted;
  - busy;
  - not_started;
  - cancelling;
  - shutdown;
  - unavailable;
  - failed.

Exit semantics:

- `accepted` / HTTP 202 -> success;
- `busy` / HTTP 409 -> distinct non-zero runtime result with safe message;
- `not_started`, `cancelling`, `shutdown`, `unavailable` / HTTP 503 -> non-zero runtime result;
- `failed` / HTTP 500 -> non-zero runtime result;
- missing/invalid token / HTTP 401 -> safe unauthorized result;
- non-admin / HTTP 403 -> safe forbidden result;
- malformed or contract-invalid response -> safe invalid-response result.

Do not infer success from HTTP status alone if the public response body contradicts it. Reject mismatched HTTP/body combinations safely.

## Configuration and secrecy

- Reuse existing `CliConfig` / `ServerUrl` boundaries.
- Do not add token values to command-line flags.
- Do not print server credentials, bearer tokens, raw headers, raw response bodies, paths, roots, fingerprints, database URLs, provider payloads, request payloads, cursor/idempotency values, tickets or generations.
- Parser and renderer errors must not echo sensitive raw arguments.
- No environment dump or debug rendering of secrets.

## Tests

Add deterministic tests for:

- parser and root usage for both Worktree commands;
- rejection of trailing/unknown and forbidden control arguments without echo;
- exact GET/POST method/path and strict `{}` request construction;
- direct use/roundtrip of accepted API-P8 DTO vocabulary;
- status rendering for Disabled, Running+Busy, Failed and extreme counters/hints;
- Running+Busy remains displayed as ready;
- every sync-once outcome and exact exit classification;
- accepted explicitly says submitted/queued and not completed;
- HTTP/body mismatch rejected;
- auth/unavailable/internal/invalid response safe mappings;
- no polling, retry, ticket or completion behavior in the client boundary;
- no local DB/filesystem/provider/runtime work;
- no sensitive marker in stdout/stderr;
- existing CLI-P1..P5 commands and tests remain unchanged/green.

## Allowed scope

- `crates/haze-sync-cli/src/commands.rs`;
- `crates/haze-sync-cli/src/main.rs`;
- `crates/haze-sync-cli/src/server_api.rs` or a focused `worktree_api.rs` module;
- `crates/haze-sync-cli/src/output.rs` only for minimal stable exit classification;
- focused CLI tests/docs;
- `crates/haze-sync-cli/Cargo.toml` only for the minimal accepted API dependency or serialization support proven necessary;
- exact accepted API-P8 product files listed above;
- CLI control report.

## Forbidden scope

- local Worktree runtime hosting or direct Worktree crate dependency;
- DB, object-store, filesystem or provider calls;
- Server product changes;
- Storage/Core/Worktree/GDrive/Deployment product changes;
- token CLI arguments or secret persistence;
- retries, polling, completion waits, background tasks or daemon ownership;
- repair/destructive commands;
- migrations/workflows;
- broad CLI framework replacement or unrelated cleanup.

Formatting/style alone is non-blocking when exact-SHA CI is green.

## Completion

Create a real code-bearing commit without CI skip and obtain authoritative Component CI success on its exact final SHA.

Write `crates/haze-sync-cli/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: CLI-P6A-WORKTREE-OPERATOR-COMMANDS`;
- `chat_name: cli — W1 CLI-P6A Worktree Operator Commands`;
- status `SELF_ACCEPT`, `NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_SCOPE`, or `BLOCKED_BY_TOOLING`.

Record exact API blob verification, command/client/exit semantics, changed paths, secrecy checks, final SHA and exact CI evidence. Do not claim CLEAN_ACCEPT, begin Deployment work, merge, or change PR draft state. A focused CLI functional review follows.
