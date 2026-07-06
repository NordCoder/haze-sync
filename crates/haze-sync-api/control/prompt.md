# W1-API-P2 — API public DTO and serialization audit

Component: api
Component path: crates/haze-sync-api
Branch: component/api
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
- crates/haze-sync-api/docs/component-contract.md
- crates/haze-sync-api/docs/implementation-plan.md
- crates/haze-sync-api/docs/implementation-log.md
- crates/haze-sync-api/docs/dependency-map.md
- crates/haze-sync-api/docs/decisions.md
- crates/haze-sync-api/control/prompt.md
- relevant current code under crates/haze-sync-api/src/**

## Task

Implement phase API-P2 from the API implementation plan: Public DTO and serialization audit.

Goal:

```text
Audit all public DTO modules for stable JSON vocabulary, safe output, and clear ownership boundaries before adapters/plugin rely on them.
```

## Allowed component scope

```text
crates/haze-sync-api/src/dto/**
crates/haze-sync-api/src/contracts/errors/**
crates/haze-sync-api/docs/**
crates/haze-sync-api/control/report.md
```

## Expected work

- Verify stable `snake_case` values for shared enums and status/action vocabularies.
- Add serde roundtrip tests for server-info, changes, file metadata, upload outcomes, delete outcomes, conflict list/resolve, and admin/status DTOs where current coverage is missing.
- Verify raw file bytes are absent from JSON DTOs.
- Verify raw cursors, token hashes, bearer tokens, idempotency keys, database URLs, local paths, stack traces, provider payloads, and raw request bodies are absent from public DTOs/errors.
- Document any DTO that is intentionally partial or placeholder.

## Explicit non-goals

- No route wiring.
- No storage or Core calls.
- No client TypeScript edits.
- No provider-specific DTOs unless the public API contract requires them.
- No Axum, SQLx, provider SDK, filesystem watcher, or runtime service dependency.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires changing public field names/status values, adding provider-specific payloads, exposing raw secret-like values, or moving Core semantic decisions into DTO constructors.

## Checks

Run applicable checks if available:

```text
cargo fmt --check
cargo check -p haze-sync-api
cargo test -p haze-sync-api
cargo clippy -p haze-sync-api --all-targets -- -D warnings
```

If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
crates/haze-sync-api/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
