# W1-CORE-P3C — Core revision-service clean-code review

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P3 implementation and CI fixer are complete. Post-fix Component CI is green.

- workflow: Component CI
- workflow_run_id: 29011346160
- run_number: 536
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/implementation-log.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/control/prompt.md
- crates/haze-sync-core/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review CORE-P3 revision-service safety hardening and the CI fix.

Focus areas:

- test matrix clarity for base revision and content cases;
- same-content/idempotent outcome coverage;
- stale/null-base conflict-saved safety;
- hash-mismatch pre-storage rejection;
- no accepted-write side effects for unsafe stale overwrite paths;
- preservation of production behavior and public API semantics.

## Allowed files

- crates/haze-sync-core/src/revision_service/**
- crates/haze-sync-core/src/conflict_saved_planner/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Forbidden changes

- Do not add downstream wiring.
- Do not add Storage transaction or lock ownership.
- Do not change API DTO ownership.
- Do not add Server runtime behavior.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-core/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
