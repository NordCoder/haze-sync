# W1-CMM-P3C — Common VaultPath clean-code review

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P3 implementation completed with SELF_ACCEPT_PENDING_CI. Product-code/docs CI is green.

- workflow: Component CI
- workflow_run_id: 29009442790
- run_number: 492
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-common/docs/component-contract.md
- crates/haze-sync-common/docs/implementation-plan.md
- crates/haze-sync-common/docs/implementation-log.md
- crates/haze-sync-common/docs/dependency-map.md
- crates/haze-sync-common/control/prompt.md
- crates/haze-sync-common/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review CMM-P3 VaultPath contract hardening and improve it only where appropriate.

Focus areas:

- VaultPath test matrix clarity and coverage;
- encoded traversal and platform escape cases;
- reserved runtime/state path boundaries;
- decision that conflict/outbox materialization paths remain syncable;
- contract documentation precision;
- preservation of current production path behavior.

## Allowed files

- crates/haze-sync-common/src/path.rs
- crates/haze-sync-common/src/error.rs
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/control/report.md

## Forbidden changes

- Do not change runtime behavior unless required to fix a clear contract mismatch.
- Do not add provider-specific path exceptions.
- Do not change Core/API/Storage/Server behavior.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
