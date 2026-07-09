# W1-CLI-P3C — CLI config clean-code review

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P3 implementation and CI fixer are complete. Post-fix Component CI is green.

- workflow: Component CI
- workflow_run_id: 29011403478
- run_number: 537
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
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

Review CLI-P3 config and secret-source foundation plus the CI fix.

Focus areas:

- config source precedence model;
- safe token source descriptors;
- redaction of server URLs, token paths, OS-secret refs, and token-source values;
- inline token rejection;
- Default derivation after clippy fix;
- preservation of non-goals: no config IO, no server calls, no token creation/rotation, no provider tokens.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/**
- crates/haze-sync-cli/control/report.md

## Forbidden changes

- Do not add config/env/file/stdin/keychain IO.
- Do not add server calls.
- Do not add token creation or rotation.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
