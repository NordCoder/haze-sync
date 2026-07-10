# W1-SRV-P6C — Server admin/status clean-code review

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

SRV-P6-RERUN implementation report closed the active implementation phase with SELF_ACCEPT_PENDING_CI.

Important: Orchestrator has not confirmed green CI for the SRV-P6 code-bearing state. The older CI run 29028061038 predates SRV-P6 and must not be treated as SRV-P6 CI evidence.

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, component docs/control files, current server source/tests/docs, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review SRV-P6 admin/status, readiness, doctor, and observability hardening.

Focus areas:

- readiness-driven admin status behavior;
- DB and object-store readiness summaries;
- safe adapter summaries, cursor-presence output, pause support, and mode/status summaries;
- honest skipped/not-run/placeholder status representation;
- redacted logging or observability changes if present;
- preservation of read-only operational semantics and non-goals.

## CI handling

Attempt to observe Component CI for the SRV-P6 code-bearing commit through GitHub metadata. If no green SRV-P6 code-bearing CI run can be verified, do not report CLEAN_ACCEPT. Use CLEAN_ACCEPT_PENDING_CI if the code review passes but CI remains unknown or pending.

## Allowed files

- crates/haze-sync-server/src/routes/admin/**
- crates/haze-sync-server/src/readiness/**
- crates/haze-sync-server/src/db/**
- crates/haze-sync-server/src/http/**
- crates/haze-sync-server/docs/**
- crates/haze-sync-server/control/report.md

## Forbidden changes

No admin mutations, repair execution, token rotation, provider calls without accepted provider contract, raw cursor/status payload exposure, workflow changes, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to SRV-P6C.
