# Implementation Plan: cli

## Current state

`haze-sync-cli` is a small operator CLI scaffold with limited read-only behavior.

Implemented current-state surface:

- binary package named `haze-sync`;
- dependency-free parser for:
  - `haze-sync --help` / `haze-sync help`;
  - `haze-sync status`;
  - `haze-sync adapters list`;
- placeholder summaries for `status` and `adapters list`;
- `haze-sync doctor [--offline]` parser;
- offline doctor report using `haze-sync-core::doctor` passive models;
- safe parse errors and tests for no obvious sensitive marker leaks.

Current behavior intentionally does not:

- perform live server calls;
- read config or credentials;
- connect to databases;
- call providers;
- mutate state;
- run repair/destructive commands.

The component docs were scaffold-level before this planning pass.

## Target state

The target state for CLI is a safe operator command surface for Haze Sync.

The component is V1-ready when:

- command parsing is explicit, tested, and stable;
- read-only status/adapters/doctor commands can call Server/API safely where scoped;
- offline and live doctor modes are clearly distinguished;
- CLI output is safe for operator reports by default;
- secrets are never printed and are not encouraged on shell command lines;
- mutating/admin/destructive commands are absent unless explicitly scoped with confirmation and dry-run behavior;
- exit codes are predictable;
- JSON/machine-readable output, if added, is stable and secret-safe;
- CLI does not bypass Core/API/Server/Storage safety boundaries.

## Implementation phases

### CLI-P1 — Component contract and planning normalization

Status: completed by this Architect planning pass.

Goal:

```text
Replace scaffold CLI docs with a real contract, dependency map, implementation
plan, decisions, and baseline implementation log.
```

Allowed scope:

```text
crates/haze-sync-cli/docs/**
```

Completed deliverables:

- complete `component-contract.md`;
- complete `dependency-map.md`;
- complete `implementation-plan.md`;
- add initial component decisions;
- update implementation log with planning baseline.

Non-goals:

- no product code changes;
- no parser changes;
- no live server calls;
- no new dependencies;
- no CI/workflow changes.

Acceptance:

- docs describe current CLI behavior honestly;
- docs preserve CLI/Server/API/Core/Storage/adapter boundaries;
- future CLI phases are implementable without making CLI a policy/runtime component.

### CLI-P2 — Parser, command model, and output contract hardening

Goal:

```text
Harden the current parser and output model before adding live commands or richer
formats.
```

Allowed scope:

```text
crates/haze-sync-cli/src/**
crates/haze-sync-cli/docs/**
```

Likely work:

- audit top-level parser and doctor parser for consistent command model style;
- add explicit exit-code mapping helpers;
- add tests for stdout/stderr expectations where practical;
- ensure parse errors do not echo sensitive-looking arguments in unsafe ways;
- decide whether to add a dependency such as `clap` or keep parser manual;
- document command categories: read-only, network-backed, mutation, destructive.

Non-goals:

- no live server calls;
- no config loading;
- no provider calls;
- no repair/mutation behavior.

Contract-change triggers:

- adding command syntax that implies mutation;
- exposing raw argument values that may contain secrets;
- changing command names already documented for operators;
- adding a CLI parser dependency without review.

Acceptance:

- parser behavior is stable and tested;
- help text is accurate;
- errors and exit codes are predictable.

### CLI-P3 — Config and secret-source foundation

Goal:

```text
Introduce safe CLI configuration and secret-source handling for future network
commands without leaking tokens or encouraging unsafe shell-history usage.
```

Allowed scope:

```text
crates/haze-sync-cli/src/**
crates/haze-sync-cli/docs/**
```

Likely work:

- define config source precedence for server URL, profile, output format, and token source;
- support safe token source options such as env var, config file path, stdin, or OS secret integration if accepted;
- redact config and token values in debug/errors;
- avoid printing local absolute secret paths unless local-only diagnostics explicitly allow them;
- add tests for redaction and invalid config errors.

Non-goals:

- no token creation/rotation;
- no server calls unless scoped;
- no deployment secret provisioning;
- no provider tokens.

