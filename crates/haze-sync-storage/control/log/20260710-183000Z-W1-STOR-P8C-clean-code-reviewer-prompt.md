# W1-STOR-P8C — Storage mapping and worktree state clean-code review

Component: storage
Path: crates/haze-sync-storage
Branch: component/storage
PR: #47
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

STOR-P8 implementation and artifact-based CI correction are complete. Final source CI is green.

- code_bearing_sha: 0a0a399db83603a87c730c5a44c11cb58c2176e8
- workflow: Component CI
- workflow_run_id: 29110120192
- run_number: 1402
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current GDrive mapping and Worktree state repositories/tests, fixer changes, accepted adapter/worktree persistence boundaries, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Review STOR-P8 mapping and Worktree state persistence plus the formatter correction.

Focus on:

- full-fact upsert and lookup semantics;
- caller-owned executor and transaction boundaries;
- canonical path, opaque provider identifier, checksum, revision, sequence, boolean, and timestamp validation;
- safe repository errors without raw provider values or paths;
- duplicate/unique mapping behavior;
- adapter policy remaining outside Storage;
- feature-gated PostgreSQL roundtrip tests and unit tests;
- module boundaries, duplication, documentation, and non-goal preservation.

## Allowed files

- crates/haze-sync-storage/src/repositories/gdrive_mapping/**
- crates/haze-sync-storage/src/repositories/worktree_state/**
- crates/haze-sync-storage/src/repositories/mod.rs
- crates/haze-sync-storage/src/models/** only when directly required by review findings
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/report.md

## Boundaries

No provider calls, credential loading, provider identity interpretation, adapter mode or sync policy, filesystem behavior, public rendering, schema/migration expansion, workflow/dependency changes, sibling changes, test deletion, or assertion weakening.

## CI trigger policy

Source/test/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-storage/control/report.md. Use report-template.md, REPORT_TYPE CLEAN_CODE_REVIEW, phase_id STOR-P8C.
