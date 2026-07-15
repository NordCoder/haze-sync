REPORT_TYPE:
ARCHITECTURE_REVIEW

STATUS:
ARCHITECT_ACCEPT

AGENT:
role: architect-reviewer
chat_name: gdrive-adapter — W1 GDA Fan-In Architecture Review

COMPONENT:
name: gdrive-adapter
path: crates/haze-gdrive-adapter
branch: component/gdrive-adapter
pr: 50

WAVE:
id: W1
phase_id: GDA-FAN-IN-ARCHITECTURE-REVIEW

SUMMARY:
The GDrive cross-component fan-in may begin through small owner-scoped phases. V1 uses one standalone long-running `haze-gdrive-adapter` process, public authenticated HTTP transport through API/Server, and Storage-owned durable state exposed through Server application contracts. Direct database access, Server-hosted provider loops, dual hosting, mutable hidden startup behavior, adapter-owned public DTOs, and Deployment packaging of the current immediate-exit skeleton are rejected.

BASELINE_AND_PR:
accepted_component_local_product_sha: 06a7051a7e14c1da45de8cf96a78658b59cb823e
post_sync_sha: f9a2da6eb9ac6f59b1ec18ae4d85eb51964f3cbe
exact_main_ancestor: c1e69a664388b0cba028170e8398b9088218957d
pre_sync_report_blob: 5609c422a4a7a7046bc6537a1dc90d58559bb071
component_ci_run_id: 29409560791
component_ci_run_number: 1962
component_ci_conclusion: success
pr_state_observed: open
pr_draft_observed: true
pr_merged_observed: false
merge_readiness_assessed: false

ARCHITECTURE_DECISION:
runtime_process_owner: GDrive Adapter
hosting_model: standalone long-running process
dual_hosting_allowed: no
transport_owner: API + Server
adapter_transport: HTTP through public Server/API routes
persistence_owner: Storage
database_access_from_adapter: forbidden
persistence_delivery: Server/API-mediated
google_client_owner: GDrive Adapter
oauth_token_loading_owner: GDrive Adapter
oauth_refresh_owner: GDrive Adapter
token_secret_storage_owner: Deployment/operator
token_contents_tracked: never
token_file_write_policy: read-only V1 bootstrap token file; adapter refreshes access tokens in memory and never rewrites the configured token file
scheduling_owner: GDrive Adapter
cursor_advancement_rule: advance only after the complete provider/Core outcome and related durable mapping/echo/delete state commit succeeds
mode_consistency_rule: `HAZE_GDRIVE_MODE` is authoritative; `HAZE_GDRIVE_DRY_RUN` must be removed or accepted only as temporary deprecated compatibility input that may equal `mode=dry_run`, and every contradictory combination fails closed
status_owner: GDrive Adapter for provider-local facts; API owns public DTOs; Server owns central status ingestion/query
 doctor_owner: GDrive Adapter for read-only provider diagnostics; API/Server own central public delivery
operator_control_owner: CLI for operator UX over explicit authenticated API/Server mutation contracts; Server owns authorization, audit and durable mutation
can_product_fan_in_begin: yes
can_deployment_gdrive_service_begin: no
next_recommended_agent: orchestrator

1_PROCESS_AND_LIFECYCLE_OWNERSHIP:
- V1 runs exactly one standalone `haze-gdrive-adapter` process per configured adapter identity/root.
- The adapter binary owns Google client construction, provider polling, periodic full scans, import/export cycles, retry/backoff, rate-limit handling, signal handling, graceful shutdown and safe local status.
- Server does not host, spawn or hide the Google provider loop.
- Deployment later packages and launches only an already accepted long-running binary.
- Standalone and Server-hosted runtimes must never be active for the same adapter identity.
- The current `main.rs` and `runtime.rs` remain a lifecycle skeleton: they validate config, print status, request shutdown and exit without provider, Server or persistence work. They are not a deployable service runtime.

2_CONCRETE_SERVER_API_TRANSPORT:
existing_routes_sufficient:
- `PUT /v1/files/{path}`: submit new/modified Drive content using Authorization, Idempotency-Key, X-Content-SHA256 and known-or-explicit-null X-Base-Revision-Id; returns accepted/same-content/conflict-saved/rejected semantics.
- `DELETE /v1/files/{path}`: submit a confirmed guarded Core delete request; Core remains delete arbiter.
- `GET /v1/changes?since=&limit=`: read authoritative ordered Core changes for Drive export.
- `GET /v1/files/{path}`: retrieve authoritative revision metadata/content for export.
- `GET /v1/server-info`: capability/version negotiation.

