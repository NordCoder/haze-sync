# Archived active prompt

component: core
archived_at: 2026-07-10T11:00:00Z
wave: W1
phase: CORE-P5C
agent_role: clean-code-reviewer
source_path: crates/haze-sync-core/control/prompt.md
source_sha: d021524da2aa8e6ccc70d74b060937e71c7ad0ff

# W1-CORE-P5C — Core tombstone/delete-guard clean-code review

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P5 implementation and CI fixer are complete. Final code-bearing Component CI is green.

- code_bearing_sha: f827341777a0b1f405b446ebf564f209ea1aaa60
- workflow_run_id: 29082064452
- run_number: 976
- conclusion: success

## Read

Read process sources, component docs/control files, current source/tests/docs, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt instructs it.

## Task

Review CORE-P5 tombstone validation, restore/retention classifiers, delete-guard hardening, and CI corrections.

Focus on deterministic classifiers, metadata invariants, exact retention boundaries, threshold arithmetic, scoped unlock semantics, public-safe outputs, test fixture correctness, storage/adapter neutrality, and non-goal preservation.

## Allowed files

- crates/haze-sync-core/src/tombstone_service/**
- crates/haze-sync-core/src/delete_guard/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Boundaries

No physical cleanup, filesystem/provider actions, repository implementation, CLI parsing, API/Server wiring, workflow/dependency changes, or sibling changes.

## CI trigger policy

Source/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use REPORT_TYPE CLEAN_CODE_REVIEW and phase_id CORE-P5C.
