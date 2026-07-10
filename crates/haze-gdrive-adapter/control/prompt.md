# W1-FIX-GDA-P5-CI — GDrive full-scan CI correction

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P5 implementation is complete, but its code-bearing Component CI run failed.

- code_bearing_sha: e4fa4b6e890961ae5e54b6252bef094263071de6
- workflow: Component CI
- workflow_run_id: 29082382356
- run_number: 990
- run_attempt: 1
- artifact_id: 8223334844
- artifact_name: ci-diag__component-gdrive-adapter__wf-component-ci__run-29082382356__attempt-1
- artifact_expires_at: 2026-07-11T09:16:24Z

Use the diagnostics artifact as source of truth.

## Read

Read process sources, component docs/control files, current GDA-P5 source/tests, and PR diff. Download artifact 8223334844 and read summary.md, manifest.json, and every failed-check log. If unavailable or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Apply only the minimum artifact-proven correction for GDA-P5. Preserve full-scan/import planning, conservative delete candidates, mode rules, provider abstraction boundaries, and component non-goals.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/** only if diagnostics prove a documentation issue
- crates/haze-gdrive-adapter/control/report.md

## Boundaries

No export phase work, live provider integration, Core policy changes, immediate deletion behavior, direct DB ownership, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-gdrive-adapter/control/report.md. Use report-template.md, REPORT_TYPE FIX, phase_id FIX-GDA-P5-CI.
