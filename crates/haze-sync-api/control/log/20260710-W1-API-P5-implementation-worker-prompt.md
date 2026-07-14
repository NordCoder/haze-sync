# Archived active prompt

archived_by: orchestrator
archived_on: 2026-07-10
source_path: crates/haze-sync-api/control/prompt.md
source_phase: API-P5
source_role: implementation-worker
source_blob_sha: f171eb964736ac96044ce42f0738eee4bbc916ce

---

# W1-API-P5 — Conflict and delete route-helper contract hardening

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P4 implementation, fixer, and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29034803865
- run_number: 689
- conclusion: success

The next implementation phase is API-P5 from crates/haze-sync-api/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant current source, and PR diff.

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement API-P5: Conflict and delete route-helper contract hardening.

Follow the plan:

- test conflict list query parsing, especially status filters and bounds;
- test conflict resolve action parsing for accept_current, accept_conflict, keep_both, and mark_resolved;
- test conflict list/resolve DTO serde roundtrips and safe path/hash/revision fields;
- test DELETE metadata extraction for path, base revision/null-base, idempotency key, and adapter principal requirement;
- test delete response vocabulary for tombstoned, not_found, rejected, and guard-blocked outcomes;
- ensure API does not implement conflict resolution or tombstone creation itself.

## Allowed files

- crates/haze-sync-api/src/routes/conflicts/**
- crates/haze-sync-api/src/routes/delete/**
- crates/haze-sync-api/src/dto/conflicts/**
- crates/haze-sync-api/src/dto/files/** when delete DTOs live there
- crates/haze-sync-api/docs/**
- crates/haze-sync-api/control/report.md

## Non-goals

No conflict row repository, content replacement implementation, tombstone persistence, provider/worktree trash behavior, mass-delete guard execution, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-api/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to API-P5.
