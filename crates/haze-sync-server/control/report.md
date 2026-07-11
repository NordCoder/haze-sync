REPORT_TYPE: ARCHITECT_REVIEW

STATUS: ARCHITECT_CHANGED_CONTRACTS

AGENT:
role: architect
agent_execution_id: ARCH-SRV-P7B-CONTRACTS
chat_name: server — W1 SRV-P7B Architecture Decision

COMPONENT:
name: server
path: crates/haze-sync-server
branch: component/server
pr: 45
contract_path: crates/haze-sync-server/docs/component-contract.md
plan_path: crates/haze-sync-server/docs/implementation-plan.md
dependency_map_path: crates/haze-sync-server/docs/dependency-map.md
decisions_path: crates/haze-sync-server/docs/decisions.md
implementation_log_path: crates/haze-sync-server/docs/implementation-log.md
control_prompt_path: crates/haze-sync-server/control/prompt.md
control_report_path: crates/haze-sync-server/control/report.md

WAVE:
id: W1
phase_id: ARCH-SRV-P7B-CONTRACTS
dependency_status: SRV-P7A accepted; SRV-P7B1 returned BLOCKED_BY_CONTRACT; this architecture pass resolves the required cross-component contract decisions but does not implement product code.

SUMMARY:
Accepted one preferred architecture for real Worktree execution. Worktree keeps ownership of scanning, planning, reconciliation, materialization, echo/trash semantics and scheduling validation, but its authoritative cycle boundary becomes awaitable. Server owns reusable async application services, a concrete bounded Worktree executor and one explicit joined hosted task. Storage owns versioned per-adapter Worktree instance/path state and monotonic contiguous export cursor repositories. API owns any new public runtime/operator DTOs. CLI and Deployment remain downstream consumers.

This is ARCHITECT_CHANGED_CONTRACTS rather than ARCHITECT_ACCEPT because the accepted target requires explicit owner-component changes in Worktree and Storage before Server can implement the executor. No workaround or Server-only implementation is accepted.

ACCEPTED_BASELINE:
- SRV-P7A implementation: SELF_ACCEPT
- SRV-P7A fixer: FIX_COMPLETE
- SRV-P7A clean review: CLEAN_ACCEPT
- accepted code-bearing SHA: 37706634fd8dd2d9b299a1c453718f2de63981d0
- Component CI: run 29161721748, run number 1704, conclusion success
- Worktree source: component/worktree@4f7bc748d9b901d7d5c3e43c845ba407c0c36e59
- Worktree product synchronization: 32/32 exact blob identity

TRIGGER_EVIDENCE:
- prompt: crates/haze-sync-server/control/log/20260711-184500Z-W1-SRV-P7B1-implementation-worker-prompt.md
- report: crates/haze-sync-server/control/log/20260711-184500Z-W1-SRV-P7B1-implementation-worker-report.md
- blocker: synchronous Worktree runtime cycle cannot correctly await asynchronous SQLx/Tokio authority
- blocker: correct PUT/DELETE/conflict/idempotency behavior is private route orchestration
- blocker: durable applied/reconciliation state and export cursor contracts are incomplete
- blocker: accepted runtime budgets/invocation/config binding are absent

DECISION_A_ASYNC_WORKTREE_RUNTIME:
chosen_design: Worktree-owned async-compatible scheduler/cycle contract with synchronous filesystem primitives retained; Server owns the async concrete executor and hosted lifecycle.

contract_shape:
- add WorktreeRuntimeCancellation with safe cooperative is_cancelled observation
- WorktreeRuntimeCycle::run_cycle returns an awaited Send future over WorktreeRuntimeCycleRequest and cancellation
- WorktreeRuntimeService::poll becomes async and awaits at most one cycle
- start, cancellation request, status and watcher shutdown may remain synchronous because they perform no async authoritative mutation

ownership:
- Worktree: trait/types, scheduler state, watcher-hint coalescing, periodic correctness, budget validation, count-only result validation and no-overlap
- Server: concrete executor, Tokio host task, startup/shutdown/cancellation composition
- synchronous scanner/materializer/trash/echo calls: Worktree implementation, invoked by Server through explicit awaited bounded blocking-pool work when necessary
- SQLx/application-service calls: awaited normally, never run through blocking

