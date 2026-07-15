# W1-GDA-P5 — Full scan and import planner

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P4 implementation, clean-code review, and CI fixer are accepted. Post-fix Component CI is green.

- code_bearing_sha: 097b78cd1642ede9a393018ab3c35dc7e1752abb
- workflow: Component CI
- workflow_run_id: 29079858205
- run_number: 893
- conclusion: success

The next implementation phase is GDA-P5 from crates/haze-gdrive-adapter/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, relevant accepted Common/API/Core contracts, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement GDA-P5: Full scan and import planner.

Follow the plan:

- list a configured Drive subtree through the existing fake/provider abstraction;
- normalize supported folder/file paths into Common-compatible vault paths;
- detect new, modified, missing, and unsupported entries;
- download supported file bytes only when required by the plan;
- compute and verify content hashes;
- prepare Core/API upload requests with known base revision or explicit null base;
- represent Drive disappearance as a conservative delete candidate, never an immediate delete;
- apply adapter mode rules before import planning;
- add fake-provider tree coverage.

## Persistence guard

GDA-P4 intentionally leaves durable mapping/cursor persistence unresolved. GDA-P5 may use explicit in-memory or injected state abstractions, but must not claim or add direct DB ownership. If durable persistence is required to complete the phase, report BLOCKED_BY_CONTRACT with the exact missing boundary rather than choosing one implicitly.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/**
- crates/haze-gdrive-adapter/control/report.md

## Non-goals

No Drive export, real provider network calls unless already accepted by the provider abstraction, hard delete, Core policy decisions, Docs conversion, direct DB access, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-gdrive-adapter/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to GDA-P5.
