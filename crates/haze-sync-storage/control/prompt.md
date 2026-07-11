# W1-CLEAN-STOR-P10 — Durable Worktree state clean-code review

Before starting, name this worker chat exactly:

`storage — W1 STOR-P10 Clean-Code Review`

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: clean-code-reviewer
Phase: STOR-P10-CLEAN

Work through the GitHub connector. Do not merge PR #47 into main, change its draft state, rebase, reset, rewrite history, force-push, or modify sibling branches.

## Accepted review baseline

STOR-P10 implementation, its artifact-based correction, main synchronization, and strict PostgreSQL verification are complete.

Evidence:

- accepted pre-STOR-P10 Storage source/docs SHA: `aa59064d641f4850f7c70fa615e638b52613dd95`;
- initial STOR-P10 implementation SHA: `63d80764933cba5f23fb43bad44201a75e1dc16a`;
- post-fix Storage product/schema/test/docs SHA: `abca69058390983894465cef9d66c38960fae4c7`;
- helper synchronization PR: `#64`;
- helper merge commit into component/storage: `c62112375793002392851403299b09988e27212f`;
- final reviewed code/tooling candidate SHA: `2d2ffe03a0331f5e1f3b5cf3508fb9459c84daf2`;
- PR #47 is open, draft, unmerged and mergeable;
- authoritative Component CI run: `29171728288`;
- run number: `1818`;
- conclusion: `success`.

The run contains two successful jobs:

1. `Rust workspace`: fmt, check, workspace tests, clippy and diagnostics finalizer all passed.
2. `Storage PostgreSQL verification`: PostgreSQL readiness, exact strict ignored-test command, four-test evidence validation and diagnostics finalizer all passed.

The strict command was:

`cargo test -p haze-sync-storage --features test-support -- --ignored`

The evidence job verified successful execution of:

- `repositories::adapter_cursors::postgres_tests::exact_cursor_progression_is_locked_contiguous_and_rollback_safe`;
- `repositories::worktree_state::postgres_tests::durable_instances_and_path_state_are_isolated_and_transactional`;
- `test_support::postgres::tests::fresh_and_current_schema_preparation_is_idempotent`;
- `test_support::postgres::tests::migrates_empty_pre_p10_schema_and_rejects_nonempty_legacy_state`.

Do not read diagnostics artifacts: the authoritative final CI is green.

## Review scope

Review the complete STOR-P10 change from the accepted pre-phase baseline through the final synchronized candidate, especially:

- `migrations/0010_worktree_durable_state.sql`;
- Worktree durable-state models and repository exports;
- `src/repositories/worktree_state.rs` and its unit/PostgreSQL tests;
- `src/repositories/adapter_cursors.rs` and its unit/PostgreSQL tests;
- schema and test-support changes required by migration 0010;
- Storage documentation for durable Worktree state, implementation plan/log and test support;
- the Storage-only PostgreSQL verification job in `.github/workflows/component-ci.yml`;
- main-synchronization correctness and absence of temporary write-enabled workflows.

Review correctness, clean code, SQL safety, transaction ownership, migration determinism, error secrecy, test strength, workflow correctness, compatibility and downstream usability. Green CI is necessary but not sufficient.

## Mandatory review questions

1. Does migration 0010 fail before destructive behavior for incompatible non-empty legacy state?
2. Are adapter identity and root fingerprint versioned, fail-closed and free of raw-root persistence or rendering?
3. Is path state isolated per adapter instance with explicit present/tombstoned invariants?
4. Are bounded snapshots deterministic and correctly ordered?
5. Do repository methods remain passive and caller-transaction-owned?
6. Is cursor initialization and exact expected N to N+1 advancement lock-safe, monotonic and rollback-correct?
7. Are regression, gap, stale expected value, overflow and concurrency races rejected?
8. Can any failed transaction falsely advance cursor or durable path-state claims?
9. Are migration and repository errors redacted and safe in Display/Debug output?
10. Do unit and live PostgreSQL tests prove behavior rather than merely execute happy paths?
11. Does test support prevent accidental use of a non-test or unsafe database?
12. Does the Storage PostgreSQL CI job fail on missing DB, skipped tests, absent test evidence, timeout or diagnostics failure?
13. Are synthetic credentials confined to the ephemeral CI service and free of repository secrets?
14. Does the final branch contain current main CI infrastructure plus the accepted Storage-only DB job, with no temporary `contents: write` workflow?
15. Are Storage, Worktree, Server, Core and API ownership boundaries preserved?
16. Did STOR-P10 introduce avoidable duplication, broad visibility, weak types, ambiguous names, dead code, unbounded queries or weak assertions?

## Corrections

You may make focused Storage-local clean-code or correctness corrections when necessary. Any source, migration, test, documentation or workflow correction must preserve the accepted architecture and receive a new normal Component CI run with both required jobs.

Allowed files:

- STOR-P10 migration, Storage models/repositories/schema/test-support and tests;
- STOR-P10 Storage-owned docs;
- `.github/workflows/component-ci.yml` only for a demonstrable Storage PostgreSQL verification defect;
- `crates/haze-sync-storage/control/report.md`.

Forbidden:

- Server, Worktree, Core, API, Common, provider, CLI or Deployment source changes;
- migration redesign without a proven defect;
- raw-root persistence or logging;
- production credentials, external managed DB dependency or repository secrets;
- test deletion, ignoring, optional DB verification or assertion weakening;
- hard delete, implicit destructive cleanup or automatic repair;
- unrelated workflow refactoring;
- archiving control files.

## CI policy

If product, migration, test, docs or workflow content changes, require a new code-bearing Component CI run in which both `Rust workspace` and `Storage PostgreSQL verification` finish green. A report-only commit may skip CI but is not code-bearing evidence.

## Report

Write `crates/haze-sync-storage/control/report.md` using `report-template.md`.

Set:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`
- `phase_id: STOR-P10-CLEAN`
- `chat_name: storage — W1 STOR-P10 Clean-Code Review`

Use one honest status:

- `CLEAN_ACCEPT`
- `CLEAN_NEEDS_FIX`
- `CLEAN_BLOCKED_BY_CONTRACT`
- `CLEAN_BLOCKED_BY_SCOPE`
- `CLEAN_BLOCKED_BY_TOOLING`

The report must state the exact reviewed code-bearing/tooling SHA, complete range/files, migration and repository findings, transaction/cursor assessment, live-DB test assessment, workflow/secrecy assessment, corrections, final CI evidence if corrections were made, and whether STOR-P10 is clean-accepted for SRV-P7B3 accepted-SHA synchronization.