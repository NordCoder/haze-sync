# T0-P1C — Core clean-code review

Component: core
Component path: crates/haze-sync-core
Branch: component/core
Base branch: main
Verified base SHA: aabf74486d4d06a89136007bd17713f3c35478de
Target branch: main

## Role

You are a Clean-Code Reviewer. Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- crates/haze-sync-core/control/prompt.md
- crates/haze-sync-core/control/log/20260705-000000Z-T0-P1-implementation-report.md
- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/implementation-log.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/docs/decisions.md
- current diff of component/core against main

## Task

Review the T0-P1 Core documentation implementation. This is a docs/control process test, not a product-code task.

Check that the Core docs are clear, accurate, simple, not overclaimed, and consistent with the current Core source. Look for stale claims, unclear boundaries, missing non-goals, missing dependency notes, duplicated wording, or claims that imply runtime/API/storage/provider ownership.

## Allowed files

- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/implementation-log.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/docs/decisions.md
- crates/haze-sync-core/control/state.md
- crates/haze-sync-core/control/report.md

Do not edit Core source. Do not change files outside crates/haze-sync-core. Do not archive control files.

## Report

Write the final report to crates/haze-sync-core/control/report.md using report-template.md.

Set REPORT_TYPE: CLEAN_CODE_REVIEW.

Expected final status: CLEAN_ACCEPT or CLEAN_ACCEPT_PENDING_CI.
