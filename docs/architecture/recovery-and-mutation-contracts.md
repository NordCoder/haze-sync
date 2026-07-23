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

### 3.2 Operational jobs

Backup, restore, upgrade, and rollback MUST be durable operational jobs with a versioned record containing at least:

```text
schema_version
job_id
kind
requester_principal_id
idempotency_scope
idempotency_key_hash_or_reference
request_fingerprint
state
dry_run
confirmation_required
confirmation_state
created_at
updated_at
started_at
completed_at
safe_summary
artifact_manifest_id
recovery_checkpoint
failure_category
```

The raw idempotency key MUST NOT appear in public output, audit output, logs, metrics, or helper arguments.

Legal states are:

```text
planned
awaiting_confirmation
running
succeeded
failed
cancelled
```

The relevant legal transitions are:

```text
planned -> awaiting_confirmation | running | cancelled
awaiting_confirmation -> running | cancelled
running -> succeeded | failed | cancelled
```

A destructive restore or rollback MUST pass through `awaiting_confirmation` before entering `running`. A terminal job MUST NOT transition back to a non-terminal state. A job MUST NOT transition to `succeeded` until its manifest/evidence is durable and re-readable. A retry after terminal failure is a linked new attempt/job identity under the same idempotency decision, not a reversal of the failed job state.

### 3.3 Idempotency

The idempotency scope for operator operations is the exact tuple:

```text
(authenticated_principal_id, operation_kind, idempotency_key)
```

The request fingerprint MUST include every semantic field that can change the result, including the selected backup, target environment, release set, requested artifact set, expected source/target identity, dry-run state, and confirmation binding. It MUST exclude credentials, raw file bytes, provider payloads, host paths, database URLs, and the idempotency key itself.

For the same scope and key:

- the same fingerprint MUST replay the durable job/result;
- a different fingerprint MUST return `idempotency_key_reused` and MUST perform no new work;
- concurrent duplicates MUST serialize on one durable job identity;
- an ambiguous client timeout MUST be recovered by retrying the same request with the same key, not by inventing a new operation.

### 3.4 Credentials and helper secrets

Operator authentication and helper infrastructure credentials are separate concerns.

- CLI MUST authenticate to Server through the accepted credential mechanism.
- CLI requests MUST contain deployment-configured identifiers such as `backup_destination_id`, `target_environment_id`, and optional opaque `credential_ref`; they MUST NOT contain raw database URLs, passwords, OAuth tokens, vault secrets, provider credentials, or host filesystem paths.
- Server and audit records MUST persist only safe identifiers and references.
- A deployment-owned helper MAY resolve an approved local credential reference from a secrets manager, restricted file, inherited descriptor, or other deployment-owned secret source.
- Secret values MUST NOT be placed in process arguments, shell command strings, environment dumps, helper responses, manifests, logs, or public errors.
- Plaintext secret material MUST NOT cross back to Server or CLI.

### 3.5 Audit

Every job transition and destructive confirmation MUST append an audit event. `accept_conflict` MUST append a mutation audit event in the same database transaction as the authoritative change.

Audit metadata MUST be limited to safe values such as:

```text
audit_event_id
actor_principal_id
event_type
job_id
operation_kind
backup_id
target_environment_id
release_set_id
conflict_id
object_id
revision_id
manifest_id
safe result category
timestamps
```

Audit metadata MUST NOT contain file content, conflict bytes, credentials, token hashes, raw idempotency keys, raw database URLs, provider payloads, internal SQL, local absolute paths, stack traces, or vault content.

## 4. Canonical recovery execution ownership

The canonical chain is:

```text
CLI
  -> authenticated versioned Server operation request
  -> durable operational job and policy validation
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
- create/replay the operational job under the idempotency contract;
- require and validate destructive confirmation where applicable;
- invoke only an allowlisted helper operation;
- receive typed progress/result evidence;
- persist safe job and audit state;
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

All requests MUST carry `schema: haze-sync.recovery.request.v1` and all helper results MUST carry `schema: haze-sync.recovery.result.v1`.

### 5.1 Common request fields

```text
schema
operation_kind
job_id
requester_principal_id
idempotency_scope_id
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
failed_upgrade_job_id
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
job_id
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

`outcome` is one of `succeeded`, `failed`, or `cancelled`. No result may use `succeeded` when a required artifact, checksum, count, cleanup, or verification result is missing.

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

The quiescence record MUST be bound to the operational job and invalidated by any later maintenance-generation change or admitted mutation.

## 7. Backup contract

### 7.1 Recovery set

A complete V1 backup contains artifacts from one quiesced/maintenance window:

