# W1-GDA-GDA-P4-CURSOR-READ-CONTRACT-ARCHITECTURE-REVIEW

## Routing envelope

- protocol_version: `3`
- repository: `NordCoder/haze-sync`
- component: `gdrive-adapter`
- component_path: `crates/haze-gdrive-adapter`
- role: `architect`
- agent_execution_id: `gdrive-adapter-GDA-GDA-P4-cursor-contract-arch-20260717113213-a4c27d91`
- branch: `component/gdrive-adapter`
- pull_request: `#50`
- wave: `W1`
- phase: `GDA-GDA-P4-CURSOR-READ-CONTRACT-ARCHITECTURE-REVIEW`
- chat_key: `gdrive-adapter`
- control_prompt_path: `crates/haze-gdrive-adapter/control/prompt.md`
- control_report_path: `crates/haze-gdrive-adapter/control/report.md`
- expected_report_type: `ARCHITECT_REVIEW`

Use the component's one dedicated `gdrive-adapter` chat. For this execution only, act as Architect. Do not derive current state from chat history and do not create role-specific chat routing.

## Mandatory source order

Read and apply, in order:

1. the current project implementation manifest and report template;
2. `crates/haze-gdrive-adapter/docs/component-contract.md`;
3. `crates/haze-gdrive-adapter/docs/implementation-plan.md`;
4. `crates/haze-gdrive-adapter/docs/implementation-log.md`;
5. `crates/haze-gdrive-adapter/docs/dependency-map.md`;
6. `crates/haze-gdrive-adapter/docs/decisions.md`;
7. accepted fan-in architecture report blob `14c427880e1201d851cdc9ee04b9cd0e83334de4`;
8. blocked runtime report blob `c12872192f1be968ab77bf44adfeb37c5228b2c0` from commit `42961e78a547ae003dc3310d82c3f8505d316d2f`;
9. accepted API SHA `c60c3976696da1970d539e5cff6e9f74a61fc10e`, especially:
   - `crates/haze-sync-api/src/dto/gdrive.rs` blob `8098457eee373f63210fbd381c6487a054d7609f`;
   - `crates/haze-sync-api/src/routes/gdrive.rs` blob `ef7f8dc1d02b3742e76bb99c66ee39703bee3b4b`;
10. accepted Server SHA `c023b83e1e6f502e7d2261acccb871dd5588edf1`, especially `crates/haze-sync-server/src/routes/gdrive.rs` blob `757bea6adf9939a87e3ae14695afefd2e9df94eb`;
11. accepted Storage SHA `3617bd1cf947fdd394f1ab29d4b992f7b8859a84`, especially state-types blob `1f0d5ac29bbe52d5fc7579b9a1120179363cdfb6`;
12. current GDrive change-feed model blob `27e7775e3d4401d0222cbe3f442245be4cb449a2` and current HTTP durable-state client;
13. this prompt;
14. current component branches, PR metadata, active control state and relevant CI evidence.

Old control files are read-only evidence. Do not archive prompt/report files and do not implement product code.

## Established facts

The accepted architecture remains fixed unless the evidence proves it impossible:

- GDrive is one standalone long-running process per configured adapter identity/root;
- API owns transport DTOs and route contracts;
- Server owns authenticated route behavior and transaction choreography;
- Storage owns durable state and already persists `drive_cursor: Option<String>`;
- GDrive accesses durable state only through authenticated Server/API HTTP;
- direct database access by GDrive is forbidden;
- full scan is the correctness backstop, but it must not silently replace an unresolved durable cursor interval;
- raw cursor values are private provider facts and must never appear in admin output, status, logs, errors, reports or public diagnostics.

The blocker is exact:

- the private commit contract can durably write the bounded opaque cursor;
- the private GET snapshot returns only `generation` and `present`;
- Server reads the stored cursor but discards its value while constructing `GDriveStateSnapshotResponse`;
- the restarted adapter requires the actual cursor token to continue Google Drive change polling without weakening crash-safety semantics;
- the admin path already returns a separate `GDriveStateAdminSummaryResponse`.

## Objective

Decide and specify the smallest safe cross-component contract correction that allows the matching authenticated GDrive adapter to read its committed opaque cursor after restart while preserving admin sanitization, bounded validation, strict decoding, redaction and existing ownership boundaries.

This is an architecture and transition-planning gate. Do not write product code, tests, component docs, manifests, lockfiles or workflows. Write only the architecture report.

## Required decisions

### 1. Exact API shape

Choose one exact design and reject the alternatives explicitly:

- extend the existing adapter-private snapshot response with a cursor-bearing private DTO using the existing `GDriveRawCursorDto`;
- replace the private response's summary cursor type with a type-level private cursor state while retaining the separate admin summary DTO;
- add a dedicated matching-principal private cursor-read route only if changing the existing private snapshot is materially less safe or less compatible.

Do not merely say “return the cursor”. Define exact Rust DTO names/fields, optionality, serde behavior, bounds and validation invariants. Prefer a design that makes impossible states difficult to represent and does not put the raw value into `GDriveCursorSummaryDto` used by admin output.

Address compatibility with strict `deny_unknown_fields` clients and the required fan-in/deployment ordering. Do not assume an already deployed old client can ignore a new field.

### 2. Authorization and wire separation

Specify exact behavior for:

- matching authenticated `gdrive_adapter` principal;
- non-matching adapter principal;
- authenticated admin;
- unauthenticated caller.

