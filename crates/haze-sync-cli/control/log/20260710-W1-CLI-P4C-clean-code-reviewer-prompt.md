# Archived active prompt

archived_by: orchestrator
archived_on: 2026-07-10
source_path: crates/haze-sync-cli/control/prompt.md
source_phase: CLI-P4C
source_role: clean-code-reviewer
source_blob_sha: ce683ccb514542e3704528f40a52da2a1f202889

---

# W1-CLI-P4C — CLI status/adapters clean-code review

Component: cli
Path: crates/haze-sync-cli
Branch: component/cli
PR: #48
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

CLI-P4 implementation and follow-up CI fixer are complete. Follow-up Component CI is green.

- workflow: Component CI
- workflow_run_id: 29038501230
- run_number: 738
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant current source, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review CLI-P4 server status and adapters read-only commands plus the CI fixer.

Focus areas:

- read-only server/API client boundary;
- status/adapters command output compatibility;
- safe rendering and redaction;
- error mapping for not configured, offline, unauthorized, forbidden, server unavailable, and not ready;
- placeholder mode honesty when no server config exists;
- preservation of non-goals.

## Allowed files

- crates/haze-sync-cli/src/**
- crates/haze-sync-cli/docs/**
- crates/haze-sync-cli/control/report.md

## Forbidden changes

No admin mutations, direct DB reads, provider calls, sibling route changes, token rotation, workflow changes, sibling component changes, or test deletion.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-cli/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to CLI-P4C.
