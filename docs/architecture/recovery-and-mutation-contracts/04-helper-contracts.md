# Recovery and Mutation Contracts — Helper Request and Result Contracts

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 5. Versioned recovery request and result contracts

All executor requests MUST carry `schema: haze-sync.recovery.request.v1` and all helper results MUST carry `schema: haze-sync.recovery.result.v1`. These are bounded child contracts of one `haze-sync.operational-job.v1`; they do not define another lifecycle, CAS namespace, idempotency namespace, lease, slot, or audit authority.

### 5.1 Common executor request fields

```text
schema
operation_id
kind
helper_action
expected_job_version
executor_id
executor_fence
execution_scope_digest
expected_maintenance_generation | null
expected_adapter_control_generations[]
adapter_inventory_generation | null
quiescence_evidence_id | null
quiescence_evidence_digest | null
destructive_execution_slot_id | null
expected_slot_version | null
request_fingerprint
source_environment_id
target_environment_id
backup_destination_id
credential_ref
requested_at
dry_run
```

The current raw lease token MUST be conveyed only through accepted bounded secret transport and MUST NOT be serialized into the persisted request, command arguments, logs, audit, manifest, or recovery-control envelope. The helper MUST reject a request when its observed job version, executor fence, lease proof, slot identity/version, maintenance generation, adapter-control generation list, inventory generation, or evidence digest does not match current authority.

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
recovery_control_envelope_id
```

Upgrade (`deployment_rollout`):

```text
from_release_set_id
to_release_set_id
accepted_backup_operation_id
accepted_backup_id
accepted_manifest_id
migration_plan_id
compatibility_record_id
confirmation_binding
recovery_control_envelope_id
```

Rollback (`deployment_rollback`):

```text
failed_upgrade_operation_id
rollback_checkpoint_id
accepted_backup_id
accepted_manifest_id
rollback_release_set_id
expected_target_identity
confirmation_binding
recovery_control_envelope_id
```

### 5.3 Typed helper result

The helper result MUST include:

```text
schema
operation_id
observed_job_version
kind
helper_action
executor_id
executor_fence
destructive_execution_slot_id | null
observed_slot_version | null
attempt_id
checkpoint_version
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
adapter_inventory_generation
quiescence_evidence_id
adapter_disabled_evidence_id
recovery_control_envelope_id | null
recovery_control_envelope_version | null
recovery_checkpoint_id
```

`outcome` is one of `succeeded`, `failed`, or `cancelled`. The helper result is evidence; only Server/recovery reconciliation may transition the canonical operational job. A result from a stale `job_version`, `executor_fence`, lease, or execution slot MUST be rejected as stale evidence and MUST NOT change job state. No result may use `succeeded` when a required artifact, checksum, count, inventory entry, cleanup, checkpoint, or verification result is missing.
