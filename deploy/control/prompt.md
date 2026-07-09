# W1-DEP-P4 — Database migrations, backup, and restore runbook

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

DEP-P3 implementation and clean-code review are accepted. Component CI for the deployment code-bearing clean-code head completed green.

- workflow: Component CI
- workflow_run_id: 29025503885
- run_number: 593
- conclusion: success

The next implementation phase is DEP-P4 from deploy/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- deploy/docs/component-contract.md
- deploy/docs/implementation-plan.md
- deploy/docs/implementation-log.md
- deploy/docs/dependency-map.md
- deploy/control/prompt.md
- deploy/control/report.md
- relevant current deployment files and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement DEP-P4: Database migrations, backup, and restore runbook.

Follow the deployment plan:

- document migration execution owner: CLI, Server, manual sqlx, or deploy script;
- define pre-migration backup steps;
- define database backup command examples using placeholders;
- define restore order and consistency warnings;
- document stopped-service or quiesced-sync requirements;
- add dry-run/checklist style verification.

## Allowed files

- deploy/docs/**
- deploy/scripts/** only if explicitly accepted and safe
- deploy/control/report.md

## Non-goals

- No automatic migration runner unless Server/Storage contract accepts it.
- No production DB URLs.
- No backup archives committed.
- No hard-delete cleanup.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product/deploy docs or scripts commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to deploy/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
