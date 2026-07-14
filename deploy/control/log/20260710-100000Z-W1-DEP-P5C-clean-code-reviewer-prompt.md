# Archived active prompt

component: deployment
archived_at: 2026-07-10T10:00:00Z
wave: W1
phase: DEP-P5C
agent_role: clean-code-reviewer
source_path: deploy/control/prompt.md
source_sha: a15c2697a2e5021263eff2738cf2d8213d6e456f

# W1-DEP-P5C — Deployment host-directory provisioning clean-code review

Component: deployment
Path: deploy
Branch: component/deployment
PR: #52
Role: clean-code-reviewer

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

DEP-P5 implementation is complete and its code-bearing Component CI run is green.

- code_bearing_sha: d0b83d80348e8e91166525904aeceaebab2438c1
- workflow: Component CI
- workflow_run_id: 29067710565
- run_number: 846
- conclusion: success

## Read

Read implementation-manifest.md, report-template.md, clean-code-reviewer-prompt.md, chatgpt-gh-connector.md, deployment docs/control files, relevant deploy files, and PR diff. Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Review DEP-P5 host-directory provisioning documentation and placeholder alignment.

Focus areas:

- path classes and separation invariants;
- ownership and least-privilege permission guidance;
- accepted Server object-store/worktree config keys;
- local defaults versus production-style placeholders;
- backup classification and consistency guidance;
- never-commit rules and secret safety;
- container UID 10001 considerations;
- honest deferral of host mutation, bind mounts, Worktree runtime, cleanup, and real secret provisioning.

## Allowed files

- deploy/docs/**
- deploy/scripts/** only if already present and directly relevant
- .env.example
- deploy/control/report.md

## Forbidden changes

No actual host mutation, Compose bind mounts, Worktree runtime enablement, object-store cleanup, real secrets, production credentials, workflow changes, remote automation, or sibling component changes.

## CI trigger policy

Source/doc clean-code commits must not skip CI. A final report-only commit may skip CI.

## Report

Write only the report to deploy/control/report.md. Use report-template.md. Set REPORT_TYPE to CLEAN_CODE_REVIEW and phase_id to DEP-P5C.
