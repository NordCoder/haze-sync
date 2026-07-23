# Component Contract: cli

## Responsibility

`haze-sync-cli` is the operator-facing command-line client for Haze Sync. It owns command parsing, bounded configuration and token-source loading, authenticated calls to accepted public Server HTTP surfaces, safe output rendering, and stable process exit codes.

The CLI is not the Core policy owner, Server runtime, Storage layer, provider adapter, Worktree runtime, Obsidian plugin, or Deployment controller.

## Implemented command surface

```text
haze-sync --help
haze-sync status [--offline]
haze-sync adapters list [--offline]
haze-sync doctor [--offline]
haze-sync doctor --live
haze-sync worktree status
haze-sync worktree sync-once
haze-sync preflight
haze-sync bootstrap plan
haze-sync recovery plan
haze-sync rollout plan
```

Global options:

```text
--config <path>
--profile <name>
--server-url <url>
--output <human|text|json>
--token-source <none|env:NAME|file:PATH|stdin|os-secret:SERVICE/ACCOUNT>
```

## Current behavior

The component implements:

- explicit dependency-free command parsing;
- config precedence across command-line overrides, environment, bounded profile files, and defaults;
- validated Server URLs and redacted token-source descriptors;
- authenticated HTTP transport for accepted Server endpoints;
- read-only status, adapter, doctor, and Worktree status commands;
- bounded Server-owned `worktree sync-once`, available only when the Server contract allows it;
- `preflight`, which aggregates live health, readiness, admin status, adapter list, and Worktree runtime status;
- no-write bootstrap, recovery, and rollout plans;
- stable human output and a versioned JSON envelope;
- exit-code categories `0`, `1`, and `2`;
- safe partial output when a live surface fails.

`preflight` returns success only when all required live surfaces succeed and Worktree reports ready state, zero failed cycles, and no failed lifecycle.

The plan commands are intentionally not execution commands. They always state:

```text
dry_run
writes: none
```

They do not change Server, PostgreSQL, providers, adapter modes, deployment state, or vault files.

## Public HTTP dependencies

The CLI may consume only accepted public Server/API surfaces, including:

```text
GET  /health
GET  /ready
GET  /v1/admin/status
GET  /v1/admin/adapters
GET  /v1/admin/worktree/status
POST /v1/admin/worktree/sync-once
```

The CLI must not compensate for a missing public contract through direct SQL, object-store access, provider internals, or Server-private state.

## Input contracts

- Parse arguments into an explicit command model before execution.
- Reject unknown commands, flags, and unexpected arguments without echoing raw values.
- Offline commands must not read config files, tokens, stdin, OS secrets, or the network when their contract says no live access is required.
- Live commands require a validated Server URL.
- Raw bearer tokens must not be accepted as ordinary positional or flag values.
- Token-source metadata must be validated before resolution.
- Mutating or destructive commands require an accepted upstream contract, explicit scope, confirmation semantics, and audit behavior.

## Output contracts

Human output uses stdout for safe summaries and stderr for safe failures. Runtime failures may retain sanitized partial stdout.

Exit codes:

- `0` — command completed successfully;
- `1` — runtime/config/auth/network/readiness/preflight failure;
- `2` — usage or parse error.

Machine output uses the compatibility envelope:

```json
{
  "schema": "haze-sync.cli.output.v1",
  "command": "preflight",
  "ok": true,
  "exit_code": 0,
  "stdout": "...",
  "stderr": ""
}
```

Rules:

- the field set and schema identifier are deterministic;
- process exit code is preserved in JSON mode;
- the envelope is emitted on stdout;
- the embedded stdout/stderr strings remain sanitized;
- raw tokens, hashes, database URLs, provider cursors, request bodies, file contents, and internal errors are forbidden.

## Security and secrecy

Do not commit or print:

- bearer or OAuth tokens;
- token hashes or token values;
- database URLs or passwords;
- object-store roots;
- absolute secret paths;
- provider cursors or payloads;
- Idempotency-Key values;
- raw SQLx, provider, or Server internal errors;
- stack traces.

Configuration summaries may report only safe labels such as selected profile, whether a Server URL is configured, output format, and token-source kind.

## Ownership boundaries

CLI may own:

- parser and command model;
- config/token-source resolution;
- authenticated public HTTP calls;
- safe rendering and JSON compatibility;
- exit-code mapping;
- operator preflight and no-write plans.

CLI does not own:

- Core conflict/delete/revision policy;
- API route or DTO definitions;
- Server lifecycle or private runtime state;
- Storage schema or repositories;
- provider OAuth/API calls;
- Worktree/Obsidian/GDrive execution loops;
- deployment service mutation;
- backup/restore execution;
- adapter enable/disable or mode mutation without public upstream contracts.

## Current non-goals and blockers

The following are not implemented and must not be implied by plan output:

- live coordinated backup or restore;
- adapter enable/disable or staged mode mutation;
- GDrive dry-run/import/export execution;
- conflict resolution mutation;
- token creation or rotation;
- destructive repair or cleanup;
- direct database/provider administration;
- package publishing or release signing.

These require explicit Server/API/Core/Deployment contracts and later acceptance evidence.

## Invariants

- CLI is a client/operator surface, not runtime authority.
- Offline means no network or secret resolution.
- Skipped or not-run checks are never reported as successful live evidence.
- Unhealthy live values return exit code `1`.
- `preflight: ready` requires all included checks to pass.
- Plan commands perform zero writes.
- JSON output is versioned, deterministic, and secret-safe.
- Missing mutation contracts block implementation rather than justify direct DB/provider access.

## Test obligations

Expected checks:

```bash
cargo fmt --all --check
cargo check -p haze-sync-cli --locked
cargo test -p haze-sync-cli --locked
cargo clippy -p haze-sync-cli --all-targets --locked -- -D warnings
```

Integration evidence must also cover:

- authenticated CLI access to a real Server and PostgreSQL deployment;
- human and JSON `preflight` success;
- exact JSON v1 fields and exit code;
- no-write plan output;
- token markers absent from CLI and Server logs;
- existing Worktree and Obsidian vertical-slice behavior remaining green.

## Contract change protocol

Request architecture/orchestration direction before adding:

- new Server/API routes or DTOs;
- direct Storage/provider dependencies;
- mutation, repair, restore, or destructive commands;
- new secret transport or deployment layout;
- JSON v1 field changes;
- claims of operational success without matching live evidence.
