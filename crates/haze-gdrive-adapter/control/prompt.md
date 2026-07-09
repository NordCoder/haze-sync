# W1-GDA-P4 — Mapping, cursor, and echo-state boundary

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P3 implementation, clean-code review, CI fixer, and post-fix CI are complete.

- workflow: Component CI
- workflow_run_id: 29027970390
- run_number: 615
- conclusion: success

The next implementation phase is GDA-P4 from crates/haze-gdrive-adapter/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-gdrive-adapter/docs/component-contract.md
- crates/haze-gdrive-adapter/docs/implementation-plan.md
- crates/haze-gdrive-adapter/docs/implementation-log.md
- crates/haze-gdrive-adapter/docs/dependency-map.md
- crates/haze-gdrive-adapter/control/prompt.md
- crates/haze-gdrive-adapter/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement GDA-P4: Mapping, cursor, and echo-state boundary.

Follow the implementation plan:

- define adapter-local mapping model aligned with the accepted gdrive mapping shape;
- track path, Drive file id, parent id, name, checksum, Drive version or modified time, Core revision, Core sequence, last imported/exported timestamps, last seen timestamp, and delete candidate timestamp where scoped;
- define cursor progression model for Drive change feed and Core changes feed;
- implement echo guard for adapter-created Drive writes;
- document and enforce the accepted persistence boundary without direct DB access unless explicitly blocked and reported.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/**
- crates/haze-gdrive-adapter/control/report.md

## Non-goals

- No direct DB access unless explicitly accepted by existing component contract.
- No provider sync loop.
- No Core policy.
- No hard delete.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-gdrive-adapter/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
