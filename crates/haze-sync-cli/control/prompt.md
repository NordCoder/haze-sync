# W1-CLI-P3 — CLI config and secret-source foundation

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P2 and its CI fixer loop are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006877578
- run_number: 454
- conclusion: success

The next implementation phase is CLI-P3 from crates/haze-sync-cli/docs/implementation-plan.md.

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

Implement CLI-P3: Config and secret-source foundation.

Follow the implementation plan:

- define config source precedence for server URL, profile, output format, and token source;
- support safe token source representations without encouraging shell-history token usage;
- redact config and token values in debug/errors;
- avoid printing local absolute secret paths unless local-only diagnostics explicitly allow them;
- add tests for redaction and invalid config errors.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/**
- crates/haze-sync-cli/control/report.md

## Non-goals

- No token creation or rotation.
- No live server calls unless explicitly justified by this phase.
- No deployment secret provisioning.
- No provider tokens.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
