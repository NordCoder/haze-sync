# Recovery and Mutation Contracts

Status: normative V1 contract  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Owned scope: coordinated backup/restore, upgrade/rollback, and transactional `accept_conflict`

## 1. Normative language and scope

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

This document defines behavior and ownership boundaries. It does not authorize executable changes by itself.

The contract covers:

- coordinated backup and restore of PostgreSQL, the Haze Sync object store, and an optional Worktree recovery artifact;
- safe upgrade and rollback orchestration;
- the atomic mutation required to promote preserved conflict content through `accept_conflict`;
- the evidence required for later recovery acceptance and failure-injection work.

The contract does not redefine ordinary revision, conflict creation, tombstone, delete-retention, or metadata-only conflict-resolution semantics.

## 2. Compatibility baseline

Implementations MUST preserve these accepted boundaries:

- Core is the only revision/conflict/delete policy owner.
- Server owns application-service composition, SQL transaction choreography, path locking, object-store coordination, idempotency persistence, operation-log append, and safe API mapping.
- Storage owns passive repositories, migrations, caller-transaction-scoped persistence helpers, advisory-lock primitives, and verified content-addressed object-store primitives.
- Deployment owns host-level service ordering, migration execution, backup destinations, restore targets, release activation, and recovery-helper privileges.
- CLI is an authenticated operator client. It MUST use accepted public Server/operation surfaces and MUST NOT compensate for missing contracts through direct infrastructure access.
- `accept_current`, `keep_both`, and `mark_resolved` remain metadata-only with respect to the authoritative file revision.
- `accept_conflict` creates one new authoritative revision from the already preserved incoming conflict content. The previous current revision remains immutable history and becomes the parent of the new revision.
- Existing V1 changes-feed vocabulary remains valid. Conflict resolution continues to append `conflict_resolved`; this contract does not require a new changes-feed operation kind.
- Object-store blobs are immutable, content-addressed by SHA-256, verified before use, and are not authoritative merely because bytes exist.
- Recovery operations are exact specializations of the Stage 10 operational-control contract: they use `haze-sync.operational-job.v1`, `haze-sync.audit-event.v1`, the canonical maintenance/control generations, and operational idempotency. This document MUST NOT be implemented as a parallel job, idempotency, confirmation, or audit subsystem.
- Ordinary product-data restore MUST preserve the live operational control plane. It MUST NOT overwrite current maintenance/control state, adapter desired/effective state, principals, credentials, operational jobs, operational idempotency records, or audit events.
- `accept_conflict` effects MUST be derived from the current Core planner. The new revision preserves incoming-content provenance through `created_by = conflict.incoming_adapter_id`; the resolving principal is recorded separately as resolver/audit actor; and the conflict-copy disposition is `MarkConflictCopyResolved`.

Materially relevant accepted surfaces include:

- `crates/haze-sync-core/docs/component-contract.md`;
- `crates/haze-sync-core/src/conflict_service/mod.rs`;
- `crates/haze-sync-core/docs/idempotency-operation-log-persistence.md`;
- `crates/haze-sync-storage/docs/component-contract.md`;
- `crates/haze-sync-storage/src/object_store/mod.rs`;
- `crates/haze-sync-server/docs/component-contract.md`;
- `crates/haze-sync-server/src/routes/conflicts_impl.rs`;
- `crates/haze-sync-api/src/dto/conflicts.rs`;
- `crates/haze-sync-cli/docs/component-contract.md`;
- `deploy/docs/migrations-backup-restore.md`.

## 3. Shared operational-control boundary

This document uses the Stage 10 operational-control concepts directly so it remains independently reviewable.

### 3.1 Maintenance states

Recovery and upgrade operations MUST use the canonical lifecycle:

```text
normal -> quiescing -> quiesced -> maintenance -> resuming -> normal
```

The following rules apply:

- Backup capture MUST begin only from `quiesced` or `maintenance` after quiescence evidence is complete.
- Restore, migration, upgrade activation, and rollback restore MUST run only in `maintenance`.
- `accept_conflict` MUST be admitted only in `normal`.
- Entering `quiescing` MUST reject new public writes, new adapter cycles, conflict/delete mutations, and new destructive operational jobs while allowing bounded administrative reads and the work needed to drain or cancel in-flight activity.
- `quiesced` means mutation admission is closed and all admitted mutation work has reached a proven safe boundary.
- `maintenance` preserves the closed mutation gate while deployment-owned recovery work executes.
- `resuming` MUST keep mutation admission closed until post-operation verification succeeds and adapter desired/effective state is reconciled.
- A restart in any state other than `normal` MUST fail closed. It MUST NOT infer that recovery completed or enable adapters merely because the process restarted.

### 3.2 Exact specialization of operational jobs

This contract does not define a second job model. Every backup, restore, upgrade, and rollback operation MUST use the Stage 10 operational-control record with schema `haze-sync.operational-job.v1`. In this document, the prose words `job` and `operation` refer to that same record, and the normative identity field is `operation_id`. Recovery request, helper-result, manifest, and checkpoint records are child evidence referenced by the operational job; they MUST NOT replace it.

The exact V1 kind mapping is:

```text
backup   -> backup
restore  -> restore
upgrade  -> deployment_rollout
rollback -> deployment_rollback
```

A recovery implementation MUST NOT introduce parallel kinds named `upgrade` or `rollback` in the operational-job registry.

The job record MUST use the Stage 10 field vocabulary, including at least:

```text
schema: haze-sync.operational-job.v1
operation_id
kind
requester_principal_id
requester_credential_id | null
idempotency_scope
idempotency_key_digest
request_fingerprint
state
dry_run
confirmation_required
confirmation_digest | null
confirmation_expires_at | null
expected_control_generation
expected_maintenance_generation | null
created_at
updated_at
started_at | null
completed_at | null
cancel_requested_at | null
safe_summary
safe_error_category | null
artifact_manifest_id | null
artifact_manifest_digest | null
checkpoint | null
lease_owner | null
lease_expires_at | null
```

Recovery-specific facts such as backup identity, target identity, release set, helper attempt, artifact checksums, and recovery phase MUST be represented in the bounded child request/result, artifact manifest, or `checkpoint`; they MUST NOT create competing top-level job fields with different lifecycle semantics.

Allowed states and legal transitions are exactly:

```text
create -> planned
planned -> awaiting_confirmation | running | cancelled
awaiting_confirmation -> running | cancelled
running -> succeeded | failed | cancelled
```

A destructive restore or rollback MUST pass through `awaiting_confirmation` before entering `running`. A terminal job is immutable. A job MUST NOT become `succeeded` until every required effect, manifest, checkpoint, and audit outcome is durable and re-readable. A retry after terminal failure creates a linked new `operation_id` with a new idempotency key; it does not reopen or mutate the failed job.

### 3.3 Exact specialization of operational idempotency

Operational recovery idempotency is the Stage 10 operational-idempotency contract, not the existing adapter-write idempotency table and not a third recovery-specific mechanism.

The exact unique scope is:

```text
(requester_principal_id, operation_kind, idempotency_key_digest)
```

`idempotency_key_digest` MUST be HMAC-SHA-256 or a stronger keyed digest using a Server-managed operational-idempotency pepper. The raw key MUST NOT be persisted, logged, audited, copied into helper requests, included in manifests, or exposed publicly.

