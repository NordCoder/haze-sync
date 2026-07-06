# W1-API-P2C — API clean-code review

Component: api
Component path: crates/haze-sync-api
Branch: component/api
Base branch: main
Target branch: main

## Role

You are a Clean-Code Reviewer for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- docs-process/docs/development-model.md
- crates/haze-sync-api/control/state.md
- crates/haze-sync-api/control/report.md as the W1 API-P2 implementation report before overwriting it
- crates/haze-sync-api/docs/component-contract.md
- crates/haze-sync-api/docs/implementation-plan.md
- crates/haze-sync-api/docs/dependency-map.md
- changed files in component/api against main

## Task

Review W1 API-P2 implementation. Verify DTO serialization tests, safe public output, placeholder documentation, passive API boundaries, no route/storage/Core/provider creep, and report honesty.

## Allowed files

- crates/haze-sync-api/src/** only for small review fixes
- crates/haze-sync-api/docs/** only for small review fixes
- crates/haze-sync-api/control/report.md

Do not edit sibling components. Do not edit workflow files.

## Report

Replace crates/haze-sync-api/control/report.md with CLEAN_CODE_REVIEW report.

Expected status: CLEAN_ACCEPT_PENDING_CI, CLEAN_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