new_contracts_required:
A_gdrive_state_snapshot:
- component_owner: API contract, Server runtime/application, Storage persistence
- method: GET
- route: `/v1/adapters/{adapter_id}/gdrive/state`
- purpose: bounded adapter-scoped load of mapping, provider cursor presence/state version, Core export checkpoint, pending delete candidates and retry-safe processing facts required to resume after restart
- request_dto_owner: API
- response_dto_owner: API
- auth_requirement: authenticated principal must match adapter_id and `gdrive_adapter` role; admin may read sanitized state
- idempotency_rule: read-only
- secrecy_constraints: no OAuth/bearer values, token hashes, raw external cursor in public/admin output, raw provider payloads, database errors or unrestricted Drive IDs
- error_categories: unauthorized, forbidden, adapter_not_found, state_version_mismatch, invalid_cursor_state, unavailable, internal

B_gdrive_state_commit:
- component_owner: API contract, Server application/transaction choreography, Storage repository/schema
- method: POST
- route: `/v1/adapters/{adapter_id}/gdrive/state/commit`
- purpose: transactional compare-and-commit of mapping facts, Drive cursor transition, Core export checkpoint, echo confirmation, delete-candidate observation/confirmation and operation identity after an already determined provider/Core outcome
- request_dto_owner: API
- response_dto_owner: API
- auth_requirement: matching `gdrive_adapter` principal
- idempotency_rule: mandatory Idempotency-Key plus operation identifier and expected persisted version/cursor; same request replays, different payload conflicts
- secrecy_constraints: raw cursor may cross only this authenticated private adapter contract and must remain redacted from logs/status/errors; no tokens, raw provider bodies or file bytes except through existing file route
- error_categories: stale_state, cursor_regression, cursor_gap, mapping_conflict, idempotency_conflict, validation_error, unavailable, internal

C_gdrive_status_report:
- component_owner: API DTO, Server authenticated ingestion/storage or bounded runtime summary persistence
- method: PUT
- route: `/v1/adapters/{adapter_id}/status`
- purpose: publish sanitized process lifecycle, configured mode, timestamps, count summaries, cursor presence, mass-delete block, safe error category, provider auth/root and Server connectivity facts
- request_dto_owner: API
- response_dto_owner: API
- auth_requirement: matching adapter principal
- idempotency_rule: last-write-by-monotonic report sequence or observed_at/version; duplicate replay allowed
- secrecy_constraints: no raw cursors, tokens, token hashes, provider payloads, full Drive IDs, response bodies or absolute secret paths
- error_categories: unauthorized, forbidden, stale_status, validation_error, unavailable, internal

D_delete_unlock_control_later:
- component_owner: API + Server + Core/Storage audit contract; CLI owns operator UX
- method: POST
- route: `/v1/admin/adapters/{adapter_id}/delete-block/unlock`
- purpose: explicit time/category/run-scoped audited unlock only after separate operator-control architecture acceptance
- auth_requirement: admin only
- idempotency_rule: mandatory Idempotency-Key and immutable audit operation id
- secrecy_constraints: no reusable secret token in output; unlock value must be scoped, expiring and audit-safe
- error_categories: forbidden, invalid_scope, hard_block_not_unlockable, already_applied, conflict, unavailable
- status: deferred; not part of the first transport phase

The adapter must not call Core internals, use Server-private state, depend on Worktree routes or invent DTOs locally.

3_DURABLE_PERSISTENCE_OWNERSHIP:
Storage_owned_facts:
- Drive file to Core path/object/revision mapping
- Drive change cursor and cursor-generation/version metadata
- Core export sequence checkpoint
- provider version/head revision/checksum/modified-time facts
- echo confirmation state sufficient to identify adapter-originated provider writes
- delete candidate first-seen, last-seen, confirmation generation and blocked state
- delete confirmation/audit references
- last successful import/export/provider mutation operation identifiers
- retry-safe processing leases/version fields where required

Adapter-local memory may hold only bounded in-flight work and access-token cache. It is not authoritative across restart.

