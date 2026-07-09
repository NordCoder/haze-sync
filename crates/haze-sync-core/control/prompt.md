# W1-FIX-CORE-P4-CI-RERUN — Core CORE-P4 CI fix

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for CORE-P4, not FIX-CORE-P4-CI. Therefore FIX-CORE-P4-CI-RERUN is not already complete.

## Context

CORE-P4 implementation completed, but Component CI for the code/docs implementation head failed.

- workflow: Component CI
- workflow_run_id: 29028092129
- run_attempt: 1
- artifact_id: 8202551525
- artifact_name: ci-diag__component-core__wf-component-ci__run-29028092129__attempt-1
- artifact_expires_at: 2026-07-10T15:07:56Z

## Read

Read implementation-manifest.md, report-template.md, fixer-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant code and PR diff. Download and read diagnostics artifact 8202551525. Read summary.md, manifest.json, and every failed-check log. If missing/expired/malformed/unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the CORE-P4 CI failure inside core scope. Expected recent changed area: conflict preservation and resolution primitives. Use the artifact as source of truth.

## Allowed files

- crates/haze-sync-core/src/conflict_service/**
- crates/haze-sync-core/src/policy_engine/**
- crates/haze-sync-core/src/conflict_saved_planner/**
- crates/haze-sync-core/docs/** only if the artifact proves a docs formatting failure
- crates/haze-sync-core/control/report.md

## Forbidden changes

No conflict route wiring, conflict row repository implementation, object-store writes, API DTO ownership changes, Obsidian UI behavior, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-core/control/report.md. Use report-template.md. Set REPORT_TYPE to FIX and phase_id to FIX-CORE-P4-CI-RERUN.
