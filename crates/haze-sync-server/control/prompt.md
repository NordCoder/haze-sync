# T0-P3C — Server clean-code review

Component: server
Component path: crates/haze-sync-server
Branch: component/server
Base branch: main
Verified base SHA: aabf74486d4d06a89136007bd17713f3c35478de
Target branch: main

## Role

You are a Clean-Code Reviewer. Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- crates/haze-sync-server/control/prompt.md
- crates/haze-sync-server/control/log/20260705-000000Z-T0-P3-implementation-report.md
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/docs/decisions.md
- current diff of component/server against main
- crates/haze-sync-server/src/routes/health.rs

## Task

Review the T0-P3 Server documentation implementation and the tiny route doc-comment cleanup. This is a docs/control process test, not a route refactor.

Check that the Server docs are clear, accurate, simple, not overclaimed, and consistent with the current Server source. Verify the health route doc-comment cleanup is behavior-preserving. Look for stale claims, unclear runtime/API/Core/Storage boundaries, missing non-goals, missing dependency notes, duplicated wording, or claims that imply provider runtime ownership or unsafe delete behavior.

## Allowed files

- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/docs/decisions.md
- crates/haze-sync-server/control/state.md
- crates/haze-sync-server/control/report.md
- crates/haze-sync-server/src/routes/health.rs only if a tiny comment/doc-comment correction is needed

Do not rewrite routes/mod.rs or routes/v1.rs. Do not change route behavior. Do not change files outside crates/haze-sync-server. Do not archive control files.

## Report

Write the final report to crates/haze-sync-server/control/report.md using report-template.md.

Set REPORT_TYPE: CLEAN_CODE_REVIEW.

Expected final status: CLEAN_ACCEPT or CLEAN_ACCEPT_PENDING_CI.
