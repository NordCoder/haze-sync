# T0-P3 — Server component docs plus tiny route cleanup

Component: server
Component path: crates/haze-sync-server
Branch: component/server
Base branch: main
Verified base SHA: aabf74486d4d06a89136007bd17713f3c35478de
Target branch: main

## Role

You are an Implementation Worker. Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/docs/decisions.md
- crates/haze-sync-server/control/README.md
- crates/haze-sync-server/control/state.md
- current crates/haze-sync-server source code
- relevant API/Core/Storage code only as dependency context

## Goal

Replace generic Server scaffold docs with useful current-state component documentation. Additionally perform one tiny safe server-local cleanup only if it is obvious. This is a process test, not a route refactor.

## Allowed files

- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/docs/decisions.md
- crates/haze-sync-server/control/state.md
- crates/haze-sync-server/control/report.md
- crates/haze-sync-server/src/**/*.rs only for one tiny behavior-preserving cleanup
- crates/haze-sync-server/tests/**/*.rs if present and only for one tiny behavior-preserving cleanup

Do not change files outside crates/haze-sync-server.

## Required output

Update Server docs to describe current post-W3 responsibilities: Axum runtime wiring, auth execution, route composition, runtime state, transaction boundaries, dependencies, non-goals, safety/secrecy rules, test obligations, risks, and deferred work.

The contract must state that Server owns runtime wiring and safe internal-to-public error mapping, may call API/Core/Storage/Common, must not leak raw internal errors/secrets, must not implement provider runtime behavior, and must not introduce hard-delete behavior unless explicitly scoped.

Add one T0-P3 entry to implementation-log.md. Add one decision explaining this is docs plus tiny-cleanup process testing, not broad route refactor. Update control/state.md to PROMPT_READY or REPORT_READY as appropriate.

Optional cleanup: make at most one tiny behavior-preserving server-local cleanup if obvious, such as a private test helper rename, repeated assertion extraction, or comment/doc-comment improvement. Do not rewrite routes/mod.rs or routes/v1.rs. If no safe cleanup is obvious, do none.

Write the final report to crates/haze-sync-server/control/report.md using report-template.md.

## Checks

Run or honestly mark not run:

- cargo fmt --check
- cargo check -p haze-sync-server
- cargo test -p haze-sync-server

Expected final status: SELF_ACCEPT or SELF_ACCEPT_PENDING_CI.