The request fingerprint MUST include every semantic field that can change the result, including the selected backup, target environment, release set, requested artifact set, expected source/target identity, dry-run state, confirmation requirement, and expected control/maintenance generations. It MUST exclude credentials, confirmation values, raw file bytes, provider payloads, host paths, database URLs, and the raw idempotency key.

For the same operational scope and key digest:

- the same fingerprint MUST return the existing `operation_id` and its current or terminal state;
- a different fingerprint MUST return `idempotency_conflict` and MUST perform no new work;
- concurrent duplicates MUST reserve at most one operational job and MUST NOT start parallel helpers;
- replay of a terminal job MUST return its terminal result and MUST NOT rerun it;
- replay of a non-terminal job MUST return the existing job and MUST NOT create another executor;
- an ambiguous client timeout MUST be recovered by retrying the identical request with the same raw key so it resolves to the same digest and operation.

A new job request with a stale control or maintenance generation MUST be rejected before job creation. Immediately before `planned` or `awaiting_confirmation` enters `running`, the executor MUST revalidate both expected generations; mismatch fails the job as `stale_control_generation` before any external effect.

`accept_conflict` is not an operational recovery job. It continues to use the accepted authenticated adapter/public-write idempotency model and Core replay decision (`NewRequest`, `ReplaySameRequest`, or `ConflictDifferentRequest`) inside its single mutation transaction. Its public different-fingerprint category is also `idempotency_conflict`, but its storage scope remains the accepted adapter-write scope rather than the operational-job digest scope.

### 3.4 Credentials and helper secrets

Operator authentication and helper infrastructure credentials are separate concerns.

- CLI MUST authenticate to Server through the accepted credential mechanism.
- CLI requests MUST contain deployment-configured identifiers such as `backup_destination_id`, `target_environment_id`, and optional opaque `credential_ref`; they MUST NOT contain raw database URLs, passwords, OAuth tokens, vault secrets, provider credentials, or host filesystem paths.
- Server and audit records MUST persist only safe identifiers and references.
- A deployment-owned helper MAY resolve an approved local credential reference from a secrets manager, restricted file, inherited descriptor, or other deployment-owned secret source.
- Secret values MUST NOT be placed in process arguments, shell command strings, environment dumps, helper responses, manifests, logs, or public errors.
- Plaintext secret material MUST NOT cross back to Server or CLI.

### 3.5 Exact specialization of audit

Recovery job events use the Stage 10 append-only schema `haze-sync.audit-event.v1`. Recovery-specific evidence MAY add bounded safe metadata, but MUST NOT create a parallel audit event model or rename the canonical identity fields.

Every operational-job transition and destructive confirmation MUST append an audit event. When job state and audit share one storage authority, they MUST commit atomically. `accept_conflict` MUST append its mutation audit event in the same database transaction as the authoritative revision change.

Recovery audit events use the canonical fields, including:

```text
audit_id
schema_version
occurred_at
event_type
outcome
actor_principal_id | null
actor_credential_id | null
actor_role | null
actor_origin
request_id | null
correlation_id | null
target_type
target_id | null
operation_id | null
control_generation | null
maintenance_generation | null
previous_state | null
next_state | null
safe_error_category | null
artifact_manifest_id | null
safe_metadata
```

Recovery `safe_metadata` MAY contain safe values such as backup ID, target environment ID, release-set ID, conflict/object/revision IDs, manifest digest, bounded counts, result category, and timestamps.

Audit metadata MUST NOT contain file content, conflict bytes, credentials, credential verifier material, token hashes, confirmation digests, raw idempotency keys, raw database URLs, provider payloads, internal SQL, local absolute paths, arbitrary shell text, stack traces, or vault content.

## 4. Canonical recovery execution ownership

The canonical chain is:

```text
CLI
  -> authenticated versioned Server operation request
  -> durable `haze-sync.operational-job.v1` validation and reservation
  -> deployment-owned recovery helper
  -> typed PostgreSQL/object-store/optional-Worktree artifact results
  -> Server job/audit/evidence finalization
  -> safe CLI result
```

### 4.1 CLI ownership

CLI MUST:

- submit only versioned structured requests to an accepted public administrative/operation boundary;
- send an idempotency key through the accepted protected transport field/header;
- render safe job status, confirmation requirements, manifest identity, and result categories;
- require an explicit operator confirmation workflow for destructive operations;
- default planning and inspection commands to no-write behavior.

CLI MUST NOT:

- execute SQL or invoke `pg_dump`, `pg_restore`, migration tools, or database reset commands directly;
- read or mutate object-store files directly;
- access provider-private APIs or private adapter state;
- mutate deployment services directly;
- accept or construct arbitrary shell commands;
- pass secret values as flags, positional arguments, output, or logs;
- claim backup, restore, upgrade, or rollback success from a plan-only response.

### 4.2 Server/operation ownership

Server MUST:

- authenticate and authorize the operator;
- validate the versioned request and named deployment resources;
- enforce maintenance-state admission;
- create/replay the exact Stage 10 operational job under the operational-idempotency contract;
- require and validate destructive confirmation where applicable;
- invoke only an allowlisted helper operation;
- receive typed progress/result evidence;
- persist canonical job and `haze-sync.audit-event.v1` state, or reconcile it from the external recovery-control envelope when the product PostgreSQL target is unavailable;
- expose safe status and result categories.

Server MUST NOT:

- accept an arbitrary executable path, shell fragment, SQL fragment, host path, or provider command from CLI;
- treat helper exit code alone as proof of success;
- mark a job successful before manifest and verification evidence are durable and re-readable;
- enable adapters or resume public mutation automatically after restore, upgrade, or rollback.

### 4.3 Deployment-owned helper

The recovery helper is the only component authorized to perform validated host-level backup/restore work.

The helper MUST be invoked through a fixed deployment configuration and a typed operation enum. Acceptable transports include a fixed executable receiving JSON on standard input or an authenticated local RPC boundary. The transport MUST NOT evaluate a shell command string.

The helper MUST:

- accept only schema-validated V1 requests;
- resolve named destinations/targets through deployment configuration;
- enforce path allowlists and target identity;
- enforce required filesystem permissions;
- run only allowlisted backup, verification, restore, and migration primitives;
- produce typed progress and final result records;
- sanitize all errors before returning them;
- preserve a local detailed log only when it is access-controlled and secret-safe.

The helper MUST NOT:

- execute arbitrary operator-provided shell;
- expose host paths to CLI/public API;
- read provider-private data unless a separate provider-specific contract explicitly authorizes it;
- use automatic destructive fallback;
- infer a successful job from partial artifacts.

## 5. Versioned recovery request and result contracts

All executor requests MUST carry `schema: haze-sync.recovery.request.v1` and all helper results MUST carry `schema: haze-sync.recovery.result.v1`. These are bounded child contracts of one `haze-sync.operational-job.v1`; they do not define another lifecycle, idempotency namespace, or audit authority.

### 5.1 Common request fields

```text
schema
operation_kind
operation_id
requester_principal_id
request_fingerprint
source_environment_id
target_environment_id
backup_destination_id
credential_ref
requested_at
dry_run
expected_maintenance_generation
```