1. PostgreSQL metadata and schema/migration state;
2. the complete Haze Sync object-store artifact;
3. a Worktree artifact when it may contain authoritative or unreplicated content;
4. the backup manifest and safe verification evidence.

A database-only, object-store-only, or Worktree-only artifact MUST NOT be labeled a complete recovery point.

Provider-private data such as Google Drive OAuth material or provider payloads is not part of this backup contract. Durable provider mapping/cursor state already stored in PostgreSQL is included through the database artifact.

### 7.2 Backup manifest

The manifest schema is `haze-sync.backup-manifest.v1`.

Required top-level fields:

```text
schema
manifest_id
backup_id
job_id
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
artifacts
verification
safe_source_metadata
```

`source_instance_fingerprint` MUST be a deployment-generated, non-secret recovery identity. It MUST NOT expose a raw host path, database URL, credential-derived value, or non-public Worktree root fingerprint.

`status` is one of:

```text
staging
complete
failed
```

Only `complete` is restorable.

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

### 7.3 Backup consistency and order

The helper MUST:

1. validate the job, quiescence evidence, destination identity, permissions, capacity, and artifact requirements;
2. create a private staging directory for the attempt;
3. capture PostgreSQL metadata and migration state;
4. capture the object store from the same unchanged recovery window;
5. capture Worktree data when required;
6. compute SHA-256 and size for every artifact;
7. verify every artifact is readable and its checksum recomputes;
8. collect counts and deterministic selected hashes;
9. write a `complete` manifest only after all required checks succeed;
10. atomically publish the staging set under the final `backup_id` identity;
11. re-read the final manifest and required artifacts before reporting success.

PostgreSQL and object-store capture MAY be implemented through different tools, but all writers MUST remain quiesced until the final manifest is published.

### 7.4 Counts and selected hashes

Verification evidence MUST record at least:

- database counts for sync objects, revisions, content blobs, conflicts, tombstones, operation-log rows, and idempotency records;
- highest operation sequence and accepted migration set;
- object-store committed blob count and total bytes;
- a deterministic bounded selection of individual blob hashes and verified sizes;
- Worktree file count, total bytes, and deterministic selected file hashes when Worktree is included.

Selection MUST be reproducible from recorded rules, such as the first and last bounded entries in canonical hash/path order. It MUST NOT depend on nondeterministic directory enumeration.

### 7.5 Partial failure and cleanup

- A failed attempt MUST leave no `complete` manifest.
- Staging artifacts MUST remain outside the final backup identity.
- The helper MUST attempt bounded cleanup of temporary database dumps, partial archives, and staging files.
- Cleanup failure MUST be reported as `failed_cleanup_required`; it MUST NOT be hidden by the primary failure.
- A partial artifact MUST NOT be reused as a complete artifact merely because its file exists.
- A failed attempt MAY leave access-controlled diagnostic metadata and immutable deduplicated content, but it MUST be marked non-restorable.

### 7.6 Overwrite and filesystem permissions

V1 backup creation MUST be create-only:

- an existing final `backup_id` MUST NOT be overwritten;
- while a job is non-terminal, a retry with the same idempotency key and fingerprint MUST replay current status or safely resume only a phase proven idempotent under the same job and a distinct recorded `attempt_id`;
- after a terminal failure, the same idempotency key MUST replay that failure; a new execution requires a new job/idempotency key linked to the failed job and MUST use a new attempt identity;
- no retry or new attempt may replace a complete backup;
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
empty database
empty object-store target
empty optional Worktree target
adapters disabled
public mutation admission closed
```

### 8.2 Restore preflight

Before confirmation, Server/helper MUST validate:

- the job is in `maintenance` with current quiescence evidence;
- the backup manifest exists, is `complete`, and matches `expected_manifest_sha256`;
- all required artifacts exist, are readable, and recompute to manifest checksums;
- artifact formats/tool versions are supported;
- the target identity matches `expected_target_identity`;
- the target is separate and empty for the initial acceptance path;
- the target has sufficient capacity and enforceable private permissions;
- the requested release set is compatible with the backup schema, migration state, object-store format, and Worktree state format;
- adapters and external writers are disabled/stopped;
- no destructive step has executed before confirmation.

Any failure MUST leave the job in a safe non-running state and MUST perform no target mutation.

### 8.3 Destructive confirmation

Confirmation MUST be a durable, single-use decision bound to:

```text
job_id
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

### 8.4 Restore order

After confirmation, the helper MUST execute this order:

