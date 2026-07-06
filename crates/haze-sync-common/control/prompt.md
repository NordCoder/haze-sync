# W1-CMM-P2C — Common clean-code review

Component: common
Component path: crates/haze-sync-common
Branch: component/common
Base branch: main
Target branch: main

## Role

You are a Clean-Code Reviewer for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- docs-process/docs/development-model.md
- crates/haze-sync-common/control/state.md
- crates/haze-sync-common/control/report.md as the W1 CMM-P2 implementation report before overwriting it
- crates/haze-sync-common/docs/component-contract.md
- crates/haze-sync-common/docs/implementation-plan.md
- crates/haze-sync-common/docs/dependency-map.md
- changed files in component/common against main

## Task

Review W1 CMM-P2 implementation. Verify scope, simplicity, tests/rustdoc quality, public primitive stability, and report honesty.

## Allowed files

- crates/haze-sync-common/src/** only for small review fixes
- crates/haze-sync-common/docs/** only for small review fixes
- crates/haze-sync-common/control/report.md

Do not edit sibling components. Do not edit workflow files.

## Report

Replace crates/haze-sync-common/control/report.md with CLEAN_CODE_REVIEW report.

Expected status: CLEAN_ACCEPT_PENDING_CI, CLEAN_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
