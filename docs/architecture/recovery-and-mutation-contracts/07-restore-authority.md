# Recovery and Mutation Contracts — Restore Control Authority

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 8. Restore contract

### 8.1 Initial acceptance target

The first required V1 acceptance path is restore into a separate environment with an empty product-data target.

A production in-place restore MUST NOT be the first acceptance path and MUST NOT be represented as proven merely because empty-target restore succeeds.

The initial path requires:

```text
source_environment_id != target_environment_id
replace_target = false
empty product-data database/schema target
pre-provisioned live control plane isolated from the product restore set
current inventory-complete quiescence evidence
canonical restore job in running with current job CAS/executor fence/lease
global destructive execution slot bound to the restore operation
active external recovery-control envelope durable and re-readable
empty object-store target
empty optional Worktree target
all centrally controlled adapters acknowledged or individually externally fenced
public mutation admission closed
```

### 8.2 Live operational-control-plane preservation and external execution authority

Ordinary product-data restore MUST NOT overwrite, import, roll back, or reactivate any live target record in these Stage 10 control families:

```text
maintenance state and maintenance_generation
adapter inventory and adapter_inventory_generation
adapter desired/effective records and adapter_control_generation values
principals and credentials with their namespace-specific CAS values
operational jobs and job_version values
executor identities, executor_fence values, leases, and checkpoints
scoped/global execution slots and slot_version values
confirmations and operational idempotency
audit events
```

Before destructive PostgreSQL work begins, Deployment MUST create a durable external recovery-control envelope with schema `haze-sync.recovery-control-envelope.v1` in storage outside the PostgreSQL product restore set. The envelope is a fenced continuation checkpoint for the same canonical job; it is not an independent job, lease namespace, slot, confirmation, idempotency, or audit system.

The envelope MUST contain at least:

```text
schema
envelope_id
envelope_version
state: pending | armed | active | reconciling | closed | blocked_uncertain
operation_id
kind
job_version
maintenance_required
destructive
execution_scope_digest
request_fingerprint
expected_maintenance_generation
expected_adapter_control_generations[]
adapter_inventory_generation
quiescence_evidence_id
quiescence_evidence_digest
adapter_instances[]
obsidian_authoritative_mutation_gate
artifact_manifest_id
artifact_manifest_digest
confirmation_digest
confirmation_expires_at
executor_id
executor_fence
lease_token_digest
lease_heartbeat_at
lease_expires_at
destructive_execution_slot:
  slot_id
  slot_version
  operation_id
  maintenance_generation
  executor_fence
  blocked_uncertain
current_phase
current_step_id | null
last_completed_checkpoint
last_external_effect_identity | null
created_at
updated_at
```

`envelope_version` is a namespace-specific monotonic CAS token. `adapter_instances` is the complete immutable inventory snapshot defined in section 6, including every configured Worktree/GDrive identity and its drain/fence evidence. The envelope MAY contain only the non-reversible `confirmation_digest` and `lease_token_digest` required to preserve authority. It MUST NOT contain credentials, raw idempotency keys, raw confirmation values, raw lease tokens, database URLs, host paths, provider payloads, or file content.

#### 8.2.1 Authority handoff before control-storage unavailability

The external envelope becomes active through this fail-closed sequence:

1. The confirmed restore job transitions to `running` through the canonical Stage 10 transaction, acquiring the global destructive slot, assigning `executor_id`, allocating a new `executor_fence`, storing the lease digest/timestamps, incrementing `job_version`, and appending the start audit event.
2. The executor creates a `pending` envelope from the exact post-start job, slot, confirmation, manifest, quiescence, inventory, and adapter-generation snapshot.
3. Server re-reads the pending envelope and, in one canonical control transaction, compares the current job version/fence/lease/slot/generations, stores `envelope_id` and envelope digest in the job checkpoint, changes the checkpoint to `external_authority_armed`, increments `job_version`, and appends an audit event.
4. The executor CAS-transitions the envelope from `pending` to `armed`, recording the canonical arm checkpoint proof. It then revalidates the updated job version/fence/lease/slot and CAS-transitions the envelope to `active` immediately before the first action that can make canonical PostgreSQL control storage unavailable.
5. No destructive PostgreSQL effect may start unless the canonical job checkpoint and active envelope mutually bind the same `operation_id`, current `job_version`, `executor_id`, `executor_fence`, lease digest, slot ID/version, maintenance generation, complete adapter-generation list, inventory generation, evidence digest, manifest digest, and request fingerprint.

A partially created, stale, or unarmed envelope grants no execution authority.

#### 8.2.2 Authority while canonical PostgreSQL is unavailable

While canonical control storage is unavailable:

- only the same executor holding the raw lease token matching the envelope digest may continue;
- every heartbeat and phase/checkpoint update MUST compare and increment `envelope_version` and match `operation_id`, `executor_fence`, and the bound destructive slot;
- before each bounded external step, the executor MUST verify the envelope lease and fence and record the step intent/checkpoint before the effect;
- the global destructive slot remains exclusively bound to the operation and `executor_fence`; no competing operation may start or infer release;
- lease renewal extends time only and MUST NOT change `executor_fence`;
- executor takeover is forbidden while canonical control storage is unavailable because a new canonical fence/job version cannot be allocated transactionally;
- envelope lease expiry, executor loss, envelope CAS conflict, missing inventory/fence evidence, or uncertain step outcome changes the envelope to `blocked_uncertain` when possible and MUST stop further side effects;
- lease expiry alone MUST NOT release the slot, authorize takeover, or prove that an external effect stopped.

#### 8.2.3 Reconciliation after control storage returns

When canonical control storage is readable again, Server/reconciler MUST:

1. read the canonical job, global slot, active/blocked envelope, immutable evidence, and external effect status;
2. require exact operation/fence/slot binding and reject any stale job, slot, or envelope version;
3. persist envelope checkpoints and truthful outcomes into the same canonical job using `expected_job_version`, current `executor_fence`, lease proof or explicit reconciliation authority, and slot CAS;
4. append canonical audit events containing the resulting `job_version` and `executor_fence`;
5. leave the global slot held or `blocked_uncertain` until no further side effect can start;
6. close the envelope only after canonical state is durable and, for a terminal operation, the slot is safely released through canonical slot CAS.

Disagreement, missing envelope data, mismatched fence/slot, or an unverifiable external outcome MUST fail closed as `recovery_control_state_uncertain`. It MUST NOT be repaired by inventing a replacement job, decrementing a fence/version, or silently releasing the slot.

The initial separate-target acceptance path MUST provision the target control plane independently and put it in `maintenance` before product data is restored. The live target control plane MAY be held in a separate database/schema excluded from restore, or the helper MAY perform a verified filtered restore leaving those record families untouched. A blind full-database restore into live control authority is forbidden.

Source control-plane rows present in a physical backup MUST remain quarantined as `control_plane_recovery_only`. They MUST NOT reopen mutation admission, alter adapter inventory/control generations, revive revoked credentials, replace the current restore job, overwrite its fence/lease/slot, erase idempotency decisions, or rewrite audit history.

Restoring the control plane itself requires a separate explicitly confirmed control-plane recovery procedure whose own job/fence/lease/slot authority remains outside the restore set. That procedure is not the ordinary restore path and is not accepted by this Issue.