1. revalidate maintenance generation, target identity, emptiness/replaceability, artifact checksums, and confirmation binding;
2. ensure Server and all adapters remain stopped or forced into recovery validation mode;
3. provision the target database without accepting application writes;
4. restore PostgreSQL metadata;
5. validate restored schema and migration state;
6. restore the matching object-store artifact;
7. verify object-store counts, archive checksum, and selected blob hashes;
8. restore Worktree data when the manifest requires it;
9. verify Worktree counts and selected hashes when applicable;
10. while application writers remain stopped, apply only explicitly compatible sequential forward migrations required by the selected release;
11. validate the resulting schema and migration state before application startup;
12. start the compatible Server release in recovery validation mode with mutation admission closed and all adapters forced disabled regardless of restored desired state;
13. run object-store consistency checks, doctor, and preflight;
14. record a rollback/resume checkpoint;
15. wait for explicit operator acceptance before entering `resuming`;
16. reconcile desired/effective adapter state and open mutation admission only after all gates succeed.

Recovery validation mode is deployment/Server fail-closed authority. Restored database values MUST NOT be able to auto-enable adapters or public mutations before verification.

### 8.5 Restore failure behavior

On any failure:

- target mutation admission MUST remain closed;
- adapters MUST remain disabled;
- the target MUST be marked quarantined/incomplete;
- the job MUST record the last completed phase and safe failure category;
- automatic retry MUST resume only from a phase proven idempotent by durable evidence;
- automatic destructive cleanup or fallback MUST NOT occur;
- cleanup of the failed separate target requires a distinct authorized cleanup action or operator-owned environment disposal;
- the source/original environment MUST remain untouched by the empty-target acceptance path.

A restore MUST NOT combine artifacts from different backup IDs or recovery windows.

## 9. Upgrade contract

### 9.1 Preconditions

An upgrade job MAY be planned before quiescence and backup. It MUST NOT apply a migration, activate the target release, or perform any irreversible release/schema mutation unless all of the following are true:

- current release, schema, migration state, object-store format, and adapter protocol state are known;
- the target release compatibility record is accepted;
- a complete backup of the current environment has been captured and verified under this contract;
- the backup is bound to the upgrade job and its manifest checksum is recorded;
- quiescence evidence is current;
- all adapters are effectively disabled/stopped at the requested generation;
- the operator has confirmed the exact target release and rollback backup after seeing their bound identities/checksums;
- the migration plan contains an ordered forward-only sequence.

### 9.2 Ordering

The upgrade sequence is:

1. `normal -> quiescing` and close new mutation admission;
2. drain/cancel in-flight work to a proven safe boundary;
3. reach `quiesced` and capture quiescence evidence;
4. enter `maintenance`;
5. create and verify the mandatory backup;
6. stop external adapters and the Server processes that could write;
7. verify the pre-upgrade rollback checkpoint;
8. obtain a durable confirmation bound to the target release set, migration plan, accepted backup, manifest checksum, and maintenance generation;
9. apply migrations sequentially in repository order;
10. activate the complete compatible release set;
11. start Server in maintenance/recovery validation mode with adapters disabled;
12. verify binary startup, schema/migration state, object-store compatibility, Worktree state compatibility, doctor, and preflight;
13. record the post-upgrade checkpoint;
14. enter `resuming` only after explicit operator acceptance;
15. restore approved desired adapter state, verify effective state, then enter `normal`.

A release set includes every mutually constrained binary/protocol artifact required for safe operation, including Server, helper/migration tooling, CLI compatibility, and applicable adapter protocol versions. Partial activation MUST NOT be accepted as a successful upgrade.

### 9.3 Migration rules

- Migrations MUST be applied one at a time in canonical sequence.
- Each migration MUST complete transactionally when PostgreSQL supports the operation.
- A failed migration MUST stop the sequence immediately.
- A migration MUST NOT be skipped, reordered, silently marked applied, or replaced by a database reset.
- Reverse migrations are not the rollback mechanism.
- The job MUST record the exact pre-state, attempted migration, observed post-state, and transaction result without raw SQL or database URLs.

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

At that point the upgrade job MUST record the failed gate and transition to `failed` without reopening mutation admission. The durable rollback checkpoint below MUST be persisted with that failure. If the operator selects rollback, Server MUST create a linked rollback job in `planned`; that job MUST enter `awaiting_confirmation` before any rollback restore begins:

```text
failed_upgrade_job_id
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
```

The operator MUST explicitly choose one of:

- retry a bounded non-destructive verification/correction step;
- resume the original release without restore, but only when the original data/schema state is proven unchanged and compatible;
- execute rollback restore from the accepted backup and compatible release set;
- leave the environment in maintenance for investigation.

No default choice is implied by timeout or process restart.

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
job ids and attempts
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
operator confirmation audit ids
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

