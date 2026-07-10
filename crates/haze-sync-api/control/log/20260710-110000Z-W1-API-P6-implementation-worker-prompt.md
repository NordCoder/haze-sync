# Archived active prompt

component: api
archived_at: 2026-07-10T11:00:00Z
wave: W1
phase: API-P6
agent_role: implementation-worker
source_path: crates/haze-sync-api/control/prompt.md
source_sha: 27b427c9386410e097a2ceedaaf4f11aa80f7a56

# W1-API-P6 — Admin/status and doctor-facing contract hardening

Component: api
Path: crates/haze-sync-api
Branch: component/api
PR: #44
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

API-P5 implementation, fixer, and clean-code review are accepted. Green CI evidence applies to the accepted code-bearing state.

- code_bearing_sha: 8a80d45685d29a681d37e0861ec8ea1ba2e734c7
- workflow_run_id: 29079842861
- run_number: 892
- conclusion: success

The next plan phase is API-P6.

## Read

Read process sources, component docs/control files, current admin/server DTOs and route helpers, accepted Server/Common contracts, and PR diff. Do not read diagnostics artifacts unless a future fixer prompt instructs it.

## Task

Implement API-P6 admin/status and doctor-facing contract hardening.

- test server-info capabilities and version/protocol metadata;
- harden admin status DTOs for readiness, adapter summaries, cursor presence, pause support, and sanitized runtime state;
- represent skipped/not-run/placeholder status honestly;
- prevent public DTOs from containing raw cursor values, token hashes, database URLs, absolute local paths, provider payloads, or raw errors;
- keep mutation/admin actions out unless an accepted public contract already exists.

## Allowed files

- crates/haze-sync-api/src/routes/admin/**
- crates/haze-sync-api/src/dto/server/**
- crates/haze-sync-api/src/dto/common/** when shared status types live there
- crates/haze-sync-api/docs/**
- crates/haze-sync-api/control/report.md

## Non-goals

No live doctor checks, Server readiness implementation, adapter pause/resume mutation, repair execution, provider calls, workflow/dependency changes, or sibling changes.

## CI trigger policy

Source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only crates/haze-sync-api/control/report.md. Use REPORT_TYPE IMPLEMENTATION and phase_id API-P6.