Fields that do not apply MUST be omitted, not populated with secret placeholders.

### 5.2 Operation-specific fields

Backup:

```text
artifact_set: [postgresql, object_store, worktree?]
source_release_set_id
expected_schema_state
expected_object_store_format
worktree_requirement: required | omitted_with_reason | not_configured
```

Restore:

```text
backup_id
manifest_id
expected_manifest_sha256
expected_target_identity
replace_target: false for the initial acceptance path
confirmation_binding
requested_release_set_id
```

Upgrade:

```text
from_release_set_id
to_release_set_id
accepted_backup_id
accepted_manifest_id
migration_plan_id
compatibility_record_id
confirmation_binding
```

Rollback:

```text
failed_upgrade_operation_id
rollback_checkpoint_id
accepted_backup_id
accepted_manifest_id
rollback_release_set_id
expected_target_identity
confirmation_binding
```

### 5.3 Typed helper result

The helper result MUST include:

```text
schema
operation_id
operation_kind
attempt_id
started_at
finished_at
outcome
safe_category
manifest_id
manifest_sha256
artifact_results
verification_results
cleanup_result
release_state
schema_state
adapter_disabled_evidence_id
recovery_checkpoint_id
```

`outcome` is one of `succeeded`, `failed`, or `cancelled`. The helper result is evidence; only Server/recovery reconciliation may transition the canonical operational job. No result may use `succeeded` when a required artifact, checksum, count, cleanup, or verification result is missing.

## 6. Quiescence and maintenance evidence

A backup, restore, migration, upgrade, or rollback MUST NOT begin until a durable quiescence evidence record proves all relevant mutation sources are closed.

The evidence MUST include:

```text
maintenance_generation
maintenance_state
mutation_admission_closed_at
in_flight_public_mutations: 0
in_flight_conflict_delete_mutations: 0
in_flight_operational_mutations: 0
server_worktree_cycle_state
server_worktree_in_flight: 0
server_worktree_effective_mode
server_worktree_last_applied_generation
gdrive_effective_state
gdrive_last_applied_generation
gdrive_last_heartbeat_at
external_gdrive_process_state
obsidian_server_mutation_gate: closed
active_cli_write_jobs: 0
object_store_writer_count: 0
database_writer_count: 0
worktree_external_writer_evidence
captured_at
```

Required interpretation:

- Server-owned Worktree MUST be disabled or stopped, with no in-flight cycle.
- A separately running GDrive adapter MUST report the requested disabled/quiesced generation and MUST then be stopped or held in a state that cannot submit mutations.
- Obsidian remains device-owned; Server maintenance MUST reject its writes. Device-local edits do not mutate the recovery set until later resumption.
- If the Worktree is included, deployment MUST prove no external process or user is writing to the target path during capture/restore. A read-only Server bind alone is insufficient proof.
- Health/readiness/status responses alone are insufficient. Evidence MUST include generation/state and zero-in-flight facts from the actual mutation owners.
- A timeout, stale heartbeat, disconnected adapter, unknown process, or unverifiable external Worktree writer MUST fail the precondition. It MUST NOT be treated as quiesced.

The quiescence record MUST be bound to `operation_id` and invalidated by any later maintenance-generation change or admitted mutation.

Before a destructive PostgreSQL restore, the same evidence MUST also be copied by reference into the external recovery-control envelope defined below. The envelope is not a second job record; it is an out-of-restore-set checkpoint that allows the canonical job and maintenance fence to be reconciled after the target product database is replaced.

## 7. Backup contract

### 7.1 Recovery set

A complete V1 backup contains artifacts from one quiesced/maintenance window:

1. PostgreSQL product metadata and schema/migration state, with an explicit ordinary-restore projection that excludes live operational-control records;
2. the complete Haze Sync object-store artifact;
3. a Worktree artifact when it may contain authoritative or unreplicated content;
4. the backup manifest and safe verification evidence.

A physical PostgreSQL capture MAY include source control-plane rows for separately authorized disaster-recovery evidence, but the manifest MUST identify them as `control_plane_recovery_only`. The ordinary restore path in this contract MUST use a filtered product-data projection or an equivalent isolated schema/database layout and MUST NOT apply those rows to the live target control plane.

A database-only, object-store-only, or Worktree-only artifact MUST NOT be labeled a complete recovery point.

Provider-private data such as Google Drive OAuth material or provider payloads is not part of this backup contract. Durable provider mapping/cursor state already stored in PostgreSQL is included through the product-data projection. Adapter desired/effective control, principal/credential, operational-job/idempotency, maintenance, and audit rows are excluded from ordinary restore.

### 7.2 Backup manifest

The manifest schema is `haze-sync.backup-manifest.v1`.

Required top-level fields:

```text
schema
manifest_id
backup_id
operation_id
status
created_at
capture_started_at
capture_completed_at
source_environment_id
source_instance_fingerprint
source_release_set
source_repository_commit
source_schema_state
source_migration_state
source_object_store_format
source_worktree_state_format
maintenance_generation
quiescence_evidence_id
postgresql_restore_scope
control_plane_exclusion_set
control_plane_snapshot_present
artifacts
verification
safe_source_metadata
```

Attempt-local manifests may use:

```text
staging
failed
```

The canonical final manifest under the public `backup_id` identity MUST use `status = complete`. A `staging` or `failed` manifest MUST remain in the private attempt namespace and MUST NOT be discoverable as a restorable backup. Only the immutable final `complete` manifest is restorable.

Each artifact entry MUST contain:

```text
artifact_id
kind: postgresql | object_store | worktree
required
format
format_version
tool_name
tool_version
relative_artifact_name
size_bytes
sha256
created_at
source_identity
restore_scope
excluded_record_families
item_count
byte_count
selected_item_hashes
readability_verified
checksum_verified
```

The manifest MUST NOT contain:

- credentials, tokens, token hashes, OAuth material, raw database URLs, `.pgpass` content, secret environment values, vault secrets, or provider secrets;
- raw file content or object-store bytes;
- absolute host paths;
- secret-store contents or references that reveal secret material;
- unrestricted helper logs or stack traces.

`relative_artifact_name` is relative to the named backup destination. Public Server/CLI output MUST use manifest/artifact identifiers rather than resolved host paths.

### 7.3 Backup consistency and publication order

The helper MUST:

1. validate the canonical operation, quiescence evidence, destination identity, permissions, capacity, artifact requirements, and control-plane exclusion policy;
2. create a private attempt namespace and write an attempt-local `staging` manifest;
3. capture PostgreSQL metadata and migration state, producing or identifying the ordinary product-data restore projection and the excluded control-plane families;
4. capture the object store from the same unchanged recovery window;
5. capture Worktree data when required;
6. compute SHA-256 and size for every artifact;
7. verify every artifact is readable and its checksum recomputes;
8. collect counts and deterministic selected hashes;
9. publish immutable artifacts into a create-only candidate namespace under the final `backup_id`, without a final manifest;
10. re-read and verify the published candidate artifacts, counts, checksums, selected hashes, permissions, and control-plane exclusion metadata;
11. atomically create the immutable final manifest with `status = complete` as the last publication action;
12. re-read the final manifest and reconcile the canonical operational job before reporting success.

