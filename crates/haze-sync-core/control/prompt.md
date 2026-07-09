# W1-CORE-P3 — Revision service safety hardening

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P2 and its CI fixer loop are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006893977
- run_number: 456
- conclusion: success

The next implementation phase is CORE-P3 from crates/haze-sync-core/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
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

Implement CORE-P3: Revision service safety hardening.

Follow the implementation plan:

- complete tests for the base revision and content matrix;
- preserve file-missing/current-base/same-content/stale/null-base/conflict-saved semantics;
- verify hash-mismatch rejection remains safe;
- clarify public status naming in docs/tests where needed;
- preserve storage-agnostic repository/content-store/operation-log trait boundaries;
- ensure unsafe stale overwrite paths do not insert accepted revisions or append accepted operations.

## Allowed files

- crates/haze-sync-core/src/revision_service/**
- crates/haze-sync-core/src/conflict_saved_planner/**
- crates/haze-sync-core/docs/**
- crates/haze-sync-core/control/report.md

## Non-goals

- No downstream wiring.
- No Storage transaction or lock ownership.
- No API DTO ownership.
- No Server runtime behavior.
- No provider or filesystem behavior.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to crates/haze-sync-core/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
