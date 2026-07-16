# W1-FIX-SRV-GDA-P1-OUTCOME-MAPPING

Before starting, name this worker chat exactly:

`server — W1 FIX-SRV-GDA-P1 Outcome Mapping`

Repository: `NordCoder/haze-sync`
Component: server
Path: `crates/haze-sync-server`
Branch/ref: `component/server`
PR: #45
Role: fixer-worker
Phase: `FIX-SRV-GDA-P1-OUTCOME-MAPPING`

This is a focused review-fixer for the existing `SRV-GDA-P1-STATE-ROUTES-AND-TRANSACTIONS` phase. Do not begin another Server product phase.

Review target:
- code-bearing SHA: `b0ae522229bbc6422763a2cd075b995768346963`;
- clean-review report blob: `d4fe8c9150184f34383d708048d402df7c008078`;
- DB-capable green Component CI run: `29490745222`, number `2041`.

The review accepted authorization, route registration, transaction ownership, rollback atomicity, dependency identity, PostgreSQL route evidence, concurrency/replay/isolation and secrecy. Fix only these two blocking Server outcome mappings:

1. `RepositoryError::GDriveCursorGenerationMismatch` must not map to the `cursor_gap` commit outcome.
   - After rollback, return the accepted fixed HTTP 409 route error envelope with code `invalid_cursor_state`.
   - Add a real PostgreSQL route test using the current state version with a mismatched persisted cursor generation.
   - Prove the response category and no state mutation.

2. Internal or unexpected Storage failures must not map to caller-facing `validation_failed`.
   - `DatabaseOperationFailed`, `UnsupportedGDriveStateVersion`, `StateVersionOverflow` and other internal/unexpected invariant failures must return the accepted safe HTTP 500 `internal` error envelope after rollback.
   - Keep `validation_failed` only for genuine request/storage validation categories.
   - Update the existing rollback PostgreSQL test to assert the internal envelope while preserving all no-partial-facts and secrecy assertions.

Allowed scope:
- `crates/haze-sync-server/src/routes/gdrive.rs`;
- focused Server GDrive route/PostgreSQL tests;
- minimal Server docs only if outcome mapping documentation must be corrected;
- control report.

Preserve:
- accepted API and Storage files byte-identical to owner refs;
- existing route paths and authorization;
- caller-owned transaction choreography;
- committed/replayed commit behavior;
- stale, cursor gap/regression, mapping and idempotency outcomes;
- rollback, concurrency, isolation and redaction coverage.

Forbidden:
- API or Storage contract/schema edits;
- provider/OAuth/Core policy/scheduler/status-control/CLI/Deployment work;
- sibling or workflow changes;
- raw cursor, provider facts, Idempotency-Key, request body, SQLx/database errors or URLs in output;
- test weakening, merge, rebase, force-push or PR draft-state changes.

Create code/test changes without CI skip. Obtain full exact-SHA DB-capable Component CI with fmt/check/test/clippy, Server/PostgreSQL tests and diagnostics finalization green.

Write only `crates/haze-sync-server/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-SRV-GDA-P1-OUTCOME-MAPPING`;
- `chat_name: server — W1 FIX-SRV-GDA-P1 Outcome Mapping`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Record exact error mapping, changed tests, rollback evidence, final code-bearing SHA and exact DB-capable CI. Do not claim CLEAN_ACCEPT; a repeat clean functional review follows.