cancellation_and_shutdown:
- one mutable runtime service/executor is owned by one named host task
- no new cycle begins after cancellation
- cancellation is checked before/after every phase and between bounded items
- one current atomic filesystem operation may finish; subsequent phases do not start
- open DB transaction commits only on complete success and otherwise rolls back
- host waits for bounded in-flight cycle, shuts down watcher resources and joins
- no detached task

no_overlap:
- at most one cycle in progress
- watcher, periodic and manual triggers are serialized through one host
- manual requests while busy are rejected safely or coalesced; never run concurrently

correctness:
- watcher hints are latency only
- startup, periodic and watcher-triggered local-observing cycles require a full scan
- watcher failure does not remove periodic correctness
- failures/status are safe categories and counts only

compatibility:
- retain current request, policy, summary and status types where practical
- migrate synchronous test executors to immediately-ready async executors
- migrate scheduler tests to async while retaining deterministic fake clock/watcher behavior
- do not change scanner/planner/materializer public behavior except through the explicit Worktree-owner phase

rejected_alternatives:
- fully synchronous scheduler/cycle: cannot await authority without blocking or nested runtime
- hidden Server task behind synchronous summary: detached/fabricated/error-unsafe
- scheduling moved into Server: duplicates Worktree correctness semantics
- make all filesystem primitives async immediately: unnecessary scope; boundary, not primitives, is incompatible
- nested Tokio runtime, Handle::block_on, internal HTTP and ad hoc blocking: explicitly forbidden and unsafe

DECISION_B_SERVER_APPLICATION_SERVICES:
chosen_design: ServerApplicationServices is a reusable async internal service layer used by routes and Worktree execution.

proposed_modules_and_types:
- src/application/mod.rs
- src/application/files.rs
- src/application/deletes.rs
- src/application/changes.rs
- src/application/idempotency.rs
- ServerApplicationServices
- ApplicationActor
- ApplyFileCommand / ApplyFileOutcome
- ApplyDeleteCommand / ApplyDeleteOutcome
- AuthoritativeChangesQuery / AuthoritativeChangeBatch
- RevisionContentQuery / AuthoritativeRevisionContent
- ApplicationError

required_operations:
- apply_file(actor, normalized path/base/hash/bytes/idempotency metadata)
- apply_delete(actor, normalized path/base/delete-guard/idempotency metadata)
- list_authoritative_changes(since, bounded limit)
- load_revision_content(path/revision) with verified metadata and bytes

required_outcomes:
- accepted new/current revision
- same-content replay/no-op with authoritative revision metadata
- conflict saved with safe conflict identity/materialized path metadata
- rejected/stale/unsafe write
- tombstoned/not-found/rejected delete
- ordered bounded change batch with from/to/has_more
- verified revision metadata/content for materialization

transaction_ownership:
- durable idempotency lookup and fingerprint comparison
- SQLx transaction begin
- normalized vault-path advisory lock for mutations
- authoritative current-state read
- Core decision/planning
- object-store write/read coordination
- Storage object/revision/conflict/tombstone persistence
- operation-log append
- safe replay outcome persistence
- commit/rollback

Core policy is never duplicated. Storage remains passive and caller-transaction-owned. Object-store writes remain content-addressed and replay-safe; a blob orphaned by DB rollback cannot become authoritative without metadata commit.

idempotency_for_worktree:
- put key derivation: wt:v1:<adapter_id>:put:<path_hash>:<base_or_null>:<content_hash>
- delete key derivation: wt:v1:<adapter_id>:delete:<path_hash>:<base_or_null>
- deterministic and retry-stable
- key value never logged, exposed or placed in status
- same durable idempotency repository/fingerprint comparison as routes
- HTTP client-supplied keys remain API-owned inputs

route_extraction:
- extract planning from routes/v1/planning.rs
- extract persistence from routes/v1/persistence.rs
- extract PUT/DELETE idempotency choreography
- extract DELETE guard/tombstone/operation-log transaction flow from routes/delete.rs
- retain parsing, auth, HTTP status, API DTO and response construction in routes
- delete obsolete private helpers only after route parity and Worktree consumer tests pass

backward_compatibility:
- no public HTTP route, status, header or body change in application-service extraction
- dependency-free router tests remain valid
- application services return internal typed outcomes, not public DTOs

rejected_alternatives:
- Worktree calling HTTP handlers/localhost routes
- duplicated route transaction/policy logic in executor
- full application services in Storage
- SQLx/runtime behavior in Core

