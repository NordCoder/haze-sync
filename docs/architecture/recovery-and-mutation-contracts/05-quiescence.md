# Recovery and Mutation Contracts — Quiescence and Adapter Inventory

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 6. Quiescence and maintenance evidence

A backup, restore, migration, rollout, or rollback MUST NOT begin until the current Stage 10 immutable quiescence evidence proves all relevant mutation sources are closed. Aggregate singleton Worktree/GDrive status fields are insufficient because V1 may configure multiple instances.

The evidence MUST contain at least:

```text
schema_version
quiescence_evidence_id
maintenance_generation
adapter_inventory_generation
admission_fence_closed_at
server_instance_id
server_started_at
active_authoritative_mutations = 0
open_authoritative_transactions = 0
adapter_instances[]
obsidian_authoritative_mutation_gate = closed
active_cli_write_jobs = 0
object_store_writer_count = 0
database_writer_count = 0
worktree_external_writer_evidence
captured_at
```

`adapter_instances` MUST contain exactly one identity-complete entry for every centrally controlled adapter instance in the durable inventory captured at `adapter_inventory_generation`. The inventory includes every configured hosted Worktree instance and every configured standalone GDrive instance, including desired-disabled instances. It increments whenever an instance is added, removed, replaced, or changes stable identity/kind. Duplicate, missing, unknown, or subsequently added/removed identities invalidate the evidence.

Each adapter entry MUST contain at least:

```text
adapter_id
adapter_kind
control_authority
adapter_control_generation
maintenance_generation
desired_enabled
desired_mode
last_applied_adapter_control_generation | null
last_applied_maintenance_generation | null
runtime_lifecycle
connection_state
in_flight = false
drain_proof = acknowledged | externally_fenced
external_fence_id | null
checkpoint_summary
captured_at
```

For `drain_proof = acknowledged`, both last-applied generations MUST equal the desired record values, the current maintenance hold MUST be applied, and `in_flight` MUST be false. For `drain_proof = externally_fenced`, `external_fence_id` MUST identify immutable evidence preventing that exact `adapter_id` from starting Core or replica mutations; a fence for one identity MUST NOT satisfy another.

A standalone GDrive instance is quiesced only when that exact identity either acknowledges its current adapter-control and maintenance generations with no in-flight provider/Core mutation, or has a versioned external fence that stops its process/exclusive runtime lease, revokes every mutating Haze Sync credential for its principal, and waits through bounded credential/cache validity. A missing heartbeat, stale/disconnected process, desired-disabled state, or fence for another identity is not evidence.

A hosted Worktree entry MUST identify its exact runtime instance and prove the current maintenance hold, no in-flight cycle, and its durable checkpoint summary. When a Worktree artifact is included, `worktree_external_writer_evidence` MUST additionally prove that no process/user outside the controlled adapter inventory can write the captured path. A read-only Server bind alone is insufficient.

Obsidian devices are not centrally frozen and are not members of `adapter_instances`; the evidence asserts only that Server rejects their authoritative mutations. Device-local edits do not mutate the recovery set until normal admission resumes.

Evidence becomes invalid before maintenance entry or job start when `maintenance_generation`, `adapter_inventory_generation`, any included adapter desired/control generation, external fence, admission fence, active mutation/transaction count, or recovery-specific writer fact changes. Server MUST re-read and compare all identities transactionally before `quiesced -> maintenance` and before a recovery job enters `running`.

A timeout, stale heartbeat, disconnected un-fenced adapter, unknown process, incomplete inventory, non-zero writer count, or unverifiable external Worktree writer MUST fail the precondition. It MUST NOT be treated as quiesced.

Every recovery job that depends on quiescence MUST persist references to the exact `quiescence_evidence_id`, its digest, `maintenance_generation`, `adapter_inventory_generation`, and the complete expected adapter-control-generation list. Before destructive PostgreSQL work, the external recovery-control envelope MUST preserve the full immutable adapter inventory/evidence snapshot, not only an aggregate status or evidence ID.
