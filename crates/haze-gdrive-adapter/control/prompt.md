# W1-GDA-P3 — Google client abstraction and provider-safe DTO normalization

Component: gdrive-adapter
Path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
PR: #50
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

GDA-P2 and its CI fixer loop are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006846543
- run_number: 451
- conclusion: success

The next implementation phase is GDA-P3 from crates/haze-gdrive-adapter/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- crates/haze-gdrive-adapter/docs/component-contract.md
- crates/haze-gdrive-adapter/docs/implementation-plan.md
- crates/haze-gdrive-adapter/docs/implementation-log.md
- crates/haze-gdrive-adapter/docs/dependency-map.md
- crates/haze-gdrive-adapter/control/prompt.md
- crates/haze-gdrive-adapter/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement GDA-P3: Google client abstraction and provider-safe DTO normalization.

Follow the implementation plan:

- define a testable Drive provider trait/interface;
- implement fake provider support for tests;
- map Drive metadata into provider-neutral adapter facts;
- classify supported and unsupported V1 file types explicitly;
- sanitize provider errors and raw payloads;
- add tests for metadata normalization and unsupported entry classification.

## Allowed files

- crates/haze-gdrive-adapter/src/**
- crates/haze-gdrive-adapter/docs/**
- crates/haze-gdrive-adapter/control/report.md

## Non-goals

- No real Google SDK wiring unless explicitly justified within this phase and still fake-first.
- No Core API writes.
- No mapping DB persistence.
- No provider delete actions.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to crates/haze-gdrive-adapter/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
