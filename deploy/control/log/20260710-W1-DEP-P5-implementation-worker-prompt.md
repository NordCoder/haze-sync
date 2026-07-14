# Archived active prompt

archived_by: orchestrator
archived_on: 2026-07-10
source_path: deploy/control/prompt.md
source_phase: DEP-P5
source_role: implementation-worker
source_blob_sha: d3552896db333c54b4606711ea673c2676858396

---

# W1-DEP-P5 — Object-store, worktree, and host directory provisioning

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

DEP-P4 implementation and clean-code review are accepted. Component CI evidence for the accepted docs state is green.

- workflow: Component CI
- workflow_run_id: 29034814421
- run_number: 690
- conclusion: success

The next implementation phase is DEP-P5 from deploy/docs/implementation-plan.md.

## Read

Read implementation-manifest.md, report-template.md, implementation-worker-prompt.md, chatgpt-gh-connector.md, deployment docs/control files, relevant deploy files, and PR diff.

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement DEP-P5: Object-store, worktree, and host directory provisioning.

Follow the plan:

- document expected directories for object store, worktree, secrets, logs, backups, and runtime temp paths using placeholders/examples;
- define ownership and permission expectations;
- distinguish data, config, secret, log, backup, and runtime temp paths;
- document what must be backed up and what must never be committed;
- coordinate Worktree root and object-store root with accepted Server config.

## Allowed files

- deploy/docs/**
- deploy/scripts/** only if explicitly accepted and safe
- .env.example if placeholders need alignment
- deploy/control/report.md

## Non-goals

No actual host mutation through GitHub connector, Worktree runtime behavior, object-store cleanup job, real secret files, workflow changes, or sibling component changes.

## CI trigger policy

Deployment/docs/scripts commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to deploy/control/report.md. Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION and phase_id to DEP-P5.