The existence of candidate artifacts without the final `complete` manifest MUST NOT make the backup restorable. The final manifest is the commit marker.

If the final `complete` manifest was durably created but helper response delivery or operational-job persistence becomes unavailable, the backup is complete and the job outcome is uncertain; it MUST NOT be relabeled as a failed backup. Recovery reconciliation MUST re-read the final manifest and artifacts, then transition the existing operational job truthfully. It MUST NOT delete, replace, or downgrade the complete manifest.

PostgreSQL and object-store capture MAY use different tools, but all writers MUST remain quiesced until the final `complete` manifest is durable. Any later integrity failure of a previously complete artifact is a `complete_manifest_inconsistent` recovery incident, not a retroactive failed publication.

### 7.4 Counts and selected hashes

Verification evidence MUST record at least:

- database counts for sync objects, revisions, content blobs, conflicts, tombstones, operation-log rows, and idempotency records;
- highest operation sequence and accepted migration set;
- object-store committed blob count and total bytes;
- a deterministic bounded selection of individual blob hashes and verified sizes;
- Worktree file count, total bytes, and deterministic selected file hashes when Worktree is included.

Selection MUST be reproducible from recorded rules, such as the first and last bounded entries in canonical hash/path order. It MUST NOT depend on nondeterministic directory enumeration.

### 7.5 Partial failure, uncertainty, and cleanup

- A failure before final manifest creation MUST leave no final `complete` manifest.
- Attempt-local `staging` or `failed` manifests and unpublished candidate artifacts MUST remain non-restorable.
- The helper MUST attempt bounded cleanup of temporary database dumps, partial archives, and private attempt files.
- Cleanup failure MUST be reported as `failed_cleanup_required`; it MUST NOT be hidden by the primary failure.
- Candidate artifacts already copied under a create-only backup namespace MAY remain quarantined when safe cleanup cannot be proven; they MUST NOT be reused as a complete backup without a new full verification and a new backup identity.
- A partial artifact MUST NOT be reused merely because its file exists.
- Once a final `complete` manifest exists, later response/job-persistence uncertainty MUST use `publication_outcome_unknown` and reconciliation; it MUST NOT create a contradictory failed manifest or failed-backup claim.
- A complete manifest whose referenced artifacts no longer verify MUST be quarantined as `complete_manifest_inconsistent` and MUST NOT be restored until an explicit integrity decision is recorded.

### 7.6 Overwrite, retry, and filesystem permissions

V1 backup creation MUST be create-only:

- an existing final `backup_id` MUST NOT be overwritten;
- while the canonical operational job is non-terminal, retry with the same operational idempotency key and fingerprint MUST return the same `operation_id` and may resume only a phase proven idempotent from its durable checkpoint using a distinct recorded `attempt_id`;
- after terminal failure, the same key MUST replay that failure; a new execution requires a new key and new `operation_id` linked to the failed operation;
- no retry, helper attempt, or new operation may replace a complete backup;
- replacing a complete backup is not a V1 operation.

For POSIX-style local storage, the helper MUST enforce directory mode `0700` and artifact/manifest mode `0600` or stricter. Equivalent private ACLs are acceptable on other platforms. If privacy cannot be enforced or verified, backup MUST fail.

### 7.7 Retention

Backup retention is owned by Deployment/operator policy, not CLI, Core, adapters, or the backup operation itself.

- The helper MAY report retention classification and age.
- V1 backup creation MUST NOT automatically delete an older complete backup.
- Deletion requires a separate explicitly authorized retention job, accepted policy, confirmation where destructive, and audit evidence.

## 8. Restore contract

### 8.1 Initial acceptance target

The first required V1 acceptance path is restore into a separate empty environment.

A production in-place restore MUST NOT be the first acceptance path and MUST NOT be represented as proven merely because empty-target restore succeeds.

The initial path requires:

```text
source_environment_id != target_environment_id
replace_target = false
empty product-data database/schema target
pre-provisioned live control plane isolated from the product restore set
external recovery-control envelope durable and re-readable
empty object-store target
empty optional Worktree target
adapters disabled
public mutation admission closed
```

### 8.2 Live operational-control-plane preservation

Ordinary product-data restore MUST NOT overwrite, import, roll back, or reactivate any live target record in these Stage 10 control families:

```text
maintenance state and generation
control generation and admission fence
adapter desired/effective state and runtime reports
principals and credentials
operational jobs, leases, confirmations, checkpoints, and operational idempotency
audit events
```

Before destructive PostgreSQL work begins, Deployment MUST create a durable external recovery-control envelope with schema `haze-sync.recovery-control-envelope.v1` in storage outside the PostgreSQL product restore set. The envelope is a checkpoint mirror of the canonical operational job, not an independent job or audit system. It MUST contain at least:

```text
schema
operation_id
operation_kind
target_environment_id
target_identity
request_fingerprint
expected_control_generation
expected_maintenance_generation
confirmation_digest_reference
artifact_manifest_id
artifact_manifest_digest
quiescence_evidence_id
adapter_fence_evidence_id
current_phase
last_completed_checkpoint
created_at
updated_at
```

The envelope MUST NOT contain credentials, raw idempotency keys, raw confirmation values, database URLs, host paths, provider payloads, or file content.

The initial separate-target acceptance path MUST provision the target control plane independently and put it in `maintenance` before product data is restored. The live target control plane MAY be held in a separate database/schema that is excluded from restore, or the helper MAY perform a verified filtered restore that leaves those record families untouched. A blind full-database restore into the live control-plane authority is forbidden.

Source control-plane rows present in a physical backup MUST remain quarantined as `control_plane_recovery_only`. They MUST NOT reopen mutation admission, restore old desired adapter modes, revive revoked credentials, replace the current restore job, erase current idempotency decisions, or rewrite current audit history.

Every destructive restore phase MUST checkpoint the external envelope before the phase starts and after its result is known. If Server or target PostgreSQL is unavailable during replacement, the helper continues from the external envelope and keeps the deployment-level admission/adapter fences closed. When target storage is available again, Server MUST reconcile the same `operation_id` and append truthful audit outcomes; it MUST NOT create a replacement job.

Restoring the control plane itself requires a separate explicitly confirmed control-plane recovery procedure whose own authority remains outside the restore set. That procedure is not the ordinary restore path and is not accepted by this Issue.

### 8.3 Restore preflight

Before confirmation, Server/helper MUST validate:

- the canonical operation is in `maintenance` with current quiescence evidence;
- the external recovery-control envelope exists, matches the same operation/target/generations, and is durable and re-readable;
- the backup manifest exists, is `complete`, and matches `expected_manifest_sha256`;
- all required artifacts exist, are readable, and recompute to manifest checksums;
- the PostgreSQL restore scope excludes every live control-plane family listed above;
- source control-plane rows, when physically present, are marked `control_plane_recovery_only` and cannot be selected by the ordinary restore plan;
- artifact formats/tool versions are supported;
- the target identity matches `expected_target_identity`;
- the target product-data area is separate and empty for the initial acceptance path while the isolated live control plane remains intact;
- the target has sufficient capacity and enforceable private permissions;
- the requested release set is compatible with the backup schema, migration state, object-store format, Worktree state format, and live control-plane schema;
- adapters and external writers are disabled/stopped by the live target control plane and deployment fence;
- no destructive step has executed before confirmation.