Before object-store verification, Server MUST perform a non-mutating idempotency lookup. When the same scope/key already has a committed response with the same fingerprint, Server MUST replay it immediately without requiring object-store availability. A different fingerprint MUST fail with `idempotency_key_reused`. Absence of a record is only a preliminary observation; the transaction MUST lock and re-evaluate idempotency to resolve races.

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
7. perform all precondition and consistency validation;
8. insert one immutable new file revision;
9. update `sync_objects.current_revision_id` to the new revision;
10. append the existing V1 `conflict_resolved` operation-log entry with `conflict_id` and the new `revision_id`;
11. append the safe `conflict.accept_conflict.succeeded` audit event;
12. mark the conflict resolved with resolver and resolution timestamp;
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
- the incoming revision belongs to the expected object/path lineage;
- the incoming revision hash and size equal the conflict metadata and request preconditions;
- the committed object-store blob recomputes to the expected hash and size;
- the principal remains authorized and maintenance admission remains `normal` for the bound control generation.

### 12.7 Revision and operation effects

The new revision MUST contain:

```text
new revision id
same object id
original normalized path
parent_revision_id = previously current revision id
content_sha256 = accepted incoming content hash
size_bytes = accepted incoming size
created_by = authenticated resolution actor or accepted system actor mapping
created_at
```

The previous current revision MUST remain unchanged and queryable as history.

The object current pointer MUST change exactly once to the new revision. The operation-log entry MUST use `conflict_resolved`, reference the conflict and new revision, and receive the globally monotonic sequence only inside the transaction.

The audit event MUST identify the action, conflict, object, old current revision, new current revision, actor, and safe result category. It MUST not contain content or raw paths beyond the accepted normalized vault path policy.

The conflict MUST be marked resolved only after the new revision, object pointer, operation log, and audit event have been written successfully in the transaction.

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

- return `idempotency_key_reused`;
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

The accepted actions retain these semantics:

`accept_current`:

- current revision unchanged;
- no new file revision;
- conflict resolved;
- conflict copy may be marked resolved according to accepted materialization behavior;
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
| `idempotency_key_reused` | same key, different fingerprint | 409 |
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

- repositories for operational jobs, confirmation bindings, audit events, and recovery evidence;
- lock/read/update primitives needed by atomic `accept_conflict`, including conflict and object/current-revision locking under caller transactions;
- immutable revision insertion and current-pointer update composition under one caller transaction;
- idempotency uniqueness and transaction-safe replay persistence;
- backup/doctor fact providers for schema state, counts, operation sequence, blob metadata, and selected-hash verification;
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

- maintenance-state admission and quiescence evidence collection;
- operational-job/idempotency/confirmation/audit choreography;
- allowlisted helper invocation and result validation;
- recovery validation startup mode with forced adapter disablement;
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
- private staging/final publication, permissions, checksums, readability verification, and bounded cleanup;
- PostgreSQL/object-store/optional-Worktree capture and restore;
- sequential migration invocation and exact result evidence;
- recovery validation launch/stop ordering;
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
- partial-failure cleanup and non-restorable staging state;
- idempotent retry and overwrite refusal;
- restore into a separate empty environment;
- adapters disabled during validation;
- schema/migration/object-store/Worktree verification;
- doctor and preflight result handling;
- failed migration/startup/schema/object/doctor gates remain in maintenance;
- rollback from accepted backup and compatible release set;
- crash/failure injection around object-store finalization, revision insert, current-pointer update, operation/audit append, idempotency persistence, and commit;
- concurrent `accept_conflict`, stale revision, replay, missing content, hash mismatch, storage failure, and ambiguous commit behavior;
- absence of executable changes from this documentation Candidate.

## 14. V1 safety invariants

1. No recovery operation begins without current quiescence evidence.
2. No destructive restore/rollback begins without a bound durable confirmation.
3. CLI never becomes an infrastructure shell, SQL client, object-store client, or provider-private client.
4. A complete backup is one manifest-bound recovery set from one quiesced window.
5. Partial artifacts are never restorable merely because files exist.
6. The initial restore acceptance target is separate and empty.
7. Adapters remain disabled until post-operation doctor/preflight and explicit resume.
8. Rollback restores an accepted backup and compatible release set; it does not guess reverse migrations.
9. Previous authoritative revisions remain immutable history.
10. Object-store bytes are non-authoritative until referenced by a committed database revision/current pointer.
11. `accept_conflict` is one atomic database transaction after verified immutable content preparation.
12. Idempotency replay returns the original effect; key reuse with a different fingerprint performs no work.
13. Metadata-only conflict actions remain metadata-only.
14. Public output, manifests, audit, and logs remain free of secrets, content, internal SQL, provider payloads, database URLs, and host paths.
15. Missing or skipped evidence is never reported as success.
