# W1-STOR-P2C — Storage clean-code review

Component: storage
Component path: crates/haze-sync-storage
Branch: component/storage
Base branch: main
Target branch: main

## Role

You are a Clean-Code Reviewer for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- crates/haze-sync-storage/control/state.md
- crates/haze-sync-storage/control/report.md as the W1 STOR-P2 implementation report before overwriting it
- crates/haze-sync-storage/docs/component-contract.md
- crates/haze-sync-storage/docs/implementation-plan.md
- changed files in component/storage against main

## Task

Review W1 STOR-P2 implementation. Verify storage scope, schema metadata tests, row model tests, no schema behavior drift, and report honesty.

## Allowed files

- crates/haze-sync-storage/src/** only for small review fixes
- crates/haze-sync-storage/docs/** only for small review fixes
- crates/haze-sync-storage/control/report.md

Do not edit sibling components. Do not edit workflow files.

## Report

Replace crates/haze-sync-storage/control/report.md with CLEAN_CODE_REVIEW report.

Expected status: CLEAN_ACCEPT_PENDING_CI, CLEAN_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
