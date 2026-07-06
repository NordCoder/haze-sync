# W1-CORE-P2C — Core clean-code review

Component: core
Component path: crates/haze-sync-core
Branch: component/core
Base branch: main
Target branch: main

## Role

You are a Clean-Code Reviewer for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- docs-process/docs/development-model.md
- crates/haze-sync-core/control/state.md
- crates/haze-sync-core/control/report.md as the W1 CORE-P2 implementation report before overwriting it
- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/dependency-map.md
- changed files in component/core against main

## Task

Review W1 CORE-P2 implementation. Verify public Core module surface, rustdoc/test quality, serialization safety, no runtime/dependency creep, and report honesty.

## Allowed files

- crates/haze-sync-core/src/** only for small review fixes
- crates/haze-sync-core/docs/** only for small review fixes
- crates/haze-sync-core/control/report.md

Do not edit sibling components. Do not edit workflow files.

## Report

Replace crates/haze-sync-core/control/report.md with CLEAN_CODE_REVIEW report.

Expected status: CLEAN_ACCEPT_PENDING_CI, CLEAN_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
