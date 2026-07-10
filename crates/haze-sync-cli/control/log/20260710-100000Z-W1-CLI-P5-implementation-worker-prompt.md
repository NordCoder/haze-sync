# Archived active prompt

component: cli
archived_at: 2026-07-10T10:00:00Z
wave: W1
phase: CLI-P5
agent_role: implementation-worker
source_path: crates/haze-sync-cli/control/prompt.md
source_sha: d19b342e9076d58941a37e2abfdc46e993921592

# W1-CLI-P5 — Live doctor command integration

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P4 implementation, fixer, and clean-code review are accepted. The clean-code code-bearing Component CI run is green.

- code_bearing_sha: 91b9a0cfddacace23701e964ddd11d0dea7ef3c5
- workflow: Component CI
- workflow_run_id: 29067608009
- run_number: 824
- conclusion: success

The next implementation phase is CLI-P5 from crates/haze-sync-cli/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, relevant accepted Server/API/Core diagnostic contracts, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement CLI-P5: Live doctor command integration.

Follow the plan:

- add explicit live-doctor behavior only through accepted public Server diagnostic/readiness/status surfaces;
- preserve `doctor --offline` as no-network mode;
- render Core doctor status/summary consistently where accepted;
- distinguish live, offline, skipped, and not-run checks honestly;
- avoid raw diagnostics, provider payloads, cursor values, secrets, and local paths in output;
- maintain predictable exit-code and safe public-error behavior.

## Dependency guard

Do not invent a Server doctor endpoint or DTO. If accepted Server/API surfaces are insufficient for live doctor behavior, implement only the contract-backed portion and report `BLOCKED_BY_DEPENDENCY` or `BLOCKED_BY_CONTRACT` with the exact missing surface.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/**
- crates/haze-sync-cli/control/report.md

## Non-goals

No repair behavior, direct DB/provider access, destructive checks, provider OAuth validation, Server/API route changes, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to CLI-P5.
