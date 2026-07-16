# W1-SRV-GDA-P1-CONTINUE-TESTS-AND-VERIFICATION

Before starting, name this worker chat exactly:

`server — W1 SRV-GDA-P1 Tests and Verification`

Repository: `NordCoder/haze-sync`
Component: server
Path: `crates/haze-sync-server`
Branch/ref: `component/server`
PR: #45
Role: implementation-worker
Phase: `SRV-GDA-P1-CONTINUE-TESTS-AND-VERIFICATION`

This is a continuation of the existing `SRV-GDA-P1-STATE-ROUTES-AND-TRANSACTIONS` product phase. Do not repeat the initial fan-in or start another Server phase.

## Current incomplete candidate

- code-bearing SHA: `8bf6d2fa881bfca3c53553ea5b1b63aed01be5f0`;
- implementation report blob: `ad664f9a0dce10c6e49fffedec9a40b74366816c`;
- Component CI run `29484442496`, run number `2030`;
- cargo fmt/check/test/clippy: success;
- diagnostics finalizer: failure;
- mandatory Server-owned PostgreSQL route/application tests: missing;
- final accepted dependency identity verification: incomplete;
- Server implementation-log alignment: incomplete.

The previous candidate is not review-ready. Continue implementation from the actual current branch head.

## Accepted owner inputs

- API GDrive SHA: `c60c3976696da1970d539e5cff6e9f74a61fc10e`;
- API clean-review blob: `55ff6047c9c6c0f6f548f10197b76706c0a244e1`;
- Storage GDrive SHA: `3617bd1cf947fdd394f1ab29d4b992f7b8859a84`;
- Storage clean-review blob: `4584b8705221d3cd2aa43b5776674b3a1ec9a0f4`;
- architecture report blob: `14c427880e1201d851cdc9ee04b9cd0e83334de4`.

## Required completion work

1. Preserve the already implemented GET/POST route and transaction boundary unless tests prove a focused defect.
2. Add Server-owned real PostgreSQL route/application tests covering at minimum:
   - matching GDrive adapter private read;
   - admin sanitized read;
   - unauthorized, forbidden and adapter-type mismatch;
   - successful commit and deterministic replay;
   - stale state and concurrent-writer loser;
   - cursor gap and regression;
   - mapping conflict and idempotency conflict;
   - rollback with no partial mapping/echo/delete-candidate/operation facts;
   - adapter isolation;
   - safe error and redaction boundaries.
3. Exercise the actual Server route/application boundary, not only Storage repository tests.
4. Verify every fanned-in API and Storage dependency file that should match an accepted owner artifact against the exact accepted owner refs. Record the compared paths and blob identities in the report.
5. Do not semantically modify accepted API or Storage owner contracts. If a mismatch is required for Server correctness, report `BLOCKED_BY_CONTRACT` instead.
6. Complete the minimal Server implementation-log update required by the parent prompt.
7. Obtain a new exact-SHA DB-capable Component CI run with:
   - cargo fmt/check/test/clippy green;
   - Server/PostgreSQL verification green;
   - diagnostics finalization green.

Do not spend this slot diagnosing the old run's finalizer artifact before completing the missing product/test work. The old candidate was incomplete. If the new complete candidate still has red diagnostics, record its new artifact and report `SELF_NEEDS_FIX` for a separate artifact-first fixer slot.

## Protected scope

Allowed:
- focused Server route/application/auth/state modules;
- Server-owned tests and PostgreSQL test support;
- exact accepted dependency fan-in verification/correction only where files must equal owner blobs;
- Server docs/implementation log;
- control report.

Forbidden:
- API or Storage contract/schema redesign;
- Google/OAuth/provider work;
- Core policy, scheduler, status-control, CLI or Deployment work;
- sibling branch or workflow changes;
- raw cursor, provider facts, idempotency values, request bodies or SQL/database errors in output;
- test weakening;
- merge, rebase, force-push or PR draft-state changes.

## Report

Write only `crates/haze-sync-server/control/report.md` with:
- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-GDA-P1-CONTINUE-TESTS-AND-VERIFICATION`;
- `chat_name: server — W1 SRV-GDA-P1 Tests and Verification`;
- status `SELF_ACCEPT`, `SELF_ACCEPT_PENDING_CI`, `SELF_NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_DEPENDENCY`, or `BLOCKED_BY_TOOLING`.

Record exact tests, transaction/outcome evidence, dependency blob verification, final code-bearing SHA and exact DB-capable CI. Do not claim CLEAN_ACCEPT.
