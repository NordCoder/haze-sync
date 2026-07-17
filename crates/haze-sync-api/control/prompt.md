# W1-API-GDA-P2-PRIVATE-CURSOR-SNAPSHOT-CONTRACT

## Routing envelope

- protocol_version: `3`
- repository: `NordCoder/haze-sync`
- component: `api`
- component_path: `crates/haze-sync-api`
- role: `implementation-worker`
- agent_execution_id: `api-API-GDA-P2-private-cursor-impl-20260717122757-3799a1`
- chat_key: `api`
- branch: `component/api`
- pull_request: `#44`
- wave: `W1`
- phase: `API-GDA-P2-PRIVATE-CURSOR-SNAPSHOT-CONTRACT`
- control_prompt_path: `crates/haze-sync-api/control/prompt.md`
- control_report_path: `crates/haze-sync-api/control/report.md`
- expected_report_type: `IMPLEMENTATION`

Use the existing dedicated API component chat. For this execution only, act as `implementation-worker`. Do not create role-specific routing and do not derive current state from chat history.

## Mandatory source order

Read and apply, in order:

1. the current project implementation manifest and report template;
2. `crates/haze-sync-api/docs/component-contract.md`;
3. `crates/haze-sync-api/docs/implementation-plan.md`;
4. `crates/haze-sync-api/docs/implementation-log.md`;
5. `crates/haze-sync-api/docs/dependency-map.md`;
6. `crates/haze-sync-api/docs/decisions.md`;
7. Architect report blob `dc95fa55d3b707da462beebe56b32d73cd54db86`;
8. accepted API baseline SHA `c60c3976696da1970d539e5cff6e9f74a61fc10e`, especially:
   - `crates/haze-sync-api/src/dto/gdrive.rs` blob `8098457eee373f63210fbd381c6487a054d7609f`;
   - `crates/haze-sync-api/src/routes/gdrive.rs` blob `ef7f8dc1d02b3742e76bb99c66ee39703bee3b4b`;
9. accepted Storage state evidence SHA `3617bd1cf947fdd394f1ab29d4b992f7b8859a84`, read-only as contract evidence;
10. accepted Server SHA `c023b83e1e6f502e7d2261acccb871dd5588edf1`, read-only as downstream evidence;
11. current API branch code, tests, docs, control state, PR metadata and relevant CI evidence;
12. this active prompt.

Old prompts and reports are read-only evidence unless explicitly identified above. Do not archive control files.

## Accepted baseline

The accepted API GDrive contract baseline is:

- code-bearing SHA: `c60c3976696da1970d539e5cff6e9f74a61fc10e`;
- clean-review report blob: `55ff6047c9c6c0f6f548f10197b76706c0a244e1`;
- Component CI run `29446546229`, run number `2025`, conclusion `success`.

The current branch contains later control-only history. Do not reset, rebase, force-push or discard that history. Preserve all accepted GDrive DTO, route, error, header, pagination, idempotency and redaction behavior except for the exact private cursor response correction below.

## Architecture decision

The private GET contract currently stores a cursor through Server/Storage but cannot return its opaque value to the matching adapter after restart. The accepted correction is a coordinated breaking change to the existing private response shape.

- Existing route remains `GET /v1/adapters/{adapter_id}/gdrive/state`.
- No new route is authorized.
- Storage requires no code change.
- Matching `gdrive_adapter` receives the private snapshot.
- Admin continues to receive the separate sanitized summary with generation/presence only.
- API implementation must finish and pass clean review/CI before Server work begins.

## Required implementation

### 1. Exact private cursor DTO

In `crates/haze-sync-api/src/dto/gdrive.rs`, add exactly this type-level state model:

```rust
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum GDrivePrivateCursorStateDto {
    Absent {
        generation: u64,
    },
    Present {
        generation: u64,
        cursor: GDriveRawCursorDto,
    },
}
```

Requirements:

- variants serialize as tagged states `absent` and `present`;
- `Absent` cannot carry a cursor value;
- `Present` cannot omit the cursor value;
- do not replace `GDriveRawCursorDto` or weaken its bounds;
- `MAX_GDRIVE_CURSOR_BYTES` remains `8192`;
- empty, over-bound and control-character cursor values remain invalid;
- implement custom redacted `Debug` and `Display` that never format the raw cursor;
- do not derive a value-bearing `Debug` implementation.

### 2. Private snapshot shape

Change only the private response field:

```rust
pub cursor: GDriveCursorSummaryDto
```

to:

```rust
pub cursor: GDrivePrivateCursorStateDto
```

in `GDriveStateSnapshotResponse`.

Preserve every other private snapshot field and the current pagination shape.

`GDriveCursorSummaryDto { generation, present }` remains unchanged and remains the cursor representation for `GDriveStateAdminSummaryResponse` only.

### 3. Validation invariants

Update passive API validation so these are the only valid private cursor states:

- no stored cursor: `Absent { generation: 0 }`;
- stored cursor: `Present { generation > 0, cursor }`;
- `Absent` with non-zero generation fails as the existing safe invalid-cursor-state category;
- `Present` with generation zero fails as the existing safe invalid-cursor-state category;
- no clear/reset operation is added;
- existing commit `advance = None` continues to mean unchanged, not cleared;
- state-format, state-version, pagination, collection and numeric bounds remain unchanged.

Provide narrowly scoped helpers/accessors only when they reduce duplication and preserve passive DTO ownership. Do not add runtime or Storage behavior.

### 4. Admin sanitization

Update `sanitize_snapshot_for_admin` so it converts either private variant into the existing admin summary:

- `Absent { generation }` -> `GDriveCursorSummaryDto { generation, present: false }`;
- `Present { generation, .. }` -> `GDriveCursorSummaryDto { generation, present: true }`.

The admin response must remain:

- cursor-value-free;
- provider-identifier-free;
- mapping-free;
- strict and bounded;
- unchanged in wire shape.

Do not move the raw cursor into `GDriveCursorSummaryDto` and do not add an optional raw field beside the summary.

### 5. Authorization and route ownership

Preserve existing passive route-contract behavior:

- matching `gdrive_adapter` principal may receive the private snapshot;
- non-matching adapter remains `403 forbidden`;
- unauthenticated caller remains `401 unauthorized`;
- authenticated admin receives only `GDriveStateAdminSummaryResponse`;
- route strings, auth vocabulary, error mapping and request pagination remain unchanged.

API owns passive DTO and route validation only. Do not add Axum execution, database access, transaction choreography or Server handler behavior.

### 6. Strict serde, secrecy and regression tests

Add focused tests proving at least:

- exact `absent` private wire variant with generation only;
- exact `present` private wire variant with generation plus a synthetic bounded cursor sentinel;
- `present` without cursor fails strict decoding;
- `absent` with a cursor field fails strict decoding;
- unknown fields fail strict decoding;
- empty, over-bound and control-character cursor values fail;
- absent/non-zero and present/zero generation pairs fail private snapshot validation;
- admin sanitization JSON contains generation and presence but no private cursor field/value;
- private DTO, snapshot, route wrappers and errors do not expose the synthetic sentinel through `Debug` or `Display`;
- existing mapping, pagination, commit, idempotency, error and fixture tests remain valid or are updated only for the accepted private wire shape.

Use synthetic values only. Do not place a real provider cursor, token, credential, provider payload or private path in code, fixtures, logs, reports or diagnostics.

### 7. Compatibility boundary

This private response change is intentionally breaking for strict old GDrive clients.

- API library acceptance is not a deployment event.
- Do not add a versioned duplicate route in this phase.
- Do not claim old/new mixed compatibility.
- Do not edit Deployment.
- Record that Server and corrected GDrive client must later be released as one coordinated compatibility unit.
- If current repository evidence proves a live old GDrive service exists, stop with `BLOCKED_BY_CONTRACT` and exact evidence instead of assuming compatibility.

