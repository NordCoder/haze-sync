# W1-SRV-GDA-P1-STATE-ROUTES-AND-TRANSACTIONS

Before starting, name this worker chat exactly:

`server — W1 SRV-GDA-P1 State Routes and Transactions`

Repository: `NordCoder/haze-sync`
Component: server
Path: `crates/haze-sync-server`
Branch/ref: `component/server`
PR: #45
Role: implementation-worker
Phase: `SRV-GDA-P1-STATE-ROUTES-AND-TRANSACTIONS`

This is the only active Server phase.

## Accepted inputs

- accepted Server baseline SHA: `50461354c18ddc4d2e47202d9303b4358a27ee45`;
- Server baseline clean-review blob: `e3271abaf3d667f9ffd4f4ff0652e5d26892b9e5`;
- accepted API GDrive contract SHA: `c60c3976696da1970d539e5cff6e9f74a61fc10e`;
- API clean-review blob: `55ff6047c9c6c0f6f548f10197b76706c0a244e1`;
- accepted Storage GDrive SHA: `3617bd1cf947fdd394f1ab29d4b992f7b8859a84`;
- Storage clean-review blob: `4584b8705221d3cd2aa43b5776674b3a1ec9a0f4`;
- GDrive fan-in architecture report blob: `14c427880e1201d851cdc9ee04b9cd0e83334de4`.

Fetch the actual branch head before editing. Do not assume the accepted product SHA is the current control-only branch head.

## Goal

Implement the Server-owned authenticated HTTP/application/transaction boundary for the accepted passive API GDrive state contracts and accepted Storage durable-state repositories.

The Server owns authentication, adapter lookup, authorization, request-to-storage translation, caller-owned PostgreSQL transactions, commit/rollback, safe outcome mapping and route registration. Storage owns persistence and invariants. API owns DTO/header/error vocabulary. The GDrive Adapter remains database-independent.

## Required deliverables

1. Register and implement the accepted routes:
   - `GET /v1/adapters/{adapter_id}/gdrive/state`;
   - `POST /v1/adapters/{adapter_id}/gdrive/state/commit`.
2. Enforce accepted authorization:
   - matching GDrive adapter may read private state and submit commits;
   - admin may read only the sanitized summary;
   - admin and unrelated adapters cannot submit commits;
   - unauthorized, forbidden and adapter-not-found cases use the accepted safe API vocabulary.
3. Resolve adapter identity/type before Storage access and fail closed on mismatch.
4. For state reads:
   - open a caller-owned read transaction;
   - obtain one bounded consistent adapter snapshot through accepted Storage repositories;
   - map private or sanitized response according to the authorized role;
   - commit/close cleanly with no raw DB/provider errors exposed.
5. For compare-and-commit:
   - validate accepted API request and idempotency metadata before mutation;
   - begin one caller-owned PostgreSQL transaction;
   - execute accepted Storage compare-and-commit atomically;
   - commit only on committed/replayed accepted outcomes;
   - rollback on validation, stale state, cursor gap/regression, mapping conflict, idempotency conflict or internal failure;
   - map typed Storage outcomes to the exact accepted API outcome/error vocabulary.
6. Preserve Storage invariants:
   - exact expected state-version CAS;
   - exact cursor generation transition;
   - non-regressing Core export checkpoint;
   - atomic mapping/echo/delete-candidate/operation facts;
   - deterministic replay/conflict behavior;
   - adapter isolation and bounded snapshots.
7. Add focused unit/application tests and mandatory real PostgreSQL integration tests for:
   - private and admin reads;
   - unauthorized/forbidden/type mismatch;
   - successful commit and replay;
   - stale state and concurrent writer loser;
   - cursor gap/regression;
   - mapping/idempotency conflicts;
   - rollback with no partial facts;
   - adapter isolation;
   - safe error/redaction boundaries.
8. Update Server docs/implementation log minimally.

## Allowed scope

- focused Server route/application/auth/state modules under `crates/haze-sync-server/src/**`;
- Server tests and test support;
- minimum Server dependency wiring for accepted API and Storage modules;
- Server docs and control report.

## Forbidden

- API or Storage contract/schema changes;
- direct GDrive Adapter database access;
- Google/OAuth/provider calls;
- Core sync policy changes;
- scheduler/background polling;
- GDrive status ingestion/operator-control contracts;
- CLI or Deployment work;
- sibling component or workflow changes;
- raw cursor, provider facts, idempotency values, SQL/database errors or request bodies in logs/errors;
- test weakening;
- merge, rebase, force-push or PR draft-state changes.

If accepted API and Storage contracts cannot be connected without changing either owner contract, report `BLOCKED_BY_CONTRACT` with the exact mismatch. Do not edit sibling owners.

## Validation

Create product/test changes without CI skip. Obtain full exact-SHA Component CI with:
- cargo fmt/check/test/clippy green;
- mandatory DB-capable Server/PostgreSQL verification green;
- diagnostics finalization green;
- PR #45 open, draft and unmerged.

## Report

Write only `crates/haze-sync-server/control/report.md` with:
- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: SRV-GDA-P1-STATE-ROUTES-AND-TRANSACTIONS`;
- `chat_name: server — W1 SRV-GDA-P1 State Routes and Transactions`;
- status `SELF_ACCEPT`, `SELF_ACCEPT_PENDING_CI`, `SELF_NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_DEPENDENCY`, or `BLOCKED_BY_TOOLING`.

Record exact routes, authorization, transaction ownership, outcome mapping, DB tests, final code-bearing SHA and exact CI evidence. Do not claim CLEAN_ACCEPT or merge readiness.
