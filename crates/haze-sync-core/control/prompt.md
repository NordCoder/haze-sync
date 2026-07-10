# W1-FIX-CORE-P8-CI — Core CORE-P8 validation correction

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: fixer-worker

Use only the GitHub connector. Do not merge the PR or change its lifecycle state.

## Evidence

The final CORE-P8 fixture/test/docs head has a failed Component CI result.

- code_bearing_sha: 14b92f467b5806d31ec14723b5857e64660fdf06
- workflow_run_id: 29116378264
- run_number: 1501
- run_attempt: 1
- artifact_id: 8236809649
- artifact_name: ci-diag__component-core__wf-component-ci__run-29116378264__attempt-1
- artifact_expires_at: 2026-07-11T18:59:37Z

Use the diagnostics artifact as the authoritative failure evidence.

## Required work

Read the project process files, current Core control files, CORE-P8 fixtures/tests/docs, PR diff, and every required file in artifact 8236809649. If the artifact is unavailable or incomplete, report FIX_BLOCKED_BY_LOGS.

Apply only the smallest correction demonstrated by the artifact. Preserve the versioned synthetic fixture catalog, strict public-type roundtrips, semantic decision recomputation, secrecy/path-safety checks, stable contract documentation, conflict-saved compatibility explanation, lack of production Core changes, and component ownership.

## Allowed files

- crates/haze-sync-core/tests/**
- crates/haze-sync-core/fixtures/**
- crates/haze-sync-core/docs/** only when directly required by diagnostics
- crates/haze-sync-core/src/** only if the diagnostics directly require a fixture helper or public re-export correction
- crates/haze-sync-core/control/report.md

Do not add API DTOs, TypeScript generation, runtime integrations, persistence/provider behavior, shared workflow/dependency changes, sibling components, or unrelated test changes.

Source/test/fixture/docs correction commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-sync-core/control/report.md using REPORT_TYPE FIX and phase_id FIX-CORE-P8-CI.