The admin wire response must remain provider-identifier-free and cursor-value-free. Decide whether the existing route can safely return different response DTOs by access class, as it already does, or whether a new route is required.

### 3. Cursor invariants

Define a complete invariant matrix covering:

- no stored cursor;
- stored cursor present;
- cursor generation and value consistency;
- state version consistency;
- pagination across mapping pages;
- stale or malformed persisted cursor;
- cursor clear/reset, if allowed at all;
- cursor invalidation recovery and full-scan interaction;
- atomicity between the returned cursor value and its generation/state version.

State whether private cursor metadata/value is repeated on every paginated snapshot page or delivered through a separate non-paginated read, and justify the choice.

### 4. Secrecy and validation

Preserve or strengthen:

- `MAX_GDRIVE_CURSOR_BYTES` and `GDriveRawCursorDto` validation;
- redacted `Debug`/`Display` for private DTOs and route/request/response wrappers;
- no raw cursor in route errors, logs, tracing, admin summaries, status, diagnostics artifacts or reports;
- no raw response-body or dependency-error leakage;
- bounded response bodies and strict response decoding;
- no bearer token or Idempotency-Key exposure.

Define required secrecy sentinel tests without using a real cursor value or provider credential.

### 5. Owner-scoped implementation plan

Produce an exact ordered phase plan with one active execution per component. At minimum decide whether the sequence is:

1. API owner contract implementation and clean review;
2. Server owner route/application implementation and DB-capable clean review;
3. GDrive API fan-in/client adaptation and clean review;
4. rerun `GDA-GDA-P4-LONG-RUNNING-RUNTIME` from its accepted P2/OAuth baseline.

State whether Storage needs any code phase. Current evidence says Storage already persists the raw cursor; do not schedule Storage work without a specific missing contract or test obligation.

For each required phase provide:

- exact component and branch;
- proposed phase ID;
- role sequence;
- allowed files and forbidden ownership expansion;
- dependencies and accepted SHA inputs;
- required tests;
- exact CI gate, including DB-capable PostgreSQL evidence where Server transaction behavior is involved;
- clean-review acceptance criteria;
- fan-in order and rollback/compatibility constraints.

Do not route implementation/review/fixer roles to separate ChatGPT chats. Each component uses its one dedicated chat.

### 6. Runtime resume gate

Define the exact evidence required before GDrive runtime implementation may resume, including:

- accepted API contract SHA and clean report blob;
- accepted Server implementation SHA and DB-capable clean report blob;
- exact GDrive fan-in/client SHA and clean report blob if a client adaptation phase is required;
- round-trip proof that a committed cursor is returned to the matching adapter after restart-style reload;
- proof that admin output and all formatted/error surfaces contain no cursor value;
- proof that the adapter does not fall back to a fresh cursor or advance progress implicitly.

## Boundaries

Forbidden:

- product or test edits;
- edits to API, Server, Storage or GDrive implementation docs;
- direct DB access from GDrive;
- weakening the accepted commit, pagination, mode, retry or crash-safety contracts;
- adding public status/control or operator APIs;
- exposing cursor values to admin or public diagnostics;
- real credentials, provider payloads or external-network testing;
- workflow changes, merge, rebase, force-push or PR draft-state changes;
- invented commit, blob, run, job or artifact identifiers.

Allowed:

- read repository files/blobs/commits/PR and CI metadata;
- reason about cross-component contracts and compatibility;
- write only `crates/haze-gdrive-adapter/control/report.md` as a control-only commit with CI skip.

## Reporting

Write only `crates/haze-gdrive-adapter/control/report.md` using the project report template.

The report routing envelope must include:

- `REPORT_TYPE: ARCHITECT_REVIEW`;
- terminal `STATUS`: `ARCHITECT_ACCEPT`, `ARCHITECT_NEEDS_CHANGES`, `ARCHITECT_CHANGED_CONTRACTS`, or `ARCHITECT_BLOCKED`;
- `role: architect`;
- `agent_execution_id: gdrive-adapter-GDA-GDA-P4-cursor-contract-arch-20260717113213-a4c27d91`;
- `chat_name: gdrive-adapter`;
- component `gdrive-adapter`;
- branch `component/gdrive-adapter`;
- wave `W1`;
- phase_id `GDA-GDA-P4-CURSOR-READ-CONTRACT-ARCHITECTURE-REVIEW`;
- control prompt/report paths;
- prompt commit SHA and prompt blob SHA supplied by the launch envelope.

The report must include:

- exact chosen DTO/route design;
- rejected alternatives and rationale;
- authorization and cursor invariant matrices;
- secrecy constraints and tests;
- ordered owner-scoped phase plan;
- exact next component/phase to dispatch;
- whether Storage work is required;
- runtime resume evidence;
- PR state observed;
- statement that no product code or contract file was changed by the Architect.

Use `ARCHITECT_CHANGED_CONTRACTS` when a concrete API/Server contract change is accepted and owner-scoped implementation phases are authorized. Do not claim CI green, product readiness, deployment readiness or merge readiness from this report-only execution.

## Next gate

- `ARCHITECT_CHANGED_CONTRACTS` or `ARCHITECT_ACCEPT` with an executable owner-scoped plan -> Orchestrator opens only the first dependency-ready owner phase;
- `ARCHITECT_NEEDS_CHANGES` -> revised architecture review;
- `ARCHITECT_BLOCKED` -> component remains on contract hold with exact blocker evidence.
