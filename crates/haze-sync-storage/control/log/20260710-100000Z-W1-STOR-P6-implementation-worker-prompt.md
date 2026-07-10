# Archived active prompt

component: storage
archived_at: 2026-07-10T10:00:00Z
wave: W1
phase: STOR-P6
agent_role: implementation-worker
source_path: crates/haze-sync-storage/control/prompt.md
source_sha: fd5352b380ec3dafaa5f83f617237539ff201d84

# W1-STOR-P6 — Conflict, tombstone, and delete repository support

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P5 implementation, fixer, and clean-code review are accepted. The clean-code code-bearing Component CI run is green.

- code_bearing_sha: ff8c960dcd26ca5285df47d1d5446107caefaa6c
- workflow: Component CI
- workflow_run_id: 29067675101
- run_number: 839
- conclusion: success

The next implementation phase is STOR-P6 from crates/haze-sync-storage/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement STOR-P6: Conflict, tombstone, and delete repository support.

Follow the plan:

- verify conflict row insertion, listing, lookup, and status update behavior;
- verify tombstone insertion, lookup, and restore metadata behavior;
- verify operation-log rows represent conflict, delete, and tombstone events safely;
- reject unsupported conflict statuses and invalid persisted sizes through safe repository errors;
- document that full `accept_conflict` content replacement remains a Core/API/Server fan-in responsibility;
- keep transaction ownership with the caller.

## Allowed files

- crates/haze-sync-storage/src/repositories/conflicts/**
- crates/haze-sync-storage/src/repositories/tombstones/**
- crates/haze-sync-storage/src/repositories/operation_log/**
- crates/haze-sync-storage/src/models/** only when directly required by this phase
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Non-goals

No conflict policy, hard delete, filesystem/provider trash behavior, API handlers, retention cleanup job, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-storage/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to STOR-P6.
