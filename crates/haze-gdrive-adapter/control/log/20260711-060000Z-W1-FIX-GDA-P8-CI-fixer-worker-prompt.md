# W1-FIX-GDA-P8-CI — GDrive GDA-P8 validation correction

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Use only the GitHub connector. Do not merge the PR or change its lifecycle state.

## Evidence

The final GDA-P8 source/test head has a failed Component CI result.

- code_bearing_sha: 46a8840e143f0c911dc02a3107d0546bfd8242ed
- workflow_run_id: 29125187320
- run_number: 1587
- run_attempt: 1
- artifact_id: 8240045016
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29125187320__attempt-1
- artifact_expires_at: 2026-07-11T21:37:36Z

Use the diagnostics artifact as the authoritative failure evidence.

## Required work

Read the project process files, current GDrive control files, GDA-P8 source/tests, PR diff, and every required file in artifact 8240045016. If the artifact is unavailable or incomplete, report FIX_BLOCKED_BY_LOGS.

Apply only the smallest correction demonstrated by the artifact. Preserve repeated authoritative absence, unreliable-scan blocking, movement versus disappearance classification, adapter count/ratio limits, injected Core delete guard, unavailable unaudited manual unlock, dry-run immutability, idempotent Core delete submission, mapping retirement only after accepted outcomes, redaction, and component boundaries.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/** only when directly required by diagnostics
- crates/haze-gdrive-adapter/control/report.md

Do not add Drive hard delete, live credentials/provider wiring, direct Storage/DB ownership, concrete Server/API transport, background scheduling, shared workflow/dependency changes, sibling changes, or unrelated test changes.

Source/test/docs correction commits must run CI normally. A final report-only commit may skip CI.

Write only crates/haze-gdrive-adapter/control/report.md using REPORT_TYPE FIX and phase_id FIX-GDA-P8-CI.
