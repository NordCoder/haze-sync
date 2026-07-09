# W1-DEP-P3C — Deployment server packaging clean-code review

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

DEP-P3 implementation completed with SELF_ACCEPT_PENDING_CI.

The implementation report says Deployment added server container packaging and local service wiring inside deployment scope:

- deploy/server.Dockerfile
- deploy/docker-compose.yml server service
- local-only PostgreSQL and HTTP bindings
- documented Server config variables and volumes
- healthcheck using accepted Server /health behavior
- local compose/runbook documentation

Orchestrator has not yet confirmed a completed green CI run for the DEP-P3 code-bearing commit. Treat CI as pending or unknown unless you observe a completed green run through GitHub connector metadata.

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
- relevant current deployment files and PR diff
- accepted Server dependency evidence referenced by the DEP-P3 implementation report, read-only only

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review DEP-P3 server packaging and local service wiring.

Focus areas:

- deploy/server.Dockerfile safety, reproducibility, and non-root execution;
- docker-compose server wiring and local-only bind behavior;
- documented Server environment variables and volumes;
- PostgreSQL and object-store volume behavior;
- /health versus /ready documentation correctness;
- migration policy explicitly deferred, no auto-running migrations;
- no secrets, real credentials, TLS keys, provider services, Worktree runtime, or sibling changes.

## Allowed files

- deploy/**
- deploy/control/report.md

## Forbidden changes

- Do not modify Server code.
- Do not modify GDrive, Worktree, Obsidian, Core, API, Storage, CLI, or Common code.
- Do not add real credentials.
- Do not add production TLS/private keys.
- Do not add provider services.
- Do not enable Worktree runtime unless a future explicit fan-in scopes it.
- Do not change workflow files.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and clean-code-reviewer-prompt.md. Deployment/source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to deploy/control/report.md.

Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW.

If no source/docs changes are made and CI is still not observed green, use CLEAN_ACCEPT_PENDING_CI rather than CLEAN_ACCEPT.