transaction_rules:
- Drive cursor never advances before every change represented by that cursor is durably processed or safely classified for replay.
- Core export checkpoint advances only after the provider mutation is confirmed and the matching mapping/echo state commit succeeds.
- Mapping updates occur after confirmed Core or provider outcomes, never at plan time.
- Echo confirmation and provider version/mapping update commit atomically from Server's perspective.
- Delete-candidate validation, state transition and any resulting Core delete submission bookkeeping use compare-and-commit versioning and transactionally consistent mutation.
- Idempotency and operation identifiers make repeated adapter requests replay-safe and prevent duplicate Core mutations.
- Server owns PostgreSQL transaction boundaries; Storage remains passive; Adapter never receives DATABASE_URL.

crash_recovery:
- crash before provider/Core mutation: durable state is unchanged; item replays.
- crash after Core mutation but before state commit: deterministic Idempotency-Key replays the Core result, then state commit repairs progress.
- crash after provider mutation but before state commit: replay first reads provider state using stable operation/mapping facts; if provider outcome is already present, commit mapping/echo/checkpoint without repeating mutation; otherwise retry safely.
- provider APIs without usable idempotency must use precondition/version checks and post-crash reconciliation before any repeated write.
- no cursor/checkpoint may skip an unresolved ambiguous provider outcome.

Existing `gdrive_mapping` repository is useful but insufficient by itself: it lacks a complete adapter-scoped state/version contract, durable Drive cursor transaction, explicit echo confirmation generation, complete delete confirmation history and crash-recovery operation records. Separate Storage fan-in is required.

4_PROVIDER_AND_OAUTH_LIFECYCLE:
- GDrive Adapter owns the concrete Google API client and OAuth refresh execution.
- Deployment/operator owns initial OAuth authorization and creation/placement of the token secret file outside the repository.
- V1 token file is a versioned JSON document with only the fields required to construct refresh-capable credentials, for example schema version, client identity reference/client fields as accepted, refresh token and optional scopes/subject metadata. Raw access tokens need not be persisted.
- The adapter loads the token file at startup from an absolute secret path, validates it and keeps refreshed access tokens in memory.
- The configured token file is read-only to the service in V1. Adapter does not rewrite it and no mutable token cache is required.
- Refresh-token rotation/revocation is an operator action: atomically replace the secret file outside the repo with owner-only permissions, then restart the service.
- Recommended ownership/mode: dedicated service user, directory 0700, file 0600 or stricter, not readable by unrelated services.
- If a future Google response requires refresh-token replacement persistence, that is a separate contract phase requiring same-directory temp file, fsync, atomic rename, directory fsync, ownership/mode preservation and crash tests.
- Corrupted token: fail closed before provider loop, status category `oauth_token_invalid`.
- Expired access token: refresh in memory.
- Revoked refresh token/insufficient scope: stop provider mutation cycles, keep local process/status alive in degraded state, and require operator credential replacement.
- Status/errors expose only configured/not_configured, valid/invalid/revoked/scope_insufficient and last refresh category/time; never token values, raw Authorization headers, provider bodies or absolute secret path.
- Ordinary CI uses fakes/mocks/synthetic fixtures and never real Google credentials.

5_SCHEDULING_AND_CORRECTNESS_LOOP:
- One serialized scheduler owns all cycles. It may use bounded internal async operations, but only one authoritative cycle/checkpoint transition is active per adapter identity.
- Default cadence remains config-owned and bounded: change-feed poll approximately 60 seconds and full scan approximately 3600 seconds unless later validated limits change.
- Add bounded jitter to provider polling/backoff to avoid synchronized fleets; jitter must not postpone the full-scan correctness bound indefinitely.
- Full scan is the correctness backstop. Change feed is latency optimization.
- Cursor invalidation, permission-scope uncertainty or mapping inconsistency triggers a full scan before cursor reinitialization.
- Rate limit/provider unavailable: degrade safely, retain cursor, exponential capped backoff with jitter, no duplicate mutation.
- Server unavailable: continue provider reads only when they do not advance durable state or cause unbounded buffering; perform no provider exports or Core imports; retry with capped backoff.
- Auth revoked/scope loss/root unreachable: stop all mutations and deletion confirmation, report degraded/blocked, require operator action and a successful full scan before resuming.
- Partial batch failure: commit only a contiguous successfully processed prefix or item-level independently versioned commits; never advance a batch cursor past failed work.
- Graceful shutdown stops new cycles, requests cooperative cancellation, allows the current bounded atomic provider/HTTP operation to finish, persists only complete outcomes, then exits after a configured deadline.
- On deadline, exit without advancing unresolved state; restart reconciliation handles ambiguity.
- Maximum authoritative cycle concurrency: one. Per-cycle download/upload concurrency may later be bounded and proven safe, but mapping/checkpoint commits remain serialized.

