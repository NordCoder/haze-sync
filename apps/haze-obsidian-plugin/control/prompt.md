# W1-OBS-P4 — Vault path mapping, scan, and pending queue foundation

Component: obsidian-plugin
Path: apps/haze-obsidian-plugin
Branch: component/obsidian-plugin
PR: #51
Role: implementation-worker

Work only through the GitHub connector. Do not use SSH. Do not use local git. Do not open PR. Do not merge. Do not mark PRs ready for review. Do not decide merge readiness.

## Context

OBS-P3 implementation and clean-code review are complete. Current Component CI is green for the current PR head.

- workflow: Component CI
- workflow_run_id: 29006856129
- run_number: 452
- conclusion: success

The next implementation phase is OBS-P4 from apps/haze-obsidian-plugin/docs/implementation-plan.md.

## Read

Read before editing:

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- chatgpt-gh-connector.md from ChatGPT Project Sources
- apps/haze-obsidian-plugin/docs/component-contract.md
- apps/haze-obsidian-plugin/docs/implementation-plan.md
- apps/haze-obsidian-plugin/docs/implementation-log.md
- apps/haze-obsidian-plugin/docs/dependency-map.md
- apps/haze-obsidian-plugin/control/prompt.md
- apps/haze-obsidian-plugin/control/report.md
- relevant current repository code and PR diff

Do not read CI diagnostics artifacts unless a future active prompt explicitly instructs it.

## Task

Implement OBS-P4: Vault path mapping, scan, and pending queue foundation.

Follow the implementation plan:

- map Obsidian TFile paths to API-compatible vault paths;
- exclude plugin metadata, internal directories, temporary files, and unsupported resources;
- implement an explicit scan/reconcile entrypoint;
- capture safe local file facts: path, mtime or revision marker where available, hash, size;
- maintain pending queue state for new, modified, and deleted files;
- distinguish event hints from full scans;
- make queue state safe to inspect through status/UI helpers where scoped.

## Allowed files

- apps/haze-obsidian-plugin/src/**
- apps/haze-obsidian-plugin/docs/**
- apps/haze-obsidian-plugin/control/report.md

## Non-goals

- No upload/download execution.
- No server writes.
- No conflict resolution.
- No direct filesystem APIs outside Obsidian abstractions unless justified.
- No Google Drive behavior.
- No sibling component changes.
- No workflow changes.

## CI trigger policy

Follow the CI skip policy in implementation-manifest.md and the relevant Project Source worker prompt. Product, test, dependency, contract, workflow, and implementation commits must not skip CI.

## Report

Write only the report to apps/haze-obsidian-plugin/control/report.md.

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.
