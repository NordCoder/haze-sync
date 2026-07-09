# W1-SRV-P5C — Server conflict/delete fan-in clean-code review

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

SRV-P5 implementation and CI fixer are complete. Post-fix Component CI is green.

- workflow: Component CI
- workflow_run_id: 29028061038
- run_number: 619
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/control/prompt.md
- crates/haze-sync-server/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review SRV-P5 conflict, delete, and idempotency fan-in plus the CI fixer.

Focus areas:

- conflict list missing-storage behavior and safe public error mapping;
- metadata-only conflict resolution behavior preservation;
- DELETE base/null-base and idempotency handling;
- path locks and Core delete guard integration;
- tombstone/current-state/operation-log/idempotency atomicity where accepted dependencies allow it;
- stale shell-router test update from fixer;
- preservation of non-goals: no hard delete, no provider/worktree trash side effects, no new conflict policy, no workflow changes, no sibling changes.

## Allowed files

- crates/haze-sync-server/src/routes/conflicts.rs
- crates/haze-sync-server/src/routes/conflicts/**
- crates/haze-sync-server/src/routes/delete/**
- crates/haze-sync-server/src/routes/v1/**
- crates/haze-sync-server/src/routes/mod.rs
- crates/haze-sync-server/docs/**
- crates/haze-sync-server/control/report.md

## Forbidden changes

- Do not change API/Core/Storage contracts.
- Do not add provider/worktree behavior.
- Do not add hard delete or new conflict policy.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
