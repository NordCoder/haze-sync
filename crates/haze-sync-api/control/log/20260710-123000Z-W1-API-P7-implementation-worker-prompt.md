# W1-API-P7 — Cross-component compatibility fixtures

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P6 implementation, CI fixer, and clean-code review are accepted. Post-fix Component CI is green.

- code_bearing_sha: c88e33a5f5f6bcca928bd0fcb119e56e2e6dde7d
- workflow: Component CI
- workflow_run_id: 29086605020
- run_number: 1099
- conclusion: success

The next implementation phase is API-P7 from crates/haze-sync-api/docs/implementation-plan.md. Its acceptance is required to unblock Obsidian OBS-P9 compatibility work.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, all current public DTOs/routes, accepted Common fixtures, downstream Server/CLI/Obsidian contract expectations, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Implement API-P7: Cross-component compatibility fixtures.

- add stable JSON fixtures for server-info, file PUT outcomes, file metadata, changes pages, conflict list/resolve, delete outcomes, public errors, and admin/status summaries where the public contract exists;
- add Rust tests asserting fixture deserialization, serialization, vocabulary, and stable public shape;
- document downstream TypeScript mirror guidance for Obsidian without editing TypeScript in this phase;
- use placeholder/synthetic values only and exclude provider-specific or runtime-sensitive payloads;
- keep client behavior outside API.

## Allowed files

- crates/haze-sync-api/**

## Non-goals

No TypeScript edits, generated client pipeline, Server route tests, provider-specific real payloads, runtime implementation, sibling component changes, workflow changes, or public contract expansion solely to make fixtures easier.

## CI trigger policy

Product/tests/docs/fixture commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-api/control/report.md. Use report-template.md, REPORT_TYPE IMPLEMENTATION, phase_id API-P7.