DECISION_C_DURABLE_WORKTREE_STATE_AND_CURSOR:
chosen_owner: Storage owns schema, migration, row and repository primitives; Server owns transaction timing; Worktree owns semantic planner/materializer state types.

existing_schema_assessment:
- worktree_state is path-only and lacks adapter/root identity, explicit present/tombstoned kind, state version and accepted repository methods
- therefore existing schema is insufficient
- adapter_cursors.last_core_seq is suitable for the authoritative export checkpoint after locked contiguous advancement is added

minimum_schema_contract:
- versioned Worktree runtime-instance binding keyed by adapter_id
- non-public SHA-256 fingerprint of normalized configured root
- state format version and safe timestamps
- path-state key (adapter_id, path)
- explicit present/tombstoned state kind
- last-applied revision and present-file content hash
- bounded/versioned reconciliation observation fields required by Worktree contracts
- use adapter_cursors.last_core_seq for export checkpoint

identity:
- per Worktree adapter instance, not global path state
- default V1 adapter id: worktree
- root fingerprint binding is fail-closed on mismatch
- raw root is not public output

repository_contracts:
- bind_or_verify_instance
- load_path_state / load_snapshot
- upsert_present_state
- upsert_tombstoned_state
- update reconciliation observation fields
- lock/read export cursor
- advance exact contiguous cursor monotonically
- all over caller-owned executor/transaction with safe errors

serialization_versioning:
- relational typed columns for identity/checkpoint-critical fields
- optional reconciliation JSON must carry explicit bounded schema version
- unknown versions fail safely

state_advancement:
- accepted/same-content import then present state
- accepted tombstone then tombstoned state
- conflict/rejection/failure does not claim applied state
- export materializes one authoritative operation, then path state and exact cursor advancement are committed in one DB transaction
- cursor never advances beyond an unprocessed operation

crash_restart_semantics:
- crash before materialization: cursor remains old; replay
- crash after materialization before DB state/cursor commit: replay observes AlreadyCurrent; then commit state/cursor
- crash after accepted import before state update: deterministic idempotency replays accepted outcome; then state update
- rollback does not leave false applied state/cursor
- orphaned content-addressed blob is not authoritative without DB metadata

runtime_persistence:
- durable: instance binding/version, path state, cursor, last-success metadata
- process-local and labelled since_start: cycle counters, pending hints, in-progress and last-cycle summary

cleanup_retention:
- retain state while adapter instance and referenced revision/tombstone state remain operationally relevant
- cleanup requires separate explicit owner phases
- no automatic destructive cleanup

test_obligations:
- no skipped export under crash/restart
- replay of materialized-but-uncheckpointed change
- no unsafe duplicate import after accepted mutation
- no false last-applied state on conflict/failure
- cursor regression rejection
- root-binding/version mismatch rejection
- transaction rollback

rejected_alternatives:
- production in-memory WorktreeStateSnapshot
- authoritative hidden metadata files under Worktree root
- opaque path state solely in adapter_cursors.external_cursor_json
- cursor advance before materialization

DECISION_D_RUNTIME_POLICY_CONFIGURATION_INVOCATION:
chosen_owner: Server config and hosted lifecycle; Worktree validates scheduler policy; Deployment supplies operator values/runbook.

defaults:
- adapter_id: worktree
- max_import_actions: 100
- max_delete_candidates: 100
- max_export_actions: 100
- periodic_correctness_interval: 60 seconds
- watcher_debounce: 500 milliseconds
- max_watcher_hints_per_poll: 256
- host_idle_wake_interval: 250 milliseconds
- graceful_cycle_shutdown_budget: 30 seconds

policy:
- explicit config fields with deterministic documented conservative defaults
- every value non-zero, validated and bounded
- delete action budget does not replace Worktree/Core count/ratio guards
- no unbounded background work or hidden retries

invocation:
- startup binds durable identity before first cycle
- one joined host owns periodic/watcher/manual invocation
- explicit manual one-cycle invocation uses same host queue
- shutdown cancels, stops scheduling, waits bounded cycle, shuts watcher and joins

mode_semantics:
- disabled: no host, watcher, state binding work or cycle
- read_only: authoritative Core-to-Worktree export only
- import_only: full-scan import and guarded local-delete submission only
- export_only: authoritative Core-to-Worktree export only
- bidirectional: both directions within budgets
- dry_run: explicit manual cycle only; may load/scan/query/plan and return count-only summaries, but no Core mutation, cursor advance, materialization, trash move, echo write or durable state mutation

