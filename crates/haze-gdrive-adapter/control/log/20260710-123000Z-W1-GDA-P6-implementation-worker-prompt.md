# W1-GDA-P6 — Change feed polling and reconciliation loop

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P5 implementation, clean-code review, and artifact-based CI fixer are complete. Post-fix Component CI is green.

- code_bearing_sha: c19ce18b9c0035e0be98f75fe9a75e5589d705ca
- workflow: Component CI
- workflow_run_id: 29088281762
- run_number: 1136
- conclusion: success

The next implementation phase is GDA-P6 from crates/haze-sync-gdrive-adapter/docs/implementation-plan.md.

STOR-P7 concrete cursor persistence is not accepted yet. GDA-P6 must remain executable through an injected adapter-local cursor-store abstraction. Do not add direct Storage/DB ownership. Report concrete persistence wiring as a fan-in dependency.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current scan/mapping/cursor/retry abstractions, read-only STOR-P7 contract context if useful, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement GDA-P6: Change feed polling and reconciliation loop.

- poll Drive changes through accepted provider abstractions using cursor state;
- classify change entries into safe scan/import/export work;
- fall back to full scan when the cursor is invalidated;
- debounce and coalesce duplicate or reordered entries safely;
- advance cursor state only after successful processing through an injected abstraction;
- classify retry/backoff for provider limits and failures;
- add focused tests for cursor invalidation, duplicate/reordered entries, success-only advancement, and full-scan fallback.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/**
- crates/haze-gdrive-adapter/control/report.md

## Non-goals

No webhook/public callback infrastructure, change-feed-only correctness claim, provider deletion side effects, direct DB writes, concrete Storage wiring, Core conflict policy, export apply runner from GDA-P7, workflow/dependency changes, or sibling-component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-gdrive-adapter/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id GDA-P6.
