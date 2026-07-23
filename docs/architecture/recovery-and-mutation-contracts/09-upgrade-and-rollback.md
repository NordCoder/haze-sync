# Recovery and Mutation Contracts — Upgrade and Rollback

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

## 9. Upgrade contract

### 9.1 Preconditions and job classification

Upgrade is the `deployment_rollout` operational kind with `maintenance_required = true` and `destructive = true`. It MUST NOT enter `running`, acquire the global destructive slot, apply a migration, activate a release, or perform irreversible release/schema mutation unless all of the following are true:

- current release, product/control schema, migration state, object-store format, adapter protocol state, `maintenance_generation`, and complete adapter inventory are known;
- the target release compatibility record is accepted;
- a separate linked `backup` operation completed a manifest-last verified backup of the current environment;
- the accepted backup operation/manifest/digest are bound into the rollout plan/fingerprint;
- current quiescence evidence covers every configured Worktree/GDrive identity and every expected adapter-control generation;
- all adapters are acknowledged-held or individually externally fenced;
- the rollout job's current `job_version`, immutable plan/manifest, execution scope, and one-time confirmation are valid;
- the global destructive slot is available;
- the migration plan contains an ordered forward-only sequence.

A rollout job MAY remain `planned` while the linked backup operation runs. It MUST NOT perform backup or deployment side effects while merely planned/awaiting confirmation. The rollout enters `running` only immediately before its first deployment/migration effect and only through the canonical lease/fence/slot transaction.

### 9.2 Ordering

The upgrade sequence is:

1. create the `deployment_rollout` job in `planned` with classifications, execution scope, expected namespace-specific generations, and complete adapter-generation list;
2. `normal -> quiescing`, close admission, and drain/cancel in-flight work;
3. reach `quiesced` with inventory-complete evidence;
4. run and complete a linked scoped `backup` job, producing the accepted complete manifest;
5. enter `maintenance`, finalize the immutable rollout plan/manifest, and bind the backup, migration plan, target release, generations, inventory/evidence, and execution scope;
6. issue and accept one-time confirmation against current `job_version` and exact plan inputs;
7. atomically transition rollout to `running`, acquire the global destructive slot, allocate executor fence/lease, increment `job_version`, and audit;
8. create/arm/activate the external recovery-control envelope with the full inventory and job/fence/lease/slot snapshot before any migration or service action can make control storage unavailable;
9. stop/fence all external adapters and Server processes that could write;
10. verify the pre-upgrade rollback checkpoint;
11. apply migrations sequentially in repository order, checking envelope/job fence/lease/slot before each bounded step;
12. activate the complete compatible release set;
13. start Server in maintenance/recovery validation mode with adapters disabled;
14. reconcile envelope checkpoints into canonical control state and verify binary startup, product/control schema, object-store/Worktree compatibility, doctor, and preflight;
15. record the post-upgrade checkpoint;
16. enter `resuming` only after explicit operator acceptance;
17. restore approved desired adapter state for every inventory identity, verify effective generations, then enter `normal`;
18. transition rollout terminal, release the global slot only after no further effect can start, and close the envelope after canonical audit/state is durable.

A release set includes every mutually constrained binary/protocol artifact required for safe operation, including Server, helper/migration tooling, CLI compatibility, control-store compatibility, and applicable adapter protocol versions. Partial activation MUST NOT be accepted as success.

### 9.3 Migration rules

- Migrations MUST be applied one at a time in canonical sequence.
- Each migration MUST complete transactionally when PostgreSQL supports the operation.
- A failed migration MUST stop the sequence immediately.
- A migration MUST NOT be skipped, reordered, silently marked applied, or replaced by a database reset.
- Reverse migrations are not the rollback mechanism.
- Before each step, the executor MUST validate its current fence/lease/slot and checkpoint intent in canonical control storage or the active envelope.
- The rollout job/envelope MUST record exact pre-state, attempted migration, external step identity/fence, observed post-state, and transaction result without raw SQL or database URLs.

### 9.4 Failure boundaries

Migration failure:

- stop further migrations;
- remain in `maintenance`;
- preserve the current executor fence and global slot until the failed/uncertain effect is reconciled;
- verify whether the migration transaction rolled back and whether schema equals the pre-upgrade checkpoint;
- do not start the new release unless compatibility with the observed schema is explicitly accepted.

