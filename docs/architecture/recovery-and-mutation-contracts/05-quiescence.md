# Recovery and Mutation Contracts — Quiescence and Adapter Inventory

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 6. Quiescence and maintenance evidence

A backup, restore, migration, rollout, or rollback MUST NOT begin until the current Stage 10 immutable quiescence evidence proves all relevant mutation sources are closed. Aggregate singleton Worktree/GDrive status fields are insufficient because V1 may configure multiple instances.

This section specializes both the parent operational-control contract and its normative runtime addendum `operational-control-contracts/runtime-and-credential-clarifications.md`. For standalone GDrive, runtime lease/epoch authority, ordered effective reports, bounded mutation permits, and uncertain external effects are mandatory quiescence inputs rather than optional diagnostics.

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

### 6.1 Standalone GDrive runtime authority

Every standalone GDrive adapter entry MUST additionally contain the exact current durable runtime authority:

```text
runtime_instance_id
runtime_lease_version
standalone_runtime_epoch
last_accepted_report_sequence
last_accepted_report_fingerprint
open_mutation_permits = 0
uncertain_external_effects = 0
takeover_state = clear
runtime_lease_expires_at
```

For `drain_proof = acknowledged`, those values MUST match the current runtime lease record and latest accepted effective report at evidence capture. The report MUST be the latest contiguous ordered report for the current `standalone_runtime_epoch`; it MUST apply the current adapter-control and maintenance generations, prove `in_flight = false`, close every bounded mutation permit, and leave no uncertain provider/Core external effect. A heartbeat, desired-disabled state, expired lease, stale report, prior epoch, skipped sequence, or process disconnection cannot satisfy this proof.

A standalone GDrive instance using `drain_proof = externally_fenced` MUST bind the fence to the exact `adapter_id`, runtime instance, lease version, and epoch known at fence creation. The fence MUST prevent the old runtime from obtaining further mutation admission and MUST either reconcile every open permit and uncertain external effect to a terminal durable outcome or explicitly cover those exact effects with an independently verifiable provider/external fence. Stopping the process, revoking credentials, expiring the lease, or allocating a newer epoch does not by itself prove that an already admitted external mutation stopped.

A runtime takeover with `takeover_state = reconciliation_required` cannot satisfy quiescence. The new epoch has no quiescence authority until prior-epoch checkpoints, open permits, and uncertain effects are reconciled or covered by an exact external fence, after which a latest ordered report for the current epoch proves the clean state.

A hosted Worktree entry MUST identify its exact runtime instance and prove the current maintenance hold, no in-flight cycle, and its durable checkpoint summary. When a Worktree artifact is included, `worktree_external_writer_evidence` MUST additionally prove that no process/user outside the controlled adapter inventory can write the captured path. A read-only Server bind alone is insufficient.

Obsidian devices are not centrally frozen and are not members of `adapter_instances`; the evidence asserts only that Server rejects their authoritative mutations. Device-local edits do not mutate the recovery set until normal admission resumes.

Evidence becomes invalid before maintenance entry or job start when `maintenance_generation`, `adapter_inventory_generation`, any included adapter desired/control generation, external fence, admission fence, active mutation/transaction count, or recovery-specific writer fact changes. For standalone GDrive, evidence is also invalidated by any runtime lease-version or epoch change, accepted later report, changed latest report fingerprint/sequence, newly opened mutation permit, newly discovered uncertain external effect, or takeover-state change. Server MUST re-read and compare all identities and runtime-authority values transactionally before `quiesced -> maintenance` and before a recovery job enters `running`.

A timeout, stale heartbeat, disconnected un-fenced adapter, unknown process, incomplete inventory, non-zero writer/permit/effect count, non-latest runtime report, unresolved takeover, or unverifiable external Worktree writer MUST fail the precondition. It MUST NOT be treated as quiesced.

Every recovery job that depends on quiescence MUST persist references to the exact `quiescence_evidence_id`, its digest, `maintenance_generation`, `adapter_inventory_generation`, the complete expected adapter-control-generation list, and the complete standalone GDrive runtime-authority list. Before destructive PostgreSQL work, the external recovery-control envelope MUST preserve the full immutable adapter inventory/evidence snapshot and every bound runtime lease/epoch/report/permit/effect value, not only an aggregate status or evidence ID.
