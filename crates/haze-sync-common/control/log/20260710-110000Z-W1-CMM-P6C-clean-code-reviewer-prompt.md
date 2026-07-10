# Archived active prompt

component: common
archived_at: 2026-07-10T11:00:00Z
wave: W1
phase: CMM-P6C
agent_role: clean-code-reviewer
source_path: crates/haze-sync-common/control/prompt.md
source_sha: 9b65e948ef70c08d2475c97f8c1d18697225d8cd

# W1-CMM-P6C — Common compatibility fixture clean-code review

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P6 implementation and CI fixer are complete. Final fixture/test/docs-bearing Component CI is green.

- code_bearing_sha: 30b796ce4993d944adad0d13c0110cc62e0b34e0
- workflow_run_id: 29081853439
- run_number: 958
- conclusion: success

## Read

Read process sources, component docs/control files, current fixture/tests/docs, Common primitives, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt instructs it.

## Task

Review CMM-P6 shared primitive compatibility fixtures plus the formatting correction.

Focus on fixture completeness, stable wire vocabularies, VaultPath/hash/ID examples, adapter role/mode capability expectations, validation-error safety, Rust fixture tests, downstream mirroring guidance, deterministic secret-free examples, and Common ownership boundaries.

## Allowed files

- crates/haze-sync-common/fixtures/**
- crates/haze-sync-common/tests/**
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/src/** when directly relevant to review findings
- crates/haze-sync-common/control/report.md

## Boundaries

No TypeScript edits, generated client pipeline, API DTO ownership, Core policy, runtime/provider behavior, workflow/dependency changes, or sibling changes.

## CI trigger policy

Source/tests/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-common/control/report.md. Use REPORT_TYPE CLEAN_CODE_REVIEW and phase_id CMM-P6C.