Contract-change triggers:

- accepting tokens via positional args by default;
- printing secrets or DB URLs;
- changing deployment secret layout;
- adding OS keychain dependencies without review.

Acceptance:

- future network commands have safe config/token foundation;
- config errors are actionable and redacted;
- CLI docs discourage unsafe token handling.

### CLI-P4 — Server status and adapters read-only commands

Goal:

```text
Implement read-only live `status` and `adapters list` commands using public Server/API
contracts.
```

Allowed scope:

```text
crates/haze-sync-cli/src/**
crates/haze-sync-cli/docs/**
```

Likely work:

- implement HTTP client abstraction for Server status/adapters endpoints;
- map API public errors to CLI-safe messages;
- render human-readable summaries;
- optionally support `--json` if output contract is accepted;
- distinguish not configured, offline, unauthorized, forbidden, server unavailable, and not ready;
- keep placeholder mode honest when no server config exists.

Non-goals:

- no admin mutations;
- no direct DB reads;
- no provider calls;
- no route changes;
- no token rotation.

Contract-change triggers:

- requiring Server/API route changes;
- exposing raw cursors/token hashes/provider payloads;
- reaching into Storage directly;
- treating status as proof of E2E sync correctness.

Acceptance:

- read-only commands use public API only;
- output is sanitized and operator-useful;
- network failures are classified safely.

### CLI-P5 — Live doctor command integration

Goal:

```text
Add live doctor behavior only through accepted Server/Core/API diagnostic surfaces,
while preserving offline mode and honest skipped checks.
```

Allowed scope:

```text
crates/haze-sync-cli/src/**
crates/haze-sync-cli/docs/**
```

Likely work:

- add explicit `doctor --live` or equivalent only if accepted by contract;
- call Server doctor/readiness/status surfaces rather than direct DB/provider checks by default;
- render Core doctor summary/status consistently;
- preserve `doctor --offline` as no-network mode;
- show skipped/not-run checks honestly;
- avoid raw diagnostics and local paths in output.

Non-goals:

- no repair behavior;
- no direct DB/provider access by default;
- no destructive checks;
- no provider OAuth validation unless Server/GDrive exposes safe summary.

Contract-change triggers:

- claiming live checks without performing them;
- requiring direct SQLx/provider dependencies;
- exposing local absolute paths or provider payloads;
- adding repair flags to doctor without a repair contract.

Acceptance:

- doctor output distinguishes offline/live/skipped states;
- live checks use public diagnostic contracts;
- output remains safe for reports.

### CLI-P6 — Bootstrap and sync operation commands

Goal:

```text
Add high-level operator workflows for bootstrap/import/export only after Server,
Core, Storage, Worktree, and GDrive contracts define safe behavior.
```

Allowed scope:

```text
crates/haze-sync-cli/src/**
crates/haze-sync-cli/docs/**
```

Likely future commands:

```text
haze-sync bootstrap status
haze-sync bootstrap gdrive-import --dry-run
haze-sync bootstrap worktree-export --dry-run
haze-sync sync once --adapter <id> --dry-run
```

Likely work:

- call Server/API operator endpoints or run explicit local orchestration only if accepted;
- support dry-run first;
- require confirmations for non-dry-run operations;
- render clear phase progress and rollback/stop guidance;
- refuse to run when readiness/safety checks fail.

Non-goals:

- no hidden sync daemon;
- no provider calls directly unless GDrive/Server contract accepts CLI-mediated operation;
- no Core policy decisions;
- no hard delete.

Contract-change triggers:

- adding bootstrap endpoints not present in Server/API;
- bypassing adapter/Core safety modes;
- mutating without confirmation;
- running destructive cleanup.

Acceptance:

- bootstrap/sync commands are explicit and dry-run-capable;
- operations are contract-backed;
- unsafe states block execution.

### CLI-P7 — Admin mutation commands with confirmation and audit

Goal:

```text
Implement admin mutation commands only after API/Core/Server/Storage contracts
exist for those actions.
```

