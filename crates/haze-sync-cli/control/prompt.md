# W1-CLI-P4 — Server status and adapters read-only commands

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P3 implementation, CI fixer, and clean-code review are accepted. Component CI evidence for the accepted source state is green.

- workflow: Component CI
- workflow_run_id: 29011403478
- run_number: 537
- conclusion: success

The next implementation phase is CLI-P4 from crates/haze-sync-cli/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-cli/docs/component-contract.md
- crates/haze-sync-cli/docs/implementation-plan.md
- crates/haze-sync-cli/docs/implementation-log.md
- crates/haze-sync-cli/docs/dependency-map.md
- crates/haze-sync-cli/control/prompt.md
- crates/haze-sync-cli/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement CLI-P4: Server status and adapters read-only commands.

Follow the implementation plan:

- implement an HTTP client abstraction for accepted Server status/adapters endpoints;
- map API public errors to CLI-safe messages;
- render human-readable summaries;
- optionally support JSON output only if the output contract is accepted;
- distinguish not configured, offline, unauthorized, forbidden, server unavailable, and not ready;
- keep placeholder mode honest when no server config exists.

## Dependency guard

If Server/API status or adapters endpoints are not accepted enough for live read-only commands, do not invent route contracts. Report BLOCKED_BY_DEPENDENCY or BLOCKED_BY_CONTRACT with the exact missing endpoint/contract.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/**
- crates/haze-sync-cli/control/report.md

## Non-goals

- No admin mutations.
- No direct DB reads.
- No provider calls.
- No route changes.
- No token rotation.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, test, dependency, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