Binary startup, schema, object-store, or doctor/preflight failure:

- keep admission closed and every adapter held/fenced;
- preserve observed schema/release/effect evidence in the canonical job/envelope;
- do not fall back by starting an arbitrary old binary against unknown state;
- do not enter `resuming`;
- retain/block the global slot when any external outcome is uncertain;
- require evidence-backed correction or a linked rollback after safe slot reconciliation.

## 10. Rollback contract

Rollback is coordinated restoration of an accepted backup plus a compatible release set through operational kind `deployment_rollback`. It is not speculative reverse migration.

### 10.1 Rollback decision point and execution authority

The exact rollout rollback decision point is after any rollout gate fails and before `resuming` opens mutation admission or enables adapters.

The failed `deployment_rollout` job MUST record the gate and transition through current job CAS without reopening admission. If its external outcome is known and no further effect can start, its executor releases the global destructive slot through slot CAS after terminal persistence. If any effect is uncertain, the rollout job/envelope retains or blocks the slot; a rollback job MUST NOT start until a reconciler proves the prior executor can no longer act and safely releases/reassigns authority.

If the operator selects rollback, Server creates a linked `deployment_rollback` job in `planned` with a new operation ID/idempotency key, current `job_version`, classifications, execution scope, expected maintenance/adapter generations, complete inventory/evidence, accepted backup, and compatible release set. It MUST enter `awaiting_confirmation` and use the exact confirmation binding before start.

The durable rollback checkpoint contains at least:

```text
failed_upgrade_operation_id
failed_upgrade_job_version
failed_upgrade_executor_fence
failed_upgrade_slot_id
failed_upgrade_slot_version
failed_phase
pre_upgrade_backup_id
pre_upgrade_manifest_sha256
pre_upgrade_release_set_id
observed_release_set_id
pre_upgrade_schema_state
observed_schema_state
target_environment_identity
maintenance_generation
adapter_inventory_generation
expected_adapter_control_generations
quiescence_evidence_id
adapter_disabled_evidence_id
external_recovery_control_envelope_id
doctor_preflight_summary
```

The operator MUST explicitly choose one of:

- leave the environment in maintenance for investigation;
- run a linked bounded non-destructive verification/correction operation;
- resume the original release without restore only when data/schema/control state is proven unchanged and compatible;
- after prior-slot reconciliation, confirm and execute a linked `deployment_rollback` restore.

No default choice is implied by timeout, lease expiry, process restart, or executor loss. The failed rollout job MUST NOT itself transition back to `awaiting_confirmation` or `running`.

### 10.2 Rollback execution

Rollback enters `running` only by acquiring the global destructive slot and a new executor fence/lease through the canonical transaction. It MUST create/activate its own recovery-control envelope and follow the restore contract, including complete inventory/evidence, manifest/checksum validation, confirmation binding, adapter-disabled recovery mode, ordered product-data/object-store/Worktree restore, compatible sequential migrations, doctor/preflight, explicit resumption, slot release, and envelope closure.

The rollback release set MUST be compatible with restored backup schema/formats. An old binary MUST NOT be started against a newer schema merely because it was previously active.

### 10.3 Resuming the original environment

The original environment MAY resume without restore only when evidence proves all of the following:

- no destructive restore or partial target replacement occurred;
- the database schema/migration state equals a release-compatible checkpoint;
- object-store identity, counts, and selected hashes remain valid;
- required Worktree state remains valid;
- the full adapter inventory/generation snapshot remains valid and every adapter is held/fenced;
- no mutation was admitted during maintenance;
- the selected release set is compatible with observed product/control state;
- no executor with an older/current fence can still act and the global destructive slot is safely reconciled;
- doctor and preflight succeed with adapters still disabled;
- the operator explicitly approves resumption.

Otherwise, the environment MUST remain in maintenance until rollback restore or another accepted recovery plan succeeds.

If mutation admission already reopened after an upgrade, returning to the old backup is a new destructive recovery incident, not the bounded pre-resume rollback path. New post-upgrade data MUST be assessed before restore.
