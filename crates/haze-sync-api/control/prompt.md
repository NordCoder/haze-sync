# W1-FIX-API-GDA-P1-DEBUG-REDACTION

Before starting, name this worker chat exactly:

`api — W1 FIX-API-GDA-P1 Debug Redaction`

Repository: `NordCoder/haze-sync`
Component: api
Path: `crates/haze-sync-api`
Branch/ref: `component/api`
PR: #44
Role: fixer-worker
Phase: `FIX-API-GDA-P1-DEBUG-REDACTION`

This is a focused review-fixer for the existing API-GDA-P1 phase. Do not begin another API product phase.

Review target:
- code-bearing SHA: `77945118a37c6e8efec04ecd054e0e5a5e4435ba`;
- clean-review report blob: `bcd51da350f8d7bdb7dec19a7a9f6757addff3e3`;
- green CI run: `29440533856`, number `2017`.

Required fix:
1. Remove unsafe derived Debug exposure from private GDrive mapping, snapshot and commit DTOs.
2. Ensure authenticated commit request Debug does not recursively format the private body.
3. Use explicit redacted markers or safe summaries only.
4. Add sentinel tests proving path, drive name, MIME, checksum, Core IDs, timestamps, provider facts, raw cursor and idempotency values never appear in Debug output.
5. Preserve all serde wire shapes, compatibility fixture JSON, authorization metadata, CAS/cursor/checkpoint semantics, safe errors and API passivity.

Allowed scope:
- `crates/haze-sync-api/src/dto/gdrive.rs`;
- `crates/haze-sync-api/src/routes/gdrive.rs`;
- focused API tests/docs only if needed;
- control report.

Forbidden:
- Server/runtime registration;
- Storage/Core/provider/OAuth/scheduler/status-control work;
- sibling or workflow changes;
- fixture vocabulary changes unrelated to secrecy;
- test weakening, merge, rebase, force-push or draft-state changes.

Create code/test changes without CI skip. Obtain a new full exact-SHA Component CI run with fmt/check/test/clippy and diagnostics finalization green.

Write only `crates/haze-sync-api/control/report.md` with:
- `REPORT_TYPE: FIX`;
- `phase_id: FIX-API-GDA-P1-DEBUG-REDACTION`;
- `chat_name: api — W1 FIX-API-GDA-P1 Debug Redaction`;
- status `FIX_COMPLETE`, `FIX_NEEDS_MORE`, `FIX_BLOCKED_BY_CONTRACT`, or `FIX_BLOCKED_BY_TOOLING`.

Do not claim CLEAN_ACCEPT. A repeat clean review follows.
