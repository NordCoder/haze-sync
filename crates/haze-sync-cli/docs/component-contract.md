# Component Contract: cli

## Responsibility

`haze-sync-cli` owns the operator-facing command-line interface for Haze Sync.

The CLI may parse commands, validate operator inputs, render safe status and doctor summaries, call accepted public Server HTTP surfaces through injected client boundaries, and map outcomes to stable process exit codes. It is not the Core policy owner, Server runtime, Storage repository layer, provider adapter, Deployment runtime, Worktree runtime, or Obsidian plugin.

## Current implemented behavior

The current component implements a read-only operator foundation:

- dependency-free parsing for root help, `status`, `adapters list`, and `doctor`;
- explicit stdout, stderr, and exit-code channels;
- CLI-local config and secret-source models with redacted debug/report output;
- read-only Server/API client boundaries for admin status and adapter summaries;
- `doctor` and `doctor --offline` as offline-by-default, no-network diagnostics;
- explicit `doctor --live` aggregation over accepted public Server surfaces:
  - `GET /health`;
  - `GET /ready`;
  - `GET /v1/admin/status`;
- mapping of sanitized readiness data into Core doctor result models;
- honest `skipped` or `not_run` results where no accepted public diagnostic surface exists;
- safe partial-result output when one live surface fails.

The current binary still uses default in-memory config and a deferred transport implementation. It does not load environment variables, config files, tokens, stdin, keychains, or perform real HTTP calls. Live mode therefore reports `not_run` when no server URL is available and must not claim that runtime checks passed.

## Public interfaces

Current binary surface:

```text
haze-sync --help
haze-sync help
haze-sync status [--offline]
haze-sync adapters list [--offline]
haze-sync doctor [--offline]
haze-sync doctor --live
```

Command categories:

- `status` and `adapters list` are read-only Server/API summaries with honest offline/not-configured behavior;
- `doctor` and `doctor --offline` are read-only local/offline summaries and never attempt network access;
- `doctor --live` is read-only and network-backed only when validated config and a concrete transport are supplied;
- no current command is an admin mutation, repair operation, provider call, or destructive action.

Current internal module responsibilities:

```text
src/commands.rs     command model and parsing
src/config.rs       safe config and secret-source models
src/doctor.rs       doctor modes, parsing, Core report rendering
src/doctor_live.rs  accepted live diagnostic aggregation boundary
src/output.rs       stdout/stderr/exit-code model
src/server_api.rs   read-only Server/API status and adapter boundary
src/main.rs         process wiring
```

## Input contracts

CLI inputs may come from process arguments and, in future explicitly scoped phases, validated config or secret sources.

Required rules:

- parse arguments into explicit command models before execution;
- reject unknown flags and unexpected arguments without echoing raw values;
- default doctor mode to offline;
- require `--live` explicitly before attempting live doctor behavior;
- reject conflicting doctor modes;
- validate server URL and secret-source metadata before any future network call;
- do not accept raw tokens in ways that encourage shell-history exposure;
- do not silently turn placeholder or offline commands into network-backed commands;
- require explicit scope and confirmation for any future mutation or destructive behavior.

## Output contracts

CLI outputs are split into stdout summaries, stderr errors, and process exit codes.

Exit-code contract:

- `0` — command parsed and completed successfully;
- `1` — runtime execution could not start, a required live surface failed, or a live diagnostic result was unhealthy;
- `2` — usage or parse error.

Read-only successful summaries go to stdout. Safe runtime and parse explanations go to stderr. A runtime failure may preserve sanitized partial stdout when useful to operators.

Live doctor output must distinguish:

- offline mode;
- live mode;
- checks that completed;
- checks that were skipped because no public surface exists;
- checks that were not run because config or a required surface was unavailable;
- healthy and unhealthy values returned by successful public responses.

A successful HTTP response containing `not_ready`, degraded, maintenance, failed Core doctor checks, or equivalent unhealthy state is not a successful doctor command and must return exit code `1`.

Machine-readable output, if introduced later, becomes a compatibility contract and must be deterministic and secret-safe.

## Error contracts

CLI errors must be stable, actionable, and sanitized.

Current parse errors include:

- unknown command;
- missing or unknown adapters command;
- unexpected argument;
- unknown doctor flag;
- unexpected doctor argument;
- conflicting doctor modes.

Runtime errors may classify:

```text
config_error
auth_error
network_error
server_unavailable
forbidden
not_ready
doctor_failure
invalid_response
unsupported_command
unsafe_operation_blocked
internal_error
```

