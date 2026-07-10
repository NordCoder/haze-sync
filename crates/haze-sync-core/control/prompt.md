# W1-CORE-P7C — Core doctor and safety-report clean-code review

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P7 implementation and artifact-based CI correction are complete. Final source CI is green.

- code_bearing_sha: ae5fe8a8099973a8705af98e091b369038dbf71e
- workflow: Component CI
- workflow_run_id: 29107333698
- run_number: 1368
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current doctor source/tests/docs, fixer changes, accepted CLI/Server consumers, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt explicitly instructs it.

## Task

Review CORE-P7 passive doctor and safety-report models plus both artifact-based compatibility corrections.

Focus on:

- explicit ok/warning/failed/skipped/not_run/placeholder semantics;
- deterministic aggregate precedence and empty-report behavior;
- fixed redacted message vocabulary;
- validating deserialization and check/detail consistency;
- adapter cursor, GDrive mapping, Worktree drift, DB, object-store, missing-blob, and token-sanity models;
- source compatibility restored for accepted CLI consumers without reverting to arbitrary strings;
- modular decomposition and public re-exports;
- test and documentation clarity;
- passive Core ownership and non-goal preservation.

## Allowed files

- crates/haze-sync-core/src/doctor/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Boundaries

No live checks, CLI implementation changes, HTTP policy, persistence, provider behavior, repair execution, workflow/dependency changes, sibling changes, test deletion, assertion weakening, or arbitrary public diagnostic strings.

## CI trigger policy

Source/test/docs clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-core/control/report.md. Use report-template.md, REPORT_TYPE CLEAN_CODE_REVIEW, phase_id CORE-P7C.
