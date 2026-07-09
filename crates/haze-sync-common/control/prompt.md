# W1-CMM-P4C — Common identifier/hash clean-code review

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P4 implementation and CI fixer are complete. Post-fix Component CI is green.

- workflow: Component CI
- workflow_run_id: 29028045014
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

Review CMM-P4 identifier and hash contract hardening plus the CI formatter fix.

Focus areas:

- identifier max length and allowed character tests;
- typed ID prefix rejection for missing, wrong, and case-mismatched prefixes;
- adapter IDs that resemble typed IDs;
- invalid identifier input rejection;
- SHA-256 canonical lowercase prefixed output and digest normalization;
- invalid hash length and invalid hash character rejection;
- serde accepted/rejected input coverage;
- docs decisions to avoid speculative Blob, Cursor, and Tombstone IDs and hash computation helpers in common.

## Allowed files

- crates/haze-sync-common/src/ids.rs
- crates/haze-sync-common/src/hash.rs
- crates/haze-sync-common/src/error.rs
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/control/report.md

## Forbidden changes

- Do not add storage behavior.
- Do not add Core behavior.
- Do not add runtime/provider behavior.
- Do not change sibling components.
- Do not change workflow files.
- Do not remove test coverage without replacement coverage.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