Raw upstream error strings must not be forwarded directly. CLI owns mapping accepted API/Server/Core outcomes to safe operator text; upstream components retain ownership of policy and public DTO/error vocabulary.

## Persistence and runtime ownership

CLI may own:

- command parsing and validation;
- output formatting;
- process exit-code mapping;
- local CLI config models and source precedence;
- future safe config loading when explicitly scoped;
- HTTP client calls to accepted public Server/API surfaces;
- rendering accepted Server/API/Core summaries;
- future confirmation UX for explicitly scoped mutations.

CLI does not own:

- Core conflict, delete, revision, or idempotency policy;
- API DTO/header/error definitions;
- Server route registration, listener startup, or private runtime state;
- Storage schema, migrations, repositories, or direct database access;
- provider API calls, OAuth validation, or adapter loops;
- Worktree or Obsidian runtime behavior;
- repair execution, hard delete, or background sync loops.

Live behavior must use public Server/API contracts. CLI must not work around missing contracts by reaching into Storage, provider implementations, or Server-private state.

## Security and secrecy rules

Do not commit or print:

- bearer tokens or OAuth tokens;
- token hashes or secret values;
- Idempotency-Key values;
- database URLs;
- object-store roots;
- local absolute secret paths;
- raw external cursor values;
- provider payloads;
- request bodies or file contents;
- stack traces;
- raw SQLx, provider, or Server internal errors.

Config debug and summaries must remain redacted. Public adapter output may indicate cursor presence but must not expose cursor content. Doctor output must be safe for copy/paste into reports.

## Non-goals

The current component must not implement:

- admin mutations;
- token creation or rotation;
- repair or destructive cleanup;
- direct database or object-store inspection;
- provider/GDrive calls or OAuth validation;
- Server/API route or DTO definitions;
- Core policy decisions;
- hidden background synchronization;
- Deployment service management;
- concrete HTTP/config/secret IO unless a later phase explicitly scopes it.

## Dependencies

See `dependency-map.md`.

The CLI may consume accepted Core doctor models, Common value types, and public Server/API contracts. It must not depend on Storage internals, provider internals, Server-private state, deployment secrets, or workflow implementation details.

## Dependents

See `dependency-map.md`.

Expected consumers are human operators, future scripts using stable output, Deployment runbooks, support workflows, and CI/package validation.

## Invariants

- CLI is an operator surface, not runtime authority or source of truth.
- Offline commands never perform network access.
- Live commands are explicit and use public Server/API contracts only.
- CLI never claims a live check passed when it was skipped, not run, or only modeled through a deferred transport.
- Unhealthy live diagnostic values return a non-zero runtime exit code.
- Partial output remains sanitized.
- Missing public diagnostic surfaces are reported honestly instead of bypassed.
- Unsafe, mutating, or destructive operations require future explicit contracts and confirmation semantics.

## Test obligations

Tests should cover:

- root, status, adapters, doctor offline, and doctor live parsing;
- conflicting modes and unsupported arguments;
- stdout, stderr, and exit-code separation;
- offline mode making no live client calls;
- missing config producing `not_run` and exit code `1`;
- accepted health/readiness/status endpoint modeling;
- healthy live results returning `0`;
- not-ready, degraded, maintenance, failed Core checks, and surface errors returning `1`;
- partial live output remaining available and sanitized;
- skipped unsupported checks being labeled honestly;
- absence of tokens, hashes, URLs, local paths, raw cursors, provider payloads, and stack traces;
- no default requirement for production services, credentials, providers, or databases.

Expected checks when shell or CI is available:

```bash
cargo fmt --all --check
cargo check -p haze-sync-cli
cargo test -p haze-sync-cli
cargo clippy -p haze-sync-cli --all-targets -- -D warnings
```

Connector-only workers must not claim shell checks passed unless they actually ran them or observed CI metadata.

## Deferred work

Deferred until explicitly scoped:

- real config and secret-source loading;
- concrete authenticated HTTP transport and response decoding;
- full live end-to-end tests against Server;
- stable JSON output;
- bootstrap, sync, mutation, token, repair, and destructive workflows.

## Contract change protocol

Request Architect/Orchestrator direction instead of silently broadening scope when implementation would require:

- new Server/API routes or DTOs;
- direct Storage or provider dependencies;
- Core policy inside CLI;
- production secret layout changes;
- raw sensitive output;
- mutation, repair, or destructive commands;
- claims of live validation without real accepted surfaces and transport.