readiness_status:
- /health remains process health
- Disabled does not make Server unready when required HTTP dependencies are ready
- enabled mode requires valid config, durable identity binding, DB/object-store readiness and running/non-terminal host
- host/cycle failure degrades Worktree readiness/status, not /health
- output contains mode/lifecycle/category/count/cursor-presence only
- no raw root, cursor, idempotency key, token, SQLx/I/O error or payload

rejected_alternatives:
- invisible hard-coded policy
- every value mandatory with no deterministic defaults
- enabled startup before durable binding/readiness
- continuously hosted DryRun

PHASED_IMPLEMENTATION_ORDER:
1.
phase_id: WT-P10
chat_name: worktree — W1 WT-P10 Async Runtime Contract
owner_component: worktree
branch: component/worktree
role: implementation-worker
prerequisites: accepted architecture and Worktree snapshot
allowed_files: Worktree runtime, tests, exports and Worktree-owned docs/control explicitly assigned
forbidden_scope: Server/Storage/API source, DB/HTTP/provider behavior, blocking bridges, hidden tasks
key_deliverables: awaitable cycle/poll, cancellation, DryRun representation, no-overlap/full-scan/budget compatibility tests
test_ci: Worktree fmt/check/test/clippy and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: SRV-P7B3 and SRV-P7B4

2.
phase_id: STOR-P10
chat_name: storage — W1 STOR-P10 Worktree Durable State
owner_component: storage
branch: component/storage
role: implementation-worker
prerequisites: accepted architecture
allowed_files: Storage migration/schema/model/repository/tests/docs/control explicitly assigned
forbidden_scope: Core policy, Server runtime, Worktree filesystem, API DTOs
key_deliverables: versioned instance/path state, migration, typed repositories, contiguous cursor operations, mismatch/rollback tests
test_ci: Storage unit/DB/migration tests and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: SRV-P7B3 and Deployment migration work

3.
phase_id: SRV-P7B2
chat_name: server — W1 SRV-P7B2 Application Services
owner_component: server
branch: component/server
role: implementation-worker
prerequisites: this architecture decision
allowed_files: Server application modules, minimal route delegation/state/tests/docs/control explicitly assigned
forbidden_scope: Worktree execution/host, schema, public DTO changes, provider behavior, route behavior changes
key_deliverables: shared async file/delete/changes/revision-content services, deterministic Worktree idempotency, route extraction and parity tests
test_ci: DB-backed transaction/idempotency/conflict/delete parity and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: SRV-P7B3

4.
phase_id: SRV-P7B3
chat_name: server — W1 SRV-P7B3 Bounded Worktree Executor
owner_component: server
branch: component/server
role: implementation-worker
prerequisites: WT-P10, STOR-P10 and SRV-P7B2 clean-accepted and synchronized
allowed_files: Server Worktree executor/application/tests/docs/control plus explicit accepted fan-in files
forbidden_scope: scheduler redesign, route duplication, internal HTTP, unbounded/hidden work, public DTO changes
key_deliverables: real async executor, bounded scan/reconcile/import/delete/export/materialization/checkpoint, cancellation and crash/replay tests
test_ci: DB/object-store/temp-worktree integration/failure injection and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: SRV-P7B4

5.
phase_id: SRV-P7B4
chat_name: server — W1 SRV-P7B4 Hosted Worktree Runtime
owner_component: server
branch: component/server
role: implementation-worker
prerequisites: SRV-P7B3 clean-accepted
allowed_files: Server config/startup/host/state/tests/docs/control
forbidden_scope: API DTO, CLI/deploy files, detached tasks, overlapping cycles, unbounded retry
key_deliverables: joined host, explicit policy config/defaults, identity binding, periodic/watcher/manual scheduling, graceful cancellation/shutdown, exact modes
test_ci: paused-time/no-overlap/startup/shutdown tests and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: API/status/Deployment

6.
phase_id: API-P8
chat_name: api — W1 API-P8 Worktree Runtime Status Contract
owner_component: api
branch: component/api
role: implementation-worker
prerequisites: accepted internal status vocabulary from SRV-P7B4
allowed_files: passive API DTO/parser/error tests/docs/control
forbidden_scope: Server runtime, DB, filesystem, mutation execution
key_deliverables: safe runtime status and optional manual-cycle DTO contract; no raw root/cursor/error/secret
test_ci: serde/backward-compatibility/redaction tests and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: SRV-P7B5 and CLI

