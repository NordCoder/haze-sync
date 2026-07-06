# W1-SRV-P2 — Server router, state, auth, and safe error audit

Component: server
Component path: crates/haze-sync-server
Branch: component/server
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
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/docs/decisions.md
- crates/haze-sync-server/control/prompt.md
- relevant current code under crates/haze-sync-server/src/**

## Task

Implement phase SRV-P2 from the Server implementation plan: Router, state, auth, and safe error audit.

Goal:

```text
Audit current Server runtime shell and route behavior against component contracts, then harden tests/docs around passive-safe dependency-free behavior and public error sanitization.
```

## Allowed component scope

```text
crates/haze-sync-server/src/routes/**
crates/haze-sync-server/src/state.rs
crates/haze-sync-server/src/http/**
crates/haze-sync-server/src/config/**
crates/haze-sync-server/src/readiness/**
crates/haze-sync-server/docs/**
crates/haze-sync-server/control/report.md
```

## Expected work

- Verify router construction remains explicit and free of hidden globals.
- Verify dependency-free router routes fail safely without mutation.
- Test auth states: disabled, static principal, database lookup failure, and role checks where current coverage is missing.
- Audit debug/display behavior for redaction.
- Test public error bodies for absence of tokens, token hashes, idempotency keys, DB URLs, object-store roots, local paths, stack traces, raw SQLx errors, and request bodies.
- Document any route that is intentionally placeholder or partial.

## Explicit non-goals

- No production listener.
- No provider runtime.
- No broad route decomposition unless required for SRV-P2 clarity.
- No Core policy changes.
- No Storage schema changes.
- No sibling component edits.

## Contract-change triggers

Report BLOCKED_BY_CONTRACT or request a contract change if this work requires hidden global runtime state, changing API public error shapes inside Server, token creation/rotation in Server routes, or exposing raw runtime internals.

## Checks

Run applicable checks if available:

```text
cargo fmt --check
cargo check -p haze-sync-server
cargo test -p haze-sync-server
cargo clippy -p haze-sync-server --all-targets -- -D warnings
```

If working through GitHub connector only and shell checks cannot run, report that honestly.

## Report

Write the final report to:

```text
crates/haze-sync-server/control/report.md
```

Use report-template.md. Set REPORT_TYPE to IMPLEMENTATION.

Expected final status: SELF_ACCEPT_PENDING_CI, SELF_NEEDS_FIX, or BLOCKED_BY_CONTRACT.
