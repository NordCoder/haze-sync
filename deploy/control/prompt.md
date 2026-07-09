# W1-DEP-P4C — Deployment migration/backup runbook clean-code review

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

DEP-P4 implementation is complete. Component CI for the code/docs-bearing commit is green.

- workflow: Component CI
- workflow_run_id: 29034814421
- run_number: 690
- conclusion: success

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- clean-code-reviewer-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- deploy/docs/component-contract.md
- deploy/docs/implementation-plan.md
- deploy/docs/implementation-log.md
- deploy/docs/dependency-map.md
- deploy/control/prompt.md
- deploy/control/report.md
- relevant current deployment docs/files and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review DEP-P4 database migrations, backup, and restore runbook.

Focus areas:

- migration execution owner is explicit and secret-safe;
- pre-migration backup steps are coherent;
- PostgreSQL backup/restore command examples use placeholders only;
- object-store backup coordination and restore order are clear;
- stopped-service or quiesced-sync requirements are explicit;
- dry-run/checklist verification is useful;
- no automatic migration runner, production DB URLs, backup archives, destructive cleanup, real credentials, or sibling changes.

## Allowed files

- deploy/docs/**
- deploy/scripts/** only if explicitly accepted and safe
- deploy/control/report.md

## Forbidden changes

- Do not add automatic migration runner.
- Do not add production DB URLs or credentials.
- Do not commit backup or restore artifacts.
- Do not add hard-delete cleanup.
- Do not change workflow files.
- Do not change sibling components.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to deploy/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.