Any failure MUST leave the canonical job in a safe non-running state, preserve the live control plane and external envelope, and perform no target product-data mutation.

### 8.4 Destructive confirmation

Confirmation MUST be a durable, single-use decision bound to:

```text
operation_id
operation_kind
backup_id
manifest_sha256
target_environment_id
target_identity
release_set_id
request_fingerprint
confirmation_expires_at
confirming_principal_id
```

A generic `--yes`, an unbound boolean, or confirmation from a different job/target/manifest MUST NOT authorize restore.

Confirmation MUST expire and MUST be invalidated by target-identity, manifest, release-set, maintenance-generation, or request-fingerprint changes.

### 8.5 Restore order

After confirmation, the helper MUST execute this order:

1. revalidate the canonical operation, external recovery-control envelope, maintenance/control generations, target identity, product-data emptiness/replaceability, artifact checksums, control-plane exclusion plan, and confirmation binding;
2. ensure Server and all adapters remain stopped or forced into recovery validation mode under deployment-level fences;
3. checkpoint `postgresql_restore_starting` in the external envelope and record a digest of the still-live target control-plane state;
4. provision the target product-data database/schema area without accepting application writes and without replacing the isolated live control plane;
5. restore only the PostgreSQL product-data projection;
6. verify that every excluded live control-plane family is unchanged and that no source control-plane row became active;
7. validate restored product schema and migration state;
8. restore the matching object-store artifact;
9. verify object-store counts, archive checksum, and selected blob hashes;
10. restore Worktree data when the manifest requires it;
11. verify Worktree counts and selected hashes when applicable;
12. while application writers remain stopped, apply only explicitly compatible sequential forward migrations required by the selected release, including separately controlled live-control-plane migrations when required by that release;
13. validate resulting product and control schema/migration state before application startup;
14. start the compatible Server release in recovery validation mode using the preserved live target control plane, with mutation admission closed and all adapters forced disabled regardless of source backup values;
15. reconcile the same `operation_id` from the external envelope, then run object-store consistency checks, doctor, and preflight;
16. record a rollback/resume checkpoint in both the canonical job and external envelope;
17. wait for explicit operator acceptance before entering `resuming`;
18. reconcile approved target desired/effective adapter state and open mutation admission only after all gates succeed.

Recovery validation mode is deployment/Server fail-closed authority. Restored product rows and quarantined source control-plane rows MUST NOT be able to auto-enable adapters, revive credentials, replace the current job, alter maintenance generation, or open public mutations before verification.

### 8.6 Restore failure behavior

On any failure:

- target mutation admission MUST remain closed under the preserved live control plane and deployment fence;
- adapters MUST remain disabled;
- the external recovery-control envelope MUST remain authoritative for the current phase/checkpoint until the canonical job is reconciled;
- the target product-data area MUST be marked quarantined/incomplete;
- source control-plane rows MUST remain inactive and MUST NOT replace target control records;
- the canonical job MUST record, or later reconcile, the last completed phase and safe failure category under the same `operation_id`;
- automatic retry MUST resume only from a phase proven idempotent by both the external envelope and durable artifact evidence;
- automatic destructive cleanup or fallback MUST NOT occur;
- cleanup of the failed separate target requires a distinct authorized cleanup action or operator-owned environment disposal;
- the source/original environment MUST remain untouched by the empty-target acceptance path.

Loss or overwrite of the external envelope, inability to prove live control-plane preservation, or disagreement between the envelope and canonical job MUST fail closed as `recovery_control_state_uncertain`. No resume, retry, or cleanup may infer the missing state.

A restore MUST NOT combine artifacts from different backup IDs or recovery windows.

## 9. Upgrade contract

### 9.1 Preconditions

Upgrade MUST NOT begin unless all of the following are true:

- current release, schema, migration state, object-store format, and adapter protocol state are known;
- the target release compatibility record is accepted;
- a complete backup of the current environment has been captured and verified under this contract;
- the backup is bound to the rollout `operation_id` and its manifest checksum is recorded;
- quiescence evidence is current;
- all adapters are effectively disabled/stopped at the requested generation;
- the operator has confirmed the target release and rollback backup;
- the migration plan contains an ordered forward-only sequence.

### 9.2 Ordering

The upgrade sequence is:

1. `normal -> quiescing` and close new mutation admission;
2. drain/cancel in-flight work to a proven safe boundary;
3. reach `quiesced` and capture quiescence evidence;
4. enter `maintenance`;
5. create and verify the mandatory backup;
6. stop external adapters and the Server processes that could write;
7. verify the pre-upgrade rollback checkpoint and external recovery-control envelope;
8. apply migrations sequentially in repository order;
9. activate the complete compatible release set;
10. start Server in maintenance/recovery validation mode with adapters disabled;
11. verify binary startup, schema/migration state, object-store compatibility, Worktree state compatibility, doctor, and preflight;
12. record the post-upgrade checkpoint;
13. enter `resuming` only after explicit operator acceptance;
14. restore approved desired adapter state, verify effective state, then enter `normal`.

A release set includes every mutually constrained binary/protocol artifact required for safe operation, including Server, helper/migration tooling, CLI compatibility, and applicable adapter protocol versions. Partial activation MUST NOT be accepted as a successful upgrade.

### 9.3 Migration rules

- Migrations MUST be applied one at a time in canonical sequence.
- Each migration MUST complete transactionally when PostgreSQL supports the operation.
- A failed migration MUST stop the sequence immediately.
- A migration MUST NOT be skipped, reordered, silently marked applied, or replaced by a database reset.
- Reverse migrations are not the rollback mechanism.
- The canonical rollout job and external recovery-control envelope MUST record the exact pre-state, attempted migration, observed post-state, and transaction result without raw SQL or database URLs.

### 9.4 Failure boundaries

Migration failure:

- stop further migrations;
- remain in `maintenance`;
- verify whether the failed migration transaction rolled back and whether schema state equals the pre-upgrade checkpoint;
- do not start the new release unless compatibility with the observed schema is explicitly accepted.

Binary startup failure:

- keep all mutation admission closed and adapters disabled;
- retain the observed schema and release evidence;
- do not fall back by merely starting an arbitrary old binary against an unknown schema.

Schema mismatch:

- fail closed;
- do not mark migration state manually;
- require either an evidence-backed correction compatible with the current checkpoint or rollback restore.

Object-store mismatch or unreadable committed blobs:

- fail closed;
- do not create, delete, or rewrite authoritative metadata to hide the mismatch;
- require diagnosis or rollback restore.

Post-upgrade doctor/preflight failure:

- do not enter `resuming`;
- keep adapters disabled;
- record the failed check categories and rollback checkpoint.

## 10. Rollback contract

Rollback is coordinated restoration of an accepted backup plus a compatible release set. It is not speculative reverse migration.

### 10.1 Rollback decision point

The exact upgrade rollback decision point is after any upgrade gate fails and before `resuming` opens mutation admission or enables adapters.

At that point the `deployment_rollout` job MUST record the failed gate and transition to `failed` without reopening mutation admission. The durable rollback checkpoint below MUST be persisted in that failed operation and the external recovery-control envelope. If the operator selects rollback, Server MUST create a linked `deployment_rollback` job in `planned`; that new job MUST enter `awaiting_confirmation` before any rollback restore begins:

```text
failed_upgrade_operation_id
failed_phase
pre_upgrade_backup_id
pre_upgrade_manifest_sha256
pre_upgrade_release_set_id
observed_release_set_id
pre_upgrade_schema_state
observed_schema_state
target_environment_identity
maintenance_generation
adapter_disabled_evidence_id
doctor_preflight_summary
external_recovery_control_envelope_id
```

The operator MUST explicitly choose one of:

- retry a bounded non-destructive verification/correction step under a linked operation;
- resume the original release without restore, but only when the original data/schema/control state is proven unchanged and compatible;
- execute a linked `deployment_rollback` restore from the accepted backup and compatible release set;
- leave the environment in maintenance for investigation.

No default choice is implied by timeout or process restart. The failed rollout job MUST NOT itself transition back to `awaiting_confirmation` or `running`.

### 10.2 Rollback execution

Rollback restore MUST follow the restore contract, including manifest/checksum validation, confirmation binding, adapter-disabled recovery validation mode, ordered artifact restore, sequential compatible migrations if required, doctor/preflight, and explicit resumption.

The rollback release set MUST be compatible with the restored backup schema and formats. An old binary MUST NOT be started against a newer schema merely because it was the previous binary.

### 10.3 Resuming the original environment

The original environment MAY resume without restore only when evidence proves all of the following:

- no destructive restore or partial target replacement occurred;
- the database schema/migration state equals a release-compatible checkpoint;
- object-store identity, counts, and selected hashes remain valid;
- required Worktree state remains valid;
- no mutation was admitted during maintenance;
- the selected release set is compatible with the observed state;
- doctor and preflight succeed with adapters still disabled;
- the operator explicitly approves resumption.

Otherwise, the environment MUST remain in maintenance until rollback restore or another accepted recovery plan succeeds.

If mutation admission had already reopened after an upgrade, returning to the old backup is a new destructive recovery incident, not the bounded pre-resume rollback path. New post-upgrade data MUST be assessed before any restore.

## 11. Recovery acceptance evidence

Later R6-4 recovery acceptance MUST retain a durable evidence bundle containing at least:

```text
operation IDs, helper attempt IDs, and kind mappings
source and target environment identities
backup and manifest ids
manifest SHA-256
artifact SHA-256 values
artifact sizes and readability results
database table counts
object-store blob count and byte count
Worktree file count and byte count when applicable
deterministic selected blob/file hashes
highest operation-log sequence
schema and applied migration state
source, target, upgrade, and rollback release-set identities
maintenance generation and quiescence evidence
adapter desired/effective disabled state and generations
doctor result
preflight result
restore/upgrade phase checkpoints
rollback decision checkpoint
operator confirmation audit IDs
external recovery-control envelope identity and checkpoints
live target control-plane before/after digests and excluded record families
proof that source control-plane rows remained inactive
final resumption decision
```

Evidence MUST distinguish `not run`, `skipped`, `failed`, and `passed`. Missing evidence MUST NOT be represented as success.

## 12. Transactional `accept_conflict`

### 12.1 Purpose and admission

`accept_conflict` promotes the preserved incoming conflict content to one new authoritative revision at the original object path.

It MUST:

- be admitted only in maintenance state `normal`;
- require an authenticated principal authorized to resolve conflicts;
- use a durable idempotency key;
- use explicit optimistic preconditions;
- invoke the current Core `plan_conflict_resolution` policy with `AcceptConflict` after locks are acquired and persist effects that match its returned plan;
- preserve the previous authoritative revision and all older revision history;
- append operation and audit evidence atomically with the resolution;
- leave the conflict open when any required step fails before commit.

It MUST NOT:

- mutate provider state directly;
- replace bytes in place under an existing revision;
- delete the previous current revision or its blob;
- trust client-supplied bytes without hash/size verification;
- mark the conflict resolved before the new authoritative revision and object pointer are durable;
- expose conflict content, SQL, object-store paths, or host paths in results/errors.

### 12.2 Versioned request

The service/API request contract is `haze-sync.accept-conflict.request.v1`.

Required semantic fields are:

```text
schema
conflict_id
resolution: accept_conflict
expected_current_revision_id
expected_incoming_revision_id
expected_content_sha256
expected_size_bytes
```

The protected transport MUST also provide the authenticated principal and idempotency key.

The expected incoming revision/hash/size bind the operator action to the exact preserved content that was reviewed. A request that omits these fields MUST be rejected as `precondition_required` and MUST not mutate state.

The current conflict-resolve route MAY be extended conditionally for `accept_conflict` or a dedicated versioned operation route MAY be introduced by the API owner. In either case, the required semantics and failure categories in this document are mandatory. Metadata-only actions MUST continue to accept their existing action-only request semantics.

### 12.3 Request fingerprint

The fingerprint MUST include:

```text
operation = accept_conflict
conflict_id
expected_current_revision_id
expected_incoming_revision_id
expected_content_sha256
expected_size_bytes
authenticated principal scope
```

It MUST exclude raw content bytes, bearer credentials, the raw idempotency key, database details, and object-store paths.

### 12.4 Object-store preparation and authority rule

The conflict's `incoming_revision_id` MUST reference immutable preserved content metadata.

Before object-store verification, Server MUST perform a non-mutating lookup under the accepted adapter-write idempotency scope. When the same scope/key already has a committed response with the same fingerprint, Server MUST replay it immediately without requiring object-store availability. A different fingerprint MUST fail with `idempotency_conflict`. Absence of a record is only a preliminary observation; the authoritative transaction MUST lock and re-evaluate idempotency to resolve races.

When no committed replay exists, before opening the authoritative database transaction Server MUST verify one of these states:

1. the referenced content-addressed blob is already committed, readable, and matches the expected SHA-256 and size; or
2. when an accepted future boundary supplies bytes, the bytes are written to object-store staging, verified, and atomically finalized under the immutable expected hash before any authoritative database pointer is changed.

Object-store finalization MUST be create-or-verify and idempotent by content hash. It MUST NOT overwrite different bytes.

A finalized blob without a committed database revision is non-authoritative. Database rollback or commit failure MAY leave an unreferenced immutable blob, but MUST NOT leave a falsely authoritative revision. Such blobs are handled by a separate retention/consistency policy, never by automatic deletion in this mutation.

### 12.5 Lock order and atomic transaction

After object-store verification, Server MUST execute one database transaction with this lock order:

1. acquire a transaction-scoped idempotency lock for the authenticated scope/key;
2. evaluate any existing idempotency record;
3. read the conflict once to derive its normalized original path, then acquire the standard transaction-scoped path advisory lock;
4. load the conflict row `FOR UPDATE` and require status `open`;
5. load and lock the current object row and current revision state;
6. load the incoming conflict revision and its content metadata;
7. perform all precondition and consistency validation and invoke Core `plan_conflict_resolution` with the locked open conflict and `AcceptConflict`;
8. require the Core plan to specify `CreateCurrentRevisionFromConflict` and `MarkConflictCopyResolved`, then insert one immutable new file revision exactly from `NewCurrentRevisionPlan`;
9. update `sync_objects.current_revision_id` to the new revision;
10. append the existing V1 `conflict_resolved` operation-log entry with `conflict_id` and the new `revision_id`;
11. append the safe `conflict.accept_conflict.succeeded` audit event;
12. mark the conflict resolved and persist the Core `MarkConflictCopyResolved` disposition, or a transactionally created idempotent materialization work item that encodes that disposition, together with resolver identity and resolution timestamp;
13. persist the safe idempotency response;
14. commit.

