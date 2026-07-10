# W1-CORE-P8 — Core compatibility fixtures and integration contract examples

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P7 implementation, clean-code review, and artifact-based CI correction are accepted. Final source CI is green.

- code_bearing_sha: 6bb7da230d18e68c023117498cefcd9ea81137e7
- workflow: Component CI
- workflow_run_id: 29113629870
- run_number: 1448
- conclusion: success

The next implementation phase is CORE-P8 from crates/haze-sync-core/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current public Core models and serde contracts, accepted API compatibility guidance, downstream mapping examples, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement CORE-P8: Core compatibility fixtures and integration contract examples.

- add minimal synthetic serialized examples for accepted write, same-content, conflict-saved, hash mismatch, tombstone, delete-guard block, idempotency replay/conflict, operation-log cursor outcomes, and doctor summaries where current public Core models support them;
- keep fixtures language-neutral, deterministic, secret-free, path-safe, and free of raw bytes;
- add strict fixture-stability tests that detect field, vocabulary, invariant, and semantic drift;
- document which fields are stable semantic contracts and which remain internal implementation details;
- ensure downstream components can map examples without depending on private Core internals;
- avoid duplicating API-owned HTTP DTOs or claiming runtime integration support.

## Allowed files

- crates/haze-sync-core/src/** only when fixture helpers or public re-exports are directly required
- crates/haze-sync-core/tests/**
- crates/haze-sync-core/fixtures/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Non-goals

No API DTO duplication, TypeScript generation, Server route tests, Storage repository tests, adapter runtime tests, persistence, provider behavior, live checks, workflow/dependency changes, or sibling component changes.

## CI trigger policy

Product/source/test/fixture/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id CORE-P8.