7.
phase_id: SRV-P7B5
chat_name: server — W1 SRV-P7B5 Worktree Status and Readiness
owner_component: server
branch: component/server
role: implementation-worker
prerequisites: SRV-P7B4 and API-P8 clean-accepted and synchronized
allowed_files: Server readiness/admin/operator mapping/state/tests/docs/control
forbidden_scope: API shape invention, destructive repair, raw diagnostics, provider behavior
key_deliverables: safe readiness/status, optional one-cycle route over single host, disabled/enabled/failure semantics, dependency-free compatibility
test_ci: route/status/readiness/redaction tests and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: CLI and Deployment

8.
phase_id: CLI-P6A
chat_name: cli — W1 CLI-P6A Worktree Sync Once
owner_component: cli
branch: component/cli
role: implementation-worker
prerequisites: API-P8 and SRV-P7B5 clean-accepted
allowed_files: CLI source/tests/docs/control
forbidden_scope: direct DB/filesystem mutation, runtime hosting, provider calls, bypassing Server/API
key_deliverables: read-only runtime status and explicit sync-once/DryRun operator commands with safe busy/not-ready/failure output
test_ci: parser/client/output/redaction tests and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: operator rollout

9.
phase_id: DEP-P5A
chat_name: deployment — W1 DEP-P5A Worktree Runtime Fan-In
owner_component: deployment
branch: component/deployment
role: implementation-worker
prerequisites: STOR-P10, SRV-P7B4 and SRV-P7B5 clean-accepted; migration execution policy accepted
allowed_files: deploy/** and placeholder .env.example alignment
forbidden_scope: product source, real secrets, automatic bidirectional enablement, hidden migrations, destructive cleanup
key_deliverables: config/default docs, adapter/root binding, permissions, migration/backup/rollback, staged rollout and lifecycle/readiness runbook
test_ci: applicable config/compose validation and normal Component CI
clean_gate: mandatory clean-code review after green CI
downstream_unblocks: deployment-ready Worktree runtime

ORCHESTRATOR_NEXT_ACTIVATIONS:
May activate next, independently:
- worktree — W1 WT-P10 Async Runtime Contract
- storage — W1 STOR-P10 Worktree Durable State
- server — W1 SRV-P7B2 Application Services

Must remain blocked:
- SRV-P7B3 until WT-P10, STOR-P10 and SRV-P7B2 are each clean-accepted and exact SHAs are synchronized
- SRV-P7B4 until SRV-P7B3 clean acceptance
- API-P8 until internal status vocabulary is accepted from hosted runtime work
- SRV-P7B5 until API-P8 and SRV-P7B4 are synchronized
- CLI-P6A until API-P8 and SRV-P7B5
- DEP-P5A until Storage/Server runtime/status and migration policy are accepted

CHANGED_FILES:
- crates/haze-sync-server/docs/component-contract.md
- crates/haze-sync-server/docs/implementation-plan.md
- crates/haze-sync-server/docs/dependency-map.md
- crates/haze-sync-server/docs/decisions.md
- crates/haze-sync-server/docs/implementation-log.md
- crates/haze-sync-server/control/report.md

DOCUMENTATION_COMMITS:
- f9c2b07f30419b78829bcda58c104ca502b80a37
- f33bec013e3ea7a5a8ca5d7af17565319c254ecf
- 0f722cad95a40724ace6a1c9dc37638a93552b6f
- 8fc072641a78d01f2d164e07e9e9806d56d35110
- b98f5079ed90ccf7eaec79e617ae591e0c308ff4

BRANCH_AND_CONTROL:
current_branch: component/server
base_branch: main
pr_number: 45
default_branch_modified: no
sibling_branch_modified: no
pr_merged: no
pr_draft_state_changed: no
rebase_reset_force_push_used: no
control_prompt_read: yes
control_state_read: yes
control_report_written: yes
ci_skip_used: yes
ci_skip_reason: final report-only commit; all contract/plan/dependency/decision/log changes ran normal Component CI

SCOPE:
allowed_files_only: yes
scope_expansion_used: no
cross_component_product_changes: no
product_source_changes: no
cargo_manifest_changes: no
migration_changes: no
workflow_changes: no
sibling_component_changes: no

CONTRACT:
contract_read: yes
contract_satisfied: yes
contract_changes_requested: yes
contract_changes_accepted_in_server_docs: yes
contract_change_rationale: real Worktree execution requires a Worktree-owner async cycle contract and Storage-owner durable state/cursor contract before Server consumption; Server-only work cannot be correct.
affected_components: worktree, storage, server, api, cli, deployment
Core policy changes required: no
public HTTP compatibility changes in this phase: no

IMPLEMENTATION_OR_REVIEW:
completed:
- read active Server prompt/state and archived SRV-P7B1 evidence
- read archived SRV-P7A implementation recovery, wiring fix, CI fixer and clean-review evidence
- reviewed Server current contract/plan/dependencies/decisions/log, config, startup, state, readiness, runtime boundary, route transaction helpers, repositories and object store
- reviewed accepted Worktree contract/runtime docs and public runtime/scanner/import/delete/reconciliation/materialization/echo/trash interfaces
- reviewed Storage schema/repository ownership, current worktree_state and adapter_cursors tables, operation log, revisions and object store
- reviewed Core and API ownership contracts
- reviewed CLI and Deployment upstream blockers
- chose one target architecture and rejected unsafe/inferior alternatives
- produced owner-ordered phases with exact suggested chat names and gates
main_changes: documentation contracts only
behavior_changes: none
bugs_found: architectural incompatibility and persistence/config gaps confirmed
bugs_fixed: none; architecture phase intentionally contains no product code
cleanups_made: normalized Server docs around the accepted SRV-P7B contract
non_goals_preserved: no source implementation, provider behavior, hard delete, repair, public route redesign, hidden background work or fake state

TESTS_AND_CHECKS:
checks_run:
- GitHub connector review of required contracts/source/evidence
- normal Component CI on documentation-bearing SHA b98f5079ed90ccf7eaec79e617ae591e0c308ff4
ci_status: CI_GREEN
workflow: Component CI
workflow_run_id: 29165123142
workflow_run_number: 1719
workflow_status: completed
workflow_conclusion: success
passed:
- cargo fmt
- cargo check
- cargo test
- cargo clippy
- Finalize CI diagnostics
artifact_or_raw_logs_read: no
known_failures: none
report_only_commit_ci: skipped by allowed [skip ci] policy; not architecture validation evidence

SAFETY_AND_SECRECY:
secrets_committed: no
unsafe_public_output: no
raw_errors_exposed: no
raw_sqlx_or_io_output_added: no
absolute_roots_exposed: no
provider_calls_added: no
hard_delete_added: no
automatic_destructive_repair_added: no
background_jobs_added: no; only future explicit contract documented
internal_http_self_call_accepted: no
blocking_async_bridge_accepted: no
production_in_memory_state_accepted: no

ISSUES_FOUND:
- Existing WorktreeRuntimeCycle and WorktreeRuntimeService::poll are synchronous and cannot host async authority safely.
- Correct file/delete/conflict/idempotency choreography is route-private and must become a reusable application service.
- Existing worktree_state is insufficient for per-instance durable state and tombstones/versioning.
- adapter_cursors needs a locked exact-contiguous advancement contract for Worktree export.
- Current Server config has only root/mode and needs explicit bounded runtime policy and adapter identity.
- Public Worktree operator/status changes require API ownership before Server/CLI consumption.

BLOCKERS:
- No blocker remains for the architecture decision itself.
- Product implementation remains contract-gated by the ordered owner phases above.
- SRV-P7B1 must not be retried as a Server-only phase.

NEXT_RECOMMENDED_AGENT:
- implementation-worker for `worktree — W1 WT-P10 Async Runtime Contract`
- implementation-worker for `storage — W1 STOR-P10 Worktree Durable State`
- implementation-worker for `server — W1 SRV-P7B2 Application Services`
These three may be activated independently; later consumers remain blocked by their exact clean-accepted SHAs.

FINAL_VERDICT:
ARCHITECT_CHANGED_CONTRACTS. The current evidence is sufficient and one preferred, implementable architecture is accepted. The target preserves Core/API/Storage/Worktree/Server ownership, eliminates the synchronous/async incompatibility without blocking or hidden tasks, defines reusable application services, durable crash-safe state/cursors, explicit bounded runtime policy, safe readiness/status semantics and an owner-ordered execution plan. No product code was implemented. Orchestrator should activate WT-P10, STOR-P10 and SRV-P7B2 next and must not start SRV-P7B3 until all three pass normal CI and clean-code review and are synchronized by exact accepted SHAs.

PUSHED:
yes