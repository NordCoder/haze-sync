# Recovery and Mutation Contracts — Maintenance and Operational Jobs

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

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

This contract does not define a second job model. Every backup, restore, upgrade, and rollback operation MUST use the Stage 10 operational-control record with schema `haze-sync.operational-job.v1`. In this document, the prose words `job` and `operation` refer to that same record, and the normative identity field is `operation_id`. Recovery request, helper-result, manifest, checkpoint, and recovery-control-envelope records are bounded child evidence referenced by the operational job; they MUST NOT replace its lifecycle, CAS, executor, lease, slot, confirmation, idempotency, or audit authority.

The exact V1 kind mapping is:

```text
backup   -> backup
restore  -> restore
upgrade  -> deployment_rollout
rollback -> deployment_rollback
```

A recovery implementation MUST NOT introduce parallel kinds named `upgrade` or `rollback` in the operational-job registry.

The job record MUST use the current Stage 10 field vocabulary, including at least:

```text
schema_version
operation_id
job_version
kind
maintenance_required
destructive
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
expected_maintenance_generation | null
expected_adapter_control_generations[]
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
execution_scope_digest
executor_id | null
executor_fence
lease_token_digest | null
lease_heartbeat_at | null
lease_expires_at | null
destructive_execution_slot_id | null
```

Each `expected_adapter_control_generations` entry MUST contain exactly `adapter_id` and `adapter_control_generation`; duplicate adapter IDs are invalid. `job_version` starts at `1`. `executor_fence` starts at `0` before the first lease and increments only through a successful lease acquisition or takeover. An unqualified durable field named `expected_control_generation` is forbidden because Stage 10 uses namespace-specific CAS tokens.

The recovery executor classifications are:

| Kind | `maintenance_required` | `destructive` | Required execution authority |
|---|---:|---:|---|
| `backup` | `false` | `false` | Current quiescence evidence plus a scoped execution slot derived from `execution_scope_digest`; capture may run in `quiesced` or `maintenance`. |
| `restore` | `true` | `true` | Global destructive execution slot, executor lease/fence, and active recovery-control envelope before destructive PostgreSQL work. |
| `deployment_rollout` | `true` | `true` | Global destructive execution slot, executor lease/fence, and active recovery-control envelope before migration or release activation. |
| `deployment_rollback` | `true` | `true` | Global destructive execution slot, executor lease/fence, and active recovery-control envelope before restore or release replacement. |

Recovery-specific facts such as backup identity, target identity, release set, helper attempt, artifact checksums, adapter inventory snapshot, and recovery phase MUST be represented in the bounded child request/result, artifact manifest, or `checkpoint`; they MUST NOT create competing top-level job fields with different lifecycle semantics.

Allowed states are exactly `planned`, `awaiting_confirmation`, `running`, `succeeded`, `failed`, and `cancelled`. Legal transitions are:

| Current | Next | Recovery requirement |
|---|---|---|
| create | `planned` | Request, idempotency reservation, classifications, execution scope, and initial `job_version = 1` are durable. |
| `planned` | `awaiting_confirmation` | Immutable plan/manifest and all confirmation inputs are persisted. |
| `planned` | `running` | Confirmation is not required; `expected_job_version`, `expected_maintenance_generation`, every expected `(adapter_id, adapter_control_generation)`, inventory/evidence, and execution slot are current. |
| `planned` | `cancelled` | No external side effect has started. |
| `planned` | `failed` | Planning, generation, manifest, inventory, or pre-execution validation failed before any external side effect. |
| `awaiting_confirmation` | `running` | One-time confirmation is valid; `expected_job_version`, `expected_maintenance_generation`, every expected `(adapter_id, adapter_control_generation)`, inventory/evidence, and execution slot are current. |
| `awaiting_confirmation` | `cancelled` | Operator cancellation or confirmation expiry. |
| `awaiting_confirmation` | `failed` | Plan/manifest/inventory became invalid, a required generation changed, or restart recovery proves execution cannot safely start. |
| `running` | `succeeded` | Every required effect, checkpoint, manifest, lease/slot release condition, and audit outcome is proven. |
| `running` | `failed` | Safe terminal failure is recorded; uncertain external effects keep the destructive slot blocked until reconciliation. |
| `running` | `cancelled` | Cancellation completed and no further side effect can start. |

