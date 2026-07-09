# W1-DEP-P3 — Deployment server packaging and service wiring

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review.

## Context

DEP-P2 clean-code review completed with CLEAN_ACCEPT_PENDING_CI.

Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29003702641
- run_number: 413
- conclusion: success

The next deployment phase is DEP-P3 from deploy/docs/implementation-plan.md.

## Read

Read all required sources before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- deploy/docs/component-contract.md
- deploy/docs/implementation-plan.md
- deploy/docs/implementation-log.md
- deploy/docs/dependency-map.md
- deploy/control/prompt.md
- deploy/control/report.md
- relevant current repository code

Also inspect the current Server component contract/docs only as dependency context for startup/config/readiness assumptions. Do not modify Server files.

## Task

Implement DEP-P3: Server packaging and service wiring.

Follow the DEP-P3 plan:

- decide and document the server packaging path;
- define server environment variables and volumes using placeholders only;
- wire PostgreSQL connection and object-store path for deployment configuration;
- expose local/proxy-bound listen address safely;
- add readiness/liveness checks where appropriate;
- document startup and shutdown;
- avoid auto-running migrations unless the accepted contracts explicitly require it.

## Dependency guard

If Server startup/config/readiness behavior is not sufficiently accepted for DEP-P3 wiring, do not invent it in Deployment.

In that case, report BLOCKED_BY_DEPENDENCY or BLOCKED_BY_CONTRACT with the exact missing dependency.

## Allowed files

- deploy/**
- .env.example only if placeholder alignment is required

## Forbidden changes

- Do not modify Server code.
- Do not modify GDrive adapter code.
- Do not modify Worktree code.
- Do not add production secrets.
- Do not add TLS private keys.
- Do not add remote deployment automation.
- Do not start provider/runtime sync behavior.
- Do not auto-run migrations unless accepted by contract.
- Do not archive control files.

## Checks

Run applicable checks if possible.

If you cannot run shell commands, say so in the report. Do not claim checks passed unless you actually ran them or observed CI metadata.

## Report

Write only the report to deploy/control/report.md.

Use report-template.md.

Set REPORT_TYPE to IMPLEMENTATION.

Use one of:

- SELF_ACCEPT
- SELF_ACCEPT_PENDING_CI
- SELF_NEEDS_FIX
- BLOCKED_BY_CONTRACT
- BLOCKED_BY_DEPENDENCY
- BLOCKED_BY_TOOLING
