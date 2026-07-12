# W1-SRV-P7B2-CLEAN-RETRY — Complete Server clean-code review and write report

Before starting, name this worker chat exactly:

`server — W1 SRV-P7B2 Clean-Code Review Retry`

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer
Phase: SRV-P7B2-CLEAN-RETRY

## Why this slot was reissued

The prior `SRV-P7B2-CLEAN` slot produced no `control/report.md` and no branch advance. The Orchestrator therefore has no review verdict and cannot advance the lifecycle.

This retry must complete the review and must write the required report even when no code correction is needed.

Work through the GitHub connector. Do not merge PR #45 into main, change its draft state, rebase, reset, rewrite history, force-push, modify sibling branches, or read failure artifacts when the final authoritative CI is green.

## Authoritative candidate

- accepted SRV-P7A baseline: `37706634fd8dd2d9b299a1c453718f2de63981d0`;
- initial SRV-P7B2 implementation SHA: `81e6f6ae69f4bda92d284991e4909457a6ef8e60`;
- post-source-fix SHA: `9304263f4be8234e513bf335dc886f96c53d0cce`;
- synchronized helper merge commit: `716bd357f52831d796c7e1ea57c83a2849e6ce03`;
- final code-bearing/tooling candidate SHA: `647dce7b624d67663632808906896cb6745ea7e7`;
- authoritative Component CI run: `29186058268`;
- run number: `1835`;
- conclusion: `success`.

The final CI passed PostgreSQL Server and Storage services, fmt, check, isolated Server tests, isolated Storage tests, all remaining workspace tests, clippy and diagnostics finalizer.

## Mandatory review scope

Review and explicitly report on:

1. one reusable async `ServerApplicationServices` authority for PUT, guarded DELETE, changes and revision-content retrieval;
2. routes remaining transport/auth/DTO/status adapters;
3. atomic transaction, advisory-lock, idempotency, Core, object-store, revision, conflict, tombstone and operation-log choreography;
4. stale-base, replay, conflict and guarded-delete parity;
5. sanitized errors and secret-safe Worktree idempotency derivation;
6. test-only DB lease/schema probe/cleanup remaining strictly test-scoped;
7. persisted stale-revision fixtures using real foreign-key-valid revisions;
8. isolated Server and Storage PostgreSQL environments preserving full workspace coverage;
9. current-main synchronization and absence of temporary write-enabled workflows;
10. SRV-P7A startup/composition and dependency-free router construction;
11. exact Storage dependency gating at candidate SHA:
   - normal `[dependencies]`: `haze-sync-storage` without `test-support`;
   - `[dev-dependencies]`: `haze-sync-storage` with `features = ["test-support"]`.

Storage contract states that production crates must not enable `test-support` in normal dependencies. A `CLEAN_ACCEPT` report must explicitly confirm this point so STOR-P10 can be unblocked for final cross-branch confirmation.

## Corrections

Make no speculative changes. Focused Server-local corrections are allowed only for concrete material findings. Any executable change requires a new DB-capable Component CI run with all gates green.

Allowed:

- SRV-P7B2 Server application services, route delegation and tests;
- test-only Server DB harness;
- `crates/haze-sync-server/Cargo.toml` for a proven dependency-gating defect;
- SRV-P7B2 Server docs;
- `.github/workflows/component-ci.yml` for a proven isolation/coverage defect;
- `crates/haze-sync-server/control/report.md`.

Forbidden:

- Storage, Worktree, Core, API, Common, CLI, Deployment, provider, schema or migration changes;
- production mutex/cleanup behavior;
- test deletion, ignoring, silent skip or assertion weakening;
- Worktree executor/host/background implementation;
- broad unrelated refactoring;
- archiving control files.

## Required report

Write `crates/haze-sync-server/control/report.md` using `report-template.md` before finishing.

Set exactly:

- `REPORT_TYPE: CLEAN_CODE_REVIEW`
- `phase_id: SRV-P7B2-CLEAN-RETRY`
- `chat_name: server — W1 SRV-P7B2 Clean-Code Review Retry`

Use one honest status:

- `CLEAN_ACCEPT`
- `CLEAN_NEEDS_FIX`
- `CLEAN_BLOCKED_BY_CONTRACT`
- `CLEAN_BLOCKED_BY_SCOPE`
- `CLEAN_BLOCKED_BY_TOOLING`

The report must include exact reviewed code-bearing SHA, complete range, application-service/route/transaction assessment, test-harness and CI-isolation assessment, explicit normal-versus-dev Storage dependency evidence, secrecy assessment, corrections and final CI evidence if any, and whether SRV-P7B2 is accepted for SRV-P7B3 fan-in and Storage contract unblocking.

A response in chat without the committed `control/report.md` does not complete this slot.