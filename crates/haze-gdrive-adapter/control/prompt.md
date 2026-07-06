# W1-GDA-P2 — GDrive config, secret loading, and runtime skeleton

Component: gdrive-adapter
Component path: crates/haze-gdrive-adapter
Branch: component/gdrive-adapter
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
- crates/haze-gdrive-adapter/docs/component-contract.md
- crates/haze-gdrive-adapter/docs/implementation-plan.md
- crates/haze-gdrive-adapter/docs/implementation-log.md
- crates/haze-gdrive-adapter/docs/dependency-map.md
- crates/haze-gdrive-adapter/docs/decisions.md
- crates/haze-gdrive-adapter/control/prompt.md
- relevant current code under crates/haze-gdrive-adapter/src/**

## Task

Implement phase GDA-P2 from the GDrive adapter implementation plan: Config, secret loading, and runtime skeleton.

Goal:

```text
Implement explicit adapter configuration, secret-path handling, safe redaction, and process lifecycle foundation before provider calls are added.
```

## Allowed component scope

```text
crates/haze-gdrive-adapter/src/**
crates/haze-gdrive-adapter/docs/**
crates/haze-gdrive-adapter/control/report.md
```

## Expected work

- Define config for server URL, adapter token, Drive root folder id, OAuth token path, adapter mode, dry-run, polling/full-scan intervals, and delete safety thresholds as appropriate for this phase.
- Load config from explicit environment/files without committing secrets.
- Represent secret values with redacted debug/display behavior.
- Add skeleton lifecycle with startup validation and graceful shutdown hooks if feasible without provider calls.
- Add safe status/error classification for config failures.

## Explicit non-goals

- No Google API calls.
- No Core API calls.
- No mapping persistence.
- No sync loop.
- No token refresh implementation beyond safe loading if explicitly scoped.
- No direct DB dependency.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires committing tokens/sample credentials, logging secret paths or values unsafely, changing deployment secret layout without deployment docs, or adding hidden direct database access.

## Checks

Run applicable checks if available:

```text
cargo fmt --check
cargo check -p haze-gdrive-adapter
cargo test -p haze-gdrive-adapter
cargo clippy -p haze-gdrive-adapter --all-targets -- -D warnings
```

If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
crates/haze-gdrive-adapter/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
