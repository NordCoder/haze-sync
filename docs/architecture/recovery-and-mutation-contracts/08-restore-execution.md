# Recovery and Mutation Contracts — Restore Preflight and Execution

Status: normative V1 contract part  
Contract version: `haze-sync.recovery-and-mutation.v1`  
Authority: this file is normative only as part of the ordered contract set listed in `../recovery-and-mutation-contracts.md`.

### 8.3 Restore preflight

Before confirmation and again before start, Server/helper MUST validate:

- the canonical restore job uses the exact current model, classifications, `job_version`, execution scope, and expected namespace-specific generations;
- current quiescence evidence covers the complete adapter inventory and matches every expected adapter-control generation;
- the global destructive slot is available before start and exactly bound after start;
- the one-time confirmation is bound to the current `job_version`, maintenance generation, complete expected adapter-generation list, manifest digest, request fingerprint, operation kind, target, backup, release set, and expiry;
- the external recovery-control envelope is pending/armed/active as appropriate and matches the same operation, job version, executor fence/lease, slot version, inventory, evidence, target, and manifest;
- the backup manifest exists, is `complete`, and matches `expected_manifest_sha256`;
- all required artifacts exist, are readable, and recompute to manifest checksums;
- the PostgreSQL restore scope excludes every live control-plane family listed above;
- source control-plane rows, when physically present, are marked `control_plane_recovery_only` and cannot be selected by the ordinary restore plan;
- artifact formats/tool versions are supported;
- the target identity matches `expected_target_identity`;
- the target product-data area is separate and empty for the initial path while isolated live control authority remains intact;
- the target has sufficient capacity and enforceable private permissions;
- the requested release set is compatible with backup product schema, object-store/Worktree formats, and the live control-plane schema;
- every centrally controlled adapter remains acknowledged or individually fenced and all external writers remain stopped;
- no destructive step executed before confirmation, running transition, lease/fence/slot acquisition, and envelope activation.

Any pre-start failure MUST transition `planned`/`awaiting_confirmation` to `failed` or `cancelled` through current job CAS as applicable, preserve the live control plane, release no authority that was not acquired, and perform no target product-data mutation.

### 8.4 Destructive confirmation

Confirmation MUST use the exact Stage 10 binding:

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

The immutable recovery plan/fingerprint MUST additionally bind:

```text
backup_id
manifest_sha256
target_environment_id
target_identity
release_set_id
execution_scope_digest
adapter_inventory_generation
quiescence_evidence_digest
```

A generic `--yes`, an unbound boolean, stale job version, incomplete adapter list, or confirmation from another operation/target/manifest MUST NOT authorize restore. Expiry, generation/inventory drift, changed job/manifest/target/release/scope, or reuse invalidates confirmation without starting work.

### 8.5 Restore order

After valid confirmation, the system MUST execute this order:

1. atomically transition the restore job to `running` by comparing job/generation/evidence/inventory/slot CAS, acquiring the global destructive slot, allocating the executor fence/lease, incrementing `job_version`, and appending audit;
2. create, arm, and activate the recovery-control envelope through section 8.2 before any action can make canonical control storage unavailable;
3. revalidate target identity, product-data emptiness/replaceability, artifact checksums, control-plane exclusion plan, full adapter inventory/fences, current executor lease/fence, and slot binding;
4. ensure Server/application writers and every controlled/external adapter writer remain stopped or forced into recovery validation mode under the recorded fences;
5. checkpoint `postgresql_restore_starting` in the canonical job and active envelope before the external effect;
6. provision the target product-data database/schema area without replacing the isolated live control plane;
7. restore only the PostgreSQL product-data projection;
8. verify every excluded live control-plane family, generation, job/fence/lease/slot record, and audit history is unchanged and no source control row became active;
9. validate restored product schema and migration state;
10. restore and verify the matching object-store artifact, counts, archive checksum, and selected blob hashes;
11. restore and verify Worktree data when required;
12. while writers remain stopped, apply only explicitly compatible sequential forward migrations, using current fence/lease/slot checks before each bounded step;
13. validate resulting product and live-control schemas before application startup;
14. start the compatible Server release in recovery validation mode using preserved live target control authority, mutation admission closed, and every adapter forced disabled/fenced regardless of source backup values;
15. reconcile the same operation, job version, executor fence/lease, slot, and envelope into canonical control storage, then run object-store consistency checks, doctor, and preflight;
16. record rollback/resume checkpoints in the canonical job and envelope through CAS;
17. wait for explicit operator acceptance before entering `resuming`;
18. reconcile approved target desired/effective adapter state for every inventory identity and open admission only after all gates succeed;
19. transition the job terminal, release the global slot only after proving no further effect can start, and close the envelope after canonical terminal/audit state is durable.

Recovery validation mode is deployment/Server fail-closed authority. Restored product rows and quarantined source control rows MUST NOT auto-enable adapters, alter inventory/generations, revive credentials, replace the current job, invalidate its fence/lease/slot, or open public mutations.

### 8.6 Restore failure behavior

On any failure:

- target mutation admission MUST remain closed under the preserved live control plane and deployment fence;
- every adapter identity MUST remain acknowledged-held or individually externally fenced;
- the current executor fence/lease and global destructive slot MUST remain authoritative; lease expiry alone does not release or transfer them;
- the envelope MUST preserve the complete adapter inventory, current job/fence/lease/slot binding, current phase, last completed checkpoint, and uncertain external effect identity;
- the target product-data area MUST be marked quarantined/incomplete;
- source control-plane rows MUST remain inactive and MUST NOT replace target control records;
- the canonical job MUST record, or later reconcile, the last completed phase and safe failure category under the same operation and fence;
- automatic retry/takeover MUST resume only when canonical/external checkpoints prove the previous effect is complete, absent, or exactly queryable/deduplicated;
- an uncertain effect MUST retain or block the global slot and envelope as `blocked_uncertain`;
- automatic destructive cleanup or fallback MUST NOT occur;
- cleanup requires a distinct authorized operation after current job/slot reconciliation;
- the source/original environment MUST remain untouched by the empty-target acceptance path.

Loss/overwrite of the envelope, incomplete adapter inventory, stale job/slot/envelope CAS, lease/fence disagreement, or inability to prove live control-plane preservation MUST fail closed as `recovery_control_state_uncertain`. No resume, retry, takeover, cleanup, or slot release may infer missing state.

A restore MUST NOT combine artifacts from different backup IDs or recovery windows.
