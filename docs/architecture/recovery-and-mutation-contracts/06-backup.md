# Recovery and Mutation Contracts — Backup and Manifest Publication

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

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
