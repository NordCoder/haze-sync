# W1-STOR-P10-FINAL-CLEAN — Final durable Worktree state acceptance review

Before starting, name this worker chat exactly:

`storage — W1 STOR-P10 Final Clean Acceptance`

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: clean-code-reviewer
Phase: STOR-P10-FINAL-CLEAN

Work through the GitHub connector. Do not merge PR #47 into main, change its draft state, rebase, reset, rewrite history, force-push, or modify sibling branches.

## Purpose

This is the final clean-acceptance pass after the previous review strengthened migration evidence and the follow-up fixer corrected only rustfmt layout.

The task is not to repeat broad implementation work. Confirm that the complete accepted STOR-P10 range, the added direct migration-guard evidence, and the final formatter-only correction are clean and ready for downstream accepted-SHA synchronization.

## Authoritative baseline

- accepted pre-phase Storage SHA: `aa59064d641f4850f7c70fa615e638b52613dd95`;
- initial STOR-P10 implementation SHA: `63d80764933cba5f23fb43bad44201a75e1dc16a`;
- post-implementation fixer SHA: `abca69058390983894465cef9d66c38960fae4c7`;
- synchronized initial clean-review candidate: `2d2ffe03a0331f5e1f3b5cf3508fb9459c84daf2`;
- clean-review correction SHA: `a6f1edf48d23c767f0f9b34ab28aacd8bd586000`;
- final formatter correction code-bearing SHA: `66b6a1f554aae1d1b774cc88560d46dd140c7a54`;
- authoritative Component CI run: `29185466870`;
- run number: `1833`;
- attempt: `1`;
- conclusion: `success`.

Both required jobs are green:

1. `Rust workspace`: fmt, check, workspace tests, clippy and diagnostics finalizer succeeded.
2. `Storage PostgreSQL verification`: readiness, strict ignored-test command, five mandatory evidence checks and diagnostics finalizer succeeded.

Exact strict command:

`cargo test -p haze-sync-storage --features test-support -- --ignored`

Do not read failure artifacts; the final authoritative CI is green.

## Required final review

Confirm the final branch preserves and cleanly implements:

- migration 0010 fail-before-destructive behavior for incompatible non-empty legacy state;
- the direct savepoint-backed SQL guard test and its exact assertions;
- all five mandatory PostgreSQL evidence checks;
- versioned adapter/root-fingerprint binding without raw-root persistence or rendering;
- per-instance present/tombstoned path-state invariants;
- bounded deterministic snapshots;
- passive caller-owned transaction behavior;
- exact-contiguous cursor locking, progression, rollback, stale/gap/overflow rejection and concurrency semantics;
- redacted errors, test-database URL safety and synthetic ephemeral CI credentials;
- current-main CI synchronization, `contents: read`, and absence of temporary write-enabled workflows;
- Storage/Server/Worktree/Core/API ownership boundaries;
- no test weakening, hidden skips, hard delete, implicit repair or destructive cleanup.

Also verify that the final fixer changed only rustfmt layout in `crates/haze-sync-storage/tests/stor_p10_migration_guard.rs` and did not alter assertions or behavior.

## Corrections

Do not make speculative improvements. A correction is allowed only for a concrete material defect discovered in this final pass and must remain Storage-local and architecture-preserving.

Any source, migration, test, documentation or workflow correction requires a new Component CI run where both `Rust workspace` and `Storage PostgreSQL verification` are fully green. A report-only commit is not code-bearing evidence.

Allowed files:

- STOR-P10 Storage migration/models/repositories/schema/test-support/tests and Storage-owned docs;
- `.github/workflows/component-ci.yml` only for a proven Storage PostgreSQL verification defect;
- `crates/haze-sync-storage/control/report.md`.

Forbidden:

- Server, Worktree, Core, API, Common, provider, CLI or Deployment changes;
- raw-root persistence/logging;
- production credentials or external managed database use;
- test deletion, ignore, optional DB verification or assertion weakening;
- broad refactoring unrelated to a material finding;
- archiving control files.

## Report

Write `crates/haze-sync-storage/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`
- `phase_id: STOR-P10-FINAL-CLEAN`
- `chat_name: storage — W1 STOR-P10 Final Clean Acceptance`

Use one honest status:

- `CLEAN_ACCEPT`
- `CLEAN_NEEDS_FIX`
- `CLEAN_BLOCKED_BY_CONTRACT`
- `CLEAN_BLOCKED_BY_SCOPE`
- `CLEAN_BLOCKED_BY_TOOLING`

The report must state the exact final reviewed code-bearing SHA, complete review range, confirmation of the five-test migration evidence, repository/cursor/transaction/secrecy assessment, formatter-only fix verification, final CI evidence, any corrections, and whether STOR-P10 is accepted for SRV-P7B3 SHA synchronization.