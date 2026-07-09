# W1-DEP-P4C-RERUN — Deployment migration/backup runbook clean-code review

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Rerun guard

This is an explicit refreshed active prompt. The current report before this prompt was for DEP-P4, not DEP-P4C. Therefore DEP-P4C-RERUN is not already complete.

## Context

DEP-P4 implementation is complete. Component CI for the code/docs-bearing commit is green.

- workflow: Component CI
- workflow_run_id: 29034814421
- run_number: 690
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, deployment docs/control files, relevant deployment docs/files and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review DEP-P4 database migrations, backup, and restore runbook. Focus on explicit migration execution ownership, secret-safe placeholders, backup/restore order, object-store coordination, stopped/quiesced writer requirements, dry-run/checklist usefulness, and non-goal preservation.

## Allowed files

- deploy/docs/**
- deploy/scripts/** only if explicitly accepted and safe
- deploy/control/report.md

## Forbidden changes

No automatic migration runner, production DB URLs or credentials, backup/restore artifacts, hard-delete cleanup, workflow changes, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to deploy/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to DEP-P4C-RERUN.
