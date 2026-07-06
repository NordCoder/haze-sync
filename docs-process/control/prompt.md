# W1-DOC-P8 — Docs-process system docs index alignment

Component: docs-process
Component path: docs-process
Branch: component/docs-process
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
- docs-process/docs/component-contract.md
- docs-process/docs/implementation-plan.md
- docs-process/docs/implementation-log.md
- docs-process/docs/dependency-map.md
- docs-process/docs/decisions.md
- docs-process/control/prompt.md
- README.md and docs/** if they exist

## Task

Implement phase DOC-P8 from the docs-process implementation plan: System docs index alignment.

Goal:

```text
Align root/system docs with the canonical process model without mixing process docs and product architecture docs.
```

## Allowed component scope

```text
docs-process/docs/**
docs/** only if explicitly needed for index alignment
README.md only if explicitly needed for links/index alignment
docs-process/control/report.md
```

## Expected work

- Ensure process docs and product/system docs are clearly separated.
- Ensure root/system docs point to `docs-process/**` only where that is useful and accurate.
- Avoid implying unmerged component branch docs already exist on `main` unless they do.
- Update docs-process docs if index/ownership wording is stale after process baseline fan-in.
- Keep all changes behavior-neutral and repository-relative.

## Explicit non-goals

- No product architecture change.
- No component contract rewrite.
- No local archive ingestion.
- No prompt/report template ingestion into public docs.
- No product code, workflow, deployment, or runtime behavior changes.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires changing component ownership, merging component docs into main, changing workflow policy, or exposing private project-source content.

## Checks

Run applicable docs checks if available. If no docs checker exists, perform manual link/path review and report it honestly. If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
docs-process/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
