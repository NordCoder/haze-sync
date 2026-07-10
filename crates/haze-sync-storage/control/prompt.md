# W1-FIX-STOR-P9-CI — Storage STOR-P9 validation correction

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: fixer-worker

Use only the GitHub connector. Do not merge the PR or change its lifecycle state.

## Evidence

The final STOR-P9 source/docs head has a failed Component CI result.

- code_bearing_sha: 46b13f986f2a338e86a8811639d706a64a378565
- workflow_run_id: 29116418343
- run_number: 1502
- run_attempt: 1
- artifact_id: 8236830104
- artifact_name: ci-diag__component-storage__wf-component-ci__run-29116418343__attempt-1
- artifact_expires_at: 2026-07-11T19:00:35Z

Use the diagnostics artifact as the authoritative failure evidence.

## Required work

Read the project process files, current Storage control files, STOR-P9 source/tests/docs, PR diff, and every required file in artifact 8236830104. If the artifact is unavailable or incomplete, report FIX_BLOCKED_BY_LOGS.

Apply only the smallest correction demonstrated by the artifact. Preserve explicit test-only database configuration, redacted configuration errors, production isolation, schema setup locking, partial-schema rejection, bounded fixture identifiers, observable test object cleanup, pure harness tests, documentation, and Storage ownership boundaries.

## Allowed files

- crates/haze-sync-storage/src/test_support/**
- crates/haze-sync-storage/docs/** only when directly required by diagnostics
- crates/haze-sync-storage/src/lib.rs or Cargo.toml only when directly required by diagnostics
- crates/haze-sync-storage/control/report.md

Do not change production repository semantics, migrations/schema, Server fan-in, provider behavior, shared workflows/dependencies, sibling components, or unrelated test expectations.

Source/test/docs correction commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-sync-storage/control/report.md using REPORT_TYPE FIX and phase_id FIX-STOR-P9-CI.
