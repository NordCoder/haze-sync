# W1-SRV-P2C — Server clean-code review

Component: server
Component path: crates/haze-sync-server
Branch: component/server
Base branch: main
Target branch: main

## Role

You are a Clean-Code Reviewer for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- crates/haze-sync-server/control/state.md
- crates/haze-sync-server/control/report.md as the W1 SRV-P2 implementation report before overwriting it
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- changed files in component/server against main

## Task

Review W1 SRV-P2 implementation. Verify router/test/docs hardening, safe error output, no runtime behavior expansion, no Core/Storage policy changes, and report honesty.

## Allowed files

- crates/haze-sync-server/src/** only for small review fixes
- crates/haze-sync-server/docs/** only for small review fixes
- crates/haze-sync-server/control/report.md

Do not edit sibling components. Do not edit workflow files.

## Report

Replace crates/haze-sync-server/control/report.md with CLEAN_CODE_REVIEW report.

Expected status: CLEAN_ACCEPT_PENDING_CI, CLEAN_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