## Allowed scope

- `crates/haze-sync-api/src/dto/gdrive.rs`;
- `crates/haze-sync-api/src/routes/gdrive.rs`;
- `crates/haze-sync-api/src/dto/mod.rs` only if an export is required;
- `crates/haze-sync-api/src/routes/mod.rs` only if an export is required;
- focused API tests colocated with or already covering these modules;
- `crates/haze-sync-api/docs/**` only for focused contract/decision/implementation-log alignment required by this phase;
- `crates/haze-sync-api/control/report.md`.

A broader internal API refactor is allowed only when required for a clean passive contract implementation and must be explained in the report.

## Forbidden scope

- Server handler, Axum, transaction or PostgreSQL changes;
- Storage schema, migration or repository changes;
- GDrive client, provider, scheduler or runtime changes;
- Core, Common, Worktree, Obsidian, CLI or Deployment changes;
- new public/admin status or operator surfaces;
- new route or route version;
- cursor clear/reset semantics or fresh-cursor recovery policy;
- raw cursor exposure in admin output, status, diagnostics, errors, logs, tracing, Debug or Display;
- dependency, lockfile or workflow changes unless an unavoidable component-local tooling fact is proven and reported before writing;
- test weakening, real credentials, live provider calls or external-network CI;
- merge, rebase, force-push or PR draft-state changes.

## Commit and CI honesty

- Product, test, contract-doc and implementation-doc commits must not use CI skip.
- A final report-only commit may use CI skip.
- Obtain a real final code-bearing SHA.
- Obtain full Component CI for that exact SHA.
- Record `cargo fmt`, `cargo check`, `cargo test`, `cargo clippy` and diagnostics-finalizer conclusions separately.
- A skipped workflow is not green CI evidence.
- Do not fabricate commit, blob, run, job or artifact identifiers.
- If CI is pending, use `SELF_ACCEPT_PENDING_CI` with exact pending coordinates.
- If CI is red, report the exact run and diagnostics artifact metadata without guessing the failure; a focused fixer phase follows.
- If the architecture cannot be implemented within API ownership, stop with `BLOCKED_BY_CONTRACT` and exact evidence.

## Reporting

Write only `crates/haze-sync-api/control/report.md` using the project report template.

The report routing envelope must include exactly:

- `REPORT_TYPE: IMPLEMENTATION`;
- a terminal implementation `STATUS`;
- `role: implementation-worker`;
- `agent_execution_id: api-API-GDA-P2-private-cursor-impl-20260717122757-3799a1`;
- `chat_name: api`;
- component `api`;
- branch `component/api`;
- wave `W1`;
- phase_id `API-GDA-P2-PRIVATE-CURSOR-SNAPSHOT-CONTRACT`;
- control prompt/report paths;
- prompt commit SHA and prompt blob SHA supplied by the dispatcher launch envelope.

Also record:

- changed files;
- exact DTO and serde shape;
- invariant and admin-sanitization behavior;
- strict and secrecy tests;
- compatibility statement;
- final code-bearing SHA;
- exact Component CI metadata;
- statement that no Server, Storage or GDrive implementation was changed.

Do not archive prompt/report files. Do not claim `CLEAN_ACCEPT`, Server readiness, GDrive runtime readiness, deployment readiness or merge readiness.

## Next gate

- `SELF_ACCEPT` with exact-SHA green CI -> focused API clean-code/security review in the same dedicated API chat;
- `SELF_ACCEPT_PENDING_CI` -> Orchestrator verifies exact-SHA CI before transition;
- `SELF_NEEDS_FIX` -> continuation or focused API fixer;
- `BLOCKED_BY_CONTRACT` -> architecture/contract hold with exact evidence;
- `BLOCKED_BY_TOOLING` -> tooling hold with exact evidence.

Server phase `SRV-GDA-P2-PRIVATE-CURSOR-SNAPSHOT-ROUTE` remains blocked until this API implementation and its clean review are accepted with exact-SHA green CI.
