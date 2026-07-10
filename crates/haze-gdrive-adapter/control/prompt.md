# W1-GDA-P7 — Core changes export planner and Drive apply runner

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P6 implementation, clean-code review, and artifact-based CI correction are accepted. Final source CI is green.

- code_bearing_sha: 4604182f01acc30cf4f689ac3a3216d0f5cc1298
- workflow: Component CI
- workflow_run_id: 29107022381
- run_number: 1347
- conclusion: success

The next implementation phase is GDA-P7 from crates/haze-gdrive-adapter/docs/implementation-plan.md.

Storage STOR-P8 mapping/state repository work is not yet accepted. Keep mapping persistence and Core/API access behind injected abstractions; do not add direct DB or concrete sibling wiring.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current change-feed/mapping/echo abstractions, accepted API/Core change and tombstone contracts, fake provider/client boundaries, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement GDA-P7: Core changes export planner and Drive apply runner.

- read accepted Core/API changes through an injected client boundary;
- plan provider create, update, and trash operations from accepted revisions and tombstones;
- verify source bytes and expected content hash before provider mutation;
- respect export_only, bidirectional, dry_run, and non-exporting modes;
- update mapping and echo state only after confirmed provider success;
- classify provider conflicts, rate limits, transient failures, and retries safely;
- preserve at-least-once/replay-safe semantics where caller persistence can fail;
- add fake-provider and fake-Core tests for success, retry, conflict, dry-run, mode filtering, mapping ordering, and echo confirmation.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/**
- crates/haze-gdrive-adapter/control/report.md

## Non-goals

No Core conflict policy, Drive hard delete, Docs conversion, real credentials/OAuth, direct Storage/DB ownership, concrete Server wiring, background scheduler, plugin/worktree behavior, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/test/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-gdrive-adapter/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id GDA-P7.
