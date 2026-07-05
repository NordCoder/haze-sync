# T0-P2 — API component contract audit

Component: api
Component path: crates/haze-sync-api
Branch: component/api
Base branch: main
Verified base SHA: aabf74486d4d06a89136007bd17713f3c35478de
Target branch: main

## Role

You are an Implementation Worker. Work only through the GitHub connector. Do not use SSH/local git. Do not open PR. Do not merge.

## Read

- implementation-manifest.md from ChatGPT Project Sources
- report-template.md from ChatGPT Project Sources
- implementation-worker-prompt.md from ChatGPT Project Sources
- crates/haze-sync-api/docs/component-contract.md
- crates/haze-sync-api/docs/implementation-plan.md
- crates/haze-sync-api/docs/implementation-log.md
- crates/haze-sync-api/docs/dependency-map.md
- crates/haze-sync-api/docs/decisions.md
- crates/haze-sync-api/control/README.md
- crates/haze-sync-api/control/state.md
- current crates/haze-sync-api source code
- crates/haze-sync-common source only as dependency context
- relevant server route code only to understand API helper consumers

## Goal

Replace generic API scaffold docs with useful current-state component documentation. This is a process test, not a feature task.

## Allowed files

- crates/haze-sync-api/docs/component-contract.md
- crates/haze-sync-api/docs/implementation-plan.md
- crates/haze-sync-api/docs/implementation-log.md
- crates/haze-sync-api/docs/dependency-map.md
- crates/haze-sync-api/docs/decisions.md
- crates/haze-sync-api/control/state.md
- crates/haze-sync-api/control/report.md

Do not change API source unless it is a tiny comment/doc typo fix. Do not change files outside crates/haze-sync-api.

## Required output

Update API docs to describe current post-W3 responsibilities: DTOs, route-contract helpers, auth/header contracts, safe public errors, non-goals, dependencies, safety/secrecy rules, test obligations, risks, and deferred work.

The contract must state that API is passive: it does not own Axum runtime wiring, DB queries, provider calls, or broad Core execution. Server owns runtime wiring.

Add one T0-P2 entry to implementation-log.md. Add one decision explaining that API remains passive and server owns runtime route wiring. Update control/state.md to PROMPT_READY or REPORT_READY as appropriate.

Write the final report to crates/haze-sync-api/control/report.md using report-template.md.

## Checks

Run or honestly mark not run:

- cargo fmt --check
- cargo check -p haze-sync-api
- cargo test -p haze-sync-api

Expected final status: SELF_ACCEPT or SELF_ACCEPT_PENDING_CI.