# W1-API-GDA-P1-CONTRACTS

Before starting, name this worker chat exactly:

`api — W1 API-GDA-P1 GDrive Contracts`

Repository: `NordCoder/haze-sync`
Component: api
Path: `crates/haze-sync-api`
Branch/ref: `component/api`
PR: #44
Role: implementation-worker
Phase: `API-GDA-P1-CONTRACTS`

This is the only active API phase.

## Accepted inputs

- accepted API baseline SHA: `56ae94570441d68715f34b5d54381a0fc4d7c231`;
- accepted Storage GDrive durable-state SHA: `3617bd1cf947fdd394f1ab29d4b992f7b8859a84`;
- Storage clean-review blob: `4584b8705221d3cd2aa43b5776674b3a1ec9a0f4`;
- GDrive architecture report blob: `14c427880e1201d851cdc9ee04b9cd0e83334de4`;
- exact main ancestor: `c1e69a664388b0cba028170e8398b9088218957d`.

Fetch the actual branch head before editing.

## Fixed public contract

Add passive authenticated API contracts for:

1. `GET /v1/adapters/{adapter_id}/gdrive/state`
   - bounded adapter-scoped durable-state snapshot;
   - state version, cursor generation/presence, Core export checkpoint, mapping/echo/delete-candidate/operation summaries required for restart;
   - no OAuth values, token hashes, database errors or unrestricted provider payloads.

2. `POST /v1/adapters/{adapter_id}/gdrive/state/commit`
   - expected state version and expected cursor generation;
   - exactly one cursor generation transition where supplied;
   - non-regressing Core checkpoint;
   - typed mapping, echo, delete-candidate and operation facts;
   - mandatory idempotency metadata;
   - safe outcomes for stale state, cursor regression/gap, mapping conflict, idempotency conflict and validation failure.

Keep API passive: DTOs, parsing, validation, serde and route metadata only. Server owns auth lookup, transactions and execution. Storage owns persistence. Adapter never receives DB access.

## Required deliverables

- typed request/response DTOs with bounded collections;
- adapter identity and role-compatible route helpers;
- stable public error vocabulary mapped from the accepted architecture;
- raw cursor allowed only in the authenticated private commit contract where correctness requires it, always redacted from Debug/Display/errors/fixtures;
- deterministic serde and validation tests;
- compatibility fixtures for later Server and GDrive client work;
- minimal API docs/log alignment.

## Allowed scope

- `crates/haze-sync-api/src/dto/**`;
- `crates/haze-sync-api/src/routes/**`;
- `crates/haze-sync-api/src/contracts/**` only for focused safe error vocabulary;
- API fixtures/tests/docs/control report.

## Forbidden

- Axum route registration or Server behavior;
- Storage/SQLx calls;
- Core policy;
- GDrive provider/OAuth/scheduler behavior;
- status ingestion/operator controls beyond these first state contracts;
- sibling component or workflow changes;
- raw secrets/provider bodies/database internals;
- merge, draft-state change, rebase or force-push.

If the accepted Storage model cannot be represented safely without a contract decision, report `BLOCKED_BY_CONTRACT`; do not redesign Storage.

## Validation

Create product/test commits without CI skip. Required exact-SHA Component CI: fmt, check, test and clippy green. PR #44 remains open, draft and unmerged.

## Report

Write only `crates/haze-sync-api/control/report.md` with:

- `REPORT_TYPE: IMPLEMENTATION`;
- `phase_id: API-GDA-P1-CONTRACTS`;
- `chat_name: api — W1 API-GDA-P1 GDrive Contracts`;
- status `SELF_ACCEPT`, `SELF_ACCEPT_PENDING_CI`, `SELF_NEEDS_FIX`, `BLOCKED_BY_CONTRACT`, `BLOCKED_BY_DEPENDENCY`, or `BLOCKED_BY_TOOLING`.

Record exact DTO/route/error/fixture changes, secrecy rules, final code-bearing SHA and exact CI. Do not claim CLEAN_ACCEPT or merge readiness.
