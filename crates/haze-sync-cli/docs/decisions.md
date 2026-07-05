# Decisions: cli

## 2026-07-05 — CLI is an operator surface, not runtime authority

Decision:

`haze-sync-cli` is an operator-facing client and renderer. It may parse commands, render summaries, and call accepted Server/API diagnostics or admin endpoints. It must not own Core policy, Server runtime, Storage persistence, or adapter/provider behavior.

Rationale:

CLI commands are convenient but dangerous if they become a backdoor around normal sync safety. Core/Server/API/Storage boundaries must remain authoritative even for operator workflows.

Alternatives:

- Let CLI write directly to the database for convenience.
- Put sync repair policy in CLI commands.
- Let CLI call provider APIs directly.

Consequences:

- Live CLI behavior should use Server/API contracts.
- Destructive/admin commands require explicit upstream contracts.
- CLI tests should focus on parsing, rendering, redaction, exit codes, and safe client behavior.

Affected contracts:

- component contract;
- dependency map;
- status/doctor/admin/bootstrap phases;
- Server/API operator surfaces.

## 2026-07-05 — Current commands are read-only placeholders or offline diagnostics

Decision:

The current `status`, `adapters list`, and `doctor --offline` behavior remains read-only and non-mutating. Placeholder commands must say live server calls are unavailable rather than implying production behavior.

Rationale:

The existing CLI scaffold parses useful command names but does not have config, auth, or HTTP client wiring. Honest placeholder output prevents operators from mistaking parser coverage for live diagnostics.

Alternatives:

- Pretend placeholder output is live status.
- Remove placeholder commands until fully wired.
- Add live calls without config/secret policy.

Consequences:

- Docs and help text must remain honest.
- Future live phases should explicitly replace placeholder summaries with Server/API calls.
- Offline doctor output must distinguish skipped/offline checks from live checks.

Affected contracts:

- commands parser;
- doctor command;
- output contract;
- implementation plan.

## 2026-07-05 — Secret-safe output is a hard CLI boundary

Decision:

CLI stdout/stderr/help/status/doctor/admin output must not expose tokens, token hashes, OAuth values, idempotency keys, database URLs, object-store roots, raw provider payloads, stack traces, or raw server internals.

Rationale:

CLI output is often copied into tickets, reports, terminals, shells, logs, and chat. Leaks from CLI are operationally high-risk.

Alternatives:

- Print raw errors for easier debugging.
- Allow `--verbose` to dump raw internals by default.
- Echo all parsed arguments in parse errors.

Consequences:

- Runtime and parse errors need safe classification.
- Tests should assert absence of sensitive markers.
- Any verbose/debug mode requires explicit local-only redaction design.

Affected contracts:

- command output;
- error rendering;
- doctor/status/admin commands;
- test obligations.

## 2026-07-05 — Live behavior should use Server/API, not direct DB/provider access

Decision:

Future live CLI commands should call public Server/API surfaces by default. Direct DB/provider access is out of scope unless a specific local-admin contract accepts it.

Rationale:

Server/API enforce auth, public DTOs, Core policy mapping, and runtime state. Direct DB/provider access from CLI can bypass these safety boundaries.

Alternatives:

- Give CLI SQLx access for status and repair.
- Let CLI call Google Drive for doctor checks.
- Link CLI directly to Server private route modules.

Consequences:

- Server/API operator endpoints may be needed before rich CLI features.
- Live doctor should prefer Server-provided diagnostics.
- Deployment must ensure CLI can authenticate to Server safely.

Affected contracts:

- live status/adapters/doctor phases;
- Server admin/status/doctor routes;
- API DTOs;
- deployment runbooks.

## 2026-07-05 — Destructive/admin commands require confirmation and upstream contracts

Decision:

Commands that mutate state, repair data, rotate tokens, unlock deletes, pause/resume adapters, or change modes require explicit upstream contracts and confirmation/dry-run behavior where practical.

Rationale:

Operator commands can cause data loss or security changes. They must be auditable, intentional, and aligned with Core/API/Server/Storage semantics.

Alternatives:

- Add mutation commands opportunistically.
- Hide destructive behavior behind status/doctor flags.
- Let CLI mutate local DB/provider state without confirmation.

Consequences:

- Admin/destructive phases are deferred until upstream support exists.
- CLI must make operation scope visible before execution.
- Tests should cover confirmation refusal paths.

Affected contracts:

- admin mutation plan;
- Core delete/repair/token semantics;
- Server/API admin endpoints;
- operational runbooks.

## 2026-07-05 — JSON output is a compatibility surface once introduced

Decision:

If CLI adds `--json` or other machine-readable output, those fields become a compatibility surface and require fixture/snapshot tests and explicit versioning or stability notes.

Rationale:

Scripts and runbooks will depend on JSON output. Accidental field changes can break operations even when Rust tests pass.

Alternatives:

- Treat JSON as best-effort/debug-only.
- Only support human output.
- Expose raw API DTOs directly without CLI contract.

Consequences:

- JSON output is deferred until fields are known.
- CLI should map API/server data into stable operator-oriented shapes.
- Secret redaction applies equally to JSON and human output.

Affected contracts:

- output format phases;
- E2E/runbook scripts;
- API DTO mapping;
- CI/snapshot tests.
