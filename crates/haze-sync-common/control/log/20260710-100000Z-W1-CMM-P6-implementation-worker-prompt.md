# Archived active prompt

component: common
archived_at: 2026-07-10T10:00:00Z
wave: W1
phase: CMM-P6
agent_role: implementation-worker
source_path: crates/haze-sync-common/control/prompt.md
source_sha: 5f9f4b020676d29538f34491457c95e45f617ff7

# W1-CMM-P6 — Shared primitive compatibility fixtures

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P5 implementation and clean-code review are accepted. The clean-code code-bearing Component CI run is green.

- code_bearing_sha: 987d76b451b209a4996c5bdaabf2c16cc0e0f5ca
- workflow: Component CI
- workflow_run_id: 29067608602
- run_number: 825
- conclusion: success

The next implementation phase is CMM-P6 from crates/haze-sync-common/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, downstream mirror-type documentation where read-only access is useful, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement CMM-P6: Shared primitive compatibility fixtures.

Follow the plan:

- provide lightweight JSON examples or fixtures for VaultPath, shared IDs, content hashes, adapter roles/modes, and safe validation errors;
- add Rust tests that assert fixture compatibility and stable wire values;
- document how downstream Rust components and TypeScript clients should mirror the examples;
- keep fixtures limited to Common-owned primitives rather than API DTOs or Core policy;
- ensure examples contain no real credentials, local paths, or environment-specific values.

## Allowed files

- crates/haze-sync-common/**
- crates/haze-sync-common/control/report.md

## Non-goals

No TypeScript edits, generated-code pipeline, API DTO ownership, Core policy, adapter runtime behavior, workflow changes, dependency changes, or sibling component changes.

## CI trigger policy

Product/source/docs/fixture commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to CMM-P6.
