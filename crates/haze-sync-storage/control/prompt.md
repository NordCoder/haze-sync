# W1-STOR-P8 — Adapter mapping and worktree state repository support

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P7 implementation, clean-code review, and artifact-based CI correction are accepted. Final source CI is green.

- code_bearing_sha: 134923151514f5d65bad7939a5703e90ff268609
- workflow: Component CI
- workflow_run_id: 29102448996
- run_number: 1243
- conclusion: success

The next implementation phase is STOR-P8 from crates/haze-sync-storage/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current repository/model modules, accepted GDrive mapping and Worktree state contracts, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement STOR-P8: Adapter mapping and Worktree state repository support.

- verify or add repository helpers for durable GDrive mapping when Storage is the accepted persistence owner;
- verify or add repository helpers for Worktree state;
- keep provider identity interpretation, echo handling, delete candidate logic, import/export decisions, scans, materialization, and dirty-state semantics outside Storage;
- validate stored identifiers, paths, hashes, sequences, timestamps, and safe row mappings;
- add focused storage-level persistence tests and documentation.

## Allowed files

- crates/haze-sync-storage/src/repositories/**
- crates/haze-sync-storage/src/models/**
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Non-goals

No provider API calls, credential loading, Worktree filesystem behavior, adapter mode enforcement, sync policy decisions, public admin rendering, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/test/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id STOR-P8.
