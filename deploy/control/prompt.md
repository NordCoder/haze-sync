# W1-DEP-P3 — Server packaging and service wiring

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

DEP-P2 clean-code review and Component CI were previously accepted. DEP-P3 was blocked until Server startup/config/readiness behavior became accepted enough to use as a deployment dependency.

The Server component now has accepted Server startup/config/readiness work from earlier phases and accepted SRV-P4 clean-code review with green CI. Orchestrator authorizes DEP-P3 to use accepted Server control reports/docs as dependency evidence, but not to modify Server.

Server dependency evidence to read:

- crates/haze-sync-server/control/report.md on component/server
- crates/haze-sync-server/docs/component-contract.md on component/server
- crates/haze-sync-server/docs/implementation-plan.md on component/server
- crates/haze-sync-server/docs/implementation-log.md on component/server

Deployment must still remain deploy-scope only. If the accepted Server surface is insufficient for a concrete deployment wiring decision, report BLOCKED_BY_DEPENDENCY with exact missing behavior.

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
- accepted Server dependency files listed above
- relevant current deployment files and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement DEP-P3: Server packaging and service wiring.

Follow the deployment plan:

- decide binary versus container packaging path inside deploy scope;
- define server environment variables and volumes using documented Server config only;
- wire PostgreSQL connection and object-store path through placeholders or safe local defaults;
- expose local/proxy-bound listen address safely;
- add readiness/liveness checks using accepted Server readiness/health behavior;
- document startup and shutdown;
- avoid auto-running migrations until migration policy is accepted.

## Allowed files

- deploy/**
- Dockerfile or server packaging files only if already accepted by deployment scope
- docs/runbooks only if introduced under deployment scope
- deploy/control/report.md

## Non-goals

- No Server code changes.
- No GDrive adapter service.
- No Worktree runtime unless Server/Worktree fan-in is ready.
- No production TLS/private keys.
- No real credentials.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and implementation-worker-prompt.md. Product, deployment, docs, script, contract, workflow, and implementation commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to deploy/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
