# W1-CORE-P5 — Tombstone, delete guard, restore, and retention classifiers

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P4 implementation, fixer, and clean-code review are accepted. The clean-code code-bearing Component CI run is green.

- code_bearing_sha: 6ff7583831a83069f2032c2dea4a51c7dea0c634
- workflow: Component CI
- workflow_run_id: 29067684021
- run_number: 840
- conclusion: success

The next implementation phase is CORE-P5 from crates/haze-sync-core/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant current source, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement CORE-P5: Tombstone, delete guard, restore, and retention classifiers.

Follow the plan:

- harden tombstone metadata validation and retention-window tests;
- test restore-ready metadata and add pure restore/retention eligibility classifiers where required;
- test delete-guard count/ratio thresholds, zero totals, exact boundaries, overflow safety, and scoped unlock mismatch;
- keep all outputs deterministic, public-safe, storage-neutral, and adapter-neutral;
- keep physical deletion and trash movement outside Core.

## Allowed files

- crates/haze-sync-core/src/tombstone_service/**
- crates/haze-sync-core/src/delete_guard/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Non-goals

No hard-delete cleanup, filesystem trash behavior, provider calls, tombstone repository, CLI parsing, API/Server route wiring, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-core/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to CORE-P5.