All steps from lock acquisition through idempotency-response persistence MUST use the same SQL transaction. The transaction MUST commit only when every required database step succeeds.

The conflict row is reloaded after the path lock because the initial non-locking read is only a path lookup. The locked row is authoritative.

### 12.6 Required validation

Before inserting the new revision, Server MUST verify:

- the conflict exists and is still open;
- its original path matches the locked object path;
- the locked object's current revision equals `expected_current_revision_id`;
- the conflict's recorded current revision and the actual current revision are consistent, or the request fails stale rather than overwriting newer content;
- the conflict's incoming revision equals `expected_incoming_revision_id`;
- the incoming revision/content metadata is the preserved incoming side identified by the conflict and matches the original path expected by the Core planner;
- the incoming revision hash and size equal the conflict metadata and request preconditions;
- the committed object-store blob recomputes to the expected hash and size;
- the Core plan parent revision, content hash, size, `created_by`, and conflict-copy disposition exactly match the locked conflict facts;
- the principal remains authorized and maintenance admission remains `normal` for the bound control generation.

### 12.7 Core-planner revision provenance and conflict-copy effects

The new revision MUST be created from the current Core `NewCurrentRevisionPlan` without reinterpretation:

```text
new revision id = Server-assigned immutable ID
same object id = locked current object
path = conflict.original_path
parent_revision_id = conflict.current_revision_id = previously current revision id
content_sha256 = conflict.incoming_content_hash
size_bytes = conflict.incoming_size_bytes
created_by = conflict.incoming_adapter_id
created_at = Server transaction time
```

`created_by` records the provenance of the accepted content and MUST NOT be replaced with the authenticated resolver identity. The authenticated resolver remains authoritative for `conflicts.resolved_by`, the resolution audit actor, authorization evidence, and request/idempotency scope.

The previous current revision MUST remain unchanged and queryable as history. The object current pointer MUST change exactly once to the new revision.

For `accept_conflict`, the Core planner's exact conflict-copy disposition is `MarkConflictCopyResolved`:

- the materialized conflict copy MUST no longer be represented as an active kept-both side after successful resolution;
- the disposition MUST be durably attached to the committed resolution result or a transactionally created idempotent post-commit materialization work item;
- physical adapter/Worktree handling MAY occur after database commit, but it MUST apply `MarkConflictCopyResolved`, MUST NOT reinterpret it as `KeepMaterializedConflictCopy` or `NoConflictCopyChange`, and MUST NOT delete authoritative revision/blob history;
- a later materialization failure is a safe degraded follow-up state and MUST NOT create another authoritative revision, change the revision provenance, or silently reopen the committed conflict.

The operation-log entry MUST use `conflict_resolved`, reference the conflict and new revision, and receive the globally monotonic sequence only inside the transaction. The audit event MUST identify the action, conflict, object, old current revision, new current revision, incoming content source adapter, resolver actor, and safe result category. It MUST not contain content or raw paths beyond the accepted normalized vault-path policy.

The conflict MUST be marked resolved only after the new revision, object pointer, operation log, audit event, Core conflict-copy disposition, and idempotency response have all been written successfully in the transaction.

### 12.8 Concurrency and replay

Concurrent resolution of the same conflict:

- the path/conflict locks MUST serialize attempts;
- exactly one attempt may commit;
- a later different-key attempt observes resolved state and returns `conflict_already_resolved`;
- the authoritative revision MUST not be duplicated.

Stale expected current revision:

- return `stale_current_revision`;
- include only safe expected/actual revision identifiers when public policy permits;
- leave the conflict open and state unchanged.

Same idempotency key and same fingerprint:

- replay the committed safe response, including the same new revision id and operation sequence;
- perform no object-store or database effect again.

Same idempotency key and different fingerprint:

- return `idempotency_conflict`;
- perform no mutation.

Already-resolved replay with the original key:

- idempotency replay wins and returns the original success.

Already-resolved request with a new key:

- return `conflict_already_resolved`;
- do not infer that the caller requested the same historical action unless durable audit metadata proves it.

### 12.9 Failure behavior

Missing conflict or object:

- return `conflict_not_found` or `object_not_found` as appropriate;
- perform no mutation.

Missing incoming revision/blob:

- return `conflict_content_unavailable`;
- leave the conflict open;
- do not create a new revision or alter the object pointer.

Invalid hash or size:

- return `conflict_content_mismatch`;
- leave state unchanged;
- do not expose bytes.

Object-store read/verification failure:

- return `storage_unavailable` or `conflict_content_unavailable` according to whether the failure is transient or an integrity absence;
- do not begin or commit authoritative database effects.

Database/repository failure before commit:

- roll back the entire transaction;
- leave the conflict open and current revision unchanged;
- return a sanitized retryable/non-retryable category.

Commit outcome ambiguity:

- return `commit_outcome_unknown` with instruction to retry the identical request using the same idempotency key;
- MUST NOT advise a new key;
- the retry MUST resolve through the durable idempotency record if commit succeeded.

### 12.10 Metadata-only action compatibility

The exact current Core dispositions are:

```text
accept_current  -> MetadataOnlyCurrentUnchanged + MarkConflictCopyResolved
accept_conflict -> CreateCurrentRevisionFromConflict + MarkConflictCopyResolved
keep_both       -> MetadataOnlyCurrentUnchanged + KeepMaterializedConflictCopy
mark_resolved   -> MetadataOnlyCurrentUnchanged + NoConflictCopyChange
```

The accepted metadata-only actions retain these semantics:

`accept_current`:

- current revision unchanged;
- no new file revision;
- conflict resolved;
- Core disposition is `MarkConflictCopyResolved`; the conflict copy MUST be marked resolved/superseded according to accepted materialization behavior;
- `conflict_resolved` operation and audit evidence recorded.

`keep_both`:

- current revision unchanged;
- no new file revision;
- materialized conflict copy remains the second copy;
- conflict resolved;
- `conflict_resolved` operation and audit evidence recorded.

`mark_resolved`:

- current revision unchanged;
- no new file revision;
- no required conflict-copy materialization change;
- conflict metadata closed;
- `conflict_resolved` operation and audit evidence recorded.

Implementations MAY strengthen transaction, lock, idempotency, and audit integrity for these actions, but MUST NOT silently convert them into content mutation or revision creation.

### 12.11 Safe result and error categories

The service-layer success result contract is `haze-sync.accept-conflict.result.v1` and contains only:

```text
schema
outcome: accepted | replayed
conflict_id
resolution: accept_conflict
previous_current_revision_id
new_current_revision_id
operation_seq
audit_event_id
replayed
```

Public API mapping MUST provide stable categories equivalent to:

