# W1-CI-P2 — GitHub CI trigger and branch policy normalization

Component: github-ci
Component path: .github
Branch: component/github-ci
Base branch: main
Current main baseline SHA: 9ee3ced989bf60a71d0d7b37ff046118b0b2d1a2
Target branch: main

## Role

You are an Implementation Worker for NordCoder/haze-sync.

Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read before editing

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- docs-process/docs/development-model.md
- .github/docs/component-contract.md
- .github/docs/implementation-plan.md
- .github/docs/implementation-log.md
- .github/docs/dependency-map.md
- .github/docs/decisions.md
- .github/control/prompt.md
- current .github/workflows/**

## Task

Implement phase CI-P2 from the GitHub CI implementation plan: Workflow trigger and branch policy normalization.

Goal: align workflow triggers with current branch/process policy while preserving safe PR validation and avoiding unexpected deployment behavior.

## Allowed component scope

```text
.github/workflows/**
.github/docs/**
.github/control/report.md
```

## Expected work

- Review `ci.yml` push branches and remove legacy branch names if appropriate.
- Decide whether component branches should run full CI, Component CI, or PR-only checks according to docs-process policy.
- Document trigger matrix for `main`, `process/**`, `component/**`, PRs, and manual dispatch.
- Preserve concurrency cancellation where useful.
- Keep permissions minimal.

## Explicit non-goals

- No production deployment.
- No live provider tests.
- No secret use.
- No product code changes outside `.github/**`.
- No disabling PR validation.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires changing required status checks, expanding triggers to secret-using deployment jobs, disabling PR validation, or making component branches unvalidated without a process decision.

## Checks

Run relevant checks if available. At minimum, inspect workflow YAML carefully. If shell access is available and repo tooling supports it, run normal repo checks after workflow edits. If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
.github/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
