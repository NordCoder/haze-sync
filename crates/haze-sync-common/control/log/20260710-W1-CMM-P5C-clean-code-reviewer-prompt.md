# Archived active prompt

archived_by: orchestrator
archived_on: 2026-07-10
source_path: crates/haze-sync-common/control/prompt.md
source_phase: CMM-P5C
source_role: clean-code-reviewer
source_blob_sha: 9be5890bc3c1df74f26467d576e6ee48dce9e70a
archive_note: one sensitive-terminology line is recoverable from source_blob_sha and was omitted because the connector safety filter rejected the exact archived payload

---

# W1-CMM-P5C — Common adapter/security clean-code review

Component: common
Path: crates/haze-sync-common
Branch: component/common
PR: #46
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CMM-P5 implementation is complete. Component CI evidence for the code-bearing commit is green.

- workflow: Component CI
- workflow_run_id: 29038641448
- run_number: unknown
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current source, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review CMM-P5 adapter mode, role, and security primitive hardening.

Focus areas:

- AdapterRole and AdapterMode wire values and serde behavior;
- policy-free mode helpers;
- ReadonlyAgent V1 decision;
- SecretString display/debug/formatting behavior;
- the existing decision to keep credential lifecycle and speculative wrappers outside common;
- docs and tests clarity.

## Allowed files

- crates/haze-sync-common/src/adapter.rs
- crates/haze-sync-common/src/security/**
- crates/haze-sync-common/src/error.rs
- crates/haze-sync-common/docs/**
- crates/haze-sync-common/control/report.md

## Forbidden changes

No storage behavior, Core behavior, runtime/provider behavior, sibling component changes, workflow changes, permission enforcement in common, or credential lifecycle behavior.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-common/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to CMM-P5C.