Possible future commands:

```text
haze-sync adapters pause <adapter-id>
haze-sync adapters resume <adapter-id>
haze-sync adapters set-mode <adapter-id> <mode>
haze-sync delete-unlock create --scope <scope>
haze-sync tokens rotate <adapter-id>
haze-sync repair plan
haze-sync repair apply --confirm <id>
```

Allowed scope:

```text
crates/haze-sync-cli/src/**
crates/haze-sync-cli/docs/**
```

Required safeguards:

- explicit API/Server contract exists;
- confirmation prompt or explicit `--yes`/challenge flag for risky operations;
- dry-run/plan mode where practical;
- audit-safe request metadata;
- no secrets in output;
- safe failure and rollback guidance.

Non-goals:

- no local mutation without server confirmation;
- no direct DB writes by default;
- no token printing;
- no hard delete/repair without explicit policy.

Contract-change triggers:

- implementing mutations before API/Server supports them;
- bypassing audit/authorization;
- adding destructive behavior without confirmations;
- printing tokens or unlock secrets.

Acceptance:

- admin commands are safe, explicit, and auditable;
- dangerous operations cannot be triggered accidentally;
- CLI remains a client/operator surface.

### CLI-P8 — Output formats, scripting, and packaging readiness

Goal:

```text
Stabilize CLI output for both humans and scripts and document install/run behavior.
```

Allowed scope:

```text
crates/haze-sync-cli/**
```

Likely work:

- define stable JSON output for read-only commands if accepted;
- keep human output concise and secret-safe;
- document exit-code matrix;
- add snapshot/fixture tests for output;
- document local installation and configuration;
- avoid committing generated artifacts unless policy accepts them.

Non-goals:

- no package publishing unless scoped;
- no deployment service management;
- no shell completions unless accepted;
- no hidden telemetry.

Contract-change triggers:

- changing stable JSON fields after scripts depend on them;
- leaking secrets through JSON output;
- adding packaging artifacts outside policy;
- introducing telemetry/reporting.

Acceptance:

- CLI is usable by humans and scripts;
- outputs are tested and stable;
- install/run docs are honest.

## Dependency gates

CLI implementation depends on stable upstream contracts:

- Core owns doctor models and sync policy.
- API owns DTO/header/error vocabulary.
- Server owns runtime HTTP behavior and operator endpoints.
- Storage owns persistence and must not be accessed directly unless an explicit local-admin contract accepts it.
- GDrive/Worktree/Obsidian own adapter behavior.
- Deployment owns service installation and secret placement.

CLI mutation/admin phases must run only after corresponding Server/API/Core/Storage contracts exist.

## Known risks

- CLI commands can accidentally become a backdoor around Server/Core safety boundaries.
- Secrets can leak through arguments, shell history, debug output, or copied reports.
- Offline doctor summaries can be mistaken for live environment checks.
- Direct DB/provider dependencies would make CLI an unreviewed runtime owner.
- Admin/destructive commands are high-risk without confirmations and audit semantics.
- Stable JSON output can become a compatibility contract and must be versioned carefully.

## Deferred work

Deferred outside this Architect documentation/planning pass:

- run shell checks or observe CI;
- execute CLI-P2 through CLI-P8 implementation/clean-code/CI phases;
- add live Server API client;
- add config/secret-source implementation;
- add live doctor/status/adapters commands;
- add bootstrap/admin/repair commands;
- define JSON output compatibility;
- decide packaging/completion policy.

## Completion criteria for the component

`haze-sync-cli` is V1-ready when:

- parser/help/exit-code behavior is stable;
- read-only status/adapters/doctor commands use public Server/API/Core contracts;
- offline/live diagnostics are clearly distinguished;
- secrets and raw internals are never printed;
- config/token handling is safe;
- any admin/destructive commands are explicitly scoped, confirmed, auditable, and dry-run-capable where practical;
- output formats are tested and documented;
- CLI does not own Core policy, Server runtime, Storage persistence, provider behavior, Worktree logic, or Obsidian behavior.