| Category | Meaning | Recommended HTTP class |
|---|---|---|
| `accepted` | transaction committed | 200 |
| `replayed` | same-key committed result replayed | 200 |
| `precondition_required` | required expected values absent | 422 |
| `invalid_request` | malformed identifier/hash/size/action | 400 or 422 |
| `conflict_not_found` | conflict does not exist | 404 |
| `object_not_found` | referenced object is absent | 409 |
| `conflict_already_resolved` | new-key attempt after resolution | 409 |
| `stale_current_revision` | current revision changed | 409 |
| `idempotency_conflict` | same idempotency scope/key, different request fingerprint | 409 |
| `conflict_content_unavailable` | preserved revision/blob absent or unreadable | 409 |
| `conflict_content_mismatch` | hash/size/precondition mismatch | 409 or 422 |
| `maintenance_rejected` | mutation admission is not `normal` | 409 or 503 |
| `storage_unavailable` | transient storage dependency failure | 503 |
| `commit_outcome_unknown` | client must retry same key | 503 |
| `internal_error` | sanitized non-specific failure | 500 |

Errors MUST NOT include file content, credentials, raw idempotency keys, internal SQL, SQLx errors, provider payloads, object-store paths, database URLs, host paths, or stack traces.

## 13. Downstream responsibilities

### 13.1 Storage

Storage follow-up MUST provide:

- the exact Stage 10 repositories for `haze-sync.operational-job.v1`, operational idempotency, confirmation bindings, `haze-sync.audit-event.v1`, and recovery evidence;
- lock/read/update primitives needed by atomic `accept_conflict`, including conflict and object/current-revision locking under caller transactions;
- immutable revision insertion and current-pointer update composition under one caller transaction;
- idempotency uniqueness and transaction-safe replay persistence;
- backup/doctor fact providers for schema state, counts, operation sequence, blob metadata, selected-hash verification, and a verified ordinary-restore projection that excludes live control-plane families;
- migration compatibility evidence;
- no automatic hard-delete or orphan-blob cleanup as part of `accept_conflict`.

### 13.2 API

API follow-up MUST define:

- versioned operation/job request, confirmation, status, manifest-summary, and safe error DTOs;
- conditional precondition fields or a dedicated versioned route for `accept_conflict`;
- safe result mapping that preserves existing metadata-only action semantics;
- no raw content, credentials, provider-private state, internal paths, SQL, or helper diagnostics.

### 13.3 Server

Server follow-up MUST implement:

- maintenance-state admission and quiescence evidence collection using the exact Stage 10 control generations;
- exact Stage 10 operational-job/idempotency/confirmation/audit choreography without a parallel recovery model;
- allowlisted helper invocation and result validation;
- recovery validation startup mode with forced adapter disablement, preserved live target control-plane authority, and reconciliation from the external recovery-control envelope;
- atomic `accept_conflict` application service using the lock and transaction order above;
- exact safe error/result mapping and ambiguous-commit retry behavior.

### 13.4 CLI

CLI follow-up MUST implement:

- job planning, submission, polling, confirmation, and safe evidence display through public Server APIs;
- same-key retry guidance;
- explicit target/manifest/release confirmation display;
- no direct SQL, object-store, provider, host-service, or shell access;
- no raw secret arguments or logs.

### 13.5 Deployment helper

Deployment follow-up MUST implement:

- the fixed typed helper interface and named resource resolution;
- private staging, create-only candidate publication, final-manifest commit-marker semantics, permissions, checksums, readability verification, uncertainty reconciliation, and bounded cleanup;
- PostgreSQL/object-store/optional-Worktree capture and restore with a filtered/isolated product-data PostgreSQL restore that cannot overwrite the live control plane;
- sequential migration invocation and exact result evidence;
- recovery validation launch/stop ordering and the durable external recovery-control envelope outside the restore set;
- no arbitrary shell or unapproved path access.

### 13.6 Worktree

Worktree follow-up MUST provide:

- truthful stop/quiesce/effective-state evidence;
- no in-flight cycle evidence;
- optional artifact inclusion rules based on authority/unreplicated-content status;
- deterministic restore verification counts and selected hashes;
- disabled startup until post-restore acceptance.

### 13.7 GDrive

GDrive follow-up MUST provide:

- desired/effective disabled or quiesced generation handling;
- heartbeat and last-applied-generation evidence;
- stop/restart behavior that cannot submit writes during maintenance;
- no OAuth/provider-secret inclusion in manifests, jobs, helper requests, or audit;
- no claim that provider contents are part of the Haze Sync coordinated backup unless a separate provider-backup contract is accepted.

### 13.8 Acceptance CI

Acceptance CI follow-up MUST prove:

- backup to a private staging destination and final complete manifest;
- checksum/readability/count/selected-hash evidence;
- partial-failure cleanup, non-restorable staging/candidate state, manifest-last publication, and post-publication uncertainty reconciliation;
- idempotent retry and overwrite refusal;
- restore into a separate environment with an empty product-data target and a pre-provisioned isolated live control plane;
- live maintenance/job/idempotency/audit/credential/adapter-control records preserved across PostgreSQL product-data restore, with source control-plane rows unable to reactivate;
- schema/migration/object-store/Worktree verification;
- doctor and preflight result handling;
- failed migration/startup/schema/object/doctor gates remain in maintenance;
- rollback from accepted backup and compatible release set;
- crash/failure injection around backup artifact publication/final manifest creation, external recovery-envelope checkpoints, PostgreSQL restore, object-store finalization, revision insert, current-pointer update, operation/audit append, idempotency persistence, and commit;
- concurrent `accept_conflict`, stale revision, replay, missing content, hash mismatch, storage failure, ambiguous commit behavior, exact `created_by = incoming_adapter_id`, resolver separation, and `MarkConflictCopyResolved` disposition;
- absence of executable changes from this documentation Candidate.

## 14. V1 safety invariants

1. No recovery operation begins without current quiescence evidence.
2. No destructive restore/rollback begins without a bound durable confirmation.
3. CLI never becomes an infrastructure shell, SQL client, object-store client, or provider-private client.
4. A complete backup is one manifest-bound recovery set from one quiesced window, committed by an immutable final manifest created last.
5. Partial or candidate artifacts are never restorable merely because files exist; a durable complete manifest cannot coexist with a failed-publication claim and requires reconciliation on uncertain delivery.
6. The initial restore acceptance target is separate with an empty product-data area; its live control plane and external recovery envelope remain outside the restore set.
7. Ordinary restore never overwrites maintenance/control generations, adapter control, principals/credentials, operational jobs/idempotency, or audit.
8. Adapters remain disabled until post-operation doctor/preflight and explicit resume.
9. Rollback restores an accepted backup and compatible release set; it does not guess reverse migrations.
10. Previous authoritative revisions remain immutable history.
11. Object-store bytes are non-authoritative until referenced by a committed database revision/current pointer.
12. `accept_conflict` is one atomic database transaction after verified immutable content preparation and exactly preserves Core planner provenance/disposition.
13. Idempotency replay returns the original effect; key reuse with a different fingerprint performs no work.
14. Metadata-only conflict actions remain metadata-only with their exact Core conflict-copy dispositions.
15. Public output, manifests, audit, and logs remain free of secrets, content, internal SQL, provider payloads, database URLs, and host paths.
16. Missing or skipped evidence is never reported as success.