Every transition MUST compare and increment `job_version`; executor-owned transitions additionally require the current `executor_fence` and lease proof. Terminal states are immutable. A retry after terminal failure creates a linked new `operation_id` with a new idempotency key; it does not reopen or mutate the failed job.

#### 3.2.1 Confirmation binding

A destructive recovery confirmation is the Stage 10 one-time confirmation and MUST be bound to:

```text
operation_id
kind
request_fingerprint
expected_maintenance_generation
expected_adapter_control_generations
job_version
artifact_manifest_digest
confirmation_expiry
```

The recovery request fingerprint and immutable plan/manifest MUST additionally bind the exact backup, target environment/identity, release set, migration plan, requested artifact set, and `execution_scope_digest`. A stale `job_version`, changed adapter inventory/generation, changed manifest, changed target, expired confirmation, or reused confirmation MUST reject start without an external effect.

#### 3.2.2 Executor lease and fencing

A recovery job enters `running` only by atomically:

1. comparing `expected_job_version`, `expected_maintenance_generation`, and every expected adapter-control generation;
2. revalidating the exact quiescence evidence and adapter-inventory generation when required;
3. acquiring the applicable scoped or global execution slot;
4. assigning `executor_id`;
5. incrementing `executor_fence` to a value never previously used by the operation;
6. storing only a non-reversible `lease_token_digest` plus bounded heartbeat/expiry timestamps;
7. incrementing `job_version` and appending the start audit event.

The raw lease token exists only in bounded executor memory or accepted secret transport. Every heartbeat, checkpoint, cancellation acknowledgement, lease release, helper result acceptance, and terminal transition MUST compare `(operation_id, expected_job_version, executor_fence, lease proof)`. A stale executor receives `stale_executor_fence` and MUST stop before another side effect.

Before every bounded external step, the executor MUST re-read or strongly validate its fence and lease. External deployment/provider actions SHOULD use an idempotency or fencing value derived from `(operation_id, step_id, executor_fence)` when supported. Lease expiry alone MUST NOT authorize takeover while an external outcome is uncertain. Takeover requires a larger fence and is allowed only when the durable checkpoint proves no prior step is in flight or the exact effect is externally queryable/deduplicated. A former executor can never write after takeover.

#### 3.2.3 Execution slots

Every recovery executor contract MUST provide a canonical `execution_scope_digest`. The global destructive execution slot uses the Stage 10 record containing at least:

```text
slot_id
slot_version
operation_id | null
maintenance_generation | null
executor_fence | null
blocked_uncertain
timestamps
```

Acquisition, release, or reconciliation requires `expected_slot_version` and increments `slot_version`.

- At most one `running` job with `destructive = true` or `maintenance_required = true` may exist globally.
- Transition to `running` for restore/rollout/rollback MUST acquire the global destructive slot in the same transaction as the job lease and bind it to `operation_id`, `expected_maintenance_generation`, and `executor_fence`.
- A competing start returns `operation_in_progress`; it MUST NOT wait invisibly or begin partial work.
- The slot is released only after a terminal outcome proves no further side effect can start. An uncertain external effect retains or blocks the slot until explicit reconciliation.
- Non-destructive backup jobs MAY overlap only when their execution scopes are proven disjoint. A backup scope that includes the same source environment, PostgreSQL product data, object store, or Worktree as a restore/rollout/rollback overlaps that destructive scope and MUST serialize. Equal or uncertain scope digests MUST also serialize through scoped execution authority.
- Planning and confirmation-pending jobs do not reserve execution authority; their generations, inventory, manifests, job CAS, and slot are revalidated at start.
