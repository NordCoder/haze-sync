# W1-CLI-P2C — CLI clean-code review

Component: cli
Component path: crates/haze-sync-cli
Branch: component/cli
Base branch: main
Target branch: main

## Role

You are a Clean-Code Reviewer for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- crates/haze-sync-cli/control/state.md
- crates/haze-sync-cli/control/report.md as the W1 CLI-P2 implementation report before overwriting it
- crates/haze-sync-cli/docs/component-contract.md
- crates/haze-sync-cli/docs/implementation-plan.md
- changed files in component/cli against main

## Task

Review W1 CLI-P2 implementation. Verify parser/output model quality, safe parse errors, no live calls or mutation behavior, testability, and report honesty.

## Allowed files

- crates/haze-sync-cli/src/** only for small review fixes
- crates/haze-sync-cli/docs/** only for small review fixes
- crates/haze-sync-cli/control/report.md

Do not edit sibling components. Do not edit workflow files.

## Report

Replace crates/haze-sync-cli/control/report.md with CLEAN_CODE_REVIEW report.

Expected status: CLEAN_ACCEPT_PENDING_CI, CLEAN_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