error_class_behavior:
- config/token corruption: process fails startup.
- revoked auth/scope loss/root inaccessible/mass-delete block: process remains alive degraded, all mutation cycles stopped.
- provider or Server transient outage/rate limit: process remains alive degraded and retries.
- invalid cursor: process remains alive, discards no durable state, schedules full scan.
- local invariant/state-version mismatch: fail closed for affected adapter and require operator/contract intervention.

6_ADAPTER_MODES_AND_DRY_RUN:
Authoritative mechanism:
- `HAZE_GDRIVE_MODE` is the single V1 authority.
- Values: disabled, dry_run, read_only, import_only, export_only, bidirectional.
- Current `HAZE_GDRIVE_DRY_RUN` boolean is compatibility debt. A dedicated GDrive config-normalization phase must reject any supplied boolean unless it exactly represents `mode=dry_run`; after a documented deprecation window it is removed.
- `mode=bidirectional` plus `dry_run=true` is invalid and fails startup closed.

mode_permissions:
- disabled: no provider/Core reads, no durable state mutation, status only.
- dry_run: provider reads and Core reads allowed for planning; no Core writes, provider writes/trash, cursor/checkpoint advancement, mapping/echo/delete-candidate durable mutation or unlock.
- read_only: globally non-mutating observation mode; provider reads and Core reads only; no persistence except sanitized status heartbeat. It is not an alias for export_only.
- import_only: provider reads, Core writes through existing file/delete routes, durable mapping/cursor/delete-candidate commits; no provider writes/trash and no Core export checkpoint consumption beyond non-mutating comparison.
- export_only: Core reads, provider writes/trash only after accepted Core tombstone semantics, durable mapping/echo/export checkpoint commits; no provider-originated Core writes or delete-candidate submission.
- bidirectional: both import_only and export_only capabilities after explicit operator approval.

Rollout order is fixed:
disabled -> dry_run -> one-direction mode -> verified opposite direction -> bidirectional only after explicit operator approval.
No code, Compose or Deployment default may enable bidirectional automatically.

7_DELETE_CANDIDATE_AND_DESTRUCTIVE_BEHAVIOR:
- Candidate storage owner: Storage; transition orchestration: Server; observation/confirmation facts: GDrive Adapter; final delete policy: Core.
- First disappearance creates/refreshes a candidate only after a full scan or corroborated provider evidence.
- Confirmation requires a distinct later full scan/cycle with unchanged root scope and healthy auth permissions. Change-feed disappearance alone is insufficient.
- Folder/root disappearance, auth scope loss, permission filtering, root move or provider-wide anomaly blocks deletion processing and triggers root/full-scan diagnostics.
- A confirmed candidate becomes a Core delete request through existing authenticated `DELETE /v1/files/{path}` with deterministic idempotency and known-or-explicit-null base semantics.
- Provider-side trash is allowed only in export-capable mode after an authoritative Core tombstone/delete operation is observed. Permanent Drive delete is forbidden by default.
- Dry-run changes no provider, Core or persistent candidate state.
- Mass-delete threshold/ratio block stops candidate confirmation and Core delete submission. Hard blocks cannot be overridden.
- Only an authenticated admin through a future audited API/CLI control may lift an unlockable block; no environment toggle or local adapter command may bypass it.
- Audit trail is Storage-owned and includes immutable operation id, adapter identity, scope/run/category, actor, approval timestamp, expiry and result without secret values.

8_STATUS_DOCTOR_AND_OPERATOR_CONTROLS:
local_status:
- structured stdout/log lifecycle events and a read-only local status command or signal-safe snapshot may expose safe facts while the process is running.
- no local mutation/unlock command in V1.

central_status:
- GDrive Adapter produces provider-local facts.
- API owns public DTOs.
- Server authenticates and stores/maps sanitized status, then serves existing/future admin adapter status routes.
- CLI renders central status and doctor results.

status_fields:
- process lifecycle, configured mode, last full scan, last feed poll, last successful import/export, cursor presence only, pending delete candidate count, mass-delete block category, last safe error category, Server connectivity, provider auth category and root reachability.

doctor:
- read-only checks only: config valid, token readable/parseable, auth refresh possible via fake/live explicit mode, root reachable, cursor state sane, mapping consistency summaries, Server connectivity and delete-block state.
- Doctor does not refresh/repair persisted state, mutate provider/Core, clear candidates or unlock blocks without a separate explicit mutation contract.

