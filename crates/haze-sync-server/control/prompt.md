# W1-SRV-P6-RERUN — Admin/status, readiness, doctor, and observability hardening

Component: server
Path: crates/haze-sync-server
Branch: component/server
PR: #45
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for SRV-P5C, not SRV-P6. Therefore SRV-P6-RERUN is not already complete.

## Context

SRV-P5 implementation, CI fixer, and clean-code review are accepted. Component CI evidence for the accepted product-code state is green.

- workflow: Component CI
- workflow_run_id: 29028061038
- run_number: 619
- conclusion: success

The next implementation phase is SRV-P6 from crates/haze-sync-server/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, component docs/control files, relevant code and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement SRV-P6: Admin/status, readiness, doctor, and observability hardening. Focus on honest read-only operational surfaces, safe readiness checks, safe adapter/status summaries, skipped/not-run check representation, redacted logs/tracing if scoped, and metrics only if accepted by system scope.

## Allowed files

- crates/haze-sync-server/src/routes/admin/**
- crates/haze-sync-server/src/readiness/**
- crates/haze-sync-server/src/db/**
- crates/haze-sync-server/src/http/**
- crates/haze-sync-server/docs/**
- crates/haze-sync-server/control/report.md

## Non-goals

No admin mutations by default, repair execution, token rotation, provider calls without accepted provider contract, raw cursor/status payload exposure, workflow changes, or sibling component changes.

## CI trigger policy

Product/source/docs commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to crates/haze-sync-server/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to SRV-P6-RERUN.
