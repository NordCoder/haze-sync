# W1-FIX-CORE-P4-CI — Core CORE-P4 CI fix

Component: core
Path: crates/haze-sync-core
Branch: component/core
PR: #43
Role: fixer-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CORE-P4 implementation completed, but Component CI for the code/docs implementation head failed.

- workflow: Component CI
- workflow_run_id: 29028092129
- run_number: unknown
- run_attempt: 1
- artifact_id: 8202551525
- artifact_name: ci-diag__component-core__wf-component-ci__run-29028092129__attempt-1
- artifact_expires_at: 2026-07-10T15:07:56Z

Use the diagnostics artifact as source of truth.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- fixer-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-sync-core/docs/component-contract.md
- crates/haze-sync-core/docs/implementation-plan.md
- crates/haze-sync-core/docs/implementation-log.md
- crates/haze-sync-core/docs/dependency-map.md
- crates/haze-sync-core/control/prompt.md
- crates/haze-sync-core/control/report.md
- relevant current repository code and PR diff

Download and read diagnostics artifact 8202551525. Read summary.md, manifest.json, and every failed-check log.

If the artifact is missing, expired, malformed, or unreadable, report FIX_BLOCKED_BY_LOGS.

## Task

Fix the minimum cause of the CORE-P4 CI failure inside core scope.

Expected recent changed area: conflict preservation and resolution primitives. Use the artifact as source of truth.

## Allowed files

- crates/haze-sync-core/src/conflict_service/**
- crates/haze-sync-core/src/policy_engine/**
- crates/haze-sync-core/src/conflict_saved_planner/**
- crates/haze-sync-core/docs/** only if the artifact proves a docs formatting failure
- crates/haze-sync-core/control/report.md

## Forbidden changes

- Do not add conflict route wiring.
- Do not add conflict row repository implementation.
- Do not add object-store writes.
- Do not change API DTO ownership.
- Do not add Obsidian conflict UI behavior.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and fixer-worker-prompt.md. Product/source/docs fixer commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-core/control/report.md.

Use report-template.md. Set REPORT_TYPE to FIX.