operator_controls:
- CLI owns confirmation UX and rendering.
- API owns mutation DTOs.
- Server owns authentication, authorization, audit, transaction and application orchestration.
- Storage owns durable audit/state.
- Repair/unlock is separate from status/doctor and requires admin auth, Idempotency-Key, exact scope and immutable audit record.

9_AUTHENTICATION_BOUNDARY:
- Adapter authenticates to Server with `Authorization: Bearer <adapter token>` mapped to a stable adapter identity and minimum `gdrive_adapter` role.
- Server verifies token hash and enforces endpoint-level role plus adapter_id match.
- Token must never appear in process args, tracked Compose, status, doctor, logs or errors.
- Current `HAZE_GDRIVE_ADAPTER_TOKEN` may remain for local development and tests, but production Deployment must use `HAZE_GDRIVE_ADAPTER_TOKEN_FILE` or an accepted secret-manager file projection.
- Supplying both direct and file sources fails closed, as current config already enforces.
- Rotation: operator atomically replaces the token file and restarts the standalone adapter; Server supports overlapping old/new hashes only through a separately accepted rotation procedure.
- Minimum permissions: file PUT/GET/DELETE, changes read, GDrive state snapshot/commit and status report for its own adapter identity. No general admin permission.

10_DEPLOYMENT_READINESS_GATE:
local_compose_service_required:
- standalone loop accepted and no immediate exit
- authenticated public HTTP transport implemented
- durable state contract implemented
- graceful shutdown and safe status implemented
- fake Server/provider smoke integration green
- explicit adapter-token and OAuth secret-file mounts with disabled default

vps_systemd_or_container_required:
- all local Compose requirements
- live Google client and OAuth refresh lifecycle
- bounded retry/backoff/rate-limit behavior
- owner/mode/permission runbook
- restart policy that does not create dual instances
- health/status command that distinguishes degraded from healthy
- backup/migration compatibility for Storage state

 dry_run_rollout_required:
- live provider read/auth/root access
- Core read transport
- mutation-free mode tests proving no provider/Core/persistent state changes
- safe status/doctor

import_only_rollout_required:
- durable Drive cursor/mapping/delete-candidate state
- authenticated PUT/DELETE and state commit routes
- Core idempotency/conflict/delete outcomes integration tests
- mass-delete block tests and no provider mutation

bidirectional_rollout_required:
- export checkpoint and provider mutation recovery implemented
- echo state atomicity
- provider trash versus permanent-delete rules
- crash tests for provider mutation before persistence commit
- verified import-only and export-only stages
- explicit operator approval and coordinated recovery checkpoint

Deployment may not begin GDrive service wiring now because the accepted binary still exits immediately and transport, durable state, live OAuth/provider lifecycle and scheduling are not implemented.

11_ORDERED_IMPLEMENTATION_PHASES:

