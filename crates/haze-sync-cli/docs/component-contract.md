# Component Contract: cli

## Responsibility

`haze-sync-cli` owns the operator-facing command-line interface for Haze Sync.

The CLI is an operational surface. It may parse commands, render status/doctor/admin summaries, call Server HTTP APIs, and help operators run safe diagnostics. It is not the Core policy owner, Server runtime, Storage repository layer, provider adapter, or Obsidian/Worktree runtime.

Current implemented behavior is intentionally limited:

- parse `haze-sync status` as a read-only placeholder;
- parse `haze-sync adapters list` as a read-only placeholder;
- parse `haze-sync doctor [--offline]`;
- render a safe offline doctor summary from Core doctor models;
- reject unsupported live/repair/provider/destructive arguments safely.

Future CLI behavior may include:

- server status and adapter list commands backed by HTTP API;
- doctor commands that call safe Server/Core/API/Storage diagnostic surfaces;
- bootstrap/import/export operator workflows when explicitly scoped;
- safe token/admin/config helpers when explicitly scoped;
- human-readable and machine-readable output formats;
- explicit confirmations for destructive/admin actions.

## Public interfaces

Current public/runtime interface:

```text
Binary: haze-sync
Commands:
  haze-sync --help
  haze-sync help
  haze-sync status
  haze-sync adapters list
  haze-sync doctor [--offline]
```

Current Rust module surface:

```text
src/main.rs
src/commands.rs
src/doctor.rs
```

Current constraints:

- command parsing is dependency-free and side-effect-free;
- `status` and `adapters list` are placeholders and perform no live server calls;
- `doctor` is read-only offline summary only;
- live checks, repair, provider calls, and destructive actions are intentionally unavailable.

Target future interface should keep command behavior explicit and safe. New commands must document whether they are:

```text
read-only
network-backed
local-diagnostic
admin mutation
destructive/repair
experimental/dry-run
```

## Input contracts

CLI inputs come from:

- process arguments;
- environment/config files when future phases add them;
- optional server URL/token inputs when future phases add network commands;
- Server/API responses;
- operator confirmation prompts for future destructive/admin operations.

Required input rules:

- command arguments must be parsed into explicit command models before execution;
- unknown flags/arguments must fail safely with non-zero exit code;
- secrets must not be accepted in ways that cause shell history/log exposure unless the command explicitly documents the risk and safer alternatives;
- server URL and token config must be validated before network calls;
- live commands must not be silently attempted by placeholder/offline commands;
- destructive/admin commands must require explicit scope and confirmation.

## Output contracts

CLI outputs include stdout summaries, stderr parse/runtime errors, and process exit codes.

Required output rules:

- read-only successful output goes to stdout;
- parse/runtime errors go to stderr;
- unsupported or invalid arguments exit with code `2` under current behavior;
- successful placeholder/offline commands exit successfully only when no mutation was attempted;
- output must be safe for copy/paste into reports unless a command is explicitly marked local-only and still redacts secrets;
- machine-readable output formats, if added, must be stable and secret-safe.

CLI output must not expose:

- bearer tokens;
- OAuth tokens;
- token hashes;
- Idempotency-Key values;
- database URLs;
- object-store roots;
- local absolute paths unless explicitly accepted for local-only diagnostics;
- raw provider payloads;
- stack traces;
- raw SQLx/provider/server internals;
- raw file content.

## Error contracts

CLI errors must be safe and actionable.

Current safe parse errors:

- unknown command;
- missing adapters command;
- unknown adapters command;
- unexpected argument;
- unknown doctor flag;
- unexpected doctor argument.

Future runtime errors should be classified without leaking internals:

```text
config_error
auth_error
network_error
server_unavailable
forbidden
not_ready
unsupported_command
unsafe_operation_blocked
doctor_warning
doctor_failure
internal_error
```

Mapping from API/Server/Core/Storage errors to CLI text belongs in CLI, but the underlying policy and public error vocabulary remain owned by upstream components.

## Persistence/runtime ownership

CLI may own local operator UX state only when scoped.

CLI may own:

- command parsing;
- output formatting;
- confirmation prompts;
- optional config file parsing for CLI-only settings;
- HTTP client calls to Server API;
- rendering Server/API/Core doctor/status data;
- exit-code mapping.

CLI does not own:

- Core conflict/delete/revision/idempotency policy;
- API DTO/header/public error vocabulary;
- Server startup/listener/runtime state;
- Storage schema/repositories/migrations;
- provider adapter loops;
- Worktree scanner/materializer behavior;
- Obsidian plugin behavior;
- direct DB writes unless a future local-admin contract explicitly accepts them;
- hard delete/repair behavior without explicit confirmation and upstream contracts.

## Security and secrecy rules

- Do not commit secrets, local config files with tokens, DB URLs, OAuth tokens, token hashes, logs, dumps, or generated reports containing sensitive data.
- Do not print tokens, token hashes, idempotency keys, DB URLs, object-store roots, raw provider payloads, or stack traces.
- Avoid accepting secrets directly on command line where shell history can capture them; prefer config files, env vars, stdin, or OS secret mechanisms if future phases add token commands.
- Destructive/admin commands require explicit contract, confirmation, and dry-run support where practical.
- Doctor/status output must distinguish skipped/offline checks from live checks.
- CLI must not claim live checks passed when it only built offline summaries.

## Non-goals

CLI must not implement:

- Core sync policy decisions;
- Server route handlers or listener startup;
- Storage schema/repository ownership;
- provider/GDrive API runtime behavior;
- Worktree scanner/materializer logic;
- Obsidian plugin behavior;
- automatic repair/destructive cleanup by default;
- hidden background sync loops;
- production deployment service management unless deployment component explicitly scopes it;
- token creation/rotation unless a future security/admin contract accepts it.

## Dependencies

See `dependency-map.md`.

## Dependents

See `dependency-map.md`.

## Invariants

- CLI is an operator surface, not the source of truth.
- Placeholder commands must be honest and non-mutating.
- Offline doctor output must be labeled offline/skipped where live checks were not performed.
- Live server calls must use public Server/API contracts.
- CLI must not bypass Core/API/Server/Storage safety boundaries.
- Unsafe/destructive operations must be explicit, confirmed, and contract-backed.
- Public CLI output must be secret-safe.
- Exit codes must distinguish success, usage/parse failure, and runtime failure when future runtime behavior exists.

## Test obligations

CLI tests should cover:

- command parsing for help/status/adapters/doctor;
- unknown/unsupported arguments and exit-code behavior;
- placeholder commands remaining non-mutating;
- offline doctor summary using Core doctor models;
- output redaction for token/OAuth/DB/object-store/path/provider markers;
- future HTTP client request construction and API error mapping;
- future JSON output stability if machine-readable output is added;
- confirmation behavior for future admin/destructive commands;
- honest skipped/offline/live doctor status rendering.

Expected checks when shell or CI is available:

```bash
cargo fmt --check
cargo check -p haze-sync-cli
cargo test -p haze-sync-cli
```

## Contract change protocol

Request a contract change instead of silently broadening scope when implementation requires:

- direct database/storage writes from CLI;
- implementing Core policy locally;
- changing API/Server DTO/header/error contracts;
- adding provider/GDrive calls;
- adding Worktree/Obsidian runtime behavior;
- adding destructive/repair commands without explicit safety/confirmation contract;
- printing secrets or local sensitive paths;
- claiming live checks without real live checks.