phase_id: STOR-GDA-P1-DURABLE-STATE
owner_component: storage
branch: component/storage
agent_role: implementation-worker
prerequisites: this architecture report accepted; synchronize exact main and accepted GDrive facts model
allowed_paths: crates/haze-sync-storage/src/models/**; crates/haze-sync-storage/src/repositories/**; crates/haze-sync-storage/src/schema/**; migrations/** only for the minimum accepted GDrive state migration; crates/haze-sync-storage/docs/**; component control files
 deliverables: adapter-scoped versioned GDrive runtime state; Drive cursor and Core export checkpoint repositories; mapping/echo/delete-candidate/operation state transaction APIs; safe redaction; PostgreSQL tests
protected_boundaries: no provider semantics, no Server routes, no public DTOs, no direct adapter DB access decision reversal
acceptance_tests: unit validation/redaction; fresh/pre-state/current migration tests; transaction rollback; cursor regression/gap/stale; candidate compare-and-commit; duplicate operation replay
required_ci: Storage component CI plus mandatory dedicated PostgreSQL evidence on exact code-bearing SHA
required_review: mandatory clean-code review after implementation and after any fixer code-bearing SHA
unblocks: API-GDA-P1 and SRV-GDA-P1

phase_id: API-GDA-P1-CONTRACTS
owner_component: api
branch: component/api
agent_role: implementation-worker
prerequisites: accepted Storage semantic vocabulary, not necessarily Server implementation
allowed_paths: crates/haze-sync-api/src/**; crates/haze-sync-api/docs/**; API control files
 deliverables: GDrive state snapshot/commit DTOs and route contracts; sanitized status report DTO; error vocabulary; role/adapter-id requirements; compatibility fixtures
protected_boundaries: passive only; no DB, Axum, provider or runtime behavior
acceptance_tests: serde fixtures, validation, redaction, raw-cursor exclusion from status, idempotency metadata contract
required_ci: API Component CI exact SHA
required_review: mandatory clean-code review
unblocks: SRV-GDA-P1 and GDA-GDA-P1-HTTP-CLIENT

phase_id: SRV-GDA-P1-APPLICATION-AND-ROUTES
owner_component: server
branch: component/server
agent_role: implementation-worker
prerequisites: exact accepted STOR-GDA-P1 and API-GDA-P1 SHAs synchronized
allowed_paths: crates/haze-sync-server/src/application/**; crates/haze-sync-server/src/routes/**; crates/haze-sync-server/src/state.rs; crates/haze-sync-server/docs/**; Server control files; Cargo manifest only if required
 deliverables: authenticated state snapshot/commit/status routes; Server-owned transaction/idempotency choreography; adapter-id authorization; safe error mapping
protected_boundaries: no Google provider code, no adapter scheduling, no direct public DTO invention, no Deployment changes
acceptance_tests: dependency-free route/auth tests; PostgreSQL transaction tests; stale cursor/version, replay/conflict, rollback and redaction tests
required_ci: DB-capable Server Component CI exact SHA
required_review: mandatory clean-code review
unblocks: GDA-GDA-P1-HTTP-CLIENT

phase_id: GDA-GDA-P1-CONFIG-MODE-NORMALIZATION
owner_component: gdrive-adapter
branch: component/gdrive-adapter
agent_role: implementation-worker
prerequisites: architecture report accepted; independent of Storage/API implementation
allowed_paths: crates/haze-gdrive-adapter/src/config.rs; runtime config tests; relevant docs/control
 deliverables: single authoritative mode contract; fail-closed contradictory dry-run boolean; production file-source auth guidance; no lifecycle/provider work
protected_boundaries: no HTTP, DB, Google client, scheduling or Deployment changes
acceptance_tests: full mode permission matrix; contradictory combinations rejected; secret redaction; direct+file token ambiguity rejected
required_ci: GDrive Component CI exact SHA
required_review: mandatory clean-code review
unblocks: long-running runtime and Deployment config contract

phase_id: GDA-GDA-P2-HTTP-AND-DURABLE-STATE-CLIENT
owner_component: gdrive-adapter
branch: component/gdrive-adapter
agent_role: implementation-worker
prerequisites: exact accepted API-GDA-P1 and SRV-GDA-P1 SHAs; GDA config normalization accepted
allowed_paths: crates/haze-gdrive-adapter/src/** excluding live Google implementation modules if separable; docs/control
 deliverables: authenticated public HTTP client; existing file/change route integration; state snapshot/commit client; deterministic idempotency keys; durable restart/replay integration over fakes
protected_boundaries: no direct DB, no API DTO invention, no live provider calls, no Deployment files
acceptance_tests: fake Server contract tests for PUT/DELETE/GET changes/file content/state commit; replay/conflict; secret/error sanitization
required_ci: GDrive Component CI exact SHA
required_review: mandatory clean-code review
unblocks: GDA live provider and scheduler integration

phase_id: GDA-GDA-P3-LIVE-GOOGLE-OAUTH
owner_component: gdrive-adapter
branch: component/gdrive-adapter
agent_role: implementation-worker
prerequisites: token-file contract and provider abstraction accepted; may proceed independently from Server route implementation if tested with fakes
allowed_paths: Google client/OAuth modules, Cargo manifest, adapter docs/control
 deliverables: concrete Google client; read-only versioned token-file parser; in-memory access-token refresh; safe provider error mapping; fake transport tests
protected_boundaries: no token-file rewrite, no Server/Storage/Deployment changes, no real credentials in tests
acceptance_tests: synthetic OAuth refresh success/expiry/revocation/scope errors; provider pagination/rate limit; redaction; no file writes
required_ci: GDrive Component CI exact SHA without real credentials
required_review: mandatory security-focused clean review
unblocks: long-running scheduler

phase_id: GDA-GDA-P4-LONG-RUNNING-RUNTIME
owner_component: gdrive-adapter
branch: component/gdrive-adapter
agent_role: implementation-worker
prerequisites: GDA HTTP/durable client and live provider client accepted
allowed_paths: adapter runtime/scheduler modules, main.rs, docs/control
 deliverables: standalone serialized loop; full-scan cadence; change-feed poll; retry/backoff/jitter; cancellation/graceful shutdown; degraded states; no immediate exit
protected_boundaries: no Server-hosting, no Deployment service files, no direct DB
acceptance_tests: paused time/fake clock; no overlap; full-scan backstop; cursor invalidation; transient outages; partial batch; shutdown/crash replay
required_ci: GDrive Component CI exact SHA
required_review: mandatory clean-code review
unblocks: status/operator integration and Deployment local service gate

phase_id: API-SRV-GDA-P2-STATUS-AND-CONTROLS
owner_component: api then server, sequential owner slots
branch: component/api then component/server
agent_role: implementation-worker per component
prerequisites: stable runtime status vocabulary from GDA-GDA-P4
allowed_paths: API DTO/contracts first; Server routes/application/audit second
 deliverables: central status ingestion/query; read-only doctor surfaces; separately scoped audited unlock contract only if Core/Storage audit semantics are accepted
protected_boundaries: no raw provider state, no unaudited repair, no adapter/provider implementation
acceptance_tests: role/auth, redaction, stale status, read-only doctor, audited mutation tests if unlock included
required_ci: exact-SHA Component CI for each component, Server DB evidence where persistence changes
required_review: mandatory clean review for each owner slot
unblocks: CLI-GDA-P1 and final deployment readiness

phase_id: CLI-GDA-P1-OPERATOR-SURFACES
owner_component: cli
branch: component/cli
agent_role: implementation-worker
prerequisites: accepted API/Server status and any audited control contracts
allowed_paths: crates/haze-sync-cli/**
 deliverables: read-only GDrive status/doctor rendering; explicit confirmation UX for accepted unlock only; safe exit codes
protected_boundaries: no direct DB/provider access, no hidden repair, no token args
acceptance_tests: output/redaction, role failures, confirmation, no mutation in status/doctor
required_ci: CLI Component CI exact SHA
required_review: mandatory clean review
unblocks: operator runbook and rollout

phase_id: GDA-E2E-P1-FAKE-INTEGRATION
owner_component: gdrive-adapter with explicitly coordinated test-only Server fixtures; prefer adapter branch consuming accepted public contracts
branch: component/gdrive-adapter
agent_role: implementation-worker
prerequisites: runtime, transport and persistence accepted
allowed_paths: adapter tests/fixtures/docs; cross-component test harness only if separately authorized
 deliverables: fake Google plus real Server/API test harness covering import, export, echo, candidates, cursor invalidation, crash replay and modes
protected_boundaries: no live credentials, no production provider calls, no workflow edits without github-ci slot
acceptance_tests: complete staged fake-provider scenarios and leak assertions
required_ci: exact-SHA Component CI; dedicated integration run if DB service required
required_review: mandatory clean review
unblocks: Deployment service wiring

phase_id: DEP-GDA-P1-SERVICE-WIRING
owner_component: deployment
branch: component/deployment
agent_role: implementation-worker
prerequisites: long-running runtime; live OAuth; transport/persistence; status; fake integration; explicit secret-file contract all clean-accepted
allowed_paths: deploy/**; .env.example placeholders; deployment docs/control
 deliverables: disabled-by-default local service; secret file mounts; one-instance lifecycle; shutdown/restart; local-only and VPS runbooks
protected_boundaries: no product code, no real secrets, no migration runner, no automatic bidirectional mode
acceptance_tests: compose/config syntax; immediate-exit rejection/smoke; permissions/runbook review; disabled/dry-run startup
required_ci: Deployment Component CI exact SHA
required_review: mandatory clean review
unblocks: staged rollout runbook

phase_id: DEP-GDA-P2-STAGED-ROLLOUT
owner_component: deployment
branch: component/deployment
agent_role: implementation-worker
prerequisites: DEP-GDA-P1 accepted; operator status surfaces accepted
allowed_paths: deployment runbooks/control only unless explicit config adjustment scoped
 deliverables: disabled -> dry_run -> one-direction -> opposite direction -> explicit bidirectional approval; backup/rollback/stop criteria
protected_boundaries: no automatic provider mutation, no secrets, no production-readiness overclaim
acceptance_tests: manual checklist review and safe placeholder validation
required_ci: docs/config Component CI exact SHA
required_review: clean review
unblocks: operator-approved environment rollout, not merge readiness by itself

parallelism:
- STOR-GDA-P1 and GDA-GDA-P1-CONFIG-MODE-NORMALIZATION may execute independently after this review.
- API-GDA-P1 may begin once Storage state semantics are stable enough to name DTOs; it must not guess them.
- GDA-GDA-P3-LIVE-GOOGLE-OAUTH may proceed after its token/provider contract is pinned and can remain fake-tested, but it does not unblock deployment without transport/persistence/runtime.
- Server follows exact accepted Storage and API SHAs.
- Adapter transport follows exact accepted API and Server SHAs.
- Status/controls follow stable long-running runtime vocabulary.
- Deployment is last.

12_FIRST_EXECUTABLE_SLOT:
first_executable_phase: STOR-GDA-P1-DURABLE-STATE
first_worker_chat_name: storage — W1 STOR-GDA-P11 GDrive Durable State
first_worker_role: implementation-worker
first_component: storage
first_allowed_paths:
- crates/haze-sync-storage/src/models/**
- crates/haze-sync-storage/src/repositories/**
- crates/haze-sync-storage/src/schema/**
- migrations/** only for minimum GDrive durable-state migration
- crates/haze-sync-storage/docs/**
- crates/haze-sync-storage/control/**
first_deliverables:
- versioned adapter-scoped GDrive runtime-state model
- safe Drive cursor and Core export checkpoint repositories
- mapping/echo/delete-candidate/operation compare-and-commit repository contracts
- migration and real PostgreSQL tests
- updated Storage contract/dependency map/implementation log
first_non_goals:
- no API DTO or Server route work
- no Google provider/OAuth code
- no adapter HTTP client or scheduling
- no direct adapter database access
- no Deployment files
- no Core policy or delete-unlock decision
why_no_new_architecture_decision_required:
- ownership and delivery are fixed by this report: Storage persists caller-decided facts through caller-owned Server transactions; Adapter has no DB access; cursor and mapping transaction invariants are explicitly pinned.

DOCUMENTS_AND_SOURCE_READ:
- crates/haze-gdrive-adapter/docs/component-contract.md
- crates/haze-gdrive-adapter/docs/dependency-map.md
- crates/haze-gdrive-adapter/docs/implementation-plan.md
- crates/haze-gdrive-adapter/docs/implementation-log.md
- crates/haze-gdrive-adapter/docs/decisions.md
- crates/haze-gdrive-adapter/control/state.md
- crates/haze-gdrive-adapter/control/prompt.md
- crates/haze-gdrive-adapter/src/config.rs
- crates/haze-gdrive-adapter/src/main.rs
- crates/haze-gdrive-adapter/src/runtime.rs
- crates/haze-sync-api/docs/component-contract.md
- crates/haze-sync-storage/docs/component-contract.md
- crates/haze-sync-storage/src/repositories/gdrive_mapping.rs
- previously accepted Server runtime/application and Deployment migration/service-boundary contracts as synchronized into the branch baseline

EXISTING_CONTRACTS_SUFFICIENT:
- Core authority and conflict/revision/delete/idempotency semantics
- existing authenticated file PUT/GET/DELETE and changes read data-plane routes
- API ownership of DTO/header/error/auth vocabulary
- Server ownership of HTTP runtime, authentication and transaction choreography
- Storage caller-owned repository model and existing passive `gdrive_mapping` foundation
- GDrive full-scan/change-feed/import/export/delete planner invariants
- Deployment ownership of secret placement/service packaging and prohibition on packaging the skeleton

MISSING_CONTRACTS:
- complete Storage GDrive durable state/version/cursor/echo/delete-operation contract
- API GDrive state snapshot/commit/status DTOs and route contracts
- Server application/routes for those contracts
- accepted read-only token-file OAuth refresh lifecycle implementation
- standalone scheduler/shutdown implementation
- central status/doctor and audited unlock contracts
- fake provider plus real Server/API persistence integration evidence
- Deployment service and staged rollout runbooks

AMBIGUITIES:
blocking_ambiguities_found: none after this review
policy_change_required_before_first_phase: no
implementation_plan_documentation_drift:
- existing component docs still describe a placeholder and unresolved persistence despite accepted component-local planner code; this report is the controlling fan-in decision until a later docs-alignment phase updates those sections.

FINAL_VERDICT:
ARCHITECT_ACCEPT
can_product_fan_in_begin: yes
can_deployment_gdrive_service_begin: no
next_recommended_agent: orchestrator
The Orchestrator may archive this review and activate exactly one owner-scoped first implementation slot, `STOR-GDA-P1-DURABLE-STATE`. This report performs no product implementation and does not establish PR merge readiness.

PUSHED:
yes
